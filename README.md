# Loom

A Rust-based model weight streaming agent built around the cold-start problem in inference infrastructure.

## The Problem

When a GPU container spins up, it needs model weights before it can serve requests. Fetching a multi-gigabyte file sequentially from object storage is slow. Cold starts measured in minutes are a real cost in production inference systems.

## How It Works

**Parallel chunk fetching**
Splits the file into 10MB chunks and downloads them concurrently using HTTP Range requests. A semaphore-bounded worker pool caps concurrency to avoid rate limiting. Failed chunks retry up to 3 times automatically.

**Content-addressable cache**
Hashes the URL with SHA-256 to produce a stable cache key. On a cache hit, hard-links the cached file to the output path instead of downloading. Hard-linking is nearly instantaneous regardless of file size since no data is copied.

**Control plane**
A lightweight HTTP server that tracks which nodes have which models cached. Nodes register after caching a model. Cold-starting containers query the control plane to find a local source before falling back to object storage.

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

## Stack

Rust, tokio, reqwest, axum, sha2
