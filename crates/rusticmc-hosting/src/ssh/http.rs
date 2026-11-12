//! Just enough HTTP/1.1 to call Horizon's control API (`/api/v1`, docs/api/server-reference.md)
//! through an SSH `direct-tcpip` channel: one request per connection (`Connection: close`), bodies
//! with a `Content-Length` or up to the end of the stream (what Horizon's `http.rs` sends), and a
//! server-sent events reader for `GET /api/v1/events`.
//!
//! The API checks `Host` against its own address (`panel::host_allowed`), so requests carry the
//! address from `.horizon/api.json`, not the tunnel's.

use crate::error::{HostingError, Result};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// Horizon's `.horizon/api.json`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ApiEndpoint {
    /// `127.0.0.1:PORT` on the VPS.
    pub address: String,
    pub token: String,
}

impl ApiEndpoint {
    pub fn host_port(&self) -> Result<(String, u16)> {
        let (h, p) = self
            .address
            .rsplit_once(':')
            .ok_or_else(|| HostingError::Protocol(format!("bad API address {}", self.address)))?;
        let port = p
            .parse()
            .map_err(|_| HostingError::Protocol(format!("bad API port in {}", self.address)))?;
        Ok((h.trim_matches(['[', ']']).to_owned(), port))
    }
}

/// Build the request bytes.
pub fn request_bytes(
    method: &str,
    path: &str,
    endpoint: &ApiEndpoint,
    body: Option<&serde_json::Value>,
    accept: &str,
) -> Vec<u8> {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\nAccept: {accept}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
        host = endpoint.address,
        token = endpoint.token,
        len = body.len(),
    )
    .into_bytes()
}

/// A response head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub content_length: Option<u64>,
    pub content_type: String,
}

/// Read the status line and headers.
pub async fn read_head<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Result<Head> {
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let mut parts = line.split_whitespace();
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") {
        return Err(HostingError::Protocol(format!(
            "not an HTTP answer: {}",
            line.trim()
        )));
    }
    let status = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| HostingError::Protocol(format!("bad status line: {}", line.trim())))?;
    let mut head = Head {
        status,
        content_length: None,
        content_type: String::new(),
    };
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Err(HostingError::Protocol(
                "the answer ended in its headers".into(),
            ));
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let v = v.trim();
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => head.content_length = v.parse().ok(),
                "content-type" => head.content_type = v.to_owned(),
                "transfer-encoding" if v.eq_ignore_ascii_case("chunked") => {
                    return Err(HostingError::Protocol(
                        "chunked answers are not expected from the control API".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(head)
}

/// One JSON call. Errors (`{"error": "..."}` with 4xx) become [`HostingError::Protocol`] with the
/// API's message.
pub async fn call<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    method: &str,
    path: &str,
    endpoint: &ApiEndpoint,
    body: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let mut reader = BufReader::new(stream);
    reader
        .get_mut()
        .write_all(&request_bytes(
            method,
            path,
            endpoint,
            body,
            "application/json",
        ))
        .await?;
    reader.get_mut().flush().await?;
    let head = read_head(&mut reader).await?;
    let mut buf = Vec::new();
    match head.content_length {
        Some(n) => {
            buf.resize(n as usize, 0);
            reader.read_exact(&mut buf).await?;
        }
        None => {
            reader.read_to_end(&mut buf).await?;
        }
    }
    let value: serde_json::Value = if buf.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&buf).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&buf).into_owned())
        })
    };
    if !(200..300).contains(&head.status) {
        let msg = value["error"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        return Err(HostingError::Protocol(format!(
            "{method} {path}: {} {msg}",
            head.status
        )));
    }
    Ok(value)
}

/// Stream a body to `sink` (backup downloads). Returns the bytes written.
pub async fn download<S, W>(
    stream: S,
    path: &str,
    endpoint: &ApiEndpoint,
    sink: &mut W,
    on_bytes: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<u64>
where
    S: AsyncRead + AsyncWrite + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(stream);
    reader
        .get_mut()
        .write_all(&request_bytes("GET", path, endpoint, None, "*/*"))
        .await?;
    reader.get_mut().flush().await?;
    let head = read_head(&mut reader).await?;
    if !(200..300).contains(&head.status) {
        let mut body = String::new();
        let _ = reader.read_to_string(&mut body).await;
        return Err(HostingError::Protocol(format!(
            "GET {path}: {} {}",
            head.status,
            body.trim()
        )));
    }
    let mut done = 0u64;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let want = match head.content_length {
            Some(n) if done >= n => break,
            Some(n) => buf.len().min((n - done) as usize),
            None => buf.len(),
        };
        let got = reader.read(&mut buf[..want]).await?;
        if got == 0 {
            break;
        }
        sink.write_all(&buf[..got]).await?;
        done += got as u64;
        on_bytes(done, head.content_length);
    }
    sink.flush().await?;
    if let Some(n) = head.content_length
        && done != n
    {
        return Err(HostingError::Protocol(format!(
            "the download stopped at {done} of {n} bytes"
        )));
    }
    Ok(done)
}

/// One server-sent event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: String,
    pub data: String,
}

/// Incremental SSE parser (`id:`, `event:`, `data:` lines, blank line ends an event, `:` comments).
#[derive(Debug, Default)]
pub struct SseParser {
    current: SseEvent,
    has_data: bool,
}

