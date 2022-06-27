use indicatif::ProgressBar;

pub trait ProgressCollector {
    fn inc(&self, delta: u64);
    fn finish(&self);
}

pub struct TUICollector {
    pb: ProgressBar,
}

impl TUICollector {
    pub fn new(len: u64) -> Self {
        Self {
            pb: ProgressBar::new(len),
        }
    }
}

impl ProgressCollector for TUICollector {
    fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    fn finish(&self) {
        self.pb.finish();
    }
}
