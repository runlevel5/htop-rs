//! Process representation
//!
//! This module contains the Process struct and related types that represent
//! a single process in the system.

use std::cmp::Ordering;
use std::time::SystemTime;

/// Process state enum - core states shared by all platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProcessState {
    #[default]
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

/// Tristate for optional boolean values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tristate {
    #[default]
    Initial,
    Off,
    On,
}

/// Process fields for display and sorting
///
/// Field numbers match C htop's RowField.h for compatibility.
/// Platform-specific fields are conditionally compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
#[derive(Default)]
pub enum ProcessField {
    // === Common fields (all platforms) - from RowField.h ===
    Pid = 1,
    Command = 2,
    State = 3,
    Ppid = 4,
    Pgrp = 5,
    Session = 6,
    Tty = 7,
    Tpgid = 8,
    Minflt = 10,
    Majflt = 12,
    Priority = 18,
    Nice = 19,
    Starttime = 21,
    Processor = 38,
    MSize = 39,      // M_VIRT
    MResident = 40,  // M_RESIDENT
    StUid = 46,
    #[default]
    PercentCpu = 47,
    PercentMem = 48,
    User = 49,
    Time = 50,
    Nlwp = 51,
    Tgid = 52,
    PercentNormCpu = 53,
    Elapsed = 54,
    SchedulerPolicy = 55,
    ProcComm = 124,
    ProcExe = 125,
    Cwd = 126,

    // === Linux-specific fields - from linux/ProcessField.h ===
    #[cfg(target_os = "linux")]
    Cminflt = 11,
    #[cfg(target_os = "linux")]
    Cmajflt = 13,
    #[cfg(target_os = "linux")]
    Utime = 14,
    #[cfg(target_os = "linux")]
    Stime = 15,
    #[cfg(target_os = "linux")]
    Cutime = 16,
    #[cfg(target_os = "linux")]
    Cstime = 17,
    #[cfg(target_os = "linux")]
    MShare = 41,
    #[cfg(target_os = "linux")]
    MText = 42,  // M_TRS (CODE)
    #[cfg(target_os = "linux")]
    MData = 43,  // M_DRS (DATA)
    #[cfg(target_os = "linux")]
    MLib = 44,   // M_LRS (LIB)
    #[cfg(target_os = "linux")]
    Rchar = 103,
    #[cfg(target_os = "linux")]
    Wchar = 104,
    #[cfg(target_os = "linux")]
    Syscr = 105,
    #[cfg(target_os = "linux")]
    Syscw = 106,
    #[cfg(target_os = "linux")]
    Rbytes = 107,
    #[cfg(target_os = "linux")]
    Wbytes = 108,
    #[cfg(target_os = "linux")]
    Cnclwb = 109,
    #[cfg(target_os = "linux")]
    IOReadRate = 110,
    #[cfg(target_os = "linux")]
    IOWriteRate = 111,
    #[cfg(target_os = "linux")]
    IORate = 112,
    #[cfg(target_os = "linux")]
    CGroup = 113,
    #[cfg(target_os = "linux")]
    Oom = 114,
    #[cfg(target_os = "linux")]
    IOPriority = 115,
    #[cfg(target_os = "linux")]
    PercentCpuDelay = 116,
    #[cfg(target_os = "linux")]
    PercentIODelay = 117,
    #[cfg(target_os = "linux")]
    PercentSwapDelay = 118,
    #[cfg(target_os = "linux")]
    MPss = 119,
    #[cfg(target_os = "linux")]
    MSwap = 120,
    #[cfg(target_os = "linux")]
    MPsswp = 121,
    #[cfg(target_os = "linux")]
    Ctxt = 122,
    #[cfg(target_os = "linux")]
    SecAttr = 123,
    #[cfg(target_os = "linux")]
    AutogroupId = 127,
    #[cfg(target_os = "linux")]
    AutogroupNice = 128,
    #[cfg(target_os = "linux")]
    CCGroup = 129,
    #[cfg(target_os = "linux")]
    Container = 130,
    #[cfg(target_os = "linux")]
    MPriv = 131,
    #[cfg(target_os = "linux")]
    GpuTime = 132,
    #[cfg(target_os = "linux")]
    GpuPercent = 133,
    #[cfg(target_os = "linux")]
    IsContainer = 134,

    // === macOS-specific fields - from darwin/ProcessField.h ===
    #[cfg(target_os = "macos")]
    Translated = 100,
}

impl ProcessField {
    /// Get all process fields for the current platform
    pub fn all() -> Vec<ProcessField> {
        let mut fields = vec![
            // Common fields (all platforms)
            ProcessField::Pid,
            ProcessField::Command,
            ProcessField::State,
            ProcessField::Ppid,
            ProcessField::Pgrp,
            ProcessField::Session,
            ProcessField::Tty,
            ProcessField::Tpgid,
            ProcessField::Minflt,
            ProcessField::Majflt,
            ProcessField::Priority,
            ProcessField::Nice,
            ProcessField::Starttime,
            ProcessField::Processor,
            ProcessField::MSize,
            ProcessField::MResident,
            ProcessField::StUid,
            ProcessField::PercentCpu,
            ProcessField::PercentMem,
            ProcessField::User,
            ProcessField::Time,
            ProcessField::Nlwp,
            ProcessField::Tgid,
            ProcessField::PercentNormCpu,
            ProcessField::Elapsed,
            ProcessField::SchedulerPolicy,
            ProcessField::ProcComm,
            ProcessField::ProcExe,
            ProcessField::Cwd,
        ];

        // Linux-specific fields
        #[cfg(target_os = "linux")]
        {
            fields.extend([
                ProcessField::Cminflt,
                ProcessField::Cmajflt,
                ProcessField::Utime,
                ProcessField::Stime,
                ProcessField::Cutime,
                ProcessField::Cstime,
                ProcessField::MShare,
                ProcessField::MText,
                ProcessField::MData,
                ProcessField::MLib,
                ProcessField::Rchar,
                ProcessField::Wchar,
                ProcessField::Syscr,
                ProcessField::Syscw,
                ProcessField::Rbytes,
                ProcessField::Wbytes,
                ProcessField::Cnclwb,
                ProcessField::IOReadRate,
                ProcessField::IOWriteRate,
                ProcessField::IORate,
                ProcessField::CGroup,
                ProcessField::Oom,
                ProcessField::IOPriority,
                ProcessField::PercentCpuDelay,
                ProcessField::PercentIODelay,
                ProcessField::PercentSwapDelay,
                ProcessField::MPss,
                ProcessField::MSwap,
                ProcessField::MPsswp,
                ProcessField::Ctxt,
                ProcessField::SecAttr,
                ProcessField::AutogroupId,
                ProcessField::AutogroupNice,
                ProcessField::CCGroup,
                ProcessField::Container,
                ProcessField::MPriv,
                ProcessField::GpuTime,
                ProcessField::GpuPercent,
                ProcessField::IsContainer,
            ]);
        }

        // macOS-specific fields
        #[cfg(target_os = "macos")]
        {
            fields.push(ProcessField::Translated);
        }

        fields
    }

