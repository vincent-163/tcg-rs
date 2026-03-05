//! TCG profile decoder: reads and displays profile.bin data.
//!
//! Usage:
//!   tcg-profile-decode <profile.bin> [options]
//!
//! Options:
//!   --sort-count    Sort by execution count (default)
//!   --sort-offset   Sort by file offset
//!   --min-count N   Only show entries with count >= N
//!   --indirect      Only show indirect targets
//!   --stats         Show summary statistics only

use std::env;
use std::path::Path;
use std::process;

use tcg_exec::profile::{ProfileData, ProfileEntry};

const USAGE: &str = "\
usage: tcg-profile-decode <profile.bin> [options]

Reads and displays TCG profile data collected during a
profiling run (TCG_PROFILE=1).

Options:
  --sort-count    Sort by execution count (default)
  --sort-offset   Sort by file offset
  --min-count N   Only show entries with count >= N
  --indirect      Only show indirect targets
  --stats         Show summary statistics only

Examples:
  tcg-profile-decode profile.bin
  tcg-profile-decode profile.bin --min-count 1000
  tcg-profile-decode profile.bin --indirect
  tcg-profile-decode profile.bin --stats";

#[derive(Debug)]
struct Options {
    profile_path: String,
    sort_by_count: bool,
    min_count: u64,
    indirect_only: bool,
    stats_only: bool,
}

impl Options {
    fn parse() -> Self {
        let args: Vec<String> = env::args().collect();

        // Check for help first
        if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
            println!("{}", USAGE);
            process::exit(0);
        }

        if args.len() < 2 {
            eprintln!("{}", USAGE);
            process::exit(1);
        }

        let profile_path = args[1].clone();
        let mut sort_by_count = true;
        let mut min_count = 0;
        let mut indirect_only = false;
        let mut stats_only = false;

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--sort-count" => sort_by_count = true,
                "--sort-offset" => sort_by_count = false,
                "--min-count" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("error: --min-count requires argument");
                        process::exit(1);
                    }
                    min_count = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("error: invalid count: {}", args[i]);
                        process::exit(1);
                    });
                }
                "--indirect" => indirect_only = true,
                "--stats" => stats_only = true,
                "--help" | "-h" => {
                    println!("{}", USAGE);
                    process::exit(0);
                }
                other => {
                    eprintln!("error: unknown option: {}", other);
                    eprintln!("{}", USAGE);
                    process::exit(1);
                }
            }
            i += 1;
        }

        Self {
            profile_path,
            sort_by_count,
            min_count,
            indirect_only,
            stats_only,
        }
    }
}

fn format_count(count: u64) -> String {
    if count >= 1_000_000_000 {
        format!("{:.2}B", count as f64 / 1_000_000_000.0)
    } else if count >= 1_000_000 {
        format!("{:.2}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.2}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn print_stats(data: &ProfileData) {
    let total_entries = data.entries.len();
    let indirect_count = data.entries.iter()
        .filter(|e| e.indirect_target)
        .count();
    let total_execs: u64 = data.entries.iter()
        .map(|e| e.exec_count)
        .sum();
    let min_count = data.entries.iter()
        .map(|e| e.exec_count)
        .min()
        .unwrap_or(0);
    let max_count = data.entries.iter()
        .map(|e| e.exec_count)
        .max()
        .unwrap_or(0);
    let avg_count = if total_entries > 0 {
        total_execs / total_entries as u64
    } else {
        0
    };

    println!("Profile Statistics");
    println!("==================");
    println!("Threshold:        {}", data.threshold);
    println!("Total TBs:        {}", total_entries);
    println!("Indirect targets: {}", indirect_count);
    println!("Total executions: {} ({})",
        total_execs, format_count(total_execs));
    println!("Min count:        {} ({})",
        min_count, format_count(min_count));
    println!("Max count:        {} ({})",
        max_count, format_count(max_count));
    println!("Avg count:        {} ({})",
        avg_count, format_count(avg_count));
}

fn print_entries(entries: &[ProfileEntry]) {
    println!("{:>18} {:>12} {:>10} {:>8}",
        "File Offset", "Exec Count", "Formatted", "Indirect");
    println!("{}", "-".repeat(60));

    for e in entries {
        println!("{:#18x} {:>12} {:>10} {:>8}",
            e.file_offset,
            e.exec_count,
            format_count(e.exec_count),
            if e.indirect_target { "yes" } else { "" });
    }
}

fn main() {
    let opts = Options::parse();

    let path = Path::new(&opts.profile_path);
    let data = ProfileData::load(path).unwrap_or_else(|e| {
        eprintln!("error: failed to load profile: {}", e);
        process::exit(1);
    });

    if opts.stats_only {
        print_stats(&data);
        return;
    }

    // Filter entries
    let mut entries: Vec<ProfileEntry> = data.entries.iter()
        .filter(|e| e.exec_count >= opts.min_count)
        .filter(|e| !opts.indirect_only || e.indirect_target)
        .copied()
        .collect();

    // Sort entries
    if opts.sort_by_count {
        entries.sort_by(|a, b| b.exec_count.cmp(&a.exec_count));
    } else {
        entries.sort_by_key(|e| e.file_offset);
    }

    println!("Profile: {}", opts.profile_path);
    println!("Threshold: {}", data.threshold);
    println!("Showing {} of {} entries",
        entries.len(), data.entries.len());
    println!();

    print_entries(&entries);

    println!();
    println!("Summary:");
    println!("  Total TBs: {}", data.entries.len());
    println!("  Displayed: {}", entries.len());
    println!("  Total executions: {}",
        entries.iter().map(|e| e.exec_count).sum::<u64>());
}
