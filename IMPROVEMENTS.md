# Improvements over C htop

This document outlines intentional improvements and enhancements made in htop-rs compared to the original C htop implementation.

## User Interface

### Dimmed F7 "Nice -" When Unavailable

htop-rs visually indicates when the F7 "Nice -" action is unavailable by dimming/graying out the function bar item.

- **Why**: Decreasing a process's nice value (increasing its priority) typically requires elevated privileges
- **Behavior**:
  - **Root users**: F7 is always enabled
  - **macOS non-root**: F7 is always dimmed (macOS does not allow non-root users to decrease nice)
  - **Linux non-root**: F7 is enabled/dimmed based on `RLIMIT_NICE` resource limit:
    - `RLIMIT_NICE = 0` (default): F7 dimmed (cannot decrease nice)
    - `RLIMIT_NICE > 0`: F7 enabled (can decrease nice within the configured limit)
- **C htop behavior**: Shows F7 normally regardless of privileges; users only discover it doesn't work after pressing the key and seeing the operation fail

This provides immediate visual feedback about available actions without requiring trial and error.

### Pause Indicator in Screen Tab Row

When updates are paused (Z key), htop-rs shows a pause icon before the screen tabs:

- **UTF-8 terminals**: ` ⏸ ` followed by `[Main] [I/O]` - uses the ⏸ (U+23F8) pause symbol
- **Non-UTF-8 terminals**: ` PAUSED ` followed by `[Main] [I/O]` - text fallback
- **Color**: Uses the same color as the active tab for visual cohesion

This makes the paused state visible at all times, regardless of which tab is active or where you're scrolling in the process list.

