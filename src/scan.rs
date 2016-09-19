use std::io::Read;
use std::fmt::Write;
use std::fs::File;
use std::time::{Instant, SystemTime};
use std::collections::HashMap;

use {Meter, Snapshot, ThreadInfo, Pid, Error};
use error::{UptimeError, StatError};


impl Meter {
    pub fn scan(&mut self) -> Result<(), Error> {
        let mut snap = if self.snapshots.len() >= self.num_snapshots {
            self.snapshots.pop_front().unwrap()
        } else {
            Snapshot::new(&self.thread_names)
        };
        // First scan everything that relates to cpu_time to have as accurate
        // CPU usage measurements as possible
        snap.timestamp = SystemTime::now();
        snap.instant = Instant::now();
        read_cpu_times(&mut snap.process,
            &mut snap.threads,
            &mut snap.uptime, &mut snap.idle_time,
            &self.thread_names);
        self.snapshots.push_back(snap);
        Ok(())
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

fn read_stat(path: &str, thread_info: &mut ThreadInfo, buf: &mut String)
    -> Result<(), StatError>
{
    buf.truncate(0);
    try!(File::open(path)
         .and_then(|mut f| f.read_to_string(buf)));
    let right_paren = try!(buf.rfind(')').ok_or(StatError::BadFormat));
    let mut iter = buf[right_paren+1..].split_whitespace();
    thread_info.user_time = try!(
        try!(iter.nth(11).ok_or(StatError::BadFormat)).parse());
    thread_info.system_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    thread_info.child_user_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    thread_info.child_system_time = try!(
        try!(iter.next().ok_or(StatError::BadFormat)).parse());
    println!("Child {} / {}", thread_info.child_user_time,
        thread_info.child_system_time);
    Ok(())
}

fn read_cpu_times(process: &mut ThreadInfo,
                  threads: &mut HashMap<Pid, ThreadInfo>,
                  uptime: &mut u64, idle_time: &mut u64,
                  thread_names: &HashMap<Pid, String>)
    -> Result<(), Error>
{
    let mut buf = String::with_capacity(1024);
    try!(File::open("/proc/uptime")
         .and_then(|mut f| f.read_to_string(&mut buf))
         .map_err(|e| Error::Uptime(e.into())));
    {
        let mut iter = buf.split_whitespace();
        let seconds = try!(iter.next()
            .ok_or(Error::Uptime(UptimeError::BadFormat)));
        let idle_sec = try!(iter.next()
            .ok_or(Error::Uptime(UptimeError::BadFormat)));
        *uptime = try!(parse_uptime(seconds));
        *idle_time = try!(parse_uptime(idle_sec));
    }
    try!(read_stat("/proc/self/stat", process, &mut buf)
        .map_err(Error::Stat));
    let mut path_buf = String::with_capacity(100);
    for (&tid, value) in thread_names {
        path_buf.truncate(0);
        write!(&mut path_buf, "/proc/self/{}/stat", tid).unwrap();
        try!(read_stat(&path_buf[..],
            threads.entry(tid).or_insert_with(ThreadInfo::new), &mut buf)
            .map_err(|e| Error::ThreadStat(tid, e)));
    }
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
