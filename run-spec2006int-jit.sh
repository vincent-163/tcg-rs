#!/usr/bin/env bash
# Run all SPEC2006int benchmarks in tcg-rs JIT mode with profiling
# Usage: ./run-spec2006int-jit.sh [parallel|serial]
# Default: parallel
# Features:
#   - Logs always written to files, not printed to console
#   - Per-testcase progress logging
#   - Profile collection to cache/ directory

set -euo pipefail

MODE="${1:-parallel}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKTREE="${WORKTREE:-$SCRIPT_DIR}"
SPEC_ROOT="${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}"
TAG="${TAG:-$(date +%Y%m%d%H%M%S)}"

OUT_DIR="$WORKTREE/spec-logs"
RESULTS_DIR="$WORKTREE/spec2006int-jit-results-$TAG"
LOGS_DIR="$RESULTS_DIR/logs"
MAIN_LOG="$RESULTS_DIR/run.log"
CACHE_DIR="$WORKTREE/cache"

BENCHMARKS=(
    "400.perlbench"
    "401.bzip2"
    "403.gcc"
    "429.mcf"
    "445.gobmk"
    "456.hmmer"
    "458.sjeng"
    "462.libquantum"
    "464.h264ref"
    "471.omnetpp"
    "473.astar"
    "483.xalancbmk"
    "999.specrand"
)

mkdir -p "$OUT_DIR" "$LOGS_DIR" "$CACHE_DIR/profiles"

# Redirect all output to main log file
exec > >(tee -a "$MAIN_LOG")
exec 2>&1

echo "=========================================="
echo "SPEC2006 INT JIT Mode Test Runner"
echo "=========================================="
echo "Mode: $MODE"
echo "Tag: $TAG"
echo "Worktree: $WORKTREE"
echo "SPEC_ROOT: $SPEC_ROOT"
echo "Output: $OUT_DIR"
echo "Results: $RESULTS_DIR"
echo "Cache: $CACHE_DIR"
echo "Benchmarks: ${#BENCHMARKS[@]}"
echo "=========================================="

# Generate JIT config with profile support
echo ""
echo "==> Generating JIT config..."
JIT_CFG=$("$WORKTREE/tools/spec/write-runspec-configs.sh" "$TAG" 2>&1 | grep "jit.cfg$") || {
    echo "ERROR: Failed to generate JIT config"
    exit 1
}
echo "JIT config: $JIT_CFG"

# Initialize summary
echo "benchmark,status,exit_code,profile_file" > "$RESULTS_DIR/summary.csv"

# Function to resolve bench name for profile file
resolve_profile_name() {
    case "$1" in
        403.gcc) echo "gcc" ;;
        483.xalancbmk) echo "Xalan" ;;
        *.*) echo "${1#*.}" ;;
        *) echo "$1" ;;
    esac
}

