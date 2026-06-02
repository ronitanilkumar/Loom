#!/usr/bin/env bash
set -euo pipefail

BINARY="./target/release/loom"
URL="http://localhost:8080/model.gguf"
CACHE_DIR=".cache"

ms() { python3 -c "import time; print(int(time.time() * 1000))"; }

cleanup() {
  rm -f /tmp/loom_bench_out /tmp/loom_bench_out.part*
  rm -rf "$CACHE_DIR"
}

echo "=== Loom Benchmark ==="
echo "File: 2.69 GB model.gguf"
echo ""

# --- Run 1: Sequential (1 worker, cold cache) ---
cleanup
echo "[1/3] Sequential download (1 worker, cold cache)..."
T0=$(ms)
$BINARY --url "$URL" --out /tmp/loom_bench_out --workers 1 2>&1 | grep -E "^(File size|Split|Assembled|Cache|Done)" || true
T1=$(ms)
SEQ_MS=$((T1 - T0))
echo "    -> ${SEQ_MS} ms"
echo ""

# --- Run 2: Parallel (16 workers, cold cache) ---
cleanup
echo "[2/3] Parallel download (16 workers, cold cache)..."
T0=$(ms)
$BINARY --url "$URL" --out /tmp/loom_bench_out --workers 16 2>&1 | grep -E "^(File size|Split|Assembled|Cache|Done)" || true
T1=$(ms)
PAR_MS=$((T1 - T0))
echo "    -> ${PAR_MS} ms"
echo ""

# --- Run 3: Cache hit (hard-link, no download) ---
echo "[3/3] Cache hit (hard-link, no download)..."
T0=$(ms)
$BINARY --url "$URL" --out /tmp/loom_bench_out --workers 16 2>&1 | grep -E "^(File size|Split|Assembled|Cache|Done)" || true
T1=$(ms)
CACHE_MS=$((T1 - T0))
echo "    -> ${CACHE_MS} ms"
echo ""

# --- Summary ---
echo "=== Results ==="
printf "%-35s %8s ms\n" "Sequential (1 worker, cold):"    "$SEQ_MS"
printf "%-35s %8s ms\n" "Parallel (16 workers, cold):"    "$PAR_MS"
printf "%-35s %8s ms\n" "Cache hit (hard-link):"          "$CACHE_MS"
echo ""

SPEEDUP=$(python3 -c "print(f'{$SEQ_MS / $PAR_MS:.1f}x')")
CACHE_SPEEDUP=$(python3 -c "print(f'{$SEQ_MS / $CACHE_MS:.1f}x')")
echo "Parallel speedup over sequential:  ${SPEEDUP}"
echo "Cache hit speedup over sequential: ${CACHE_SPEEDUP}"

cleanup
