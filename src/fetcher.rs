use anyhow::Result;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;

pub struct Chunk {
    pub index: usize,
    pub start: u64,
    pub end: u64,
}

pub fn split_into_chunks(file_size: u64, chunk_size: u64) -> Vec<Chunk> {
    let mut chunks = vec![];
    let mut start = 0u64;
    let mut index = 0;

    while start < file_size {
        let end = (start + chunk_size - 1).min(file_size - 1);
        chunks.push(Chunk { index, start, end });
        start += chunk_size;
        index += 1;
    }

    chunks
}

pub async fn download_chunk(
    client: Arc<reqwest::Client>,
    url: String,
    chunk: Chunk,
    out_path: String,
) -> anyhow::Result<()> {
    let range = format!("bytes={}-{}", chunk.start, chunk.end);
    let path = format!("{}.part{}", out_path, chunk.index);

    for attempt in 0..3 {
        let result = client
            .get(&url)
            .header("Range", &range)
            .send()
            .await?
            .bytes()
            .await;

        match result {
            Ok(bytes) => {
                let mut file = File::create(&path).await?;
                file.write_all(&bytes).await?;
                println!("Chunk {} done ({} bytes)", chunk.index, bytes.len());
                return Ok(());
            }
            Err(e) => {
                println!("Chunk {} attempt {} failed: {}", chunk.index, attempt + 1, e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    anyhow::bail!("Chunk {} failed after 3 attempts", chunk.index)
}

pub async fn assemble_chunks(out_path: &str, num_chunks: usize) -> anyhow::Result<()> {
    let mut output = File::create(out_path).await?;

    for i in 0..num_chunks {
        let part_path = format!("{}.part{}", out_path, i);
        let bytes = tokio::fs::read(&part_path).await?;
        output.write_all(&bytes).await?;
        tokio::fs::remove_file(&part_path).await?;
    }

    println!("Assembled {} chunk into {}", num_chunks, out_path);
    Ok(())
}