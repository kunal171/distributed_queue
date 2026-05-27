//! Length-prefixed framing protocol for TCP communication.
//!
//! TCP is a byte stream — it has no concept of message boundaries. This module
//! provides framing: each message is sent as `[4-byte big-endian length][payload]`,
//! so the receiver knows exactly how many bytes to read for each message.
//!
//! Functions are generic over `AsyncRead`/`AsyncWrite` so they work with both
//! full `TcpStream` and split halves (`ReadHalf`/`WriteHalf`).

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Writes a length-prefixed frame to the stream.
///
/// Format: `[4 bytes: payload length as big-endian u32][payload bytes]`
pub async fn write_frame<W: AsyncWrite + Unpin>(stream: &mut W, data: &[u8]) -> std::io::Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

/// Reads a length-prefixed frame from the stream.
///
/// Returns `Ok(None)` if the connection is closed (EOF on the length header).
/// Returns `Ok(Some(bytes))` with the full payload on success.
pub async fn read_frame<R: AsyncRead + Unpin>(stream: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    // EOF here means the peer closed the connection — not an error
    if stream.read_exact(&mut len_buf).await.is_err() {
        return Ok(None);
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(Some(buf))
}