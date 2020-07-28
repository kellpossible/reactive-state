use crate::{
    middleware::{Middleware, ReduceMiddlewareResult},
    AsListener, Listener, Reducer, StoreEvent,
};
use std::iter::FromIterator;
use std::ops::Deref;
use std::{
    cell::{Cell, RefCell},
    collections::{HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
};

/// A [Listener] associated with (listening to) a given set of
/// `Events`s produced by a [Store::dispatch()].
struct ListenerEventPair<State, Event> {
    pub listener: Listener<State, Event>,
    pub events: HashSet<Event>,
}

impl<State, Event> Debug for ListenerEventPair<State, Event> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListenerEventPair")
    }
}

/// An action to modify some aspect of the [Store], to be stored in a
/// queue and executed at the start of a [Store::dispatch()] for a
/// given `Action`.
enum StoreModification<State, Action, Event, Effect> {
    AddListener(ListenerEventPair<State, Event>),
    AddMiddleware(Rc<dyn Middleware<State, Action, Event, Effect>>),
}

/// A wrapper for an [Rc] reference to a [Store].
///
/// This wrapper exists to provide a standard interface for re-useable
/// middleware and other components which may require a long living
/// reference to the store in order to dispatch actions or modify it
/// in some manner that could not be handled by a simple `&Store`.
#[derive(Clone)]
pub struct StoreRef<State, Action, Event, Effect>(Rc<Store<State, Action, Event, Effect>>);

impl<State, Action, Event, Effect> StoreRef<State, Action, Event, Effect>
where
    Event: StoreEvent + Clone + Hash + Eq,
{
    pub fn new<R: Reducer<State, Action, Event, Effect> + 'static>(
        reducer: R,
        initial_state: State,
    ) -> Self {
        Self(Rc::new(Store::new(reducer, initial_state)))
    }
}

impl<State, Action, Event, Effect> Deref for StoreRef<State, Action, Event, Effect> {
    type Target = Store<State, Action, Event, Effect>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<State, Action, Event, Effect> PartialEq for StoreRef<State, Action, Event, Effect> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// This struct is designed to operate as a central source of truth
/// and global "immutable" state within your application.
///
/// The current state of this store ([Store::state()]()) can only be
/// modified by dispatching an `Action` via [Store::dispatch()] to the
/// store. These actions are taken by a [Reducer] which you provided
/// to the store (at construction) and a new current state is
/// produced. The reducer also produces `Events` associated with the
/// change. The previous state is never mutated, and remains as a
/// reference for any element of your application which may rely upon
/// it (ensure that it gets dropped when it is no longer required,
/// lest it become a memory leak when large `State`s are involved).
///
/// Listeners can susbscribe to changes to the `State` in this store
/// (and `Event`s produced) with [Store::subscribe()], or they can
/// also subscribe to changes associated with specific `Event`s via
/// [subscribe_event()](Store::subscribe_event())/[subscribe_events()](Store::subscribe_events()).
pub struct Store<State, Action, Event, Effect> {
    /// This lock is used to prevent dispatch recursion.
    dispatch_lock: RefCell<()>,
    /// Queue of actions to be dispatched by [Store::dispatch()].
    dispatch_queue: RefCell<VecDeque<Action>>,
    /// Queue of [StoreModification]s to be executed by
    /// [Store::dispatch()] before the next `Action` is dispatched.
    modification_queue: RefCell<VecDeque<StoreModification<State, Action, Event, Effect>>>,
    /// The [Reducer] for this store, which takes `Actions`, modifies
    /// the `State` stored in this store, and produces `Events` to be
    /// sent to the store listeners.
    reducer: Box<dyn Reducer<State, Action, Event, Effect>>,
    /// The current state of this store.
    state: RefCell<Rc<State>>,
    /// The listeners which are notified of changes to the state of
    /// this store, and events produced by this store during a
    /// [Store::dispatch()].
    listeners: RefCell<Vec<ListenerEventPair<State, Event>>>,
    /// Middleware which modifies the functionality of this store.
    middleware: RefCell<Vec<Rc<dyn Middleware<State, Action, Event, Effect>>>>,
    /// Used during recursive execution of [Middleware] to keep track
    /// of the middleware currently executing. It is an index into
    /// [Store::middleware].
    prev_middleware: Cell<i32>,
    phantom_action: PhantomData<Action>,
    phantom_event: PhantomData<Event>,
}

impl<State, Action, Event, Effect> Store<State, Action, Event, Effect>
where
    Event: StoreEvent + Clone + Hash + Eq,
{
    /// Create a new [Store], which uses the specified `reducer` to
    /// handle `Action`s which mutate the state and produce `Event`s,
    /// and with the `initial_state`.
    pub fn new<R: Reducer<State, Action, Event, Effect> + 'static>(
        reducer: R,
        initial_state: State,
    ) -> Self {
        Self {
            dispatch_lock: RefCell::new(()),
            dispatch_queue: RefCell::new(VecDeque::new()),
            modification_queue: RefCell::new(VecDeque::new()),
            reducer: Box::new(reducer),
            state: RefCell::new(Rc::new(initial_state)),
            listeners: RefCell::new(Vec::new()),
            middleware: RefCell::new(Vec::new()),
            prev_middleware: Cell::new(-1),
            phantom_action: PhantomData,
            phantom_event: PhantomData,
        }
    }

