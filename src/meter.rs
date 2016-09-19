use std::io;
use std::fs;
use std::time::Duration;
use std::collections::{VecDeque, HashMap};

use {Meter, Error};


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
            num_cpus: try!(num_cpus().map_err(Error::Cpu)),
            num_snapshots: 10,
            snapshots: VecDeque::with_capacity(10),
            thread_names: HashMap::new(),
        })
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
