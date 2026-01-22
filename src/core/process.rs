//! Process representation
//!
//! This module contains the Process struct and related types that represent
//! a single process in the system.

use std::cmp::Ordering;
use std::time::SystemTime;

/// Process state enum - core states shared by all platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessState {
    Unknown,
    Runnable,
    Running,
    Queued,
    Waiting,
    UninterruptibleWait,
    Blocked,
    Paging,
    Stopped,
    Traced,
    Zombie,
    Defunct,
    Idle,
    Sleeping,
}

impl ProcessState {
    /// Convert process state to a single character representation
    pub fn to_char(self) -> char {
        match self {
            ProcessState::Unknown => '?',
            ProcessState::Runnable => 'R',
            ProcessState::Running => 'R',
            ProcessState::Queued => 'Q',
            ProcessState::Waiting => 'S',
            ProcessState::UninterruptibleWait => 'D',
            ProcessState::Blocked => 'D',
            ProcessState::Paging => 'W',
            ProcessState::Stopped => 'T',
            ProcessState::Traced => 't',
            ProcessState::Zombie => 'Z',
            ProcessState::Defunct => 'X',
            ProcessState::Idle => 'I',
            ProcessState::Sleeping => 'S',
        }
    }

    /// Get a description of the process state
    pub fn description(self) -> &'static str {
        match self {
            ProcessState::Unknown => "Unknown",
            ProcessState::Runnable => "Runnable",
            ProcessState::Running => "Running",
            ProcessState::Queued => "Queued",
            ProcessState::Waiting => "Waiting",
            ProcessState::UninterruptibleWait => "Uninterruptible Wait",
            ProcessState::Blocked => "Blocked",
            ProcessState::Paging => "Paging",
            ProcessState::Stopped => "Stopped",
            ProcessState::Traced => "Traced",
            ProcessState::Zombie => "Zombie",
            ProcessState::Defunct => "Defunct",
            ProcessState::Idle => "Idle",
            ProcessState::Sleeping => "Sleeping",
        }
    }
}

impl Default for ProcessState {
    fn default() -> Self {
        ProcessState::Unknown
    }
}

/// Tristate for optional boolean values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tristate {
    #[default]
    Initial,
    Off,
    On,
}

/// Process fields for display and sorting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ProcessField {
    Pid = 1,
    Command,
    State,
    Ppid,
    Pgrp,
    Session,
    TtyNr,
    Tpgid,
    Minflt,
    Majflt,
    Priority,
    Nice,
    Starttime,
    Processor,
    MSize,
    MResident,
    MShare,
    MText,
    MLib,
    MData,
    MDirty,
    StUid,
    User,
    Time,
    Nlwp,
    Tty,
    CmdLine,
    Comm,
    Exe,
    Cwd,
    PercentCpu,
    PercentMem,
    IOPriority,
    IORead,
    IOWrite,
    IORate,
    IOReadRate,
    IOWriteRate,
    IOReadOps,
    IOWriteOps,
    PercentIODelay,
    PercentSwapDelay,
    Ctxt,
    CGroup,
    OomScore,
    SecAttr,
    Elapsed,
}

