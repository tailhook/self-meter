use std::io::Read;
use std::fs::File;
use std::fmt::Write;
use std::num::ParseIntError;
use std::time::{Instant, SystemTime};
use std::collections::HashMap;

use {Meter, Snapshot, ThreadInfo, Pid, Error};
use error::{UptimeError, StatError, StatusError};


impl Meter {
    pub fn scan(&mut self) -> Result<(), Error> {
        // We reuse Snapshot structure (mostly becasuse of threads hash map)
        // to have smaller allocations on the fast path
        let mut snap = if self.snapshots.len() >= self.num_snapshots {
            self.snapshots.pop_front().unwrap()
        } else {
            Snapshot::new(&self.thread_names)
        };
        snap.timestamp = SystemTime::now();
        snap.instant = Instant::now();

        // First scan everything that relates to cpu_time to have as accurate
        // CPU usage measurements as possible
        try!(self.read_cpu_times(&mut snap.process,
            &mut snap.threads,
            &mut snap.uptime, &mut snap.idle_time));

        try!(self.read_memory(&mut snap));
        try!(self.read_io(&mut snap));

        if snap.memory_rss > self.memory_rss_peak {
            self.memory_rss_peak = snap.memory_rss;
        }
        if snap.memory_swap > self.memory_swap_peak {
            self.memory_swap_peak = snap.memory_swap;
        }

        self.snapshots.push_back(snap);
        Ok(())
    }

    fn read_cpu_times(&mut self, process: &mut ThreadInfo,
                      threads: &mut HashMap<Pid, ThreadInfo>,
                      uptime: &mut u64, idle_time: &mut u64)
        -> Result<(), Error>
    {
        self.text_buf.truncate(0);
        try!(File::open("/proc/uptime")
             .and_then(|mut f| f.read_to_string(&mut self.text_buf))
             .map_err(|e| Error::Uptime(e.into())));
        {
            let mut iter = self.text_buf.split_whitespace();
            let seconds = try!(iter.next()
                .ok_or(Error::Uptime(UptimeError::BadFormat)));
            let idle_sec = try!(iter.next()
                .ok_or(Error::Uptime(UptimeError::BadFormat)));
            *uptime = try!(parse_uptime(seconds));
            *idle_time = try!(parse_uptime(idle_sec));
        }
        try!(read_stat(&mut self.text_buf, "/proc/self/stat", process)
            .map_err(Error::Stat));
        for (&tid, _) in &self.thread_names {
            self.path_buf.truncate(0);
            write!(&mut self.path_buf,
                "/proc/self/task/{}/stat", tid).unwrap();
            try!(read_stat(&mut self.text_buf, &self.path_buf[..],
                    threads.entry(tid).or_insert_with(ThreadInfo::new))
                .map_err(|e| Error::ThreadStat(tid, e)));
        }
        Ok(())
    }

    fn read_memory(&mut self, snap: &mut Snapshot)
        -> Result<(), StatusError>
    {
        self.text_buf.truncate(0);
        try!(File::open("/proc/self/status")
             .and_then(|mut f| f.read_to_string(&mut self.text_buf)));
        for line in self.text_buf.lines() {
            let mut pairs = line.split(':');
            match (pairs.next(), pairs.next()) {
                (Some("VmPeak"), Some(text))
                => snap.memory_virtual_peak = try!(parse_memory(text)),
                (Some("VmSize"), Some(text))
                => snap.memory_virtual = try!(parse_memory(text)),
                (Some("VmRSS"), Some(text))
                => snap.memory_rss = try!(parse_memory(text)),
                (Some("VmSwap"), Some(text))
                => snap.memory_swap = try!(parse_memory(text)),
                _ => {}
            }
        }
        Ok(())
    }

    fn read_io(&mut self, snap: &mut Snapshot)
        -> Result<(), Error>
    {
        let err = &|e: ParseIntError| Error::IoStat(e.into());
        self.text_buf.truncate(0);
        try!(File::open("/proc/self/io")
             .and_then(|mut f| f.read_to_string(&mut self.text_buf)));
        for line in self.text_buf.lines() {
            let mut pairs = line.split(':');
            match (pairs.next(), pairs.next().map(|x| x.trim())) {
                (Some("rchar"), Some(text))
                => snap.read_bytes = try!(text.parse().map_err(err)),
                (Some("wchar"), Some(text))
                => snap.write_bytes = try!(text.parse().map_err(err)),
                (Some("syscr"), Some(text))
                => snap.read_ops = try!(text.parse().map_err(err)),
                (Some("syscw"), Some(text))
                => snap.write_ops = try!(text.parse().map_err(err)),
                (Some("read_bytes"), Some(text))
                => snap.read_disk_bytes = try!(text.parse().map_err(err)),
                (Some("write_bytes"), Some(text))
                => snap.write_disk_bytes = try!(text.parse().map_err(err)),
                (Some("cancelled_write_bytes"), Some(text)) => {
                    snap.write_cancelled_bytes =
                        try!(text.parse().map_err(err));
                }
                _ => {}
            }
        }
        Ok(())
    }

}

fn parse_memory(value: &str) -> Result<u64, StatusError> {
    let mut pair = value.split_whitespace();
    let value = try!(try!(pair.next().ok_or(StatusError::BadFormat))
        .parse::<u64>());
    match pair.next() {
        Some("kB") => Ok(value * 1024),
        _ => Err(StatusError::BadUnit),
    }
}

fn parse_uptime(value: &str) -> Result<u64, UptimeError> {
    if value.len() <= 3 {
        return Err(UptimeError::BadFormat);
    }
    let dot = value.len()-3;
    if !value.is_char_boundary(dot) || &value[dot..dot+1] != "." {
        return Err(UptimeError::BadFormat);
    }
    Ok(try!(value[..dot].parse::<u64>()) * 100 +
       try!(value[dot+1..].parse::<u64>()))
}

fn read_stat(text_buf: &mut String, path: &str, thread_info: &mut ThreadInfo)
    -> Result<(), StatError>
{
    text_buf.truncate(0);
    try!(File::open(path)
         .and_then(|mut f| f.read_to_string(text_buf)));
    let right_paren = try!(text_buf.rfind(')')
        .ok_or(StatError::BadFormat));
    let mut iter = text_buf[right_paren+1..].split_whitespace();
    thread_info.user_time = try!(
        try!(iter.nth(11).ok_or(StatError::BadFormat)).parse());
    thread_info.system_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    thread_info.child_user_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    thread_info.child_system_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    Ok(())
}


impl ThreadInfo {
    fn new() -> ThreadInfo {
        ThreadInfo {
            user_time: 0,
            system_time: 0,
            child_user_time: 0,
            child_system_time: 0,
        }
    }
}

impl Snapshot {
    fn new(threads: &HashMap<Pid, String>) -> Snapshot {
        Snapshot {
            timestamp: SystemTime::now(),
            instant: Instant::now(),
            uptime: 0,
            idle_time: 0,
            process: ThreadInfo::new(),
            memory_rss: 0,
            memory_virtual: 0,
            memory_virtual_peak: 0,
            memory_swap: 0,
            read_bytes: 0,
            write_bytes: 0,
            read_ops: 0,
            write_ops: 0,
            read_disk_bytes: 0,
            write_disk_bytes: 0,
            write_cancelled_bytes: 0,
            threads: threads.iter()
                .map(|(&pid, _)| (pid, ThreadInfo::new()))
                .collect(),
        }
    }
}
