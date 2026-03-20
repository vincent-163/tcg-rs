#!/bin/bash
# Test all SPEC2006int benchmarks in JIT mode (no profiling, no AOT)

set -e

SPEC_ROOT="/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64"
TCG_RS_ROOT="/data/Sync/all/projects/2026-02-11-cc-work/2026-03-tcg-rs"
RESULTS_DIR="$TCG_RS_ROOT/spec2006int-jit-results-$(date +%Y%m%d-%H%M%S)"
LOGS_DIR="$RESULTS_DIR/logs"

mkdir -p "$LOGS_DIR"

# SPEC2006int benchmarks
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

# Function to run single benchmark in JIT mode
run_benchmark() {
    local bench=$1
    local bench_num=$(echo "$bench" | cut -d. -f1)
    local bench_name=$(echo "$bench" | cut -d. -f2)
    local exe_dir="$SPEC_ROOT/benchspec/CPU2006/$bench/exe"
    local data_dir="$SPEC_ROOT/benchspec/CPU2006/$bench/data/ref/input"
    local exe="$exe_dir/${bench_name}_base.aarch64.Ofast.cogbt"

    local log_file="$LOGS_DIR/${bench_name}.log"

    echo "==> [$bench] Testing JIT mode..." | tee -a "$log_file"

    if [[ ! -f "$exe" ]]; then
        echo "ERROR: Executable not found: $exe" | tee -a "$log_file"
        echo "$bench,ERROR,Executable not found" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi

    # Get benchmark-specific command and args
    local cmd_args
    case "$bench_name" in
        perlbench)
            cmd_args="-I./lib checkspam.pl 2500 5 25 11 150 1 1 1 1"
            ;;
        bzip2)
            cmd_args="input.source 280"
            ;;
        gcc)
            cmd_args="166.i -o 166.s"
            ;;
        mcf)
            cmd_args="inp.in"
            ;;
        gobmk)
            cmd_args="--quiet --mode gtp < 13x13.tst"
            ;;
        hmmer)
            cmd_args="nph3.hmm swiss41"
            ;;
        sjeng)
            cmd_args="ref.txt"
            ;;
        libquantum)
            cmd_args="1397 8"
            ;;
        h264ref)
            cmd_args="-d foreman_ref_encoder_baseline.cfg"
            ;;
        omnetpp)
            cmd_args="omnetpp.ini"
            ;;
        astar)
            cmd_args="BigLakes2048.cfg"
            ;;
        xalancbmk)
            cmd_args="-v t5.xml xalanc.xsl"
            ;;
        specrand)
            cmd_args="1255440327 234923"
            ;;
        *)
            echo "ERROR: Unknown benchmark: $bench_name" | tee -a "$log_file"
            echo "$bench,ERROR,Unknown benchmark" >> "$RESULTS_DIR/summary.csv"
            return 1
            ;;
    esac

    # Run in JIT mode with timeout
    echo "  Running: $exe $cmd_args" | tee -a "$log_file"
    local start=$(date +%s)
    cd "$data_dir" 2>/dev/null || cd "$exe_dir"

    if timeout 300 "$TCG_RS_ROOT/target/release/tcg-aarch64" "$exe" $cmd_args \
        >> "$log_file" 2>&1; then
        local end=$(date +%s)
        local run_time=$((end - start))
        echo "  Run time: ${run_time}s" | tee -a "$log_file"
        echo "$bench,PASS,Success,${run_time}s" >> "$RESULTS_DIR/summary.csv"
        echo "==> [$bench] PASSED!" | tee -a "$log_file"
        return 0
    else
        local exit_code=$?
        local end=$(date +%s)
        local run_time=$((end - start))
        if [[ $exit_code -eq 124 ]]; then
            echo "ERROR: Timeout (300s)" | tee -a "$log_file"
            echo "$bench,TIMEOUT,Timeout after 300s,${run_time}s" >> "$RESULTS_DIR/summary.csv"
        else
            echo "ERROR: JIT run failed (exit code $exit_code)" | tee -a "$log_file"
            echo "$bench,FAIL,JIT run failed,${run_time}s" >> "$RESULTS_DIR/summary.csv"
        fi
        return 1
    fi
}

# Export function for parallel execution
export -f run_benchmark
export SPEC_ROOT TCG_RS_ROOT LOGS_DIR RESULTS_DIR

# Initialize summary CSV
echo "Benchmark,Status,Message,Run Time" > "$RESULTS_DIR/summary.csv"

# Run benchmarks serially for easier debugging
echo "==> Running benchmarks in SERIAL mode..."
for bench in "${BENCHMARKS[@]}"; do
    run_benchmark "$bench"
done

# Print summary
echo ""
echo "=========================================="
echo "SPEC2006int JIT Results Summary"
echo "=========================================="
cat "$RESULTS_DIR/summary.csv" | column -t -s,
echo ""
echo "Results saved to: $RESULTS_DIR"
echo "Logs: $LOGS_DIR"
