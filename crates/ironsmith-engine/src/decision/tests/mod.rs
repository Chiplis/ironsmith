use super::*;
use crate::CardDefinitionBuilder;
use crate::ability::{Ability, AbilityKind};
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::definitions::{basic_forest, basic_island};
use crate::color::ColorSet;
use crate::costs::PaymentReason;
use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{Effect, Value};
use crate::effects::ExecutionContext;
use crate::filter::Comparison;
use crate::grant::Grantable;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn setup_game() -> GameState {
    crate::tests::test_helpers::setup_two_player_game()
}

fn stage_spell_cast_for_test(
    game: &mut GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell_id, caster, from_zone),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_cards_drawn_for_test(game: &mut GameState, player: PlayerId, count: u32) {
    let cards = (0..count).map(|_| game.new_object_id()).collect();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::other::CardsDrawnEvent::new(player, cards, count > 0),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_commit_crime_for_test(game: &mut GameState, player: PlayerId) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::other::KeywordActionEvent::new(
            crate::events::other::KeywordActionKind::CommitCrime,
            player,
            ObjectId::from_raw(0),
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_life_gain_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::LifeGainEvent::new(player, amount),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_life_loss_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::LifeLossEvent::from_effect(player, amount),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

mod actions;
mod costs;
