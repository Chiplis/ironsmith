//! Life gain and loss events and matchers.

mod life_gain;
mod life_loss;
pub mod matchers;

pub use life_gain::LifeGainEvent;
pub use life_loss::LifeLossEvent;
