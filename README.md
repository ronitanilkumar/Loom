# Loom

A Rust-based model weight streaming agent built around the cold-start problem in inference infrastructure.

## The Problem

When a GPU container spins up, it needs model weights before it can serve requests. Fetching a multi-gigabyte file sequentially from object storage is slow. Cold starts measured in minutes are a real cost in production inference systems.

## How It Works

**Parallel chunk fetching**
Splits the file into 10 MB chunks and downloads them concurrently using HTTP Range requests. A semaphore-bounded worker pool caps concurrency to avoid rate limiting. Failed chunks retry with exponential backoff (2s, 4s, 8s).

**Content-addressable cache**
Hashes the URL with SHA-256 to produce a stable cache key. On a cache hit, hard-links the cached file to the output path instead of downloading. Hard-linking is nearly instantaneous regardless of file size since no data is copied.

**Streaming assembly**
Chunks are written to the final file using `tokio::io::copy`, streaming bytes directly between file handles without buffering each chunk in memory. Keeps memory usage flat regardless of file size.

**Control plane**
A lightweight HTTP server that tracks which nodes have which models cached. Nodes register after caching a model. Cold-starting containers query the control plane to find a local source before falling back to object storage. The registry uses `RwLock` so concurrent read queries do not block each other.

## Usage

```bash
# Download a model
cargo run -- --url https://huggingface.co/.../model.gguf --out model.gguf --workers 8

# Register a node
curl -X POST http://localhost:3000/register \
  -H "Content-Type: application/json" \
  -d '{"node": "node1", "model": "llama-2-7b"}'

# Locate a model
curl http://localhost:3000/locate/llama-2-7b
```

| Flag | Default | Description |
|------|---------|-------------|
| `--url` | required | Direct URL to the model weight file |
| `--out` | `output` | Local destination path |
| `--workers` | `10` | Max concurrent chunk downloads |

## Stack

Rust, tokio, reqwest, axum, sha2

## Known Limitations

The control plane runs in the same process as the downloader via `tokio::spawn`. They share a failure domain: if the process dies, the in-memory registry is lost. In a real deployment these would be separate services.
