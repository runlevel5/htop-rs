//! Menu screens for htop-rs
//!
//! This module contains popup menu screens extracted from ScreenManager:
//! - Help screen (F1/h/?)
//! - Kill signal menu (F9/k)
//! - Sort column menu (F6/>.)
//! - User filter menu (u)

use super::crt::ColorElement;
use super::function_bar::FunctionBar;
use super::header::Header;
use super::main_panel::MainPanel;
use super::panel::Panel;
use super::Crt;
use crate::core::{Machine, ProcessField, Settings};

/// Convert SPDX license identifier to display string
fn license_display() -> &'static str {
    const LICENSE_SPDX: &str = env!("CARGO_PKG_LICENSE");
    match LICENSE_SPDX {
        "GPL-2.0-or-later" => "GNU GPLv2+",
        "GPL-2.0" | "GPL-2.0-only" => "GNU GPLv2",
        "GPL-3.0-or-later" => "GNU GPLv3+",
        "GPL-3.0" | "GPL-3.0-only" => "GNU GPLv3",
        "MIT" => "MIT License",
        "Apache-2.0" => "Apache License 2.0",
        _ => LICENSE_SPDX,
    }
}

/// Show help screen (matches C htop actionHelp)
#[allow(unused_must_use)]
pub fn show_help(crt: &mut Crt, settings: &Settings) {
    crt.clear();

    let default_color = crt.color(ColorElement::DefaultColor);
    let bold = crt.color(ColorElement::HelpBold);
    let bar_border = crt.color(ColorElement::BarBorder);
    let bar_shadow = crt.color(ColorElement::BarShadow);
    let cpu_nice = crt.color(ColorElement::CpuNice);
    let cpu_normal = crt.color(ColorElement::CpuNormal);
    let cpu_system = crt.color(ColorElement::CpuSystem);
    let cpu_guest = crt.color(ColorElement::CpuGuest);
    let mem_used = crt.color(ColorElement::MemoryUsed);
    let mem_shared = crt.color(ColorElement::MemoryShared);
    let mem_compressed = crt.color(ColorElement::MemoryCompressed);
    let mem_buffers = crt.color(ColorElement::MemoryBuffersText);
    let mem_cache = crt.color(ColorElement::MemoryCache);
    let swap_color = crt.color(ColorElement::Swap);
    let process_run = crt.color(ColorElement::ProcessRunState);
    let process_shadow = crt.color(ColorElement::ProcessShadow);
    let process_d = crt.color(ColorElement::ProcessDState);
    let process_thread = crt.color(ColorElement::ProcessThread);
    let help_shadow = crt.color(ColorElement::HelpShadow);
    let theme_help_text = crt.theme_help_text();
    let height = crt.height();
    let width = crt.width();
    let readonly = settings.readonly;

    // Prepare title strings
    let title1 = format!("htop {} - (C) 2026 Trung Le.", env!("CARGO_PKG_VERSION"));
    let title2 = format!(
        "Released under the {}. See 'man' page for more info.",
        license_display()
    );

    // Left column items: key at col 1, info at col 10
    let help_left = [
        ("      #: ", "hide/show header meters", false),
        ("    Tab: ", "switch to next screen tab", false),
        (" Arrows: ", "scroll process list", false),
        (" Digits: ", "incremental PID search", false),
        ("   F3 /: ", "incremental name search", false),
        ("   F4 \\: ", "incremental name filtering", false),
        ("   F5 t: ", "tree view", false),
        ("      p: ", "toggle program path", false),
        ("      m: ", "toggle merged command", false),
        ("      Z: ", "pause/resume process updates", false),
        ("      u: ", "show processes of a single user", false),
        ("      H: ", "hide/show user process threads", false),
        ("      K: ", "hide/show kernel threads", false),
        ("      O: ", "hide/show processes in containers", false),
        ("      F: ", "cursor follows process", false),
        ("  + - *: ", "expand/collapse tree/toggle all", false),
        ("N P M T: ", "sort by PID, CPU%, MEM% or TIME", false),
        ("      I: ", "invert sort order", false),
        (" F6 > .: ", "select sort column", false),
    ];

    // Right column items: key at col 43, info at col 52
    let help_right = [
        ("  S-Tab: ", "switch to previous screen tab", false),
        ("  Space: ", "tag process", false),
        ("      c: ", "tag process and its children", false),
        ("      U: ", "untag all processes", false),
        ("   F9 k: ", "kill process/tagged processes", true),
        ("   F7 ]: ", "higher priority (- nice)", true),
        ("   F8 [: ", "lower priority (+ nice)", true),
        ("      e: ", "show process environment", false),
        ("      i: ", "set IO priority", true),
        ("      l: ", "list open files with lsof", true),
        ("      x: ", "list file locks of process", false),
        ("      s: ", "trace syscalls with strace", true),
        ("      w: ", "wrap process command in multiple lines", false),
        (" F2 C S: ", "setup", false),
        (" F1 h ?: ", "show this help screen", false),
        ("  F10 q: ", "quit", false),
    ];

    // Fill screen with HELP_BOLD background (like C htop)
    crt.attrset(bold);
    for i in 0..height - 1 {
        crt.mv(i, 0);
        for _ in 0..width {
            crt.addch_raw(' ' as u32);
        }
    }

    let mut line = 0;

    // Title
    crt.attrset(bold);
    crt.mvaddstr_raw(line, 0, &title1);
    line += 1;
    crt.mvaddstr_raw(line, 0, &title2);
    line += 2;

    // CPU usage bar legend
    crt.attrset(default_color);
    crt.mvaddstr_raw(line, 0, "CPU usage bar: ");
    crt.attrset(bar_border);
    crt.addstr_raw("[");
    crt.attrset(cpu_nice);
    crt.addstr_raw("low");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(cpu_normal);
    crt.addstr_raw("normal");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(cpu_system);
    crt.addstr_raw("kernel");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(cpu_guest);
    crt.addstr_raw("guest");
    crt.attrset(default_color);
    crt.addstr_raw("                            "); // 28 spaces
    crt.attrset(bar_shadow);
    crt.addstr_raw("used%");
    crt.attrset(bar_border);
    crt.addstr_raw("]");
    line += 1;

    // Memory bar legend
    crt.attrset(default_color);
    crt.mvaddstr_raw(line, 0, "Memory bar:    ");
    crt.attrset(bar_border);
    crt.addstr_raw("[");
    crt.attrset(mem_used);
    crt.addstr_raw("used");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(mem_shared);
    crt.addstr_raw("shared");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(mem_compressed);
    crt.addstr_raw("compressed");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(mem_buffers);
    crt.addstr_raw("buffers");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(mem_cache);
    crt.addstr_raw("cache");
    crt.attrset(default_color);
    crt.addstr_raw("          "); // 10 spaces
    crt.attrset(bar_shadow);
    crt.addstr_raw("used");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(bar_shadow);
    crt.addstr_raw("total");
    crt.attrset(bar_border);
    crt.addstr_raw("]");
    line += 1;

    // Swap bar legend
    crt.attrset(default_color);
    crt.mvaddstr_raw(line, 0, "Swap bar:      ");
    crt.attrset(bar_border);
    crt.addstr_raw("[");
    crt.attrset(swap_color);
    crt.addstr_raw("used");
    crt.attrset(default_color);
    crt.addstr_raw("                                          "); // 42 spaces
    crt.attrset(bar_shadow);
    crt.addstr_raw("used");
    crt.attrset(default_color);
    crt.addstr_raw("/");
    crt.attrset(bar_shadow);
    crt.addstr_raw("total");
    crt.attrset(bar_border);
    crt.addstr_raw("]");
    line += 2;

    // Info about meter configuration
    crt.attrset(default_color);
    crt.mvaddstr_raw(
        line,
        0,
        "Type and layout of header meters are configurable in the setup screen.",
    );
    line += 1;

    // Theme-specific help text (e.g., monochrome bar character explanation)
    if let Some(help_text) = theme_help_text {
        crt.mvaddstr_raw(line, 0, help_text);
        line += 1;
    }
    line += 1;

    // Process state legend
    crt.attrset(default_color);
    crt.mvaddstr_raw(line, 0, "Process state: ");
    crt.attrset(process_run);
    crt.addstr_raw("R");
    crt.attrset(default_color);
    crt.addstr_raw(": running; ");
    crt.attrset(process_shadow);
    crt.addstr_raw("S");
    crt.attrset(default_color);
    crt.addstr_raw(": sleeping; ");
    crt.attrset(process_run);
    crt.addstr_raw("t");
    crt.attrset(default_color);
    crt.addstr_raw(": traced/stopped; ");
    crt.attrset(process_d);
    crt.addstr_raw("Z");
    crt.attrset(default_color);
    crt.addstr_raw(": zombie; ");
    crt.attrset(process_d);
    crt.addstr_raw("D");
    crt.attrset(default_color);
    crt.addstr_raw(": disk sleep");
    line += 2;

    let start_line = line;

    // Draw left column
    for (i, (key, info, ro_inactive)) in help_left.iter().enumerate() {
        let inactive = *ro_inactive && readonly;
        let key_attr = if inactive { help_shadow } else { bold };
        let info_attr = if inactive { help_shadow } else { default_color };

        crt.attrset(key_attr);
        crt.mvaddstr_raw(start_line + i as i32, 1, key);
        crt.attrset(info_attr);
        crt.mvaddstr_raw(start_line + i as i32, 10, info);

        // Special coloring for "threads" keyword
        let thread_color = if inactive {
            help_shadow
        } else {
            process_thread
        };
        if *key == "      H: " {
            crt.attrset(thread_color);
            crt.mvaddstr_raw(start_line + i as i32, 33, "threads");
        } else if *key == "      K: " {
            crt.attrset(thread_color);
            crt.mvaddstr_raw(start_line + i as i32, 27, "threads");
        }
    }

    // Draw right column
    for (i, (key, info, ro_inactive)) in help_right.iter().enumerate() {
        let inactive = *ro_inactive && readonly;
        let key_attr = if inactive { help_shadow } else { bold };
        let info_attr = if inactive { help_shadow } else { default_color };

        crt.attrset(key_attr);
        crt.mvaddstr_raw(start_line + i as i32, 43, key);
        crt.attrset(info_attr);
        crt.mvaddstr_raw(start_line + i as i32, 52, info);
    }

    line = start_line + help_left.len().max(help_right.len()) as i32 + 1;

    // "Press any key to return"
    crt.attrset(bold);
    crt.mvaddstr_raw(line, 0, "Press any key to return.");
    crt.attrset(default_color);

    crt.refresh();

    // Wait for key - disable timeout so we block until key press
    // (matches C htop CRT_readKey behavior)
    crt.set_blocking(true);
    crt.getch();
    // Re-enable delay for main loop
    crt.enable_delay();
}

