#!/bin/bash
# End-to-end script to run all SPEC2006int benchmarks in tcg-rs AOT mode
# Usage: ./run-spec2006int-aot.sh [parallel|serial]
# Default: parallel

set -e

MODE="${1:-parallel}"
SPEC_ROOT="/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64"
TCG_RS_ROOT="/data/Sync/all/projects/2026-02-11-cc-work/2026-03-tcg-rs"
RESULTS_DIR="$TCG_RS_ROOT/spec2006int-results-$(date +%Y%m%d-%H%M%S)"
PROFILE_DIR="$RESULTS_DIR/profiles"
AOT_DIR="$RESULTS_DIR/aot"
LOGS_DIR="$RESULTS_DIR/logs"

mkdir -p "$PROFILE_DIR" "$AOT_DIR" "$LOGS_DIR"

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

# Build tcg-rs release binaries
echo "==> Building tcg-rs release binaries..."
cd "$TCG_RS_ROOT"
cargo build --release --features llvm --bin tcg-aarch64
cargo build --release --features llvm -p tcg-aot

# Function to run single benchmark through 3-phase AOT pipeline
run_benchmark() {
    local bench=$1
    local bench_num=$(echo "$bench" | cut -d. -f1)
    local bench_name=$(echo "$bench" | cut -d. -f2)
    local exe_dir="$SPEC_ROOT/benchspec/CPU2006/$bench/exe"
    local data_dir="$SPEC_ROOT/benchspec/CPU2006/$bench/data/ref/input"
    local exe="$exe_dir/${bench_name}_base.aarch64.Ofast.cogbt"

    local profile_bin="$PROFILE_DIR/${bench_name}.profile.bin"
    local aot_o="$AOT_DIR/${bench_name}.aot.o"
    local aot_so="$AOT_DIR/${bench_name}.aot.so"
    local log_file="$LOGS_DIR/${bench_name}.log"

    echo "==> [$bench] Starting AOT pipeline..." | tee -a "$log_file"

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

    # Step 1: Profile run
    echo "  [1/3] Profiling..." | tee -a "$log_file"
    local start_profile=$(date +%s)
    cd "$data_dir" 2>/dev/null || cd "$exe_dir"

    if ! TCG_LLVM=1 TCG_PROFILE=1 TCG_PROFILE_OUT="$profile_bin" \
        "$TCG_RS_ROOT/target/release/tcg-aarch64" "$exe" $cmd_args \
        >> "$log_file" 2>&1; then
        local end_profile=$(date +%s)
        local profile_time=$((end_profile - start_profile))
        echo "ERROR: Profile run failed" | tee -a "$log_file"
        echo "$bench,FAIL,Profile failed,${profile_time}s" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi
    local end_profile=$(date +%s)
    local profile_time=$((end_profile - start_profile))
    echo "  Profile time: ${profile_time}s" | tee -a "$log_file"

    # Step 2: AOT compile
    echo "  [2/3] AOT compiling..." | tee -a "$log_file"
    local start_aot=$(date +%s)

    if ! "$TCG_RS_ROOT/target/release/tcg-aot" "$profile_bin" "$exe" -o "$aot_o" \
        >> "$log_file" 2>&1; then
        local end_aot=$(date +%s)
        local aot_time=$((end_aot - start_aot))
        echo "ERROR: AOT compile failed" | tee -a "$log_file"
        echo "$bench,FAIL,AOT compile failed,${profile_time}s,${aot_time}s" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi

    if ! cc -shared -o "$aot_so" "$aot_o" >> "$log_file" 2>&1; then
        local end_aot=$(date +%s)
        local aot_time=$((end_aot - start_aot))
        echo "ERROR: AOT linking failed" | tee -a "$log_file"
        echo "$bench,FAIL,AOT link failed,${profile_time}s,${aot_time}s" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi

    local end_aot=$(date +%s)
    local aot_time=$((end_aot - start_aot))
    echo "  AOT compile time: ${aot_time}s" | tee -a "$log_file"

    # Step 3: AOT run
    echo "  [3/3] Running with AOT..." | tee -a "$log_file"
    local start_run=$(date +%s)

    if ! TCG_LLVM=1 TCG_AOT="$aot_so" \
        "$TCG_RS_ROOT/target/release/tcg-aarch64" "$exe" $cmd_args \
        >> "$log_file" 2>&1; then
        local end_run=$(date +%s)
        local run_time=$((end_run - start_run))
        echo "ERROR: AOT run failed" | tee -a "$log_file"
        echo "$bench,FAIL,AOT run failed,${profile_time}s,${aot_time}s,${run_time}s" >> "$RESULTS_DIR/summary.csv"
        return 1
    fi

    local end_run=$(date +%s)
    local run_time=$((end_run - start_run))
    local total_time=$((profile_time + aot_time + run_time))

    echo "  AOT run time: ${run_time}s" | tee -a "$log_file"
    echo "  Total time: ${total_time}s" | tee -a "$log_file"
    echo "$bench,PASS,Success,${profile_time}s,${aot_time}s,${run_time}s,${total_time}s" >> "$RESULTS_DIR/summary.csv"
    echo "==> [$bench] Completed successfully!" | tee -a "$log_file"

    return 0
}

# Export function for parallel execution
export -f run_benchmark
export SPEC_ROOT TCG_RS_ROOT PROFILE_DIR AOT_DIR LOGS_DIR RESULTS_DIR

# Initialize summary CSV
echo "Benchmark,Status,Message,Profile Time,AOT Time,Run Time,Total Time" > "$RESULTS_DIR/summary.csv"

# Run benchmarks
if [[ "$MODE" == "parallel" ]]; then
    echo "==> Running benchmarks in PARALLEL mode..."
    printf '%s\n' "${BENCHMARKS[@]}" | xargs -P $(nproc) -I {} bash -c 'run_benchmark "$@"' _ {}
else
    echo "==> Running benchmarks in SERIAL mode..."
    for bench in "${BENCHMARKS[@]}"; do
        run_benchmark "$bench"
    done
fi

# Print summary
echo ""
echo "=========================================="
echo "SPEC2006int AOT Results Summary"
echo "=========================================="
cat "$RESULTS_DIR/summary.csv" | column -t -s,
echo ""
echo "Results saved to: $RESULTS_DIR"
echo "Logs: $LOGS_DIR"
echo "Profiles: $PROFILE_DIR"
echo "AOT objects: $AOT_DIR"

# Log to SPEC2006LOG.md
LOG_FILE="$TCG_RS_ROOT/SPEC2006LOG.md"
echo "" >> "$LOG_FILE"
echo "## $(date '+%Y-%m-%d %H:%M:%S')" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"
echo "- Commit: $(git rev-parse HEAD)" >> "$LOG_FILE"
echo "- Mode: AOT" >> "$LOG_FILE"
passed_count=$(grep -c ",PASS," "$RESULTS_DIR/summary.csv" || echo "0")
failed_count=$(grep -c ",FAIL," "$RESULTS_DIR/summary.csv" || echo "0")
echo "- Passed: $passed_count" >> "$LOG_FILE"
echo "- Failed: $failed_count" >> "$LOG_FILE"
if [[ $failed_count -gt 0 ]]; then
    failed_tests=$(grep ",FAIL," "$RESULTS_DIR/summary.csv" | cut -d',' -f1 | tr '\n' ' ')
    echo "- Failed tests: $failed_tests" >> "$LOG_FILE"
fi
echo "- Results: $RESULTS_DIR" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"
echo "Logged to SPEC2006LOG.md"
