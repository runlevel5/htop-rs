#!/usr/bin/env rust-script
//! List kernel threads on Linux and macOS for verification against htop
//!
//! Usage:
//!   cargo run --bin list_kernel_threads
//!
//! Or compile directly:
//!   rustc tools/list_kernel_threads.rs -o list_kernel_threads && ./list_kernel_threads

#[cfg(target_os = "linux")]
use std::fs;

fn main() {
    #[cfg(target_os = "linux")]
    list_kernel_threads_linux();

    #[cfg(target_os = "macos")]
    list_kernel_threads_macos();

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    println!("Unsupported platform");
}

#[cfg(target_os = "linux")]
fn list_kernel_threads_linux() {
    println!("=== Linux Kernel Threads ===\n");
    println!("Kernel threads are identified by:");
    println!("  1. PPID = 2 (parent is kthreadd), OR");
    println!("  2. PID = 2 (kthreadd itself), OR");
    println!("  3. PF_KTHREAD flag (0x00200000) in /proc/[pid]/stat flags field\n");
    println!("{:<8} {:<8} {:<6} {:<20} {}", "PID", "PPID", "KTHRD", "COMM", "CMDLINE");
    println!("{}", "-".repeat(70));

    let mut kernel_threads = Vec::new();
    let mut user_processes = Vec::new();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            // Only process numeric directories (PIDs)
            if let Ok(pid) = name_str.parse::<i32>() {
                let proc_path = entry.path();
                
                // Read stat file
                let stat_path = proc_path.join("stat");
                let stat_content = match fs::read_to_string(&stat_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                
                // Parse stat - format: pid (comm) state ppid ...
                // Find the last ')' to handle comm with spaces/parens
                let comm_end = match stat_content.rfind(')') {
                    Some(pos) => pos,
                    None => continue,
                };
                
                let comm_start = match stat_content.find('(') {
                    Some(pos) => pos + 1,
                    None => continue,
                };
                
                let comm = &stat_content[comm_start..comm_end];
                let after_comm = &stat_content[comm_end + 2..]; // skip ") "
                let fields: Vec<&str> = after_comm.split_whitespace().collect();
                
                if fields.len() < 7 {
                    continue;
                }
                
                let ppid: i32 = fields[1].parse().unwrap_or(0);
                let flags: u64 = fields[6].parse().unwrap_or(0);
                
                // Check if kernel thread
                // PF_KTHREAD = 0x00200000 (from Linux kernel headers)
                const PF_KTHREAD: u64 = 0x00200000;
                let is_kthread_by_flag = (flags & PF_KTHREAD) != 0;
                let is_kthread_by_ppid = ppid == 2 || pid == 2;
                let is_kernel_thread = is_kthread_by_flag || is_kthread_by_ppid;
                
                // Read cmdline
                let cmdline_path = proc_path.join("cmdline");
                let cmdline = fs::read_to_string(&cmdline_path)
                    .unwrap_or_default()
                    .replace('\0', " ")
                    .trim()
                    .to_string();
                
                let cmdline_display = if cmdline.is_empty() {
                    format!("[{}]", comm) // Kernel threads have empty cmdline
                } else {
                    cmdline.chars().take(40).collect()
                };
                
                let entry = (pid, ppid, is_kernel_thread, comm.to_string(), cmdline_display);
                
                if is_kernel_thread {
                    kernel_threads.push(entry);
                } else {
                    user_processes.push(entry);
                }
            }
        }
    }
    
    // Sort by PID
    kernel_threads.sort_by_key(|e| e.0);
    user_processes.sort_by_key(|e| e.0);
    
    println!("\n--- Kernel Threads (first 30) ---");
    for (pid, ppid, _, comm, cmdline) in kernel_threads.iter().take(30) {
        println!("{:<8} {:<8} {:<6} {:<20} {}", pid, ppid, "YES", comm, cmdline);
    }
    
    println!("\n--- User Processes (first 10) ---");
    for (pid, ppid, _, comm, cmdline) in user_processes.iter().take(10) {
        println!("{:<8} {:<8} {:<6} {:<20} {}", pid, ppid, "NO", comm, cmdline);
    }
    
    println!("\n=== Summary ===");
    println!("Total kernel threads: {}", kernel_threads.len());
    println!("Total user processes: {}", user_processes.len());
    println!("\nTo verify in htop-rs:");
    println!("  1. Run htop-rs");
    println!("  2. Press 'K' to toggle hide kernel threads");
    println!("  3. Kernel threads should disappear/reappear");
    println!("  4. Kernel threads are shown with brackets: [kthreadd], [ksoftirqd/0], etc.");
}

#[cfg(target_os = "macos")]
fn list_kernel_threads_macos() {
    use std::process::Command;
    
    println!("=== macOS Kernel Threads ===\n");
    println!("On macOS, kernel threads are identified by:");
    println!("  1. Processes owned by root (UID 0) with specific names, OR");
    println!("  2. Processes in the kernel_task family\n");
    println!("Note: macOS doesn't expose kernel threads the same way Linux does.");
    println!("The 'kernel_task' process (PID 0) represents kernel activity.\n");
    
    // Use ps to list processes
    let output = Command::new("ps")
        .args(["-axo", "pid,ppid,uid,comm"])
        .output()
        .expect("Failed to run ps");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    println!("{:<8} {:<8} {:<8} {}", "PID", "PPID", "UID", "COMMAND");
    println!("{}", "-".repeat(60));
    
    let mut kernel_like = Vec::new();
    let mut user_procs = Vec::new();
    
    for line in stdout.lines().skip(1) { // skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let pid: i32 = parts[0].parse().unwrap_or(-1);
            let ppid: i32 = parts[1].parse().unwrap_or(-1);
            let uid: u32 = parts[2].parse().unwrap_or(u32::MAX);
            let comm = parts[3..].join(" ");
            
            // On macOS, "kernel-like" processes:
            // - kernel_task (PID 0)
            // - launchd (PID 1) 
            // - Various system daemons owned by root
            let is_kernel_like = pid == 0 
                || comm.contains("kernel_task")
                || (uid == 0 && (
                    comm.starts_with("com.apple.") 
                    || comm.contains("kext")
                    || comm.contains("kernel")
                ));
            
            if is_kernel_like {
                kernel_like.push((pid, ppid, uid, comm));
            } else {
                user_procs.push((pid, ppid, uid, comm));
            }
        }
    }
    
    println!("\n--- Kernel-like Processes ---");
    for (pid, ppid, uid, comm) in kernel_like.iter().take(20) {
        println!("{:<8} {:<8} {:<8} {}", pid, ppid, uid, comm);
    }
    
    println!("\n--- Sample User Processes (first 10) ---");
    for (pid, ppid, uid, comm) in user_procs.iter().take(10) {
        println!("{:<8} {:<8} {:<8} {}", pid, ppid, uid, comm);
    }
    
    println!("\n=== Summary ===");
    println!("Kernel-like processes: {}", kernel_like.len());
    println!("User processes: {}", user_procs.len());
    println!("\nNote: macOS htop-rs identifies kernel threads differently than Linux.");
    println!("On macOS, there's no direct equivalent to Linux kernel threads.");
    println!("The 'K' key in htop-rs may have limited effect on macOS.");
}
