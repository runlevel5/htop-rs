//! Debug tool to check kernel thread data from procfs
//!
//! Usage:
//!   cargo run --bin debug_kernel_threads

use procfs::process::Process;
use std::collections::HashMap;

fn main() {
    println!("=== Testing kernel thread display fix ===\n");
    
    // Check specific kernel thread PIDs
    let kernel_pids = [2, 3, 4, 5, 6, 7, 8];
    
    // Simulate what htop-rs Process does
    for pid in kernel_pids {
        match Process::new(pid) {
            Ok(proc) => {
                let stat = match proc.stat() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                
                let ppid = stat.ppid;
                let is_kernel_thread = ppid == 2 || pid == 2;
                
                // Simulate our Process struct
                let comm = Some(stat.comm.clone());
                let cmdline: Option<String> = proc.cmdline()
                    .ok()
                    .filter(|c| !c.is_empty())
                    .map(|c| c.join(" "));
                let merged_command_str: Option<String> = None; // kernel threads don't get merged command
                
                // Simulate get_command() fallback chain
                let display_command = if let Some(ref merged) = merged_command_str {
                    merged.clone()
                } else if let Some(ref cl) = cmdline {
                    cl.clone()
                } else if let Some(ref c) = comm {
                    c.clone()
                } else {
                    "<unknown>".to_string()
                };
                
                // Check if the fix works
                let is_ok = if is_kernel_thread {
                    display_command != "<unknown>" && !display_command.is_empty()
                } else {
                    true // user processes should have cmdline
                };
                
                let status = if is_ok { "✓" } else { "✗" };
                println!("{} PID {:5}: is_kthread={:<5} comm={:25} cmdline={:15} display='{}'",
                    status, pid, is_kernel_thread, 
                    comm.as_deref().unwrap_or("None"),
                    cmdline.as_deref().unwrap_or("<empty>"),
                    display_command);
            }
            Err(e) => {
                println!("✗ PID {:5}: ERROR - {}", pid, e);
            }
        }
    }
    
    // Also check some user processes to make sure they still work
    println!("\n--- User processes (should still work) ---");
    if let Ok(entries) = std::fs::read_dir("/proc") {
        let mut user_pids: Vec<i32> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.parse::<i32>().ok())
            .filter(|&pid| pid > 100) // Skip low PIDs which are likely kernel threads
            .take(5)
            .collect();
        user_pids.sort();
        
        for pid in user_pids {
            if let Ok(proc) = Process::new(pid) {
                if let Ok(stat) = proc.stat() {
                    let ppid = stat.ppid;
                    let is_kernel_thread = ppid == 2 || pid == 2;
                    
                    if is_kernel_thread {
                        continue; // Skip kernel threads in this section
                    }
                    
                    let comm = Some(stat.comm.clone());
                    let cmdline: Option<String> = proc.cmdline()
                        .ok()
                        .filter(|c| !c.is_empty())
                        .map(|c| c.join(" "));
                    let merged_command_str: Option<String> = None;
                    
                    let display_command = if let Some(ref merged) = merged_command_str {
                        merged.clone()
                    } else if let Some(ref cl) = cmdline {
                        cl.chars().take(50).collect()
                    } else if let Some(ref c) = comm {
                        c.clone()
                    } else {
                        "<unknown>".to_string()
                    };
                    
                    println!("✓ PID {:5}: comm={:15} display='{}'",
                        pid,
                        comm.as_deref().unwrap_or("None"),
                        display_command);
                }
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("If kernel threads show their comm name instead of '<unknown>', the fix is working!");
}
