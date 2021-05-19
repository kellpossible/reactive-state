//! Logging [Middleware](crate::middleware::Middleware) for
//! applications running in the browser using `wasm-bindgen`.
//! Publishes actions/events that occur within the
//! [Store](crate::Store).

use super::{Middleware, ReduceMiddlewareResult};
use serde::Serialize;
use std::{fmt::Display, hash::Hash};
use wasm_bindgen::JsValue;
use web_sys::console;

pub enum LogLevel {
    Trace,
    Debug,
    Warn,
    Info,
    Log,
}

impl LogLevel {
    pub fn log_1(&self, message: &JsValue) {
        #[allow(unused_unsafe)]
        unsafe {
            match self {
                LogLevel::Trace => console::trace_1(message),
                LogLevel::Debug => console::debug_1(message),
                LogLevel::Warn => console::warn_1(message),
                LogLevel::Info => console::info_1(message),
                LogLevel::Log => console::log_1(message),
            }
        }
    }

    pub fn log(&self, messages: Vec<JsValue>) {
        let messages_array = js_sys::Array::new_with_length(messages.len() as u32);

        for (i, m) in messages.into_iter().enumerate() {
            messages_array.set(i as u32, m);
        }

        #[allow(unused_unsafe)]
        unsafe {
            match self {
                LogLevel::Trace => console::trace(&messages_array),
                LogLevel::Debug => console::debug(&messages_array),
                LogLevel::Warn => console::warn(&messages_array),
                LogLevel::Info => console::info(&messages_array),
                LogLevel::Log => console::log(&messages_array),
            }
        }
    }
}

pub enum DisplayType {
    /// Print using the browser's log groups. Unfortunately this isn't
    /// always very consistent, especially with
    /// asynchronous/concurrent events.
    Groups,
    /// Print the data in a single javascript object tree.
    SingleObject,
}

impl Default for DisplayType {
    fn default() -> Self {
        Self::Groups
    }
}

#[derive(Serialize)]
struct OnReduceLog<'a, State, Action, Effect> {
    action: &'a Option<Action>,
    prev_state: &'a State,
    next_state: &'a State,
    effects: &'a [Effect],
}

#[derive(Serialize)]
struct OnNotifyLog<'a, State, Event> {
    state: &'a State,
    events: &'a [Event],
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Log
    }
}

/// Logging middleware for applications running in the browser.
///
/// See [web_logger](super::web_logger) for more details.
pub struct WebLoggerMiddleware {
    log_level: LogLevel,
    display_type: DisplayType,
}

