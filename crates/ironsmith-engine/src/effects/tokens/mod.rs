//! Token creation effects.
//!
//! This module contains effects for creating tokens:
//! - `CreateTokenEffect` - Basic token creation
//! - `CreateTokenCopyEffect` - Create token copies of permanents

mod amass;
mod create_token;
mod create_token_copy;
mod empower_jace;
mod incubate;
mod investigate;
mod lifecycle;

pub use amass::AmassEffect;
pub use create_token::CreateTokenEffect;
pub(crate) use create_token::materialize_named_creator_source_in_token;
pub use create_token_copy::{
    CopyAttackTargetMode, CreateTokenCopyEffect, TokenCopyReferenceSurface,
};
pub use empower_jace::EmpowerJaceEffect;
pub use incubate::IncubateEffect;
pub use investigate::InvestigateEffect;

pub(crate) mod resources;
pub use resources::TokenCreationLimits;

pub(crate) use lifecycle::{
    execute_resource_transaction_atomically, execute_token_instruction_atomically,
};
pub(crate) use lifecycle::{
    execute_resource_transaction_with_pending_value, execute_token_instruction_with_pending_value,
};

pub(crate) use create_token::{
    create_tokens_with_entry_counters_with_outputs, multiplied_token_instruction,
};