impl ProcessField {
    /// Get all process fields
    pub fn all() -> &'static [ProcessField] {
        &[
            ProcessField::Pid,
            ProcessField::Command,
            ProcessField::State,
            ProcessField::Ppid,
            ProcessField::Pgrp,
            ProcessField::Session,
            ProcessField::TtyNr,
            ProcessField::Tpgid,
            ProcessField::Minflt,
            ProcessField::Majflt,
            ProcessField::Priority,
            ProcessField::Nice,
            ProcessField::Starttime,
            ProcessField::Processor,
            ProcessField::MSize,
            ProcessField::MResident,
            ProcessField::MShare,
            ProcessField::MText,
            ProcessField::MLib,
            ProcessField::MData,
            ProcessField::MDirty,
            ProcessField::StUid,
            ProcessField::User,
            ProcessField::Time,
            ProcessField::Nlwp,
            ProcessField::Tty,
            ProcessField::CmdLine,
            ProcessField::Comm,
            ProcessField::Exe,
            ProcessField::Cwd,
            ProcessField::PercentCpu,
            ProcessField::PercentMem,
        ]
    }

    /// Get the name of a process field (matches C htop naming convention)
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            ProcessField::Pid => "PID",
            ProcessField::Command => "Command",
            ProcessField::State => "STATE",
            ProcessField::Ppid => "PPID",
            ProcessField::Pgrp => "PGRP",
            ProcessField::Session => "SESSION",
            ProcessField::TtyNr => "TTY_NR",
            ProcessField::Tpgid => "TPGID",
            ProcessField::Minflt => "MINFLT",
            ProcessField::Majflt => "MAJFLT",
            ProcessField::Priority => "PRIORITY",
            ProcessField::Nice => "NICE",
            ProcessField::Starttime => "STARTTIME",
            ProcessField::Processor => "PROCESSOR",
            ProcessField::MSize => "M_VIRT",
            ProcessField::MResident => "M_RESIDENT",
            ProcessField::MShare => "M_SHARE",
            ProcessField::MText => "M_TRS",
            ProcessField::MLib => "M_LRS",
            ProcessField::MData => "M_DRS",
            ProcessField::MDirty => "M_DT",
            ProcessField::StUid => "ST_UID",
            ProcessField::User => "USER",
            ProcessField::Time => "TIME",
            ProcessField::Nlwp => "NLWP",
            ProcessField::Tty => "TTY",
            ProcessField::CmdLine => "Command",
            ProcessField::Comm => "COMM",
            ProcessField::Exe => "EXE",
            ProcessField::Cwd => "CWD",
            ProcessField::PercentCpu => "PERCENT_CPU",
            ProcessField::PercentMem => "PERCENT_MEM",
            ProcessField::IOPriority => "IO_PRIORITY",
            ProcessField::IORead => "RBYTES",
            ProcessField::IOWrite => "WBYTES",
            ProcessField::IORate => "IO_RATE",
            ProcessField::IOReadRate => "IO_READ_RATE",
            ProcessField::IOWriteRate => "IO_WRITE_RATE",
            ProcessField::IOReadOps => "IO_OPS",
            ProcessField::IOWriteOps => "IO_OPS",
            ProcessField::PercentIODelay => "PERCENT_IO_DELAY",
            ProcessField::PercentSwapDelay => "PERCENT_SWAP_DELAY",
            ProcessField::Ctxt => "CTXT",
            ProcessField::CGroup => "CGROUP",
            ProcessField::OomScore => "OOM",
            ProcessField::SecAttr => "SECATTR",
            ProcessField::Elapsed => "ELAPSED",
        })
    }

    /// Get the title (column header) for a process field
    pub fn title(self) -> &'static str {
        match self {
            ProcessField::Pid => "  PID ",
            ProcessField::Command => "Command ",
            ProcessField::State => "S ",
            ProcessField::Ppid => " PPID ",
            ProcessField::Pgrp => " PGRP ",
            ProcessField::Session => "  SID ",
            ProcessField::TtyNr => "TTY_NR ",
            ProcessField::Tpgid => "TPGID ",
            ProcessField::Minflt => "     MINFLT ",
            ProcessField::Majflt => "     MAJFLT ",
            ProcessField::Priority => "PRI ",
            ProcessField::Nice => " NI ",
            ProcessField::Starttime => "START ",
            ProcessField::Processor => "CPU ",
            ProcessField::MSize => " VIRT ",
            ProcessField::MResident => "  RES ",
            ProcessField::MShare => "  SHR ",
            ProcessField::MText => " CODE ",
            ProcessField::MLib => "  LIB ",
            ProcessField::MData => " DATA ",
            ProcessField::MDirty => "DIRTY ",
            ProcessField::StUid => "  UID ",
            ProcessField::User => "USER       ",
            ProcessField::Time => "  TIME+  ",
            ProcessField::Nlwp => "NLWP ",
            ProcessField::Tty => "TTY      ",
            ProcessField::CmdLine => "Command",
            ProcessField::Comm => "COMM ",
            ProcessField::Exe => "EXE ",
            ProcessField::Cwd => "CWD ",
            ProcessField::PercentCpu => "CPU% ",
            ProcessField::PercentMem => "MEM% ",
            ProcessField::IOPriority => "IO ",
            ProcessField::IORead => "  IO_R ",
            ProcessField::IOWrite => "  IO_W ",
            ProcessField::IORate => "  DISK R/W ",
            ProcessField::IOReadRate => " DISK READ ",
            ProcessField::IOWriteRate => "DISK WRITE ",
            ProcessField::IOReadOps => " IO_ROP ",
            ProcessField::IOWriteOps => " IO_WOP ",
            ProcessField::PercentIODelay => " IOD% ",
            ProcessField::PercentSwapDelay => "SWPD% ",
            ProcessField::Ctxt => "    CTXT ",
            ProcessField::CGroup => "CGROUP ",
            ProcessField::OomScore => " OOM ",
            ProcessField::SecAttr => "Security Attribute ",
            ProcessField::Elapsed => "ELAPSED  ",
        }
    }

    /// Get the description for a process field
    pub fn description(self) -> &'static str {
        match self {
            ProcessField::Pid => "Process ID",
            ProcessField::Command => "Command (process name and arguments)",
            ProcessField::State => "Process state (S sleeping, R running, D disk, Z zombie)",
            ProcessField::Ppid => "Parent process ID",
            ProcessField::Pgrp => "Process group ID",
            ProcessField::Session => "Session ID",
            ProcessField::TtyNr => "Controlling terminal",
            ProcessField::Tpgid => "Foreground process group ID of the controlling terminal",
            ProcessField::Minflt => "Minor page faults (pages from memory)",
            ProcessField::Majflt => "Major page faults (pages from disk)",
            ProcessField::Priority => "Kernel scheduling priority",
            ProcessField::Nice => {
                "Nice value (the higher, the more it lets other processes take priority)"
            }
            ProcessField::Starttime => "Start time of the process",
            ProcessField::Processor => "CPU last executed on",
            ProcessField::MSize => "Virtual memory size",
            ProcessField::MResident => "Resident set size",
            ProcessField::MShare => "Shared memory size",
            ProcessField::MText => "Text (code) memory size",
            ProcessField::MLib => "Library memory size",
            ProcessField::MData => "Data + stack memory size",
            ProcessField::MDirty => "Dirty pages",
            ProcessField::StUid => "User ID of process owner",
            ProcessField::User => "Username of process owner",
            ProcessField::Time => "Total CPU time used",
            ProcessField::Nlwp => "Number of threads",
            ProcessField::Tty => "Controlling terminal name",
            ProcessField::CmdLine => "Full command line",
            ProcessField::Comm => "Process name (comm)",
            ProcessField::Exe => "Executable path",
            ProcessField::Cwd => "Current working directory",
            ProcessField::PercentCpu => "Percentage of CPU time",
            ProcessField::PercentMem => "Percentage of resident memory",
            ProcessField::IOPriority => "I/O priority",
            ProcessField::IORead => "Bytes of read(2) I/O for the process",
            ProcessField::IOWrite => "Bytes of write(2) I/O for the process",
            ProcessField::IORate => "Total I/O rate in bytes per second",
            ProcessField::IOReadRate => "The I/O rate of read(2) in bytes per second",
            ProcessField::IOWriteRate => "The I/O rate of write(2) in bytes per second",
            ProcessField::IOReadOps => "Read operations",
            ProcessField::IOWriteOps => "Write operations",
            ProcessField::PercentIODelay => "Block I/O delay %",
            ProcessField::PercentSwapDelay => "Swapin delay %",
            ProcessField::Ctxt => "Context switches",
            ProcessField::CGroup => "Control group",
            ProcessField::OomScore => "OOM killer score",
            ProcessField::SecAttr => "Security attribute (e.g., SELinux label)",
            ProcessField::Elapsed => "Time since process started",
        }
    }

    /// Create a process field from a name string
    pub fn from_name(name: &str) -> Option<ProcessField> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            "PID" => Some(ProcessField::Pid),
            "COMMAND" => Some(ProcessField::Command),
            "S" | "STATE" => Some(ProcessField::State),
            "PPID" => Some(ProcessField::Ppid),
            "PGRP" => Some(ProcessField::Pgrp),
            "SID" | "SESSION" => Some(ProcessField::Session),
            "TTY_NR" => Some(ProcessField::TtyNr),
            "TPGID" => Some(ProcessField::Tpgid),
            "MINFLT" => Some(ProcessField::Minflt),
            "MAJFLT" => Some(ProcessField::Majflt),
            "PRI" | "PRIORITY" => Some(ProcessField::Priority),
            "NI" | "NICE" => Some(ProcessField::Nice),
            "START" | "STARTTIME" => Some(ProcessField::Starttime),
            "CPU" | "PROCESSOR" => Some(ProcessField::Processor),
            "VIRT" | "M_SIZE" => Some(ProcessField::MSize),
            "RES" | "M_RESIDENT" => Some(ProcessField::MResident),
            "SHR" | "M_SHARE" => Some(ProcessField::MShare),
            "CODE" | "M_TRS" => Some(ProcessField::MText),
            "LIB" | "M_LRS" => Some(ProcessField::MLib),
            "DATA" | "M_DRS" => Some(ProcessField::MData),
            "DIRTY" | "M_DT" => Some(ProcessField::MDirty),
            "UID" | "ST_UID" => Some(ProcessField::StUid),
            "USER" => Some(ProcessField::User),
            "TIME" | "TIME+" => Some(ProcessField::Time),
            "NLWP" => Some(ProcessField::Nlwp),
            "TTY" => Some(ProcessField::Tty),
            "CMDLINE" => Some(ProcessField::CmdLine),
            "COMM" => Some(ProcessField::Comm),
            "EXE" => Some(ProcessField::Exe),
            "CWD" => Some(ProcessField::Cwd),
            "CPU%" | "PERCENT_CPU" => Some(ProcessField::PercentCpu),
            "MEM%" | "PERCENT_MEM" => Some(ProcessField::PercentMem),
            "IO_RATE" => Some(ProcessField::IORate),
            "IO_READ_RATE" => Some(ProcessField::IOReadRate),
            "IO_WRITE_RATE" => Some(ProcessField::IOWriteRate),
            "IO_PRIORITY" => Some(ProcessField::IOPriority),
            "PERCENT_IO_DELAY" | "IOD%" => Some(ProcessField::PercentIODelay),
            "PERCENT_SWAP_DELAY" | "SWPD%" => Some(ProcessField::PercentSwapDelay),
            _ => None,
        }
    }

    /// Should this field sort in descending order by default?
    pub fn default_sort_desc(self) -> bool {
        matches!(
            self,
            ProcessField::PercentCpu
                | ProcessField::PercentMem
                | ProcessField::MSize
                | ProcessField::MResident
                | ProcessField::Time
                | ProcessField::Minflt
                | ProcessField::Majflt
                | ProcessField::IORead
                | ProcessField::IOWrite
                | ProcessField::IORate
                | ProcessField::IOReadRate
                | ProcessField::IOWriteRate
                | ProcessField::PercentIODelay
                | ProcessField::PercentSwapDelay
        )
    }
}