    /// Get the name of a process field (matches C htop naming convention)
    pub fn name(self) -> &'static str {
        match self {
            // Common fields
            ProcessField::Pid => "PID",
            ProcessField::Command => "Command",
            ProcessField::State => "STATE",
            ProcessField::Ppid => "PPID",
            ProcessField::Pgrp => "PGRP",
            ProcessField::Session => "SESSION",
            ProcessField::Tty => "TTY",
            ProcessField::Tpgid => "TPGID",
            ProcessField::Minflt => "MINFLT",
            ProcessField::Majflt => "MAJFLT",
            ProcessField::Priority => "PRIORITY",
            ProcessField::Nice => "NICE",
            ProcessField::Starttime => "STARTTIME",
            ProcessField::Processor => "PROCESSOR",
            ProcessField::MSize => "M_VIRT",
            ProcessField::MResident => "M_RESIDENT",
            ProcessField::StUid => "ST_UID",
            ProcessField::PercentCpu => "PERCENT_CPU",
            ProcessField::PercentMem => "PERCENT_MEM",
            ProcessField::User => "USER",
            ProcessField::Time => "TIME",
            ProcessField::Nlwp => "NLWP",
            ProcessField::Tgid => "TGID",
            ProcessField::PercentNormCpu => "PERCENT_NORM_CPU",
            ProcessField::Elapsed => "ELAPSED",
            ProcessField::SchedulerPolicy => "SCHEDULERPOLICY",
            ProcessField::ProcComm => "COMM",
            ProcessField::ProcExe => "EXE",
            ProcessField::Cwd => "CWD",

            // Linux-specific fields
            #[cfg(target_os = "linux")]
            ProcessField::Cminflt => "CMINFLT",
            #[cfg(target_os = "linux")]
            ProcessField::Cmajflt => "CMAJFLT",
            #[cfg(target_os = "linux")]
            ProcessField::Utime => "UTIME",
            #[cfg(target_os = "linux")]
            ProcessField::Stime => "STIME",
            #[cfg(target_os = "linux")]
            ProcessField::Cutime => "CUTIME",
            #[cfg(target_os = "linux")]
            ProcessField::Cstime => "CSTIME",
            #[cfg(target_os = "linux")]
            ProcessField::MShare => "M_SHARE",
            #[cfg(target_os = "linux")]
            ProcessField::MText => "M_TRS",
            #[cfg(target_os = "linux")]
            ProcessField::MData => "M_DRS",
            #[cfg(target_os = "linux")]
            ProcessField::MLib => "M_LRS",
            #[cfg(target_os = "linux")]
            ProcessField::Rchar => "RCHAR",
            #[cfg(target_os = "linux")]
            ProcessField::Wchar => "WCHAR",
            #[cfg(target_os = "linux")]
            ProcessField::Syscr => "SYSCR",
            #[cfg(target_os = "linux")]
            ProcessField::Syscw => "SYSCW",
            #[cfg(target_os = "linux")]
            ProcessField::Rbytes => "RBYTES",
            #[cfg(target_os = "linux")]
            ProcessField::Wbytes => "WBYTES",
            #[cfg(target_os = "linux")]
            ProcessField::Cnclwb => "CNCLWB",
            #[cfg(target_os = "linux")]
            ProcessField::IOReadRate => "IO_READ_RATE",
            #[cfg(target_os = "linux")]
            ProcessField::IOWriteRate => "IO_WRITE_RATE",
            #[cfg(target_os = "linux")]
            ProcessField::IORate => "IO_RATE",
            #[cfg(target_os = "linux")]
            ProcessField::CGroup => "CGROUP",
            #[cfg(target_os = "linux")]
            ProcessField::Oom => "OOM",
            #[cfg(target_os = "linux")]
            ProcessField::IOPriority => "IO_PRIORITY",
            #[cfg(target_os = "linux")]
            ProcessField::PercentCpuDelay => "PERCENT_CPU_DELAY",
            #[cfg(target_os = "linux")]
            ProcessField::PercentIODelay => "PERCENT_IO_DELAY",
            #[cfg(target_os = "linux")]
            ProcessField::PercentSwapDelay => "PERCENT_SWAP_DELAY",
            #[cfg(target_os = "linux")]
            ProcessField::MPss => "M_PSS",
            #[cfg(target_os = "linux")]
            ProcessField::MSwap => "M_SWAP",
            #[cfg(target_os = "linux")]
            ProcessField::MPsswp => "M_PSSWP",
            #[cfg(target_os = "linux")]
            ProcessField::Ctxt => "CTXT",
            #[cfg(target_os = "linux")]
            ProcessField::SecAttr => "SECATTR",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupId => "AUTOGROUP_ID",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupNice => "AUTOGROUP_NICE",
            #[cfg(target_os = "linux")]
            ProcessField::CCGroup => "CCGROUP",
            #[cfg(target_os = "linux")]
            ProcessField::Container => "CONTAINER",
            #[cfg(target_os = "linux")]
            ProcessField::MPriv => "M_PRIV",
            #[cfg(target_os = "linux")]
            ProcessField::GpuTime => "GPU_TIME",
            #[cfg(target_os = "linux")]
            ProcessField::GpuPercent => "GPU_PERCENT",
            #[cfg(target_os = "linux")]
            ProcessField::IsContainer => "ISCONTAINER",

            // macOS-specific fields
            #[cfg(target_os = "macos")]
            ProcessField::Translated => "TRANSLATED",
        }
    }

    /// Get the title (column header) for a process field
    pub fn title(self) -> &'static str {
        match self {
            // Common fields
            ProcessField::Pid => "PID",
            ProcessField::Command => "Command ",
            ProcessField::State => "S ",
            ProcessField::Ppid => "PPID",
            ProcessField::Pgrp => "PGRP",
            ProcessField::Session => "SID",
            ProcessField::Tty => "TTY      ",
            ProcessField::Tpgid => "TPGID",
            ProcessField::Minflt => "     MINFLT ",
            ProcessField::Majflt => "     MAJFLT ",
            ProcessField::Priority => "PRI ",
            ProcessField::Nice => " NI ",
            ProcessField::Starttime => "START ",
            ProcessField::Processor => "CPU ",
            ProcessField::MSize => " VIRT ",
            ProcessField::MResident => "  RES ",
            ProcessField::StUid => "UID",
            ProcessField::PercentCpu => " CPU%",
            ProcessField::PercentMem => "MEM% ",
            ProcessField::User => "USER       ",
            ProcessField::Time => "  TIME+  ",
            ProcessField::Nlwp => "NLWP ",
            ProcessField::Tgid => "TGID",
            ProcessField::PercentNormCpu => "NCPU%",
            ProcessField::Elapsed => "ELAPSED  ",
            ProcessField::SchedulerPolicy => "SCHED ",
            ProcessField::ProcComm => "COMM            ",
            ProcessField::ProcExe => "EXE             ",
            ProcessField::Cwd => "CWD                       ",

            // Linux-specific fields
            #[cfg(target_os = "linux")]
            ProcessField::Cminflt => "    CMINFLT ",
            #[cfg(target_os = "linux")]
            ProcessField::Cmajflt => "    CMAJFLT ",
            #[cfg(target_os = "linux")]
            ProcessField::Utime => " UTIME+  ",
            #[cfg(target_os = "linux")]
            ProcessField::Stime => " STIME+  ",
            #[cfg(target_os = "linux")]
            ProcessField::Cutime => " CUTIME+ ",
            #[cfg(target_os = "linux")]
            ProcessField::Cstime => " CSTIME+ ",
            #[cfg(target_os = "linux")]
            ProcessField::MShare => "  SHR ",
            #[cfg(target_os = "linux")]
            ProcessField::MText => " CODE ",
            #[cfg(target_os = "linux")]
            ProcessField::MData => " DATA ",
            #[cfg(target_os = "linux")]
            ProcessField::MLib => "  LIB ",
            #[cfg(target_os = "linux")]
            ProcessField::Rchar => "RCHAR ",
            #[cfg(target_os = "linux")]
            ProcessField::Wchar => "WCHAR ",
            #[cfg(target_os = "linux")]
            ProcessField::Syscr => "  READ_SYSC ",
            #[cfg(target_os = "linux")]
            ProcessField::Syscw => " WRITE_SYSC ",
            #[cfg(target_os = "linux")]
            ProcessField::Rbytes => " IO_R ",
            #[cfg(target_os = "linux")]
            ProcessField::Wbytes => " IO_W ",
            #[cfg(target_os = "linux")]
            ProcessField::Cnclwb => " IO_C ",
            #[cfg(target_os = "linux")]
            ProcessField::IOReadRate => "  DISK READ ",
            #[cfg(target_os = "linux")]
            ProcessField::IOWriteRate => " DISK WRITE ",
            #[cfg(target_os = "linux")]
            ProcessField::IORate => "   DISK R/W ",
            #[cfg(target_os = "linux")]
            ProcessField::CGroup => "CGROUP (raw)",
            #[cfg(target_os = "linux")]
            ProcessField::Oom => " OOM ",
            #[cfg(target_os = "linux")]
            ProcessField::IOPriority => "IO ",
            #[cfg(target_os = "linux")]
            ProcessField::PercentCpuDelay => "CPUD% ",
            #[cfg(target_os = "linux")]
            ProcessField::PercentIODelay => " IOD% ",
            #[cfg(target_os = "linux")]
            ProcessField::PercentSwapDelay => "SWPD% ",
            #[cfg(target_os = "linux")]
            ProcessField::MPss => "  PSS ",
            #[cfg(target_os = "linux")]
            ProcessField::MSwap => " SWAP ",
            #[cfg(target_os = "linux")]
            ProcessField::MPsswp => " PSSWP ",
            #[cfg(target_os = "linux")]
            ProcessField::Ctxt => " CTXT ",
            #[cfg(target_os = "linux")]
            ProcessField::SecAttr => "Security Attribute",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupId => "AGRP",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupNice => " ANI",
            #[cfg(target_os = "linux")]
            ProcessField::CCGroup => "CGROUP (compressed)",
            #[cfg(target_os = "linux")]
            ProcessField::Container => "CONTAINER",
            #[cfg(target_os = "linux")]
            ProcessField::MPriv => " PRIV ",
            #[cfg(target_os = "linux")]
            ProcessField::GpuTime => "GPU_TIME ",
            #[cfg(target_os = "linux")]
            ProcessField::GpuPercent => " GPU% ",
            #[cfg(target_os = "linux")]
            ProcessField::IsContainer => "CONT ",

            // macOS-specific fields
            #[cfg(target_os = "macos")]
            ProcessField::Translated => "T ",
        }
    }

    /// Get the description for a process field
    pub fn description(self) -> &'static str {
        match self {
            // Common fields
            ProcessField::Pid => "Process/thread ID",
            ProcessField::Command => "Command line (insert as last column only)",
            ProcessField::State => "Process state (S sleeping, R running, D disk, Z zombie, T traced, W paging)",
            ProcessField::Ppid => "Parent process ID",
            ProcessField::Pgrp => "Process group ID",
            ProcessField::Session => "Process's session ID",
            ProcessField::Tty => "Controlling terminal",
            ProcessField::Tpgid => "Process ID of the fg process group of the controlling terminal",
            ProcessField::Minflt => "Number of minor faults which have not required loading a memory page from disk",
            ProcessField::Majflt => "Number of major faults which have required loading a memory page from disk",
            ProcessField::Priority => "Kernel's internal priority for the process",
            ProcessField::Nice => "Nice value (the higher the value, the more it lets other processes take priority)",
            ProcessField::Starttime => "Time the process was started",
            ProcessField::Processor => "Id of the CPU the process last executed on",
            ProcessField::MSize => "Total program size in virtual memory",
            ProcessField::MResident => "Resident set size, size of the text and data sections, plus stack usage",
            ProcessField::StUid => "User ID of the process owner",
            ProcessField::PercentCpu => "Percentage of the CPU time the process used in the last sampling",
            ProcessField::PercentMem => "Percentage of the memory the process is using, based on resident memory size",
            ProcessField::User => "Username of the process owner (or user ID if name cannot be determined)",
            ProcessField::Time => "Total time the process has spent in user and system time",
            ProcessField::Nlwp => "Number of threads in the process",
            ProcessField::Tgid => "Thread group ID (i.e. process ID)",
            ProcessField::PercentNormCpu => "Normalized percentage of the CPU time the process used in the last sampling (normalized by cpu count)",
            ProcessField::Elapsed => "Time since the process was started",
            ProcessField::SchedulerPolicy => "Current scheduling policy of the process",
            ProcessField::ProcComm => "comm string of the process from /proc/[pid]/comm",
            ProcessField::ProcExe => "Basename of exe of the process from /proc/[pid]/exe",
            ProcessField::Cwd => "The current working directory of the process",

            // Linux-specific fields
            #[cfg(target_os = "linux")]
            ProcessField::Cminflt => "Children processes' minor faults",
            #[cfg(target_os = "linux")]
            ProcessField::Cmajflt => "Children processes' major faults",
            #[cfg(target_os = "linux")]
            ProcessField::Utime => "User CPU time - time the process spent executing in user mode",
            #[cfg(target_os = "linux")]
            ProcessField::Stime => "System CPU time - time the kernel spent running system calls for this process",
            #[cfg(target_os = "linux")]
            ProcessField::Cutime => "Children processes' user CPU time",
            #[cfg(target_os = "linux")]
            ProcessField::Cstime => "Children processes' system CPU time",
            #[cfg(target_os = "linux")]
            ProcessField::MShare => "Size of the process's shared pages",
            #[cfg(target_os = "linux")]
            ProcessField::MText => "Size of the .text segment of the process (CODE)",
            #[cfg(target_os = "linux")]
            ProcessField::MData => "Size of the .data segment plus stack usage of the process (DATA)",
            #[cfg(target_os = "linux")]
            ProcessField::MLib => "The library size of the process (calculated from memory maps)",
            #[cfg(target_os = "linux")]
            ProcessField::Rchar => "Number of bytes the process has read",
            #[cfg(target_os = "linux")]
            ProcessField::Wchar => "Number of bytes the process has written",
            #[cfg(target_os = "linux")]
            ProcessField::Syscr => "Number of read(2) syscalls for the process",
            #[cfg(target_os = "linux")]
            ProcessField::Syscw => "Number of write(2) syscalls for the process",
            #[cfg(target_os = "linux")]
            ProcessField::Rbytes => "Bytes of read(2) I/O for the process",
            #[cfg(target_os = "linux")]
            ProcessField::Wbytes => "Bytes of write(2) I/O for the process",
            #[cfg(target_os = "linux")]
            ProcessField::Cnclwb => "Bytes of cancelled write(2) I/O",
            #[cfg(target_os = "linux")]
            ProcessField::IOReadRate => "The I/O rate of read(2) in bytes per second for the process",
            #[cfg(target_os = "linux")]
            ProcessField::IOWriteRate => "The I/O rate of write(2) in bytes per second for the process",
            #[cfg(target_os = "linux")]
            ProcessField::IORate => "Total I/O rate in bytes per second",
            #[cfg(target_os = "linux")]
            ProcessField::CGroup => "Which cgroup the process is in",
            #[cfg(target_os = "linux")]
            ProcessField::Oom => "OOM (Out-of-Memory) killer score",
            #[cfg(target_os = "linux")]
            ProcessField::IOPriority => "I/O priority",
            #[cfg(target_os = "linux")]
            ProcessField::PercentCpuDelay => "CPU delay %",
            #[cfg(target_os = "linux")]
            ProcessField::PercentIODelay => "Block I/O delay %",
            #[cfg(target_os = "linux")]
            ProcessField::PercentSwapDelay => "Swapin delay %",
            #[cfg(target_os = "linux")]
            ProcessField::MPss => "proportional set size, same as M_RESIDENT but each page is divided by the number of processes sharing it",
            #[cfg(target_os = "linux")]
            ProcessField::MSwap => "Size of the process's swapped pages",
            #[cfg(target_os = "linux")]
            ProcessField::MPsswp => "shows proportional swap share of this mapping, unlike \"Swap\", this does not take into account swapped out page of underlying shmem objects",
            #[cfg(target_os = "linux")]
            ProcessField::Ctxt => "Context switches (incremental sum of voluntary_ctxt_switches and nonvoluntary_ctxt_switches)",
            #[cfg(target_os = "linux")]
            ProcessField::SecAttr => "Security attribute of the process (e.g. SELinux or AppArmor)",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupId => "The autogroup identifier of the process",
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupNice => "Nice value (the higher the value, the more other processes take priority) associated with the process autogroup",
            #[cfg(target_os = "linux")]
            ProcessField::CCGroup => "Which cgroup the process is in (condensed to essentials)",
            #[cfg(target_os = "linux")]
            ProcessField::Container => "Name of the container the process is in (guessed by heuristics)",
            #[cfg(target_os = "linux")]
            ProcessField::MPriv => "The private memory size of the process - resident set size minus shared memory",
            #[cfg(target_os = "linux")]
            ProcessField::GpuTime => "Total GPU time",
            #[cfg(target_os = "linux")]
            ProcessField::GpuPercent => "Percentage of the GPU time the process used in the last sampling",
            #[cfg(target_os = "linux")]
            ProcessField::IsContainer => "Whether the process is running inside a child container",

            // macOS-specific fields
            #[cfg(target_os = "macos")]
            ProcessField::Translated => "Translation info (T translated, N native)",
        }
    }

    /// Create a process field from a name string
    pub fn from_name(name: &str) -> Option<ProcessField> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            // Common fields
            "PID" => Some(ProcessField::Pid),
            "COMMAND" => Some(ProcessField::Command),
            "S" | "STATE" => Some(ProcessField::State),
            "PPID" => Some(ProcessField::Ppid),
            "PGRP" => Some(ProcessField::Pgrp),
            "SID" | "SESSION" => Some(ProcessField::Session),
            "TTY" => Some(ProcessField::Tty),
            "TPGID" => Some(ProcessField::Tpgid),
            "MINFLT" => Some(ProcessField::Minflt),
            "MAJFLT" => Some(ProcessField::Majflt),
            "PRI" | "PRIORITY" => Some(ProcessField::Priority),
            "NI" | "NICE" => Some(ProcessField::Nice),
            "START" | "STARTTIME" => Some(ProcessField::Starttime),
            "CPU" | "PROCESSOR" => Some(ProcessField::Processor),
            "VIRT" | "M_VIRT" => Some(ProcessField::MSize),
            "RES" | "M_RESIDENT" => Some(ProcessField::MResident),
            "UID" | "ST_UID" => Some(ProcessField::StUid),
            "CPU%" | "PERCENT_CPU" => Some(ProcessField::PercentCpu),
            "MEM%" | "PERCENT_MEM" => Some(ProcessField::PercentMem),
            "USER" => Some(ProcessField::User),
            "TIME" | "TIME+" => Some(ProcessField::Time),
            "NLWP" => Some(ProcessField::Nlwp),
            "TGID" => Some(ProcessField::Tgid),
            "NCPU%" | "PERCENT_NORM_CPU" => Some(ProcessField::PercentNormCpu),
            "ELAPSED" => Some(ProcessField::Elapsed),
            "SCHED" | "SCHEDULERPOLICY" => Some(ProcessField::SchedulerPolicy),
            "COMM" => Some(ProcessField::ProcComm),
            "EXE" => Some(ProcessField::ProcExe),
            "CWD" => Some(ProcessField::Cwd),

            // Linux-specific fields
            #[cfg(target_os = "linux")]
            "CMINFLT" => Some(ProcessField::Cminflt),
            #[cfg(target_os = "linux")]
            "CMAJFLT" => Some(ProcessField::Cmajflt),
            #[cfg(target_os = "linux")]
            "UTIME" => Some(ProcessField::Utime),
            #[cfg(target_os = "linux")]
            "STIME" => Some(ProcessField::Stime),
            #[cfg(target_os = "linux")]
            "CUTIME" => Some(ProcessField::Cutime),
            #[cfg(target_os = "linux")]
            "CSTIME" => Some(ProcessField::Cstime),
            #[cfg(target_os = "linux")]
            "SHR" | "M_SHARE" => Some(ProcessField::MShare),
            #[cfg(target_os = "linux")]
            "CODE" | "M_TRS" => Some(ProcessField::MText),
            #[cfg(target_os = "linux")]
            "DATA" | "M_DRS" => Some(ProcessField::MData),
            #[cfg(target_os = "linux")]
            "LIB" | "M_LRS" => Some(ProcessField::MLib),
            #[cfg(target_os = "linux")]
            "RCHAR" => Some(ProcessField::Rchar),
            #[cfg(target_os = "linux")]
            "WCHAR" => Some(ProcessField::Wchar),
            #[cfg(target_os = "linux")]
            "SYSCR" | "READ_SYSC" => Some(ProcessField::Syscr),
            #[cfg(target_os = "linux")]
            "SYSCW" | "WRITE_SYSC" => Some(ProcessField::Syscw),
            #[cfg(target_os = "linux")]
            "RBYTES" | "IO_R" => Some(ProcessField::Rbytes),
            #[cfg(target_os = "linux")]
            "WBYTES" | "IO_W" => Some(ProcessField::Wbytes),
            #[cfg(target_os = "linux")]
            "CNCLWB" | "IO_C" => Some(ProcessField::Cnclwb),
            #[cfg(target_os = "linux")]
            "IO_READ_RATE" | "DISK_READ" => Some(ProcessField::IOReadRate),
            #[cfg(target_os = "linux")]
            "IO_WRITE_RATE" | "DISK_WRITE" => Some(ProcessField::IOWriteRate),
            #[cfg(target_os = "linux")]
            "IO_RATE" | "DISK_R/W" => Some(ProcessField::IORate),
            #[cfg(target_os = "linux")]
            "CGROUP" => Some(ProcessField::CGroup),
            #[cfg(target_os = "linux")]
            "OOM" => Some(ProcessField::Oom),
            #[cfg(target_os = "linux")]
            "IO" | "IO_PRIORITY" => Some(ProcessField::IOPriority),
            #[cfg(target_os = "linux")]
            "CPUD%" | "PERCENT_CPU_DELAY" => Some(ProcessField::PercentCpuDelay),
            #[cfg(target_os = "linux")]
            "IOD%" | "PERCENT_IO_DELAY" => Some(ProcessField::PercentIODelay),
            #[cfg(target_os = "linux")]
            "SWPD%" | "PERCENT_SWAP_DELAY" => Some(ProcessField::PercentSwapDelay),
            #[cfg(target_os = "linux")]
            "PSS" | "M_PSS" => Some(ProcessField::MPss),
            #[cfg(target_os = "linux")]
            "SWAP" | "M_SWAP" => Some(ProcessField::MSwap),
            #[cfg(target_os = "linux")]
            "PSSWP" | "M_PSSWP" => Some(ProcessField::MPsswp),
            #[cfg(target_os = "linux")]
            "CTXT" => Some(ProcessField::Ctxt),
            #[cfg(target_os = "linux")]
            "SECATTR" => Some(ProcessField::SecAttr),
            #[cfg(target_os = "linux")]
            "AGRP" | "AUTOGROUP_ID" => Some(ProcessField::AutogroupId),
            #[cfg(target_os = "linux")]
            "ANI" | "AUTOGROUP_NICE" => Some(ProcessField::AutogroupNice),
            #[cfg(target_os = "linux")]
            "CCGROUP" => Some(ProcessField::CCGroup),
            #[cfg(target_os = "linux")]
            "CONTAINER" => Some(ProcessField::Container),
            #[cfg(target_os = "linux")]
            "PRIV" | "M_PRIV" => Some(ProcessField::MPriv),
            #[cfg(target_os = "linux")]
            "GPU_TIME" => Some(ProcessField::GpuTime),
            #[cfg(target_os = "linux")]
            "GPU%" | "GPU_PERCENT" => Some(ProcessField::GpuPercent),
            #[cfg(target_os = "linux")]
            "CONT" | "ISCONTAINER" => Some(ProcessField::IsContainer),

            // macOS-specific fields
            #[cfg(target_os = "macos")]
            "T" | "TRANSLATED" => Some(ProcessField::Translated),

            _ => None,
        }
    }

    /// Should this field sort in descending order by default?
    pub fn default_sort_desc(self) -> bool {
        match self {
            // Common fields that sort descending
            ProcessField::PercentCpu
            | ProcessField::PercentMem
            | ProcessField::MSize
            | ProcessField::MResident
            | ProcessField::Time
            | ProcessField::Minflt
            | ProcessField::Majflt
            | ProcessField::Nlwp
            | ProcessField::PercentNormCpu => true,

            // Linux-specific fields that sort descending
            #[cfg(target_os = "linux")]
            ProcessField::Cminflt
            | ProcessField::Cmajflt
            | ProcessField::Utime
            | ProcessField::Stime
            | ProcessField::Cutime
            | ProcessField::Cstime
            | ProcessField::MShare
            | ProcessField::MText
            | ProcessField::MData
            | ProcessField::MLib
            | ProcessField::Rchar
            | ProcessField::Wchar
            | ProcessField::Syscr
            | ProcessField::Syscw
            | ProcessField::Rbytes
            | ProcessField::Wbytes
            | ProcessField::Cnclwb
            | ProcessField::IOReadRate
            | ProcessField::IOWriteRate
            | ProcessField::IORate
            | ProcessField::Oom
            | ProcessField::PercentCpuDelay
            | ProcessField::PercentIODelay
            | ProcessField::PercentSwapDelay
            | ProcessField::MPss
            | ProcessField::MSwap
            | ProcessField::MPsswp
            | ProcessField::Ctxt
            | ProcessField::MPriv
            | ProcessField::GpuTime
            | ProcessField::GpuPercent => true,

            _ => false,
        }
    }

    /// Check if this field is a PID column (should use PID width formatting)
    pub fn is_pid_column(self) -> bool {
        matches!(
            self,
            ProcessField::Pid
                | ProcessField::Ppid
                | ProcessField::Pgrp
                | ProcessField::Session
                | ProcessField::Tpgid
                | ProcessField::Tgid
        )
    }
}

