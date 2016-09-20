extern crate self_meter;

use std::io::{Write, stderr};
use std::time::Duration;
use std::thread::sleep;
use std::collections::BTreeMap;


fn main() {
    let mut meter = self_meter::Meter::new(Duration::new(1, 0)).unwrap();
    meter.track_current_thread("main");
    loop {
        meter.scan()
            .map_err(|e| writeln!(&mut stderr(), "Scan error: {}", e)).ok();
        println!("Report: {:#?}", meter.report());
        println!("Threads: {:#?}",
            meter.thread_report().map(|x| x.collect::<BTreeMap<_,_>>()));
        let mut x = 0;
        for _ in 0..10000000 {
            x = u64::wrapping_mul(x, 7);
        }
        sleep(Duration::new(1, 0));
    }
}
