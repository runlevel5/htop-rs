//! htop-rs - A Rust port of htop, an interactive process viewer
//!
//! Copyright (C) 2026 Trung Le
//! Released under the GNU GPLv2+

mod core;
mod meters;
mod platform;
mod ui;

use anyhow::Result;
use clap::{ArgAction, Parser};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::{Machine, Settings};
use crate::ui::{Crt, Header, MainPanel, ScreenManager};

/// Static flag for clean shutdown
static RUNNING: AtomicBool = AtomicBool::new(true);

const VERSION: &str = env!("CARGO_PKG_VERSION");
const COPYRIGHT: &str = "(C) 2026 Trung Le.";
const LICENSE_SPDX: &str = env!("CARGO_PKG_LICENSE");

/// Convert SPDX license identifier to display string
fn license_display() -> &'static str {
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

fn print_version_full() {
    println!("htop {}", VERSION);
    println!("{}", COPYRIGHT);
    println!("Released under the {}.", license_display());
}

fn print_version() {
    println!("htop {}", VERSION);
}

fn print_help() {
    print_version_full();
    println!();
    println!("-C --no-color                   Use a monochrome color scheme");
    println!("-d --delay=DELAY                Set the delay between updates, in tenths of seconds");
    println!("-F --filter=FILTER              Show only the commands matching the given filter");
    println!("   --no-function-bar             Hide the function bar");
    println!("-h --help                       Print this help screen");
    println!("-H --highlight-changes[=DELAY]  Highlight new and old processes");
    println!("-M --no-mouse                   Disable the mouse");
    println!("   --no-meters                  Hide meters");
    println!("-n --max-iterations=NUMBER      Exit htop after NUMBER iterations/frame updates");
    println!("-p --pid=PID[,PID,PID...]       Show only the given PIDs");
    println!("   --readonly                   Disable all system and process changing features");
    println!("-s --sort-key=COLUMN            Sort by COLUMN in list view (try --sort-key=help for a list)");
    println!("-t --tree                       Show the tree view (can be combined with -s)");
    println!("-u --user[=USERNAME]            Show only processes for a given user (or $USER)");
    println!("-U --no-unicode                 Do not use unicode but plain ASCII");
    println!("-V --version                    Print version info");
    println!();
    println!("Press F1 inside htop for online help.");
    println!("See 'man htop' for more information.");
}

/// htop - an interactive process viewer
#[derive(Parser, Debug)]
#[command(name = "htop")]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
struct Args {
    /// Use a monochrome color scheme
    #[arg(short = 'C', long = "no-color")]
    no_color: bool,

    /// Set the delay between updates, in tenths of seconds
    #[arg(short = 'd', long = "delay", value_name = "DELAY")]
    delay: Option<u32>,

    /// Show only the commands matching the given filter
    #[arg(short = 'F', long = "filter", value_name = "FILTER")]
    filter: Option<String>,

    /// Highlight new and old processes
    #[arg(short = 'H', long = "highlight-changes", value_name = "DELAY")]
    highlight_changes: Option<Option<u32>>,

    /// Disable the mouse
    #[arg(short = 'M', long = "no-mouse")]
    no_mouse: bool,

    /// Exit htop after NUMBER iterations/frame updates
    #[arg(short = 'n', long = "max-iterations", value_name = "NUMBER")]
    max_iterations: Option<i64>,

    /// Show only the given PIDs
    #[arg(short = 'p', long = "pid", value_name = "PID", value_delimiter = ',')]
    pids: Option<Vec<u32>>,

    /// Disable all system and process changing features
    #[arg(long = "readonly")]
    readonly: bool,

    /// Sort by COLUMN in list view
    #[arg(short = 's', long = "sort-key", value_name = "COLUMN")]
    sort_key: Option<String>,

    /// Show the tree view
    #[arg(short = 't', long = "tree")]
    tree: bool,

    /// Show only processes for a given user
    #[arg(short = 'u', long = "user", value_name = "USERNAME")]
    user: Option<Option<String>>,

    /// Do not use unicode but plain ASCII
    #[arg(short = 'U', long = "no-unicode")]
    no_unicode: bool,

    /// Hide meters
    #[arg(long = "no-meters")]
    no_meters: bool,

    /// Hide the function bar
    #[arg(long = "no-function-bar")]
    no_function_bar: bool,

    /// Print this help screen
    #[arg(short = 'h', long = "help", action = ArgAction::SetTrue)]
    help: bool,

    /// Print version info
    #[arg(short = 'V', long = "version", action = ArgAction::SetTrue)]
    version: bool,
}

fn setup_signal_handlers() {
    // Set up Ctrl+C handler
    ctrlc_handler();
}

