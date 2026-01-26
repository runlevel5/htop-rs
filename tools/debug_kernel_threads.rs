//! Debug tool to check kernel thread data from procfs
//!
//! Usage:
//!   cargo run --bin debug_kernel_threads
//!
//! Note: This tool only works on Linux

#[cfg(target_os = "linux")]
use procfs::process::Process;

fn main() {
    #[cfg(target_os = "linux")]
    linux_main();

    #[cfg(not(target_os = "linux"))]
    println!("This tool only works on Linux");
}

#[cfg(target_os = "linux")]
fn linux_main() {
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
                let cmdline: Option<String> = proc
                    .cmdline()
                    .ok()
                    .filter(|c| !c.is_empty())
                    .map(|c| c.join(" "));
                let merged_command_str: Option<String> = None;

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

                let is_ok = display_command != "<unknown>" && !display_command.is_empty();
                let status = if is_ok { "OK" } else { "FAIL" };
                println!(
                    "[{}] PID {:5}: kthread={:<5} comm={:25} display='{}'",
                    status,
                    pid,
                    is_kernel_thread,
                    comm.as_deref().unwrap_or("None"),
                    display_command
                );
            }
            Err(e) => {
                println!("[FAIL] PID {:5}: ERROR - {}", pid, e);
            }
        }
    }

    println!("\n=== Summary ===");
    println!("If kernel threads show their comm name instead of '<unknown>', the fix is working!");
}
