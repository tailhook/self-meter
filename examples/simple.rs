extern crate self_meter;

use std::io::{Write, stderr};
use std::time::Duration;
use std::thread::sleep;


fn main() {
    let mut meter = self_meter::Meter::new(Duration::new(1, 0)).unwrap();
    loop {
        meter.scan()
            .map_err(|e| writeln!(&mut stderr(), "Scan error: {}", e)).ok();
        println!("Report: {:?}", meter.report());
        sleep(Duration::new(1, 0));
    }
}