/// Context for kill menu
pub struct KillMenuContext<'a> {
    pub main_panel: &'a mut MainPanel,
    pub header: &'a Header,
    pub settings: &'a Settings,
    pub hide_meters: bool,
}

/// Show kill signal selection menu (matches C htop SignalsPanel)
pub fn show_kill_menu(crt: &mut Crt, machine: &mut Machine, ctx: &mut KillMenuContext, pid: i32) {
    // Platform-specific signals list (from C htop Platform.c for each OS)
    // Format matches C htop: " N SIGNAME" with leading space for single digits
    #[cfg(target_os = "linux")]
    let base_signals: &[(&str, i32)] = &[
        (" 0 Cancel", 0),
        (" 1 SIGHUP", 1),
        (" 2 SIGINT", 2),
        (" 3 SIGQUIT", 3),
        (" 4 SIGILL", 4),
        (" 5 SIGTRAP", 5),
        (" 6 SIGABRT", 6),
        (" 6 SIGIOT", 6),
        (" 7 SIGBUS", 7),
        (" 8 SIGFPE", 8),
        (" 9 SIGKILL", 9),
        ("10 SIGUSR1", 10),
        ("11 SIGSEGV", 11),
        ("12 SIGUSR2", 12),
        ("13 SIGPIPE", 13),
        ("14 SIGALRM", 14),
        ("15 SIGTERM", 15),
        ("16 SIGSTKFLT", 16),
        ("17 SIGCHLD", 17),
        ("18 SIGCONT", 18),
        ("19 SIGSTOP", 19),
        ("20 SIGTSTP", 20),
        ("21 SIGTTIN", 21),
        ("22 SIGTTOU", 22),
        ("23 SIGURG", 23),
        ("24 SIGXCPU", 24),
        ("25 SIGXFSZ", 25),
        ("26 SIGVTALRM", 26),
        ("27 SIGPROF", 27),
        ("28 SIGWINCH", 28),
        ("29 SIGIO", 29),
        ("29 SIGPOLL", 29),
        ("30 SIGPWR", 30),
        ("31 SIGSYS", 31),
    ];

    #[cfg(target_os = "macos")]
    let base_signals: &[(&str, i32)] = &[
        (" 0 Cancel", 0),
        (" 1 SIGHUP", 1),
        (" 2 SIGINT", 2),
        (" 3 SIGQUIT", 3),
        (" 4 SIGILL", 4),
        (" 5 SIGTRAP", 5),
        (" 6 SIGABRT", 6),
        (" 6 SIGIOT", 6),
        (" 7 SIGEMT", 7),
        (" 8 SIGFPE", 8),
        (" 9 SIGKILL", 9),
        ("10 SIGBUS", 10),
        ("11 SIGSEGV", 11),
        ("12 SIGSYS", 12),
        ("13 SIGPIPE", 13),
        ("14 SIGALRM", 14),
        ("15 SIGTERM", 15),
        ("16 SIGURG", 16),
        ("17 SIGSTOP", 17),
        ("18 SIGTSTP", 18),
        ("19 SIGCONT", 19),
        ("20 SIGCHLD", 20),
        ("21 SIGTTIN", 21),
        ("22 SIGTTOU", 22),
        ("23 SIGIO", 23),
        ("24 SIGXCPU", 24),
        ("25 SIGXFSZ", 25),
        ("26 SIGVTALRM", 26),
        ("27 SIGPROF", 27),
        ("28 SIGWINCH", 28),
        ("29 SIGINFO", 29),
        ("30 SIGUSR1", 30),
        ("31 SIGUSR2", 31),
    ];

    // Build signal list with optional real-time signals (Linux only)
    #[allow(unused_mut)]
    let mut signals: Vec<(String, i32)> = base_signals
        .iter()
        .map(|(name, num)| (name.to_string(), *num))
        .collect();

    // Add real-time signals on Linux (SIGRTMIN to SIGRTMAX)
    // These are determined at runtime, typically 34-64 on Linux
    #[cfg(target_os = "linux")]
    {
        // SIGRTMIN and SIGRTMAX are functions in glibc, not constants
        // libc::SIGRTMIN() returns the minimum real-time signal number
        // libc::SIGRTMAX() returns the maximum real-time signal number
        let rtmin = libc::SIGRTMIN();
        let rtmax = libc::SIGRTMAX();

        // Safety check: only add if range is reasonable (C htop checks <= 100)
        if rtmax - rtmin <= 100 {
            for sig in rtmin..=rtmax {
                let n = sig - rtmin;
                let name = if n == 0 {
                    format!("{:2} SIGRTMIN", sig)
                } else {
                    format!("{:2} SIGRTMIN+{}", sig, n)
                };
                signals.push((name, sig));
            }
        }
    }

    // Create signal panel (matches C htop SignalsPanel_new)
    // C htop uses width 14 in Action_pickFromVector
    // We use 15 to accommodate "64 SIGRTMIN+30" (14 chars) plus padding
    let signal_panel_width = 15i32;
    let panel_y = ctx.main_panel.y;
    let panel_height = crt.height() - panel_y - 1; // Leave room for function bar

    let mut signal_panel = Panel::new(0, panel_y, signal_panel_width, panel_height);
    signal_panel.set_header("Send signal:");
    signal_panel.function_bar = FunctionBar::new_enter_esc("Send   ", "Cancel ");

    // Add all signals and find SIGTERM (15) for default selection
    let mut default_position = 0i32;
    for (i, (name, number)) in signals.iter().enumerate() {
        signal_panel.add_list_item(name, *number);
        // Signal 15 (SIGTERM) is the default, but it's not always at index 15
        if *number == 15 {
            default_position = i as i32;
        }
    }
    signal_panel.set_selected(default_position);

    // Run the side panel menu
    let mut side_ctx = super::side_panel_menu::SidePanelContext {
        main_panel: ctx.main_panel,
        header: ctx.header,
        settings: ctx.settings,
        hide_meters: ctx.hide_meters,
    };

    let result =
        super::side_panel_menu::run_side_panel_menu(crt, machine, &mut side_ctx, &mut signal_panel);

    // Send the signal if one was selected
    if let super::side_panel_menu::SidePanelResult::Selected(selected_idx) = result {
        if selected_idx < signals.len() {
            let (_, sig_num) = signals[selected_idx];
            // Signal 0 means "Cancel" in C htop
            if sig_num != 0 {
                send_signal(pid, sig_num);
            }
        }
    }
}