fn ctrlc_handler() {
    let _ = ctrlc::set_handler(move || {
        RUNNING.store(false, Ordering::SeqCst);
    });
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle help and version flags first
    if args.help {
        print_help();
        return Ok(());
    }
    if args.version {
        print_version();
        return Ok(());
    }

    // Set up signal handlers
    setup_signal_handlers();

    // Initialize platform-specific code
    platform::init()?;

    // Create the machine (system state)
    let user_id = args.user.as_ref().map(|u| {
        u.as_ref()
            .map(|name| platform::get_uid_for_username(name).unwrap_or(u32::MAX))
            .unwrap_or_else(|| unsafe { libc::geteuid() })
    });

    let mut machine = Machine::new(user_id);

    // Create settings and load from config file
    let mut settings = Settings::new();
    if let Err(e) = settings.load() {
        eprintln!("Warning: Failed to load settings: {}", e);
    }

    // Apply command line arguments
    if args.no_color {
        settings.color_scheme = core::ColorScheme::Monochrome;
    }
    if let Some(delay) = args.delay {
        settings.delay = delay.clamp(1, 100);
    }
    if args.no_mouse {
        settings.enable_mouse = false;
    }
    if args.tree {
        settings.tree_view = true;
    }
    if args.highlight_changes.is_some() {
        settings.highlight_changes = true;
        if let Some(Some(delay)) = args.highlight_changes {
            settings.highlight_delay_secs = delay.max(1) as i32;
        }
    }
    if let Some(ref key) = args.sort_key {
        if key == "help" {
            print_sort_keys();
            return Ok(());
        }
        settings.sort_key = core::ProcessField::from_name(key);
    }
    if args.no_function_bar {
        settings.hide_function_bar = 2;
    }
    if args.readonly {
        settings.readonly = true;
    }

    settings.allow_unicode = !args.no_unicode;
    machine.iterations_remaining = args.max_iterations.unwrap_or(-1);

    if let Some(ref pids) = args.pids {
        machine.set_pid_filter(pids.clone());
    }

    // Initialize CRT (terminal)
    let mut crt = Crt::new(&settings)?;

    // Create header with meters
    let mut header = Header::new(&machine, settings.header_layout, settings.header_margin);
    header.populate_from_settings(&settings);

    // Create main panel
    let mut main_panel = MainPanel::new();
    if let Some(filter) = args.filter {
        main_panel.set_filter(&filter);
    }

    // Create screen manager
    let mut screen_manager = ScreenManager::new(header, &mut machine, &settings);
    screen_manager.add_panel(main_panel);

    // Main loop (platform::scan is called inside run())
    screen_manager.run(&mut crt, &mut machine, &RUNNING)?;

    // Get the updated settings back from screen manager
    let settings = screen_manager.take_settings();

    // Cleanup
    crt.done();
    platform::done();

    // Save settings if changed
    if settings.changed {
        if let Err(e) = settings.write() {
            eprintln!("Warning: Failed to save settings: {}", e);
        }
    }

    Ok(())
}

fn print_sort_keys() {
    println!("Available sort keys:");
    for field in core::ProcessField::all() {
        let name = field.name();
        println!("  {:>19} {}", name, field.description());
    }
}

// Add ctrlc as a dependency - we'll handle it simply
mod ctrlc {
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    static HANDLER_SET: AtomicBool = AtomicBool::new(false);

    // We need to store a *const dyn Fn(), which is a fat pointer (2 words).
    // Instead, we box it twice so we have a thin pointer to store.
    type Handler = Box<dyn Fn() + Send>;
    static HANDLER_PTR: AtomicPtr<Handler> = AtomicPtr::new(std::ptr::null_mut());

    pub fn set_handler<F: Fn() + Send + 'static>(handler: F) -> Result<(), ()> {
        if HANDLER_SET.swap(true, Ordering::SeqCst) {
            return Err(());
        }

        unsafe {
            libc::signal(
                libc::SIGINT,
                handle_signal as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGTERM,
                handle_signal as *const () as libc::sighandler_t,
            );
        }

        // Box the handler, then box it again to get a thin pointer
        let handler: Handler = Box::new(handler);
        let handler_ptr = Box::into_raw(Box::new(handler));
        HANDLER_PTR.store(handler_ptr, Ordering::SeqCst);

        Ok(())
    }

    extern "C" fn handle_signal(_: libc::c_int) {
        let ptr = HANDLER_PTR.load(Ordering::SeqCst);
        if !ptr.is_null() {
            // SAFETY: ptr was set by set_handler and points to a valid Box<Handler>
            let handler = unsafe { &**ptr };
            handler();
        }
    }
}
