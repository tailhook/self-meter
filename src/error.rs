use std::io;
use std::num::ParseIntError;

use Pid;


quick_error! {
    #[derive(Debug)]
    /// Error reading or parsing /proc/uptime
    pub enum UptimeError {
        Io(err: io::Error) {
            description("IO error")
            display("{}", err)
            from()
        }
        ParseInt(e: ParseIntError) {
            description("error parsing int")
            display("error parsing int: {}", e)
            from()
        }
        BadFormat {
            description("bad format")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    /// Error reading or parsing /proc/self/stat or /proc/self/task/<TID>/stat
    pub enum StatError {
        Io(err: io::Error) {
            description("IO error")
            display("{}", err)
            from()
        }
        ParseInt(e: ParseIntError) {
            description("error parsing int")
            display("error parsing int: {}", e)
            from()
        }
        BadFormat {
            description("bad format")
        }
    }
}


quick_error! {
    #[derive(Debug)]
    /// Error scanning process info in /proc
    pub enum Error {
        Cpu(err: io::Error) {
            description("Error reading /sys/devices/system/cpu")
            display("Error reading /sys/devices/system/cpu: {}", err)
            from()
        }
        Uptime(err: UptimeError) {
            description("Error reading /proc/uptime")
            display("Error reading /proc/uptime: {}", err)
            from()
        }
        Stat(err: StatError) {
            description("Error reading /proc/self/stat")
            display("Error reading /proc/self/stat: {}", err)
        }
        ThreadStat(tid: Pid, err: StatError) {
            description("Error reading /proc/self/task/<TID>/stat")
            display("Error reading /proc/self/task/{}/stat: {}", tid, err)
        }
    }
}