# Function to run single benchmark with profiling
run_benchmark() {
    local bench=$1
    local bench_name=${bench#*.}
    local profile_name=$(resolve_profile_name "$bench")
    local log_file="$LOGS_DIR/${bench_name}.log"
    local profile_file="$CACHE_DIR/profiles/${profile_name}.profile.bin"
    
    echo ""
    echo "==> [$bench] Starting JIT run with profiling..."
    local start_time=$(date +%s)
    
    # Set environment variables for profiling
    export TCG_PROFILE=1
    export TCG_PROFILE_OUT="$profile_file"
    export TCG_PROFILE_MODE=${TCG_PROFILE_MODE:-all}
    
    if "$WORKTREE/tools/spec/run-runspec-tcgrs.sh" \
        --config="$JIT_CFG" \
        --size=ref \
        --iterations=1 \
        "$bench" > "$log_file" 2>&1; then
        local end_time=$(date +%s)
        local elapsed=$((end_time - start_time))
        echo "==> [$bench] PASSED (${elapsed}s)"
        echo "$bench,PASS,0,$profile_file" >> "$RESULTS_DIR/summary.csv"
        
        # Check if profile was generated
        if [[ -f "$profile_file" ]]; then
            local profile_size=$(stat -c%s "$profile_file" 2>/dev/null || echo "0")
            echo "    Profile: $profile_file (${profile_size} bytes)"
        else
            echo "    Warning: Profile not generated"
        fi
        return 0
    else
        local exit_code=$?
        local end_time=$(date +%s)
        local elapsed=$((end_time - start_time))
        echo "==> [$bench] FAILED (exit code: $exit_code, ${elapsed}s)"
        echo "$bench,FAIL,$exit_code,$profile_file" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi
}

export -f run_benchmark resolve_profile_name
export WORKTREE SPEC_ROOT JIT_CFG LOGS_DIR RESULTS_DIR MAIN_LOG CACHE_DIR

# Run benchmarks
echo ""
if [[ "$MODE" == "parallel" ]]; then
    echo "==> Running benchmarks in PARALLEL mode (max $(nproc) concurrent)..."
    printf '%s\n' "${BENCHMARKS[@]}" | xargs -P $(nproc) -I {} bash -c 'run_benchmark "$@"' _ {}
else
    echo "==> Running benchmarks in SERIAL mode..."
    for bench in "${BENCHMARKS[@]}"; do
        run_benchmark "$bench"
    done
fi

# Wait for runspec to complete
echo ""
echo "==> Waiting for runspec processes to complete..."
sleep 10

# Generate final status
echo ""
echo "=========================================="
echo "Generating status report..."
echo "=========================================="

STATUS_OUTPUT=$("$WORKTREE/tools/spec/specint-status.sh" "$TAG" 2>/dev/null || echo "")
echo "$STATUS_OUTPUT" | tee "$RESULTS_DIR/status.txt"

# Identify failed tests and collect profiles
echo ""
echo "=========================================="
echo "Test Results Analysis"
echo "=========================================="

passed_tests=()
failed_tests=()
profile_files=()

for bench in "${BENCHMARKS[@]}"; do
    # Check if benchmark has compare output (success indicator)
    run_dirs=$(find "$SPEC_ROOT/benchspec/CPU2006/$bench/run" -maxdepth 1 -type d -name "run_base_ref_aarch64.Ofast.tcgrs.$TAG.jit.*" 2>/dev/null | sort || true)
    
    found_ok=false
    for run_dir in $run_dirs; do
        if [[ -f "$run_dir/compare.out" ]] || [[ -f "$run_dir/compare.stdout" ]] || [[ -f "$run_dir/compare.rerun.stdout" ]]; then
            found_ok=true
            break
        fi
    done
    
    profile_name=$(resolve_profile_name "$bench")
    profile_file="$CACHE_DIR/profiles/${profile_name}.profile.bin"
    
    if [[ "$found_ok" == "true" ]]; then
        passed_tests+=("$bench")
        echo "✓ $bench - PASSED"
        if [[ -f "$profile_file" ]]; then
            profile_files+=("$profile_file")
            local profile_size=$(stat -c%s "$profile_file" 2>/dev/null || echo "0")
            echo "    Profile: ${profile_name}.profile.bin (${profile_size} bytes)"
        fi
    else
        failed_tests+=("$bench")
        echo "✗ $bench - FAILED"
    fi
done

echo ""
echo "=========================================="
echo "Profile Collection Summary"
echo "=========================================="
echo "Total profiles collected: ${#profile_files[@]}"
echo "Profile directory: $CACHE_DIR/profiles/"

# List all collected profiles
echo ""
echo "Collected profiles:"
ls -lh "$CACHE_DIR/profiles/" 2>/dev/null | tail -n +2 || echo "  No profiles found"

# Save final summary
cat > "$RESULTS_DIR/summary.txt" <<EOF
SPEC2006 INT JIT Mode Results
=============================
Tag: $TAG
Date: $(date)
Mode: $MODE

Benchmarks Tested: ${#BENCHMARKS[@]}
Passed: ${#passed_tests[@]}
Failed: ${#failed_tests[@]}
Profiles Collected: ${#profile_files[@]}

Status Report:
$STATUS_OUTPUT

Passed Benchmarks:
$(printf '%s\n' "${passed_tests[@]}")

Failed Benchmarks:
$(printf '%s\n' "${failed_tests[@]}")

Profile Files:
$(printf '%s\n' "${profile_files[@]}")

Detailed Results:
$(cat "$RESULTS_DIR/summary.csv")
EOF

echo ""
echo "=========================================="
echo "Results Summary"
echo "=========================================="
echo "Passed: ${#passed_tests[@]} / ${#BENCHMARKS[@]}"
echo "Failed: ${#failed_tests[@]} / ${#BENCHMARKS[@]}"
if [[ ${#failed_tests[@]} -gt 0 ]]; then
    echo "Failed tests: ${failed_tests[*]}"
fi

echo ""
echo "Results saved to: $RESULTS_DIR"
echo "  - Main log: $MAIN_LOG"
echo "  - Benchmark logs: $LOGS_DIR/"
echo "  - Summary: $RESULTS_DIR/summary.txt"
echo "  - Status: $RESULTS_DIR/status.txt"
echo "  - Profiles: $CACHE_DIR/profiles/"

# Log to SPEC2006LOG.md
LOG_FILE="$WORKTREE/SPEC2006LOG.md"
echo "" >> "$LOG_FILE"
echo "## $(date '+%Y-%m-%d %H:%M:%S')" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"
echo "- Commit: $(git rev-parse HEAD)" >> "$LOG_FILE"
echo "- Mode: JIT (with profiling)" >> "$LOG_FILE"
echo "- Passed: ${#passed_tests[@]}" >> "$LOG_FILE"
echo "- Failed: ${#failed_tests[@]}" >> "$LOG_FILE"
if [[ ${#failed_tests[@]} -gt 0 ]]; then
    echo "- Failed tests: ${failed_tests[*]}" >> "$LOG_FILE"
fi
echo "- Profiles collected: ${#profile_files[@]}" >> "$LOG_FILE"
echo "- Results: $RESULTS_DIR" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"
echo "Logged to SPEC2006LOG.md"

if [[ ${#failed_tests[@]} -gt 0 ]]; then
    exit 1
else
    exit 0
fi
