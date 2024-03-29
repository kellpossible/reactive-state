use std::rc::{Rc, Weak};

/// A trait to take a [Callback] or other custom callback type and
/// produce a [Listener], a weak reference to that callback.
pub trait AsListener<State, Event> {
    /// Produce a [Listener], a weak reference to this callback.
    fn as_listener(&self) -> Listener<State, Event>;
}

/// A weak reference to a callback function (usually [Callback]) which
/// is notified of changes to [Store](crate::Store) `State`, and
/// `Event`s produced by the store.
///
/// See [Callback](Callback) for more information about how this is
/// typically used.
#[derive(Clone)]
pub struct Listener<State, Event>(Weak<dyn Fn(Rc<State>, Option<Event>)>);

impl<State, Event> Listener<State, Event> {
    /// Attempt to upgrade the weak reference in this listener to a
    /// [Callback], otherwise if unable to, returns `None`.
    pub fn as_callback(&self) -> Option<Callback<State, Event>> {
        match self.0.upgrade() {
            Some(listener_rc) => Some(Callback(listener_rc)),
            None => None,
        }
    }
}

impl<State, Event> AsListener<State, Event> for Listener<State, Event> {
    fn as_listener(&self) -> Listener<State, Event> {
        Listener(self.0.clone())
    }
}

/// A wrapper for a callback which is notified of changes to
/// [Store](crate::Store) `State`, and `Event`s produced by the store.
///
/// ## Example
///
/// The following example makes use of the [AsListener](AsListener)
/// trait implementation for `Callback` which allows it to be used in
/// [Store::subscribe()](crate::Store::subscribe). The
/// [AsListener](AsListener) trait creates a weak reference to this
/// callback in a [Listener](Listener), which is given to the
/// [Store](crate::Store). When the callback is dropped, the listener will be
/// removed from the store.
///
/// ```
/// # use reactive_state::{ReducerFn, Store, ReducerResult};
/// # let reducer: ReducerFn<(), (), (), ()> = |_state, _action| { ReducerResult::default() };
/// # let store = Store::new(reducer, ());
/// use reactive_state::Callback;
///
/// let callback = Callback::new(|_state, _event| {
///     println!("Callback invoked");
/// });
///
/// store.subscribe(&callback);
/// ```
///
/// ## Optional Features
///
/// If the `"yew"` crate feature is enabled, a number of `From`
/// implementations are available to convert `yew` callbacks into
/// this:
///
/// + `From<yew::Callback<Rc<State>>>`
/// + `From<yew::Callback<(Rc<State>, Event)>>`
/// + `From<yew::Callback<()>>`
#[derive(Clone)]
pub struct Callback<State, Event>(Rc<dyn Fn(Rc<State>, Option<Event>)>);

impl<State, Event> AsListener<State, Event> for &Callback<State, Event> {
    fn as_listener(&self) -> Listener<State, Event> {
        Listener(Rc::downgrade(&self.0))
    }
}

impl<State, Event> Callback<State, Event> {
    pub fn new<C: Fn(Rc<State>, Option<Event>) + 'static>(closure: C) -> Self {
        Callback(Rc::new(closure))
    }
    pub fn emit(&self, state: Rc<State>, event: Option<Event>) {
        (self.0)(state, event)
    }
}

impl<C, State, Event> From<C> for Callback<State, Event>
where
    C: Fn(Rc<State>, Option<Event>) + 'static,
{
    fn from(closure: C) -> Self {
        Callback(Rc::new(closure))
    }
}

#[cfg(feature = "yew")]
#[cfg_attr(docsrs, doc(cfg(feature = "yew")))]
impl<State, Event> From<yew::Callback<Rc<State>>> for Callback<State, Event>
where
    State: 'static,
    Event: 'static,
{
    fn from(yew_callback: yew::Callback<Rc<State>>) -> Self {
        Callback(Rc::new(move |state, _| {
            yew_callback.emit(state);
        }))
    }
}

#[cfg(feature = "yew")]
#[cfg_attr(docsrs, doc(cfg(feature = "yew")))]
impl<State, Event> From<yew::Callback<(Rc<State>, Option<Event>)>> for Callback<State, Event>
where
    State: 'static,
    Event: 'static,
{
    fn from(yew_callback: yew::Callback<(Rc<State>, Option<Event>)>) -> Self {
        Callback(Rc::new(move |state, event| {
            yew_callback.emit((state, event));
        }))
    }
}

#[cfg(feature = "yew")]
#[cfg_attr(docsrs, doc(cfg(feature = "yew")))]
impl<State, Event> From<yew::Callback<()>> for Callback<State, Event>
where
    State: 'static,
    Event: 'static,
{
    fn from(yew_callback: yew::Callback<()>) -> Self {
        Callback(Rc::new(move |_, _| {
            yew_callback.emit(());
        }))
    }
}
