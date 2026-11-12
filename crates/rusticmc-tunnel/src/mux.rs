//! A task that drives one yamux connection, with a cloneable handle to open streams.
//!
//! yamux's `Connection` makes progress only while it is polled; [`spawn`] polls it in its own task
//! (inbound streams, outbound opens, I/O) and hands streams out as tokio `AsyncRead + AsyncWrite`.
use std::{collections::VecDeque, task::Poll};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};

/// A yamux stream as a tokio stream.
pub type MuxStream = Compat<yamux::Stream>;

#[derive(Debug, thiserror::Error)]
pub enum MuxError {
    #[error("the tunnel is closed")]
    Closed,
    #[error("multiplexer: {0}")]
    Yamux(String),
}

enum Command {
    Open(oneshot::Sender<Result<yamux::Stream, MuxError>>),
    Close,
}

/// Opens streams on the connection; the connection closes when every clone is dropped.
#[derive(Clone, Debug)]
pub struct MuxControl {
    tx: mpsc::Sender<Command>,
}

impl MuxControl {
    /// Open a new outbound stream.
    pub async fn open(&self) -> Result<MuxStream, MuxError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Open(tx))
            .await
            .map_err(|_| MuxError::Closed)?;
        rx.await
            .map_err(|_| MuxError::Closed)?
            .map(FuturesAsyncReadCompatExt::compat)
    }

    /// Close the connection (streams end; the driver task finishes).
    pub async fn close(&self) {
        let _ = self.tx.send(Command::Close).await;
    }

    /// Whether the driver has stopped.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// The yamux settings both sides use: at most `max_streams` streams, 256 KiB windows each.
pub fn config(max_streams: usize) -> yamux::Config {
    let mut c = yamux::Config::default();
    // Within yamux's default 1 GiB connection window (asserted: 256 KiB per stream at least).
    let max_streams = max_streams.clamp(1, 4096);
    c.set_max_num_streams(max_streams);
    // Window memory bound: 256 KiB per stream is yamux's minimum; allow growth up to 1 MiB each.
    c.set_max_connection_receive_window(Some(max_streams * 1024 * 1024));
    c
}

/// Drive `io` as a yamux connection in a new task. Returns the handle, the inbound streams (at most
/// `inbound_capacity` queued; more are dropped, which resets them) and the driver task, which ends
/// when the connection does.
pub fn spawn<T>(
    io: T,
    mode: yamux::Mode,
    config: yamux::Config,
    inbound_capacity: usize,
) -> (MuxControl, mpsc::Receiver<MuxStream>, JoinHandle<()>)
where
    T: futures::AsyncRead + futures::AsyncWrite + Unpin + Send + 'static,
{
    let (tx, cmds) = mpsc::channel(64);
    let (inbound_tx, inbound_rx) = mpsc::channel(inbound_capacity.max(1));
    let conn = yamux::Connection::new(io, config, mode);
    let task = tokio::spawn(drive(conn, cmds, inbound_tx));
    (MuxControl { tx }, inbound_rx, task)
}

async fn drive<T>(
    mut conn: yamux::Connection<T>,
    mut cmds: mpsc::Receiver<Command>,
    inbound: mpsc::Sender<MuxStream>,
) where
    T: futures::AsyncRead + futures::AsyncWrite + Unpin,
{
    let mut waiting: VecDeque<oneshot::Sender<Result<yamux::Stream, MuxError>>> = VecDeque::new();
    let mut closing = false;
    let result: Result<(), MuxError> = std::future::poll_fn(|cx| {
        if !closing {
            loop {
                match cmds.poll_recv(cx) {
                    Poll::Ready(Some(Command::Open(tx))) => waiting.push_back(tx),
                    Poll::Ready(Some(Command::Close)) | Poll::Ready(None) => {
                        closing = true;
                        break;
                    }
                    Poll::Pending => break,
                }
            }
        }
        if closing {
            for tx in waiting.drain(..) {
                let _ = tx.send(Err(MuxError::Closed));
            }
            return conn
                .poll_close(cx)
                .map(|r| r.map_err(|e| MuxError::Yamux(e.to_string())));
        }
        while !waiting.is_empty() {
            match conn.poll_new_outbound(cx) {
                Poll::Ready(Ok(stream)) => {
                    if let Some(tx) = waiting.pop_front() {
                        // A requester that went away drops the stream, which resets it.
                        let _ = tx.send(Ok(stream));
                    }
                }
                Poll::Ready(Err(e)) => {
                    let msg = e.to_string();
                    for tx in waiting.drain(..) {
                        let _ = tx.send(Err(MuxError::Yamux(msg.clone())));
                    }
                    return Poll::Ready(Err(MuxError::Yamux(msg)));
                }
                Poll::Pending => break,
            }
        }
        loop {
            match conn.poll_next_inbound(cx) {
                Poll::Ready(Some(Ok(stream))) => {
                    let _ = inbound.try_send(stream.compat());
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(MuxError::Yamux(e.to_string())));
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => break,
            }
        }
        Poll::Pending
    })
    .await;
    for tx in waiting.drain(..) {
        let _ = tx.send(Err(MuxError::Closed));
    }
    match result {
        Ok(()) => tracing::debug!(closing, "multiplexer ended"),
        Err(e) => tracing::debug!(error = %e, closing, "multiplexer ended"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_util::compat::TokioAsyncReadCompatExt;

    #[tokio::test]
    async fn streams_both_ways() {
        let (a, b) = tokio::io::duplex(64 * 1024);
        let (client, mut client_in, _t1) = spawn(a.compat(), yamux::Mode::Client, config(8), 8);
        let (server, mut server_in, _t2) = spawn(b.compat(), yamux::Mode::Server, config(8), 8);

        let mut s = client.open().await.unwrap();
        s.write_all(b"hello").await.unwrap();
        s.flush().await.unwrap();
        let mut r = server_in.recv().await.unwrap();
        let mut buf = [0u8; 5];
        r.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");

        let mut s2 = server.open().await.unwrap();
        s2.write_all(b"back").await.unwrap();
        s2.shutdown().await.unwrap();
        let mut r2 = client_in.recv().await.unwrap();
        let mut got = Vec::new();
        r2.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, b"back");

        client.close().await;
        assert!(server_in.recv().await.is_none());
    }
}
