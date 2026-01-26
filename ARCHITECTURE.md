# htop-rs Architecture Guide

## For C htop Developers

This document maps the htop-rs Rust codebase to the familiar C htop architecture. The goal is to help developers who know C htop quickly understand and contribute to htop-rs.

---

## Table of Contents

1. [Project Structure](#project-structure)
2. [Component Mapping](#component-mapping)
3. [Core Architecture](#core-architecture)
4. [UI Layer](#ui-layer)
5. [Platform Abstraction](#platform-abstraction)
6. [Key Patterns & Idioms](#key-patterns--idioms)
7. [Data Flow](#data-flow)
8. [Quick Reference](#quick-reference)

---

## Project Structure

### C htop Layout
```
htop/
├── *.c, *.h          # Core source files
├── linux/            # Linux platform
├── darwin/           # macOS platform
├── freebsd/          # FreeBSD platform
└── ...               # Other platforms
```

### htop-rs Layout
```
htop-rs/
├── src/
│   ├── main.rs           # Entry point (≈ htop.c + CommandLine.c)
│   ├── core/             # Data model (≈ Machine, Process, Table, Settings)
│   │   ├── mod.rs
│   │   ├── machine.rs    # ≈ Machine.c
│   │   ├── process.rs    # ≈ Process.c + Row.c + Table.c
│   │   └── settings.rs   # ≈ Settings.c
│   ├── ui/               # UI layer (≈ Panel, CRT, Header, Meter)
│   │   ├── mod.rs
│   │   ├── crt.rs        # ≈ CRT.c (uses ncurses-rs, a pure Rust ncurses implementation)
│   │   ├── panel.rs      # ≈ Panel.c
│   │   ├── main_panel.rs # ≈ MainPanel.c + IncSet.c
│   │   ├── screen_manager.rs  # ≈ ScreenManager.c + Action.c
│   │   ├── header.rs     # ≈ Header.c
│   │   ├── function_bar.rs    # ≈ FunctionBar.c
│   │   ├── rich_string.rs     # ≈ RichString.c
│   │   ├── row_print.rs  # Field formatting helpers
│   │   ├── info_screen.rs     # Generic popup info screen
│   │   ├── process_info_screens.rs  # Process info popups (env, lsof, strace, etc.)
│   │   ├── menus.rs      # Menu screens (help, kill, sort, user filter)
│   │   ├── side_panel_menu.rs # Side panel menu helper
│   │   └── setup/        # ≈ all *Panel.c for F2 Setup screen
│   │       ├── mod.rs    # SetupScreen struct and implementation
│   │       ├── types.rs  # SetupCategory, OptionItem, SettingField
│   │       └── meter_registry.rs  # MeterInfo and platform meter lists
│   ├── meters/           # Individual meter implementations
│   │   ├── mod.rs        # ≈ Meter.c (base trait + factory)
│   │   ├── cpu_meter.rs  # ≈ CPUMeter.c
│   │   ├── memory_meter.rs    # ≈ MemoryMeter.c
│   │   └── ...           # Other *Meter.c equivalents
│   └── platform/         # Platform-specific code
│       ├── mod.rs        # Platform abstraction layer
│       ├── linux.rs      # ≈ linux/Platform.c + linux/*Process*.c
│       └── darwin.rs     # ≈ darwin/Platform.c + darwin/*Process*.c
└── Cargo.toml
```

---

## Component Mapping

### Core Data Model

| C htop | htop-rs | Description |
|--------|---------|-------------|
| `Object.c/h` | Rust traits | OOP base class → traits like `Display` |
| `Vector.c/h` | `Vec<T>` | Dynamic array |
| `Hashtable.c/h` | `HashMap<K,V>` | Hash table |
| `Machine.c/h` | `src/core/machine.rs` | System state container |
| `Table.c/h` | `ProcessList` in `process.rs` | Row container with sorting/filtering |
| `Row.c/h` | Part of `Process` struct | Base displayable entry |
| `Process.c/h` | `src/core/process.rs` | Process data |
| `Settings.c/h` | `src/core/settings.rs` | Configuration |
| `UsersTable.c/h` | `users` crate | UID → username mapping |

### UI Components

| C htop | htop-rs | Description |
|--------|---------|-------------|
| `CRT.c/h` | `src/ui/crt.rs` | Terminal abstraction (via ncurses-rs, pure Rust) |
| `RichString.c/h` | `src/ui/rich_string.rs` | Attributed strings |
| `Panel.c/h` | `src/ui/panel.rs` | Generic scrollable list |
| `MainPanel.c/h` | `src/ui/main_panel.rs` | Process list panel |
| `IncSet.c/h` | `IncSearch` in `main_panel.rs` | Search/filter state |
| `ScreenManager.c/h` | `src/ui/screen_manager.rs` | Panel manager + event loop |
| `Header.c/h` | `src/ui/header.rs` | Meter header |
| `Meter.c/h` | `src/meters/` | Individual meter implementations |
| `FunctionBar.c/h` | `src/ui/function_bar.rs` | F-key bar |
| `Action.c/h` | `screen_manager.rs` + `menus.rs` | Key handlers + menu screens |
| `*Panel.c` (Setup) | `src/ui/setup/` | F2 Setup screen panels |
| `*Screen.c` (Info) | `src/ui/process_info_screens.rs` | Process info popups (env, lsof, strace) |
| `SignalsPanel.c` | `src/ui/menus.rs` | Kill signal selection menu |

### Platform Layer

| C htop | htop-rs | Description |
|--------|---------|-------------|
| `linux/Platform.c` | `src/platform/linux.rs` | Linux system calls |
| `linux/LinuxProcess.c` | Part of `linux.rs` | Linux process fields |
| `linux/LinuxProcessTable.c` | `scan()` in `linux.rs` | /proc parsing |
| `darwin/Platform.c` | `src/platform/darwin.rs` | macOS system calls |
| `darwin/DarwinProcess.c` | Part of `darwin.rs` | macOS process fields |

---

## Core Architecture

### Machine (System State)

**C htop** (`Machine.c`):
```c
struct Machine_ {
    Settings* settings;
    memory_t totalMem, usedMem, cachedMem, ...;
    memory_t totalSwap, usedSwap;
    unsigned int activeCPUs, existingCPUs;
    UsersTable* usersTable;
    Table* processTable;
    Table* activeTable;
};
```

**htop-rs** (`src/core/machine.rs`):
```rust
pub struct Machine {
    // Memory stats
    pub total_mem: u64,
    pub used_mem: u64,
    pub cached_mem: u64,
    pub buffers_mem: u64,
    pub available_mem: u64,

    // Swap stats
    pub total_swap: u64,
    pub used_swap: u64,

    // CPU info
    pub cpu_count: usize,
    pub cpus: Vec<CpuData>,

    // Process list (≈ Table + ProcessTable)
    pub processes: ProcessList,

    // Sorting
    pub sort_key: ProcessField,
    pub sort_descending: bool,
}
```

**Key difference**: htop-rs combines `Machine`, `Table`, and `ProcessTable` into a simpler structure. The `ProcessList` struct handles what C htop splits across `Table` and `ProcessTable`.

### Process

**C htop** (`Process.c` + `Row.c`):
```c
struct Row_ {
    Object super;
    int id, parent;
    bool tag, showChildren;
    int32_t indent;           // Tree indentation
};

struct Process_ {
    Row super;
    pid_t pid, ppid, pgrp;
    uid_t st_uid;
    char* cmdline;
    char* procComm, *procExe;
    float percent_cpu, percent_mem;
    long m_virt, m_resident;
    ProcessState state;
};
```

**htop-rs** (`src/core/process.rs`):
```rust
pub struct Process {
    // Identity (from Row)
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,

    // Command info
    pub comm: Option<String>,      // ≈ procComm
    pub exe: Option<String>,       // ≈ procExe
    pub cmdline: Option<String>,   // ≈ cmdline

    // Resource usage
    pub percent_cpu: f32,
    pub percent_mem: f32,
    pub m_virt: u64,
    pub m_resident: u64,

    // State
    pub state: ProcessState,

    // Tree view (from Row)
    pub indent: i32,               // Tree indentation bits
    pub show_children: bool,
    pub tagged: bool,

    // Flags
    pub is_kernel_thread: bool,
    pub is_userland_thread: bool,
}
```

### ProcessList (≈ Table)

**C htop** uses `Table` for generic row management and `ProcessTable` for process-specific scanning:

```c
struct Table_ {
    Vector* rows;           // All rows
    Vector* displayList;    // Flattened tree for display
    Hashtable* table;       // id → Row lookup
    bool needsSort;
    int following;          // Row being followed
};
```

**htop-rs** (`src/core/process.rs`):
```rust
pub struct ProcessList {
    pub processes: Vec<Process>,
    tree_order: Vec<usize>,      // ≈ displayList indices
    pid_to_index: HashMap<i32, usize>,
    pub tree_built: bool,
}

impl ProcessList {
    pub fn iter(&self) -> impl Iterator<Item = &Process>;
    pub fn iter_tree(&self) -> impl Iterator<Item = &Process>;
    pub fn build_tree(&mut self, sort_key: ProcessField, ascending: bool);
    pub fn sort(&mut self, key: ProcessField, descending: bool);
}
```

---

## UI Layer

### Panel System

**C htop** (`Panel.c`):
```c
struct Panel_ {
    Object super;
    int x, y, w, h;
    Vector* items;
    int selected, oldSelected;
    int scrollV, scrollH;
    bool needsRedraw;
    RichString header;
    FunctionBar* currentBar;
};

void Panel_draw(Panel* this, bool force_redraw, bool focus, ...);
bool Panel_onKey(Panel* this, int key);
```

**htop-rs** (`src/ui/panel.rs`):
```rust
pub struct Panel {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,

    items: Vec<Box<dyn PanelItem>>,

    pub selected: i32,
    pub old_selected: i32,
    pub scroll_v: i32,
    pub scroll_h: i32,

    pub needs_redraw: bool,
    pub header: RichString,
    pub function_bar: FunctionBar,
}

impl Panel {
    pub fn draw(&mut self, crt: &Crt, focus: bool, show_header: bool);
    pub fn on_key(&mut self, key: i32) -> HandlerResult;
}
```

### MainPanel (Process List)

**C htop** (`MainPanel.c` + `IncSet.c`):
```c
struct MainPanel_ {
    Panel super;
    State* state;
    IncSet* inc;              // Search/filter
    Htop_Action* keys;        // Key bindings
    pid_t idSearch;           // PID search accumulator
};
```

**htop-rs** (`src/ui/main_panel.rs`):
```rust
pub struct MainPanel {
    // Panel fields (composition, not inheritance)
    pub x: i32, pub y: i32, pub w: i32, pub h: i32,
    pub selected: i32,
    old_selected: i32,
    pub scroll_v: i32,

    // Process display
    pub fields: Vec<ProcessField>,
    pub tree_view: bool,

    // Search/filter (≈ IncSet)
    pub inc_search: IncSearch,
    pub filter: Option<String>,

    // Display cache (≈ Table.displayList)
    cached_display_indices: Vec<usize>,
    display_list_valid: bool,
}

pub struct IncSearch {
    pub active: bool,
    pub text: String,
    pub mode: Option<IncType>,  // Search or Filter
    pub found: bool,
}
```

### ScreenManager (Event Loop)

**C htop** (`ScreenManager.c`):
```c
struct ScreenManager_ {
    Vector* panels;
    Header* header;
    Machine* host;
    State* state;
};

void ScreenManager_run(ScreenManager* this, Panel** lastFocus, int* lastKey, const char* name);
```

The main loop in C htop:
```c
while (!quit) {
    checkRecalculation(...);     // Update data if timer elapsed
    if (redraw || force_redraw)
        ScreenManager_drawPanels(...);

    ch = Panel_getCh(panelFocus);

    // Handle input...
    result = Panel_eventHandler(panelFocus, ch);
    Panel_onKey(panelFocus, ch);
}
```

**htop-rs** (`src/ui/screen_manager.rs`):
```rust
pub struct ScreenManager {
    pub header: Header,
    main_panel: MainPanel,
    settings: Settings,

    // Optimization flags
    header_needs_redraw: bool,
    sort_timeout: u8,
}

impl ScreenManager {
    pub fn run(&mut self, crt: &mut Crt, machine: &mut Machine,
               running: &AtomicBool) -> Result<()> {
        loop {
            // Check update timer
            if should_update {
                platform::scan(machine);
                self.header.update(machine);
                self.main_panel.invalidate_display_list();
            }

            // Draw
            self.draw(crt, machine);

            // Handle input
            if let Some(key) = crt.read_key() {
                self.sort_timeout = SORT_TIMEOUT_RESET;
                match self.handle_key(key, crt, machine) {
                    HandlerResult::BreakLoop => break,
                    ...
                }
            } else {
                if self.sort_timeout > 0 {
                    self.sort_timeout -= 1;
                }
            }
        }
    }
}
```

### Meter System

**C htop** (`Meter.c`):
```c
typedef struct MeterClass_ {
    const Meter_UpdateValues updateValues;
    const Meter_Draw draw;
    const char* name;
    const uint8_t maxItems;
} MeterClass;

struct Meter_ {
    Meter_Draw draw;          // Current draw function
    MeterModeId mode;         // BAR, TEXT, GRAPH, LED
    double* values;
    double total;
};
```

**htop-rs** (`src/ui/meter.rs`):
```rust
pub enum MeterType {
    Cpu(usize),               // CPU meter for core N
    AllCpus,                  // All CPUs combined
    Memory,
    Swap,
    Tasks,
    LoadAverage,
    Uptime,
    Clock,
    Hostname,
    DiskIO,
    NetworkIO,
    Blank,
}

pub enum MeterMode {
    Bar,
    Text,
    Graph,
    Led,
}

pub struct Meter {
    pub meter_type: MeterType,
    pub mode: MeterMode,
    caption: String,
    values: Vec<f64>,
    total: f64,
}

impl Meter {
    pub fn update(&mut self, machine: &Machine);
    pub fn draw(&self, crt: &Crt, x: i32, y: i32, width: i32);
}
```

---

## Platform Abstraction

### C htop Pattern

Each platform implements `Platform.h`:
```c
// linux/Platform.c
void Platform_init(void);
void Platform_done(void);
void Platform_setMemoryValues(Meter* m);
void Platform_setSwapValues(Meter* m);
void Platform_setCPUValues(Meter* m, int cpu);
char* Platform_getProcessEnv(pid_t pid);
// ... etc
```

And extends Machine/Process:
```c
// linux/LinuxMachine.c
typedef struct LinuxMachine_ {
    Machine super;
    CPUData* cpuData;
    // Linux-specific fields...
} LinuxMachine;

// linux/LinuxProcess.c
typedef struct LinuxProcess_ {
    Process super;
    long m_share, m_pss;
    unsigned long long io_read_bytes;
    // Linux-specific fields...
} LinuxProcess;
```

### htop-rs Pattern

**Platform trait** (`src/platform/mod.rs`):
```rust
// Conditional compilation selects the right module
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "macos")]
pub use darwin::*;

// Common interface (implemented by each platform)
pub fn scan(machine: &mut Machine);
pub fn get_process_env(pid: i32) -> Vec<(String, String)>;
pub fn get_uptime() -> u64;
```

**Linux implementation** (`src/platform/linux.rs`):
```rust
pub fn scan(machine: &mut Machine) {
    scan_memory(machine);      // /proc/meminfo
    scan_cpus(machine);        // /proc/stat
    scan_processes(machine);   // /proc/[pid]/*
}

fn scan_processes(machine: &mut Machine) {
    for entry in fs::read_dir("/proc")? {
        if let Ok(pid) = entry.file_name().parse::<i32>() {
            scan_process(machine, pid);
        }
    }
}

fn scan_process(machine: &mut Machine, pid: i32) {
    // Read /proc/[pid]/stat, status, cmdline, etc.
}
```

**macOS implementation** (`src/platform/darwin.rs`):
```rust
pub fn scan(machine: &mut Machine) {
    scan_memory(machine);      // host_statistics64()
    scan_cpus(machine);        // host_processor_info()
    scan_processes(machine);   // sysctl + proc_pidinfo
}
```

---

## Terminal Layer (ncurses-rs)

htop-rs uses [ncurses-rs](https://github.com/runlevel5/ncurses-pure-rs), a pure Rust implementation of the ncurses API. This eliminates the dependency on system ncurses libraries.

**Key benefits:**
- No external C library dependencies
- Memory safety guarantees from Rust
- Easier cross-platform builds
- Direct terminal escape sequence handling

**API compatibility:** ncurses-rs provides a similar API to C ncurses, making the `crt.rs` code familiar to developers who know C htop's `CRT.c`.

### Theme System

The `Theme` struct in `crt.rs` encapsulates theme-specific rendering behavior, allowing components to be theme-agnostic.

**C htop approach:**
```c
// Components check color scheme directly
if (CRT_colorScheme == COLORSCHEME_MONOCHROME) {
    ch = BAR_CHARS[valueIndex];  // Different char per segment
} else {
    ch = '|';  // Same char, colors distinguish
}
```

**htop-rs approach:**
```rust
// Theme struct holds theme-specific behavior
pub struct Theme {
    bar_chars: [char; 8],           // Characters for bar meter segments
    help_text: Option<&'static str>, // Optional help screen text
}

// Crt owns the theme
pub struct Crt {
    pub theme: Theme,
    // ...
}

// Components ask Crt for theme-appropriate values
let bar_ch = crt.bar_char(segment_index);  // Theme-agnostic!
if let Some(text) = crt.theme_help_text() {
    // Display theme-specific help
}
```

**Benefits:**
- Components don't need to know which theme is active
- Adding new themes only requires updating `Theme::from_color_scheme()`
- Single source of truth for theme-specific rendering

---

## Key Patterns & Idioms

### OOP Translation

| C htop Pattern | Rust Equivalent |
|---------------|-----------------|
| `Object` base class | Trait (`trait PanelItem`) |
| Virtual methods via function pointers | Trait methods |
| Inheritance via struct embedding | Composition or enums |
| `Object_delete(obj)` | Automatic via `Drop` trait |
| `As_Panel(this)` cast macros | Pattern matching on enums |

### Example: Panel Items

**C htop**:
```c
// Object with virtual display method
typedef void (*Object_Display)(const Object*, RichString*);

struct ListItem_ {
    Object super;
    char* value;
    int key;
};

void ListItem_display(const Object* obj, RichString* str) {
    const ListItem* this = (const ListItem*)obj;
    RichString_writeWide(str, 0, this->value);
}
```

**htop-rs**:
```rust
pub trait PanelItem {
    fn display(&self, buffer: &mut RichString, highlighted: bool);
    fn sort_key(&self) -> &str;
}

pub struct ListItem {
    pub value: String,
    pub key: i32,
}

impl PanelItem for ListItem {
    fn display(&self, buffer: &mut RichString, _highlighted: bool) {
        buffer.append(&self.value, A_NORMAL);
    }

    fn sort_key(&self) -> &str {
        &self.value
    }
}
```

### needsRedraw Logic

The `needsRedraw` flag controls partial vs full redraws. This is critical for performance.

**C htop** (`Panel.c`):
```c
// UP/DOWN do NOT set needsRedraw
case KEY_DOWN:
    this->selected++;
    break;

// PAGEUP/DOWN DO set needsRedraw (via PANEL_SCROLL macro)
case KEY_PPAGE:
    PANEL_SCROLL(-(this->h - Panel_headerHeight(this)));
    break;

#define PANEL_SCROLL(amount) do {              \
    this->selected += (amount);                \
    this->scrollV = CLAMP(...);                \
    this->needsRedraw = true;                  \
} while (0)

// In Panel_draw():
if (this->needsRedraw || force_redraw) {
    // Full redraw - all visible rows
} else {
    // Partial redraw - only old and new selected rows
}
```

**htop-rs** (`main_panel.rs`):
```rust
// UP/DOWN use move_selection() - does NOT set needs_redraw
fn move_selection(&mut self, delta: i32, machine: &Machine) {
    self.selected = (self.selected + delta).clamp(0, count - 1);
    self.ensure_visible(count);  // May set needs_redraw if scroll changes
}

// PAGEUP/DOWN use scroll_wheel() - DOES set needs_redraw
pub fn scroll_wheel(&mut self, amount: i32, machine: &Machine) {
    self.selected += amount;
    self.scroll_v = (self.scroll_v + amount).clamp(0, max_scroll);
    self.needs_redraw = true;  // Always full redraw for scroll
}

// In draw():
if self.needs_redraw {
    // Full redraw - all visible rows
} else {
    // Partial redraw - only old and new selected rows
}
```

### sort_timeout (Deferred Sorting)

Both implementations defer sorting during rapid user interaction:

**C htop** (`ScreenManager.c`):
```c
int sortTimeout = 0;
int resetSortTimeout = 5;

// In main loop:
if (rescan) {
    if (sortTimeout == 0 || treeView) {
        host->activeTable->needsSort = true;
        sortTimeout = 1;
    }
}

// On key press:
sortTimeout = resetSortTimeout;

// On idle (ERR):
if (sortTimeout > 0)
    sortTimeout--;
```

**htop-rs** (`screen_manager.rs`):
```rust
const SORT_TIMEOUT_RESET: u8 = 5;

// In run():
if should_update {
    if self.sort_timeout == 0 || self.settings.tree_view {
        machine.needs_sort = true;
    }
}

// On key press:
self.sort_timeout = SORT_TIMEOUT_RESET;

// On idle (None):
if self.sort_timeout > 0 {
    self.sort_timeout -= 1;
}
```

---

## Data Flow

### Startup Sequence

```
main.rs
  │
  ├─► Parse command line args (clap)
  │
  ├─► Load Settings from ~/.config/htop/htoprc
  │
  ├─► Initialize CRT (pure Rust terminal via ncurses-rs)
  │
  ├─► Create Machine (system state)
  │
  ├─► Create Header with meters
  │
  ├─► Create ScreenManager
  │
  └─► ScreenManager::run() ─── main event loop
```

### Main Loop (ScreenManager::run)

```
┌─────────────────────────────────────────────────────────────┐
│                    ScreenManager::run()                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Check timer     │
                    │ elapsed?        │
                    └────────┬────────┘
                             │ Yes
                              ▼
              ┌─────────────────────────────────┐
              │ platform::scan(machine)         │
              │  ├─ scan_memory()               │
              │  ├─ scan_cpus()                 │
              │  └─ scan_processes()            │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │ header.update(machine)          │
              │ main_panel.invalidate_display() │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │ draw(crt, machine)              │
              │  ├─ header.draw()               │
              │  ├─ main_panel.draw()           │
              │  └─ function_bar.draw()         │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │ crt.read_key()                  │
              └────────┬──────────────┬─────────┘
                       │              │
                   Key pressed     No key (timeout)
                       │              │
                       ▼              ▼
              ┌────────────────┐ ┌──────────────┐
              │ handle_key()   │ │ sort_timeout │
              │ sort_timeout=5 │ │ -= 1         │
              └────────────────┘ └──────────────┘
                       │
                       ▼
              ┌─────────────────────────────────┐
              │ Match key:                      │
              │  F1 → show_help()               │
              │  F2 → show_setup()              │
              │  F5/t → toggle_tree_view()      │
              │  F9/k → show_kill_menu()        │
              │  ↑/↓ → main_panel.on_key()      │
              │  q → break loop                 │
              └─────────────────────────────────┘
```

---

## Quick Reference

### File Mapping Cheat Sheet

| To understand... | Read C htop... | Read htop-rs... |
|-----------------|----------------|-----------------|
| Entry point | `htop.c`, `CommandLine.c` | `main.rs` |
| System state | `Machine.c` | `core/machine.rs` |
| Process data | `Process.c`, `Row.c` | `core/process.rs` |
| Process scanning | `linux/LinuxProcessTable.c` | `platform/linux.rs` |
| Terminal/colors | `CRT.c` | `ui/crt.rs` |
| Panel widget | `Panel.c` | `ui/panel.rs` |
| Process list UI | `MainPanel.c` | `ui/main_panel.rs` |
| Main event loop | `ScreenManager.c` | `ui/screen_manager.rs` |
| Header/meters | `Header.c`, `Meter.c` | `ui/header.rs`, `meters/` |
| Settings | `Settings.c` | `core/settings.rs` |
| Search/filter | `IncSet.c` | `IncSearch` in `main_panel.rs` |
| F2 Setup screen | `*Panel.c` (Setup panels) | `ui/setup/` |
| Help screen | `Action.c` (actionHelp) | `ui/menus.rs` |
| Kill menu | `SignalsPanel.c` | `ui/menus.rs` |
| Sort menu | `Action.c` (actionSetSortColumn) | `ui/menus.rs` |
| User filter menu | `Action.c` (actionFilterByUser) | `ui/menus.rs` |
| Process info popups | `*Screen.c` | `ui/process_info_screens.rs` |

### Key Functions Mapping

| C htop Function | htop-rs Equivalent |
|----------------|-------------------|
| `ScreenManager_run()` | `ScreenManager::run()` |
| `Panel_draw()` | `Panel::draw()` / `MainPanel::draw()` |
| `Panel_onKey()` | `Panel::on_key()` / `MainPanel::on_key()` |
| `Table_rebuildPanel()` | `MainPanel::rebuild_display_list()` |
| `Machine_scan()` | `platform::scan()` |
| `Header_draw()` | `Header::draw()` |
| `Meter_updateValues()` | `Meter::update()` |
| `Process_display()` | `MainPanel::draw_process()` |
| `IncSet_handleKey()` | `MainPanel::handle_search_key()` |

### Key Structs Mapping

| C htop Struct | htop-rs Struct |
|--------------|----------------|
| `Machine` | `Machine` |
| `Table` | `ProcessList` |
| `Process` + `Row` | `Process` |
| `Panel` | `Panel` |
| `MainPanel` | `MainPanel` |
| `ScreenManager` | `ScreenManager` |
| `Header` | `Header` |
| `Meter` | `Meter` |
| `Settings` | `Settings` |
| `IncSet` | `IncSearch` |
| `RichString` | `RichString` |
| `FunctionBar` | `FunctionBar` |

---

## Building and Running

```bash
# Build
cargo build --release

# Run
cargo run --release

# Run with options (same as C htop)
cargo run --release -- -d 10      # 1 second delay
cargo run --release -- -u root    # Filter by user
cargo run --release -- -p 1234    # Show specific PID
cargo run --release -- -t         # Tree view
```

---

## Contributing

When porting features from C htop:

1. **Find the C implementation** in the appropriate `.c` file
2. **Identify the equivalent Rust module** using this guide
3. **Follow existing patterns** in that module
4. **Match behavior exactly** - htop-rs aims for 1:1 parity with C htop

The goal is that developers familiar with C htop can read and understand htop-rs code with minimal friction.
