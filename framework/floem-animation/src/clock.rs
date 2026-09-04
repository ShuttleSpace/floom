//! Clocks for animation (real or fake).

pub trait Clock {
    fn now_secs(&self) -> f64;
}

/// Deterministic test clock.
#[derive(Debug, Clone, Default)]
pub struct FakeClock {
    pub secs: f64,
}

impl FakeClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self, dt: f64) {
        self.secs += dt;
    }
}

impl Clock for FakeClock {
    fn now_secs(&self) -> f64 {
        self.secs
    }
}