impl SseParser {
    /// Feed one line (without its newline). Returns an event when the line completes one.
    pub fn line(&mut self, line: &str) -> Option<SseEvent> {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            if !self.has_data && self.current.event.is_empty() {
                return None;
            }
            self.has_data = false;
            let mut ev = std::mem::take(&mut self.current);
            if ev.event.is_empty() {
                ev.event = "message".into();
            }
            return Some(ev);
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "id" => self.current.id = Some(value.to_owned()),
            "event" => self.current.event = value.to_owned(),
            "data" => {
                if self.has_data {
                    self.current.data.push('\n');
                }
                self.current.data.push_str(value);
                self.has_data = true;
            }
            _ => {}
        }
        None
    }
}

/// Open an SSE stream and send every event to `tx` until the stream or the receiver ends.
pub async fn events<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    path: &str,
    endpoint: &ApiEndpoint,
    tx: tokio::sync::mpsc::Sender<SseEvent>,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    reader
        .get_mut()
        .write_all(&request_bytes(
            "GET",
            path,
            endpoint,
            None,
            "text/event-stream",
        ))
        .await?;
    reader.get_mut().flush().await?;
    let head = read_head(&mut reader).await?;
    if head.status != 200 {
        return Err(HostingError::Protocol(format!(
            "GET {path}: {}",
            head.status
        )));
    }
    let mut parser = SseParser::default();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(());
        }
        if let Some(ev) = parser.line(line.trim_end_matches('\n'))
            && tx.send(ev).await.is_err()
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::duplex;

    fn endpoint() -> ApiEndpoint {
        ApiEndpoint {
            address: "127.0.0.1:41234".into(),
            token: "t0k3n".into(),
        }
    }

    #[test]
    fn request_carries_the_api_host_and_token() {
        let r = String::from_utf8(request_bytes(
            "POST",
            "/api/v1/command",
            &endpoint(),
            Some(&json!({"line": "list"})),
            "application/json",
        ))
        .unwrap();
        assert!(r.starts_with("POST /api/v1/command HTTP/1.1\r\nHost: 127.0.0.1:41234\r\n"));
        assert!(r.contains("Authorization: Bearer t0k3n\r\n"));
        assert!(r.contains("Content-Length: 15\r\n"));
        assert!(r.ends_with("\r\n\r\n{\"line\":\"list\"}"));
        assert_eq!(
            endpoint().host_port().unwrap(),
            ("127.0.0.1".to_owned(), 41234)
        );
    }

    async fn serve_once(answer: &'static [u8]) -> tokio::io::DuplexStream {
        let (client, mut server) = duplex(64 * 1024);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let _ = server.read(&mut buf).await;
            server.write_all(answer).await.unwrap();
            server.shutdown().await.unwrap();
        });
        client
    }

    #[tokio::test]
    async fn json_call_and_errors() {
        let s = serve_once(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"phase\":\"run\"}").await;
        let v = call(s, "GET", "/api/v1/status", &endpoint(), None)
            .await
            .unwrap();
        assert_eq!(v["phase"], "run");
        let s = serve_once(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 30\r\n\r\n{\"error\":\"missing or wrong t\"}").await;
        let e = call(s, "GET", "/api/v1/status", &endpoint(), None)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("401 missing or wrong t"), "{e}");
        let s = serve_once(b"SSH-2.0-nope\r\n").await;
        assert!(call(s, "GET", "/", &endpoint(), None).await.is_err());
    }

    #[tokio::test]
    async fn download_counts_bytes() {
        let s = serve_once(b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 5\r\n\r\nPK\x03\x04!").await;
        let mut out = Vec::new();
        let n = download(
            s,
            "/api/v1/backups/download?name=a.zip",
            &endpoint(),
            &mut out,
            &|_, _| {},
        )
        .await
        .unwrap();
        assert_eq!(n, 5);
        assert_eq!(out, b"PK\x03\x04!");
        let s = serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nshort").await;
        let mut out = Vec::new();
        assert!(
            download(s, "/x", &endpoint(), &mut out, &|_, _| {})
                .await
                .is_err()
        );
    }

    #[test]
    fn sse_parser() {
        let mut p = SseParser::default();
        assert!(p.line(": keep-alive").is_none());
        assert!(p.line("").is_none());
        assert!(p.line("id: 41").is_none());
        assert!(p.line("event: chat").is_none());
        assert!(p.line("data: {\"name\":\"Alex\"}").is_none());
        let ev = p.line("").unwrap();
        assert_eq!(ev.id.as_deref(), Some("41"));
        assert_eq!(ev.event, "chat");
        assert_eq!(ev.data, "{\"name\":\"Alex\"}");
        p.line("data: a");
        p.line("data: b\r");
        let ev = p.line("\r").unwrap();
        assert_eq!(ev.event, "message");
        assert_eq!(ev.data, "a\nb");
    }

    #[tokio::test]
    async fn sse_stream() {
        let s = serve_once(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nid: 1\nevent: log\ndata: {\"id\":1,\"type\":\"log\",\"line\":\"Done\"}\n\n: keep-alive\n\nid: 2\nevent: phase\ndata: {\"phase\":\"running\"}\n\n").await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        events(s, "/api/v1/events", &endpoint(), tx).await.unwrap();
        let a = rx.recv().await.unwrap();
        assert_eq!(a.event, "log");
        let b = rx.recv().await.unwrap();
        assert_eq!(b.event, "phase");
        assert!(rx.recv().await.is_none());
    }
}
