//! Life-related effect implementations.
//!
//! This module contains effects that modify player life totals:
//! - `GainLifeEffect` - A player gains life
//! - `LoseLifeEffect` - A player loses life
//! - `PayLifeEffect` - A player pays a fixed amount of life
//! - `SetLifeTotalEffect` - Set a player's life to a specific value
//! - `ExchangeLifeTotalsEffect` - Exchange life totals between two players

mod exchange_life_totals;
mod gain_life;
mod lose_life;
mod note_life_total;
mod pay_life;
mod set_life_total;

pub use exchange_life_totals::ExchangeLifeTotalsEffect;
pub use gain_life::GainLifeEffect;
pub use lose_life::LoseLifeEffect;
pub use note_life_total::NoteLifeTotalEffect;
pub use pay_life::PayLifeEffect;
pub use set_life_total::SetLifeTotalEffect;