impl Default for ProcessField {
    fn default() -> Self {
        ProcessField::PercentCpu
    }
}

/// Command line highlight information
#[derive(Debug, Clone, Default)]
pub struct CmdlineHighlight {
    pub offset: usize,
    pub length: usize,
    pub attr: i32,
    pub flags: u32,
}

/// Merged command string with highlight information
#[derive(Debug, Clone, Default)]
pub struct MergedCommand {
    pub last_update: u64,
    pub str_value: Option<String>,
    pub highlights: Vec<CmdlineHighlight>,
}

/// Represents a single process
#[derive(Debug, Clone)]
pub struct Process {
    // Basic identification
    pub pid: i32,
    pub ppid: i32,
    pub pgrp: i32,
    pub session: i32,
    pub tpgid: i32,
    pub tty_nr: u64,
    pub tty_name: Option<String>,

    // User information
    pub uid: u32,
    pub user: Option<String>,

    // Process type flags
    pub is_kernel_thread: bool,
    pub is_userland_thread: bool,
    pub is_running_in_container: Tristate,
    pub elevated_priv: Tristate,

    // Command information
    pub cmdline: Option<String>,
    pub cmdline_basename_start: usize,
    pub cmdline_basename_end: usize,
    pub comm: Option<String>,
    pub exe: Option<String>,
    pub exe_basename_offset: usize,
    pub exe_deleted: bool,
    pub uses_deleted_lib: bool,
    pub cwd: Option<String>,

    // CPU information
    pub processor: i32,
    pub percent_cpu: f32,
    pub percent_mem: f32,

    // Scheduling
    pub priority: i64,
    pub nice: i64,
    pub scheduling_policy: i32,

    // Time information
    pub time: u64,            // in hundredths of a second
    pub starttime_ctime: i64, // epoch seconds
    pub starttime_show: String,

    // Memory information (in KB)
    pub m_virt: i64,
    pub m_resident: i64,
    pub m_share: i64,
    pub m_text: i64,
    pub m_lib: i64,
    pub m_data: i64,
    pub m_dirty: i64,

    // Fault counters
    pub minflt: u64,
    pub majflt: u64,

    // Process state
    pub state: ProcessState,
    pub nlwp: i64, // number of threads

    // Tree display state
    pub tree_depth: i32,
    pub indent: i32, // Indentation bits for tree drawing (negative = last child)
    pub show_children: bool,
    pub is_visible: bool,
    pub is_root: bool, // True if this is a root process in the tree