/// Send signal to process
pub fn send_signal(pid: i32, signal: i32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid, signal);
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
    }
}

/// Context for sort menu
pub struct SortMenuContext<'a> {
    pub main_panel: &'a mut MainPanel,
    pub header: &'a Header,
    pub settings: &'a Settings,
    pub hide_meters: bool,
}

/// Result of sort menu selection
pub struct SortMenuResult {
    /// The selected field, if any
    pub field: Option<ProcessField>,
}

/// Show sort column selection menu (matches C htop actionSetSortColumn)
///
/// Returns the selected field if user made a selection, None if cancelled.
pub fn show_sort_menu(
    crt: &mut Crt,
    machine: &mut Machine,
    ctx: &mut SortMenuContext,
    tree_view: bool,
    current_sort_key: ProcessField,
    fields: &[ProcessField],
) -> SortMenuResult {
    // Determine the active sort key - in tree view, it's always PID
    let active_sort_key = if tree_view {
        ProcessField::Pid
    } else {
        current_sort_key
    };

    // Create the sort panel (matches C htop Panel_new with FunctionBar_newEnterEsc)
    // C htop uses width 14 in Action_pickFromVector
    let sort_panel_width = 14i32;
    let panel_y = ctx.main_panel.y;
    let panel_height = crt.height() - panel_y - 1; // Leave room for function bar

    let mut sort_panel = Panel::new(0, panel_y, sort_panel_width, panel_height);
    sort_panel.set_header("Sort by");
    sort_panel.function_bar = FunctionBar::new_enter_esc("Sort   ", "Cancel ");

    // Add fields from the currently displayed columns (like C htop)
    // C htop uses settings->ss->fields, we use the provided fields
    let mut current_selection = 0i32;
    for (i, field) in fields.iter().enumerate() {
        // Get the field name (trimmed, like C htop String_trim)
        let name = field.name().trim();
        sort_panel.add_list_item(name, *field as i32);

        // Pre-select the current sort key
        if *field == active_sort_key {
            current_selection = i as i32;
        }
    }
    sort_panel.set_selected(current_selection);

    // Run the side panel menu
    let mut side_ctx = super::side_panel_menu::SidePanelContext {
        main_panel: ctx.main_panel,
        header: ctx.header,
        settings: ctx.settings,
        hide_meters: ctx.hide_meters,
    };

    let result =
        super::side_panel_menu::run_side_panel_menu(crt, machine, &mut side_ctx, &mut sort_panel);

    // Return the selected field
    if let super::side_panel_menu::SidePanelResult::Selected(selected_idx) = result {
        if selected_idx < fields.len() {
            return SortMenuResult {
                field: Some(fields[selected_idx]),
            };
        }
    }

    SortMenuResult { field: None }
}

