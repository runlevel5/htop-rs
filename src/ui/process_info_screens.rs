//! Process Info Screens
//!
//! This module contains the various popup screens for displaying
//! process-related information:
//! - Environment variables (EnvScreen)
//! - File locks (ProcessLocksScreen)
//! - Open files via lsof (OpenFilesScreen)
//! - Strace output (TraceScreen)
//! - Command line (CommandScreen)

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use super::crt::{
    ColorElement, A_NORMAL, KEY_BACKSLASH, KEY_BACKSPACE, KEY_CTRL_L, KEY_CTRL_N, KEY_CTRL_P,
    KEY_DOWN, KEY_END, KEY_ENTER, KEY_ESC, KEY_F10, KEY_F15, KEY_F3, KEY_F4, KEY_F5, KEY_F8,
    KEY_F9, KEY_HOME, KEY_LC_F, KEY_LC_Q, KEY_LC_T, KEY_MOUSE, KEY_NPAGE, KEY_PPAGE, KEY_SLASH,
    KEY_UP, KEY_WHEELDOWN, KEY_WHEELUP,
};
use super::function_bar::FunctionBar;
use super::info_screen::{run_info_screen, InfoScreenConfig};
use super::search_filter::{process_mouse_event, HandleResult, SearchFilterState};
use super::Crt;

/// Parsed lsof file entry (from lsof -F output)
#[derive(Default)]
struct LsofFileEntry {
    fd: String,        // File descriptor
    file_type: String, // File type (REG, DIR, etc.)
    mode: String,      // Access mode (r/w/u)
    device: String,    // Device number
    size: String,      // File size
    offset: String,    // File offset
    inode: String,     // Inode number
    name: String,      // File name/path
}

pub fn show_process_env(crt: &mut Crt, pid: i32, command: &str) {
    // Helper to read environment
    let read_env = || -> Vec<String> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("ps")
                .args(["-p", &pid.to_string(), "-E", "-o", "command="])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| {
                    let mut vars: Vec<String> = Vec::new();
                    if let Some(pos) = s.find(' ') {
                        let env_part = &s[pos + 1..];
                        for part in env_part.split_whitespace() {
                            if part.contains('=') {
                                vars.push(part.to_string());
                            }
                        }
                    }
                    vars.sort();
                    vars
                })
                .unwrap_or_else(|| vec!["Could not read process environment.".to_string()])
        }
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            fs::read_to_string(format!("/proc/{}/environ", pid))
                .map(|s| {
                    let mut vars: Vec<String> = s
                        .split('\0')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    vars.sort();
                    vars
                })
                .unwrap_or_else(|_| vec!["Could not read process environment.".to_string()])
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            vec!["Environment reading not supported on this platform.".to_string()]
        }
    };

    let mut lines = read_env();

    let config = InfoScreenConfig {
        title: format!("Environment of process {} - {}", pid, command),
        header: None,
        use_redraw_optimization: false,
    };

    run_info_screen(crt, &config, &mut lines, Some(&read_env));
}

/// Show file locks for process (like C htop ProcessLocksScreen)
pub fn show_file_locks(crt: &mut Crt, pid: i32, command: &str) {
    // Helper to read file locks (for refresh)
    let read_locks = || -> Vec<String> {
        match get_process_locks(pid) {
            Ok(locks) if locks.is_empty() => {
                vec!["No locks have been found for the selected process.".to_string()]
            }
            Ok(locks) => locks,
            Err(msg) => vec![msg],
        }
    };

    let mut lines = read_locks();

    // Header matching C htop ProcessLocksScreen
    let header_str =
        "   FD TYPE       EXCLUSION  READ/WRITE DEVICE       NODE               START                 END  FILENAME";

    let config = InfoScreenConfig {
        title: format!("Snapshot of file locks of process {} - {}", pid, command),
        header: Some(header_str),
        use_redraw_optimization: false,
    };

    run_info_screen(crt, &config, &mut lines, Some(&read_locks));
}