    // I/O statistics
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub io_read_rate: f64,
    pub io_write_rate: f64,

    // Delay accounting (Linux-specific, requires CONFIG_TASKSTATS)
    pub blkio_delay_percent: f32,  // Block I/O delay %
    pub swapin_delay_percent: f32, // Swapin delay %

    // Context switches
    pub ctxt_switches: u64,

    // CGroup
    pub cgroup: Option<String>,

    // OOM
    pub oom_score: i32,

    // IO Priority (Linux-specific, from ioprio_get syscall)
    pub io_priority: i32,

    // Security
    pub sec_attr: Option<String>,

    // For display
    pub merged_command: MergedCommand,

    // Update tracking
    pub updated: bool,
    pub was_shown: bool,
    pub show_timestamp: SystemTime,

    // Tagging
    pub tagged: bool,
}

impl Process {
    /// Create a new process with the given PID
    pub fn new(pid: i32) -> Self {
        Process {
            pid,
            ppid: 0,
            pgrp: 0,
            session: 0,
            tpgid: 0,
            tty_nr: 0,
            tty_name: None,
            uid: 0,
            user: None,
            is_kernel_thread: false,
            is_userland_thread: false,
            is_running_in_container: Tristate::Initial,
            elevated_priv: Tristate::Initial,
            cmdline: None,
            cmdline_basename_start: 0,
            cmdline_basename_end: 0,
            comm: None,
            exe: None,
            exe_basename_offset: 0,
            exe_deleted: false,
            uses_deleted_lib: false,
            cwd: None,
            processor: -1,
            percent_cpu: 0.0,
            percent_mem: 0.0,
            priority: 0,
            nice: 0,
            scheduling_policy: 0,
            time: 0,
            starttime_ctime: 0,
            starttime_show: String::new(),
            m_virt: 0,
            m_resident: 0,
            m_share: 0,
            m_text: 0,
            m_lib: 0,
            m_data: 0,
            m_dirty: 0,
            minflt: 0,
            majflt: 0,
            state: ProcessState::Unknown,
            nlwp: 1,
            tree_depth: 0,
            indent: 0,
            show_children: true,
            is_visible: true,
            is_root: false,
            io_read_bytes: 0,
            io_write_bytes: 0,
            io_read_rate: 0.0,
            io_write_rate: 0.0,
            blkio_delay_percent: f32::NAN,
            swapin_delay_percent: f32::NAN,
            ctxt_switches: 0,
            cgroup: None,
            oom_score: 0,
            io_priority: -1, // -1 indicates not yet read
            sec_attr: None,
            merged_command: MergedCommand::default(),
            updated: false,
            was_shown: false,
            show_timestamp: SystemTime::now(),
            tagged: false,
        }
    }

    /// Check if this is any kind of thread
    pub fn is_thread(&self) -> bool {
        self.is_kernel_thread || self.is_userland_thread
    }

    /// Get the display command string
    pub fn get_command(&self) -> &str {
        if let Some(ref merged) = self.merged_command.str_value {
            merged.as_str()
        } else if let Some(ref cmdline) = self.cmdline {
            cmdline.as_str()
        } else if let Some(ref comm) = self.comm {
            comm.as_str()
        } else {
            "<unknown>"
        }
    }

    /// Get the basename of the command
    pub fn get_basename(&self) -> &str {
        if let Some(ref cmdline) = self.cmdline {
            if self.cmdline_basename_end > self.cmdline_basename_start {
                return &cmdline[self.cmdline_basename_start..self.cmdline_basename_end];
            }
        }
        if let Some(ref comm) = self.comm {
            return comm.as_str();
        }
        "<unknown>"
    }

    /// Get the command starting from the basename (basename + arguments)
    /// This is used when showProgramPath is false - matches C htop behavior
    /// which shows cmdline[cmdlineBasenameStart..] (not just the basename)
    pub fn get_command_from_basename(&self) -> &str {
        if let Some(ref cmdline) = self.cmdline {
            if self.cmdline_basename_start < cmdline.len() {
                return &cmdline[self.cmdline_basename_start..];
            }
        }
        // Fallback to get_command behavior
        self.get_command()
    }

    /// Update the cmdline and compute basename indices
    /// This matches C htop's Process_updateCmdline behavior
    /// - cmdline: the full command line string
    /// - basename_end: position of end of first argument (before first space), or 0 to auto-detect
    pub fn update_cmdline(&mut self, cmdline: String, basename_end: usize) {
        let end = if basename_end == 0 {
            // Auto-detect: find first space or end of string
            cmdline.find(' ').unwrap_or(cmdline.len())
        } else {
            basename_end.min(cmdline.len())
        };

        // Compute basename_start by finding last '/' before end
        // This matches C htop's skipPotentialPath logic
        let start = if cmdline.starts_with('/') {
            Self::skip_potential_path(&cmdline, end)
        } else {
            0
        };

        self.cmdline_basename_start = start;
        self.cmdline_basename_end = end;
        self.cmdline = Some(cmdline);
    }

    /// Skip potential path prefix - find position after last '/' before end
    /// This matches C htop's skipPotentialPath function
    fn skip_potential_path(cmdline: &str, end: usize) -> usize {
        if !cmdline.starts_with('/') {
            return 0;
        }

        let bytes = cmdline.as_bytes();
        let mut slash = 0;
        let mut i = 1;

        while i < end && i < bytes.len() {
            let c = bytes[i];

            if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] != 0 {
                slash = i + 1;
                i += 1;
                continue;
            }

            // Space not preceded by backslash ends the search
            if c == b' ' && (i == 0 || bytes[i - 1] != b'\\') {
                return slash;
            }

            // Colon followed by space ends the search
            if c == b':' && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
                return slash;
            }

