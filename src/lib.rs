#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
//! A tiny library to measure resource usage of the process it's used in.
//! Currently it measures:
//!
//! * Memory Usage
//! * CPU Usage with breakdown by each thread
//! * Disk Usage
//!
//! More metrics might be added later. Currently, library supports only linux,
//! but pull requests for other platforms are welcome.
//!
//! # Example
//!
//! ```rust,no_run
//!
//! # use std::io::{Write, stderr};
//! # use std::time::Duration;
//! # use std::thread::sleep;
//! # use std::collections::BTreeMap;
//!
//! fn main() {
//!    let mut meter = self_meter::Meter::new(Duration::new(1, 0)).unwrap();
//!    meter.track_current_thread("main");
//!    loop {
//!        meter.scan()
//!            .map_err(|e| writeln!(&mut stderr(), "Scan error: {}", e)).ok();
//!        println!("Report: {:#?}", meter.report());
//!        println!("Threads: {:#?}",
//!            meter.thread_report().map(|x| x.collect::<BTreeMap<_,_>>()));
//!        // Put your task here
//!        // ...
//!        //
//!        sleep(Duration::new(1, 0));
//!    }
//! }
//! ```
extern crate libc;
extern crate num_cpus;
extern crate serde;

#[macro_use] extern crate quick_error;
#[macro_use] extern crate serde_derive;

use std::fs::File;
use std::time::{SystemTime, Instant, Duration};
use std::collections::{VecDeque, HashMap};

mod meter;
mod scan;
mod error;
mod report;
mod serialize;
mod debug;

pub use error::Error;
pub use report::ThreadReportIter;
/// A Pid type used to identify processes and threads
pub type Pid = u32;

struct ThreadInfo {
    user_time: u64,
    system_time: u64,
    child_user_time: u64,
    child_system_time: u64,
}

struct Snapshot {
    timestamp: SystemTime,
    instant: Instant,
    /// System uptime in centisecs
    uptime: u64,
    /// System idle time in centisecs
    idle_time: u64,
    process: ThreadInfo,
    memory_rss: u64,
    memory_virtual: u64,
    memory_virtual_peak: u64,
    memory_swap: u64,
    read_bytes: u64,
    write_bytes: u64,
    read_ops: u64,
    write_ops: u64,
    read_disk_bytes: u64,
    write_disk_bytes: u64,
    write_cancelled_bytes: u64,
    threads: HashMap<Pid, ThreadInfo>,
}


/// CPU usage of a single thread
#[derive(Debug)]
pub struct ThreadUsage {
    /// Thread's own CPU usage. 100% is a single core
    pub cpu_usage: f32,
    /// Thread's CPU usage with its awaited children. 100% is a single core
    pub cpu_usage_with_children: f32,
}

/// Report returned by `Meter::report`
///
/// Note: this structure implements `serde::Serialize`, and all timestamps and
/// durations are stored as integers in milliseconds.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Timestamp
    #[serde(serialize_with="serialize::serialize_timestamp")]
    pub timestamp: SystemTime,

    /// The interval time this data has averaged over in milliseconds
    #[serde(serialize_with="serialize::serialize_duration")]
    pub duration: Duration,

    /// Start time
    #[serde(serialize_with="serialize::serialize_timestamp")]
    pub start_time: SystemTime,

    /// The uptime of the system
    ///
    /// Note this value can be smaller than time since `start_time`
    /// because this value doesn't include time when system was sleeping
    #[serde(serialize_with="serialize::serialize_duration")]
    pub system_uptime: Duration,
    /// Whole system CPU usage. 100% is all cores
    pub global_cpu_usage: f32,
    /// Process' own CPU usage. 100% is a single core
    pub process_cpu_usage: f32,
    /// Process' CPU usage with its awaited children. 100% is a single core
    pub gross_cpu_usage: f32,
    /// Process' memory usage
    pub memory_rss: u64,
    /// Process' virtual memory usage
    pub memory_virtual: u64,
    /// Process' swap usage
    pub memory_swap: u64,
    /// Process' peak memory usage (not precise)
    pub memory_rss_peak: u64,
    /// Process' peak virtual memory usage (tracked by OS)
    pub memory_virtual_peak: u64,
    /// Process' swap usage (not precise)
    pub memory_swap_peak: u64,
    /// Bytes read per second from block-backed filesystems
    pub disk_read: f32,
    /// Bytes written per second from block-backed filesystems
    pub disk_write: f32,
    /// Bytes per second of cancelled writes (i.e. removed temporary files)
    pub disk_cancelled: f32,
    /// Bytes read per second (total)
    pub io_read: f32,
    /// Bytes written per second (total)
    pub io_write: f32,
    /// Read operations (syscalls) per second (total)
    pub io_read_ops: f32,
    /// Write operations (syscalls) per second (total)
    pub io_write_ops: f32,
}

/// Report of CPU usage by single thread
#[derive(Debug, Serialize)]
pub struct ThreadReport {
    /// Threads' own CPU usage. 100% is a single core
    pub cpu_usage: f32,
    /// Threads' own CPU usage in kernel space. 100% is a single core
    pub system_cpu: f32,
    /// Threads' own CPU usage in user space. 100% is a single core
    pub user_cpu: f32,
}

/// The main structure that makes mesurements and reports values
///
/// Create it with `new()` then add threads that you want to track in a thread
/// breakdown information with `meter.track_thread()` and
/// `meter.untrack_thread()`.
///
/// Then add `meter.scan()` with a timer to scan the process info. It's
/// recommended to call it on the interval of one second.
///
/// Method `report()` may be used to get structure with stats. `report_json()`
/// can return a `rustc_serialize::Json` and `report_json_str()` returns that
/// serialized.
///
/// Note that the structure returned with `report()` can be changed when we
/// bump **major version** of the library. And while `report_json()` and
/// `report_json_str()` will never break the type system, their format will
/// always reflect that of `report()` call.
///
/// We don't track all the threads separately because thread ids are useless
/// without names, and we can fine-tune performance in the case we have known
/// number of threads. Obviously, process-wide info accounts all the threads.
pub struct Meter {
    #[allow(dead_code)]
    scan_interval: Duration,
    num_cpus: usize,
    num_snapshots: usize,
    start_time: SystemTime,
    snapshots: VecDeque<Snapshot>,
    thread_names: HashMap<Pid, String>,
    /// This is a buffer for reading some text data from /proc/anything.
    /// We use it to avoid memory allocations. This makes code a little bit
    /// more complex, but we want to avoid overhead as much as possible
    text_buf: String,
    /// This is a smaller buffer for formatting paths, similar to `text_buf`
    path_buf: String,

    /// This file is always open because if we drop privileges and then
    /// try to open a file we can't open it back again
    #[cfg(target_os="linux")]
    io_file: File,

    memory_rss_peak: u64,
    memory_swap_peak: u64,
}
