use std::fmt;

use {Meter, ThreadReportIter};

impl fmt::Debug for Meter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Meter")
        .field("scan_interval", &self.scan_interval)
        .field("snapshots", &self.snapshots.len())
        .field("threads", &self.thread_names.len())
        .finish()
    }
}

impl<'a> fmt::Debug for ThreadReportIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadReportIter")
        .finish()
    }
}
