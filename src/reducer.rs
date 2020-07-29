use std::rc::Rc;

/// A wrapper for a function that implements the [Reducer](Reducer)
/// trait.
///
/// ## Example
///
/// ```
/// # #[derive(Clone)]
/// # struct MyState {
/// #     pub variable: bool
/// # }
/// #
/// # enum MyAction {
/// #     SomeAction
/// # }
/// #
/// # enum MyEvent {
/// #     SomeEvent
/// # }
/// #
/// # enum MyEffect {
/// #     SomeEffect
/// # }
/// use reactive_state::{ReducerFn, ReducerResult, Reducer};
/// use std::rc::Rc;
///
/// let reducer: ReducerFn<MyState, MyAction, MyEvent, MyEffect> = |state, action| {
///     let new_state = match action {
///         MyAction::SomeAction => {
///             // create a new state to mutate and return
///             let mut new_state = MyState::clone(state);
///             new_state.variable = true;
///             Rc::new(new_state)
///         }
///     };
///
///     ReducerResult {
///         state: new_state,
///         events: vec![],
///         effects: vec![],
///     }
/// };
///
/// let state1 = Rc::new(MyState {
///     variable: false
/// });
///
/// let result = reducer.reduce(&state1, &MyAction::SomeAction);
/// let state2 = &result.state;
///
/// assert_eq!(false, state1.variable);
/// assert_eq!(true, state2.variable);
/// ```
///
/// For a more comprehensive example of how reducers are used in the
/// context of the entire system, see [reactive_state](crate).
pub type ReducerFn<State, Action, Event, Effect> =
    fn(&Rc<State>, &Action) -> ReducerResult<State, Event, Effect>;

impl<State, Action, Event, Effect> Reducer<State, Action, Event, Effect>
    for ReducerFn<State, Action, Event, Effect>
{
    fn reduce(
        &self,
        prev_state: &Rc<State>,
        action: &Action,
    ) -> ReducerResult<State, Event, Effect> {
        (self)(prev_state, action)
    }
}

/// Using the [reduce()](Reducer::reduce()) method, implementors of
/// this trait take an `Action` submitted to a store via
/// [Store::dispatch()](crate::Store::dispatch()) and modifies the
/// `State` in the store, producing a new `State`, and also producing
/// events and effects associated with the `Action` and state
/// modifications that occurred.
///
/// For an example of how a reducer function should work, see
/// [ReducerFn](ReducerFn). For an example of how to use one in
/// conjunction with a [Store](crate::Store), see
/// [reactive_state](crate).
pub trait Reducer<State, Action, Event, Effect> {
    /// Take an `Action` submitted to a store via
    /// [Store::dispatch()](crate::Store::dispatch()) and modifies the
    /// `prev_state`, producing a new `State`, and also producing
    /// events associated with the `Action` and state modifications
    /// that occurred.
    ///
    /// This method should be a pure function, with any required side
    /// effects being emmitted via the returned
    /// [ReducerResult](ReducerResult).
    ///
    /// `Events`s should generally be treated purely as a notification
    /// that some subset of the state has been modified, such that
    /// playing the events and state transitions in reverse will
    /// result in the same application behaviour.
    ///
    /// If no `Event`s are returned then it is assumed that the state
    /// has not changed, and store listeners do not need to be
    /// notified.
    ///
    /// `Effect`s are side effects invoked as a result of the action,
    /// these may involve dispatching further actions, or modifying
    /// some other part of the system that the store is involved with.
    /// `Effect`s are processed using
    /// [Middleware](crate::middleware::Middleware) which has been
    /// added to the [Store](crate::Store).
    fn reduce(
        &self,
        prev_state: &Rc<State>,
        action: &Action,
    ) -> ReducerResult<State, Event, Effect>;
}

/// The result of a [Reducer::reduce()] function.
///
/// `Events`s should generally be treated purely as a notification
/// that some subset of the state has been modified, such that
/// playing the events and state transitions in reverse will
/// result in the same application behaviour.
///
/// `Effect`s are side effects invoked as a result of the action,
/// these may involve dispatching further actions, or modifying
/// some other part of the system that the store is involved with.
/// `Effect`s are processed using [Middleware](crate::middleware::Middleware)
/// which has been added to the [Store](crate::Store).
pub struct ReducerResult<State, Event, Effect> {
    pub state: Rc<State>,
    pub events: Vec<Event>,
    pub effects: Vec<Effect>,
}

