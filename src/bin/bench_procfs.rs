#!/usr/bin/env rust
//! Benchmark script to measure procfs read times
//! Run with: cargo run --bin bench_procfs
//!
//! This benchmark is Linux-only since procfs is only available on Linux.

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This benchmark is only available on Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    use std::time::Instant;
    println!("Benchmarking procfs read operations...\n");

    // Get all processes
    let start = Instant::now();
    let all_procs: Vec<_> = procfs::process::all_processes()
        .unwrap()
        .filter_map(|p| p.ok())
        .collect();
    let all_procs_time = start.elapsed();
    println!(
        "all_processes() + collect: {:>8.2}ms ({} processes)",
        all_procs_time.as_secs_f64() * 1000.0,
        all_procs.len()
    );

    // Benchmark individual operations
    let mut stat_time = std::time::Duration::ZERO;
    let mut status_time = std::time::Duration::ZERO;
    let mut cmdline_time = std::time::Duration::ZERO;
    let mut exe_time = std::time::Duration::ZERO;
    let mut statm_time = std::time::Duration::ZERO;
    let mut io_time = std::time::Duration::ZERO;
    let mut cgroups_time = std::time::Duration::ZERO;
    let mut tasks_time = std::time::Duration::ZERO;

    let mut stat_count = 0;
    let mut status_count = 0;
    let mut uid_count = 0;
    let mut cmdline_count = 0;
    let mut exe_count = 0;
    let mut statm_count = 0;
    let mut io_count = 0;
    let mut cgroups_count = 0;
    let mut tasks_count = 0;
    let mut total_tasks = 0;
    let mut uid_time = std::time::Duration::ZERO;
    let mut maps_time = std::time::Duration::ZERO;
    let mut maps_count = 0;

    for proc in &all_procs {
        // stat()
        let start = Instant::now();
        if proc.stat().is_ok() {
            stat_time += start.elapsed();
            stat_count += 1;
        }

        // status()
        let start = Instant::now();
        if proc.status().is_ok() {
            status_time += start.elapsed();
            status_count += 1;
        }

        // uid() - fast alternative to status() for getting UID
        let start = Instant::now();
        if proc.uid().is_ok() {
            uid_time += start.elapsed();
            uid_count += 1;
        }

        // cmdline()
        let start = Instant::now();
        if proc.cmdline().is_ok() {
            cmdline_time += start.elapsed();
            cmdline_count += 1;
        }

        // exe()
        let start = Instant::now();
        if proc.exe().is_ok() {
            exe_time += start.elapsed();
            exe_count += 1;
        }

        // statm()
        let start = Instant::now();
        if proc.statm().is_ok() {
            statm_time += start.elapsed();
            statm_count += 1;
        }

        // io()
        let start = Instant::now();
        if proc.io().is_ok() {
            io_time += start.elapsed();
            io_count += 1;
        }

        // cgroups()
        let start = Instant::now();
        if proc.cgroups().is_ok() {
            cgroups_time += start.elapsed();
            cgroups_count += 1;
        }

        // tasks()
        let start = Instant::now();
        if let Ok(tasks) = proc.tasks() {
            let task_list: Vec<_> = tasks.filter_map(|t| t.ok()).collect();
            total_tasks += task_list.len();
            tasks_time += start.elapsed();
            tasks_count += 1;
        }

        // maps (for deleted lib check)
        let start = Instant::now();
        if let Ok(maps) = proc.maps() {
            let _: Vec<_> = maps.into_iter().collect();
            maps_time += start.elapsed();
            maps_count += 1;
        }
    }

    println!("\nPer-process file read times (total across all processes):");
    println!("─────────────────────────────────────────────────────────");
    println!(
        "stat():     {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        stat_time.as_secs_f64() * 1000.0,
        stat_count,
        stat_time.as_secs_f64() * 1_000_000.0 / stat_count as f64
    );
    println!(
        "status():   {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        status_time.as_secs_f64() * 1000.0,
        status_count,
        status_time.as_secs_f64() * 1_000_000.0 / status_count as f64
    );
    println!(
        "uid():      {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← FAST alternative!",
        uid_time.as_secs_f64() * 1000.0,
        uid_count,
        uid_time.as_secs_f64() * 1_000_000.0 / uid_count as f64
    );
    println!(
        "cmdline():  {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        cmdline_time.as_secs_f64() * 1000.0,
        cmdline_count,
        cmdline_time.as_secs_f64() * 1_000_000.0 / cmdline_count as f64
    );
    println!(
        "exe():      {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        exe_time.as_secs_f64() * 1000.0,
        exe_count,
        exe_time.as_secs_f64() * 1_000_000.0 / exe_count as f64
    );
    println!(
        "statm():    {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        statm_time.as_secs_f64() * 1000.0,
        statm_count,
        statm_time.as_secs_f64() * 1_000_000.0 / statm_count as f64
    );
    println!(
        "io():       {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        io_time.as_secs_f64() * 1000.0,
        io_count,
        io_time.as_secs_f64() * 1_000_000.0 / io_count as f64
    );
    println!(
        "cgroups():  {:>8.2}ms ({:>4} calls, {:>6.2}µs avg)",
        cgroups_time.as_secs_f64() * 1000.0,
        cgroups_count,
        cgroups_time.as_secs_f64() * 1_000_000.0 / cgroups_count as f64
    );
    println!(
        "tasks():    {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) [{} total tasks]",
        tasks_time.as_secs_f64() * 1000.0,
        tasks_count,
        tasks_time.as_secs_f64() * 1_000_000.0 / tasks_count as f64,
        total_tasks
    );
    println!(
        "maps():     {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← for deleted lib check",
        maps_time.as_secs_f64() * 1000.0,
        maps_count,
        maps_time.as_secs_f64() * 1_000_000.0 / maps_count as f64
    );

    let total = stat_time
        + status_time
        + uid_time
        + cmdline_time
        + exe_time
        + statm_time
        + io_time
        + cgroups_time
        + tasks_time
        + maps_time;
    println!("─────────────────────────────────────────────────────────");
    println!("TOTAL:      {:>8.2}ms", total.as_secs_f64() * 1000.0);

    // Now measure scan_processes()-like loop
    println!("\n\nFull scan simulation (like htop-rs scan_processes):");
    println!("─────────────────────────────────────────────────────────");

    let start = Instant::now();
    let mut full_scan_count = 0;
    for proc in procfs::process::all_processes()
        .unwrap()
        .filter_map(|p| p.ok())
    {
        let _ = proc.stat();
        let _ = proc.status();
        let _ = proc.cmdline();
        let _ = proc.exe();
        let _ = proc.statm();
        full_scan_count += 1;
        // Skip io(), cgroups(), tasks() - they're conditional in htop-rs
    }
    let full_scan_time = start.elapsed();
    println!(
        "Full scan (stat+status+cmdline+exe+statm): {:>8.2}ms ({} procs)",
        full_scan_time.as_secs_f64() * 1000.0,
        full_scan_count
    );

    // Minimal scan (just stat) - collect PIDs for later use
    let start = Instant::now();
    let pids: Vec<i32> = procfs::process::all_processes()
        .unwrap()
        .filter_map(|p| p.ok())
        .filter_map(|p| p.stat().ok().map(|s| s.pid))
        .collect();
    let minimal_scan_time = start.elapsed();
    println!(
        "Minimal scan (stat only):                  {:>8.2}ms ({} procs)",
        minimal_scan_time.as_secs_f64() * 1000.0,
        pids.len()
    );

    // Measure raw file read vs procfs parsing for /proc/PID/stat
    println!("\n\nString allocation overhead analysis:");
    println!("─────────────────────────────────────────────────────────");

    // Raw file read (no parsing) - just measure I/O
    let start = Instant::now();
    let mut raw_count = 0;
    for pid in &pids {
        let path = format!("/proc/{}/stat", pid);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let _ = content; // Just read, don't parse
            raw_count += 1;
        }
    }
    let raw_read_time = start.elapsed();
    println!(
        "Raw read /proc/PID/stat (no parse):        {:>8.2}ms ({} files, {:>6.2}µs avg)",
        raw_read_time.as_secs_f64() * 1000.0,
        raw_count,
        raw_read_time.as_secs_f64() * 1_000_000.0 / raw_count as f64
    );

    // procfs stat() parsing (includes string allocation)
    let start = Instant::now();
    let mut parsed_count = 0;
    let mut total_comm_len = 0;
    for pid in &pids {
        if let Ok(proc) = procfs::process::Process::new(*pid) {
            if let Ok(stat) = proc.stat() {
                total_comm_len += stat.comm.len();
                parsed_count += 1;
            }
        }
    }
    let parsed_time = start.elapsed();
    println!(
        "procfs::stat() with parsing:               {:>8.2}ms ({} procs, {:>6.2}µs avg)",
        parsed_time.as_secs_f64() * 1000.0,
        parsed_count,
        parsed_time.as_secs_f64() * 1_000_000.0 / parsed_count as f64
    );
    println!(
        "  Total comm string bytes allocated: {} bytes",
        total_comm_len
    );

    // Measure cmdline allocation overhead
    let start = Instant::now();
    let mut cmdline_count = 0;
    let mut total_cmdline_bytes = 0;
    let mut total_cmdline_parts = 0;
    for pid in &pids {
        if let Ok(proc) = procfs::process::Process::new(*pid) {
            if let Ok(cmdline) = proc.cmdline() {
                total_cmdline_parts += cmdline.len();
                total_cmdline_bytes += cmdline.iter().map(|s| s.len()).sum::<usize>();
                cmdline_count += 1;
            }
        }
    }
    let cmdline_time = start.elapsed();
    println!(
        "cmdline() allocation analysis:             {:>8.2}ms ({} procs)",
        cmdline_time.as_secs_f64() * 1000.0,
        cmdline_count
    );
    println!("  Total Vec<String> parts: {}", total_cmdline_parts);
    println!("  Total string bytes: {} bytes", total_cmdline_bytes);
    println!(
        "  Avg parts per process: {:.1}",
        total_cmdline_parts as f64 / cmdline_count as f64
    );

    // WORST CASE: All columns enabled - benchmark everything
    println!("\n\n════════════════════════════════════════════════════════════");
    println!("WORST CASE SCENARIO: ALL COLUMNS ENABLED");
    println!("════════════════════════════════════════════════════════════");

    // Additional files we need to read for all columns
    let mut oom_time = std::time::Duration::ZERO;
    let mut cwd_time = std::time::Duration::ZERO;
    let mut smaps_time = std::time::Duration::ZERO;
    let mut autogroup_time = std::time::Duration::ZERO;
    let mut secattr_time = std::time::Duration::ZERO;

    let mut oom_count = 0;
    let mut cwd_count = 0;
    let mut smaps_count = 0;
    let mut autogroup_count = 0;
    let mut secattr_count = 0;

    for pid in &pids {
        // OOM score
        let path = format!("/proc/{}/oom_score", pid);
        let start = Instant::now();
        if std::fs::read_to_string(&path).is_ok() {
            oom_time += start.elapsed();
            oom_count += 1;
        }

        // CWD (symlink read)
        let path = format!("/proc/{}/cwd", pid);
        let start = Instant::now();
        if std::fs::read_link(&path).is_ok() {
            cwd_time += start.elapsed();
            cwd_count += 1;
        }

        // smaps_rollup (expensive!)
        let path = format!("/proc/{}/smaps_rollup", pid);
        let start = Instant::now();
        if std::fs::read_to_string(&path).is_ok() {
            smaps_time += start.elapsed();
            smaps_count += 1;
        }

        // autogroup
        let path = format!("/proc/{}/autogroup", pid);
        let start = Instant::now();
        if std::fs::read_to_string(&path).is_ok() {
            autogroup_time += start.elapsed();
            autogroup_count += 1;
        }

        // Security attributes
        let path = format!("/proc/{}/attr/current", pid);
        let start = Instant::now();
        if std::fs::read_to_string(&path).is_ok() {
            secattr_time += start.elapsed();
            secattr_count += 1;
        }
    }

    println!("\nAdditional column-specific reads:");
    println!("─────────────────────────────────────────────────────────");
    println!(
        "oom_score:     {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← OOM column",
        oom_time.as_secs_f64() * 1000.0,
        oom_count,
        if oom_count > 0 {
            oom_time.as_secs_f64() * 1_000_000.0 / oom_count as f64
        } else {
            0.0
        }
    );
    println!(
        "cwd:           {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← CWD column",
        cwd_time.as_secs_f64() * 1000.0,
        cwd_count,
        if cwd_count > 0 {
            cwd_time.as_secs_f64() * 1_000_000.0 / cwd_count as f64
        } else {
            0.0
        }
    );
    println!(
        "smaps_rollup:  {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← PSS/SWAP columns (EXPENSIVE!)",
        smaps_time.as_secs_f64() * 1000.0,
        smaps_count,
        if smaps_count > 0 {
            smaps_time.as_secs_f64() * 1_000_000.0 / smaps_count as f64
        } else {
            0.0
        }
    );
    println!(
        "autogroup:     {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← AUTOGROUP columns",
        autogroup_time.as_secs_f64() * 1000.0,
        autogroup_count,
        if autogroup_count > 0 {
            autogroup_time.as_secs_f64() * 1_000_000.0 / autogroup_count as f64
        } else {
            0.0
        }
    );
    println!(
        "attr/current:  {:>8.2}ms ({:>4} calls, {:>6.2}µs avg) ← SECATTR column",
        secattr_time.as_secs_f64() * 1000.0,
        secattr_count,
        if secattr_count > 0 {
            secattr_time.as_secs_f64() * 1_000_000.0 / secattr_count as f64
        } else {
            0.0
        }
    );

    // Sum up worst case total
    let worst_case_total = stat_time
        + uid_time      // We use uid() instead of status() now
        + cmdline_time
        + exe_time
        + statm_time
        + io_time
        + cgroups_time
        + maps_time     // For deleted lib check
        + oom_time
        + cwd_time
        + smaps_time
        + autogroup_time
        + secattr_time;

    println!("\n─────────────────────────────────────────────────────────");
    println!("WORST CASE TOTAL (all columns):");
    println!("─────────────────────────────────────────────────────────");
    println!(
        "  stat():          {:>8.2}ms  (always needed)",
        stat_time.as_secs_f64() * 1000.0
    );
    println!(
        "  uid():           {:>8.2}ms  (always needed)",
        uid_time.as_secs_f64() * 1000.0
    );
    println!(
        "  cmdline():       {:>8.2}ms  (Command column)",
        cmdline_time.as_secs_f64() * 1000.0
    );
    println!(
        "  exe():           {:>8.2}ms  (Command column)",
        exe_time.as_secs_f64() * 1000.0
    );
    println!(
        "  statm():         {:>8.2}ms  (M_SHARE/M_TRS/M_DRS)",
        statm_time.as_secs_f64() * 1000.0
    );
    println!(
        "  io():            {:>8.2}ms  (IO columns)",
        io_time.as_secs_f64() * 1000.0
    );
    println!(
        "  cgroups():       {:>8.2}ms  (CGROUP/CONTAINER)",
        cgroups_time.as_secs_f64() * 1000.0
    );
    println!(
        "  maps():          {:>8.2}ms  (deleted lib check)",
        maps_time.as_secs_f64() * 1000.0
    );
    println!(
        "  oom_score:       {:>8.2}ms  (OOM column)",
        oom_time.as_secs_f64() * 1000.0
    );
    println!(
        "  cwd:             {:>8.2}ms  (CWD column)",
        cwd_time.as_secs_f64() * 1000.0
    );
    println!(
        "  smaps_rollup:    {:>8.2}ms  (PSS/SWAP - EXPENSIVE!)",
        smaps_time.as_secs_f64() * 1000.0
    );
    println!(
        "  autogroup:       {:>8.2}ms  (AUTOGROUP columns)",
        autogroup_time.as_secs_f64() * 1000.0
    );
    println!(
        "  attr/current:    {:>8.2}ms  (SECATTR column)",
        secattr_time.as_secs_f64() * 1000.0
    );
    println!("─────────────────────────────────────────────────────────");
    println!(
        "  TOTAL:           {:>8.2}ms  ({} processes)",
        worst_case_total.as_secs_f64() * 1000.0,
        pids.len()
    );
    println!(
        "  Per-process:     {:>8.2}µs",
        worst_case_total.as_secs_f64() * 1_000_000.0 / pids.len() as f64
    );

    // Compare with default columns
    let default_total = stat_time + uid_time + cmdline_time + exe_time;
    println!("\n─────────────────────────────────────────────────────────");
    println!("COMPARISON:");
    println!("─────────────────────────────────────────────────────────");
    println!(
        "  Default columns: {:>8.2}ms  (stat+uid+cmdline+exe)",
        default_total.as_secs_f64() * 1000.0
    );
    println!(
        "  All columns:     {:>8.2}ms  (everything)",
        worst_case_total.as_secs_f64() * 1000.0
    );
    println!(
        "  Overhead:        {:>8.2}ms  ({:.1}x slower)",
        (worst_case_total - default_total).as_secs_f64() * 1000.0,
        worst_case_total.as_secs_f64() / default_total.as_secs_f64()
    );

    // =========================================================================
    // PARALLEL vs SEQUENTIAL BENCHMARK
    // =========================================================================
    println!("\n=========================================================");
    println!("PARALLEL vs SEQUENTIAL BENCHMARK");
    println!("=========================================================\n");

    // Collect PIDs first
    let pids: Vec<i32> = procfs::process::all_processes()
        .unwrap()
        .filter_map(|p| p.ok())
        .filter_map(|p| p.stat().ok().map(|s| s.pid))
        .collect();

    println!("Testing with {} processes...\n", pids.len());

    // Sequential: read stat+cmdline for all processes
    let seq_start = Instant::now();
    let mut seq_results = Vec::with_capacity(pids.len());
    for &pid in &pids {
        if let Ok(proc) = procfs::process::Process::new(pid) {
            let stat = proc.stat().ok();
            let cmdline = proc.cmdline().ok();
            seq_results.push((pid, stat.is_some(), cmdline.is_some()));
        }
    }
    let seq_time = seq_start.elapsed();
    println!(
        "Sequential (stat+cmdline): {:>8.2}ms  ({} processes)",
        seq_time.as_secs_f64() * 1000.0,
        seq_results.len()
    );

    // Test with different thread counts
    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    for num_threads in [2, 4, 8, 16, max_threads]
        .iter()
        .filter(|&&n| n <= max_threads)
    {
        let num_threads = *num_threads;

        let par_start = Instant::now();
        let chunk_size = pids.len().div_ceil(num_threads);
        let chunks: Vec<_> = pids.chunks(chunk_size).collect();

        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let chunk = chunk.to_vec();
                std::thread::spawn(move || {
                    let mut results = Vec::with_capacity(chunk.len());
                    for pid in chunk {
                        if let Ok(proc) = procfs::process::Process::new(pid) {
                            let stat = proc.stat().ok();
                            let cmdline = proc.cmdline().ok();
                            results.push((pid, stat.is_some(), cmdline.is_some()));
                        }
                    }
                    results
                })
            })
            .collect();

        let _par_results: Vec<_> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();
        let par_time = par_start.elapsed();

        let speedup = seq_time.as_secs_f64() / par_time.as_secs_f64();
        println!(
            "Parallel {:>2} threads:        {:>8.2}ms  ({:.2}x speedup)",
            num_threads,
            par_time.as_secs_f64() * 1000.0,
            speedup
        );
    }
    println!("─────────────────────────────────────────────────────────");

    // =========================================================================
    // PARALLEL EXPENSIVE OPERATIONS ONLY
    // =========================================================================
    println!("\n=========================================================");
    println!("PARALLEL EXPENSIVE OPERATIONS (statm+cgroups+oom+smaps)");
    println!("=========================================================\n");

    // Sequential expensive operations
    let exp_seq_start = Instant::now();
    for &pid in &pids {
        if let Ok(proc) = procfs::process::Process::new(pid) {
            let _ = proc.statm();
            let _ = proc.cgroups();
            let _ = std::fs::read_to_string(format!("/proc/{}/oom_score", pid));
            let _ = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", pid));
        }
    }
    let exp_seq_time = exp_seq_start.elapsed();
    println!(
        "Sequential expensive ops:  {:>8.2}ms  ({} processes)",
        exp_seq_time.as_secs_f64() * 1000.0,
        pids.len()
    );

    // Parallel expensive operations with different thread counts
    for num_threads in [2, 4, 8].iter() {
        let num_threads = *num_threads;

        let exp_par_start = Instant::now();
        let chunk_size = pids.len().div_ceil(num_threads);
        let chunks: Vec<_> = pids.chunks(chunk_size).collect();

        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let chunk = chunk.to_vec();
                std::thread::spawn(move || {
                    for pid in chunk {
                        if let Ok(proc) = procfs::process::Process::new(pid) {
                            let _ = proc.statm();
                            let _ = proc.cgroups();
                            let _ = std::fs::read_to_string(format!("/proc/{}/oom_score", pid));
                            let _ = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", pid));
                        }
                    }
                })
            })
            .collect();

        for h in handles {
            let _ = h.join();
        }
        let exp_par_time = exp_par_start.elapsed();

        let speedup = exp_seq_time.as_secs_f64() / exp_par_time.as_secs_f64();
        println!(
            "Parallel {:>2} threads:        {:>8.2}ms  ({:.2}x speedup)",
            num_threads,
            exp_par_time.as_secs_f64() * 1000.0,
            speedup
        );
    }
    println!("─────────────────────────────────────────────────────────");

    // =========================================================================
    // RAYON PARALLEL TEST
    // =========================================================================
    println!("\n=========================================================");
    println!("RAYON PARALLEL EXPENSIVE OPERATIONS");
    println!("=========================================================\n");

    use rayon::prelude::*;

    // Warm up rayon thread pool
    let _: Vec<_> = (0..100).into_par_iter().map(|x| x * 2).collect();

    // Test with rayon
    let rayon_start = Instant::now();
    let _results: Vec<_> = pids
        .par_iter()
        .filter_map(|&pid| {
            if let Ok(proc) = procfs::process::Process::new(pid) {
                let statm = proc.statm().ok();
                let cgroups = proc.cgroups().ok();
                let oom = std::fs::read_to_string(format!("/proc/{}/oom_score", pid)).ok();
                let smaps = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", pid)).ok();
                Some((
                    pid,
                    statm.is_some(),
                    cgroups.is_some(),
                    oom.is_some(),
                    smaps.is_some(),
                ))
            } else {
                None
            }
        })
        .collect();
    let rayon_time = rayon_start.elapsed();

    let rayon_speedup = exp_seq_time.as_secs_f64() / rayon_time.as_secs_f64();
    println!(
        "Rayon (work-stealing):     {:>8.2}ms  ({:.2}x speedup vs sequential)",
        rayon_time.as_secs_f64() * 1000.0,
        rayon_speedup
    );
    println!("─────────────────────────────────────────────────────────");
}