            i += 1;
        }

        slash
    }

    /// Compare two processes by a specific field
    pub fn compare_by_field(&self, other: &Process, field: ProcessField) -> Ordering {
        match field {
            ProcessField::Pid => self.pid.cmp(&other.pid),
            ProcessField::Ppid => self.ppid.cmp(&other.ppid),
            ProcessField::Pgrp => self.pgrp.cmp(&other.pgrp),
            ProcessField::Session => self.session.cmp(&other.session),
            ProcessField::User => self.user.cmp(&other.user),
            ProcessField::StUid => self.uid.cmp(&other.uid),
            ProcessField::State => (self.state as i32).cmp(&(other.state as i32)),
            ProcessField::Priority => self.priority.cmp(&other.priority),
            ProcessField::Nice => self.nice.cmp(&other.nice),
            ProcessField::Processor => self.processor.cmp(&other.processor),
            ProcessField::PercentCpu => self
                .percent_cpu
                .partial_cmp(&other.percent_cpu)
                .unwrap_or(Ordering::Equal),
            ProcessField::PercentMem => self
                .percent_mem
                .partial_cmp(&other.percent_mem)
                .unwrap_or(Ordering::Equal),
            ProcessField::Time => self.time.cmp(&other.time),
            ProcessField::MSize => self.m_virt.cmp(&other.m_virt),
            ProcessField::MResident => self.m_resident.cmp(&other.m_resident),
            ProcessField::MShare => self.m_share.cmp(&other.m_share),
            ProcessField::Minflt => self.minflt.cmp(&other.minflt),
            ProcessField::Majflt => self.majflt.cmp(&other.majflt),
            ProcessField::Nlwp => self.nlwp.cmp(&other.nlwp),
            ProcessField::Starttime => self.starttime_ctime.cmp(&other.starttime_ctime),
            ProcessField::Command | ProcessField::CmdLine | ProcessField::Comm => {
                self.get_command().cmp(other.get_command())
            }
            _ => self.pid.cmp(&other.pid),
        }
    }

    /// Format a field value as a string for display
    pub fn format_field(&self, field: ProcessField, width: usize) -> String {
        match field {
            ProcessField::Pid => format!("{:>width$}", self.pid, width = width),
            ProcessField::Ppid => format!("{:>width$}", self.ppid, width = width),
            ProcessField::User => {
                let user = self.user.as_deref().unwrap_or("?");
                if user.len() > width {
                    format!("{:.width$}", user, width = width)
                } else {
                    format!("{:<width$}", user, width = width)
                }
            }
            ProcessField::State => format!("{}", self.state.to_char()),
            ProcessField::Priority => format!("{:>3}", self.priority),
            ProcessField::Nice => format!("{:>3}", self.nice),
            ProcessField::PercentCpu => {
                if self.percent_cpu < 10.0 {
                    format!("{:>4.1}", self.percent_cpu)
                } else if self.percent_cpu < 100.0 {
                    format!("{:>4.0}", self.percent_cpu)
                } else {
                    format!("{:>4.0}", self.percent_cpu)
                }
            }
            ProcessField::PercentMem => format!("{:>4.1}", self.percent_mem),
            ProcessField::MSize => Self::format_memory(self.m_virt),
            ProcessField::MResident => Self::format_memory(self.m_resident),
            ProcessField::MShare => Self::format_memory(self.m_share),
            ProcessField::Time => Self::format_time(self.time),
            ProcessField::Command | ProcessField::CmdLine => self.get_command().to_string(),
            ProcessField::Nlwp => format!("{:>4}", self.nlwp),
            ProcessField::Processor => format!("{:>3}", self.processor),
            ProcessField::Tty => self.tty_name.as_deref().unwrap_or("?").to_string(),
            _ => "?".to_string(),
        }
    }

    /// Format memory value with human-readable units
    pub fn format_memory(kb: i64) -> String {
        if kb < 1000 {
            format!("{:>5}K", kb)
        } else if kb < 1000 * 1024 {
            format!("{:>5.1}M", kb as f64 / 1024.0)
        } else {
            format!("{:>5.2}G", kb as f64 / 1024.0 / 1024.0)
        }
    }

    /// Format time in htop format (h:mm:ss or m:ss.ss)
    pub fn format_time(hundredths: u64) -> String {
        let total_seconds = hundredths / 100;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let centiseconds = hundredths % 100;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}.{:02}", minutes, seconds, centiseconds)
        }
    }
}

impl Default for Process {
    fn default() -> Self {
        Process::new(0)
    }
}

/// Process list that manages a collection of processes
#[derive(Debug, Default)]
pub struct ProcessList {
    pub processes: Vec<Process>,
    /// Tracks which PIDs exist (used for O(1) existence check in add())
    /// Note: Does NOT store indices - use linear search for position lookup
    pub by_pid: std::collections::HashSet<i32>,
    pub tree_display_order: Vec<i32>, // PIDs in tree display order
}

impl ProcessList {
    pub fn new() -> Self {
        ProcessList {
            processes: Vec::new(),
            by_pid: std::collections::HashSet::new(),
            tree_display_order: Vec::new(),
        }
    }

    /// Add or update a process
    pub fn add(&mut self, process: Process) {
        let pid = process.pid;
        if self.by_pid.contains(&pid) {
            // Process exists - find and update via linear search
            // (HashMap only tracks existence, not position, since indices change after sort)
            if let Some(p) = self.processes.iter_mut().find(|p| p.pid == pid) {
                *p = process;
            }
        } else {
            // New process
            self.processes.push(process);
            self.by_pid.insert(pid); // Just track that this PID exists
        }
    }