- **C htop behavior**: Only shows "PAUSED" text appended to the function bar at the bottom of the screen
- **htop-rs behavior**: Shows both the tab row indicator AND the function bar text (matching C htop's bottom indicator)

The tab row indicator is particularly useful because it remains visible regardless of whether the function bar is hidden (via `hide_function_bar` setting).

### Search and Filter Mode Visual Indicator

When using F3 Search or F4 Filter functions, htop-rs provides enhanced visual feedback:

- **Yellow header background**: The column header row turns yellow when search or filter is active, making it immediately clear that the view is in a special mode
- **Yellow "following" selection**: The selected row uses the yellow selection color to indicate an active search/filter match

In C htop, there is no visual indication in the header that a search or filter is active, which can make it unclear whether the process list is currently filtered or a search is in progress.

### StackedGraph Meter Mode

htop-rs introduces a new meter display mode called **StackedGraph** that provides a multi-colored stacked graph visualization for meters with multiple components.

- **Mode number**: 5 (cycle through modes in Setup → Meters to reach it)
- **Supported meters**: Memory, Swap, CPU meters (AllCPUs, LeftCPUs, RightCPUs, Average, CPU(n))
- **Display**: Shows historical data as a stacked area graph where each component is rendered in its own color, stacked on top of each other

**Memory Meter StackedGraph segments (5 segments):**

| Segment | Color | Description |
|---------|-------|-------------|
| Used | MemoryUsed (green) | Memory actively used by applications |
| Shared | MemoryShared (magenta) | Shared memory (Linux only) |
| Compressed | MemoryCompressed (cyan) | Compressed memory (macOS only) |
| Buffers | MemoryBuffers (blue) | Kernel buffer cache |
| Cache | MemoryCache (yellow) | Page cache |

**Swap Meter StackedGraph segments (2 segments):**

| Segment | Color | Description |
|---------|-------|-------------|
| Used | Swap (red) | Swap space actively in use |
| Cache | SwapCache (yellow) | Swap cache (Linux only) |

**CPU Meter StackedGraph segments (8 segments):**

| Segment | Color | Description |
|---------|-------|-------------|
| User | CpuNormal (green) | User-space CPU time |
| Nice | CpuNice (cyan) | Low-priority user-space CPU time |
| System | CpuSystem (red) | Kernel CPU time |
| IRQ | CpuIrq (yellow) | Hardware interrupt handling |
| SoftIRQ | CpuSoftIrq (magenta) | Software interrupt handling |
| Steal | CpuSteal (cyan) | Time stolen by hypervisor (VMs) |
| Guest | CpuGuest (cyan) | Time running guest VMs |
| IOWait | CpuIOWait (dark blue) | Waiting for I/O completion |

**Platform behavior:**
- **Memory on Linux**: Shows used, shared, buffers, cache (no compressed segment)
- **Memory on macOS**: Shows used, compressed, buffers, cache (no shared segment)
- **Swap on Linux**: Shows used and cache segments
- **Swap on macOS**: Shows used only (no swap cache on macOS)
- **CPU**: All 8 segments shown on all platforms (steal/guest may be 0 on non-virtualized systems)

This provides a richer visualization than the standard Graph mode, which only shows a single aggregated value. The StackedGraph mode makes it easy to see how resource usage breaks down over time.

- **C htop behavior**: Does not have a StackedGraph mode; Graph mode shows only a single-color historical graph
- **htop-rs behavior**: Offers both Graph (single value) and StackedGraph (multi-component) modes

## Platform-Specific Improvements

### macOS: Accurate Minor/Major Page Fault Counts

htop-rs provides more accurate page fault statistics on macOS by properly separating minor and major faults:

| Field | htop-rs | C htop |
|-------|---------|--------|
| `MINFLT` | `pti_faults - pti_pageins` | Always 0 |
| `MAJFLT` | `pti_pageins` | `pti_faults` (total) |

**Explanation:**
- **Minor faults (MINFLT)**: Page faults satisfied from memory (page was resident but not mapped)
- **Major faults (MAJFLT)**: Page faults requiring disk I/O (page had to be read from disk)

macOS provides both `pti_faults` (total page faults) and `pti_pageins` (actual page-ins from disk) via `proc_pidinfo`. C htop only uses `pti_faults` for `majflt` and leaves `minflt` as 0, which conflates minor and major faults.

htop-rs correctly calculates:
- `majflt = pti_pageins` (true major faults)
- `minflt = pti_faults - pti_pageins` (true minor faults)

### Setup Screen: Enhanced Mouse Support for Meters Category

htop-rs provides comprehensive mouse support for the Meters category in the Setup screen (F2), making it easier to configure header meters.

**Left Column / Right Column Panels:**

| Action | Effect |
|--------|--------|
| Left-click on item | Select the meter |
| Right-click anywhere | Toggle move mode for selected meter |
| Middle-click anywhere | Cycle meter display style (Bar → Text → Graph → LED → StackedGraph) |
| Scroll wheel | Navigate up/down through meters (normal mode) |
| Scroll wheel in move mode | Move the selected meter up/down |
| Left-click on item in move mode | Insert selected meter above clicked item |
| Left-click on empty space in move mode | Append meter to bottom of column |

**Available Meters Panel (rightmost):**

| Action | Effect |
|--------|--------|
| Left-click on item | Select the meter |
| Right-click anywhere | Add selected meter to focused column (same as F5) |
| Scroll wheel | Navigate up/down through available meters |

**Cross-Column Move Mode:**
- Right-click in either column panel enters/exits move mode
- When in move mode, clicking on an item in the *other* column moves the meter there
- The meter is inserted above the clicked item, or appended if clicking empty space

**Additional behaviors:**
- Clicking on Available Meters panel exits move mode
- Selection follows the meter when moving with scroll wheel
- Move mode indicator shows which meter is being moved

- **C htop behavior**: Limited mouse support; no move mode, style cycling, or intuitive drag-and-drop-like behavior
- **htop-rs behavior**: Full mouse support with middle-click style cycling and cross-column move operations

### Setup Screen: Enhanced Mouse Support for Screens Category

htop-rs provides comprehensive mouse support for the Screens category in the Setup screen (F2), making it easier to manage screen tabs and their columns.

**Screens List Panel (leftmost):**

| Action | Effect |
|--------|--------|
| Left-click on item | Select the screen |
| Right-click anywhere | Toggle move mode for selected screen |
| Middle-click on item | Toggle rename mode for selected screen |
| Scroll wheel | Navigate up/down through screens |
| Scroll wheel in move mode | Move the selected screen up/down |
| Left-click on item in move mode | Insert selected screen above clicked item |
| Left-click on empty space in move mode | Append screen to bottom of list |

**Active Columns Panel (middle):**

| Action | Effect |
|--------|--------|
| Left-click on item | Select the column |
| Right-click anywhere | Toggle move mode for selected column |
| Scroll wheel | Navigate up/down through columns |
| Scroll wheel in move mode | Move the selected column up/down |
| Left-click on item in move mode | Insert selected column above clicked item |
| Left-click on empty space in move mode | Append column to bottom of list |

**Available Columns Panel (rightmost):**

| Action | Effect |
|--------|--------|
| Left-click on item | Select the column |
| Right-click anywhere | Add selected column to Active Columns (same as F5) |
| Scroll wheel | Navigate up/down through available columns |

**Renaming Mode:**

| Action | Effect |
|--------|--------|
| Middle-click on screen | Enter rename mode |
| Middle-click while renaming | Exit rename mode (saves changes) |
| Click outside Screens panel | Exit rename mode (saves changes) |
| Enter key | Confirm rename |
| Escape key | Cancel rename |

**Additional behaviors:**
- Clicking on a different panel automatically exits move mode
- Clicking on a different panel while renaming saves and exits rename mode
- Scroll wheel is disabled during rename mode to prevent accidental navigation
- Function bar shows only "Enter: Done" during rename mode
- Selection follows the item when moving with scroll wheel

- **C htop behavior**: Limited mouse support in Setup screen; no move mode, rename mode, or scroll wheel navigation for Screens category
- **htop-rs behavior**: Full mouse support with intuitive gestures for all common operations

### Setup Screen: Mouse Support for Display Options

htop-rs adds mouse support for Number options (like Update Interval) in the Display Options category:

| Action | Effect |
|--------|--------|
| Left-click on Number option | Increase value (same as F8) |
| Right-click on Number option | Decrease value (same as F7) |

Values wrap around at min/max boundaries, matching the keyboard behavior.

- **C htop behavior**: No mouse support for changing Number values; must use F7/F8 keys
- **htop-rs behavior**: Left-click to increase, right-click to decrease

### Side Panel Menus: Mouse Support

htop-rs adds mouse support for side panel menus (Kill/Signal menu F9, Sort menu F6, User filter menu u):

| Action | Effect |
|--------|--------|
| Left-click on item | Select the item |
| Right-click on item | Select and confirm (same as Enter) |
| Scroll wheel up/down | Navigate through the menu |
| Left-click on function bar | Activate the corresponding function key |

- **C htop behavior**: No mouse support in side panel menus; must use keyboard navigation
- **htop-rs behavior**: Full mouse support including quick selection with right-click

