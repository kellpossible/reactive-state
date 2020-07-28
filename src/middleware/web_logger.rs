use super::{Middleware, ReduceMiddlewareResult};
use crate::StoreEvent;
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
    pub fn log(&self, message: &JsValue) {
        match self {
            LogLevel::Trace => console::trace_1(message),
            LogLevel::Debug => console::debug_1(message),
            LogLevel::Warn => console::warn_1(message),
            LogLevel::Info => console::info_1(message),
            LogLevel::Log => console::log_1(message),
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Log
    }
}

/// Aiming to be something similar to https://github.com/LogRocket/redux-logger
pub struct WebLoggerMiddleware {
    log_level: LogLevel,
}

impl WebLoggerMiddleware {
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::default(),
        }
    }

    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }
}

impl<State, Action, Event, Effect> Middleware<State, Action, Event, Effect> for WebLoggerMiddleware
where
    State: Serialize,
    Action: Serialize + Display,
    Event: StoreEvent + Clone + Hash + Eq + Serialize,
    Effect: Serialize,
{
    fn on_reduce(
        &self,
        store: &crate::Store<State, Action, Event, Effect>,
        action: Option<&Action>,
        reduce: super::ReduceFn<State, Action, Event, Effect>,
    ) -> ReduceMiddlewareResult<Event, Effect> {
        let prev_state_js = JsValue::from_serde(&(*store.state())).unwrap();

        // TODO: what will happen when action is None?
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

        console::group_collapsed_3(
            &JsValue::from_serde(&format!("%caction %c{}", action_display)).unwrap(),
            &JsValue::from_str("color: gray; font-weight: lighter;"),
            &JsValue::from_str("inherit"),
        );
        console::group_collapsed_2(
            &JsValue::from_str("%cprev state"),
            &JsValue::from_str("color: #9E9E9E; font-weight: bold;"),
        );
        self.log_level.log(&prev_state_js);
        console::group_end();

        console::group_collapsed_3(
            &JsValue::from_str(&format!("%caction: %c{}", action_display)),
            &JsValue::from_str("color: #03A9F4; font-weight: bold;"),
            &JsValue::from_str("color: gray; font-weight: lighter;"),
        );
        self.log_level.log(&action_js);
        console::group_end();

        console::group_collapsed_2(
            &JsValue::from_str("%cnext state"),
            &JsValue::from_str("color: #4CAF50; font-weight: bold;"),
        );
        self.log_level.log(&next_state_js);
        console::group_end();

        console::group_collapsed_3(
            &JsValue::from_str(&format!("%ceffects: %c{}", effects_display)),
            &JsValue::from_str("color: #C210C2; font-weight: bold;"),
            &JsValue::from_str("color: gray; font-weight: lighter;"),
        );
        self.log_level.log(&effects_js);
        console::group_end();

        result
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
        let events_js = JsValue::from_serde(&events).unwrap();
        let events_display = match events.len() {
            0 => "None".to_string(),
            _ => format!("({})", events.len()),
        };
        console::group_collapsed_3(
            &JsValue::from_str(&format!("%cevents: %c{}", events_display)),
            &JsValue::from_str("color: #FCBA03; font-weight: bold;"),
            &JsValue::from_str("color: gray; font-weight: lighter;"),
        );
        self.log_level.log(&events_js);
        console::group_end();
        console::group_end();
        notify(store, events)
    }
}