    /// Get a process by PID (linear search - called rarely for user interactions)
    pub fn get(&self, pid: i32) -> Option<&Process> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    /// Get a mutable reference to a process by PID (linear search)
    pub fn get_mut(&mut self, pid: i32) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }

    /// Remove processes that weren't updated in the last scan
    pub fn cleanup(&mut self) {
        self.processes.retain(|p| p.updated);
        // Rebuild the existence set
        self.by_pid.clear();
        for process in self.processes.iter() {
            self.by_pid.insert(process.pid);
        }
        // Mark all as not updated for next scan
        for process in &mut self.processes {
            process.updated = false;
        }
    }

    /// Sort processes by a field using insertion sort
    /// Insertion sort is O(n) for nearly-sorted data (common case between scans)
    pub fn sort_by(&mut self, field: ProcessField, ascending: bool) {
        let len = self.processes.len();
        if len <= 1 {
            return;
        }

        // Insertion sort - O(n) for nearly-sorted data, O(n²) worst case
        for i in 1..len {
            let mut j = i;
            while j > 0 {
                let cmp = self.processes[j - 1].compare_by_field(&self.processes[j], field);
                let should_swap = if ascending {
                    cmp == std::cmp::Ordering::Greater
                } else {
                    cmp == std::cmp::Ordering::Less
                };

                if should_swap {
                    self.processes.swap(j - 1, j);
                    j -= 1;
                } else {
                    break;
                }
            }
        }
        // No need to rebuild by_pid - it only tracks existence, not position
    }

    /// Get the number of processes
    pub fn len(&self) -> usize {
        self.processes.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    /// Iterator over all processes
    pub fn iter(&self) -> impl Iterator<Item = &Process> {
        self.processes.iter()
    }

    /// Mutable iterator over all processes
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Process> {
        self.processes.iter_mut()
    }

    /// Iterator over processes in tree display order (only visible ones)
    pub fn iter_tree(&self) -> impl Iterator<Item = &Process> {
        self.tree_display_order
            .iter()
            .filter_map(move |pid| self.get(*pid))
            .filter(|p| p.is_visible)
    }

    /// Build tree structure - sets indent and tree_depth for each process
    /// Also stores the display order in tree_display_order
    pub fn build_tree(&mut self) {
        // First, mark root processes and set all as visible
        let pids: std::collections::HashSet<i32> = self.processes.iter().map(|p| p.pid).collect();

        for process in &mut self.processes {
            // A process is a root if:
            // 1. ppid == 0 (truly root), OR
            // 2. ppid == pid (self-parented), OR
            // 3. ppid doesn't exist AND ppid != 1 (orphan, but not if parent is init/launchd)
            // Note: On macOS, we may not have permission to see PID 1 (launchd),
            // but we know it exists, so treat ppid=1 processes as children of an implicit root
            let is_orphan = !pids.contains(&process.ppid) && process.ppid != 1;
            process.is_root = process.ppid == 0 || process.ppid == process.pid || is_orphan;
            process.indent = 0;
            process.tree_depth = 0;
            process.is_visible = true; // Reset visibility
        }

        // Sort by parent, then by comparison key (roots first)
        self.processes.sort_by(|a, b| {
            let a_parent = if a.is_root { 0 } else { a.ppid };
            let b_parent = if b.is_root { 0 } else { b.ppid };
            match a_parent.cmp(&b_parent) {
                std::cmp::Ordering::Equal => a.pid.cmp(&b.pid),
                other => other,
            }
        });

        // No need to rebuild by_pid - it only tracks existence, not position

        // Build display list in tree order
        let mut display_list: Vec<i32> = Vec::new();

        // Check if PID 1 exists in our process list
        let has_pid_1 = pids.contains(&1);

        // Process root nodes
        let root_indices: Vec<usize> = self
            .processes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_root)
            .map(|(i, _)| i)
            .collect();

        for idx in root_indices {
            let pid = self.processes[idx].pid;
            self.processes[idx].indent = 0;
            self.processes[idx].tree_depth = 0;
            self.processes[idx].is_visible = true; // Root processes are always visible
            display_list.push(pid);

            let show_children = self.processes[idx].show_children;
            self.build_tree_branch(pid, 0, 0, show_children, &mut display_list);
        }

        // If PID 1 is not in our list, handle its children specially
        // These processes have ppid=1 but aren't marked as roots (they're children of the implicit init)
        if !has_pid_1 {
            // Process children of PID 1 as top-level entries (like roots but with tree connectors)
            let pid_1_children: Vec<(usize, i32)> = self
                .processes
                .iter()
                .enumerate()
                .filter(|(_, p)| p.ppid == 1 && !p.is_root)
                .map(|(i, p)| (i, p.pid))
                .collect();

            if !pid_1_children.is_empty() {
                let last_idx = pid_1_children.len() - 1;

                for (i, (idx, pid)) in pid_1_children.iter().enumerate() {
                    let is_last = i == last_idx;

                    // Add to display list
                    display_list.push(*pid);

                    // Set indent for PID 1 children (depth 1)
                    let next_indent = 1; // 1 << 0 = 1
                    let process = &mut self.processes[*idx];
                    if is_last {
                        process.indent = -next_indent;
                    } else {
                        process.indent = next_indent;
                    }
                    process.tree_depth = 1;
                    process.is_visible = true;

                    // Recursively process children
                    let child_indent = if is_last { 0 } else { next_indent };
                    let child_show = process.show_children;
                    self.build_tree_branch(*pid, 1, child_indent, child_show, &mut display_list);
                }
            }
        }

        // Store the display order
        self.tree_display_order = display_list;
    }

    /// Recursively build tree branch
    fn build_tree_branch(
        &mut self,
        parent_id: i32,
        level: u32,
        indent: i32,
        show: bool,
        display_list: &mut Vec<i32>,
    ) {
        // Find all children of this parent (matching C htop Row_isChildOf check)
        let children: Vec<(usize, i32)> = self
            .processes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.ppid == parent_id && p.pid != parent_id)
            .map(|(i, p)| (i, p.pid))
            .collect();

        if children.is_empty() {
            return;
        }

        let last_idx = children.len() - 1;

        for (i, (idx, pid)) in children.iter().enumerate() {
            let is_last = i == last_idx;

            // Add to display list first (like C htop line 127)
            display_list.push(*pid);

            // Calculate indent bits (matches C htop Table.c line 129)
            let next_indent = indent | (1i32 << std::cmp::min(level, 30));

            // Recursive call (like C htop line 130)
            // Pass next_indent to children if this is NOT the last item, otherwise pass current indent
            let child_indent = if is_last { indent } else { next_indent };
            let process = &self.processes[*idx];
            let child_show = show && process.show_children;
            self.build_tree_branch(*pid, level + 1, child_indent, child_show, display_list);

            // NOW set indent on this process (like C htop lines 131-134)
            let process = &mut self.processes[*idx];
            if is_last {
                process.indent = -next_indent; // Negative indicates last child
            } else {
                process.indent = next_indent;
            }

            process.tree_depth = (level + 1) as i32;
            process.is_visible = show;
        }
    }

    /// Expand all tree branches
    pub fn expand_all(&mut self) {
        for process in &mut self.processes {
            process.show_children = true;
        }
    }

    /// Collapse all tree branches (except roots)
    pub fn collapse_all(&mut self) {
        for process in &mut self.processes {
            if process.tree_depth > 0 && process.pid > 1 {
                process.show_children = false;
            }
        }
    }

    /// Toggle tag on a process
    pub fn toggle_tag(&mut self, pid: i32) {
        if let Some(process) = self.get_mut(pid) {
            process.tagged = !process.tagged;
        }
    }

    /// Tag a process and all its descendants
    pub fn tag_with_children(&mut self, pid: i32) {
        // First tag the process itself
        if let Some(process) = self.get_mut(pid) {
            process.tagged = true;
        }

        // Collect all descendant PIDs
        let descendants: Vec<i32> = self.collect_descendants(pid);

        // Tag all descendants
        for desc_pid in descendants {
            if let Some(process) = self.get_mut(desc_pid) {
                process.tagged = true;
            }
        }
    }

    /// Collect all descendant PIDs of a process
    fn collect_descendants(&self, pid: i32) -> Vec<i32> {
        let mut descendants = Vec::new();
        let mut to_visit: Vec<i32> = self
            .processes
            .iter()
            .filter(|p| p.ppid == pid && p.pid != pid)
            .map(|p| p.pid)
            .collect();

        while let Some(current) = to_visit.pop() {
            descendants.push(current);
            // Add children of current
            for p in &self.processes {
                if p.ppid == current && p.pid != current {
                    to_visit.push(p.pid);
                }
            }
        }

        descendants
    }

    /// Untag all processes
    pub fn untag_all(&mut self) {
        for process in &mut self.processes {
            process.tagged = false;
        }
    }

    /// Get all tagged PIDs
    pub fn get_tagged(&self) -> Vec<i32> {
        self.processes
            .iter()
            .filter(|p| p.tagged)
            .map(|p| p.pid)
            .collect()
    }

    /// Expand a specific tree node
    pub fn expand_tree(&mut self, pid: i32) {
        if let Some(process) = self.get_mut(pid) {
            process.show_children = true;
        }
    }

    /// Collapse a specific tree node
    pub fn collapse_tree(&mut self, pid: i32) {
        if let Some(process) = self.get_mut(pid) {
            process.show_children = false;
        }
    }

    /// Toggle all tree nodes (if any collapsed, expand all; otherwise collapse all)
    pub fn toggle_all_tree(&mut self) {
        // Check if any non-root process has show_children = false
        let any_collapsed = self
            .processes
            .iter()
            .any(|p| p.tree_depth >= 0 && !p.show_children);

        if any_collapsed {
            self.expand_all();
        } else {
            self.collapse_all();
        }
    }

    /// Sort processes by field
    pub fn sort(&mut self, field: ProcessField, descending: bool) {
        self.sort_by(field, !descending);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_building() {
        let mut pl = ProcessList::new();

        // Create a simple tree: 1 (root) -> 2, 5; 2 -> 3, 4
        let mut p1 = Process::new(1);
        p1.ppid = 0;
        p1.comm = Some("root".to_string());
        pl.add(p1);

        let mut p2 = Process::new(2);
        p2.ppid = 1;
        p2.comm = Some("child1".to_string());
        pl.add(p2);

        let mut p3 = Process::new(3);
        p3.ppid = 2;
        p3.comm = Some("grandchild1".to_string());
        pl.add(p3);

        let mut p4 = Process::new(4);
        p4.ppid = 2;
        p4.comm = Some("grandchild2".to_string());
        pl.add(p4);

        let mut p5 = Process::new(5);
        p5.ppid = 1;
        p5.comm = Some("child2".to_string());
        pl.add(p5);

        pl.build_tree();

        println!("tree_display_order: {:?}", pl.tree_display_order);
        for pid in &pl.tree_display_order {
            let p = pl.get(*pid).unwrap();
            println!(
                "  PID {} (ppid={}) indent={} depth={} is_root={} is_visible={}",
                p.pid, p.ppid, p.indent, p.tree_depth, p.is_root, p.is_visible
            );
        }

        // Expected tree structure:
        // 1 (root, indent=0, depth=0)
        //   2 (child, indent=1, depth=1)
        //     3 (grandchild, indent=3, depth=2)
        //     4 (last grandchild, indent=-3, depth=2)
        //   5 (last child, indent=-1, depth=1)

        assert_eq!(pl.tree_display_order, vec![1, 2, 3, 4, 5]);

        // Check root
        let p1 = pl.get(1).unwrap();
        assert_eq!(p1.indent, 0);
        assert_eq!(p1.tree_depth, 0);
        assert!(p1.is_root);

        // Check first child (not last)
        let p2 = pl.get(2).unwrap();
        assert_eq!(p2.indent, 1);
        assert_eq!(p2.tree_depth, 1);

        // Check grandchild (not last)
        let p3 = pl.get(3).unwrap();
        assert_eq!(p3.indent, 3); // 1 | (1 << 1) = 3
        assert_eq!(p3.tree_depth, 2);

        // Check last grandchild
        let p4 = pl.get(4).unwrap();
        assert_eq!(p4.indent, -3); // negative indicates last
        assert_eq!(p4.tree_depth, 2);

        // Check last child
        let p5 = pl.get(5).unwrap();
        assert_eq!(p5.indent, -1); // negative indicates last
        assert_eq!(p5.tree_depth, 1);
    }

    #[test]
    fn test_macos_like_tree() {
        // Simulate macOS-like scenario:
        // 1 (launchd, ppid=0) -> many direct children, some with their own children
        let mut pl = ProcessList::new();

        // launchd (PID 1)
        let mut p1 = Process::new(1);
        p1.ppid = 0;
        p1.comm = Some("launchd".to_string());
        pl.add(p1);

        // Direct children of launchd
        let mut p100 = Process::new(100);
        p100.ppid = 1;
        p100.comm = Some("logd".to_string());
        pl.add(p100);

        // Slack main (direct child of launchd)
        let mut p5259 = Process::new(5259);
        p5259.ppid = 1;
        p5259.comm = Some("Slack".to_string());
        pl.add(p5259);

        // Slack helpers (children of Slack main)
        let mut p5268 = Process::new(5268);
        p5268.ppid = 5259;
        p5268.comm = Some("Slack Helper (GPU)".to_string());
        pl.add(p5268);

        let mut p5269 = Process::new(5269);
        p5269.ppid = 5259;
        p5269.comm = Some("Slack Helper".to_string());
        pl.add(p5269);

        let mut p5270 = Process::new(5270);
        p5270.ppid = 5259;
        p5270.comm = Some("Slack Helper (Renderer)".to_string());
        pl.add(p5270);

        // Another direct child of launchd (after Slack in PID order)
        let mut p6000 = Process::new(6000);
        p6000.ppid = 1;
        p6000.comm = Some("other".to_string());
        pl.add(p6000);

        pl.build_tree();

        println!("\nMacOS-like tree:");
        println!("tree_display_order: {:?}", pl.tree_display_order);
        for pid in &pl.tree_display_order {
            let p = pl.get(*pid).unwrap();
            let indent_str = "  ".repeat(p.tree_depth as usize);
            println!(
                "{}PID {} ({}) indent={} depth={} is_root={}",
                indent_str,
                p.pid,
                p.comm.as_deref().unwrap_or("?"),
                p.indent,
                p.tree_depth,
                p.is_root
            );
        }

        // Verify tree structure
        // launchd should be root
        let launchd = pl.get(1).unwrap();
        assert!(launchd.is_root);
        assert_eq!(launchd.indent, 0);

        // Slack main should be a child of launchd (depth 1)
        let slack = pl.get(5259).unwrap();
        assert!(!slack.is_root);
        assert_eq!(slack.tree_depth, 1);
        assert!(
            slack.indent != 0,
            "Slack main should have indent != 0, got {}",
            slack.indent
        );

        // Slack helpers should be children of Slack (depth 2)
        let helper_gpu = pl.get(5268).unwrap();
        assert_eq!(helper_gpu.tree_depth, 2);
        assert!(
            helper_gpu.indent != 0,
            "Slack Helper GPU should have indent != 0"
        );

        let helper = pl.get(5269).unwrap();
        assert_eq!(helper.tree_depth, 2);

        let helper_renderer = pl.get(5270).unwrap();
        assert_eq!(helper_renderer.tree_depth, 2);
        // Last child should have negative indent
        assert!(
            helper_renderer.indent < 0,
            "Last Slack helper should have negative indent"
        );
    }

    #[test]
    fn test_tree_without_pid_1() {
        // Simulate macOS scenario where we can't see PID 1 (launchd)
        // but processes have ppid = 1
        let mut pl = ProcessList::new();

        // NO PID 1 - we don't have permission to see it

        // Direct children of launchd (ppid=1)
        let mut p100 = Process::new(100);
        p100.ppid = 1;
        p100.comm = Some("logd".to_string());
        pl.add(p100);

        let mut p200 = Process::new(200);
        p200.ppid = 1;
        p200.comm = Some("configd".to_string());
        pl.add(p200);

        // Process with child
        let mut p5259 = Process::new(5259);
        p5259.ppid = 1;
        p5259.comm = Some("Slack".to_string());
        pl.add(p5259);

        // Slack's children
        let mut p5268 = Process::new(5268);
        p5268.ppid = 5259;
        p5268.comm = Some("Slack Helper".to_string());
        pl.add(p5268);

        let mut p5269 = Process::new(5269);
        p5269.ppid = 5259;
        p5269.comm = Some("Slack Helper 2".to_string());
        pl.add(p5269);

        pl.build_tree();

        println!("\nTree without PID 1:");
        println!("tree_display_order: {:?}", pl.tree_display_order);
        for pid in &pl.tree_display_order {
            let p = pl.get(*pid).unwrap();
            let indent_str = "  ".repeat(p.tree_depth as usize);
            println!(
                "{}PID {} ({}) indent={} depth={} is_root={}",
                indent_str,
                p.pid,
                p.comm.as_deref().unwrap_or("?"),
                p.indent,
                p.tree_depth,
                p.is_root
            );
        }

        // Processes with ppid=1 should be treated as top-level children (depth 1)
        // since PID 1 doesn't exist in our list
        let logd = pl.get(100).unwrap();
        assert!(!logd.is_root, "logd should NOT be marked as root");
        assert_eq!(logd.tree_depth, 1, "logd should be at depth 1");
        assert!(logd.indent != 0, "logd should have non-zero indent");

        let slack = pl.get(5259).unwrap();
        assert!(!slack.is_root, "Slack should NOT be marked as root");
        assert_eq!(slack.tree_depth, 1, "Slack should be at depth 1");
        assert!(slack.indent != 0, "Slack should have non-zero indent");

        // Slack's children should be at depth 2
        let helper = pl.get(5268).unwrap();
        assert_eq!(helper.tree_depth, 2, "Slack helper should be at depth 2");
        assert!(
            helper.indent != 0,
            "Slack helper should have non-zero indent"
        );
    }
}
