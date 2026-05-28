use clap::Parser;
mod fetcher;
use reqwest;
use std::sync::Arc;
use fetcher::download_chunk;
use tokio::sync::Semaphore;

#[derive(Parser, Debug)]
#[command(name = "loom", about = "High-speed model weight fetcher")]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long, default_value = "output")]
    out: String,

    #[arg(short, long, default_value_t = 10)]
    workers: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    let client = reqwest::Client::new();

    let res = client.head(&args.url).send().await?;
    let file_size = res
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .expect("Could not determine file size");

    println!("File size: {} bytes", file_size);

    let chunk_size = 10 * 1024 * 1024;
    let chunks = fetcher::split_into_chunks(file_size, chunk_size);
    println!("Split into {} chunks", chunks.len());

    let client = Arc::new(client);
    let semaphore = Arc::new(Semaphore::new(args.workers));
    let mut handles = vec![];

    for chunk in chunks {
        let client = Arc::clone(&client);
        let url = args.url.clone();
        let out = args.out.clone();
        let sem = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            download_chunk(client, url, chunk, out).await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    println!("All chunks downloaded.");

    Ok(())
}
