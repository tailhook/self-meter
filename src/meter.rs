use std::io;
use std::fs;
use std::time::Duration;
use std::collections::{VecDeque, HashMap};

use libc::{syscall, SYS_gettid};

use {Meter, Error, Pid};


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
        Ok(Meter {
            scan_interval: scan_interval,
            num_cpus: try!(num_cpus().map_err(Error::Cpu)),
            num_snapshots: 10,
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
    pub fn track_current_thread(&mut self, name: &str) -> Pid {
        let tid = unsafe { syscall(SYS_gettid) } as Pid;
        self.track_thread(tid, name);
        return tid;
    }
}


fn is_cpu(name: &str) -> bool {
    name.starts_with("cpu") && name[3..].parse::<u32>().is_ok()
}


fn num_cpus() -> io::Result<u32> {
    let mut cnt = 0;
    for entry in try!(fs::read_dir("/sys/devices/system/cpu")) {
        let entry = try!(entry);
        if entry.file_name().to_str().map(is_cpu).unwrap_or(false) {
            cnt += 1;
        }
    }
    Ok(cnt)
}
