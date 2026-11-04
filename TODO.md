# TODO

Open work, in three parts: features that are not implemented yet, engineering
debt found by review, and notes on how to verify changes. Finished work is
described in [IMPROVEMENTS.md](IMPROVEMENTS.md) and
[ARCHITECTURE.md](ARCHITECTURE.md), not here.

## Unimplemented Meters

The following meters have stub implementations that display "Not implemented".
Total: **15 meters** in 8 files.

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
- **Colours**: every colour comes from a semantic role in `src/ui/palette.rs`.
  The PSI, ZFS, Zram and HugePages meters need series roles added there (one
  per segment, like `MemoryColors`), not per-scheme colour assignments. The
  old `PressureStall*`, `Zfs*`, `Zram*` and `HugePage*` colour elements were
  removed because nothing used them.
- Slow reads belong in the background scanners (`platform/bg_scanner.rs`,
  `meters/meter_bg_scanner.rs`), which run on the rayon pool. Nothing that
  touches the filesystem should run in a meter's `update()`.

## Unimplemented Columns

### Delay accounting columns (Linux taskstats)
Implement `PercentCpuDelay`, `PercentIODelay`, `PercentSwapDelay` columns which show scheduling delay percentages. The `cpu_delay_percent` field exists on `Process` but nothing fills it.

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

**Notes:**
- This is complex due to driver-specific formats
- May want to make this optional/feature-gated
- Consider caching GPU enumeration (GPUs don't change at runtime)
- The per-process reads belong in the Linux background scanner

## Feature Enhancements

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
- ZFS support (needs the ZFS meters above, plus a FreeBSD data source)
- A `scan_tables` implementation; the table scanner thread and the rest of
  the UI are platform-independent

Reference: C htop's `freebsd/` directory for implementation details.

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
   - Color legend using the palette's series roles, so it is right in every scheme
   - Platform availability (Linux only, macOS only, etc.)
5. Handle F1 key in meters panel input handling

**Files affected:**
- `src/meters/mod.rs` - Add `description()` to trait
- `src/meters/*.rs` - Add descriptions to each meter
- `src/ui/setup/mod.rs` - Add F1 to function bars, handle key, render help popup

## Engineering Backlog

Found by a review in November 2026. None of these change behaviour on their
own; they make the code safer or easier to read.

### Correctness

- `Process::compare_by_field` falls back to comparing PIDs for every field it
  does not list. The I/O columns were fixed; the remaining Linux-only columns
  (cgroup, container, OOM score, context switches, autogroup, library size,
  PSS and swap sizes, and others) still sort by PID when selected. Add real
  comparisons, or make the fallback impossible by matching exhaustively.

### Safety

- `scan_cpu` in `src/platform/darwin.rs` reads `cpu_info` by `cpu_count * 4`
  without checking it against `processor_info_cnt`, the length the kernel
  returned. Wrap the buffer in `slice::from_raw_parts` with that length and
  iterate `chunks_exact(4)`.
- About eight FFI structs in `darwin.rs` are built with `mem::zeroed()`.
  Sound today because they are plain data; a `Default` derive or a comment on
  each saying why zero is valid stops the next field addition breaking that.
- Two real `unwrap`s outside tests: `make_command_str` unwraps `exe` and
  `comm` in its "full merged command" branch, and `start_pid_search` in
  `main_panel.rs` unwraps `pid_search` right after an `if let` on it.
- Narrowing `as` casts on kernel-returned sizes and counts in `darwin.rs`;
  use `try_from` with a fallback at the FFI boundary.
- `#[allow(dead_code)]` on five helpers in `linux.rs` and
  `#[allow(unused_must_use)]` on `show_help` hide real findings.

### Readability

- Very long functions: `write_field` (over 900 lines), `show_strace`, the CPU
  meter `draw`, `make_command_str` and `handle_key` (400 to 600 lines each).
  `write_field` splits naturally into one function per field group.
- `draw_process` and `write_field` take 12 to 18 parameters, and ten meter
  functions carry `allow(too_many_arguments)`. A context struct built once per
  draw, holding the settings flags and colours, would cut them to three or
  four.
- Two `get_username` methods, on `UsersTable` (returns `&str`) and on
  `Machine` (returns an owned `String`); the second exists only to dodge a
  borrow.
- 45 `RefCell`s in meters exist only because `Meter::draw` takes `&self`
  while graph meters need to push history. Let `draw` take `&mut self`, or
  move the history update into `update`.
- Comments that cite C htop line numbers go stale with every upstream
  release; describe the behaviour instead.
- Pedantic clippy flags hundreds of mechanical cleanups (`Self` repetitions,
  inlinable format arguments, long literals without separators, redundant
  closures); one `cargo clippy --fix` pass with those lints enabled is cheap.

## Verifying Changes

- `HTOP_DEBUG_TIMING=1` prints per-phase tick timings on the UI thread, the
  scan time on the worker, and the full redraw time. `HTOP_DEBUG_COLORS=1`
  prints the colour count the terminal was detected with; check it first when
  a colour looks wrong on one machine.
- Drive the app under a pseudo-terminal rather than judging by eye: spawn it
  with Python's `pty`, send keys (`\x1b[B` for Down, `\x1b[12~` for F2,
  `\t` for the next screen tab), and either timestamp every output read to
  measure latency, or replay the captured bytes into a character grid,
  tracking SGR codes per cell, to check what each cell ended up showing. The
  raw capture accumulates every draw, so only a replay tells you the final
  state.
- Compare against C htop on the same machine where it is installed; the
  ppc64le box has it, macOS does not.
- Check sorting by configuring each key in `htoprc`, letting a few ticks
  pass, replaying the final screen and asserting the visible rows are ordered.
