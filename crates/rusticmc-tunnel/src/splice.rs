//! Copying bytes between a player's connection and a tunnel stream, with a hook per chunk for
//! counting, quotas and rate limits.
use std::{io, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The copy buffer per direction.
pub const CHUNK: usize = 16 * 1024;

/// What a [`pump`] does with a chunk it just read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Write it now.
    Go,
    /// Wait this long, then write it (rate limiting).
    Wait(Duration),
    /// Drop it and end the copy (quota used up).
    Stop,
}

/// Copy `r` to `w` until end of stream, then shut `w`'s write side. `on_chunk` sees each chunk's
/// length before it is written. Returns the bytes written.
pub async fn pump<R, W, F>(mut r: R, mut w: W, mut on_chunk: F) -> io::Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(usize) -> Flow,
{
    let mut buf = vec![0u8; CHUNK];
    let mut total = 0u64;
    loop {
        let n = r.read(&mut buf).await?;
        if n == 0 {
            let _ = w.shutdown().await;
            return Ok(total);
        }
        match on_chunk(n) {
            Flow::Go => {}
            Flow::Wait(d) => tokio::time::sleep(d).await,
            Flow::Stop => {
                let _ = w.shutdown().await;
                return Err(io::Error::other("quota exceeded"));
            }
        }
        w.write_all(&buf[..n]).await?;
        w.flush().await?;
        total += n as u64;
    }
}

/// Copy both ways between `a` and `b` until both directions ended, or either failed. Returns the
/// bytes copied a to b and b to a.
pub async fn splice<A, B, F, G>(a: A, b: B, a_to_b: F, b_to_a: G) -> io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite,
    B: AsyncRead + AsyncWrite,
    F: FnMut(usize) -> Flow,
    G: FnMut(usize) -> Flow,
{
    let (ar, aw) = tokio::io::split(a);
    let (br, bw) = tokio::io::split(b);
    tokio::try_join!(pump(ar, bw, a_to_b), pump(br, aw, b_to_a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn copies_and_counts_both_ways() {
        let (a, mut a_peer) = tokio::io::duplex(1024);
        let (b, mut b_peer) = tokio::io::duplex(1024);
        let task = tokio::spawn(splice(a, b, |_| Flow::Go, |_| Flow::Go));
        a_peer.write_all(b"to b").await.unwrap();
        b_peer.write_all(b"to a!").await.unwrap();
        a_peer.shutdown().await.unwrap();
        b_peer.shutdown().await.unwrap();
        let mut got_b = Vec::new();
        b_peer.read_to_end(&mut got_b).await.unwrap();
        let mut got_a = Vec::new();
        a_peer.read_to_end(&mut got_a).await.unwrap();
        assert_eq!(got_b, b"to b");
        assert_eq!(got_a, b"to a!");
        assert_eq!(task.await.unwrap().unwrap(), (4, 5));
    }

    #[tokio::test]
    async fn stop_ends_the_copy() {
        let (mut src, r) = tokio::io::duplex(1024);
        let (w, _sink) = tokio::io::duplex(1024);
        src.write_all(b"x").await.unwrap();
        let err = pump(r, w, |_| Flow::Stop).await.unwrap_err();
        assert_eq!(err.to_string(), "quota exceeded");
    }
}