impl WebLoggerMiddleware {
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::default(),
            display_type: DisplayType::default(),
        }
    }

    /// Set the level at which the data from this middleware will be
    /// logged to.
    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    /// What type of display to use when printing the data from this
    /// middleware.
    pub fn display_type(mut self, display_type: DisplayType) -> Self {
        self.display_type = display_type;
        self
    }

    fn on_reduce_groups<State, Action, Event, Effect>(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        action: Option<&Action>,
        reduce: super::ReduceFn<State, Action, Event, Effect>,
    ) -> ReduceMiddlewareResult<Event, Effect>
    where
        State: Serialize,
        Action: Serialize + Display,
        Event: Clone + Hash + Eq + Serialize,
        Effect: Serialize,
    {
        let prev_state_js = JsValue::from_serde(&(*store.state())).unwrap();

        let action_js = JsValue::from_serde(&action).unwrap();
        let action_display = match &action {
            Some(action) => format!("{}", action),
            None => "None".to_string(),
        };

        let result = reduce(store, action);
        let next_state_js = JsValue::from_serde(&(*store.state())).unwrap();

        let effects_js = JsValue::from_serde(&result.effects).unwrap();
        let effects_display = match &result.effects.len() {
            0 => "None".to_string(),
            _ => format!("({})", result.effects.len()),
        };

        #[allow(unused_unsafe)]
        unsafe {
            console::group_collapsed_3(
                &JsValue::from_serde(&format!("%caction %c{}", action_display)).unwrap(),
                &JsValue::from_str("color: gray; font-weight: lighter;"),
                &JsValue::from_str("inherit"),
            );
            console::group_collapsed_2(
                &JsValue::from_str("%cprev state"),
                &JsValue::from_str("color: #9E9E9E; font-weight: bold;"),
            );
        }

        self.log_level.log_1(&prev_state_js);

        #[allow(unused_unsafe)]
        unsafe {
            console::group_end();

            console::group_collapsed_3(
                &JsValue::from_str(&format!("%caction: %c{}", action_display)),
                &JsValue::from_str("color: #03A9F4; font-weight: bold;"),
                &JsValue::from_str("color: gray; font-weight: lighter;"),
            );
        }

        self.log_level.log_1(&action_js);

        #[allow(unused_unsafe)]
        unsafe {
            console::group_end();

            console::group_collapsed_2(
                &JsValue::from_str("%cnext state"),
                &JsValue::from_str("color: #4CAF50; font-weight: bold;"),
            );
        }

        self.log_level.log_1(&next_state_js);

        #[allow(unused_unsafe)]
        unsafe {
            console::group_end();

            console::group_collapsed_3(
                &JsValue::from_str(&format!("%ceffects: %c{}", effects_display)),
                &JsValue::from_str("color: #C210C2; font-weight: bold;"),
                &JsValue::from_str("color: gray; font-weight: lighter;"),
            );
        }
        self.log_level.log_1(&effects_js);

        #[allow(unused_unsafe)]
        unsafe {
            console::group_end();
        }

        result
    }

    fn on_reduce_no_groups<State, Action, Event, Effect>(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        action: Option<&Action>,
        reduce: super::ReduceFn<State, Action, Event, Effect>,
    ) -> ReduceMiddlewareResult<Event, Effect>
    where
        State: Serialize,
        Action: Serialize + Display,
        Event: Clone + Hash + Eq + Serialize,
        Effect: Serialize,
    {
        let action_display = format!(
            "on_reduce(), action: {}",
            match &action {
                Some(action) => format!("{}", action),
                None => "None".to_string(),
            }
        );

        let action_display_js = JsValue::from_str(&action_display);

        let prev_state = store.state();

        let result = reduce(store, action);
        let next_state = store.state();

        let log_object = OnReduceLog {
            action: &action,
            prev_state: &*prev_state,
            next_state: &*next_state,
            effects: &result.effects,
        };

        let log_object_js = JsValue::from_serde(&log_object).unwrap();
        self.log_level.log(vec![action_display_js, log_object_js]);

        result
    }

    fn on_notify_groups<State, Action, Event, Effect>(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        events: Vec<Event>,
        notify: super::NotifyFn<State, Action, Event, Effect>,
    ) -> Vec<Event>
    where
        Event: Serialize,
    {
        let events_js = JsValue::from_serde(&events).unwrap();
        let events_display = match events.len() {
            0 => "None".to_string(),
            _ => format!("({})", events.len()),
        };

        #[allow(unused_unsafe)]
        unsafe {
            console::group_collapsed_3(
                &JsValue::from_str(&format!("%cevents: %c{}", events_display)),
                &JsValue::from_str("color: #FCBA03; font-weight: bold;"),
                &JsValue::from_str("color: gray; font-weight: lighter;"),
            );
        }

        self.log_level.log_1(&events_js);

        #[allow(unused_unsafe)]
        unsafe {
            console::group_end();
            console::group_end();
        }

        notify(store, events)
    }

    fn on_notify_no_groups<State, Action, Event, Effect>(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        events: Vec<Event>,
        notify: super::NotifyFn<State, Action, Event, Effect>,
    ) -> Vec<Event>
    where
        Event: Serialize + Clone + Hash + Eq,
        State: Serialize,
    {
        let log_object = OnNotifyLog {
            state: &*store.state(),
            events: &events,
        };

        let log_object_js = JsValue::from_serde(&log_object).unwrap();

        let display = JsValue::from_str("on_notify(): ");

        self.log_level.log(vec![display, log_object_js]);

        notify(store, events)
    }
}

impl Default for WebLoggerMiddleware {
    fn default() -> Self {
        WebLoggerMiddleware::new()
    }
}

impl<State, Action, Event, Effect> Middleware<State, Action, Event, Effect> for WebLoggerMiddleware
where
    State: Serialize,
    Action: Serialize + Display,
    Event: Clone + Hash + Eq + Serialize,
    Effect: Serialize,
{
    fn on_reduce(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        action: Option<&Action>,
        reduce: super::ReduceFn<State, Action, Event, Effect>,
    ) -> ReduceMiddlewareResult<Event, Effect> {
        match self.display_type {
            DisplayType::Groups => self.on_reduce_groups(store, action, reduce),
            DisplayType::SingleObject => self.on_reduce_no_groups(store, action, reduce),
        }
    }

    fn process_effect(
        &self,
        _store: &crate::Store<State, Action, Event, Effect>,
        effect: Effect,
    ) -> Option<Effect> {
        Some(effect)
    }

    fn on_notify(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        events: Vec<Event>,
        notify: super::NotifyFn<State, Action, Event, Effect>,
    ) -> Vec<Event> {
        match self.display_type {
            DisplayType::Groups => self.on_notify_groups(store, events, notify),
            DisplayType::SingleObject => self.on_notify_no_groups(store, events, notify),
        }
    }
}
