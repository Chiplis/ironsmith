//! Token creation effects.
//!
//! This module contains effects for creating tokens:
//! - `CreateTokenEffect` - Basic token creation
//! - `CreateTokenCopyEffect` - Create token copies of permanents

mod amass;
mod empower_jace;
mod create_token;
mod create_token_copy;
mod incubate;
mod investigate;
mod lifecycle;

pub use amass::AmassEffect;
pub use empower_jace::EmpowerJaceEffect;
pub use create_token::CreateTokenEffect;
pub(crate) use create_token::materialize_named_creator_source_in_token;
pub use create_token_copy::{
    CopyAttackTargetMode, CreateTokenCopyEffect, TokenCopyReferenceSurface,
};
pub use incubate::IncubateEffect;
pub use investigate::InvestigateEffect;

pub(crate) mod resources;
pub use resources::TokenCreationLimits;

pub(crate) use lifecycle::{execute_token_instruction_atomically, execute_resource_transaction_atomically};