    /// Get the current `State` stored in this store.
    ///
    /// Modifications to this state need to be performed by
    /// dispatching an `Action` to the store using
    /// [dispatch()](Store::dispatch()).
    pub fn state(&self) -> Rc<State> {
        self.state.borrow().clone()
    }

    /// Dispatch an `Action` to the reducer on this `Store` without
    /// invoking middleware.
    fn dispatch_reducer(&self, action: &Action) -> ReduceMiddlewareResult<Event, Effect> {
        let result = self.reducer.reduce(&self.state(), action);
        *self.state.borrow_mut() = result.state;

        ReduceMiddlewareResult {
            events: result.events,
            effects: result.effects,
        }
    }

    /// Dispatch an `Action` to the reducer on this `Store`, invoking
    /// all middleware's [reduce()][Middleware::reduce()] first.
    fn middleware_reduce(&self, action: &Action) -> ReduceMiddlewareResult<Event, Effect> {
        self.prev_middleware.set(-1);
        self.middleware_reduce_next(Some(action))
    }

    /// A recursive function which executes each middleware for this
    /// store, and invokes the next middleware, until all middleware
    /// has been invoked, at which point the `Action` is sent to the
    /// reducer.
    fn middleware_reduce_next(
        &self,
        action: Option<&Action>,
    ) -> ReduceMiddlewareResult<Event, Effect> {
        let current_middleware = self.prev_middleware.get() + 1;
        self.prev_middleware.set(current_middleware);

        if current_middleware == self.middleware.borrow().len() as i32 {
            return match action {
                Some(action) => self.dispatch_reducer(action),
                None => ReduceMiddlewareResult::default(),
            };
        }

        let result = self.middleware.borrow()[current_middleware as usize]
            .clone()
            .on_reduce(self, action, Self::middleware_reduce_next);

        result
    }

