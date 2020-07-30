/// An `Event` to be produced by a [Store](crate::Store).
pub trait StoreEvent {
    /// Produces an empty/`None` variant of this event which returns
    /// `true` when calling [StoreEvent::is_none()].
    fn none() -> Self;
    /// Returns `true` if this event is considered empty/`None`.
    fn is_none(&self) -> bool;
}

impl StoreEvent for () {
    fn none() -> Self {}

    fn is_none(&self) -> bool {
        true
    }
}

/// If all events match `Event::none()` then returns `true`, otherwise
/// `false`.
pub fn all_events_none<Event: StoreEvent>(events: &[Event]) -> bool {
    events.iter().all(|event| event.is_none())
}

#[cfg(test)]
mod test {
    use crate::{all_events_none, StoreEvent};

    enum TestEvent {
        SomeEvent,
        None,
    }

    impl StoreEvent for TestEvent {
        fn none() -> Self {
            Self::None
        }
        fn is_none(&self) -> bool {
            if let Self::None = self {
                true
            } else {
                false
            }
        }
    }

    #[test]
    fn test_all_events_none() {
        let all_none = vec![TestEvent::None, TestEvent::None];
        let not_all_none = vec![TestEvent::SomeEvent, TestEvent::None];

        assert!(all_events_none(&all_none));
        assert!(!all_events_none(&not_all_none));
    }
}
