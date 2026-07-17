//! Player decision system for MTG.
//!
//! This module provides:
//! - `DecisionMaker` and typed decision contexts for player input
//! - `LegalAction` and related types for describing legal game actions
//! - Helper functions to compute legal actions

use crate::alternative_cast::CastingMethod;
use crate::combat_state::{AttackTarget, CombatState};
use crate::derived_view::DerivedGameView;
use crate::effects::ExecutionContext;
use crate::effects::helpers::resolve_value;
use crate::game_state::{GameState, Phase, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::perf::PerfTimer;
use crate::special_actions::{SpecialAction, can_activate_mana_ability_check_with_view};
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;
use crate::zone::Zone;
use crate::{CounterType, ManaSymbol, Step};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

mod attack_block;
mod io;
mod legal_actions;
mod mana;
mod perf;
mod types;

#[allow(unused_imports)]
use attack_block::*;
#[allow(unused_imports)]
use io::*;
#[allow(unused_imports)]
use legal_actions::*;
pub(crate) use mana::can_pay_mana_cost_with_available_sources;
#[allow(unused_imports)]
use mana::*;
#[allow(unused_imports)]
use perf::*;
#[allow(unused_imports)]
use types::*;

pub use attack_block::*;
pub use io::*;
pub use legal_actions::*;
pub use mana::*;
pub use perf::*;
pub use types::*;

#[cfg(test)]
mod tests;
