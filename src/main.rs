//! htop-rs - A Rust port of htop, an interactive process viewer
//!
//! Copyright (C) 2004-2024 htop dev team
//! Released under the GNU GPLv2+

mod core;
mod meters;
mod platform;
mod ui;

use anyhow::Result;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::{Machine, Settings};
use crate::ui::{Crt, Header, MainPanel, ScreenManager};

/// Static flag for clean shutdown
static RUNNING: AtomicBool = AtomicBool::new(true);

/// htop - an interactive process viewer
#[derive(Parser, Debug)]
#[command(name = "htop-rs")]
#[command(author = "htop dev team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Interactive process viewer", long_about = None)]
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
    
    // Create settings
    let mut settings = Settings::new();
    
    // Apply command line arguments
    if args.no_color {
        settings.color_scheme = core::ColorScheme::Monochrome;
    }
    if let Some(delay) = args.delay {
        settings.delay = delay.max(1).min(100);
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
    screen_manager.add_panel(Box::new(main_panel));

    // Main loop (platform::scan is called inside run())
    screen_manager.run(&mut crt, &mut machine, &RUNNING)?;

    // Cleanup
    crt.done();
    platform::done();

    // Save settings if changed
    if settings.changed {
        settings.write()?;
    }

    Ok(())
}

fn print_sort_keys() {
    println!("Available sort keys:");
    for field in core::ProcessField::all() {
        if let Some(name) = field.name() {
            println!("  {:>19} {}", name, field.description());
        }
    }
}

// Add ctrlc as a dependency - we'll handle it simply
mod ctrlc {
    use std::sync::atomic::{AtomicBool, Ordering};
    static HANDLER_SET: AtomicBool = AtomicBool::new(false);
    
    pub fn set_handler<F: Fn() + Send + 'static>(handler: F) -> Result<(), ()> {
        if HANDLER_SET.swap(true, Ordering::SeqCst) {
            return Err(());
        }
        
        unsafe {
            libc::signal(libc::SIGINT, handle_signal as libc::sighandler_t);
            libc::signal(libc::SIGTERM, handle_signal as libc::sighandler_t);
        }
        
        // Store handler in a box leak (simple approach)
        let handler = Box::new(handler);
        let handler_ptr = Box::into_raw(handler);
        HANDLER_PTR.store(handler_ptr as *mut (), Ordering::SeqCst);
        
        Ok(())
    }
    
    static HANDLER_PTR: std::sync::atomic::AtomicPtr<()> = 
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
    
    extern "C" fn handle_signal(_: libc::c_int) {
        let ptr = HANDLER_PTR.load(Ordering::SeqCst);
        if !ptr.is_null() {
            let handler = unsafe { &*(ptr as *const Box<dyn Fn()>) };
            handler();
        }
    }
}