/// Context for user menu
pub struct UserMenuContext<'a> {
    pub main_panel: &'a mut MainPanel,
    pub header: &'a Header,
    pub settings: &'a Settings,
    pub hide_meters: bool,
}

/// Result of user menu selection
pub struct UserMenuResult {
    /// The selected user ID, or None if "All users" was selected or cancelled
    pub user_id: Option<Option<u32>>,
}

/// Show user selection menu (like C htop actionFilterByUser)
/// Displays a panel on the left side with the main process list on the right
///
/// Returns Some(None) for "All users", Some(Some(uid)) for a specific user,
/// or None if cancelled.
pub fn show_user_menu(
    crt: &mut Crt,
    machine: &mut Machine,
    ctx: &mut UserMenuContext,
) -> UserMenuResult {
    // Collect unique users from processes
    let mut users: Vec<(u32, String)> = machine
        .processes
        .iter()
        .filter_map(|p| p.user.as_ref().map(|u| (p.uid, u.clone())))
        .collect();
    users.sort_by(|a, b| a.1.cmp(&b.1));
    users.dedup_by(|a, b| a.0 == b.0);

    // Add "All users" at the top with uid 0 (we'll use value to distinguish)
    // Use i64::MIN as a sentinel value for "All users"
    let mut menu_items: Vec<(i64, String)> = vec![(i64::MIN, "All users".to_string())];
    menu_items.extend(users.into_iter().map(|(uid, name)| (uid as i64, name)));

    // Create the user panel (matches C htop Panel_new with FunctionBar_newEnterEsc)
    // C htop uses width 19 for user panel
    let user_panel_width = 19i32;
    let panel_y = ctx.main_panel.y;
    let panel_height = crt.height() - panel_y - 1; // Leave room for function bar

    let mut user_panel = Panel::new(0, panel_y, user_panel_width, panel_height);
    user_panel.set_header("Show processes of:");
    user_panel.function_bar = FunctionBar::new_enter_esc("Show   ", "Cancel ");

    // Add all users to the panel
    for (uid, name) in &menu_items {
        user_panel.add_list_item(name, *uid as i32);
    }

    // Run the side panel menu
    let mut side_ctx = super::side_panel_menu::SidePanelContext {
        main_panel: ctx.main_panel,
        header: ctx.header,
        settings: ctx.settings,
        hide_meters: ctx.hide_meters,
    };

    let result =
        super::side_panel_menu::run_side_panel_menu(crt, machine, &mut side_ctx, &mut user_panel);

    // Re-enable delay for main loop
    crt.enable_delay();

    // Return the selection
    if let super::side_panel_menu::SidePanelResult::Selected(selected_idx) = result {
        if selected_idx < menu_items.len() {
            let (uid, _) = menu_items[selected_idx];
            // i64::MIN means "All users" (no filter)
            if uid == i64::MIN {
                return UserMenuResult {
                    user_id: Some(None),
                };
            } else {
                return UserMenuResult {
                    user_id: Some(Some(uid as u32)),
                };
            }
        }
    }

    UserMenuResult { user_id: None }
}
