use std::time::{Duration, UNIX_EPOCH};
use std::collections::hash_map::Iter;

use {Pid, Meter, Report, Snapshot, ThreadReport};


pub struct ThreadReportIter<'a> {
    threads: Iter<'a,Pid, String>,
    last: &'a Snapshot,
    prev: &'a Snapshot,
    centisecs: f32,
}


fn duration_to_ms(dur: Duration) -> u64 {
    return dur.as_secs() * 1000 + (dur.subsec_nanos() / 1000000) as u64;
}


impl Meter {
    pub fn report(&self) -> Option<Report> {
        if self.snapshots.len() < 2 {
            return None;
        }
        let n = self.snapshots.len();
        let last = &self.snapshots[n-1];
        let prev = &self.snapshots[n-2];
        let lpro = &last.process;
        let ppro = &prev.process;
        let centisecs = (last.uptime - prev.uptime) as f32;
        let secs = centisecs / 100.0;
        let mut cpu_usage = 100.0 * (1.0 -
                (last.idle_time - prev.idle_time) as f32 /
                (centisecs * self.num_cpus as f32));
        if cpu_usage < 0. {  // sometimes we get inaccuracy
            cpu_usage = 0.;
        }
        Some(Report {
            timestamp_ms: duration_to_ms(
                last.timestamp.duration_since(UNIX_EPOCH).unwrap()),
            duration_ms: duration_to_ms(last.instant - prev.instant),
            global_cpu_usage: cpu_usage,
            process_cpu_usage: 100.0 *
                (lpro.user_time + lpro.system_time -
                 (ppro.user_time + ppro.system_time)) as f32 / centisecs,
            gross_cpu_usage: 100.0 *
                ((lpro.user_time  + lpro.system_time +
                  lpro.child_user_time + lpro.child_system_time) -
                 (ppro.user_time + ppro.system_time +
                  ppro.child_user_time + ppro.child_system_time)) as f32 /
                centisecs,
            memory_rss: last.memory_rss,
            memory_virtual: last.memory_virtual,
            memory_swap: last.memory_swap,
            memory_rss_peak: self.memory_rss_peak,
            memory_virtual_peak: last.memory_virtual_peak,
            memory_swap_peak: self.memory_swap_peak,
            disk_read: (last.read_disk_bytes - prev.read_disk_bytes) as f32
                / secs,
            disk_write: (last.write_disk_bytes - prev.write_disk_bytes) as f32
                / secs,
            disk_cancelled: (last.write_cancelled_bytes -
                             prev.write_cancelled_bytes) as f32 / secs,
            io_read: (last.read_bytes - prev.read_bytes) as f32 / secs,
            io_write: (last.write_bytes - prev.write_bytes) as f32 / secs,
            io_read_ops: (last.read_ops - prev.read_ops) as f32 / secs,
            io_write_ops: (last.write_ops - prev.write_ops) as f32 / secs,
        })
    }
    pub fn thread_report(&self) -> Option<ThreadReportIter> {
        if self.snapshots.len() < 2 {
            return None;
        }
        let n = self.snapshots.len();
        let last = &self.snapshots[n-1];
        let prev = &self.snapshots[n-2];
        let centisecs = (last.uptime - prev.uptime) as f32;
        Some(ThreadReportIter {
            threads: self.thread_names.iter(),
            last: last,
            prev: prev,
            centisecs: centisecs,
        })
    }
}

impl<'a> Iterator for ThreadReportIter<'a> {
    type Item = (&'a str, ThreadReport);
    fn next(&mut self) -> Option<(&'a str, ThreadReport)> {
        while let Some((&pid, name)) = self.threads.next() {
            let lth = if let Some(thread) = self.last.threads.get(&pid) {
                thread
            } else {
                continue;  // not enough stats for a thread yet
            };
            let pth = if let Some(thread) = self.prev.threads.get(&pid) {
                thread
            } else {
                continue;  // not enough stats for a thread yet
            };
            let udelta = lth.user_time - pth.user_time;
            let sdelta = lth.system_time - pth.system_time;
            return Some((&name[..], ThreadReport {
                cpu_usage: 100.0 * (udelta + sdelta) as f32 / self.centisecs,
                system_cpu: 100.0 * sdelta as f32 / self.centisecs,
                user_cpu: 100.0 * udelta as f32 / self.centisecs,
            }))
        }
        None
    }
}
