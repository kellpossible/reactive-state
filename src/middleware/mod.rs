//! [Middleware] used to modify the behaviour of a [Store] during a
//! [Store::dispatch()]. This module also contains some simple
//! middleware implementations which can be used as utilities in an
//! application.

pub mod simple_logger;
pub mod web_logger;

use crate::Store;

pub struct ReduceMiddlewareResult<Event, Effect> {
    pub events: Vec<Event>,
    pub effects: Vec<Effect>,
}

impl<Event, Effect> Default for ReduceMiddlewareResult<Event, Effect> {
    fn default() -> Self {
        ReduceMiddlewareResult {
            events: Vec::new(),
            effects: Vec::new(),
        }
    }
}

/// Executes subsequent middleware and then runs the [Reducer](crate::Reducer).
pub type ReduceFn<State, Action, Event, Effect> = fn(
    &Store<State, Action, Event, Effect>,
    Option<&Action>,
) -> ReduceMiddlewareResult<Event, Effect>;

/// Executes subsequent middleware and then notifies the listeners.
pub type NotifyFn<State, Action, Event, Effect> =
    fn(&Store<State, Action, Event, Effect>, Vec<Event>) -> Vec<Event>;

/// `Middleware` used to modify the behaviour of a [Store] during a
/// [Store::dispatch()].
pub trait Middleware<State, Action, Event, Effect> {
    /// This method is invoked by the [Store] during a
    /// [Store::dispatch()] just before the `Action` is sent to the
    /// [Reducer](crate::Reducer). It is necessary to call the
    /// provided `reduce` function, which executes subsequent
    /// middleware and runs the [Reducer](crate::Reducer), and usually
    /// the events produced by the `reduce` function are returned from
    /// this method.
    ///
    /// This method allows modifying the action in question, or even
    /// removing it, preventing the [Reducer](crate::Reducer) from
    /// processing the action. It also allows modifying the events
    /// produced by the [Reducer](crate::Reducer) before the
    /// [Middleware::on_notify()] is invoked and they are sent to the
    /// [Store] listeners.
    fn on_reduce(
        &self,
        store: &Store<State, Action, Event, Effect>,
        action: Option<&Action>,
        reduce: ReduceFn<State, Action, Event, Effect>,
    ) -> ReduceMiddlewareResult<Event, Effect> {
        reduce(store, action)
    }

    /// Process an `Effect`. Returns `None` if the effect was
    /// processed/consumed by this handler, otherwise returns
    /// `Some(effect)`.
    fn process_effect(
        &self,
        _store: &Store<State, Action, Event, Effect>,
        effect: Effect,
    ) -> Option<Effect> {
        Some(effect)
    }

    /// This method is invoked by the [Store] during a
    /// [Store::dispatch()] after the [Reducer](crate::Reducer) has
    /// processed the `Action` and all [Middleware::on_reduce()]
    /// methods have completed, just before resulting events are
    /// sent to the store listeners. It is necessary to call the
    /// provided `notify` function, which executes subsequent
    /// middleware and then notifies the listeners.
    ///
    /// This method allows modifying the events in question before the
    /// listeners are notified.
    fn on_notify(
        &self,
        store: &Store<State, Action, Event, Effect>,
        events: Vec<Event>,
        notify: NotifyFn<State, Action, Event, Effect>,
    ) -> Vec<Event> {
        notify(store, events)
    }
}
