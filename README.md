# Loom

Parallelizes GPU model weight loading by issuing concurrent HTTP Range requests against object storage, eliminating the sequential bottleneck in cold-start inference containers.

## The Problem

When a GPU container spins up, it needs model weights before it can serve requests. A sequential download from object storage pays full round-trip latency for every chunk — 270 serial requests against S3 at 150 ms each is 40 seconds of pure waiting before accounting for transfer time. Loom pipelines those requests across a worker pool so the latency is paid once, not 270 times.

## How It Works

**Parallel chunk fetching**
Splits the file into 10 MB chunks and fetches them concurrently using HTTP Range requests. A semaphore-bounded worker pool caps concurrency to avoid rate limiting. Failed chunks retry with exponential backoff (2s, 4s, 8s).

**Content-addressable cache**
Hashes the URL with SHA-256 to produce a stable cache key. On a cache hit, hard-links the cached file to the output path — file-accessible in ~40 ms for a 2.7 GB model regardless of file size, since no data is copied.

**Streaming assembly**
Chunks are streamed into the final file via `tokio::io::copy` without buffering in memory. Memory usage stays flat regardless of file size or worker count.

**Control plane**
A lightweight HTTP server tracks which nodes have which models cached. Cold-starting containers query it to find a peer source before falling back to object storage. The registry uses `RwLock` so concurrent read queries don't block each other.

## Performance

Measured on a 2.7 GB model file. The meaningful speedup is on object storage with real network latency — concurrent Range requests eliminate 270 serial round-trips.

| Scenario | Time | Note |
|---|---|---|
| Sequential (1 worker) | 11.8 s | localhost baseline |
| Parallel (16 workers) | 10.6 s | localhost — I/O already saturated |
| Cache hit (hard-link) | 41 ms | filesystem op, no download |

On localhost the parallel gain is minimal because a single connection already saturates local I/O. The architecture targets cloud object storage, where each request carries real latency: 16 workers pipelining 270 chunks against S3 reduces ~40 s of serial wait to ~2-3 s.

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

- **Control plane is in-process.** The registry lives in the same process as the downloader via `tokio::spawn`. If the process dies, the registry is lost. In production these would be separate services.
- **Performance numbers are localhost-only.** Cloud object storage validation (S3/GCS with real network latency) is the next step to quantify the actual cold-start improvement.
