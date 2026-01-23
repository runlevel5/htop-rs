# TODO

## Future Enhancements

### Locale-aware date/time formatting
Use system locale for date/time format in DateTime, Date, and Clock meters instead of hardcoded ISO 8601 (`%Y-%m-%d %H:%M:%S`).

- Linux/macOS/FreeBSD: Use `nl_langinfo(D_FMT)` and `nl_langinfo(T_FMT)` via libc to get locale-preferred formats
  - `D_FMT` - date format string (e.g., `%m/%d/%y` for en_US, `%d/%m/%y` for en_GB)
  - `T_FMT` - time format string (e.g., `%H:%M:%S`)
  - `D_T_FMT` - combined date and time format
- This is POSIX standard, works on all supported platforms
- Respects `LC_TIME` environment variable
- Would match behavior of most Unix tools

Note: C htop also uses hardcoded ISO 8601 format, so this would be an enhancement beyond the original.

### FreeBSD platform support
Add FreeBSD as a supported platform. This would involve:

- Implementing `freebsd/` platform module (similar to `linux/` and `darwin/`)
- Process reading via `kvm_getprocs()` or `sysctl()` with `KERN_PROC`
- CPU stats from `kern.cp_time` sysctl
- Memory stats from `vm.stats.*` sysctls
- Swap info from `kvm_getswapinfo()`
- Network I/O from `net.link` sysctls
- Disk I/O from `devstat`
- ZFS support (already have ZFS meters, just need FreeBSD data source)

Reference: C htop's `freebsd/` directory for implementation details.

### Theme-agnostic component architecture
Refactor theming so components don't need to check which theme is active. Currently there are theme-specific conditionals scattered in components (e.g., `if color_scheme == Monochrome`).

**Current issues:**
- `bar_meter_char()` in `crt.rs` checks for Monochrome to decide bar characters
- Help screen in `screen_manager.rs` checks for Monochrome to show special help text
- Theme logic leaks into components that should only care about rendering

**Proposed solution:**
```rust
pub struct Theme {
    colors: [attr_t; ColorElement::Count],
    
    // Rendering hints - theme provides characters, not just colors
    bar_chars: [char; 8],  // Monochrome: |#*@$%&.  Others: all '|'
    
    // Theme metadata
    name: &'static str,
    help_text: Option<&'static str>,  // e.g., "In monochrome, meters display..."
}
```

**Benefits:**
- Components become theme-agnostic (just ask theme for colors AND glyphs)
- Easier to add new themes without touching component code
- Single source of truth for all theme-related behavior
- Cleaner separation of concerns

**Files affected:**
- `src/ui/crt.rs` - Add `Theme` struct, move `bar_meter_char()` logic into theme
- `src/meters/*.rs` - Use theme for bar characters
- `src/ui/screen_manager.rs` - Use theme's help_text instead of checking scheme

### Context-sensitive meter help (F1 in Meters setup)
The F1 Help screen doesn't cover details of individual meters. Add a Help command in the Meters setup panel that shows detailed information about the currently selected meter.

**Current state:**
- Meters setup has function bars: `meters_bar`, `meters_moving_bar`, `meters_available_bar`
- F1 slot is currently unused in these function bars
- Meters have `name()` and `caption()` but no `description()` method
- C htop has an optional `description` field in `MeterClass` (e.g., "Network bytes & packets received/sent per second")

**Implementation:**
1. Add `fn description(&self) -> Option<&'static str>` to `Meter` trait in `src/meters/mod.rs`
2. Add descriptions to each meter (can reference C htop's descriptions where available)
3. Add `("Help", "F1")` to `meters_bar`, `meters_moving_bar`, and `meters_available_bar`
4. Create a meter help popup/overlay that displays:
   - Meter name and description
   - Supported display modes (Bar, Text, Graph, LED)
   - Color legend with actual theme colors (theme-aware!)
   - Platform availability (Linux only, macOS only, etc.)
5. Handle F1 key in meters panel input handling

**Example help content for CPU meter:**
```
CPU Meter
─────────
Shows CPU usage breakdown by category.

Display modes: Bar, Text, Graph, LED

Colors:
  ████ Normal (user)
  ████ System (kernel)
  ████ Nice (low priority)
  ████ IRQ (hardware interrupts)
  ████ SoftIRQ (software interrupts)
  ████ Steal (VM hypervisor)
  ████ Guest (VM guest)
  ████ I/O Wait

Platform: All
```

**Files affected:**
- `src/meters/mod.rs` - Add `description()` to trait
- `src/meters/*.rs` - Add descriptions to each meter
- `src/ui/setup_screen.rs` - Add F1 to function bars, handle key, render help popup
