/// An `Event` to be produced by a [Store](crate::Store).
pub trait StoreEvent {
    /// Produces an empty/`None` variant of this event which returns
    /// `true` when calling [StoreEvent::is_none()].
    fn none() -> Self;
    /// Returns `true` if this event is considered empty/`None`.
    fn is_none(&self) -> bool;
}

impl StoreEvent for () {
    fn none() -> Self {
        ()
    }

    fn is_none(&self) -> bool {
        true
    }
}