/// Command line highlight flags - matches C htop's CMDLINE_HIGHLIGHT_FLAG_* constants
pub mod highlight_flags {
    pub const SEPARATOR: u32 = 0x00000001;
    pub const BASENAME: u32 = 0x00000002;
    pub const COMM: u32 = 0x00000004;
    pub const DELETED: u32 = 0x00000008;
    pub const PREFIXDIR: u32 = 0x00000010;
}

/// Command line highlight information
#[derive(Debug, Clone, Default)]
pub struct CmdlineHighlight {
    pub offset: usize,
    pub length: usize,
    pub attr: u32,
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

    // Linux-specific time fields (in hundredths of a second)
    // These are only populated on Linux
    pub utime: u64,   // User CPU time
    pub stime: u64,   // System CPU time
    pub cutime: u64,  // Children's user CPU time
    pub cstime: u64,  // Children's system CPU time

    // Linux-specific children's page fault counters
    pub cminflt: u64, // Children's minor page faults
    pub cmajflt: u64, // Children's major page faults

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
            utime: 0,
            stime: 0,
            cutime: 0,
            cstime: 0,
            cminflt: 0,
            cmajflt: 0,
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
            #[cfg(target_os = "linux")]
            ProcessField::MShare => self.m_share.cmp(&other.m_share),
            ProcessField::Minflt => self.minflt.cmp(&other.minflt),
            ProcessField::Majflt => self.majflt.cmp(&other.majflt),
            ProcessField::Nlwp => self.nlwp.cmp(&other.nlwp),
            ProcessField::Starttime => self.starttime_ctime.cmp(&other.starttime_ctime),
            ProcessField::Command | ProcessField::ProcComm => {
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
                } else {
                    format!("{:>4.0}", self.percent_cpu)
                }
            }
            ProcessField::PercentMem => format!("{:>4.1}", self.percent_mem),
            ProcessField::MSize => Self::format_memory(self.m_virt),
            ProcessField::MResident => Self::format_memory(self.m_resident),
            #[cfg(target_os = "linux")]
            ProcessField::MShare => Self::format_memory(self.m_share),
            ProcessField::Time => Self::format_time(self.time),
            ProcessField::Command => self.get_command().to_string(),
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

    /// Build the merged command string and highlights
    /// This matches C htop's Process_makeCommandStr() function
    ///
    /// The merged command format depends on settings:
    /// - When showMergedCommand is true and we have exe: exe│comm│cmdline (with separators)
    /// - When showMergedCommand is false: just cmdline with optional comm prefix
    pub fn make_command_str(&mut self, params: &CommandStrParams, tree_separator: &str) {
        // Skip kernel threads
        if self.is_kernel_thread {
            return;
        }
        // Skip zombies that we haven't seen before
        if self.state == ProcessState::Zombie && self.merged_command.str_value.is_none() {
            return;
        }

        // Select thread-specific colors if this is a thread (matches C htop behavior)
        let base_attr = if self.is_thread() {
            params.thread_base_attr
        } else {
            params.base_attr
        };
        let comm_attr = if self.is_thread() {
            params.thread_comm_attr
        } else {
            params.comm_attr
        };

        // Reset highlights
        self.merged_command.highlights.clear();

        let cmdline = self.cmdline.as_deref().unwrap_or("(zombie)");
        let proc_comm = self.comm.as_deref();
        let proc_exe = self.exe.as_deref();

        let cmdline_basename_start = self.cmdline_basename_start;
        let cmdline_basename_len = if self.cmdline_basename_end > self.cmdline_basename_start {
            self.cmdline_basename_end - self.cmdline_basename_start
        } else {
            0
        };

        let exe_basename_offset = self.exe_basename_offset;
        let exe_basename_len = proc_exe.map(|e| e.len() - exe_basename_offset).unwrap_or(0);

        // Calculate match length for stripping exe from cmdline
        let mut match_len = if let (Some(exe), Some(cl)) = (proc_exe, self.cmdline.as_deref()) {
            Self::match_cmdline_prefix_with_exe_suffix(
                cl,
                cmdline_basename_start,
                exe,
                exe_basename_offset,
                exe_basename_len,
            )
        } else {
            0
        };

        // Build the command string
        let mut result = String::with_capacity(
            cmdline.len()
                + proc_comm.map(|c| c.len()).unwrap_or(0)
                + proc_exe.map(|e| e.len()).unwrap_or(0)
                + 2 * tree_separator.len()
                + 1,
        );

        // Track multi-byte character offset mismatch (separator is multi-byte but counts as 1 char for highlighting)
        let separator_len = tree_separator.len();
        let mut mb_mismatch: usize = 0;

        // Macro-like closure to add a highlight
        let add_highlight =
            |highlights: &mut Vec<CmdlineHighlight>,
             str_pos: usize,
             offset: usize,
             length: usize,
             attr: u32,
             flags: u32,
             mb_mismatch: usize| {
                highlights.push(CmdlineHighlight {
                    offset: str_pos + offset - mb_mismatch,
                    length,
                    attr,
                    flags,
                });
            };

        // Case 1: Fallback to cmdline (no merged command or missing exe/comm)
        if !params.show_merged_command || proc_exe.is_none() || proc_comm.is_none() {
            // Check if we should show comm as prefix
            if (params.show_merged_command
                || (self.is_userland_thread && params.show_thread_names))
                && proc_comm.is_some()
            {
                let comm = proc_comm.unwrap();
                if !comm.is_empty() {
                    let cmdline_base = &cmdline[cmdline_basename_start..];
                    let cmp_len = comm.len().min(TASK_COMM_LEN - 1);
                    if !cmdline_base.starts_with(&comm[..cmp_len]) {
                        // comm differs from cmdline basename - show comm prefix
                        add_highlight(
                            &mut self.merged_command.highlights,
                            result.len(),
                            0,
                            comm.len(),
                            comm_attr,
                            highlight_flags::COMM,
                            mb_mismatch,
                        );
                        result.push_str(comm);

                        if !params.show_merged_command {
                            // Not showing merged command, just comm prefix
                            self.merged_command.str_value = Some(result);
                            return;
                        }

                        // Add separator
                        add_highlight(
                            &mut self.merged_command.highlights,
                            result.len(),
                            0,
                            1,
                            params.separator_attr,
                            highlight_flags::SEPARATOR,
                            mb_mismatch,
                        );
                        mb_mismatch += separator_len - 1;
                        result.push_str(tree_separator);
                    }
                }
            }

            // Add dist path prefix shadow if enabled
            if params.shadow_dist_path_prefix && params.show_program_path {
                if let Some(prefix_len) = Self::get_dist_path_prefix_len(cmdline) {
                    add_highlight(
                        &mut self.merged_command.highlights,
                        result.len(),
                        0,
                        prefix_len,
                        params.shadow_attr,
                        highlight_flags::PREFIXDIR,
                        mb_mismatch,
                    );
                }
            }

            // Add basename highlight
            if cmdline_basename_len > 0 {
                let hl_offset = if params.show_program_path {
                    cmdline_basename_start
                } else {
                    0
                };
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    hl_offset,
                    cmdline_basename_len,
                    base_attr,
                    highlight_flags::BASENAME,
                    mb_mismatch,
                );

                // Add deleted exe highlight
                if self.exe_deleted {
                    add_highlight(
                        &mut self.merged_command.highlights,
                        result.len(),
                        hl_offset,
                        cmdline_basename_len,
                        params.del_exe_attr,
                        highlight_flags::DELETED,
                        mb_mismatch,
                    );
                } else if self.uses_deleted_lib {
                    add_highlight(
                        &mut self.merged_command.highlights,
                        result.len(),
                        hl_offset,
                        cmdline_basename_len,
                        params.del_lib_attr,
                        highlight_flags::DELETED,
                        mb_mismatch,
                    );
                }
            }

            // Append cmdline (from basename start if not showing path)
            let cmdline_to_append = if params.show_program_path {
                cmdline
            } else {
                &cmdline[cmdline_basename_start.min(cmdline.len())..]
            };
            // Convert newlines to spaces
            for c in cmdline_to_append.chars() {
                result.push(if c == '\n' { ' ' } else { c });
            }

            self.merged_command.str_value = Some(result);
            return;
        }

        // Case 2: Full merged command with exe
        let exe = proc_exe.unwrap();
        let comm = proc_comm.unwrap();

        // Check if comm is in exe basename
        let mut have_comm_in_exe = false;
        if !self.is_userland_thread || params.show_thread_names {
            let exe_basename = &exe[exe_basename_offset..];
            let cmp_len = comm.len().min(TASK_COMM_LEN - 1);
            have_comm_in_exe = exe_basename.starts_with(&comm[..cmp_len]);
        }

        let comm_len = if have_comm_in_exe { exe_basename_len } else { 0 };

        // Check if comm is in cmdline
        let mut have_comm_in_cmdline = false;
        let mut comm_start: usize = 0;
        let mut comm_len_cmdline: usize = 0;

        if !have_comm_in_exe
            && self.cmdline.is_some()
            && params.find_comm_in_cmdline
            && (!self.is_userland_thread || params.show_thread_names)
        {
            if let Some((start, len)) =
                Self::find_comm_in_cmdline(comm, cmdline, cmdline_basename_start)
            {
                have_comm_in_cmdline = true;
                comm_start = start;
                comm_len_cmdline = len;
            }
        }

        // Strip exe from cmdline if enabled
        if !params.strip_exe_from_cmdline {
            match_len = 0;
        }

        let cmdline_remainder = if match_len > 0 {
            // Adjust comm_start if we're stripping
            if have_comm_in_cmdline {
                if comm_start == cmdline_basename_start {
                    have_comm_in_exe = true;
                    have_comm_in_cmdline = false;
                } else if comm_start >= match_len {
                    comm_start -= match_len;
                }
            }
            &cmdline[match_len..]
        } else {
            cmdline
        };

        // Start with copying exe
        if params.show_program_path {
            // Add dist path prefix shadow
            if params.shadow_dist_path_prefix {
                if let Some(prefix_len) = Self::get_dist_path_prefix_len(exe) {
                    add_highlight(
                        &mut self.merged_command.highlights,
                        result.len(),
                        0,
                        prefix_len,
                        params.shadow_attr,
                        highlight_flags::PREFIXDIR,
                        mb_mismatch,
                    );
                }
            }
            // Comm highlight in exe
            if have_comm_in_exe {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    exe_basename_offset,
                    comm_len,
                    comm_attr,
                    highlight_flags::COMM,
                    mb_mismatch,
                );
            }
            // Basename highlight
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                exe_basename_offset,
                exe_basename_len,
                base_attr,
                highlight_flags::BASENAME,
                mb_mismatch,
            );
            // Deleted highlight
            if self.exe_deleted {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    exe_basename_offset,
                    exe_basename_len,
                    params.del_exe_attr,
                    highlight_flags::DELETED,
                    mb_mismatch,
                );
            } else if self.uses_deleted_lib {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    exe_basename_offset,
                    exe_basename_len,
                    params.del_lib_attr,
                    highlight_flags::DELETED,
                    mb_mismatch,
                );
            }
            result.push_str(exe);
        } else {
            // Just basename
            let exe_basename = &exe[exe_basename_offset..];
            if have_comm_in_exe {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    0,
                    comm_len,
                    comm_attr,
                    highlight_flags::COMM,
                    mb_mismatch,
                );
            }
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                0,
                exe_basename_len,
                base_attr,
                highlight_flags::BASENAME,
                mb_mismatch,
            );
            if self.exe_deleted {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    0,
                    exe_basename_len,
                    params.del_exe_attr,
                    highlight_flags::DELETED,
                    mb_mismatch,
                );
            } else if self.uses_deleted_lib {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    0,
                    exe_basename_len,
                    params.del_lib_attr,
                    highlight_flags::DELETED,
                    mb_mismatch,
                );
            }
            result.push_str(exe_basename);
        }

        // Add comm as separate field if not found in exe or cmdline
        let mut have_comm_field = false;
        if !have_comm_in_exe
            && !have_comm_in_cmdline
            && (!self.is_userland_thread || params.show_thread_names)
        {
            // Add separator
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                0,
                1,
                params.separator_attr,
                highlight_flags::SEPARATOR,
                mb_mismatch,
            );
            mb_mismatch += separator_len - 1;
            result.push_str(tree_separator);

            // Add comm highlight
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                0,
                comm.len(),
                comm_attr,
                highlight_flags::COMM,
                mb_mismatch,
            );
            result.push_str(comm);
            have_comm_field = true;
        }

        // Add separator before cmdline if needed
        if match_len == 0 || (have_comm_field && !cmdline_remainder.is_empty()) {
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                0,
                1,
                params.separator_attr,
                highlight_flags::SEPARATOR,
                mb_mismatch,
            );
            mb_mismatch += separator_len - 1;
            result.push_str(tree_separator);
        }

        // Add dist path prefix shadow for cmdline
        if params.shadow_dist_path_prefix {
            if let Some(prefix_len) = Self::get_dist_path_prefix_len(cmdline_remainder) {
                add_highlight(
                    &mut self.merged_command.highlights,
                    result.len(),
                    0,
                    prefix_len,
                    params.shadow_attr,
                    highlight_flags::PREFIXDIR,
                    mb_mismatch,
                );
            }
        }

        // Add comm highlight in cmdline if found there
        if !have_comm_in_exe
            && have_comm_in_cmdline
            && !have_comm_field
            && (!self.is_userland_thread || params.show_thread_names)
        {
            add_highlight(
                &mut self.merged_command.highlights,
                result.len(),
                comm_start,
                comm_len_cmdline,
                comm_attr,
                highlight_flags::COMM,
                mb_mismatch,
            );
        }

        // Append remaining cmdline
        if !cmdline_remainder.is_empty() {
            for c in cmdline_remainder.chars() {
                result.push(if c == '\n' { ' ' } else { c });
            }
        }

        self.merged_command.str_value = Some(result);
    }

    /// Match cmdline prefix with exe suffix to determine how much to strip
    /// Returns the number of bytes to strip from cmdline if they match, 0 otherwise
    fn match_cmdline_prefix_with_exe_suffix(
        cmdline: &str,
        cmdline_basename_start: usize,
        exe: &str,
        exe_base_offset: usize,
        exe_base_len: usize,
    ) -> usize {
        if cmdline.is_empty() || exe.is_empty() {
            return 0;
        }

        // Case 1: cmdline prefix is an absolute path - must match whole exe
        if cmdline.starts_with('/') {
            let match_len = exe_base_len + exe_base_offset;
            if cmdline.len() >= match_len && cmdline.starts_with(exe) {
                let delim = cmdline.chars().nth(match_len).unwrap_or('\0');
                if delim == '\0' || delim == '\n' || delim == ' ' {
                    return match_len;
                }
            }
            return 0;
        }

        // Case 2: cmdline prefix is a relative path
        let exe_basename = &exe[exe_base_offset..];
        let mut cmdline_base_offset = cmdline_basename_start;
        let mut delim_found = true;

        while delim_found {
            // Match basename
            let match_len = exe_base_len + cmdline_base_offset;
            if cmdline_base_offset < exe_base_offset
                && cmdline.len() >= cmdline_base_offset + exe_base_len
            {
                let cmdline_segment =
                    &cmdline[cmdline_base_offset..cmdline_base_offset + exe_base_len];
                if cmdline_segment == exe_basename {
                    let delim = cmdline.chars().nth(match_len).unwrap_or('\0');
                    if delim == '\0' || delim == '\n' || delim == ' ' {
                        // Reverse match the cmdline prefix with exe suffix
                        let mut i = cmdline_base_offset;
                        let mut j = exe_base_offset;
                        let cmdline_bytes = cmdline.as_bytes();
                        let exe_bytes = exe.as_bytes();

                        while i >= 1 && j >= 1 && cmdline_bytes[i - 1] == exe_bytes[j - 1] {
                            i -= 1;
                            j -= 1;
                        }

                        // Full match with exe suffix being a valid relative path
                        if i < 1 && j >= 1 && exe_bytes[j - 1] == b'/' {
                            return match_len;
                        }
                    }
                }
            }

            // Try to find previous potential cmdlineBaseOffset
            delim_found = false;
            if cmdline_base_offset <= 2 {
                return 0;
            }

            let cmdline_bytes = cmdline.as_bytes();
            cmdline_base_offset -= 2;
            while cmdline_base_offset > 0 {
                if delim_found {
                    if cmdline_bytes[cmdline_base_offset - 1] == b'/' {
                        break;
                    }
                } else if cmdline_bytes[cmdline_base_offset] == b' '
                    || cmdline_bytes[cmdline_base_offset] == b'\n'
                {
                    delim_found = true;
                }
                cmdline_base_offset -= 1;
            }
        }

        0
    }

    /// Try to find comm within cmdline arguments
    /// Returns Some((start_offset, length)) if found, None otherwise
    fn find_comm_in_cmdline(
        comm: &str,
        cmdline: &str,
        cmdline_basename_start: usize,
    ) -> Option<(usize, usize)> {
        let comm_len = comm.len();
        if comm_len == 0 || cmdline_basename_start >= cmdline.len() {
            return None;
        }

        let search_area = &cmdline[cmdline_basename_start..];
        let mut pos = cmdline_basename_start;

        // Iterate through tokens (space-separated, treating newlines as separators too)
        for token in search_area.split(|c| c == ' ' || c == '\n') {
            if token.is_empty() {
                pos += 1; // Account for the separator
                continue;
            }

            // Find basename of this token
            let token_basename = if let Some(slash_pos) = token.rfind('/') {
                &token[slash_pos + 1..]
            } else {
                token
            };

            let token_len = token_basename.len();
            let basename_offset = token.len() - token_basename.len();

            // Check if this token matches comm
            let matches = if token_len == comm_len {
                token_basename == comm
            } else if token_len > comm_len && comm_len == TASK_COMM_LEN - 1 {
                token_basename.starts_with(comm)
            } else {
                false
            };

            if matches {
                return Some((pos + basename_offset, token_len));
            }

            pos += token.len() + 1; // +1 for separator
        }

        None
    }

    /// Get distribution path prefix length for shadowing
    /// Returns the length of common distribution paths like /usr/bin/, /lib/, etc.
    fn get_dist_path_prefix_len(path: &str) -> Option<usize> {
        if !path.starts_with('/') {
            return None;
        }

        const PREFIXES: &[&str] = &[
            "/bin/",
            "/lib/",
            "/lib32/",
            "/lib64/",
            "/libx32/",
            "/sbin/",
            "/usr/bin/",
            "/usr/libexec/",
            "/usr/lib/",
            "/usr/lib32/",
            "/usr/lib64/",
            "/usr/libx32/",
            "/usr/local/bin/",
            "/usr/local/lib/",
            "/usr/local/sbin/",
            "/usr/sbin/",
            "/run/current-system/",
        ];

        for prefix in PREFIXES {
            if path.starts_with(prefix) {
                return Some(prefix.len());
            }
        }

        // Special case for NixOS store paths
        if let Some(rest) = path.strip_prefix("/nix/store/") {
            if let Some(pos) = rest.find('/') {
                return Some("/nix/store/".len() + pos + 1);
            }
        }

        None
    }
}

