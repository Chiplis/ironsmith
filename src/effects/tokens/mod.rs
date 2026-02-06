//! Token creation effects.
//!
//! This module contains effects for creating tokens:
//! - `CreateTokenEffect` - Basic token creation
//! - `CreateTokenCopyEffect` - Create token copies of permanents

mod create_token;
mod create_token_copy;
mod investigate;

pub use create_token::CreateTokenEffect;
pub use create_token_copy::CreateTokenCopyEffect;
pub use investigate::InvestigateEffect;
