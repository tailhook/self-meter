use std::time::{Duration, SystemTime, UNIX_EPOCH};

use {Meter, Report};


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
        Some(Report {
            timestamp_ms: duration_to_ms(
                last.timestamp.duration_since(UNIX_EPOCH).unwrap()),
            duration_ms: duration_to_ms(last.instant - prev.instant),
            global_cpu_usage: 100.0 * (1.0 -
                (last.idle_time - prev.idle_time) as f32 /
                (centisecs * self.num_cpus as f32)),
            process_cpu_usage: 100.0 *
                (lpro.user_time + lpro.system_time -
                 (ppro.user_time + ppro.system_time)) as f32 / centisecs,
            gross_cpu_usage: 100.0 *
                ((lpro.user_time  + lpro.system_time +
                  lpro.child_user_time + lpro.child_system_time) -
                 (ppro.user_time + ppro.system_time +
                  ppro.child_user_time + ppro.child_system_time)) as f32 /
                centisecs,
        })
    }
}
