# reactive-state [![crates.io badge](https://img.shields.io/crates/v/reactive-state.svg)](https://crates.io/crates/reactive-state) [![docs.rs badge](https://docs.rs/reactive-state/badge.svg)](https://docs.rs/reactive-state/) [![license badge](https://img.shields.io/github/license/kellpossible/reactive-state)](https://github.com/kellpossible/reactive-state/blob/master/LICENSE.txt) [![github action badge](https://github.com/kellpossible/reactive-state/workflows/Rust/badge.svg)](https://github.com/kellpossible/reactive-state/actions?query=workflow%3ARust)

This library is inspired by [redux](https://redux.js.org/), and designed to be
used within Rust GUI applications to have centralised global state which behaves
in a predictable way.

The behaviour of the system is customisable via middleware, and provided in this
library are a couple of examples, a simple logger, and a web based logger
inspired by [redux-logger](https://github.com/LogRocket/redux-logger).

![web_logger](./screenshots/yew_state_20200601.png)
*Web Logger Middleware*