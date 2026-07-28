use super::*;
use crate::ability::Ability;
use crate::ability::AbilityKind;
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness, PtValue};
use crate::cards::builders::CardDefinitionBuilder;
use crate::cards::definitions::emrakul_the_promised_end;
use crate::combat_state::{AttackTarget, CombatState};
use crate::decision::{AutoPassDecisionMaker, DecisionMaker, SelectFirstDecisionMaker};
use crate::effect::RestrictionExt as _;
use crate::effect::{Effect, EventValueSpec, Until, Value};
use crate::events::EventKind;
use crate::events::spells::SpellCastEvent;
use crate::events::zones::EnterBattlefieldEvent;
use crate::execute_cleanup_step;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::Phase;
use crate::ids::{CardId, ObjectId};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::ObjectKind;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectRef, PlayerFilter};
use crate::triggers::Trigger;
use crate::triggers::TriggerEvent;
use crate::types::{CardType, Subtype, Supertype};

fn setup_game() -> GameState {
    crate::tests::test_helpers::setup_two_player_game()
}

fn setup_three_player_game() -> GameState {
    GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

mod shard_00;
mod shard_01;
mod shard_02;
mod shard_03;
mod shard_04;
mod shard_05;
mod shard_06;
mod shard_07;
mod shard_08;
mod shard_09;
mod shard_10;
mod shard_11;
mod shard_12;
mod shard_13;
mod shard_14;
mod shard_15;
mod shard_16;
mod shard_17;
mod shard_18;
mod shard_19;
mod valkyries_call;
