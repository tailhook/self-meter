use std::fs::File;
use std::time::{Duration, SystemTime};
use std::collections::{VecDeque, HashMap};

use num_cpus;

use {Meter, Error, Pid};
use error::IoStatError;


impl Meter {
    /// Create a new meter with scan_interval
    ///
    /// Note: meter will not scan by itself, you are expected to call `scan()`
    /// with interval.
    ///
    /// You don't have to guarantee the interval exactly, but it influences
    /// the accuracy of your measurements.
    ///
    /// When creating a `Meter` object we are trying to discover the number
    /// of processes on the system. If that fails, we return error.
    pub fn new(scan_interval: Duration) -> Result<Meter, Error> {
        Meter::_new(scan_interval)
    }
    #[cfg(linux)]
    fn _new(scan_interval: Duration) -> Result<Meter, Error> {
        let io_file = File::open("/proc/self/io").map_err(IoStatError::Io)?;
        Ok(Meter {
            scan_interval: scan_interval,
            num_cpus: num_cpus::get(),
            num_snapshots: 10,
            start_time: SystemTime::now(),
            snapshots: VecDeque::with_capacity(10),
            thread_names: HashMap::new(),
            text_buf: String::with_capacity(1024),
            path_buf: String::with_capacity(100),
            io_file: io_file,

            memory_swap_peak: 0,
            memory_rss_peak: 0,
        })
    }

    #[cfg(not(linux))]
    fn _new(scan_interval: Duration) -> Result<Meter, Error> {
        Ok(Meter {
            scan_interval: scan_interval,
            num_cpus: num_cpus::get(),
            num_snapshots: 10,
            start_time: SystemTime::now(),
            snapshots: VecDeque::with_capacity(10),
            thread_names: HashMap::new(),
            text_buf: String::with_capacity(1024),
            path_buf: String::with_capacity(100),

            memory_swap_peak: 0,
            memory_rss_peak: 0,
        })
    }

    /// Start tracking specified thread
    ///
    /// Note you must add main thread here manually
    pub fn track_thread(&mut self, tid: Pid, name: &str) {
        self.thread_names.insert(tid, name.to_string());
    }
    /// Stop tracking specified thread (for example if it's dead)
    pub fn untrack_thread(&mut self, tid: Pid) {
        self.thread_names.remove(&tid);
        for s in &mut self.snapshots {
            s.threads.remove(&tid);
        }
    }
    /// Add current thread using `track_thread`, returns thread id
    #[cfg(target_os="linux")]
    pub fn track_current_thread(&mut self, name: &str) -> Pid {
        use libc::{syscall, SYS_gettid};
        let tid = unsafe { syscall(SYS_gettid) } as Pid;
        self.track_thread(tid, name);
        return tid;
    }
    /// Add current thread using `track_thread`, returns thread id
    ///
    /// Non-linux is not supported yet (no-op)
    #[cfg(not(target_os="linux"))]
    pub fn track_current_thread(&mut self, _name: &str) -> Pid {
        // TODO(tailhook) OS X and windows
        0
    }
    /// Remove current thread using `untrack_thread`
    #[cfg(target_os="linux")]
    pub fn untrack_current_thread(&mut self) {
        use libc::{syscall, SYS_gettid};
        let tid = unsafe { syscall(SYS_gettid) } as Pid;
        self.untrack_thread(tid);
    }
    /// Remove current thread using `untrack_thread`
    ///
    /// Non-linux is not supported yet (no-op)
    #[cfg(not(target_os="linux"))]
    pub fn untrack_current_thread(&mut self) {
        // TODO
    }
    /// Returns interval value configured in constructor
    pub fn get_scan_interval(&self) -> Duration {
        self.scan_interval
    }
}