    /// Process all the `Effect`s returned by the [Reducer::reduce()]
    /// by invoking the middleware on this store to perform the
    /// processing using [Middleware::process_effect()].q
    fn middleware_process_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            self.middleware_process_effect(effect);
        }
    }

    /// Process the specified `Effect`, invoking all middleware in this
    /// store to perform the processing using
    /// [Middleware::process_effect()].
    fn middleware_process_effect(&self, effect: Effect) {
        self.prev_middleware.set(-1);
        self.middleware_process_effects_next(effect);
    }

    /// A recursive function which executes each middleware for this
    /// store to process the specified `Effect` with
    /// [Middleware::process_effect()], and invokes the next
    /// middleware, until all middleware has been invoked.
    fn middleware_process_effects_next(&self, effect: Effect) {
        let current_middleware = self.prev_middleware.get() + 1;
        self.prev_middleware.set(current_middleware);

        if current_middleware == self.middleware.borrow().len() as i32 {
            return;
        }

        match self.middleware.borrow()[current_middleware as usize]
            .clone()
            .process_effect(self, effect)
        {
            Some(effect) => self.middleware_process_effects_next(effect),
            None => {}
        }
    }

    /// Notify store listeners of events produced during a reduce as a
    /// result of an `Action` being dispatched. Invokes all
    /// middleware's [reduce()][Middleware::reduce()] first.
    /// Notification occurs even if there are no events to report.
    fn middleware_notify(&self, events: Vec<Event>) -> Vec<Event> {
        self.prev_middleware.set(-1);
        self.middleware_notify_next(events)
    }

    /// A recursive function which executes each middleware for this
    /// store, and invokes the next middleware, until all middleware
    /// has been invoked, at which point the listeners are notified of
    /// the envents produced during a reduce as a result of an
    /// `Action` being dispatched. Notification occurs even if there
    /// are no events to report.
    fn middleware_notify_next(&self, events: Vec<Event>) -> Vec<Event> {
        let current_middleware = self.prev_middleware.get() + 1;
        self.prev_middleware.set(current_middleware);

        if current_middleware == self.middleware.borrow().len() as i32 {
            return events;
        }

        self.middleware.borrow()[current_middleware as usize]
            .clone()
            .on_notify(self, events, Self::middleware_notify_next)
    }

    /// Notify store listeners of events produced during a result of
    /// an `Action` being dispatched. Notification occurs even if
    /// there are no events to report.
    fn notify_listeners(&self, events: Vec<Event>) {
        let mut listeners_to_remove: Vec<usize> = Vec::new();
        for (i, pair) in self.listeners.borrow().iter().enumerate() {
            let retain = match pair.listener.as_callback() {
                Some(callback) => {
                    if pair.events.is_empty() {
                        callback.emit(self.state.borrow().clone(), Event::none());
                    } else {
                        //  call the listener for every matching listener event
                        for event in &events {
                            if pair.events.contains(event) {
                                callback.emit(self.state.borrow().clone(), event.clone());
                            }
                        }
                    }

                    true
                }
                None => false,
            };

            if !retain {
                listeners_to_remove.insert(0, i);
            }
        }

        for index in listeners_to_remove {
            self.listeners.borrow_mut().swap_remove(index);
        }
    }

    fn process_pending_modifications(&self) {
        while let Some(modification) = self.modification_queue.borrow_mut().pop_front() {
            match modification {
                StoreModification::AddListener(listener_pair) => {
                    self.listeners.borrow_mut().push(listener_pair);
                }
                StoreModification::AddMiddleware(middleware) => {
                    self.middleware.borrow_mut().push(middleware);
                }
            }
        }
    }

    /// Dispatch an `Action` to be passed to the [Reducer] in order to
    /// modify the `State` in this store, and produce `Events` to be
    /// sent to the store listeners.
    pub fn dispatch<A: Into<Action>>(&self, action: A) {
        self.dispatch_impl(action.into());
    }

    /// Concrete version of [Store::dispatch()], for code size
    /// reduction purposes, to avoid generating multiple versions of
    /// this complex function per action that implements
    /// `Into<Action>`, it is expected that there will be many in a
    /// typical application.
    fn dispatch_impl(&self, action: Action) {
        self.dispatch_queue.borrow_mut().push_back(action);

        // If the lock fails to acquire, then the dispatch is already in progress.
        // This prevents recursion, when a listener callback also triggers another
        // dispatch.
        if let Ok(_lock) = self.dispatch_lock.try_borrow_mut() {
            // For some strange reason can't use a while let here because
            // it requires Action to implement Copy, and also it was maintaining
            // the dispatch_queue borrow during the loop (even though it wasn't needed).
            loop {
                let dispatch_action = self.dispatch_queue.borrow_mut().pop_front();

                match dispatch_action {
                    Some(action) => {
                        self.process_pending_modifications();

                        let reduce_middleware_result = if self.middleware.borrow().is_empty() {
                            self.dispatch_reducer(&action)
                        } else {
                            self.middleware_reduce(&action)
                        };

                        match reduce_middleware_result {
                            ReduceMiddlewareResult { events, effects } => {
                                self.middleware_process_effects(effects);

                                let middleware_events = self.middleware_notify(events);
                                if !middleware_events.is_empty() {
                                    self.notify_listeners(middleware_events);
                                }
                            }
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }

    /// Subscribe a [Listener] to changes in the store state and
    /// events produced by the [Reducer] as a result of `Action`s
    /// dispatched via [dispatch()][Store::dispatch()].
    ///
    /// The listener is a weak reference; when the strong reference
    /// associated with it (usually [Callback](crate::Callback)) is
    /// dropped, the listener will be removed from this store upon
    /// [dispatch()](Store::dispatch()).
    ///
    /// If you want to subscribe to state changes associated with
    /// specific `Event`s, see
    /// [subscribe_event()][Store::subscribe_events()] or
    /// [subscribe_event()][Store::subscribe_events()]
    pub fn subscribe<L: AsListener<State, Event>>(&self, listener: L) {
        self.modification_queue
            .borrow_mut()
            .push_back(StoreModification::AddListener(ListenerEventPair {
                listener: listener.as_listener(),
                events: HashSet::new(),
            }));
    }

    /// Subscribe a [Listener] to changes in the store state and
    /// events produced by the [Reducer] as a result of `Action`s
    /// being dispatched via [dispatch()][Store::dispatch()] and
    /// reduced with the store's [Reducer]. This subscription is only
    /// active changes which produce the specific matching `event`
    /// from the [Reducer].
    ///
    /// The listener is a weak reference; when the strong reference
    /// associated with it (usually [Callback](crate::Callback)) is
    /// dropped, the listener will be removed from this store upon
    /// [dispatch()](Store::dispatch()).
    pub fn subscribe_event<L: AsListener<State, Event>>(&self, listener: L, event: Event) {
        let mut events = HashSet::with_capacity(1);
        events.insert(event);

        self.modification_queue
            .borrow_mut()
            .push_back(StoreModification::AddListener(ListenerEventPair {
                listener: listener.as_listener(),
                events,
            }));
    }

    /// Subscribe a [Listener] to changes in the store state and
    /// events produced by the [Reducer] as a result of `Action`s
    /// being dispatched via [dispatch()][Store::dispatch()] and
    /// reduced with the store's [Reducer]. This subscription is only
    /// active changes which produce any of the specific matching
    /// `events` from the [Reducer].
    ///
    /// The listener is a weak reference; when the strong reference
    /// associated with it (usually [Callback](crate::Callback)) is
    /// dropped, the listener will be removed from this store upon
    /// [dispatch()](Store::dispatch()).
    pub fn subscribe_events<L: AsListener<State, Event>, E: IntoIterator<Item = Event>>(
        &self,
        listener: L,
        events: E,
    ) {
        self.modification_queue
            .borrow_mut()
            .push_back(StoreModification::AddListener(ListenerEventPair {
                listener: listener.as_listener(),
                events: HashSet::from_iter(events.into_iter()),
            }));
    }

    /// Add [Middleware] to modify the behaviour of this [Store]
    /// during a [dispatch()][Store::dispatch()].
    pub fn add_middleware<M: Middleware<State, Action, Event, Effect> + 'static>(
        &self,
        middleware: M,
    ) {
        self.modification_queue
            .borrow_mut()
            .push_back(StoreModification::AddMiddleware(Rc::new(middleware)));
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        middleware::{Middleware, ReduceMiddlewareResult},
        Callback, Reducer, ReducerResult, Store, StoreEvent, StoreRef,
    };
    use std::{cell::RefCell, rc::Rc};

    #[derive(Debug, PartialEq)]
    struct TestState {
        counter: i32,
    }

    #[derive(Copy, Clone)]
    enum TestAction {
        Increment,
        Decrement,
        Decrement2,
        Decrent2Then1,
    }

    enum TestEffect {
        ChainAction(TestAction),
    }

    struct TestReducer;

    impl Reducer<TestState, TestAction, TestEvent, TestEffect> for TestReducer {
        fn reduce(
            &self,
            state: &Rc<TestState>,
            action: &TestAction,
        ) -> ReducerResult<TestState, TestEvent, TestEffect> {
            let mut events = Vec::new();
            let mut effects = Vec::new();

            let new_state = match action {
                TestAction::Increment => TestState {
                    counter: state.counter + 1,
                },
                TestAction::Decrement => TestState {
                    counter: state.counter - 1,
                },
                TestAction::Decrement2 => TestState {
                    counter: state.counter - 2,
                },
                TestAction::Decrent2Then1 => {
                    effects.push(TestEffect::ChainAction(TestAction::Decrement));

                    TestState {
                        counter: state.counter - 2,
                    }
                }
            };

            // All actions change the counter.
            events.push(TestEvent::CounterChanged);

            if new_state.counter != state.counter && new_state.counter == 0 {
                events.push(TestEvent::CounterIsZero);
            }

            ReducerResult {
                state: Rc::new(new_state),
                events,
                effects,
            }
        }
    }

    struct TestReduceMiddleware {
        new_action: TestAction,
    }

    impl Middleware<TestState, TestAction, TestEvent, TestEffect> for TestReduceMiddleware {
        fn on_reduce(
            &self,
            store: &Store<TestState, TestAction, TestEvent, TestEffect>,
            action: Option<&TestAction>,
            reduce: crate::middleware::ReduceFn<TestState, TestAction, TestEvent, TestEffect>,
        ) -> ReduceMiddlewareResult<TestEvent, TestEffect> {
            reduce(store, action.map(|_| &self.new_action))
        }
    }

    struct TestEffectMiddleware;

    impl Middleware<TestState, TestAction, TestEvent, TestEffect> for TestEffectMiddleware {
        fn process_effect(
            &self,
            store: &Store<TestState, TestAction, TestEvent, TestEffect>,
            effect: TestEffect,
        ) -> Option<TestEffect> {
            match effect {
                TestEffect::ChainAction(action) => {
                    store.dispatch(action);
                }
            }

            None
        }
    }

    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    enum TestEvent {
        CounterIsZero,
        CounterChanged,
        None,
    }

    impl StoreEvent for TestEvent {
        fn none() -> Self {
            Self::None
        }

        fn is_none(&self) -> bool {
            match self {
                TestEvent::None => true,
                _ => false,
            }
        }
    }

    #[test]
    fn test_notify() {
        let initial_state = TestState { counter: 0 };
        let store: Rc<RefCell<Store<TestState, TestAction, TestEvent, TestEffect>>> =
            Rc::new(RefCell::new(Store::new(TestReducer, initial_state)));

        let callback_test = Rc::new(RefCell::new(0));
        let callback_test_copy = callback_test.clone();
        let callback: Callback<TestState, TestEvent> =
            Callback::new(move |state: Rc<TestState>, _| {
                *callback_test_copy.borrow_mut() = state.counter;
            });

        store.borrow_mut().subscribe(&callback);

        assert_eq!(0, store.borrow().state().counter);

        store.borrow_mut().dispatch(TestAction::Increment);
        store.borrow_mut().dispatch(TestAction::Increment);
        assert_eq!(2, *callback_test.borrow());
        assert_eq!(2, store.borrow().state().counter);

        store.borrow_mut().dispatch(TestAction::Decrement);
        assert_eq!(1, store.borrow().state().counter);
    }

    #[test]
    fn test_reduce_middleware() {
        let initial_state = TestState { counter: 0 };
        let store = StoreRef::new(TestReducer, initial_state);

        let callback_test = Rc::new(RefCell::new(0));
        let callback_test_copy = callback_test.clone();
        let callback: Callback<TestState, TestEvent> =
            Callback::new(move |state: Rc<TestState>, _| {
                *callback_test_copy.borrow_mut() = state.counter;
            });

        store.subscribe(&callback);
        store.add_middleware(TestReduceMiddleware {
            new_action: TestAction::Decrement,
        });
        store.add_middleware(TestReduceMiddleware {
            new_action: TestAction::Decrement2,
        });

        store.dispatch(TestAction::Increment);
        assert_eq!(-2, *callback_test.borrow());
    }

    #[test]
    fn test_reduce_middleware_reverse_order() {
        let initial_state = TestState { counter: 0 };
        let store = StoreRef::new(TestReducer, initial_state);

        let callback_test = Rc::new(RefCell::new(0));
        let callback_test_copy = callback_test.clone();
        let callback: Callback<TestState, TestEvent> =
            Callback::new(move |state: Rc<TestState>, _| {
                *callback_test_copy.borrow_mut() = state.counter;
            });

        store.subscribe(&callback);
        store.add_middleware(TestReduceMiddleware {
            new_action: TestAction::Decrement2,
        });
        store.add_middleware(TestReduceMiddleware {
            new_action: TestAction::Decrement,
        });

        store.dispatch(TestAction::Increment);
        assert_eq!(-1, *callback_test.borrow());
    }

    #[test]
    fn test_effect_middleware() {
        let initial_state = TestState { counter: 0 };
        let store = StoreRef::new(TestReducer, initial_state);
        store.add_middleware(TestEffectMiddleware);

        assert_eq!(store.state().counter, 0);
        store.dispatch(TestAction::Decrent2Then1);
        assert_eq!(store.state().counter, -3);
    }

    #[test]
    fn test_subscribe_event() {
        let initial_state = TestState { counter: -2 };
        let store = StoreRef::new(TestReducer, initial_state);

        let callback_test: Rc<RefCell<Option<TestEvent>>> = Rc::new(RefCell::new(None));
        let callback_test_copy = callback_test.clone();

        let callback_zero_subscription: Callback<TestState, TestEvent> =
            Callback::new(move |_: Rc<TestState>, event| {
                assert_eq!(TestEvent::CounterIsZero, event);
                *callback_test_copy.borrow_mut() = Some(TestEvent::CounterIsZero);
            });

        store.subscribe_event(&callback_zero_subscription, TestEvent::CounterIsZero);
        store.dispatch(TestAction::Increment);
        assert_eq!(None, *callback_test.borrow());
        store.dispatch(TestAction::Increment);
        assert_eq!(Some(TestEvent::CounterIsZero), *callback_test.borrow());
    }
}
