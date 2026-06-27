use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::NetworkError;

/// Frame layout: [4-byte payload-len BE][8-byte req_id BE][payload]
///
/// The req_id is echoed back verbatim in the response so the async reader
/// can route each response to the correct waiting caller.
pub async fn write_framed<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    req_id: u64,
    data: &[u8],
) -> Result<(), NetworkError> {
    let mut header = [0u8; 12];
    header[0..4].copy_from_slice(&(data.len() as u32).to_be_bytes());
    header[4..12].copy_from_slice(&req_id.to_be_bytes());
    w.write_all(&header).await?;
    w.write_all(data).await?;
    w.flush().await?;
    Ok(())
}

/// Returns `(req_id, payload)`.
pub async fn read_framed<R: AsyncReadExt + Unpin>(
    r: &mut R,
) -> Result<(u64, Vec<u8>), NetworkError> {
    let mut header = [0u8; 12];
    r.read_exact(&mut header).await?;
    let len = u32::from_be_bytes(header[0..4].try_into().unwrap()) as usize;
    let req_id = u64::from_be_bytes(header[4..12].try_into().unwrap());

    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok((req_id, buf))
}
