use super::WasmReplayDecisionMaker;
use super::ui_snapshot::grouped_battlefield_for_player;
use super::{
    ActiveViewedCards, CustomCardFaceInput, CustomCardInput, CustomCardLayoutInput, GameSnapshot,
    HiddenObjectRef, MatchFormatInput, MatchSetupInput, PendingReplayAction, PregameState,
    ReplayOutcome, ReplayRoot, TargetChoiceView, TargetInput, WasmGame, action_drag_metadata,
    build_object_details_snapshot, build_stack_object_snapshot, convert_and_validate_targets,
    normalize_select_object_choice_ids, stable_ids_for_viewed_cards,
};
use crate::colors_for_context;
use ironsmith::ability::Ability;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardRegistry;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::continuous::ContinuousEffect;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::cost::OptionalCostsPaid;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, PriorityContext, SelectObjectsContext,
    SelectableObject, SelectableOption, TargetRequirementContext, TargetsContext, ViewCardsContext,
};
use ironsmith::effect::{Effect, Until};
use ironsmith::events::spells::SpellCastEvent;
use ironsmith::game_loop::{CastStage, PendingCast, PendingManaAbility, PriorityResponse};
use ironsmith::game_state::{
    GameState, Phase, PlayerControlDuration, PlayerControlStart, StackEntry, Step, Target,
};
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::provenance::ProvNodeId;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::{Trigger, TriggerEvent, check_triggers};
use ironsmith::types::{CardType, Subtype, Supertype};
use ironsmith::zone::Zone;
use ironsmith_registry::cards::definitions::{
    basic_island, basic_mountain, blood_artist, culling_the_weak, emrakul_the_promised_end,
    gemstone_caverns, grizzly_bears, lightning_bolt, ornithopter, phyrexian_tower, polluted_delta,
    serum_powder, stoke_the_flames, urzas_saga, yawgmoth_thran_physician,
};
use ironsmith_registry::compile_to_runtime_definition;
use serde::Deserialize;
use serde_json::json;

fn setup_two_player_game() -> GameState {
    GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}

fn setup_pregame_match(format: MatchFormatInput) -> WasmGame {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.match_format = format;
    wasm
}

fn seed_filler_cards(
    wasm: &mut WasmGame,
    player: PlayerId,
    zone: Zone,
    count: usize,
) -> Vec<ObjectId> {
    (0..count)
        .map(|_| {
            wasm.game
                .create_object_from_definition(&ornithopter(), player, zone)
        })
        .collect()
}

mod shard_00;
mod shard_01;
mod shard_02;
