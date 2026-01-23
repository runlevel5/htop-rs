# Improvements over C htop

This document outlines intentional improvements and enhancements made in htop-rs compared to the original C htop implementation.

## User Interface

### Dimmed F7 "Nice -" When Not Root

htop-rs visually indicates when the F7 "Nice -" action is unavailable by dimming/graying out the function bar item when not running as root.

- **Why**: Decreasing a process's nice value (increasing its priority) requires root privileges on Unix systems
- **Behavior**: F7 appears dimmed in the function bar when `geteuid() != 0`
- **C htop behavior**: Shows F7 normally regardless of privileges; users only discover it doesn't work after pressing the key and seeing the operation fail

This provides immediate visual feedback about available actions without requiring trial and error.

### Pause Indicator in Screen Tab Row

When updates are paused (Z key), htop-rs shows a pause icon before the screen tabs:

- **UTF-8 terminals**: `⏸ [Main] [I/O]` - uses the ⏸ (U+23F8) pause symbol
- **Non-UTF-8 terminals**: `[PAUSED] [Main] [I/O]` - text fallback

This makes the paused state visible at all times, regardless of which tab is active or where you're scrolling in the process list.

- **C htop behavior**: Only shows "PAUSED" text appended to the function bar at the bottom of the screen
- **htop-rs behavior**: Shows both the tab row indicator AND the function bar text (matching C htop's bottom indicator)

The tab row indicator is particularly useful because it remains visible regardless of whether the function bar is hidden (via `hide_function_bar` setting).

### Search and Filter Mode Visual Indicator

When using F3 Search or F4 Filter functions, htop-rs provides enhanced visual feedback:

- **Yellow header background**: The column header row turns yellow when search or filter is active, making it immediately clear that the view is in a special mode
- **Yellow "following" selection**: The selected row uses the yellow selection color to indicate an active search/filter match

In C htop, there is no visual indication in the header that a search or filter is active, which can make it unclear whether the process list is currently filtered or a search is in progress.

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
