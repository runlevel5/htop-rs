## Project Overview

We are porting htop (the interactive process viewer written in C) to Rust using the ncursesw crate for terminal UI. The project is located at /Users/tle/Work/htop/htop-rs/.

### What Was Completed

1. Project Structure Created

```
htop-rs/
├── Cargo.toml
├── src/
│   ├── main.rs             # Entry point, CLI args
│   ├── core/
│   │   ├── mod.rs
│   │   ├── process.rs      # Process struct, ProcessState, ProcessField
│   │   ├── machine.rs      # Machine struct (system state)
│   │   └── settings.rs     # Settings, ColorScheme, HeaderLayout
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── crt.rs          # Terminal abstraction (ncurses)
│   │   ├── panel.rs        # Generic scrollable list widget
│   │   ├── rich_string.rs  # Attributed string
│   │   ├── function_bar.rs # F1-F10 key labels
│   │   ├── header.rs       # Meter display area
│   │   ├── main_panel.rs   # Main process list panel
│   │   └── screen_manager.rs # Main event loop (MODIFIED)
│   ├── meters/
│   │   ├── mod.rs          # Meter trait, draw_bar(), draw_text()
│   │   ├── cpu_meter.rs
│   │   ├── memory_meter.rs # MODIFIED - fixed double bracket
│   │   ├── swap_meter.rs   # MODIFIED - fixed double bracket
│   │   ├── load_meter.rs
│   │   ├── tasks_meter.rs
│   │   ├── uptime_meter.rs
│   │   └── blank_meter.rs
│   └── platform/
│       ├── mod.rs          # Platform abstraction
│       ├── darwin.rs       # macOS implementation
│       └── linux.rs        # Linux implementation
```

2. Key Components Implemented

- Core Module: Process representation with all fields (pid, user, cpu%, mem%, state, cmdline, etc.), ProcessField enum for sorting/display, Machine struct for system state, Settings with config file support
- UI Module: Full ncurses-based terminal UI with color schemes, panel system, function bar, header with meters, main process list panel with search/filter, screen manager with event loop
- Meters Module: Meter trait with CPU, Memory, Swap, Load Average, Tasks, Uptime, and Blank meter implementations supporting bar and text display modes

3. Dependencies in Cargo.toml

- ncurses = "6.0" - Terminal UI
- clap = "4.4" - Argument parsing
- anyhow, thiserror - Error handling
- chrono - Time handling
- users - User/group info
- procfs (Linux), libproc/mach2 (macOS) - Platform-specific process reading
- nix, libc - Unix APIs

### Key Architecture Decisions Made

1. Rust traits instead of C vtables: The C code uses manual OOP with vtables. We use Rust traits (e.g., Meter trait, PanelItem trait).
2. ncurses crate: Using the ncurses crate directly as requested (not crossterm or termion).
3. Platform abstraction via cfg: Using `#[cfg(target_os = "linux")]` and similar for platform-specific code.
4. Settings persistence: Config file format compatible with original htop's `~/.config/htop/htoprc`.

### Original htop Reference Files

Key C files that were analyzed for the port:
- htop.c, CommandLine.c - Entry point and main flow
- Process.h, Machine.h - Core data structures
- CRT.h, Panel.h, ScreenManager.h - UI components
- Meter.h, CPUMeter.c, MemoryMeter.c - Meter system
- linux/LinuxProcessTable.c - Linux process reading (uses /proc)

