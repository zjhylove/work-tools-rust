#!/usr/bin/env bash
# autoresearch.sh — benchmark harness for work-tools-rust optimization
#
# Primary metric:  compile_time_ms  (cold cargo build --workspace)
# Secondary:       test_pass_count  (unit tests passing, excluding integration deps)
#                  clippy_warnings   (clippy warning count)

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== autoresearch.sh: work-tools-rust benchmark ==="
echo "workspace: $WORKSPACE_ROOT"

# ── 1. Compile time (primary metric) ──
# Measure wall-clock ms for cargo build --workspace in release profile.
# We use touch to invalidate one leaf crate, then time the incremental build
# from that point. A full clean build is too noisy for iteration.
#
# Strategy: touch a high-level leaf source file so the entire dependency
# tree of workspace crates re-checks, then measure the build.
# This is deterministic and reproducible across runs.

# First, ensure we're built
cargo build --workspace 2>/dev/null || true

# Touch the plugin-api lib.rs to trigger re-compilation of dependents
touch "$WORKSPACE_ROOT/shared/plugin-api/src/lib.rs"

# Measure compile time in ms
COMPILE_START=$(date +%s%N)
cargo build --workspace 2>&1
COMPILE_END=$(date +%s%N)

COMPILE_MS=$(( (COMPILE_END - COMPILE_START) / 1000000 ))
echo "METRIC compile_time_ms=$COMPILE_MS"

# ── 2. Test pass count (secondary) ──
# Count unit tests that pass, excluding redis-client (requires running server)
TEST_OUTPUT=$(cargo test --workspace --exclude redis-client 2>&1)
TEST_PASS=$(echo "$TEST_OUTPUT" | grep -oP '\d+ passed' | awk '{s+=$1}END{print s+0}')
echo "METRIC test_pass_count=$TEST_PASS"

# ── 3. Clippy warnings (secondary) ──
CLIPPY_OUTPUT=$(cargo clippy --workspace 2>&1)
CLIPPY_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -c "warning:" || true)
echo "METRIC clippy_warnings=$CLIPPY_COUNT"

echo "=== benchmark complete ==="
