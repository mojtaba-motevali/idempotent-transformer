use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::NetworkError;

/// Write a length-prefixed frame: 4-byte big-endian length followed by `data`.
pub async fn write_framed(stream: &mut TcpStream, data: &[u8]) -> Result<(), NetworkError> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

/// Read a length-prefixed frame: 4-byte big-endian length, then exactly that many bytes.
pub async fn read_framed(stream: &mut TcpStream) -> Result<Vec<u8>, NetworkError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}
