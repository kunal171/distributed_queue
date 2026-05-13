use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;


// write a frame to the stream, which consists of a 4-byte big-endian length followed by the data
pub async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    // Write the length of the data as a 4-byte big-endian integer, followed by the data itself
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

// read a frame from the stream, which consists of a 4-byte big-endian length followed by the data
pub async fn read_frame(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
    // Read the length of the incoming frame (4 bytes)
    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).await.is_err() {
        return Ok(None);
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(Some(buf))
}