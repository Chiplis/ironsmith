//! Control-related effects.
//!
//! This module contains effects that change control of permanents.

mod exchange_control;
mod gain_control;

pub use exchange_control::ExchangeControlEffect;
pub use exchange_control::SharedTypeConstraint;
pub use gain_control::GainControlEffect;
