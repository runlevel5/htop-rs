# TODO

## Unimplemented Meters

The following meters have stub implementations that display "Not implemented". Total: **15 meters** need implementation.

### Cross-platform meters (1)

| Meter | File | Description |
|-------|------|-------------|
| GPU | `gpu_meter.rs` | GPU utilization (requires driver-specific code) |

### Linux-only meters (14)

| Meter | File | Description |
|-------|------|-------------|
| DiskIOTime | `diskio_time_meter.rs` | Disk percent time busy |
| HugePages | `hugepages_meter.rs` | HugePages memory usage |
| PressureStallCPUSome | `pressure_stall_meter.rs` | PSI CPU pressure (some) |
| PressureStallIOSome | `pressure_stall_meter.rs` | PSI I/O pressure (some) |
| PressureStallIOFull | `pressure_stall_meter.rs` | PSI I/O pressure (full) |
| PressureStallIRQFull | `pressure_stall_meter.rs` | PSI IRQ pressure |
| PressureStallMemorySome | `pressure_stall_meter.rs` | PSI memory pressure (some) |
| PressureStallMemoryFull | `pressure_stall_meter.rs` | PSI memory pressure (full) |
| Zram | `zram_meter.rs` | Zram compressed memory stats |
| SELinux | `selinux_meter.rs` | SELinux enforcement state |
| Systemd | `systemd_meter.rs` | Systemd system state |
| SystemdUser | `systemd_meter.rs` | Systemd user session state |
| ZFSARC | `zfs_arc_meter.rs` | ZFS ARC cache statistics |
| ZFSCARC | `zfs_arc_meter.rs` | ZFS Compressed ARC stats |

### Implementation notes

- **PSI meters** require `/proc/pressure/{cpu,io,memory}` (Linux 4.20+, `CONFIG_PSI`)
- **Systemd meters** require D-Bus or parsing systemctl output
- **ZFS meters** require `/proc/spl/kstat/zfs/arcstats` (ZFS on Linux)
- **GPU meter** is complex due to driver-specific interfaces (see GPU metrics section below)

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

### ~~Theme-agnostic component architecture~~ ✓ COMPLETED

Refactored theming so components don't need to check which theme is active. Components now use `Crt::bar_char()` and `Crt::theme_help_text()` instead of checking `ColorScheme` directly.

**Implementation (commit 721f179):**
- Added `Theme` struct in `src/ui/crt.rs` with `bar_chars` and `help_text` fields
- Theme is initialized from `ColorScheme` and stored in `Crt`
- Meters use `crt.bar_char(idx)` for theme-appropriate bar characters
- Help screen uses `crt.theme_help_text()` for optional theme-specific help

**Benefits achieved:**
- Components are theme-agnostic (just ask theme for colors AND glyphs)
- Adding new themes only requires updating `Theme::from_color_scheme()`
- Single source of truth for all theme-related behavior

### Delay accounting columns (Linux taskstats)
Implement `PercentCpuDelay`, `PercentIODelay`, `PercentSwapDelay` columns which show scheduling delay percentages.

**Requirements:**
- Linux netlink socket with `NETLINK_GENERIC` family
- `TASKSTATS` genetlink family for per-task delay accounting
- Requires `CAP_NET_ADMIN` capability or root privileges
- Kernel config: `CONFIG_TASKSTATS`, `CONFIG_TASK_DELAY_ACCT`

**Implementation approach:**
1. Create netlink socket and resolve TASKSTATS family ID
2. Send `TASKSTATS_CMD_GET` with `TASKSTATS_CMD_ATTR_PID` for each process
3. Parse response for `cpu_delay_total`, `blkio_delay_total`, `swapin_delay_total` (in nanoseconds)
4. Calculate percentage: `delay_total / (delay_total + run_time) * 100`

**Reference:**
- C htop: `linux/LinuxProcessTable.c` - `LinuxProcessTable_readDelayAcctData()`
- Linux kernel: `include/uapi/linux/taskstats.h`
- Documentation: `Documentation/accounting/delay-accounting.rst`

**Fields affected:**
- `cpu_delay_percent` - CPU scheduling delay %
- `blkio_delay_percent` - Block I/O delay %
- `swapin_delay_percent` - Swap-in delay %

### GPU metrics columns (Linux DRM)
Implement `GpuTime` and `GpuPercent` columns for GPU usage per process.

**Requirements:**
- Linux DRM (Direct Rendering Manager) subsystem
- `/sys/class/drm/card*/device/` for GPU enumeration
- `/proc/[pid]/fdinfo/` for per-process GPU usage (driver-specific)
- Supported drivers: i915 (Intel), amdgpu (AMD), nvidia (requires nvidia-smi or NVML)

**Implementation approach:**
1. Enumerate GPUs from `/sys/class/drm/`
2. For each process, scan `/proc/[pid]/fdinfo/*` for DRM file descriptors
3. Parse driver-specific fields:
   - i915/amdgpu: `drm-engine-*` fields show nanoseconds of GPU time
   - nvidia: Use NVML library or parse `nvidia-smi` output
4. Calculate GPU % based on time delta between scans

**Reference:**
- C htop: `linux/GPU.c`, `linux/LinuxProcess.c` GPU-related code
- Kernel docs: `Documentation/gpu/drm-usage-stats.rst`

**Fields affected:**
- `gpu_time` - Total GPU time in centiseconds
- `gpu_percent` - GPU usage percentage

**Notes:**
- This is complex due to driver-specific formats
- May want to make this optional/feature-gated
- Consider caching GPU enumeration (GPUs don't change at runtime)

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
