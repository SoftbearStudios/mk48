/// Resettable data build from updates.
pub trait Apply<U>: Default {
    /// Applies an inbound update to the state.
    fn apply(&mut self, update: U);
    /// Resets the state to default.
    fn reset(&mut self) {
        *self = Self::default();
    }
}