/// Get file locks for a process
/// Returns formatted lock entries or error message
fn get_process_locks(pid: i32) -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let fdinfo_path = format!("/proc/{}/fdinfo", pid);
        let fd_path = format!("/proc/{}/fd", pid);

        let entries = match fs::read_dir(&fdinfo_path) {
            Ok(e) => e,
            Err(_) => return Err("Could not read process file descriptor info.".to_string()),
        };

        let mut locks = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip . and ..
            if name_str == "." || name_str == ".." {
                continue;
            }

            // Parse FD number
            let fd: i32 = match name_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Read fdinfo file
            let fdinfo_file = format!("{}/{}", fdinfo_path, name_str);
            let content = match fs::read_to_string(&fdinfo_file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Look for lock: lines
            for line in content.lines() {
                if !line.starts_with("lock:\t") {
                    continue;
                }

                // Parse lock line format:
                // lock:\t1: FLOCK  ADVISORY  WRITE 12345 08:01:123456 0 EOF
                let lock_part = &line[6..]; // Skip "lock:\t"
                let parts: Vec<&str> = lock_part.split_whitespace().collect();

                if parts.len() < 8 {
                    continue;
                }

                // Parts: [0]=id:, [1]=type, [2]=advisory, [3]=read/write, [4]=pid, [5]=dev:inode, [6]=start, [7]=end
                let locktype = parts[1];
                let exclusive = parts[2];
                let readwrite = parts[3];
                let dev_inode = parts[5];
                let start = parts[6];
                let end = parts[7];

                // Parse device:inode (format: major:minor:inode)
                let dev_parts: Vec<&str> = dev_inode.split(':').collect();
                let (dev, inode) = if dev_parts.len() >= 3 {
                    let major: u64 = dev_parts[0].parse().unwrap_or(0);
                    let minor: u64 = dev_parts[1].parse().unwrap_or(0);
                    let inode: u64 = dev_parts[2].parse().unwrap_or(0);
                    let dev = (major << 8) | minor;
                    (format!("{:#6x}", dev), inode.to_string())
                } else {
                    ("     0".to_string(), "0".to_string())
                };

                // Format end (EOF or number)
                let end_display = if end == "EOF" {
                    "<END OF FILE>".to_string()
                } else {
                    end.to_string()
                };

                // Get filename from /proc/pid/fd/N
                let fd_link = format!("{}/{}", fd_path, fd);
                let filename = fs::read_link(&fd_link)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "<N/A>".to_string());

                // Format entry matching C htop format
                let entry = format!(
                    "{:5} {:<10} {:<10} {:<10} {:>6} {:>10} {:>19} {:>19}  {}",
                    fd, locktype, exclusive, readwrite, dev, inode, start, end_display, filename
                );
                locks.push(entry);
            }
        }

        locks.sort();
        Ok(locks)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = pid;
        // macOS doesn't support this feature (same as C htop)
        Err("This feature is not supported on your platform.".to_string())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        Err("This feature is not supported on your platform.".to_string())
    }
}

