#![allow(unused_imports)]

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::effect_text_shared;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, TagKey, Zone};

mod merge_passes;
mod normalize_common;
mod normalize_post_pass;
mod oracle_style;
mod render_effects;
mod render_pipeline;

use self::merge_passes::*;
use self::normalize_common::*;
use self::normalize_post_pass::*;
use self::oracle_style::*;
use self::render_effects::*;
use self::render_pipeline::*;

pub(crate) use self::normalize_common::describe_value;
pub use self::oracle_style::canonical_compiled_lines;
pub use self::oracle_style::oracle_like_lines;
pub use self::render_effects::compile_effect_list;
pub use self::render_pipeline::compiled_lines;
pub use self::render_pipeline::raw_compiled_lines;