impl<State, Event, Effect> Default for ReducerResult<State, Event, Effect>
where
    State: Default,
{
    fn default() -> Self {
        Self {
            state: Rc::new(State::default()),
            events: vec![],
            effects: vec![],
        }
    }
}

// TODO: create a zero cost macro version of this #17
/// A [Reducer] composed of multiple reducers.
pub struct CompositeReducer<State, Action, Event, Effect> {
    reducers: Vec<Box<dyn Reducer<State, Action, Event, Effect>>>,
}

impl<State, Action, Event, Effect> CompositeReducer<State, Action, Event, Effect> {
    /// Create a new [CompositeReducer].
    pub fn new(reducers: Vec<Box<dyn Reducer<State, Action, Event, Effect>>>) -> Self {
        CompositeReducer { reducers }
    }
}

impl<State, Action, Event, Effect> Reducer<State, Action, Event, Effect>
    for CompositeReducer<State, Action, Event, Effect>
{
    fn reduce(
        &self,
        prev_state: &Rc<State>,
        action: &Action,
    ) -> ReducerResult<State, Event, Effect> {
        let mut sum_result: ReducerResult<State, Event, Effect> = ReducerResult {
            state: prev_state.clone(),
            events: Vec::new(),
            effects: Vec::new(),
        };

        for reducer in &self.reducers {
            let result = reducer.reduce(&sum_result.state, action);
            sum_result.state = result.state;
            sum_result.events.extend(result.events);
            sum_result.effects.extend(result.effects);
        }

        sum_result
    }
}

#[cfg(test)]
mod tests {
    use crate::{CompositeReducer, Reducer, ReducerResult};
    use std::rc::Rc;

    struct TestState {
        emitted_events: Vec<TestEvent>,
    }

    impl Default for TestState {
        fn default() -> Self {
            TestState {
                emitted_events: Vec::new(),
            }
        }
    }

    struct TestAction;

    #[derive(Debug, Clone, PartialEq)]
    enum TestEvent {
        Event1,
        Event2,
    }

    #[derive(Debug, PartialEq)]
    enum TestEffect {
        Effect1,
        Effect2,
    }

    struct Reducer1;

    impl Reducer<TestState, TestAction, TestEvent, TestEffect> for Reducer1 {
        fn reduce(
            &self,
            prev_state: &Rc<TestState>,
            _action: &TestAction,
        ) -> crate::ReducerResult<TestState, TestEvent, TestEffect> {
            let mut emitted_events = prev_state.emitted_events.clone();
            emitted_events.push(TestEvent::Event1);
            let state = Rc::new(TestState { emitted_events });

            ReducerResult {
                state,
                events: vec![TestEvent::Event1],
                effects: vec![TestEffect::Effect1],
            }
        }
    }

    struct Reducer2;

    impl Reducer<TestState, TestAction, TestEvent, TestEffect> for Reducer2 {
        fn reduce(
            &self,
            prev_state: &Rc<TestState>,
            _action: &TestAction,
        ) -> crate::ReducerResult<TestState, TestEvent, TestEffect> {
            let mut emitted_events = prev_state.emitted_events.clone();
            emitted_events.push(TestEvent::Event2);
            let state = Rc::new(TestState { emitted_events });

            ReducerResult {
                state,
                events: vec![TestEvent::Event2],
                effects: vec![TestEffect::Effect2],
            }
        }
    }

    #[test]
    fn composite_reducer() {
        let reducer = CompositeReducer::new(vec![Box::new(Reducer1), Box::new(Reducer2)]);

        let result = reducer.reduce(&Rc::new(TestState::default()), &TestAction);
        assert_eq!(
            result.state.emitted_events,
            vec![TestEvent::Event1, TestEvent::Event2]
        );
        assert_eq!(result.events, vec![TestEvent::Event1, TestEvent::Event2]);
        assert_eq!(
            result.effects,
            vec![TestEffect::Effect1, TestEffect::Effect2]
        );
    }
}