/// Show lsof output for process (like C htop OpenFilesScreen)
pub fn show_lsof(crt: &mut Crt, pid: i32, command: &str) {
    // Parse lsof output using -F (machine-readable format)
    let lsof_data = run_lsof(pid);

    // Build lines from parsed data
    let mut lines: Vec<String> = Vec::new();
    let mut col_widths = [5usize, 7, 4, 6, 8, 8, 8]; // FD, TYPE, MODE, DEVICE, SIZE, OFFSET, NODE

    match &lsof_data {
        Ok(files) => {
            // Calculate dynamic column widths
            for file in files {
                col_widths[4] = col_widths[4].max(file.size.len());
                col_widths[5] = col_widths[5].max(file.offset.len());
                col_widths[6] = col_widths[6].max(file.inode.len());
            }

            // Build formatted lines
            for file in files {
                let line = format!(
                    "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
                    file.fd,
                    file.file_type,
                    file.mode,
                    file.device,
                    file.size,
                    file.offset,
                    file.inode,
                    file.name,
                    width_s = col_widths[4],
                    width_o = col_widths[5],
                    width_i = col_widths[6],
                );
                lines.push(line);
            }
        }
        Err(msg) => {
            lines.push(msg.clone());
        }
    }

    // Build header with dynamic column widths
    let header_str = format!(
        "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
        "FD",
        "TYPE",
        "MODE",
        "DEVICE",
        "SIZE",
        "OFFSET",
        "NODE",
        "NAME",
        width_s = col_widths[4],
        width_o = col_widths[5],
        width_i = col_widths[6],
    );

    // State for the info screen
    let mut selected = 0i32;
    let mut scroll_v = 0i32;
    let mut sf_state = SearchFilterState::new();
    let mut needs_redraw = true;

    loop {
        let filtered_indices = sf_state.filter_indices(&lines);
        let panel_height = crt.height() - 3; // Title + header + function bar
        let panel_y = 2; // After title and header

        // Clamp selection and scroll
        let max_selected = (filtered_indices.len() as i32 - 1).max(0);
        selected = selected.clamp(0, max_selected);

        if selected < scroll_v {
            scroll_v = selected;
        } else if selected >= scroll_v + panel_height {
            scroll_v = selected - panel_height + 1;
        }
        // Clamp scroll_v to valid range
        let max_scroll = (filtered_indices.len() as i32 - panel_height).max(0);
        scroll_v = scroll_v.clamp(0, max_scroll);

        // Only redraw when needed
        if needs_redraw {
            let screen_width = crt.width();

            // Draw title
            let title_attr = crt.color(ColorElement::MeterText);
            let title = format!("Snapshot of files open in process {} - {}", pid, command);
            let title_display: String = title.chars().take(screen_width as usize).collect();
            crt.mv(0, 0);
            crt.attrset(title_attr);
            crt.hline(0, 0, ' ' as u32, screen_width);
            crt.addstr_raw(&title_display);
            crt.attrset(A_NORMAL);

            // Draw header
            let header_attr = crt.color(ColorElement::PanelHeaderFocus);
            let header_display: String = header_str.chars().take(screen_width as usize).collect();
            crt.mv(1, 0);
            crt.attrset(header_attr);
            crt.hline(1, 0, ' ' as u32, screen_width);
            crt.addstr_raw(&header_display);
            crt.attrset(A_NORMAL);

            // Draw lines
            let default_attr = crt.color(ColorElement::DefaultColor);
            let selection_attr = crt.color(ColorElement::PanelSelectionFocus);

            for row in 0..panel_height {
                let y = panel_y + row;
                let line_idx = (scroll_v + row) as usize;

                if line_idx < filtered_indices.len() {
                    let actual_idx = filtered_indices[line_idx];
                    let line = &lines[actual_idx];
                    let is_selected = (scroll_v + row) == selected;

                    let attr = if is_selected {
                        selection_attr
                    } else {
                        default_attr
                    };
                    let display_line: String = line.chars().take(screen_width as usize).collect();
                    crt.mv(y, 0);
                    crt.attrset(attr);
                    crt.hline(y, 0, ' ' as u32, screen_width);
                    crt.addstr_raw(&display_line);
                    crt.attrset(A_NORMAL);
                } else {
                    crt.mv(y, 0);
                    crt.attrset(default_attr);
                    crt.hline(y, 0, ' ' as u32, screen_width);
                    crt.attrset(A_NORMAL);
                }
            }

            // Draw function bar or search/filter bar
            let fb_y = crt.height() - 1;
            sf_state.draw_bar(crt, fb_y, screen_width);

            crt.refresh();
        }

        // Handle input
        crt.set_blocking(true);
        let mut ch = crt.getch();
        needs_redraw = true; // Assume we need redraw, unless proven otherwise

        // Handle mouse events
        ch = process_mouse_event(crt, &sf_state, ch);

        // Handle search/filter input
        if sf_state.is_active() {
            use HandleResult;
            match sf_state.handle_input(ch) {
                HandleResult::SearchChanged => {
                    if let Some(idx) = sf_state.find_first_match(&lines, &filtered_indices) {
                        selected = idx as i32;
                    }
                }
                HandleResult::FilterChanged => {
                    selected = 0;
                    scroll_v = 0;
                }
                HandleResult::SearchNext => {
                    // F3 - find next match
                    if let Some(idx) =
                        sf_state.find_next_match(&lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::SearchPrev => {
                    // S-F3 - find previous match
                    if let Some(idx) =
                        sf_state.find_prev_match(&lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::Handled => {}
                HandleResult::NotHandled => {
                    needs_redraw = false;
                }
            }
            continue;
        }

        // Handle normal keys
        if sf_state.handle_start_key(ch) {
            continue;
        }

        match ch {
            27 | 113 | KEY_F10 => {
                // Escape, 'q', or F10 - exit
                break;
            }
            x if x == KEY_F5 => {
                // F5 - refresh (preserve selected index like C htop)
                let saved_selected = selected;
                let new_data = run_lsof(pid);
                lines.clear();
                match new_data {
                    Ok(files) => {
                        // Recalculate column widths
                        col_widths = [5, 7, 4, 6, 8, 8, 8];
                        for file in &files {
                            col_widths[4] = col_widths[4].max(file.size.len());
                            col_widths[5] = col_widths[5].max(file.offset.len());
                            col_widths[6] = col_widths[6].max(file.inode.len());
                        }
                        for file in files {
                            let line = format!(
                                "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
                                file.fd,
                                file.file_type,
                                file.mode,
                                file.device,
                                file.size,
                                file.offset,
                                file.inode,
                                file.name,
                                width_s = col_widths[4],
                                width_o = col_widths[5],
                                width_i = col_widths[6],
                            );
                            lines.push(line);
                        }
                    }
                    Err(msg) => {
                        lines.push(msg);
                    }
                }
                // Recalculate filtered indices and restore selection
                let new_filtered = sf_state.filter_indices(&lines);
                let max_idx = (new_filtered.len() as i32 - 1).max(0);
                selected = saved_selected.min(max_idx);
                crt.clear();
            }
            KEY_CTRL_L => {
                // Ctrl+L - redraw
                crt.clear();
            }
            KEY_UP => {
                let old_selected = selected;
                selected = (selected - 1).max(0);
                if selected == old_selected {
                    needs_redraw = false;
                }
            }
            KEY_DOWN => {
                let old_selected = selected;
                selected = (selected + 1).min(max_selected);
                if selected == old_selected {
                    needs_redraw = false;
                }
            }
            KEY_PPAGE => {
                selected = (selected - panel_height).max(0);
            }
            KEY_NPAGE => {
                selected = (selected + panel_height).min(max_selected);
            }
            KEY_HOME => {
                selected = 0;
            }
            KEY_END => {
                selected = max_selected;
            }
            KEY_WHEELUP => {
                selected = (selected - 3).max(0);
            }
            KEY_WHEELDOWN => {
                selected = (selected + 3).min(max_selected);
            }
            _ => {
                needs_redraw = false;
            }
        }
    }

    crt.enable_delay();
}

fn run_lsof(pid: i32) -> Result<Vec<LsofFileEntry>, String> {
    use std::process::Command;

    // Run lsof with -F flag for machine-readable output
    // -P: inhibit conversion of port numbers to port names
    // -o: always print file offset
    // -F: produce output suitable for processing
    let output = Command::new("lsof")
        .args(["-P", "-o", "-p", &pid.to_string(), "-F"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            return Err(
                "Could not execute 'lsof'. Please make sure it is available in your $PATH."
                    .to_string(),
            )
        }
    };

    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        if code == 127 {
            return Err(
                "Could not execute 'lsof'. Please make sure it is available in your $PATH."
                    .to_string(),
            );
        }
        return Err("Failed listing open files.".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse lsof -F output format
    // Fields are prefixed with a single character:
    // f = file descriptor
    // a = access mode (r/w/u)
    // t = type
    // D = device
    // s = size
    // o = offset
    // i = inode
    // n = name

    let mut files: Vec<LsofFileEntry> = Vec::new();
    let mut current_file: Option<LsofFileEntry> = None;
    let mut has_size_field = false;

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let cmd = line.chars().next().unwrap_or(' ');
        let value = &line[1..];

        match cmd {
            'f' => {
                // New file entry - save previous if exists
                if let Some(file) = current_file.take() {
                    files.push(file);
                }
                current_file = Some(LsofFileEntry {
                    fd: value.to_string(),
                    ..Default::default()
                });
            }
            'a' => {
                if let Some(ref mut file) = current_file {
                    file.mode = value.to_string();
                }
            }
            't' => {
                if let Some(ref mut file) = current_file {
                    file.file_type = value.to_string();
                }
            }
            'D' => {
                if let Some(ref mut file) = current_file {
                    file.device = value.to_string();
                }
            }
            's' => {
                if let Some(ref mut file) = current_file {
                    file.size = value.to_string();
                    has_size_field = true;
                }
            }
            'o' => {
                if let Some(ref mut file) = current_file {
                    // Remove "0t" prefix if present
                    let offset = value.strip_prefix("0t").unwrap_or(value);
                    file.offset = offset.to_string();
                }
            }
            'i' => {
                if let Some(ref mut file) = current_file {
                    file.inode = value.to_string();
                }
            }
            'n' => {
                if let Some(ref mut file) = current_file {
                    file.name = value.to_string();
                }
            }
            // Ignore other fields (p, c, u, g, R, etc.)
            _ => {}
        }
    }

    // Save last file
    if let Some(file) = current_file {
        files.push(file);
    }

    // On Linux, lsof -o -F omits SIZE, so get it from stat() if needed
    #[cfg(target_os = "linux")]
    if !has_size_field {
        for file in &mut files {
            if file.size.is_empty() {
                if let Ok(metadata) = std::fs::metadata(&file.name) {
                    file.size = metadata.len().to_string();
                }
            }
        }
    }

    // Suppress unused variable warning on non-Linux
    #[cfg(not(target_os = "linux"))]
    let _ = has_size_field;

    if files.is_empty() {
        return Err("No open files found.".to_string());
    }

    Ok(files)
}

/// Show strace output for process (like C htop TraceScreen)
/// On Linux: forks strace and displays output live
/// On BSD: uses truss instead
/// On unsupported platforms: shows "Tracing unavailable" message
pub fn show_strace(crt: &mut Crt, pid: i32, command: &str) {
    use std::process::Child;
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    use std::process::{Command, Stdio};

    // Platform-specific tracer command
    #[cfg(target_os = "linux")]
    let tracer_result: Result<Child, std::io::Error> = Command::new("strace")
        .args(["-T", "-tt", "-s", "512", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // strace outputs to stderr
        .spawn();

    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let tracer_result: Result<Child, std::io::Error> = Command::new("truss")
        .args(["-s", "512", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    let tracer_result: Result<Child, std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Tracing unavailable",
    ));

    // Check if tracer started successfully
    let mut tracer_child: Option<Child>;
    let mut lines: Vec<String> = Vec::new();
    let mut strace_alive: bool;
    let error_message: Option<String>;

    match tracer_result {
        Ok(child) => {
            // Set stderr to non-blocking for live reading
            // strace outputs to stderr, not stdout
            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                if let Some(ref stderr) = child.stderr {
                    let fd = stderr.as_raw_fd();
                    unsafe {
                        let flags = libc::fcntl(fd, libc::F_GETFL);
                        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                    }
                }
            }
            tracer_child = Some(child);
            strace_alive = true;
            error_message = None;
        }
        Err(e) => {
            tracer_child = None;
            strace_alive = false;
            #[cfg(target_os = "linux")]
            {
                error_message = Some(format!(
                    "Could not execute 'strace': {}. Please make sure it is available in your $PATH.",
                    e
                ));
            }
            #[cfg(any(
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            ))]
            {
                error_message = Some(format!(
                    "Could not execute 'truss': {}. Please make sure it is available in your $PATH.",
                    e
                ));
            }
            #[cfg(not(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            )))]
            {
                let _ = e;
                error_message = Some("Tracing unavailable on this system.".to_string());
            }
        }
    }

    // Add error message as first line if any
    if let Some(msg) = error_message {
        lines.push(msg);
    }

    // State for the trace screen
    let mut selected = 0i32;
    let mut scroll_v = 0i32;
    let mut tracing = true; // Whether to capture new lines
    let mut follow = false; // Auto-scroll to bottom (C htop starts with follow=false)
    let mut cont_line = false; // For handling partial lines
    let mut partial_line = String::new();

    // Search and filter state
    let mut filter_text = String::new();
    let mut search_text = String::new();
    let mut filter_active = false;
    let mut search_active = false;

    // Get filtered lines
    let get_filtered_lines = |lines: &[String], filter: &str| -> Vec<usize> {
        if filter.is_empty() {
            (0..lines.len()).collect()
        } else {
            let filter_lower = filter.to_lowercase();
            lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.to_lowercase().contains(&filter_lower))
                .map(|(i, _)| i)
                .collect()
        }
    };

    // Disable ncurses delay for responsive input
    crt.disable_delay();

    loop {
        let filtered_indices = get_filtered_lines(&lines, &filter_text);
        let panel_height = crt.height() - 2; // Title + function bar
        let panel_y = 1; // After title

        // Read new data from strace (non-blocking)
        if strace_alive && tracing {
            if let Some(ref mut child) = tracer_child {
                // Check if child is still running
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Child exited
                        strace_alive = false;
                    }
                    Ok(None) => {
                        // Still running, try to read output
                        if let Some(ref mut stderr) = child.stderr {
                            use std::io::Read;
                            use std::os::unix::io::AsRawFd;

                            let fd = stderr.as_raw_fd();

                            // Use select() to check if data is available before reading
                            // This is more reliable than relying solely on O_NONBLOCK
                            let has_data = unsafe {
                                use std::mem::MaybeUninit;
                                let mut fds = {
                                    let f = MaybeUninit::<libc::fd_set>::zeroed();
                                    f.assume_init()
                                };
                                libc::FD_ZERO(&mut fds);
                                libc::FD_SET(fd, &mut fds);

                                let mut timeout = libc::timeval {
                                    tv_sec: 0,
                                    tv_usec: 0, // No wait, just poll
                                };

                                libc::select(
                                    fd + 1,
                                    &mut fds,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    &mut timeout,
                                ) > 0
                            };

                            if has_data {
                                let mut buf = [0u8; 4096];

                                // Read available data (non-blocking due to O_NONBLOCK)
                                // Only do one read per loop iteration to stay responsive
                                match stderr.read(&mut buf) {
                                    Ok(0) => {
                                        // EOF - child closed stderr
                                    }
                                    Ok(n) => {
                                        // Got some data, process it
                                        let data = String::from_utf8_lossy(&buf[..n]);

                                        // Prepend any partial line from previous read
                                        let full_data = if cont_line {
                                            cont_line = false;
                                            let combined = partial_line.clone() + &data;
                                            partial_line.clear();
                                            combined
                                        } else {
                                            data.to_string()
                                        };

                                        // Split into lines
                                        let mut remaining = full_data.as_str();
                                        while let Some(newline_pos) = remaining.find('\n') {
                                            let line = &remaining[..newline_pos];
                                            lines.push(line.to_string());
                                            remaining = &remaining[newline_pos + 1..];
                                        }

                                        // Save any remaining partial line
                                        if !remaining.is_empty() {
                                            partial_line = remaining.to_string();
                                            cont_line = true;
                                        }
                                    }
                                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                        // No data available right now, that's fine
                                    }
                                    Err(_) => {
                                        // Read error
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        strace_alive = false;
                    }
                }
            }

            // Auto-scroll if following
            if follow && !filtered_indices.is_empty() {
                selected = (filtered_indices.len() as i32 - 1).max(0);
            }
        }

        // Clamp selection and scroll
        let max_selected = (filtered_indices.len() as i32 - 1).max(0);
        selected = selected.clamp(0, max_selected);

        if selected < scroll_v {
            scroll_v = selected;
        } else if selected >= scroll_v + panel_height {
            scroll_v = selected - panel_height + 1;
        }
        // Clamp scroll_v to valid range
        let max_scroll = (filtered_indices.len() as i32 - panel_height).max(0);
        scroll_v = scroll_v.clamp(0, max_scroll);

        // Draw title (like C htop InfoScreen_drawTitled)
        let title_attr = crt.color(ColorElement::MeterText);
        let default_color_attr = crt.color(ColorElement::DefaultColor);
        let title = format!("Trace of process {} - {}", pid, command);
        let title_display: String = title.chars().take(crt.width() as usize).collect();
        let screen_width = crt.width();

        crt.mv(0, 0);
        crt.attrset(title_attr);
        crt.hline(0, 0, ' ' as u32, screen_width);
        crt.addstr_raw(&title_display);
        crt.attrset(default_color_attr);

        // Draw lines
        let default_attr = crt.color(ColorElement::DefaultColor);
        let selection_attr = crt.color(ColorElement::PanelSelectionFocus);

        for row in 0..panel_height {
            let y = panel_y + row;
            let line_idx = (scroll_v + row) as usize;

            if line_idx < filtered_indices.len() {
                let actual_idx = filtered_indices[line_idx];
                let line = &lines[actual_idx];
                let is_selected = (scroll_v + row) == selected;

                let attr = if is_selected {
                    selection_attr
                } else {
                    default_attr
                };
                let display_line: String = line.chars().take(screen_width as usize).collect();
                crt.mv(y, 0);
                crt.attrset(attr);
                crt.hline(y, 0, ' ' as u32, screen_width);
                crt.addstr_raw(&display_line);
                crt.attrset(A_NORMAL);
            } else {
                crt.mv(y, 0);
                crt.attrset(default_attr);
                crt.hline(y, 0, ' ' as u32, screen_width);
                crt.attrset(A_NORMAL);
            }
        }

        // Draw function bar or search/filter bar (matches C htop TraceScreen)
        // F3=Search, F4=Filter, F8=AutoScroll, F9=Stop/Resume Tracing, Esc=Done
        let fb_y = crt.height() - 1;

        if search_active || filter_active {
            let bar_attr = crt.color(ColorElement::FunctionBar);
            let key_attr = crt.color(ColorElement::FunctionKey);
            let search_text_clone = search_text.clone();
            let filter_text_clone = filter_text.clone();

            crt.mv(fb_y, 0);
            crt.attrset(bar_attr);
            crt.hline(fb_y, 0, ' ' as u32, screen_width);
            crt.attrset(A_NORMAL);
            crt.mv(fb_y, 0);

            if search_active {
                // Function hints for search mode
                crt.attrset(key_attr);
                crt.addstr_raw("F3");
                crt.attrset(bar_attr);
                crt.addstr_raw("Next  ");
                crt.attrset(key_attr);
                crt.addstr_raw("S-F3");
                crt.attrset(bar_attr);
                crt.addstr_raw("Prev  ");
                crt.attrset(key_attr);
                crt.addstr_raw("Esc");
                crt.attrset(bar_attr);
                crt.addstr_raw("Cancel ");
                // Spacer (visual separator)
                crt.attrset(key_attr);
                crt.addstr_raw("  ");
                // Search label and text
                crt.attrset(bar_attr);
                crt.addstr_raw(" Search: ");
                crt.addstr_raw(&search_text_clone);
                crt.attrset(A_NORMAL);
            } else {
                // Function hints for filter mode
                crt.attrset(key_attr);
                crt.addstr_raw("Enter");
                crt.attrset(bar_attr);
                crt.addstr_raw("Done  ");
                crt.attrset(key_attr);
                crt.addstr_raw("Esc");
                crt.attrset(bar_attr);
                crt.addstr_raw("Clear ");
                // Spacer (visual separator)
                crt.attrset(key_attr);
                crt.addstr_raw("  ");
                // Filter label and text
                crt.attrset(bar_attr);
                crt.addstr_raw(" Filter: ");
                crt.addstr_raw(&filter_text_clone);
                crt.attrset(A_NORMAL);
            }
        } else {
            let trace_label = if tracing {
                "Stop Tracing   "
            } else {
                "Resume Tracing "
            };
            let scroll_label = if follow { "AutoScroll " } else { "Manual     " };
            let f4_label = if filter_text.is_empty() {
                "Filter "
            } else {
                "FILTER "
            };
            let fb = FunctionBar::with_functions(vec![
                ("F3".to_string(), "Search ".to_string()),
                ("F4".to_string(), f4_label.to_string()),
                ("F8".to_string(), scroll_label.to_string()),
                ("F9".to_string(), trace_label.to_string()),
                ("Esc".to_string(), "Done   ".to_string()),
            ]);
            fb.draw_simple(crt, fb_y);
        }

        crt.refresh();

        // Read input in non-blocking mode with fast escape handling
        // We use nodelay mode so getch returns immediately if no input
        let mut ch = match crt.read_key_nonblocking() {
            Some(k) => k,
            None => {
                // No input available, small sleep to avoid busy-waiting
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        };

        // Handle mouse events
        if ch == KEY_MOUSE {
            let screen_height = crt.height();
            if let Some(event) = crt.get_mouse_event() {
                if event.is_left_click() && event.y == screen_height - 1 {
                    // Click on function bar - determine which button
                    if !(search_active || filter_active) {
                        // Function bar: F3 Search, F4 Filter, F8 AutoScroll, F9 Stop/Resume, Esc Done
                        // Each button is ~10 chars wide
                        let x = event.x;
                        if x < 10 {
                            ch = KEY_F3;
                        } else if x < 20 {
                            ch = KEY_F4;
                        } else if x < 34 {
                            ch = KEY_F8;
                        } else if x < 52 {
                            ch = KEY_F9;
                        } else {
                            ch = 0x1B; // Esc
                        }
                    }
                } else if event.is_wheel_up() {
                    ch = KEY_WHEELUP;
                } else if event.is_wheel_down() {
                    ch = KEY_WHEELDOWN;
                }
            }
        }

        // Handle search/filter mode input
        if search_active || filter_active {
            match ch {
                27 => {
                    // Escape - cancel search, clear filter
                    if search_active {
                        search_text.clear();
                    } else if filter_active {
                        filter_text.clear();
                        selected = 0;
                        scroll_v = 0;
                    }
                    search_active = false;
                    filter_active = false;
                }
                10 | KEY_ENTER => {
                    // Enter - confirm and exit mode (keep filter text)
                    search_active = false;
                    filter_active = false;
                }
                x if x == KEY_F3 && search_active => {
                    // F3 - find next match
                    if !search_text.is_empty() {
                        let search_lower = search_text.to_lowercase();
                        let start = (selected + 1) as usize;
                        let len = filtered_indices.len();
                        // Search from current+1 to end, then wrap to beginning
                        for offset in 0..len {
                            let i = (start + offset) % len;
                            if let Some(&idx) = filtered_indices.get(i) {
                                if let Some(line) = lines.get(idx) {
                                    if line.to_lowercase().contains(&search_lower) {
                                        selected = i as i32;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                x if x == KEY_F15 && search_active => {
                    // Shift-F3 - find previous match
                    if !search_text.is_empty() {
                        let search_lower = search_text.to_lowercase();
                        let len = filtered_indices.len();
                        if len > 0 {
                            let start = if selected > 0 {
                                (selected - 1) as usize
                            } else {
                                len - 1
                            };
                            // Search backwards with wrap
                            for offset in 0..len {
                                let i = (start + len - offset) % len;
                                if let Some(&idx) = filtered_indices.get(i) {
                                    if let Some(line) = lines.get(idx) {
                                        if line.to_lowercase().contains(&search_lower) {
                                            selected = i as i32;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                KEY_BACKSPACE | 127 | 8 => {
                    if search_active && !search_text.is_empty() {
                        search_text.pop();
                    } else if filter_active && !filter_text.is_empty() {
                        filter_text.pop();
                        selected = 0;
                        scroll_v = 0;
                    }
                }
                _ if (32..127).contains(&ch) => {
                    // Printable character
                    let c = char::from_u32(ch as u32).unwrap_or(' ');
                    if search_active {
                        search_text.push(c);
                        // Incremental search - find next match
                        let search_lower = search_text.to_lowercase();
                        for (i, idx) in filtered_indices.iter().enumerate() {
                            if lines[*idx].to_lowercase().contains(&search_lower) {
                                selected = i as i32;
                                break;
                            }
                        }
                    } else if filter_active {
                        filter_text.push(c);
                        selected = 0;
                        scroll_v = 0;
                    }
                }
                _ => {}
            }
            continue;
        }

        match ch {
            KEY_ESC | KEY_LC_Q | KEY_F10 => break, // Esc, 'q', or F10 - exit
            x if x == KEY_F3 => {
                // F3 - search
                search_active = true;
                search_text.clear();
            }
            KEY_SLASH => {
                // '/' - search
                search_active = true;
                search_text.clear();
            }
            x if x == KEY_F4 => {
                // F4 - filter
                filter_active = true;
            }
            KEY_BACKSLASH => {
                // '\' - filter
                filter_active = true;
            }
            KEY_F8 | KEY_LC_F => {
                // F8 or 'f' - toggle auto-scroll/follow
                follow = !follow;
                if follow && !filtered_indices.is_empty() {
                    selected = (filtered_indices.len() as i32 - 1).max(0);
                }
            }
            KEY_F9 | KEY_LC_T => {
                // F9 or 't' - toggle tracing
                tracing = !tracing;
            }
            KEY_UP | KEY_CTRL_P => {
                follow = false; // Manual navigation disables follow
                if selected > 0 {
                    selected -= 1;
                }
            }
            KEY_DOWN | KEY_CTRL_N => {
                follow = false;
                selected += 1;
            }
            KEY_PPAGE => {
                follow = false;
                selected = (selected - panel_height).max(0);
            }
            KEY_NPAGE => {
                follow = false;
                selected = (selected + panel_height).min(max_selected);
            }
            KEY_HOME => {
                follow = false;
                selected = 0;
            }
            KEY_END => {
                follow = false;
                selected = max_selected;
            }
            KEY_WHEELUP => {
                follow = false;
                selected = (selected - 3).max(0);
            }
            KEY_WHEELDOWN => {
                follow = false;
                selected = (selected + 3).min(max_selected);
            }
            _ => {}
        }
    }

    // Cleanup: kill the tracer child process
    if let Some(ref mut child) = tracer_child {
        let _ = child.kill();
        let _ = child.wait();
    }

    crt.enable_delay();
}

/// Show command screen (like C htop CommandScreen)
pub fn show_command_screen(crt: &mut Crt, pid: i32, command: &str) {
    // Wrap command into lines at word boundaries (like C htop CommandScreen_scan)
    let wrap_command = |cmd: &str, max_width: usize| -> Vec<String> {
        let max_width = max_width.max(40);
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut last_space = 0usize;

        for ch in cmd.chars() {
            if line.len() >= max_width {
                // Need to wrap
                let line_len = if last_space > 0 {
                    last_space
                } else {
                    line.len()
                };
                let (first, rest) = line.split_at(line_len);
                lines.push(first.to_string());
                line = rest.trim_start().to_string();
                last_space = 0;
            }

            line.push(ch);
            if ch == ' ' {
                last_space = line.len();
            }
        }

        if !line.is_empty() {
            lines.push(line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    };

    // Build wrapped lines
    let mut lines = wrap_command(command, crt.width() as usize);

    // State for the info screen
    let mut selected = 0i32;
    let mut scroll_v = 0i32;
    let mut sf_state = SearchFilterState::new();

    loop {
        let filtered_indices = sf_state.filter_indices(&lines);
        let panel_height = crt.height() - 2; // Title + function bar
        let panel_y = 1; // After title

        // Clamp selection and scroll
        let max_selected = (filtered_indices.len() as i32 - 1).max(0);
        selected = selected.clamp(0, max_selected);

        if selected < scroll_v {
            scroll_v = selected;
        } else if selected >= scroll_v + panel_height {
            scroll_v = selected - panel_height + 1;
        }
        // Clamp scroll_v to valid range
        let max_scroll = (filtered_indices.len() as i32 - panel_height).max(0);
        scroll_v = scroll_v.clamp(0, max_scroll);

        // Draw title (like C htop InfoScreen_drawTitled)
        let title_attr = crt.color(ColorElement::MeterText);
        let default_color_attr = crt.color(ColorElement::DefaultColor);
        let title = format!("Command of process {} - {}", pid, command);
        let title_display: String = title.chars().take(crt.width() as usize).collect();
        let screen_width = crt.width();

        crt.mv(0, 0);
        crt.attrset(title_attr);
        crt.hline(0, 0, ' ' as u32, screen_width);
        crt.addstr_raw(&title_display);
        crt.attrset(default_color_attr);

        // Draw lines
        let default_attr = crt.color(ColorElement::DefaultColor);
        let selection_attr = crt.color(ColorElement::PanelSelectionFocus);

        for row in 0..panel_height {
            let y = panel_y + row;
            let line_idx = (scroll_v + row) as usize;

            if line_idx < filtered_indices.len() {
                let actual_idx = filtered_indices[line_idx];
                let line = &lines[actual_idx];
                let is_selected = (scroll_v + row) == selected;

                let attr = if is_selected {
                    selection_attr
                } else {
                    default_attr
                };
                let display_line: String = line.chars().take(screen_width as usize).collect();
                crt.mv(y, 0);
                crt.attrset(attr);
                crt.hline(y, 0, ' ' as u32, screen_width);
                crt.addstr_raw(&display_line);
                crt.attrset(A_NORMAL);
            } else {
                crt.mv(y, 0);
                crt.attrset(default_attr);
                crt.hline(y, 0, ' ' as u32, screen_width);
                crt.attrset(A_NORMAL);
            }
        }

        // Draw function bar or search/filter bar
        let fb_y = crt.height() - 1;
        sf_state.draw_bar(crt, fb_y, screen_width);

        crt.refresh();

        // Handle input
        crt.set_blocking(true);
        let mut ch = crt.getch();

        // Handle mouse events
        ch = process_mouse_event(crt, &sf_state, ch);

        // Handle search/filter input
        if sf_state.is_active() {
            use HandleResult;
            match sf_state.handle_input(ch) {
                HandleResult::SearchChanged => {
                    if let Some(idx) = sf_state.find_first_match(&lines, &filtered_indices) {
                        selected = idx as i32;
                    }
                }
                HandleResult::FilterChanged => {
                    selected = 0;
                    scroll_v = 0;
                }
                HandleResult::SearchNext => {
                    // F3 - find next match
                    if let Some(idx) =
                        sf_state.find_next_match(&lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::SearchPrev => {
                    // S-F3 - find previous match
                    if let Some(idx) =
                        sf_state.find_prev_match(&lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::Handled | HandleResult::NotHandled => {}
            }
            continue;
        }

        // Handle normal keys
        if sf_state.handle_start_key(ch) {
            continue;
        }

        match ch {
            27 | 113 | KEY_F10 => break, // Esc, 'q', or F10 - exit
            x if x == KEY_F5 => {
                // F5 - refresh (re-wrap at current width, preserve selection)
                let saved_selected = selected;
                lines = wrap_command(command, crt.width() as usize);
                let max_idx = (lines.len() as i32 - 1).max(0);
                selected = saved_selected.min(max_idx);
                crt.clear();
            }
            KEY_CTRL_L => {
                // Ctrl+L - refresh screen
                crt.clear();
            }
            KEY_UP | KEY_CTRL_P => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            KEY_DOWN | KEY_CTRL_N => {
                selected += 1;
            }
            KEY_PPAGE => {
                selected = (selected - panel_height).max(0);
            }
            KEY_NPAGE => {
                selected = (selected + panel_height).min(max_selected);
            }
            KEY_HOME => {
                selected = 0;
            }
            KEY_END => {
                selected = max_selected;
            }
            KEY_WHEELUP => {
                selected = (selected - 3).max(0);
            }
            KEY_WHEELDOWN => {
                selected = (selected + 3).min(max_selected);
            }
            _ => {}
        }
    }

    crt.enable_delay();
}
