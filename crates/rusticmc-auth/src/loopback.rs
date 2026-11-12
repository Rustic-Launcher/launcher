//! The authorization-code flow's redirect: a one-shot HTTP listener on `127.0.0.1` (RFC 8252,
//! "loopback interface redirection") that waits for `GET /?code=...&state=...`, answers the browser
//! with a short page, and hands the code back. Plus PKCE (RFC 7636) helpers.

use crate::error::{AuthError, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use rusticmc_launcher_core::Secret;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A PKCE verifier and its S256 challenge.
#[derive(Clone, Debug)]
pub struct Pkce {
    pub verifier: Secret,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let verifier = URL_SAFE_NO_PAD.encode(bytes);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier: Secret::new(verifier),
            challenge,
        }
    }
}

/// A random `state` value.
pub fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The listener, bound and waiting.
pub struct Loopback {
    listener: TcpListener,
    redirect_uri: String,
}

/// What the browser brought back.
#[derive(Debug)]
pub struct Redirect {
    pub code: String,
    pub state: String,
}

const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Rustic Launcher</title>\
<body style=\"font-family:system-ui;background:#14110f;color:#f4ede4;display:grid;place-items:center;height:100vh;margin:0\">\
<div><h1>Signed in</h1><p>You can close this tab and go back to Rustic Launcher.</p></div>";

const ERROR_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Rustic Launcher</title>\
<body style=\"font-family:system-ui;background:#14110f;color:#f4ede4;display:grid;place-items:center;height:100vh;margin:0\">\
<div><h1>Sign-in not completed</h1><p>Go back to Rustic Launcher to try again.</p></div>";

impl Loopback {
    /// Binds `127.0.0.1:<port>` (0: any free port). The redirect URI is `http://localhost:<port>`
    /// (Microsoft accepts any port for a registered `http://localhost` redirect).
    pub async fn bind(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| AuthError::Unexpected(format!("loopback listener: {e}")))?;
        let port = listener
            .local_addr()
            .map_err(|e| AuthError::Unexpected(e.to_string()))?
            .port();
        Ok(Self {
            listener,
            redirect_uri: format!("http://localhost:{port}"),
        })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Waits for the redirect carrying `expected_state`. Other requests (a favicon) get a 404 and
    /// the wait goes on; an `error=` redirect ends it.
    pub async fn wait(self, expected_state: &str, timeout: Duration) -> Result<Redirect> {
        tokio::time::timeout(timeout, self.accept_loop(expected_state))
            .await
            .map_err(|_| AuthError::Timeout)?
    }

    async fn accept_loop(self, expected_state: &str) -> Result<Redirect> {
        loop {
            let (mut stream, _) = self
                .listener
                .accept()
                .await
                .map_err(|e| AuthError::Unexpected(format!("loopback: {e}")))?;
            let mut buf = vec![0u8; 8192];
            let mut len = 0;
            // Read the request head (the query is all we need).
            while len < buf.len() {
                let n = match tokio::time::timeout(
                    Duration::from_secs(5),
                    stream.read(&mut buf[len..]),
                )
                .await
                {
                    Ok(Ok(n)) => n,
                    _ => 0,
                };
                if n == 0 {
                    break;
                }
                len += n;
                if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&buf[..len]);
            let target = head
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_owned();
            let url = url::Url::parse(&format!("http://localhost{target}"))
                .map_err(|e| AuthError::Unexpected(e.to_string()))?;
            let query: std::collections::HashMap<String, String> =
                url.query_pairs().into_owned().collect();
            let respond = |status: &str, body: &str| {
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
                    body.len()
                )
            };
            if let Some(error) = query.get("error") {
                let _ = stream
                    .write_all(respond("400 Bad Request", ERROR_PAGE).as_bytes())
                    .await;
                return Err(if error == "access_denied" {
                    AuthError::Declined
                } else {
                    AuthError::Microsoft(error.clone())
                });
            }
            match (query.get("code"), query.get("state")) {
                (Some(code), Some(state)) if state == expected_state => {
                    let _ = stream
                        .write_all(respond("200 OK", DONE_PAGE).as_bytes())
                        .await;
                    let _ = stream.shutdown().await;
                    return Ok(Redirect {
                        code: code.clone(),
                        state: state.clone(),
                    });
                }
                (Some(_), _) => {
                    // A code for another sign-in (or a forged one): refuse it, keep waiting.
                    let _ = stream
                        .write_all(respond("400 Bad Request", ERROR_PAGE).as_bytes())
                        .await;
                }
                _ => {
                    let _ = stream
                        .write_all(respond("404 Not Found", "").as_bytes())
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_of_the_verifier() {
        let p = Pkce::generate();
        assert_eq!(p.verifier.expose().len(), 43);
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.expose().as_bytes()));
        assert_eq!(p.challenge, expected);
        // RFC 7636 appendix B.
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    async fn get(uri: &str) -> String {
        let http = rusticmc_launcher_core::http::client().unwrap();
        http.get(uri).send().await.unwrap().text().await.unwrap()
    }

    #[tokio::test]
    async fn takes_the_code_with_the_right_state() {
        let lb = Loopback::bind(0).await.unwrap();
        let uri = lb.redirect_uri().to_owned();
        let wait = tokio::spawn(async move { lb.wait("s1", Duration::from_secs(10)).await });
        // A favicon and a forged state are ignored.
        let _ = get(&format!("{uri}/favicon.ico")).await;
        let _ = get(&format!("{uri}/?code=evil&state=other")).await;
        let page = get(&format!("{uri}/?code=abc&state=s1")).await;
        assert!(page.contains("Signed in"));
        let redirect = wait.await.unwrap().unwrap();
        assert_eq!(redirect.code, "abc");
    }

    #[tokio::test]
    async fn an_access_denied_redirect_is_declined() {
        let lb = Loopback::bind(0).await.unwrap();
        let uri = lb.redirect_uri().to_owned();
        let wait = tokio::spawn(async move { lb.wait("s", Duration::from_secs(10)).await });
        let _ = get(&format!("{uri}/?error=access_denied&state=s")).await;
        assert!(matches!(wait.await.unwrap(), Err(AuthError::Declined)));
    }

    #[tokio::test]
    async fn times_out() {
        let lb = Loopback::bind(0).await.unwrap();
        let r = lb.wait("s", Duration::from_millis(50)).await;
        assert!(matches!(r, Err(AuthError::Timeout)));
    }
}
