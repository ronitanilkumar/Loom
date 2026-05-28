use clap::Parser;
mod fetcher;
use fetcher::download_chunk;
use reqwest;
use std::sync::Arc;
use tokio::sync::Semaphore;
mod cache;
mod control;

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

    let app = control::router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Control plane listening on port 3000");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let cache = cache::Cache::new(".cache");
    let key = cache.key(&args.url);

    if cache.exists(&key) {
        cache.link(&key, &args.out)?;
        println!("Done.");
    } else {
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
        let num_chunks = chunks.len();

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

        fetcher::assemble_chunks(&args.out, num_chunks).await?;
        cache.store(&key, &args.out)?;
    }

    tokio::signal::ctrl_c().await?;
    println!("Shutting down.");
    Ok(())
}
