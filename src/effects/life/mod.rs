//! Life-related effect implementations.
//!
//! This module contains effects that modify player life totals:
//! - `GainLifeEffect` - A player gains life
//! - `LoseLifeEffect` - A player loses life
//! - `SetLifeTotalEffect` - Set a player's life to a specific value
//! - `ExchangeLifeTotalsEffect` - Exchange life totals between two players

mod exchange_life_totals;
mod gain_life;
mod lose_life;
mod set_life_total;

pub use exchange_life_totals::ExchangeLifeTotalsEffect;
pub use gain_life::GainLifeEffect;
pub use lose_life::LoseLifeEffect;
pub use set_life_total::SetLifeTotalEffect;
