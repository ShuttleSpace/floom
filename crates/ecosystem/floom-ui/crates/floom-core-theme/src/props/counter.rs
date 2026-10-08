/// Shared, design-system-agnostic props for the `Counter` component.
pub struct CounterProps {
    /// Initial value.
    pub value: i64,
    /// Optional inclusive lower bound (clamped).
    pub min: Option<i64>,
    /// Optional inclusive upper bound (clamped).
    pub max: Option<i64>,
    /// Increment/decrement step.
    pub step: i64,
    /// Called with the new value whenever it changes.
    ///
    /// `Send + Sync`, which v1's was not. A 2.0 slot is a replayed
    /// `Fn() -> Box<dyn WidgetAny> + Send + Sync`, so the engine's
    /// `Counter::on_change` demands it and a callback that cannot cross that
    /// boundary cannot be stored at all. The only crates that read this field are
    /// the three counter crates, so tightening it broke nothing that was not being
    /// ported anyway -- recorded because "it happened to break nothing" is the
    /// condition that lets an unforced API change through unnoticed.
    pub on_change: Option<Box<dyn Fn(i64) + Send + Sync>>,
}

impl Clone for CounterProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for CounterProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterProps")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("step", &self.step)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(i64)"))
            .finish()
    }
}

impl Default for CounterProps {
    fn default() -> Self {
        Self {
            value: 0,
            min: None,
            max: None,
            step: 1,
            on_change: None,
        }
    }
}