/// Linux TASK_COMM_LEN is 16, so comm is truncated to 15 chars + NUL
const TASK_COMM_LEN: usize = 16;

/// Parameters for building the command string
/// This avoids circular dependencies with Settings
#[derive(Debug, Clone)]
pub struct CommandStrParams {
    pub show_merged_command: bool,
    pub show_program_path: bool,
    pub find_comm_in_cmdline: bool,
    pub strip_exe_from_cmdline: bool,
    pub show_thread_names: bool,
    pub shadow_dist_path_prefix: bool,
    pub base_attr: u32,
    pub comm_attr: u32,
    pub thread_base_attr: u32,
    pub thread_comm_attr: u32,
    pub del_exe_attr: u32,
    pub del_lib_attr: u32,
    pub separator_attr: u32,
    pub shadow_attr: u32,
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
    /// Also builds command strings for each process if params are provided
    pub fn cleanup(&mut self, cmd_params: Option<&CommandStrParams>, tree_separator: &str) {
        self.processes.retain(|p| p.updated);
        // Rebuild the existence set
        self.by_pid.clear();
        for process in self.processes.iter() {
            self.by_pid.insert(process.pid);
        }
        // Build command strings and mark as not updated for next scan
        for process in &mut self.processes {
            if let Some(params) = cmd_params {
                process.make_command_str(params, tree_separator);
            }
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
    /// sort_key: field to sort sibling processes by
    /// ascending: if true, sort ascending; if false, sort descending
    pub fn build_tree(&mut self, sort_key: ProcessField, ascending: bool) {
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

        // Sort by parent, then by sort_key (roots first)
        self.processes.sort_by(|a, b| {
            let a_parent = if a.is_root { 0 } else { a.ppid };
            let b_parent = if b.is_root { 0 } else { b.ppid };
            match a_parent.cmp(&b_parent) {
                std::cmp::Ordering::Equal => {
                    let cmp = a.compare_by_field(b, sort_key);
                    if ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                }
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
            self.build_tree_branch(
                pid,
                0,
                0,
                show_children,
                sort_key,
                ascending,
                &mut display_list,
            );
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
                    self.build_tree_branch(
                        *pid,
                        1,
                        child_indent,
                        child_show,
                        sort_key,
                        ascending,
                        &mut display_list,
                    );
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
        sort_key: ProcessField,
        ascending: bool,
        display_list: &mut Vec<i32>,
    ) {
        // Find all children of this parent (matching C htop Row_isChildOf check)
        let mut children: Vec<(usize, i32)> = self
            .processes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.ppid == parent_id && p.pid != parent_id)
            .map(|(i, p)| (i, p.pid))
            .collect();

        if children.is_empty() {
            return;
        }

        // Sort children by the sort key
        // Since we're sorting children which are already in self.processes,
        // we need to compare by looking up the processes by index
        children.sort_by(|(idx_a, _), (idx_b, _)| {
            let a = &self.processes[*idx_a];
            let b = &self.processes[*idx_b];
            let cmp = a.compare_by_field(b, sort_key);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

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
            self.build_tree_branch(
                *pid,
                level + 1,
                child_indent,
                child_show,
                sort_key,
                ascending,
                display_list,
            );

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

        pl.build_tree(ProcessField::Pid, true);

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

        pl.build_tree(ProcessField::Pid, true);

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

        pl.build_tree(ProcessField::Pid, true);

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
