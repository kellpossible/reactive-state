//! This library is inspired by [redux](https://redux.js.org/), and
//! designed to be used within Rust GUI applications to manage
//! centralised global state which behaves in a predictable way.
//!
//! ## Example
//!
//! The following is a simple example of how to use this system to
//! manage state, and subscribe to changes to state.
//!
//! ```
//! use reactive_state::{StoreEvent, ReducerFn, ReducerResult, Store, Callback};
//! use std::{cell::RefCell, rc::Rc};
//!
//! /// Something to hold the application state.
//! #[derive(Clone)]
//! struct MyState {
//!     pub variable: u32
//! }
//!
//! /// Some actions to perform which alter the state.
//! enum MyAction {
//!     Increment,
//!     Decrement
//! }
//!
//! /// Some events that are fired during certain action/state combinations.
//! #[derive(Clone, Eq, PartialEq, Hash)]
//! enum MyEvent {
//!     IsOne,
//!     None
//! }
//!
//! /// Ensure that the event is compatible with Store.
//! impl StoreEvent for MyEvent {
//!     fn none() -> Self {
//!         Self::None
//!     }
//!     fn is_none(&self) -> bool {
//!         if let Self::None = self {
//!             true
//!         } else {
//!             false
//!         }
//!     }
//! }
//!
//! /// A reducer to perform the actions, alter the state, and fire off events.
//! let reducer: ReducerFn<MyState, MyAction, MyEvent, ()> = |state, action| {
//!     let mut events: Vec<MyEvent> = Vec::new();
//!
//!     let new_state = match action {
//!         MyAction::Increment => {
//!             let mut new_state = MyState::clone(state);
//!             new_state.variable = state.variable + 1;
//!             Rc::new(new_state)
//!         }
//!         MyAction::Decrement => {
//!             let mut new_state = MyState::clone(state);
//!             new_state.variable = state.variable - 1;
//!             Rc::new(new_state)
//!         }
//!     };
//!
//!     if new_state.variable == 1 {
//!         events.push(MyEvent::IsOne);
//!     }
//!
//!     ReducerResult {
//!         state: new_state,
//!         events,
//!         effects: vec![],
//!     }
//! };
//!
//! // Set the initial state.
//! let initial_state = MyState {
//!     variable: 0u32
//! };
//!
//! // Create the store.
//! let store = Store::new(reducer, initial_state);
//!
//! // A test variable that will be altered by the callback.
//! let callback_invokes: Rc<RefCell<u32>> = Rc::new(RefCell::new(0u32));
//! let callback_invokes_local = callback_invokes.clone();
//!
//! let callback = Callback::new(move |_state: Rc<MyState>, _event: MyEvent| {
//!     *(callback_invokes_local.borrow_mut()) += 1;
//! });
//!
//! // Subscribe to state changes which produce the IsOne event.
//! store.subscribe_event(&callback, MyEvent::IsOne);
//!
//! assert_eq!(0, store.state().variable);
//! assert_eq!(0, *RefCell::borrow(&callback_invokes));
//!
//! // Dispatch an increment action onto the store, which will
//! // alter the state.
//! store.dispatch(MyAction::Increment);
//!
//! // The state has been altered.
//! assert_eq!(1, store.state().variable);
//!
//! // The callback was invoked.
//! assert_eq!(1, *RefCell::borrow(&callback_invokes));
//!
//! store.dispatch(MyAction::Increment);
//!
//! // The state has been altered.
//! assert_eq!(2, store.state().variable);
//!
//! // The callback was not invoked, because the event IsOne
//! // was not fired by the reducer.
//! assert_eq!(1, *RefCell::borrow(&callback_invokes));
//!
//! // Drop the callback, and it will also be removed from the store.
//! drop(callback);
//!
//! store.dispatch(MyAction::Decrement);
//!
//! // The state has been altered again.
//! assert_eq!(1, store.state().variable);
//!
//! // The callback was dropped before the action was dispatched,
//! // and so it was not invoked.
//! assert_eq!(1, *RefCell::borrow(&callback_invokes));
//! ```
//!
//! ## Side Effects
//!
//! Something that wasn't covered in the example above, was the
//! concept of side effects produced in the reducer. This is the
//! fourth type parameter `Effect` on [Store](Store), and effects
//! which are produced in the reducer are given to the store via the
//! [ReducerResult](ReducerResult) that it returns. Side effects are
//! designed to be executed/handled by store [middleware](middleware).
//!
//! ## Optional Features
//!
//! The following optional crate features can be enabled:
//!
//! + `"simple_logger"` - Logging middleware in the
//!   [simple_logger](crate::middleware::simple_logger) module which
//!   uses the `log` macros.
//! + `"web_logger"` - Logging middleware in the
//!   [web_logger](crate::middleware::web_logger) module, for
//!   applications running in the browser using
//!   [wasm-bindgen](https://crates.io/crates/wasm-bindgen).
//! + `"yew"` - Support for compatibility trait implementations on
//!   [yew](https://crates.io/crates/yew) types.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod event;
mod listener;
pub mod middleware;
mod reducer;
mod store;

#[cfg(feature = "yew")]
#[cfg_attr(docsrs, doc(cfg(feature = "yew")))]
pub mod provider;

pub use event::*;
pub use listener::*;
pub use reducer::*;
pub use store::{Store, StoreRef};
