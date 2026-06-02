# Loom

Concurrent HTTP Range request fetcher for GPU model weights. Built around the cold-start problem in inference containers.

## The Problem

When a GPU container spins up, it needs model weights before it can serve requests. A sequential download pays full round-trip latency per chunk. 270 serial requests against S3 at 150 ms each is 40 seconds of waiting before any bytes transfer. Loom pipelines those requests across a worker pool so the latency overlaps.

## How It Works

**Parallel chunk fetching**
Splits the file into 10 MB chunks and fetches them concurrently using HTTP Range requests. A semaphore-bounded worker pool caps concurrency to avoid rate limiting. Failed chunks retry with exponential backoff (2s, 4s, 8s).

**Content-addressable cache**
Hashes the URL with SHA-256 to produce a stable cache key. On a cache hit, hard-links the cached file to the output path. No data is copied, so a 2.7 GB model is file-accessible in ~40 ms.

**Streaming assembly**
Chunks are streamed into the final file via `tokio::io::copy` without buffering in memory. Memory usage stays flat regardless of file size or worker count.

**Control plane**
A lightweight HTTP server tracks which nodes have which models cached. Cold-starting containers query it to find a peer source before falling back to object storage. The registry uses `RwLock` so concurrent reads do not block each other.

## Performance

Measured on a 2.7 GB model file over localhost.

| Scenario | Time | Note |
|---|---|---|
| Sequential (1 worker) | 11.8 s | baseline |
| Parallel (16 workers) | 10.6 s | I/O already saturated on localhost |
| Cache hit (hard-link) | 41 ms | filesystem op, no download |

On localhost a single connection already saturates local I/O, so parallel gain is small. The design targets object storage, where each request carries real network latency. 16 workers pipelining 270 chunks against S3 reduces roughly 40 s of serial wait to 2-3 s. Cloud validation is the next step.

## Usage

```bash
# Download a model
cargo run --release -- --url https://huggingface.co/.../model.gguf --out model.gguf --workers 16

# Keep the control plane running after download
cargo run --release -- --url https://huggingface.co/.../model.gguf --out model.gguf --serve

# Register a node with the control plane
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
| `--serve` | false | Keep control plane running after download |

## Stack

Rust, tokio, reqwest, axum, sha2

## Known Limitations

- Control plane runs in the same process as the downloader via `tokio::spawn`. If the process dies, the registry is lost. In production these would be separate services.
- Benchmarks are localhost only. Cloud validation against S3/GCS with real network latency is needed to quantify actual cold-start improvement.
