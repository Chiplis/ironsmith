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
use ironsmith::types::{CardType, Subtype};
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

#[test]
fn validate_match_setup_accepts_loadable_normal_decks() {
    let mut wasm = WasmGame::new();
    let config = MatchSetupInput {
        player_names: vec!["Alice".to_string(), "Bob".to_string()],
        starting_life: 20,
        seed: 1,
        format: MatchFormatInput::Normal,
        decks: Some(vec![
            vec!["Lightning Bolt".to_string()],
            vec!["Ornithopter".to_string()],
        ]),
        sideboards: None,
        commanders: None,
        opening_hand_size: Some(7),
    };

    let validation = wasm
        .validate_match_setup_input(&config)
        .expect("validation should succeed");
    assert!(validation.valid, "loadable normal decks should validate");
    assert!(
        validation.issues.is_empty(),
        "valid decks should not surface issues: {:?}",
        validation.issues
    );
}

#[test]
fn start_match_loads_sideboards_outside_the_game() {
    let mut wasm = WasmGame::new();
    let config = MatchSetupInput {
        player_names: vec!["Alice".to_string(), "Bob".to_string()],
        starting_life: 20,
        seed: 1,
        format: MatchFormatInput::Normal,
        decks: Some(vec![
            vec!["Island".to_string(); 60],
            vec!["Mountain".to_string(); 60],
        ]),
        sideboards: Some(vec![
            vec!["Ornithopter".to_string(), "Lightning Bolt".to_string()],
            vec!["Grizzly Bears".to_string()],
        ]),
        commanders: None,
        opening_hand_size: Some(0),
    };

    wasm.start_match(serde_wasm_bindgen::to_value(&config).expect("config should encode"))
        .expect("match should start with sideboards");

    let alice = wasm
        .game
        .player(PlayerId::from_index(0))
        .expect("alice should exist");
    assert_eq!(alice.sideboard.len(), 2);
    assert!(
        alice
            .sideboard
            .iter()
            .filter_map(|id| wasm.game.object(*id))
            .all(|object| object.zone == Zone::OutsideGame)
    );
}

#[test]
fn validate_match_setup_reports_invalid_cards() {
    let mut wasm = WasmGame::new();
    let config = MatchSetupInput {
        player_names: vec!["Alice".to_string(), "Bob".to_string()],
        starting_life: 20,
        seed: 1,
        format: MatchFormatInput::Normal,
        decks: Some(vec![
            vec!["Definitely Not A Real Card".to_string()],
            vec!["Ornithopter".to_string()],
        ]),
        sideboards: None,
        commanders: None,
        opening_hand_size: Some(7),
    };

    let validation = wasm
        .validate_match_setup_input(&config)
        .expect("validation should succeed");
    assert!(
        !validation.valid,
        "invalid decks should block match start validation"
    );
    assert_eq!(
        validation.issues.len(),
        1,
        "expected one invalid card issue"
    );
    let issue = &validation.issues[0];
    assert_eq!(issue.player_index, 0);
    assert_eq!(issue.player_name, "Alice");
    assert_eq!(issue.section, "deck");
    assert_eq!(issue.card_name, "Definitely Not A Real Card");
    assert!(
        !issue.error.is_empty(),
        "invalid card should surface a specific error"
    );
}

#[test]
fn test_action_drag_metadata_links_suspend_special_action_to_card_and_exile() {
    let action = LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::Suspend {
        card_id: ObjectId::from_raw(42),
    });

    let (kind, object_id, ability_index, from_zone, to_zone) = action_drag_metadata(&action);

    assert_eq!(kind, "special_action");
    assert_eq!(object_id, Some(42));
    assert_eq!(ability_index, None);
    assert_eq!(from_zone.as_deref(), Some("hand"));
    assert_eq!(to_zone.as_deref(), Some("exile"));
}

fn custom_face(
    name: &str,
    card_types: &[&str],
    oracle_text: &str,
    power: Option<&str>,
    toughness: Option<&str>,
) -> CustomCardFaceInput {
    CustomCardFaceInput {
        name: name.to_string(),
        mana_cost: None,
        color_indicator: Vec::new(),
        supertypes: Vec::new(),
        card_types: card_types
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        subtypes: Vec::new(),
        oracle_text: oracle_text.to_string(),
        power: power.map(str::to_string),
        toughness: toughness.map(str::to_string),
        loyalty: None,
        defense: None,
    }
}

fn start_pregame(wasm: &mut WasmGame, opening_hand_size: usize, format: MatchFormatInput) {
    wasm.pregame = Some(PregameState::new(
        &wasm.game.turn_store.turn_order,
        opening_hand_size,
        format,
    ));
    wasm.advance_until_decision()
        .expect("pregame should produce a decision");
}

fn dispatch_matching_priority_action<F>(wasm: &mut WasmGame, predicate: F)
where
    F: FnMut(&LegalAction) -> bool,
{
    let index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx
            .actions
            .iter()
            .position(predicate)
            .expect("expected matching priority action"),
        other => panic!("expected priority decision, got {other:?}"),
    };
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": index,
        }))
        .expect("priority action should serialize"),
    )
    .expect("priority action should succeed");
}

fn snapshot_priority_action_label(wasm: &mut WasmGame, action_ref_kind: &str) -> String {
    let snapshot_json = wasm.snapshot_json().expect("snapshot should render");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");
    let actions = snapshot["decision"]["actions"]
        .as_array()
        .expect("priority decision should expose actions");
    actions
        .iter()
        .find(|action| action["action_ref"]["kind"].as_str() == Some(action_ref_kind))
        .and_then(|action| action["label"].as_str())
        .unwrap_or_else(|| panic!("missing priority action label for {action_ref_kind}"))
        .to_string()
}

fn dispatch_select_objects(wasm: &mut WasmGame, object_ids: &[u64]) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": object_ids,
        }))
        .expect("select_objects should serialize"),
    )
    .expect("select_objects should succeed");
}

fn dispatch_select_options(wasm: &mut WasmGame, option_indices: &[usize]) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": option_indices,
        }))
        .expect("select_options should serialize"),
    )
    .expect("select_options should succeed");
}

fn dispatch_select_target_object(wasm: &mut WasmGame, object_id: ObjectId) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "object", "object": object_id.0 },
            ],
        }))
        .expect("select_targets should serialize"),
    )
    .expect("select_targets should succeed");
}

fn dispatch_select_target_player(wasm: &mut WasmGame, player: PlayerId) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "player", "player": player.0 },
            ],
        }))
        .expect("select_targets should serialize"),
    )
    .expect("select_targets should succeed");
}

fn dispatch_pass_priority(wasm: &mut WasmGame) {
    dispatch_matching_priority_action(wasm, |action| matches!(action, LegalAction::PassPriority));
}

#[test]
fn battlefield_lane_prefers_artifact_over_land() {
    let artifact_land = CardBuilder::new(CardId::from_raw(70_100), "Seat of the Synod")
        .card_types(vec![CardType::Artifact, CardType::Land])
        .build();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let protected_ids = std::collections::HashSet::new();
    let object_id = game.create_object_from_card(&artifact_land, alice, Zone::Battlefield);
    let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
    let permanent = battlefield
        .iter()
        .find(|permanent| permanent.id == object_id.0)
        .expect("artifact land should exist in battlefield snapshot");

    assert_eq!(permanent.lane, "artifacts");
}

#[test]
fn battlefield_lane_prefers_creature_over_artifact() {
    let artifact_creature = CardBuilder::new(CardId::from_raw(70_103), "Ornithopter")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 2))
        .build();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let protected_ids = std::collections::HashSet::new();
    let object_id = game.create_object_from_card(&artifact_creature, alice, Zone::Battlefield);
    let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
    let permanent = battlefield
        .iter()
        .find(|permanent| permanent.id == object_id.0)
        .expect("artifact creature should exist in battlefield snapshot");

    assert_eq!(permanent.lane, "creatures");
}

#[test]
fn battlefield_lane_prefers_enchantment_over_creature_and_sorts_after_creatures() {
    let creature = CardBuilder::new(CardId::from_raw(70_101), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let enchantment_creature = CardBuilder::new(CardId::from_raw(70_102), "Nyxborn Wolf")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 1))
        .build();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let protected_ids = std::collections::HashSet::new();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let enchantment_creature_id =
        game.create_object_from_card(&enchantment_creature, alice, Zone::Battlefield);

    let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
    let enchantment_permanent = battlefield
        .iter()
        .find(|permanent| permanent.id == enchantment_creature_id.0)
        .expect("enchantment creature should exist in battlefield snapshot");
    let ordered_ids: Vec<u64> = battlefield.iter().map(|permanent| permanent.id).collect();

    assert_eq!(enchantment_permanent.lane, "enchantments");
    assert_eq!(
        ordered_ids,
        vec![creature_id.0, enchantment_creature_id.0],
        "creatures should render before enchantments"
    );
}

#[test]
fn convert_and_validate_targets_rejects_wrong_requirement_order() {
    let first = Target::Object(ObjectId::from_raw(1));
    let second = Target::Object(ObjectId::from_raw(2));
    let ctx = TargetsContext::new(
        PlayerId::from_index(0),
        ObjectId::from_raw(99),
        "test spell",
        vec![
            TargetRequirementContext {
                description: "first target".to_string(),
                legal_targets: vec![first],
                min_targets: 1,
                max_targets: Some(1),
            },
            TargetRequirementContext {
                description: "second target".to_string(),
                legal_targets: vec![second],
                min_targets: 1,
                max_targets: Some(1),
            },
        ],
    );

    let err = convert_and_validate_targets(
        &ctx,
        vec![
            TargetInput::Object { object: 2 },
            TargetInput::Object { object: 1 },
        ],
    )
    .expect_err("reversed targets should be rejected");

    assert_eq!(
        err,
        "targets do not satisfy the targeting requirements in order"
    );
}

#[test]
fn convert_and_validate_targets_accepts_unbounded_then_fixed_sequence() {
    let a = Target::Object(ObjectId::from_raw(1));
    let b = Target::Object(ObjectId::from_raw(2));
    let c = Target::Object(ObjectId::from_raw(3));
    let ctx = TargetsContext::new(
        PlayerId::from_index(0),
        ObjectId::from_raw(99),
        "test spell",
        vec![
            TargetRequirementContext {
                description: "any number".to_string(),
                legal_targets: vec![a, b],
                min_targets: 0,
                max_targets: None,
            },
            TargetRequirementContext {
                description: "last target".to_string(),
                legal_targets: vec![c],
                min_targets: 1,
                max_targets: Some(1),
            },
        ],
    );

    let converted = convert_and_validate_targets(
        &ctx,
        vec![
            TargetInput::Object { object: 1 },
            TargetInput::Object { object: 2 },
            TargetInput::Object { object: 3 },
        ],
    )
    .expect("valid unbounded assignment");

    assert_eq!(converted, vec![a, b, c]);
}

#[test]
fn object_details_reports_calculated_battlefield_power_toughness() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let bears_def = grizzly_bears();
    let bears_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

    // Apply +3/+0 until end of turn to the bears.
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            bears_id,
            alice,
            bears_id,
            3,
            0,
            Until::EndOfTurn,
        ));

    let details = build_object_details_snapshot(&game, bears_id).expect("expected object details");
    assert_eq!(details.power, Some(5));
    assert_eq!(details.toughness, Some(2));
}

#[test]
fn object_details_reports_current_granted_abilities() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let bears_def = grizzly_bears();
    let bears_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            bears_id,
            alice,
            EffectTarget::Specific(bears_id),
            Modification::AddAbility(StaticAbility::lifelink()),
        ));

    let details = build_object_details_snapshot(&game, bears_id).expect("expected object details");
    assert!(
        details
            .abilities
            .iter()
            .any(|ability| ability == "Lifelink"),
        "expected object details to expose current granted lifelink, got {:?}",
        details.abilities
    );
}

#[test]
fn object_details_compacts_changeling_type_line_display() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let definition = CardDefinitionBuilder::new(CardId::from_raw(70_010), "Valiant Mini")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .with_ability(Ability::static_ability(StaticAbility::changeling()))
        .build();
    let object_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let details = build_object_details_snapshot(&game, object_id).expect("expected object details");

    assert!(
        details.type_line.matches(' ').count() > Subtype::all_creature_types().len() / 2,
        "raw effective type line should keep the full subtype expansion for debug, got {}",
        details.type_line
    );
    assert_eq!(details.type_line_display, "Creature - Shapeshifter");
    assert_eq!(details.type_line_badges, vec!["All creature types"]);
}

#[test]
fn object_details_include_compiled_spell_effects_for_spells_with_static_abilities() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let definition = compile_to_runtime_definition(
            "Nexus of Fate",
            "Type: Instant\nTake an extra turn after this one.\nIf Nexus of Fate would be put into a graveyard from anywhere, reveal Nexus of Fate and shuffle it into its owner's library instead.",
            false,
        )
        .expect("Nexus of Fate test definition should parse");
    let object_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    let details = build_object_details_snapshot(&game, object_id).expect("expected object details");

    assert!(
        details
            .compiled_text
            .iter()
            .any(|line| line.contains("take an extra turn after this one")),
        "expected compiled inspector text to include the spell effect, got {:?}",
        details.compiled_text
    );
    assert!(
        details
            .compiled_text
            .iter()
            .any(|line| line.contains("shuffle it into its owner's library instead")),
        "expected compiled inspector text to include the static ability, got {:?}",
        details.compiled_text
    );
    assert_eq!(details.abilities.len(), 1);
}

#[test]
fn object_details_debug_compiled_text_keeps_spell_effects_when_oracle_fallback_would_apply() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let definition = compile_to_runtime_definition(
        "Rout",
        "Type: Instant\nYou may cast this spell as though it had flash if you pay {2} more to cast it.\nDestroy all creatures. They can't be regenerated.",
        false,
    )
    .expect("Rout test definition should parse");
    let object_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    let details = build_object_details_snapshot(&game, object_id).expect("expected object details");
    let compiled_text = details.compiled_text.join("\n");

    assert!(
        compiled_text.contains("Destroy all creatures"),
        "expected debug compiled text to include Rout's spell effect, got {:?}",
        details.compiled_text
    );
    assert!(
        compiled_text.contains("They can't be regenerated"),
        "expected debug compiled text to include Rout's no-regeneration effect, got {:?}",
        details.compiled_text
    );
}

#[test]
fn object_details_compiled_text_uses_normalized_surface_for_possessive_self_reference() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let definition = compile_to_runtime_definition(
        "Territorial Kavu",
        "Type: Creature — Kavu\nPower/Toughness: */*\nDomain — This creature's power and toughness are each equal to the number of basic land types among lands you control.",
        false,
    )
    .expect("Territorial Kavu-style domain CDA should parse");
    let object_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let details = build_object_details_snapshot(&game, object_id).expect("expected object details");
    let compiled_text = details.compiled_text.join("\n");

    assert!(
        compiled_text.contains("This creature's power and toughness"),
        "expected normalized possessive self-reference, got {:?}",
        details.compiled_text
    );
    assert!(
        !compiled_text.contains("creature creature's"),
        "frontend compiled text should use normalized compiled_text_lines surface, got {:?}",
        details.compiled_text
    );
}

#[test]
fn object_details_include_convoke_for_builtin_cards() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let definition = stoke_the_flames();
    let object_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    let details = build_object_details_snapshot(&game, object_id).expect("expected object details");

    assert!(
        details.oracle_text.contains("Convoke"),
        "expected oracle text to include Convoke, got {:?}",
        details.oracle_text
    );
    assert!(
        details
            .compiled_text
            .iter()
            .any(|line| line.contains("Convoke")),
        "expected compiled inspector text to include Convoke, got {:?}",
        details.compiled_text
    );
}

#[test]
fn resolving_spell_snapshot_uses_current_source_object_name_after_stack_exit() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let aura = CardBuilder::new(CardId::from_raw(70_001), "Tall as a Beanstalk")
        .card_types(vec![CardType::Enchantment])
        .build();

    let stack_id = game.create_object_from_card(&aura, alice, Zone::Stack);
    let stack_obj = game.object(stack_id).expect("spell should exist on stack");
    let entry = StackEntry::new(stack_id, alice)
        .with_source_info(stack_obj.stable_id, stack_obj.name.clone());

    let battlefield_id = game
        .move_object_by_effect(stack_id, Zone::Battlefield)
        .expect("spell should resolve to battlefield");
    let snapshot = build_stack_object_snapshot(&game, alice, None, &entry);

    assert_eq!(snapshot.name, "Tall as a Beanstalk");
    assert_eq!(snapshot.inspect_object_id, Some(battlefield_id.0));
    assert_eq!(
        snapshot.source_stable_id,
        game.object(battlefield_id).map(|obj| obj.stable_id.0.0)
    );
}

#[test]
fn delayed_trigger_snapshot_keeps_source_name_after_source_changes_zones() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = CardBuilder::new(CardId::from_raw(70_002), "Flickerwisp")
        .card_types(vec![CardType::Creature])
        .build();

    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
    let source_stable_id = game
        .object(source_id)
        .expect("source should exist")
        .stable_id;

    ironsmith::effects::delayed::queue_delayed_trigger(
        &mut game,
        ironsmith::effects::delayed::DelayedTriggerConfig::new(
            Trigger::beginning_of_end_step(ironsmith::target::PlayerFilter::Specific(alice)),
            Vec::new(),
            true,
            Vec::new(),
            alice,
        )
        .with_ability_source(Some(source_id)),
    );

    let moved_source_id = game
        .move_object_by_effect(source_id, Zone::Exile)
        .expect("source should move to exile");
    assert_ne!(moved_source_id, source_id);

    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::phase::BeginningOfEndStepEvent::new(alice),
        ironsmith::provenance::ProvNodeId::default(),
    );
    let triggered = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(triggered.len(), 1, "delayed trigger should fire");

    let mut entry = StackEntry::ability(
        triggered[0].source,
        triggered[0].controller,
        triggered[0].ability.effects.clone(),
    )
    .with_source_info(
        triggered[0].source_stable_id,
        triggered[0].source_name.clone(),
    )
    .with_triggering_event(triggered[0].triggering_event.clone());
    if let Some(snapshot) = triggered[0].source_snapshot.clone() {
        entry = entry.with_source_snapshot(snapshot);
    }
    let snapshot = build_stack_object_snapshot(&game, alice, None, &entry);

    assert_eq!(snapshot.name, "Flickerwisp");
    assert_eq!(snapshot.source_stable_id, Some(source_stable_id.0.0));
    assert_eq!(snapshot.ability_kind.as_deref(), Some("Triggered"));
}

#[test]
fn card_load_diagnostics_include_compilation_context_for_builtin_cards() {
    let mut wasm = WasmGame::new();
    let diagnostics = wasm.build_card_load_diagnostics("Urza's Saga", Some("synthetic failure"));

    assert_eq!(diagnostics.query, "Urza's Saga");
    assert_eq!(diagnostics.canonical_name.as_deref(), Some("Urza's Saga"));
    assert_eq!(diagnostics.error.as_deref(), Some("synthetic failure"));
    assert!(
        diagnostics
            .oracle_text
            .as_deref()
            .is_some_and(|oracle| oracle.contains("chapter")),
        "expected oracle text in diagnostics"
    );
    assert!(
        !diagnostics.compiled_text.is_empty(),
        "expected compiled text lines in diagnostics"
    );
    assert!(
        !diagnostics.compiled_abilities.is_empty(),
        "expected compiled abilities in diagnostics"
    );
}

#[cfg(feature = "generated-registry")]
#[test]
fn card_load_diagnostics_report_parse_error_for_unsupported_generated_cards() {
    let mut wasm = WasmGame::new();
    let diagnostics = wasm.build_card_load_diagnostics("Sicarian Infiltrator", None);

    assert_eq!(diagnostics.query, "Sicarian Infiltrator");
    assert!(
        diagnostics
            .parse_error
            .as_deref()
            .is_some_and(|error| error.to_ascii_lowercase().contains("unsupported")),
        "expected unsupported parse error in diagnostics, got {:?}",
        diagnostics.parse_error
    );
}

#[cfg(feature = "generated-registry")]
#[test]
fn load_decks_reports_threshold_and_parse_failures_separately() {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeckLoadResultView {
        loaded: u32,
        failed: Vec<String>,
        failed_below_threshold: Vec<String>,
        failed_to_parse: Vec<String>,
    }

    let mut wasm = WasmGame::new();
    let (below_threshold_name, below_threshold_score) = CardRegistry::generated_parser_card_names()
        .into_iter()
        .find_map(|name| {
            let score = WasmGame::semantic_score_for_name(name.as_str())?;
            if score >= 1.0 || CardRegistry::try_compile_card(name.as_str()).is_err() {
                return None;
            }
            Some((name, score))
        })
        .expect("expected a compilable generated card below 100% fidelity");
    let threshold_percent = ((below_threshold_score * 100.0) + 0.5).clamp(1.0, 100.0);
    wasm.set_semantic_threshold(threshold_percent);

    let threshold = threshold_percent / 100.0;
    let loaded_name = CardRegistry::generated_parser_card_names()
        .into_iter()
        .find(|name| {
            WasmGame::semantic_score_for_name(name.as_str()).is_some_and(|score| score >= threshold)
                && CardRegistry::try_compile_card(name.as_str()).is_ok()
        })
        .expect("expected a compilable generated card that meets the chosen threshold");

    let decks_js = serde_wasm_bindgen::to_value(&vec![
        vec![
            loaded_name.clone(),
            below_threshold_name.clone(),
            "Sicarian Infiltrator".to_string(),
        ],
        Vec::<String>::new(),
    ])
    .expect("should encode test deck lists");
    let result = wasm
        .load_decks(decks_js)
        .expect("deck load should return categorized failures");
    let result: DeckLoadResultView =
        serde_wasm_bindgen::from_value(result).expect("should decode deck load result");

    assert_eq!(result.loaded, 1);
    assert_eq!(
        result.failed,
        vec![
            below_threshold_name.clone(),
            "Sicarian Infiltrator".to_string(),
        ]
    );
    assert_eq!(result.failed_below_threshold, vec![below_threshold_name]);
    assert_eq!(
        result.failed_to_parse,
        vec!["Sicarian Infiltrator".to_string()]
    );
}

#[cfg(feature = "generated-registry")]
#[test]
fn add_card_to_zone_allows_below_threshold_cards_for_manual_injection() {
    let mut wasm = WasmGame::new();
    let (below_threshold_name, below_threshold_score) = CardRegistry::generated_parser_card_names()
        .into_iter()
        .find_map(|name| {
            let score = WasmGame::semantic_score_for_name(name.as_str())?;
            if score >= 1.0 || CardRegistry::try_compile_card(name.as_str()).is_err() {
                return None;
            }
            Some((name, score))
        })
        .expect("expected a compilable generated card below 100% fidelity");
    let threshold_percent = ((below_threshold_score * 100.0) + 0.5).clamp(1.0, 100.0);
    wasm.set_semantic_threshold(threshold_percent);

    let object_id = wasm
        .add_card_to_zone(0, below_threshold_name.clone(), "hand".to_string(), true)
        .expect("manual card injection should allow below-threshold cards");

    let added = wasm
        .game
        .object(ObjectId::from_raw(object_id))
        .expect("added object should exist");
    assert_eq!(added.name, below_threshold_name);
}

#[cfg(feature = "generated-registry")]
#[test]
fn load_decks_accepts_alternative_card_names() {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeckLoadResultView {
        loaded: u32,
        failed: Vec<String>,
        failed_below_threshold: Vec<String>,
        failed_to_parse: Vec<String>,
    }

    let mut wasm = WasmGame::new();
    let decks_js = serde_wasm_bindgen::to_value(&vec![
        vec![
            "T-60 Power Armor".to_string(),
            "Sunset Sarsaparilla Machine".to_string(),
        ],
        Vec::<String>::new(),
    ])
    .expect("should encode deck lists");
    let result = wasm.load_decks(decks_js).expect("deck load should succeed");
    let result: DeckLoadResultView =
        serde_wasm_bindgen::from_value(result).expect("should decode deck load result");

    assert_eq!(result.loaded, 2);
    assert!(result.failed.is_empty());
    assert!(result.failed_below_threshold.is_empty());
    assert!(result.failed_to_parse.is_empty());

    let alice = wasm
        .game
        .player(PlayerId::from_index(0))
        .expect("alice should exist");
    let library_names: Vec<String> = alice
        .library
        .iter()
        .filter_map(|&id| wasm.game.object(id).map(|object| object.name.clone()))
        .collect();

    assert!(
        library_names.iter().any(|name| name == "T-45 Power Armor"),
        "expected canonical T-45 Power Armor in library, got {library_names:?}"
    );
    assert!(
        library_names
            .iter()
            .any(|name| name == "Nuka-Cola Vending Machine"),
        "expected canonical Nuka-Cola Vending Machine in library, got {library_names:?}"
    );
}

#[cfg(feature = "generated-registry")]
#[test]
fn add_card_to_hand_accepts_alternative_card_names() {
    let mut wasm = WasmGame::new();

    let flavor_id = wasm
        .add_card_to_hand(0, "T-60 Power Armor".to_string())
        .expect("should add flavor-name alias to hand");
    let printed_id = wasm
        .add_card_to_hand(0, "Sunset Sarsaparilla Machine".to_string())
        .expect("should add flavor-name alias to hand");

    let flavor_card = wasm
        .game
        .object(ObjectId::from_raw(flavor_id))
        .expect("flavor-name object should exist");
    let printed_card = wasm
        .game
        .object(ObjectId::from_raw(printed_id))
        .expect("printed-name object should exist");

    assert_eq!(flavor_card.name, "T-45 Power Armor");
    assert_eq!(printed_card.name, "Nuka-Cola Vending Machine");
}

#[test]
fn add_card_to_zone_battlefield_applies_etb_replacement_effects() {
    let mut wasm = WasmGame::new();

    let _tayam_id = wasm
        .add_card_to_zone(
            0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield");
    wasm.game.refresh_continuous_state();

    let entered_id = wasm
        .add_card_to_zone(
            0,
            "Grizzly Bears".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Grizzly Bears to battlefield with ETB processing");

    let entered = wasm
        .game
        .object(ObjectId::from_raw(entered_id))
        .expect("entered permanent should exist");
    assert_eq!(
        entered.counters.get(&CounterType::Vigilance).copied(),
        Some(1),
        "addCardToZone battlefield path should apply Tayam ETB replacement counter"
    );
}

#[test]
fn add_card_to_zone_battlefield_adds_initial_saga_lore_counter() {
    let mut wasm = WasmGame::new();

    let entered_id = wasm
        .add_card_to_zone(
            0,
            "Urza's Saga".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Urza's Saga to battlefield with ETB processing");

    let entered = wasm
        .game
        .object(ObjectId::from_raw(entered_id))
        .expect("entered saga should exist");
    assert_eq!(
        entered.counters.get(&CounterType::Lore).copied(),
        Some(1),
        "battlefield ETB path should give a Saga its initial lore counter"
    );
}

#[test]
fn add_card_to_zone_battlefield_surfaces_roaming_throne_type_choice() {
    let mut wasm = WasmGame::new();

    let added_id = wasm
        .add_card_to_zone(
            0,
            "Roaming Throne".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should start Roaming Throne battlefield entry");

    assert_eq!(
        added_id, 0,
        "battlefield injection should defer committing until the type choice is answered"
    );

    let pending_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected creature-type selection prompt, got {other:?}"),
    };

    assert!(
        pending_ctx
            .options
            .iter()
            .any(|option| option.description == "Angel"),
        "Roaming Throne should prompt for creature types when added straight to the battlefield"
    );
    assert!(
        wasm.pending_replay_action.is_some(),
        "battlefield injection prompt should be backed by replay so the add can resume after a choice"
    );
    assert!(
        !wasm
            .game
            .battlefield
            .iter()
            .filter_map(|id| wasm.game.object(*id))
            .any(|object| object.name == "Roaming Throne"),
        "Roaming Throne should not be committed to the battlefield until the choice is confirmed"
    );
}

#[test]
fn add_card_to_zone_day_of_the_moon_goads_chosen_name_after_text_choice() {
    let mut wasm = WasmGame::new();

    let memnite_id = wasm
        .add_card_to_zone(1, "Memnite".to_string(), "battlefield".to_string(), true)
        .expect("should add Memnite");
    let vanguard_id = wasm
        .add_card_to_zone(
            1,
            "Elite Vanguard".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Elite Vanguard");
    wasm.add_card_to_zone(
        0,
        "Day of the Moon".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should start Day of the Moon chapter choice");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::TextInput(_))),
        "Day of the Moon should ask for a card name, got {:?}",
        wasm.pending_decision
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "type": "text_choice",
            "value": "Memnite",
        }))
        .expect("choice should serialize"),
    )
    .expect("dispatching Day of the Moon card-name choice should succeed");

    assert!(wasm.game.is_goaded(ObjectId::from_raw(memnite_id)));
    assert!(!wasm.game.is_goaded(ObjectId::from_raw(vanguard_id)));
}

#[test]
fn add_card_to_zone_battlefield_commits_roaming_throne_after_choice() {
    let mut wasm = WasmGame::new();

    wasm.add_card_to_zone(
        0,
        "Roaming Throne".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should start Roaming Throne battlefield entry");

    let angel_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx
            .options
            .iter()
            .find(|option| option.description == "Angel")
            .map(|option| option.index)
            .expect("Angel should be a legal creature type choice"),
        other => panic!("expected creature-type selection prompt, got {other:?}"),
    };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "type": "select_options",
            "option_indices": [angel_index],
        }))
        .expect("choice should serialize"),
    )
    .expect("dispatching Roaming Throne creature-type choice should succeed");

    let throne = wasm
        .game
        .battlefield
        .iter()
        .filter_map(|id| wasm.game.object(*id))
        .find(|object| object.name == "Roaming Throne")
        .expect("Roaming Throne should enter after choosing a type");
    assert!(
        throne.subtypes.contains(&Subtype::Angel),
        "Roaming Throne should gain the selected creature subtype once its choice resolves"
    );

    let details = build_object_details_snapshot(&wasm.game, throne.id)
        .expect("Roaming Throne inspector details should exist");
    assert!(
        details.type_line.contains("Angel"),
        "inspector details should use current battlefield subtypes, got {}",
        details.type_line
    );
}

#[test]
fn playing_urzas_saga_from_hand_adds_initial_lore_counter_and_surfaces_snapshot_counters() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let saga_id = wasm
        .game
        .create_object_from_definition(&urzas_saga(), alice, Zone::Hand);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let play_saga_index = priority_ctx
        .actions
        .iter()
        .position(
            |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == saga_id),
        )
        .expect("expected play Urza's Saga action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": play_saga_index,
        }))
        .expect("priority action should serialize"),
    )
    .expect("playing Urza's Saga should succeed");

    let entered_id = wasm
        .game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            wasm.game
                .object(id)
                .is_some_and(|object| object.name == "Urza's Saga")
        })
        .expect("Urza's Saga should be on battlefield");

    let entered = wasm
        .game
        .object(entered_id)
        .expect("played saga should still exist");
    assert_eq!(
        entered.counters.get(&CounterType::Lore).copied(),
        Some(1),
        "playing Urza's Saga as a land should give it its initial lore counter"
    );

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist");
    let saga = me
        .battlefield
        .iter()
        .find(|perm| perm.name == "Urza's Saga")
        .expect("snapshot should include Urza's Saga");

    assert_eq!(
        saga.counters.len(),
        1,
        "snapshot should surface Saga counters"
    );
    assert_eq!(saga.counters[0].kind, "Lore");
    assert_eq!(saga.counters[0].amount, 1);
}

#[test]
fn cancelability_allows_locked_pending_mana_ability_while_decision_open() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;
    wasm.priority_state.pending_mana_ability = Some(PendingManaAbility {
        source: ObjectId::from_raw(1),
        ability_index: 0,
        activator: PlayerId::from_index(0),
        provenance: ironsmith::provenance::ProvNodeId::default(),
        mana_cost: ManaCost::new(),
        other_costs: Vec::new(),
        mana_to_add: vec![ManaSymbol::Green],
        effects: ironsmith::resolution::ResolutionProgram::default(),
        mana_usage_restrictions: Vec::new(),
        mana_source_chosen_creature_type: None,
        undo_locked_by_mana: true,
    });
    wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
        PlayerId::from_index(0),
        None,
        "choose a color",
    )));

    assert!(
        wasm.is_cancelable(),
        "cancel should stay enabled while a decision prompt is open"
    );
}

#[test]
fn cancelability_allows_mana_undo_when_not_locked() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;
    wasm.priority_state.pending_mana_ability = Some(PendingManaAbility {
        source: ObjectId::from_raw(1),
        ability_index: 0,
        activator: PlayerId::from_index(0),
        provenance: ironsmith::provenance::ProvNodeId::default(),
        mana_cost: ManaCost::new(),
        other_costs: Vec::new(),
        mana_to_add: vec![ManaSymbol::Green],
        effects: ironsmith::resolution::ResolutionProgram::default(),
        mana_usage_restrictions: Vec::new(),
        mana_source_chosen_creature_type: None,
        undo_locked_by_mana: false,
    });

    assert!(
        wasm.is_cancelable(),
        "cancel should stay enabled for undo-safe mana activation chains"
    );
}

#[test]
fn cancelability_allows_epoch_undo_without_pending_chain() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;

    assert!(
        wasm.is_cancelable(),
        "cancel should stay available during a reversible priority epoch"
    );
}

#[test]
fn cancelability_blocks_epoch_undo_without_user_action() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = false;

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled when no undoable action happened in this epoch"
    );
}

#[test]
fn cancelability_allows_irreversible_mana_replay_while_decision_open() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
        PlayerId::from_index(0),
        None,
        "choose a color",
    )));
    let checkpoint = wasm.capture_replay_checkpoint();
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Response(PriorityResponse::PriorityAction(
            LegalAction::ActivateManaAbility {
                source: ObjectId::from_raw(999),
                ability_index: 0,
            },
        )),
        nested_answers: Vec::new(),
    });

    assert!(
        wasm.is_cancelable(),
        "cancel should stay enabled while replay is waiting on a decision"
    );
}

#[test]
fn cancelability_blocks_irreversible_mana_replay_without_open_decision() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    let checkpoint = wasm.capture_replay_checkpoint();
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Response(PriorityResponse::PriorityAction(
            LegalAction::ActivateManaAbility {
                source: ObjectId::from_raw(999),
                ability_index: 0,
            },
        )),
        nested_answers: Vec::new(),
    });

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled once irreversible mana replay is committed"
    );
}

#[test]
fn cancelability_blocks_when_land_played_in_epoch() {
    let mut wasm = WasmGame::new();
    let player = PlayerId::from_index(0);
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.game
        .player_mut(player)
        .expect("player should exist")
        .record_land_play();

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled after a land play in the current epoch"
    );
}

#[test]
fn cancelability_blocks_land_play_replay_even_with_open_decision() {
    let mut wasm = WasmGame::new();
    let player = PlayerId::from_index(0);
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
        player,
        None,
        "resolve trigger",
    )));
    let checkpoint = wasm.capture_replay_checkpoint();
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PlayLand {
            land_id: ObjectId::from_raw(777),
        })),
        nested_answers: Vec::new(),
    });
    wasm.game
        .player_mut(player)
        .expect("player should exist")
        .record_land_play();

    assert!(
        !wasm.is_cancelable(),
        "cancel should stay disabled once a replay chain includes a land play"
    );
}

#[test]
fn cancelability_blocks_when_epoch_is_mana_locked() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_undo_locked_by_mana = true;

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled once epoch is locked by irreversible mana activation"
    );
}

#[test]
fn dispatch_disables_cancel_when_mana_tap_trigger_adds_stack_object() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let swamp_card = CardBuilder::new(CardId::new(), "Undo Probe Swamp")
        .card_types(vec![CardType::Land])
        .build();
    let swamp_id = wasm
        .game
        .create_object_from_card(&swamp_card, alice, Zone::Battlefield);
    if let Some(swamp) = wasm.game.object_mut(swamp_id) {
        swamp.abilities.push(Ability::mana(
            ironsmith::cost::TotalCost::free(),
            vec![ManaSymbol::Black],
        ));
    }

    let trigger_source = CardBuilder::new(CardId::new(), "Undo Probe Trigger")
        .card_types(vec![CardType::Enchantment])
        .build();
    let trigger_source_id =
        wasm.game
            .create_object_from_card(&trigger_source, alice, Zone::Battlefield);
    if let Some(source) = wasm.game.object_mut(trigger_source_id) {
        source.abilities.push(Ability::triggered(
            Trigger::player_taps_for_mana(
                ironsmith::target::PlayerFilter::Any,
                ironsmith::filter::ObjectFilter::land(),
            ),
            vec![Effect::lose_life_player(
                1,
                ironsmith::target::PlayerFilter::Specific(alice),
            )],
        ));
    }

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let action_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateManaAbility { source, .. } if *source == swamp_id
            )
        })
        .expect("expected tap-for-mana action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": action_index,
        }))
        .expect("priority action should serialize"),
    )
    .expect("tapping swamp for mana should succeed");

    assert_eq!(
        wasm.game.stack.len(),
        1,
        "non-mana tap-for-mana trigger should add an object to the stack"
    );
    assert!(
        !wasm.is_cancelable(),
        "undo should be disabled once tapping for mana creates a stack object"
    );
}

#[test]
fn snapshot_surfaces_undo_land_stable_id_for_reversible_land_tap() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let swamp_card = CardBuilder::new(CardId::new(), "Undo Probe Swamp")
        .card_types(vec![CardType::Land])
        .build();
    let swamp_id = wasm
        .game
        .create_object_from_card(&swamp_card, alice, Zone::Battlefield);
    if let Some(swamp) = wasm.game.object_mut(swamp_id) {
        swamp.abilities.push(Ability::mana(
            ironsmith::cost::TotalCost::free(),
            vec![ManaSymbol::Black],
        ));
    }
    let swamp_stable_id = wasm
        .game
        .object(swamp_id)
        .expect("swamp should exist")
        .stable_id
        .0
        .0;

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let action_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateManaAbility { source, .. } if *source == swamp_id
            )
        })
        .expect("expected tap-for-mana action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": action_index,
        }))
        .expect("priority action should serialize"),
    )
    .expect("tapping swamp for mana should succeed");

    assert!(
        wasm.is_cancelable(),
        "plain land tap should remain undoable"
    );
    assert_eq!(
        wasm.priority_epoch_undo_land_stable_id,
        Some(swamp_stable_id),
        "the current undoable land tap should be tracked by stable id"
    );

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let cancelable = wasm.is_cancelable();
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        cancelable,
        wasm.visible_undo_land_stable_id(cancelable),
        0,
    );
    assert_eq!(
        snapshot.undo_land_stable_id,
        Some(swamp_stable_id),
        "snapshot should expose the reversible tapped land for the UI"
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist");
    let swamp = me
        .battlefield
        .iter()
        .find(|perm| perm.stable_id == swamp_stable_id)
        .expect("snapshot should still include the tapped swamp");
    assert!(
        swamp.tapped,
        "tracked undo land should be tapped in the snapshot"
    );

    let snapshot_value =
        serde_json::to_value(&snapshot).expect("snapshot should serialize to JSON");
    let actions = snapshot_value
        .get("decision")
        .and_then(|decision| decision.get("actions"))
        .and_then(|actions| actions.as_array())
        .expect("priority decision should expose actions");
    let untap_action = actions
        .iter()
        .find(|action| action.get("kind").and_then(|kind| kind.as_str()) == Some("untap_land"))
        .expect("reversible land tap should be exposed as an untap action");
    assert_eq!(
        untap_action
            .get("object_id")
            .and_then(|object_id| object_id.as_u64()),
        Some(swamp_id.0),
        "untap action should point at the tapped land object"
    );
    assert_eq!(
        untap_action
            .get("action_ref")
            .and_then(|action_ref| action_ref.get("kind"))
            .and_then(|kind| kind.as_str()),
        Some("untap_land"),
        "untap action should remain identifiable without relying on its index"
    );
}

#[test]
fn phyrexian_tower_sacrifice_mana_action_uses_selected_creature_without_rollback() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tower_id =
        wasm.game
            .create_object_from_definition(&phyrexian_tower(), alice, Zone::Battlefield);
    let bear_id =
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    let bear_stable_id = wasm
        .game
        .object(bear_id)
        .expect("Grizzly Bears should exist")
        .stable_id;
    let thopter_id =
        wasm.game
            .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let snapshot_json = wasm
        .snapshot_json()
        .expect("priority snapshot should render Phyrexian Tower actions");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");
    let actions = snapshot["decision"]["actions"]
        .as_array()
        .expect("priority decision should expose actions");
    let tower_sacrifice_action = actions
        .iter()
        .find(|action| {
            action["action_ref"]["kind"] == "activate_mana_ability"
                && action["action_ref"]["source"].as_u64() == Some(tower_id.0)
                && action["action_ref"]["ability_index"].as_u64() == Some(1)
        })
        .expect("Tower sacrifice mana action should be present");
    let label = tower_sacrifice_action["label"]
        .as_str()
        .expect("Tower action should have a label");
    assert!(
        label.contains("Sacrifice a creature"),
        "Tower action label should use the compact oracle cost: {label}"
    );
    assert!(
        !label.contains("Exile a creature") && !label.contains("Sacrifice a permanent"),
        "Tower action label should not expose raw tagged cost components: {label}"
    );

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(
            action,
            LegalAction::ActivateManaAbility {
                source,
                ability_index
            } if *source == tower_id && *ability_index == 1
        )
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            let candidate_ids: Vec<ObjectId> = ctx.candidates.iter().map(|obj| obj.id).collect();
            assert!(
                candidate_ids.contains(&bear_id) && candidate_ids.contains(&thopter_id),
                "Tower sacrifice prompt should offer Alice's creatures: {candidate_ids:?}"
            );
        }
        other => panic!("expected Tower sacrifice prompt, got {other:?}"),
    }

    dispatch_select_objects(&mut wasm, &[bear_id.0]);

    assert!(
        !wasm.game.battlefield.contains(&bear_id),
        "selected creature should be sacrificed, not restored by replay"
    );
    assert!(
        wasm.game
            .player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(
                &wasm
                    .game
                    .find_object_by_stable_id(bear_stable_id)
                    .expect("sacrificed creature should still be tracked by stable id")
            ),
        "selected creature should move to Alice's graveyard"
    );
    assert!(
        wasm.game.is_tapped(tower_id),
        "Tower should stay tapped after activation"
    );
    assert_eq!(
        wasm.game
            .player(alice)
            .expect("Alice should exist")
            .mana_pool
            .black,
        2,
        "Tower should add two black mana"
    );
    assert!(
        wasm.pending_replay_action.is_none(),
        "replay state should close after the sacrifice choice resolves"
    );
}

#[test]
fn cleanup_auto_discard_only_applies_for_non_perspective_player() {
    let mut wasm = WasmGame::new();
    wasm.game.turn.step = Some(Step::Cleanup);

    let perspective_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
        wasm.perspective,
        None,
        "Discard cards",
        vec![
            SelectableObject::new(ObjectId::from_raw(1), "Card A"),
            SelectableObject::new(ObjectId::from_raw(2), "Card B"),
        ],
        1,
        Some(1),
    ));
    assert!(
        !wasm.should_auto_resolve_cleanup_discard(&perspective_ctx),
        "cleanup discard should not auto-resolve for the perspective player"
    );

    let opponent = PlayerId::from_index((wasm.perspective.0 + 1) % wasm.game.players.len() as u8);
    let opponent_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
        opponent,
        None,
        "Discard cards",
        vec![
            SelectableObject::new(ObjectId::from_raw(3), "Card C"),
            SelectableObject::new(ObjectId::from_raw(4), "Card D"),
        ],
        1,
        Some(1),
    ));
    assert!(
        wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
        "cleanup discard should auto-resolve for non-perspective players"
    );
}

#[test]
fn cleanup_auto_discard_respects_toggle_and_cleanup_step() {
    let mut wasm = WasmGame::new();
    let opponent = PlayerId::from_index((wasm.perspective.0 + 1) % wasm.game.players.len() as u8);
    let opponent_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
        opponent,
        None,
        "Discard cards",
        vec![SelectableObject::new(ObjectId::from_raw(5), "Card E")],
        1,
        Some(1),
    ));

    wasm.game.turn.step = Some(Step::Cleanup);
    wasm.auto_cleanup_discard = false;
    assert!(
        !wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
        "toggle should disable cleanup auto-discard"
    );

    wasm.auto_cleanup_discard = true;
    wasm.game.turn.step = Some(Step::End);
    assert!(
        !wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
        "auto-discard should only happen during cleanup step"
    );
}

#[test]
fn snapshot_perspective_hand_cards_are_not_truncated() {
    let mut wasm = WasmGame::new();
    for _ in 0..20 {
        wasm.add_card_to_zone(0, "Ornithopter".to_string(), "hand".to_string(), true)
            .expect("adding card to hand should succeed");
    }

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|p| p.id == wasm.perspective.0)
        .expect("perspective player should exist in snapshot");

    assert_eq!(
        me.hand_cards.len(),
        me.hand_size,
        "perspective hand_cards must stay in sync with hand_size"
    );
    assert!(
        me.hand_cards.len() >= 20,
        "expected all 20 hand cards to be present in snapshot"
    );
    assert!(
        me.hand_cards
            .iter()
            .all(|card| card.oracle_text.contains("Flying")),
        "visible hand card snapshots should include oracle text for inspector fallback"
    );
}

#[test]
fn snapshot_visible_zone_cards_include_oracle_text() {
    let mut wasm = WasmGame::new();
    let bob = PlayerId::from_index(1);
    let bolt_id = wasm
        .add_card_to_zone(
            bob.0,
            "Lightning Bolt".to_string(),
            "graveyard".to_string(),
            true,
        )
        .expect("adding Lightning Bolt to graveyard should succeed");

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("Bob snapshot should exist");
    let bolt = bob_snapshot
        .graveyard_cards
        .iter()
        .find(|card| card.id == bolt_id)
        .expect("visible graveyard card should be in snapshot");

    assert!(
        bolt.oracle_text.contains("Lightning Bolt deals 3 damage"),
        "visible zone card snapshots should include oracle text for inspector fallback"
    );
}

#[test]
fn snapshot_public_top_library_static_shows_each_players_top_card() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let lantern = CardDefinitionBuilder::new(CardId::new(), "Lantern of Insight Variant")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ))
        .build();
    game.create_object_from_definition(&lantern, alice, Zone::Battlefield);

    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Alice Bottom")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Alice Top")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Bob Bottom")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Bob Top")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Library,
    );

    let snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let alice_view = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice snapshot should exist");
    let bob_view = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist");

    assert!(alice_view.can_view_library_top);
    assert_eq!(alice_view.library_top.as_deref(), Some("Alice Top"));
    assert!(bob_view.can_view_library_top);
    assert_eq!(bob_view.library_top.as_deref(), Some("Bob Top"));
}

#[test]
fn crypto_requirements_include_static_public_top_library_opening() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    let lantern = CardDefinitionBuilder::new(CardId::new(), "Lantern of Insight Variant")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ))
        .build();
    wasm.game
        .create_object_from_definition(&lantern, alice, Zone::Battlefield);
    let hidden_top = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        0,
        "alice-library-top-commitment".to_string(),
    );

    let before = wasm.capture_crypto_audit_state();
    wasm.update_crypto_requirements_from(before);

    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "public_view_window"
            && requirement.owner == alice.index() as u8
            && requirement.zone == "library"
            && requirement.count == Some(1)
    }));
    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "public_open"
            && requirement.owner == alice.index() as u8
            && requirement.zone == "library"
            && requirement.object_id == Some(hidden_top.0)
            && requirement.commitment.as_deref() == Some("alice-library-top-commitment")
    }));
}

#[test]
fn crypto_requirements_include_static_private_own_top_library_opening() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    let future_sight = CardDefinitionBuilder::new(CardId::new(), "Future Sight Variant")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::look_at_top_card_of_library(),
        ))
        .build();
    wasm.game
        .create_object_from_definition(&future_sight, alice, Zone::Battlefield);
    let hidden_top = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        0,
        "alice-private-top-commitment".to_string(),
    );

    let before = wasm.capture_crypto_audit_state();
    wasm.update_crypto_requirements_from(before);

    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "private_view_window"
            && requirement.owner == alice.index() as u8
            && requirement.viewer == Some(alice.index() as u8)
            && requirement.zone == "library"
            && requirement.count == Some(1)
    }));
    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "private_open"
            && requirement.owner == alice.index() as u8
            && requirement.viewer == Some(alice.index() as u8)
            && requirement.zone == "library"
            && requirement.object_id == Some(hidden_top.0)
            && requirement.commitment.as_deref() == Some("alice-private-top-commitment")
    }));
}

#[test]
fn snapshot_courser_static_shows_only_controllers_top_library_card() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let courser = CardDefinitionBuilder::new(CardId::new(), "Courser of Kruphix Variant")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .with_ability(Ability::static_ability(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ))
        .build();
    game.create_object_from_definition(&courser, alice, Zone::Battlefield);

    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Alice Top")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Bob Hidden Top")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Library,
    );

    let snapshot = GameSnapshot::from_game(
        &game,
        bob,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let alice_view = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice snapshot should exist");
    let bob_view = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist");

    assert!(alice_view.can_view_library_top);
    assert_eq!(alice_view.library_top.as_deref(), Some("Alice Top"));
    assert!(!bob_view.can_view_library_top);
    assert_eq!(bob_view.library_top, None);
}

#[test]
fn snapshot_telepathy_static_shows_opponents_hands_to_all_players() {
    let mut game = GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);

    let telepathy = CardDefinitionBuilder::new(CardId::new(), "Telepathy Variant")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::opponents_play_with_hands_revealed(),
        ))
        .build();
    game.create_object_from_definition(&telepathy, alice, Zone::Battlefield);

    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Alice Secret")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Hand,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Bob Revealed")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Hand,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Cara Revealed")
            .card_types(vec![CardType::Creature])
            .build(),
        cara,
        Zone::Hand,
    );

    let snapshot = GameSnapshot::from_game(
        &game,
        cara,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let alice_view = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice snapshot should exist");
    let bob_view = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist");
    let cara_view = snapshot
        .players
        .iter()
        .find(|player| player.id == cara.0)
        .expect("cara snapshot should exist");

    assert!(!alice_view.can_view_hand);
    assert!(alice_view.hand_cards.is_empty());
    assert!(bob_view.can_view_hand);
    assert_eq!(bob_view.hand_cards[0].name, "Bob Revealed");
    assert!(cara_view.can_view_hand);
    assert_eq!(cara_view.hand_cards[0].name, "Cara Revealed");
}

#[test]
fn crypto_requirements_include_static_public_hand_openings() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let telepathy = CardDefinitionBuilder::new(CardId::new(), "Telepathy Variant")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::opponents_play_with_hands_revealed(),
        ))
        .build();
    wasm.game
        .create_object_from_definition(&telepathy, alice, Zone::Battlefield);
    let hidden_hand = wasm.game.create_hidden_card_placeholder(
        bob,
        Zone::Hand,
        4,
        "bob-hand-commitment".to_string(),
    );

    let before = wasm.capture_crypto_audit_state();
    wasm.update_crypto_requirements_from(before);

    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "public_view_window"
            && requirement.owner == bob.index() as u8
            && requirement.zone == "hand"
            && requirement.count == Some(1)
    }));
    assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
        requirement.requirement_type == "public_open"
            && requirement.owner == bob.index() as u8
            && requirement.zone == "hand"
            && requirement.object_id == Some(hidden_hand.0)
            && requirement.commitment.as_deref() == Some("bob-hand-commitment")
    }));
}

#[test]
fn snapshot_redacts_hidden_opponent_select_object_candidates() {
    let mut wasm = WasmGame::new();
    let bob = PlayerId::from_index(1);
    let card_a = wasm
        .add_card_to_zone(1, "Primeval Titan".to_string(), "hand".to_string(), true)
        .expect("adding first hidden card should succeed");
    let card_b = wasm
        .add_card_to_zone(1, "Forest".to_string(), "hand".to_string(), true)
        .expect("adding second hidden card should succeed");

    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        bob,
        None,
        "Choose cards to discard",
        vec![
            SelectableObject::new(ObjectId::from_raw(card_a), "Primeval Titan"),
            SelectableObject::new(ObjectId::from_raw(card_b), "Forest"),
        ],
        1,
        Some(1),
    ));

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );

    let redacted_candidates = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the pending select-objects decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };

    assert_eq!(redacted_candidates.len(), 2);
    assert!(
        redacted_candidates
            .iter()
            .all(|candidate| candidate.name == "Hidden card"),
        "opponent hand choices should be redacted for other perspectives"
    );
    assert!(
        redacted_candidates
            .iter()
            .all(|candidate| candidate.id != card_a && candidate.id != card_b),
        "redacted candidates should not expose the real hidden object ids"
    );
    assert!(
        redacted_candidates
            .iter()
            .all(|candidate| candidate.object_controller.is_none()),
        "redacted candidates should not expose hidden object controllers"
    );
}

#[test]
fn redacted_select_object_choice_ids_resolve_to_real_candidates() {
    let wasm = WasmGame::new();
    let card_a = ObjectId::from_raw(101);
    let card_b = ObjectId::from_raw(102);
    let ctx = SelectObjectsContext::new(
        PlayerId::from_index(1),
        None,
        "Choose a card to discard",
        vec![
            SelectableObject::new(card_a, "Mountain"),
            SelectableObject::new(card_b, "Forest"),
        ],
        1,
        Some(1),
    );

    let selected = normalize_select_object_choice_ids(
        &wasm.game,
        &ctx,
        &[super::redacted_choice_id(1)],
        &[],
        &[],
    )
    .expect("redacted choice should normalize");

    assert_eq!(selected, vec![card_b.0]);
}

#[test]
fn select_object_view_exposes_stable_identity_for_public_candidates() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let card = wasm
        .add_card_to_zone(0, "Forest".to_string(), "battlefield".to_string(), true)
        .expect("card should be added");
    let card_id = ObjectId::from_raw(card);
    let stable_id = wasm.game.object(card_id).expect("object").stable_id.0.0;
    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Choose a permanent",
        vec![SelectableObject::new(card_id, "Forest")],
        1,
        Some(1),
    ));

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        None,
        None,
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let candidates = match snapshot.decision.as_ref().expect("decision") {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };

    assert_eq!(candidates[0].selection_identity, "stable_id");
    assert_eq!(candidates[0].reveal_policy, "none");
    assert_eq!(candidates[0].stable_id, Some(stable_id));
    assert!(candidates[0].hidden_ref.is_none());
}

#[test]
fn select_object_view_keeps_synthetic_candidates_on_object_ids() {
    let wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let synthetic = ObjectId::from_raw(0x8000_0001);
    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Choose a player",
        vec![SelectableObject::new(synthetic, "Alice")],
        1,
        Some(1),
    ));

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        None,
        None,
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let candidates = match snapshot.decision.as_ref().expect("decision") {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };

    assert_eq!(candidates[0].selection_identity, "object_id");
    assert_eq!(candidates[0].stable_id, None);
    assert!(candidates[0].hidden_ref.is_none());
}

#[test]
fn select_object_view_uses_hidden_reference_without_card_identity_for_hidden_candidates() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let hidden = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Hand,
        7,
        "alice-hand-slot-7".to_string(),
    );
    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Choose a hidden card",
        vec![SelectableObject::new(hidden, "Hidden card")],
        1,
        Some(1),
    ));

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        None,
        None,
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let candidates = match snapshot.decision.as_ref().expect("decision") {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };
    let hidden_ref = candidates[0].hidden_ref.as_ref().expect("hidden ref");

    assert_eq!(candidates[0].selection_identity, "hidden_reference");
    assert_eq!(candidates[0].stable_id, None);
    assert_eq!(hidden_ref.owner, Some(alice.0));
    assert_eq!(hidden_ref.zone.as_deref(), Some("hand"));
    assert_eq!(hidden_ref.slot, Some(7));
    assert_eq!(hidden_ref.commitment.as_deref(), Some("alice-hand-slot-7"));
    assert!(
        candidates[0].name.eq_ignore_ascii_case("hidden card"),
        "hidden refs must not reveal a card name"
    );
}

#[test]
fn select_object_view_uses_hidden_reference_for_face_down_exile() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let hidden = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Exile,
        2,
        "alice-exile-slot-2".to_string(),
    );
    wasm.game.set_face_down(hidden);
    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Choose a face-down exiled card",
        vec![SelectableObject::new(hidden, "Hidden card")],
        1,
        Some(1),
    ));

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        None,
        None,
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let candidates = match snapshot.decision.as_ref().expect("decision") {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };

    assert_eq!(candidates[0].selection_identity, "hidden_reference");
    assert_eq!(
        candidates[0]
            .hidden_ref
            .as_ref()
            .and_then(|hidden_ref| hidden_ref.commitment.as_deref()),
        Some("alice-exile-slot-2")
    );
}

#[test]
fn select_object_choice_ids_remap_by_stable_id_and_reject_invalid_stable_id() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let first = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Forest".to_string(), "battlefield".to_string(), true)
            .expect("first card should be added"),
    );
    let second = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Mountain".to_string(), "battlefield".to_string(), true)
            .expect("second card should be added"),
    );
    let second_stable = wasm.game.object(second).expect("second").stable_id.0.0;
    let ctx = SelectObjectsContext::new(
        alice,
        None,
        "Choose a permanent",
        vec![
            SelectableObject::new(first, "Forest"),
            SelectableObject::new(second, "Mountain"),
        ],
        1,
        Some(1),
    );

    let selected = normalize_select_object_choice_ids(
        &wasm.game,
        &ctx,
        &[999_999],
        &[Some(second_stable)],
        &[],
    )
    .expect("stable id should remap to legal candidate");
    assert_eq!(selected, vec![second.0]);

    let invalid =
        normalize_select_object_choice_ids(&wasm.game, &ctx, &[999_999], &[Some(123_456_789)], &[]);
    assert!(invalid.is_err());
}

#[test]
fn select_object_choice_ids_remap_by_hidden_ref_only_for_legal_candidates() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let hidden = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Hand,
        4,
        "alice-hidden-slot-4".to_string(),
    );
    let other = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Hand,
        5,
        "alice-hidden-slot-5".to_string(),
    );
    let hidden_ref = HiddenObjectRef {
        owner: Some(alice.0),
        zone: Some("hand".to_string()),
        slot: Some(4),
        commitment: Some("alice-hidden-slot-4".to_string()),
        public_slot: None,
        public_commitment: None,
    };
    let ctx = SelectObjectsContext::new(
        alice,
        None,
        "Choose a hidden card",
        vec![SelectableObject::new(hidden, "Hidden card")],
        1,
        Some(1),
    );

    let selected = normalize_select_object_choice_ids(
        &wasm.game,
        &ctx,
        &[999_999],
        &[],
        &[Some(hidden_ref.clone())],
    )
    .expect("hidden ref should remap to legal candidate");
    assert_eq!(selected, vec![hidden.0]);

    let illegal_ref = HiddenObjectRef {
        slot: Some(5),
        commitment: Some("alice-hidden-slot-5".to_string()),
        ..hidden_ref
    };
    let invalid =
        normalize_select_object_choice_ids(&wasm.game, &ctx, &[999_999], &[], &[Some(illegal_ref)]);
    assert!(invalid.is_err());
    assert_ne!(hidden, other);
}

#[test]
fn snapshot_shows_hidden_zone_select_object_candidates_to_decision_player() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let borrowed_card = wasm
        .add_card_to_zone(1, "Primeval Titan".to_string(), "library".to_string(), true)
        .expect("adding hidden library card should succeed");
    let borrowed_card_id = ObjectId::from_raw(borrowed_card);

    wasm.game
        .player_mut(bob)
        .expect("owner should exist")
        .library
        .retain(|id| *id != borrowed_card_id);
    wasm.game
        .player_mut(alice)
        .expect("searching player should exist")
        .library
        .push(borrowed_card_id);

    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Search library (revealed)",
        vec![SelectableObject::new(borrowed_card_id, "Primeval Titan")],
        1,
        Some(1),
    ));

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );

    let candidates = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the pending select-objects decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects view, got {other:?}"),
    };

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].name, "Primeval Titan",
        "the decision player should see candidate names for objects exposed by the prompt"
    );
    assert_eq!(
        candidates[0].id, borrowed_card,
        "the decision player should receive the real object id for exposed candidates"
    );
    assert_eq!(
        candidates[0].object_controller,
        Some(bob.0),
        "visible select-object candidates should include their current controller for UI coloring"
    );
}

#[test]
fn snapshot_routes_controlled_player_decision_to_controller() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = wasm
        .add_card_to_zone(0, "Primeval Titan".to_string(), "hand".to_string(), true)
        .expect("adding hidden hand card should succeed");
    let card_id = ObjectId::from_raw(card);

    wasm.game.add_player_control(
        bob,
        alice,
        PlayerControlStart::Immediate,
        PlayerControlDuration::UntilEndOfTurn,
        None,
    );

    let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Choose a card",
        vec![SelectableObject::new(card_id, "Primeval Titan")],
        1,
        Some(1),
    ));
    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        bob,
        Some(&decision),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );

    match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the controlled-player decision")
    {
        super::DecisionView::SelectObjects {
            player, candidates, ..
        } => {
            assert_eq!(
                *player, bob.0,
                "UI decisions should be routed to the controlling player"
            );
            assert_eq!(candidates[0].name, "Primeval Titan");
            assert_eq!(candidates[0].id, card);
        }
        other => panic!("expected select_objects view, got {other:?}"),
    }
}

#[test]
fn snapshot_redacts_hidden_opponent_priority_hand_actions() {
    let mut wasm = WasmGame::new();
    let bob = PlayerId::from_index(1);
    let spell_id = wasm
        .add_card_to_zone(1, "Lightning Bolt".to_string(), "hand".to_string(), true)
        .expect("adding hidden spell should succeed");
    let priority = DecisionContext::Priority(PriorityContext::new(
        bob,
        vec![
            LegalAction::PassPriority,
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(spell_id),
                from_zone: Zone::Hand,
                casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
            },
        ],
    ));

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        Some(&priority),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );

    let actions = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the priority decision")
    {
        super::DecisionView::Priority { actions, .. } => actions,
        other => panic!("expected priority view, got {other:?}"),
    };
    let hidden_cast = actions
        .iter()
        .find(|action| action.kind == "cast_spell")
        .expect("snapshot should include the redacted cast action");

    assert_eq!(hidden_cast.label, "Cast hidden spell");
    assert_eq!(hidden_cast.object_id, None);
    assert_eq!(hidden_cast.from_zone, None);
    assert_eq!(hidden_cast.to_zone, None);
}

#[test]
fn snapshot_surfaces_cross_owner_play_from_cards_in_pseudo_hand() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let card = CardBuilder::new(CardId::from_raw(991001), "Borrowed Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let exiled_id = game.create_object_from_card(&card, bob, Zone::Exile);

    game.effect_store.grant_registry.grant_to_card(
        exiled_id,
        Zone::Exile,
        alice,
        ironsmith::grant::Grantable::PlayFrom,
        ironsmith::grant_registry::GrantSource::Effect {
            source_id: ObjectId::from_raw(991002),
            expires_end_of_turn: game.turn.turn_number,
        },
    );

    let snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("opponent snapshot should exist");
    let exiled_card = bob_snapshot
        .exile_cards
        .iter()
        .find(|card| card.id == exiled_id.0)
        .expect("exiled card should be present in opponent exile");

    assert!(
        exiled_card.show_in_pseudo_hand,
        "play-from card in an opponent-owned exile pile should still surface in the perspective player's pseudo-hand"
    );
    assert_eq!(
        exiled_card.pseudo_hand_glow_kind.as_deref(),
        Some("play-from"),
        "cross-owner play-from cards should carry the dedicated pseudo-hand glow kind"
    );

    game.turn.turn_number = game.turn.turn_number.saturating_add(1);
    let expired_snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let expired_bob_snapshot = expired_snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("opponent snapshot should still exist");
    let expired_card = expired_bob_snapshot
        .exile_cards
        .iter()
        .find(|card| card.id == exiled_id.0)
        .expect("expired card should remain in exile");

    assert!(
        !expired_card.show_in_pseudo_hand,
        "pseudo-hand should stop surfacing the card once the play-from permission expires"
    );
    assert_eq!(
        expired_card.pseudo_hand_glow_kind, None,
        "expired play-from cards should no longer advertise a pseudo-hand glow"
    );
}

#[test]
fn snapshot_grouped_battlefield_count_matches_total() {
    let mut wasm = WasmGame::new();
    for _ in 0..3 {
        wasm.add_card_to_zone(
            0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("adding lotus to battlefield should succeed");
    }
    wasm.add_card_to_zone(0, "Mountain".to_string(), "battlefield".to_string(), true)
        .expect("adding mountain to battlefield should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|p| p.id == wasm.perspective.0)
        .expect("perspective player should exist in snapshot");

    let grouped_total: usize = me.battlefield.iter().map(|perm| perm.count).sum();
    assert_eq!(
        grouped_total, me.battlefield_total,
        "battlefield_total must equal sum of grouped permanent counts"
    );
}

#[test]
fn snapshot_grouped_battlefield_includes_mana_cost() {
    let mut wasm = WasmGame::new();
    wasm.add_card_to_zone(
        0,
        "Ornithopter".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("adding ornithopter to battlefield should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|p| p.id == wasm.perspective.0)
        .expect("perspective player should exist in snapshot");
    let ornithopter = me
        .battlefield
        .iter()
        .find(|perm| perm.name == "Ornithopter")
        .expect("expected ornithopter on battlefield");

    assert_eq!(ornithopter.mana_cost.as_deref(), Some("{0}"));
}

#[test]
fn canceling_spell_chain_after_land_play_keeps_land_on_battlefield() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let mountain_id = wasm
        .game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let play_mountain_index = priority_ctx
        .actions
        .iter()
        .position(
            |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == mountain_id),
        )
        .expect("expected play mountain action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": play_mountain_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("playing mountain should succeed");

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision after land play, got {other:?}"),
    };
    let cast_bolt_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == bolt_id
            )
        })
        .expect("expected cast lightning bolt action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_bolt_index,
        }))
        .expect("cast spell command should serialize"),
    )
    .expect("casting lightning bolt should enter its decision chain");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
        "lightning bolt cast should be waiting on targets"
    );

    wasm.cancel_decision()
        .expect("canceling the in-progress spell should succeed");

    let alice_player = wasm.game.player(alice).expect("alice should exist");
    let mountains_on_battlefield = wasm
        .game
        .battlefield
        .iter()
        .filter(|&&id| {
            wasm.game
                .object(id)
                .is_some_and(|object| object.name == "Mountain")
        })
        .count();
    let bolts_in_hand = alice_player
        .hand
        .iter()
        .filter(|&&id| {
            wasm.game
                .object(id)
                .is_some_and(|object| object.name == "Lightning Bolt")
        })
        .count();

    assert_eq!(
        mountains_on_battlefield, 1,
        "canceling the spell should keep the played land on the battlefield"
    );
    assert_eq!(
        bolts_in_hand, 1,
        "canceling the spell should return the spell to hand"
    );
    assert!(
        wasm.game.stack.is_empty(),
        "canceling the spell should remove it from the stack"
    );
}

#[test]
fn yawgmoth_activation_stays_cancelable_through_target_and_cost_prompts() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let yawgmoth_id = wasm.game.create_object_from_definition(
        &yawgmoth_thran_physician(),
        alice,
        Zone::Battlefield,
    );
    let target_id =
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    wasm.game
        .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == yawgmoth_id
            )
        })
        .expect("expected Yawgmoth activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Yawgmoth should enter target selection");

    let targets_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected target prompt after Yawgmoth activation, got {other:?}"),
    };
    assert_eq!(
        targets_ctx.player, alice,
        "Yawgmoth target prompt should belong to the activating player"
    );
    assert!(
        wasm.pending_replay_action.is_some(),
        "Yawgmoth activation should keep replay state open while choosing targets"
    );
    assert!(
        wasm.is_cancelable(),
        "Yawgmoth activation should remain cancelable during target selection"
    );

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let cancelable = wasm.is_cancelable();
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        cancelable,
        wasm.visible_undo_land_stable_id(cancelable),
        0,
    );
    assert!(
        snapshot.cancelable,
        "snapshot should expose Yawgmoth target prompt as cancelable"
    );
    assert!(
        snapshot.resolving_stack_object.is_none(),
        "activation-time target prompts should not pin a resolving stack entry"
    );
    let decision = snapshot
        .decision
        .expect("snapshot should still include the target decision");
    let player = match decision {
        super::DecisionView::Targets { player, .. } => player,
        other => panic!("expected target decision snapshot, got {other:?}"),
    };
    assert_eq!(
        player, alice.0,
        "snapshot target decision should belong to the perspective player"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "object", "object": target_id.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing Yawgmoth's target should continue activation");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost prompt after Yawgmoth target, got {other:?}"),
    };
    assert_eq!(
        next_cost_ctx.player, alice,
        "Yawgmoth next-cost prompt should belong to the activating player"
    );
    assert!(
        wasm.pending_replay_action.is_some(),
        "Yawgmoth activation should keep replay state open while choosing costs"
    );
    assert!(
        wasm.is_cancelable(),
        "Yawgmoth activation should remain cancelable after choosing targets"
    );

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let cancelable = wasm.is_cancelable();
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        cancelable,
        wasm.visible_undo_land_stable_id(cancelable),
        0,
    );
    assert!(
        snapshot.cancelable,
        "snapshot should expose Yawgmoth next-cost prompt as cancelable"
    );
    assert!(
        snapshot.resolving_stack_object.is_none(),
        "cost-payment prompts should not pin a resolving stack entry before the ability is committed"
    );
    let decision = snapshot
        .decision
        .expect("snapshot should still include the next-cost decision");
    match decision {
        super::DecisionView::SelectOptions { player, reason, .. } => {
            assert_eq!(player, alice.0);
            assert_eq!(reason.as_deref(), Some("Next cost"));
        }
        other => panic!("expected next-cost decision snapshot, got {other:?}"),
    }
}

#[test]
fn yawgmoth_proliferate_next_cost_choices_advance_in_replay_chain() {
    fn setup_proliferate_prompt() -> WasmGame {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let yawgmoth_id = wasm.game.create_object_from_definition(
            &yawgmoth_thran_physician(),
            alice,
            Zone::Battlefield,
        );
        wasm.add_card_to_zone(
            alice.0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Black Lotus to battlefield");
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Hand);
        wasm.game
            .create_object_from_definition(&ornithopter(), alice, Zone::Hand);

        let proliferate_ability_index = wasm
            .game
            .object(yawgmoth_id)
            .and_then(|object| {
                object.abilities.iter().position(|ability| {
                    matches!(
                        &ability.kind,
                        ironsmith::ability::AbilityKind::Activated(activated)
                            if activated.mana_cost.mana_cost().is_some()
                                && activated
                                    .mana_cost
                                    .costs()
                                    .iter()
                                    .any(|cost| cost.is_discard())
                    )
                })
            })
            .expect("Yawgmoth should have proliferate ability");

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateAbility { source, ability_index }
                        if *source == yawgmoth_id && *ability_index == proliferate_ability_index
                )
            })
            .expect("expected Yawgmoth proliferate activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Yawgmoth proliferate should open next-cost chooser");

        assert!(
            matches!(
                wasm.pending_decision,
                Some(DecisionContext::SelectOptions(_))
            ),
            "Yawgmoth proliferate should begin on a next-cost chooser"
        );

        wasm
    }

    let mut mana_wasm = setup_proliferate_prompt();
    mana_wasm
        .dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [0],
            }))
            .expect("next-cost mana choice should serialize"),
        )
        .expect("choosing Yawgmoth's mana cost should advance to mana payment");

    let mana_ctx = match mana_wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected mana payment prompt after choosing mana, got {other:?}"),
    };
    assert!(
        mana_ctx.description.to_lowercase().contains("pay mana pip"),
        "mana choice should advance to mana pip payment, got description: {}",
        mana_ctx.description
    );
    assert!(
        mana_ctx
            .options
            .iter()
            .any(|option| option.legal && option.description.contains("Black Lotus")),
        "mana payment prompt should offer Black Lotus"
    );

    let mut discard_wasm = setup_proliferate_prompt();
    discard_wasm
        .dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("next-cost discard choice should serialize"),
        )
        .expect("choosing Yawgmoth's discard cost should advance to discard selection");

    let discard_ctx = match discard_wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => {
            panic!("expected discard selection prompt after choosing discard, got {other:?}")
        }
    };
    assert!(
        discard_ctx.description.to_lowercase().contains("discard"),
        "discard choice should advance to discard selection, got description: {}",
        discard_ctx.description
    );
    assert_eq!(discard_ctx.min, 1);
    assert_eq!(discard_ctx.max, Some(1));
}

#[test]
fn stack_snapshot_includes_controller_and_targets() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let mountain_id = wasm
        .game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let play_mountain_index = priority_ctx
        .actions
        .iter()
        .position(
            |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == mountain_id),
        )
        .expect("expected play mountain action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": play_mountain_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("playing mountain should succeed");

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision after land play, got {other:?}"),
    };
    let cast_bolt_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == bolt_id
            )
        })
        .expect("expected cast lightning bolt action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_bolt_index,
        }))
        .expect("cast spell command should serialize"),
    )
    .expect("casting lightning bolt should enter its decision chain");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
        "lightning bolt cast should be waiting on targets"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "player", "player": bob.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing the lightning bolt target should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let stack_entry = snapshot
        .stack_objects
        .first()
        .expect("snapshot should include the cast lightning bolt on the stack");

    assert_eq!(stack_entry.name, "Lightning Bolt");
    assert_eq!(stack_entry.controller, alice.0);
    assert_eq!(stack_entry.targets.len(), 1);
    match &stack_entry.targets[0] {
        TargetChoiceView::Player { player, name } => {
            assert_eq!(*player, bob.0);
            assert_eq!(name, "Bob");
        }
        other => panic!("expected player target on stack snapshot, got {other:?}"),
    }
}

#[test]
fn wasm_stubborn_denial_can_target_and_counter_lightning_bolt() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let stubborn_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Stubborn Denial".to_string(), "hand".to_string(), true)
            .expect("Stubborn Denial should be loadable into Alice's hand"),
    );
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    let ferocious_creature = CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    wasm.game
        .create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
    let bolt_stable_id = wasm
        .game
        .object(bolt_id)
        .expect("Lightning Bolt should exist on the stack")
        .stable_id;
    wasm.game
        .push_to_stack(StackEntry::new(bolt_id, bob).with_targets(vec![Target::Player(alice)]));

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == stubborn_id),
    );

    let targets_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Stubborn Denial target prompt, got {other:?}"),
    };
    assert!(
        targets_ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter())
            .any(|target| *target == Target::Object(bolt_id)),
        "Stubborn Denial should expose Lightning Bolt as a legal noncreature spell target"
    );

    let snapshot_json = wasm
        .snapshot_json()
        .expect("target prompt snapshot should render");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot JSON should parse");
    let snapshot_targets = snapshot["decision"]["requirements"][0]["legal_targets"]
        .as_array()
        .expect("target prompt should serialize legal targets");
    assert!(
        snapshot_targets.iter().any(|target| {
            target["kind"].as_str() == Some("object")
                && target["object"].as_u64() == Some(bolt_id.0)
                && target["name"].as_str() == Some("Lightning Bolt")
        }),
        "WASM snapshot should expose Lightning Bolt as a clickable Stubborn Denial target: {snapshot_targets:?}"
    );

    dispatch_select_target_object(&mut wasm, bolt_id);

    for _ in 0..8 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => {
                let option_index = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .map(|option| option.index)
                    .unwrap_or_else(|| panic!("expected a legal payment option, got {ctx:?}"));
                dispatch_select_options(&mut wasm, &[option_index]);
            }
            Some(DecisionContext::Priority(_)) => {
                dispatch_pass_priority(&mut wasm);
                if let Some(current_bolt_id) = wasm.game.find_object_by_stable_id(bolt_stable_id)
                    && wasm
                        .game
                        .object(current_bolt_id)
                        .is_some_and(|object| object.zone == Zone::Graveyard)
                {
                    break;
                }
            }
            Some(other) => panic!("unexpected Stubborn Denial follow-up decision: {other:?}"),
            None => break,
        }
    }

    let countered_bolt_id = wasm
        .game
        .find_object_by_stable_id(bolt_stable_id)
        .expect("countered Lightning Bolt should still be tracked");
    assert_eq!(
        wasm.game
            .object(countered_bolt_id)
            .expect("Lightning Bolt should still exist")
            .zone,
        Zone::Graveyard,
        "Stubborn Denial should counter Lightning Bolt through the WASM dispatch flow"
    );
    assert_eq!(
        wasm.game.player(alice).expect("Alice should exist").life,
        20,
        "countered Lightning Bolt should not resolve and damage Alice"
    );
}

#[test]
fn wasm_dispatch_failed_counter_allows_protected_spell_to_resolve() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::SecondMain;
    wasm.game.turn.step = None;

    let goblin = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let goblin_id = wasm
        .game
        .create_object_from_card(&goblin, alice, Zone::Battlefield);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), alice, Zone::Stack);
    wasm.game.add_temporary_spell_ability_grant(
        alice,
        bolt_id,
        ironsmith::target::ObjectFilter::instant_or_sorcery().cast_by(ironsmith::PlayerFilter::You),
        StaticAbility::cant_be_countered_ability(),
        1,
    );
    wasm.game
        .consume_temporary_spell_ability_grants_for_spell(bolt_id, alice);
    wasm.game.push_to_stack(
        StackEntry::new(bolt_id, alice)
            .with_targets(vec![Target::Object(goblin_id)])
            .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
                spec: ironsmith::target::ChooseSpec::AnyTarget,
                range: 0..1,
            }]),
    );

    let counterspell = CardDefinitionBuilder::new(CardId::new(), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell.")
        .expect("counter spell should parse");
    let counter_id = wasm
        .game
        .create_object_from_definition(&counterspell, alice, Zone::Stack);
    wasm.game.push_to_stack(
        StackEntry::new(counter_id, alice)
            .with_targets(vec![Target::Object(bolt_id)])
            .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
                spec: ironsmith::target::ChooseSpec::spell(),
                range: 0..1,
            }]),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .iter()
            .any(|id| wasm
                .game
                .object(*id)
                .is_some_and(|object| object.name == "Raging Goblin")),
        "failed counter should leave the protected spell to deal lethal damage"
    );
}

#[test]
fn duress_snapshot_keeps_revealed_hand_visible_during_discard_choice() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let duress_id = wasm
        .add_card_to_zone(0, "Duress".to_string(), "hand".to_string(), true)
        .expect("should add Duress to hand");
    wasm.add_card_to_zone(
        0,
        "Black Lotus".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("should add Black Lotus to battlefield");

    let hydra_id = wasm
        .add_card_to_zone(1, "Ulvenwald Hydra".to_string(), "hand".to_string(), true)
        .expect("should add Ulvenwald Hydra to hand");
    let peek_id = wasm
        .add_card_to_zone(1, "Peek".to_string(), "hand".to_string(), true)
        .expect("should add Peek to hand");
    let keyrune_id = wasm
        .add_card_to_zone(1, "Dimir Keyrune".to_string(), "hand".to_string(), true)
        .expect("should add Dimir Keyrune to hand");
    let forest_id = wasm
        .add_card_to_zone(1, "Forest".to_string(), "hand".to_string(), true)
        .expect("should add Forest to hand");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let cast_duress_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == ObjectId::from_raw(duress_id)
                )
            })
            .expect("expected cast Duress action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_duress_index,
        }))
        .expect("cast spell command should serialize"),
    )
    .expect("casting Duress should enter its decision chain");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
        "Duress should be waiting on targets after cast"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "player", "player": bob.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing the Duress target should succeed");

    loop {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(options)) => {
                let option_index = options
                    .options
                    .iter()
                    .find(|option| option.legal && option.description.contains("Black Lotus"))
                    .or_else(|| options.options.iter().find(|option| option.legal))
                    .map(|option| option.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "expected a legal mana-payment option, got {:?}",
                            options
                                .options
                                .iter()
                                .map(|option| option.description.clone())
                                .collect::<Vec<_>>()
                        )
                    });
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [option_index],
                    }))
                    .expect("option choice command should serialize"),
                )
                .expect("payment choice should succeed");
            }
            Some(DecisionContext::SelectObjects(_)) => break,
            Some(other) => panic!("unexpected Duress follow-up decision: {other:?}"),
            None => panic!("Duress resolved without presenting the discard decision"),
        }
    }

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );

    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("Duress discard prompt should keep revealed cards in snapshot");
    assert_eq!(viewed_cards.visibility, "public");
    assert_eq!(viewed_cards.subject, bob.0);
    assert_eq!(
        viewed_cards.card_ids,
        vec![hydra_id, peek_id, keyrune_id, forest_id],
        "snapshot should surface every revealed hand card, not only legal discard choices"
    );

    let decision = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the pending discard choice")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects decision, got {other:?}"),
    };
    let candidate_ids: Vec<u64> = decision.iter().map(|candidate| candidate.id).collect();
    assert_eq!(
        candidate_ids,
        vec![peek_id, keyrune_id],
        "discard decision should only offer the legal noncreature nonland cards"
    );
}

#[test]
fn gitaxian_probe_snapshot_keeps_looked_at_hand_visible_after_draw() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.runner_awaiting_priority = true;

    let probe = compile_to_runtime_definition(
        "Gitaxian Probe",
        "Mana Cost: {U/P}\nType: Sorcery\nLook at target player's hand.\nDraw a card.",
        false,
    )
    .expect("Gitaxian Probe should compile");
    let probe_id = wasm
        .game
        .create_object_from_definition(&probe, alice, Zone::Hand);
    wasm.game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), bob, Zone::Hand);
    let mountain_id = wasm
        .game
        .create_object_from_definition(&basic_mountain(), bob, Zone::Hand);
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == probe_id),
    );
    dispatch_select_target_player(&mut wasm, bob);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(options)) => {
            let option_index = options
                .options
                .iter()
                .find(|option| option.legal && option.description.contains("2 life"))
                .map(|option| option.index)
                .unwrap_or_else(|| {
                    panic!(
                        "expected phyrexian life payment option, got {:?}",
                        options
                            .options
                            .iter()
                            .map(|option| option.description.clone())
                            .collect::<Vec<_>>()
                    )
                });
            dispatch_select_options(&mut wasm, &[option_index]);
        }
        other => panic!("expected Gitaxian Probe payment option, got {other:?}"),
    }

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );

    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("Gitaxian Probe should keep the looked-at hand in the next snapshot");
    assert_eq!(viewed_cards.visibility, "private");
    assert_eq!(viewed_cards.viewer, alice.0);
    assert_eq!(viewed_cards.subject, bob.0);
    assert_eq!(viewed_cards.zone, "Hand");
    assert_eq!(viewed_cards.card_ids, vec![bolt_id.0, mountain_id.0]);
    assert_eq!(
        viewed_cards
            .cards
            .iter()
            .map(|card| card.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Lightning Bolt", "Mountain"]
    );
}

#[test]
fn stack_snapshot_keeps_reveal_cost_card_visible_while_spell_is_on_stack() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed = CardBuilder::new(CardId::from_raw(701), "Merfolk Scout")
        .card_types(vec![CardType::Creature])
        .build();
    let revealed_id = game.create_object_from_card(&revealed, bob, Zone::Hand);

    let spell = CardBuilder::new(CardId::from_raw(702), "Silvergill Variant")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&spell, bob, Zone::Stack);

    let snapshot = {
        let obj = game.object(revealed_id).expect("revealed hand card");
        ObjectSnapshot::from_object(obj, &game)
    };
    let mut tagged = std::collections::HashMap::new();
    tagged.insert(
        ironsmith::tag::TagKey::from(ironsmith::effects::PUBLIC_REVEALED_TAG),
        vec![snapshot],
    );
    game.push_to_stack(StackEntry::new(spell_id, bob).with_tagged_objects(tagged));

    let snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("snapshot should include Bob");
    assert!(bob_snapshot.can_view_hand);
    assert!(
        bob_snapshot
            .hand_cards
            .iter()
            .any(|card| card.id == revealed_id.0),
        "revealed cost card should stay visible while the spell is on the stack"
    );

    let viewed = snapshot
        .viewed_cards
        .as_ref()
        .expect("stack reveal should populate viewed cards");
    assert_eq!(viewed.visibility, "public");
    assert_eq!(viewed.subject, bob.0);
    assert_eq!(viewed.card_ids, vec![revealed_id.0]);
}

#[test]
fn stack_snapshot_keeps_hidden_zone_activation_source_visible() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = CardBuilder::new(CardId::from_raw(703), "Street Wraith Variant")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, bob, Zone::Hand);
    let source_snapshot = {
        let obj = game.object(source_id).expect("hidden-zone source");
        ObjectSnapshot::from_object(obj, &game)
    };

    let entry = StackEntry::ability(
        source_id,
        bob,
        ironsmith::resolution::ResolutionProgram::default(),
    )
    .with_source_info(source_snapshot.stable_id, source_snapshot.name.clone())
    .with_source_snapshot(source_snapshot);
    game.push_to_stack(entry);

    let snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("snapshot should include Bob");
    assert!(bob_snapshot.can_view_hand);
    assert!(
        bob_snapshot
            .hand_cards
            .iter()
            .any(|card| card.id == source_id.0),
        "the source of an ability activated from hand should stay revealed on the stack"
    );
}

#[test]
fn tayam_black_lotus_color_choice_keeps_paid_mana_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let ornithopter_ids: Vec<ObjectId> = (0..3)
        .map(|_| {
            ObjectId::from_raw(
                wasm.add_card_to_zone(
                    alice.0,
                    "Ornithopter".to_string(),
                    "battlefield".to_string(),
                    false,
                )
                .expect("should add Ornithopter to battlefield"),
            )
        })
        .collect();
    let lotus_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Black Lotus to battlefield"),
    );

    for ornithopter_id in &ornithopter_ids {
        let ornithopter = wasm
            .game
            .object(*ornithopter_id)
            .expect("ornithopter should exist");
        assert_eq!(
            ornithopter
                .counters
                .get(&ironsmith::object::CounterType::Vigilance)
                .copied(),
            Some(1),
            "Tayam should grant each Ornithopter a vigilance counter"
        );
    }

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
    };
    let mana_choice = next_cost_ctx
        .options
        .iter()
        .find(|option| option.legal && option.description.contains("Pay {3}"))
        .map(|option| option.index)
        .unwrap_or(0);

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [mana_choice],
        }))
        .expect("next-cost choice command should serialize"),
    )
    .expect("choosing Tayam's mana cost should advance to mana payment");

    let mana_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected mana payment prompt after choosing mana, got {other:?}"),
    };
    let lotus_option = mana_ctx
        .options
        .iter()
        .find(|option| option.legal && option.description.contains("Black Lotus"))
        .map(|option| option.index)
        .expect("mana payment prompt should offer Black Lotus");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [lotus_option],
        }))
        .expect("Black Lotus mana payment command should serialize"),
    )
    .expect("activating Black Lotus during Tayam payment should succeed");

    assert!(
        !wasm.game.battlefield.contains(&lotus_id),
        "Black Lotus should be sacrificed immediately once selected"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Colors(_))),
        "Black Lotus should surface a color-choice prompt"
    );

    let colors_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Colors(ctx)) => ctx,
        other => panic!("expected color-choice decision, got {other:?}"),
    };
    let green_option = colors_for_context(colors_ctx)
        .iter()
        .position(|color| *color == ironsmith::color::Color::Green)
        .expect("green should be a legal Black Lotus color choice");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [green_option],
        }))
        .expect("color choice command should serialize"),
    )
    .expect("choosing a Black Lotus color should replay the payment chain");

    assert!(
        !wasm.game.battlefield.contains(&lotus_id),
        "Black Lotus should remain sacrificed after the replayed color choice resolves"
    );
    let pool = &wasm
        .game
        .player(alice)
        .expect("alice should exist")
        .mana_pool;
    assert_eq!(
        pool.green, 2,
        "one of the three chosen mana should pay the current generic pip and two should remain"
    );

    let pending_activation = wasm
        .priority_state
        .pending_activation
        .as_ref()
        .expect("Tayam activation should still be in progress");
    assert_eq!(
        pending_activation.remaining_mana_pips.len(),
        2,
        "paying Black Lotus into Tayam should consume exactly one generic pip"
    );
    assert!(
        matches!(
            wasm.pending_decision,
            Some(DecisionContext::SelectOptions(_))
        ),
        "after choosing the color, the UI should advance to the next payment prompt"
    );
}

#[test]
fn tayam_counter_choice_keeps_removed_counters_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let ornithopter_ids: Vec<ObjectId> = (0..3)
        .map(|_| {
            ObjectId::from_raw(
                wasm.add_card_to_zone(
                    alice.0,
                    "Ornithopter".to_string(),
                    "battlefield".to_string(),
                    false,
                )
                .expect("should add Ornithopter to battlefield"),
            )
        })
        .collect();

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
    };
    let counter_choice = next_cost_ctx
        .options
        .iter()
        .find(|option| option.legal && option.description.contains("Remove three counters"))
        .map(|option| option.index)
        .unwrap_or(1);

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [counter_choice],
        }))
        .expect("counter-cost choice command should serialize"),
    )
    .expect("choosing Tayam's counter cost should open distribution");

    let distribute_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Distribute(ctx)) => ctx,
        other => panic!("expected counter distribution prompt, got {other:?}"),
    };
    let distribution_indices: Vec<usize> = ornithopter_ids
        .iter()
        .map(|ornithopter_id| {
            distribute_ctx
                .targets
                .iter()
                .position(|target| target.target == Target::Object(*ornithopter_id))
                .expect("each Ornithopter should be a legal distribution target")
        })
        .collect();

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": distribution_indices,
        }))
        .expect("distribution command should serialize"),
    )
    .expect("distributing Tayam's counters across the Ornithopters should succeed");

    for ornithopter_id in &ornithopter_ids {
        let counters_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Counters(ctx)) => ctx,
            other => panic!("expected counter-removal prompt, got {other:?}"),
        };
        assert_eq!(
            counters_ctx.target, *ornithopter_id,
            "counter-removal replay should advance through the distributed targets in order"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [0],
            }))
            .expect("counter selection command should serialize"),
        )
        .expect("removing the selected vigilance counter should succeed");

        let ornithopter = wasm
            .game
            .object(*ornithopter_id)
            .expect("ornithopter should still exist");
        assert_eq!(
            ornithopter
                .counters
                .get(&ironsmith::object::CounterType::Vigilance)
                .copied()
                .unwrap_or(0),
            0,
            "selected Ornithopter should keep its counter removed after replay"
        );
    }

    let pending_activation = wasm
        .priority_state
        .pending_activation
        .as_ref()
        .expect("Tayam activation should still be in progress");
    assert!(
        pending_activation.remaining_cost_steps.is_empty(),
        "after removing all three counters, the counter-payment step should be complete"
    );
    assert!(
        matches!(
            wasm.pending_decision,
            Some(DecisionContext::SelectOptions(_))
        ),
        "after paying the counter cost, the UI should advance to the remaining mana payment"
    );
}

#[test]
fn tayam_activation_can_resolve_and_choose_graveyard_return_target() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let wall_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Wall of Roots".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Wall of Roots to battlefield"),
    );
    let ornithopter_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Ornithopter to battlefield"),
    );
    let forest_a = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add first Forest to battlefield"),
    );
    let forest_b = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add second Forest to battlefield"),
    );
    let return_target = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "graveyard".to_string(),
            false,
        )
        .expect("should add return target to graveyard"),
    );

    assert!(
        wasm.game.player(bob).is_some(),
        "second player should exist for priority passing"
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    loop {
        let pending = wasm
            .pending_decision
            .clone()
            .expect("Tayam activation should still have a pending decision");
        match pending {
            DecisionContext::SelectOptions(ctx) => {
                let choice = if ctx.description.contains("Choose next cost") {
                    ctx.options
                        .iter()
                        .find(|option| option.legal && option.description.contains("Pay {3}"))
                        .map(|option| option.index)
                        .expect("next-cost chooser should offer the mana payment")
                } else if ctx.description.contains("Pay mana") {
                    if let Some(option) = ctx
                        .options
                        .iter()
                        .find(|option| option.legal && option.description.contains("Wall of Roots"))
                    {
                        option.index
                    } else {
                        ctx.options
                            .iter()
                            .find(|option| option.legal && option.description.contains("Forest"))
                            .map(|option| option.index)
                            .expect("mana payment prompt should offer a legal mana source")
                    }
                } else if ctx.description.contains("Choose next cost") {
                    unreachable!("handled above")
                } else {
                    ctx.options
                        .iter()
                        .find(|option| {
                            option.legal && option.description.contains("Remove three counters")
                        })
                        .map(|option| option.index)
                        .or_else(|| {
                            ctx.options
                                .iter()
                                .find(|option| option.legal && option.description.contains("Pass"))
                                .map(|option| option.index)
                        })
                        .unwrap_or_else(|| {
                            ctx.options
                                .iter()
                                .find(|option| option.legal)
                                .map(|option| option.index)
                                .expect("select-options prompt should offer a legal choice")
                        })
                };

                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [choice],
                    }))
                    .expect("select-options command should serialize"),
                )
                .expect("dispatching Tayam select-options step should succeed");
            }
            DecisionContext::Distribute(ctx) => {
                let wall_index = ctx
                    .targets
                    .iter()
                    .position(|target| target.target == Target::Object(wall_id))
                    .expect("Wall of Roots should be a legal distribution target");
                let ornithopter_index = ctx
                    .targets
                    .iter()
                    .position(|target| target.target == Target::Object(ornithopter_id))
                    .expect("Ornithopter should be a legal distribution target");
                let indices = vec![wall_index, wall_index, ornithopter_index];
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": indices,
                    }))
                    .expect("distribute command should serialize"),
                )
                .expect("counter distribution should succeed");
            }
            DecisionContext::Counters(ctx) => {
                let counter_index = ctx
                    .available_counters
                    .iter()
                    .position(|(_, available)| *available > 0)
                    .expect("counter prompt should offer at least one removable counter");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [counter_index],
                    }))
                    .expect("counter selection command should serialize"),
                )
                .expect("counter removal should succeed");
            }
            DecisionContext::Priority(ctx) => {
                let pass_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::PassPriority))
                    .expect("priority prompt should include pass");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "priority_action",
                        "action_index": pass_index,
                    }))
                    .expect("priority pass command should serialize"),
                )
                .expect("priority pass during Tayam line should succeed");
            }
            DecisionContext::SelectObjects(ctx) => {
                let target_id = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.id == return_target)
                    .map(|candidate| candidate.id.0)
                    .expect("graveyard return target should be legal");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_objects",
                        "object_ids": [target_id],
                    }))
                    .expect("graveyard target command should serialize"),
                )
                .expect("selecting Tayam's graveyard return target should succeed");
                break;
            }
            other => panic!("unexpected Tayam resolution decision: {other:?}"),
        }
    }

    assert!(
        !wasm.game.battlefield.contains(&forest_a) || !wasm.game.battlefield.contains(&forest_b),
        "at least one Forest should remain tapped after paying Tayam's mana cost"
    );
    assert!(
        wasm.game.battlefield.iter().any(|id| {
            wasm.game
                .object(*id)
                .is_some_and(|obj| obj.name == "Forest" && obj.owner == alice)
        }),
        "a Forest should still exist on the battlefield after Tayam resolves"
    );
}

#[test]
fn polluted_delta_resolution_choice_keeps_paid_costs_and_resolved_land() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let delta_id =
        wasm.game
            .create_object_from_definition(&polluted_delta(), alice, Zone::Battlefield);
    let island_id = wasm
        .game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    wasm.game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Library);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == delta_id
            )
        })
        .expect("expected Polluted Delta activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Polluted Delta should succeed");

    assert!(
        wasm.game.player(bob).is_some(),
        "second player should exist for the pass-priority sequence"
    );
    assert!(
        !wasm.game.battlefield.contains(&delta_id),
        "Polluted Delta should be sacrificed during activation"
    );
    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .contains(&delta_id),
        "Polluted Delta should be in the graveyard after activation"
    );
    assert_eq!(
        wasm.game.player(alice).expect("alice should exist").life,
        19,
        "Polluted Delta activation should pay 1 life immediately"
    );

    loop {
        let pending = wasm
            .pending_decision
            .clone()
            .expect("fetchland line should keep producing prompts until the search resolves");
        match pending {
            DecisionContext::Priority(ctx) => {
                let pass_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::PassPriority))
                    .expect("priority prompt should include pass");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "priority_action",
                        "action_index": pass_index,
                    }))
                    .expect("priority pass command should serialize"),
                )
                .expect("passing priority during fetchland line should succeed");
            }
            DecisionContext::SelectObjects(ctx) => {
                let choice = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.id == island_id)
                    .map(|candidate| candidate.id.0)
                    .expect("basic Island should be a legal fetchland search result");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_objects",
                        "object_ids": [choice],
                    }))
                    .expect("fetchland selection command should serialize"),
                )
                .expect("choosing the searched land should succeed");
                break;
            }
            other => panic!("unexpected Polluted Delta follow-up decision: {other:?}"),
        }
    }

    assert_eq!(
        wasm.game.player(alice).expect("alice should exist").life,
        19,
        "resolving the fetchland search should not rewind the paid life cost"
    );
    assert!(
        !wasm.game.battlefield.contains(&delta_id),
        "resolving the fetchland search should not put Polluted Delta back onto the battlefield"
    );
    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .contains(&delta_id),
        "Polluted Delta should remain in the graveyard after the search completes"
    );
    assert!(
        wasm.game.battlefield.contains(&island_id),
        "the chosen Island should enter the battlefield"
    );
    assert!(
        !wasm
            .game
            .player(alice)
            .expect("alice should exist")
            .library
            .contains(&island_id),
        "the chosen Island should leave the library after resolution"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "after the search resolves, the game should return to priority"
    );
}

#[test]
fn committed_resolution_prompt_is_not_cancelable() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;
    wasm.pending_decision = Some(DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Resolve effect",
        vec![SelectableObject::new(ObjectId::from_raw(1), "Choice")],
        1,
        Some(1),
    )));
    assert!(
        wasm.pending_action_checkpoint.is_none(),
        "committed follow-up prompts should not retain the action-chain undo checkpoint"
    );
    assert!(
        !wasm.is_cancelable(),
        "once the spell has resolved into its imprint prompt, undo should be disabled"
    );
    assert!(
        wasm.cancel_decision().is_err(),
        "non-cancelable prompts must reject direct cancelDecision calls"
    );
}

#[test]
fn emrakul_cast_trigger_needs_targets_in_four_player_game() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    assert_eq!(
        wasm.trigger_queue.entries.len(),
        1,
        "Emrakul should queue its cast trigger from the stack"
    );

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");

    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    assert_eq!(
        targets_ctx.player, alice,
        "the caster should choose Emrakul's target opponent"
    );
    assert_eq!(
        targets_ctx.requirements.len(),
        1,
        "Emrakul should ask for exactly one target requirement"
    );

    let legal_targets = &targets_ctx.requirements[0].legal_targets;
    let legal_players: Vec<PlayerId> = legal_targets
        .iter()
        .filter_map(|target| match target {
            ironsmith::game_state::Target::Player(player) => Some(*player),
            ironsmith::game_state::Target::Object(_) => None,
        })
        .collect();
    assert_eq!(
        legal_players,
        vec![bob, charlie, dana],
        "all opponents should be legal Emrakul targets"
    );

    assert_eq!(
        wasm.game.stack.len(),
        1,
        "replay should leave the live game advanced to the pending target decision"
    );
}

#[test]
fn auto_advance_target_prompt_dispatch_reexecutes_replay_root() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [{ "kind": "player", "player": bob.0 }],
        }))
        .expect("target selection should serialize"),
    )
    .expect("dispatching replay-backed targets should succeed");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "after choosing Emrakul's target, auto-advance should continue to priority"
    );
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "choosing the trigger target should put Emrakul's cast trigger onto the stack"
    );
}

#[test]
fn emrakul_target_prompt_snapshot_shows_pending_triggered_ability() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot json should render pending Emrakul trigger");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");

    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include stack objects");
    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should show spell plus cast trigger"
    );
    assert_eq!(stack_objects[0]["name"], "Emrakul, the Promised End");
    assert_eq!(stack_objects[0]["ability_kind"], "Triggered");
    assert!(
        stack_objects[0]["ability_text"]
            .as_str()
            .is_some_and(|text| text.to_ascii_lowercase().contains("target opponent")),
        "pending trigger snapshot should describe Emrakul's cast trigger"
    );
    assert_eq!(stack_objects[1]["name"], "Emrakul, the Promised End");
    assert!(
        stack_objects[1]["ability_kind"].is_null(),
        "the second stack object should remain the Emrakul spell"
    );
}

#[test]
fn emrakul_target_prompt_snapshot_encodes_for_js_with_safe_stack_ids() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    let snapshot_value = wasm
        .snapshot()
        .expect("snapshot should encode for JS with safe stack ids");
    let snapshot: serde_json::Value =
        serde_wasm_bindgen::from_value(snapshot_value).expect("snapshot value should parse");
    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include stack objects");

    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should keep both stack entries"
    );
    for entry in stack_objects {
        let id = entry["id"]
            .as_u64()
            .expect("stack entry id should be a JS-safe integer");
        assert!(
            id <= 9_007_199_254_740_991,
            "stack entry id should stay within JS safe integer range, got {id}"
        );
    }

    let triggered_id = stack_objects[0]["id"]
        .as_u64()
        .expect("triggered ability id should exist");
    let spell_id = stack_objects[1]["id"]
        .as_u64()
        .expect("spell id should exist");
    assert_ne!(
        triggered_id, spell_id,
        "triggered ability and spell should keep distinct UI ids"
    );
}

#[test]
fn target_prompt_snapshot_shows_all_queued_targeted_triggers_while_spell_resolves() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let blood_artist_id =
        wasm.game
            .create_object_from_definition(&blood_artist(), alice, Zone::Battlefield);
    let victim_id =
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    let victim_snapshot = wasm
        .game
        .object(victim_id)
        .map(|object| ironsmith::snapshot::ObjectSnapshot::from_object(object, &wasm.game))
        .expect("victim snapshot should exist");
    let dies_event = TriggerEvent::new_with_provenance(
        ironsmith::events::ZoneChangeEvent::with_cause(
            victim_id,
            Zone::Battlefield,
            Zone::Graveyard,
            ironsmith::events::cause::EventCause::from_sba(),
            Some(victim_snapshot),
        ),
        ProvNodeId::default(),
    );

    let trigger = check_triggers(&wasm.game, &dies_event)
        .into_iter()
        .find(|entry| entry.source == blood_artist_id)
        .expect("Blood Artist should trigger when another creature dies");
    wasm.trigger_queue.add(trigger.clone());
    wasm.trigger_queue.add(trigger);

    let culling_id =
        wasm.game
            .create_object_from_definition(&culling_the_weak(), alice, Zone::Stack);
    let culling_snapshot = build_stack_object_snapshot(
        &wasm.game,
        wasm.perspective,
        None,
        &StackEntry::new(culling_id, alice),
    );
    wasm.active_resolving_stack_object = Some(culling_snapshot);

    wasm.pending_decision = Some(DecisionContext::Targets(TargetsContext::new(
        alice,
        blood_artist_id,
        "Blood Artist's triggered ability".to_string(),
        vec![TargetRequirementContext {
            description: "target for Blood Artist".to_string(),
            legal_targets: vec![Target::Player(alice), Target::Player(bob)],
            min_targets: 1,
            max_targets: Some(1),
        }],
    )));

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot should render queued Blood Artist triggers");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");

    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include queued stack objects");
    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should show both queued Blood Artist triggers"
    );
    assert!(
        stack_objects
            .iter()
            .all(|entry| entry["name"] == "Blood Artist" && entry["ability_kind"] == "Triggered"),
        "queued stack objects should both be Blood Artist triggers: {stack_objects:?}"
    );
    assert_ne!(
        stack_objects[0]["id"], stack_objects[1]["id"],
        "queued trigger previews should keep distinct UI ids"
    );

    let resolving = snapshot["resolving_stack_object"]
        .as_object()
        .expect("resolving spell should remain visible separately");
    assert_eq!(resolving["name"], "Culling the Weak");
}

#[test]
fn roaming_throne_blood_artist_culling_flow_reaches_two_trigger_ordering_options() {
    let mut wasm = WasmGame::new();

    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    wasm.add_card_to_zone(
        0,
        "Roaming Throne".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should start Roaming Throne battlefield entry");

    let vampire_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx
            .options
            .iter()
            .find(|option| option.description == "Vampire")
            .map(|option| option.index)
            .expect("Vampire should be a legal creature type"),
        other => panic!("expected Roaming Throne type selection, got {other:?}"),
    };
    dispatch_select_options(&mut wasm, &[vampire_index]);

    wasm.add_card_to_zone(
        0,
        "Blood Artist".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should add Blood Artist to the battlefield");

    let culling_id = wasm
        .add_card_to_zone(0, "Culling the Weak".to_string(), "hand".to_string(), false)
        .expect("should add Culling the Weak to hand");

    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));
    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == ObjectId::from_raw(culling_id)),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            let blood_artist_id = wasm
                .game
                .battlefield
                .iter()
                .find_map(|id| {
                    wasm.game
                        .object(*id)
                        .filter(|obj| obj.name == "Blood Artist")
                        .map(|_| *id)
                })
                .expect("Blood Artist should be on the battlefield");
            dispatch_select_objects(&mut wasm, &[blood_artist_id.0]);
        }
        other => panic!("expected sacrifice target prompt for Culling the Weak, got {other:?}"),
    }

    let order_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Order(ctx)) => ctx,
        other => {
            panic!("expected trigger ordering prompt after sacrificing Blood Artist, got {other:?}")
        }
    };
    assert_eq!(
        order_ctx.items.len(),
        2,
        "Roaming Throne should create two Blood Artist ordering items"
    );
    assert!(
        order_ctx
            .items
            .iter()
            .all(|(_, label)| label.starts_with("Blood Artist\n")),
        "ordering labels should both be Blood Artist triggers: {:?}",
        order_ctx.items
    );

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot json should encode trigger ordering state");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");
    let decision = snapshot["decision"]
        .as_object()
        .expect("snapshot should include ordering decision");
    assert_eq!(decision["kind"], "select_options");
    assert_eq!(decision["reason"], "Order triggers");
    assert_eq!(
        decision["options"]
            .as_array()
            .expect("ordering decision should expose options")
            .len(),
        2,
        "UI decision payload should keep both Blood Artist trigger ordering options"
    );
    assert!(
        decision["options"]
            .as_array()
            .expect("ordering decision should expose options")
            .iter()
            .all(|option| option["description"]
                .as_str()
                .is_some_and(|description| description.starts_with("Blood Artist\n"))),
        "synthetic trigger-order options should expose their public labels: {decision:?}"
    );
}

#[test]
fn priority_decision_routing_uses_replay_for_generic_modal_choices() {
    let boolean = DecisionContext::Boolean(BooleanContext::new(
        PlayerId::from_index(0),
        None,
        "play an additional land this turn",
    ));
    let number = DecisionContext::Number(NumberContext::new(
        PlayerId::from_index(0),
        None,
        0,
        3,
        "choose a number",
    ));
    let targets = DecisionContext::Targets(TargetsContext::new(
        PlayerId::from_index(0),
        ObjectId::from_raw(1),
        "resolve trigger",
        vec![TargetRequirementContext {
            description: "target player".to_string(),
            legal_targets: vec![Target::Player(PlayerId::from_index(1))],
            min_targets: 1,
            max_targets: Some(1),
        }],
    ));
    let select_objects = DecisionContext::SelectObjects(SelectObjectsContext::new(
        PlayerId::from_index(0),
        None,
        "choose a land",
        vec![SelectableObject::new(ObjectId::from_raw(1), "Forest")],
        1,
        Some(1),
    ));
    let select_options =
        DecisionContext::SelectOptions(ironsmith::decisions::context::SelectOptionsContext::new(
            PlayerId::from_index(0),
            None,
            "choose a mode",
            vec![SelectableOption::new(0, "Only option")],
            1,
            1,
        ));
    let wasm = WasmGame::new();

    assert!(
        wasm.decision_requires_root_reexecution(&boolean),
        "boolean prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&number),
        "generic number prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&targets),
        "generic target prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&select_objects),
        "resolution-time object prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&select_options),
        "generic select-options prompts should replay from the original root response"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&select_options),
        "generic select-options prompts should route through replay continuations, not the live priority responder"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&number),
        "generic number prompts should not route through the live priority responder"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&targets),
        "generic target prompts should not route through the live priority responder"
    );
}

#[test]
fn priority_decision_routing_keeps_cost_option_prompts_on_live_responder() {
    let mut wasm = WasmGame::new();
    wasm.priority_state.pending_cast = Some(PendingCast::new(
        ObjectId::from_raw(1),
        Zone::Hand,
        PlayerId::from_index(0),
        ProvNodeId::default(),
        CastStage::ChoosingOptionalCosts,
        None,
        Vec::new(),
        CastingMethod::Normal,
        OptionalCostsPaid::new(1),
        None,
        ObjectId::from_raw(1),
    ));

    let select_options =
        DecisionContext::SelectOptions(ironsmith::decisions::context::SelectOptionsContext::new(
            PlayerId::from_index(0),
            Some(ObjectId::from_raw(1)),
            "Choose optional costs",
            vec![SelectableOption::new(0, "Kicker")],
            0,
            1,
        ));

    assert!(
        wasm.decision_uses_live_priority_response(&select_options),
        "cost-selection select-options prompts should stay on the live priority responder"
    );
}

#[test]
fn backdraft_wasm_flow_offers_resolved_sorcery_history_choice() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "Omniscience".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("should add Omniscience to battlefield");
    for _ in 0..3 {
        wasm.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Ornithopter to battlefield");
    }

    let blasphemous_act_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Blasphemous Act".to_string(), "hand".to_string(), true)
            .expect("should add Blasphemous Act to hand"),
    );
    let backdraft_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Backdraft".to_string(), "hand".to_string(), true)
            .expect("should add Backdraft to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let cast_blasphemous_act_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == blasphemous_act_id
                )
            })
            .expect("expected cast Blasphemous Act action"),
        other => {
            panic!("expected priority decision before casting Blasphemous Act, got {other:?}")
        }
    };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_blasphemous_act_index,
        }))
        .expect("cast Blasphemous Act command should serialize"),
    )
    .expect("casting Blasphemous Act should succeed");

    for _ in 0..4 {
        let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
            break;
        };
        let pass_index = ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("priority prompt should include pass");
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": pass_index,
            }))
            .expect("priority pass command should serialize"),
        )
        .expect("passing priority during Blasphemous Act should succeed");
        if wasm.game.stack.is_empty() {
            break;
        }
    }

    assert!(
        wasm.game.stack.is_empty(),
        "Blasphemous Act should be resolved before casting Backdraft"
    );
    let history_after_blasphemous = wasm
        .game
        .turn_store
        .turn_history
        .spell_cast_snapshot_history();
    let blasphemous_snapshots = history_after_blasphemous
        .iter()
        .filter(|snapshot| snapshot.name == "Blasphemous Act")
        .collect::<Vec<_>>();
    assert_eq!(
        blasphemous_snapshots.len(),
        1,
        "expected Blasphemous Act cast history to persist after resolution, got {:?}",
        history_after_blasphemous
            .iter()
            .map(|snapshot| (
                snapshot.name.clone(),
                snapshot.zone,
                snapshot.card_types.clone(),
                snapshot.cast_order_this_turn
            ))
            .collect::<Vec<_>>()
    );
    let blasphemous_cast_id = blasphemous_snapshots[0].object_id;
    assert_eq!(
        wasm.game
            .turn_store
            .turn_history
            .damage_dealt_by_spell_this_turn(wasm.game.provenance_graph(), blasphemous_cast_id),
        39,
        "Blasphemous Act should record 39 total damage from the three Ornithopters"
    );

    let cast_backdraft_index = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx
                .actions
                .iter()
                .position(|action| {
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == backdraft_id)
                })
                .expect("expected cast Backdraft action"),
            other => panic!("expected priority decision before casting Backdraft, got {other:?}"),
        };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_backdraft_index,
        }))
        .expect("cast Backdraft command should serialize"),
    )
    .expect("casting Backdraft should succeed");

    for _ in 0..4 {
        let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
            break;
        };
        let pass_index = ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("priority prompt should include pass");
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": pass_index,
            }))
            .expect("priority pass command should serialize"),
        )
        .expect("passing priority during Backdraft should succeed");
        if !matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))) {
            break;
        }
    }

    let first_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected Backdraft to stop for a player choice, got {other:?}"),
    };
    let first_legal = first_choice
        .options
        .iter()
        .filter(|option| option.legal)
        .collect::<Vec<_>>();
    assert_eq!(
        first_legal.len(),
        1,
        "expected only Alice to qualify for Backdraft's player choice, got {:?}",
        first_choice
            .options
            .iter()
            .map(|option| (option.index, option.description.clone(), option.legal))
            .collect::<Vec<_>>()
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [first_legal[0].index],
        }))
        .expect("single-player choice command should serialize"),
    )
    .expect("choosing the only qualifying Backdraft player should succeed");

    let spell_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected Backdraft to prompt for the historical spell, got {other:?}"),
    };
    let legal_spell_descriptions = spell_choice
        .options
        .iter()
        .filter(|option| option.legal)
        .map(|option| option.description.clone())
        .collect::<Vec<_>>();
    assert!(
        legal_spell_descriptions
            .iter()
            .any(|description| description.contains("Blasphemous Act")),
        "expected Blasphemous Act to remain a legal Backdraft history choice, got {:?}",
        legal_spell_descriptions
    );
    assert!(
        legal_spell_descriptions
            .iter()
            .any(|description| description.contains("Backdraft")),
        "expected Backdraft to also be present in the history choice, got {:?}",
        legal_spell_descriptions
    );
}

#[test]
fn cultivator_colossus_etb_does_not_repeat_may_prompt_before_next_land_choice() {
    let mut wasm = WasmGame::new();

    let forest_a = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("first Forest should be added to hand");
    let forest_b = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("second Forest should be added to hand");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("first library filler should be added");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("second library filler should be added");

    wasm.add_card_to_zone(
        0,
        "Cultivator Colossus".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("Cultivator Colossus should enter with ETB processing");

    let first_may = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected Cultivator Colossus may prompt, got {other:?}"),
    };
    assert!(
        first_may
            .description
            .to_ascii_lowercase()
            .contains("put a land card from your hand onto the battlefield tapped"),
        "expected Cultivator Colossus may text, got {:?}",
        first_may.description
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("yes choice should serialize"),
    )
    .expect("accepting the first Cultivator iteration should succeed");

    let first_land_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected first land selection prompt, got {other:?}"),
    };
    let mut first_candidates: Vec<u64> = first_land_choice
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id.0)
        .collect();
    first_candidates.sort_unstable();
    assert_eq!(
        first_candidates,
        vec![forest_a, forest_b],
        "first land selection should offer both lands in hand"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": [forest_a],
        }))
        .expect("first land selection should serialize"),
    )
    .expect("choosing the first land should succeed");

    assert_eq!(
        wasm.game
            .player(PlayerId::from_index(0))
            .expect("player should exist")
            .hand
            .len(),
        1,
        "after choosing a land, the live game state should keep that land out of hand"
    );
    let lands_on_battlefield = wasm
        .game
        .battlefield
        .iter()
        .filter(|&&id| {
            wasm.game
                .object(id)
                .is_some_and(|object| object.is_land() && object.owner == PlayerId::from_index(0))
        })
        .count();
    assert_eq!(
        lands_on_battlefield, 1,
        "the chosen land should already be on the battlefield before the next repeat decision"
    );

    let second_may = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected second Cultivator may prompt, got {other:?}"),
    };
    assert!(
        second_may
            .description
            .to_ascii_lowercase()
            .contains("put a land card from your hand onto the battlefield tapped"),
        "expected repeated Cultivator may text, got {:?}",
        second_may.description
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("second yes choice should serialize"),
    )
    .expect("accepting the second Cultivator iteration should succeed");

    let second_land_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected second land selection prompt, got {other:?}"),
    };
    let second_candidates: Vec<u64> = second_land_choice
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id.0)
        .collect();
    assert_eq!(
        second_candidates,
        vec![forest_b],
        "after one land is chosen, the next prompt should go straight to the remaining land"
    );
}

#[test]
fn doubling_chant_same_name_search_prompts_are_ui_friendly_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "Omniscience".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("Omniscience should be added to the battlefield");
    let battlefield_ornithopter = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("battlefield Ornithopter should be added"),
    );
    let library_ornithopter_a = wasm
        .add_card_to_zone(0, "Ornithopter".to_string(), "library".to_string(), true)
        .expect("first library Ornithopter should be added");
    let library_ornithopter_b = wasm
        .add_card_to_zone(0, "Ornithopter".to_string(), "library".to_string(), true)
        .expect("second library Ornithopter should be added");
    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Doubling Chant".to_string(), "hand".to_string(), true)
            .expect("Doubling Chant should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    let free_cast_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx
            .options
            .iter()
            .position(|option| option.description.contains("Without paying mana cost"))
            .expect("Doubling Chant should surface an Omniscience cast option"),
        other => panic!("expected Doubling Chant cast-method choice, got {other:?}"),
    };
    dispatch_select_options(&mut wasm, &[free_cast_index]);
    dispatch_pass_priority(&mut wasm);

    let may_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected Doubling Chant may prompt on resolution, got {other:?}"),
    };
    let may_text = may_ctx.description.to_ascii_lowercase();
    assert!(
        may_text
            .contains("search your library for a creature card with the same name as ornithopter"),
        "expected a user-facing Doubling Chant may prompt, got {:?}",
        may_ctx.description
    );
    assert!(
        !may_text.contains("tags it as 'searched'"),
        "Doubling Chant may prompt should not expose internal search tags: {:?}",
        may_ctx.description
    );

    dispatch_select_options(&mut wasm, &[1]);

    let select_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => {
            panic!("expected Doubling Chant library choice after accepting the may, got {other:?}")
        }
    };
    let select_text = select_ctx.description.to_ascii_lowercase();
    assert!(
        select_text
            .contains("search your library for a creature card with the same name as ornithopter"),
        "expected a user-facing Doubling Chant search prompt, got {:?}",
        select_ctx.description
    );
    assert_eq!(
        select_ctx.candidates.len(),
        2,
        "the search prompt should expose the two matching library Ornithopters"
    );
    assert!(
        select_ctx
            .candidates
            .iter()
            .all(|candidate| candidate.name == "Ornithopter"),
        "Doubling Chant search candidates should be the matching library cards"
    );
    let candidate_ids: Vec<u64> = select_ctx
        .candidates
        .iter()
        .map(|candidate| candidate.id.0)
        .collect();
    assert!(
        !candidate_ids.contains(&battlefield_ornithopter.0),
        "the battlefield Ornithopter should not appear in the library search candidates"
    );
    assert!(
        candidate_ids.contains(&library_ornithopter_a)
            && candidate_ids.contains(&library_ornithopter_b),
        "the search candidates should point at the library Ornithopter objects"
    );
}

#[test]
fn cascade_replay_surfaces_adventure_choice_after_accepting_free_cast() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let bloodbraid_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Bloodbraid Elf".to_string(), "hand".to_string(), true)
            .expect("Bloodbraid Elf should be added to hand"),
    );
    for _ in 0..3 {
        wasm.add_card_to_zone(0, "Mountain".to_string(), "battlefield".to_string(), true)
            .expect("Mountain should be added");
    }
    wasm.add_card_to_zone(0, "Forest".to_string(), "battlefield".to_string(), true)
        .expect("Forest should be added");
    wasm.add_card_to_zone(0, "Curious Pair".to_string(), "library".to_string(), true)
        .expect("Curious Pair should be added to library");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));
    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == bloodbraid_id),
    );

    for _ in 0..32 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx))
                if ctx.description.starts_with("Pay mana pip") =>
            {
                let index = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .expect("mana payment should have a legal option")
                    .index;
                dispatch_select_options(&mut wasm, &[index]);
            }
            Some(DecisionContext::Priority(_)) => dispatch_pass_priority(&mut wasm),
            Some(DecisionContext::Boolean(ctx))
                if ctx.description.contains("Cast Curious Pair without paying") =>
            {
                break;
            }
            other => panic!("expected mana, priority, or cascade may prompt, got {other:?}"),
        }
    }

    dispatch_select_options(&mut wasm, &[1]);

    let choose_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected cascade Adventure choice after accepting may, got {other:?}"),
    };
    assert!(
        choose_ctx
            .options
            .iter()
            .any(|option| option.description == "Cast Treats to Share"),
        "expected Cascade to offer the Adventure half, got {:?}",
        choose_ctx.options
    );
}

#[test]
fn saw_in_half_formidable_speaker_no_advances_resolution_chain() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "Omniscience".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("Omniscience should be added to the battlefield");

    let original_speaker_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Formidable Speaker".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("Formidable Speaker should enter and trigger"),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected Formidable Speaker may prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected Formidable Speaker ETB boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    let saw_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Saw in Half".to_string(), "hand".to_string(), true)
            .expect("Saw in Half should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == saw_id),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => {
            let target_ids: Vec<ObjectId> = ctx
                .requirements
                .iter()
                .flat_map(|req| req.legal_targets.iter())
                .filter_map(|target| match target {
                    Target::Object(object_id) => Some(*object_id),
                    _ => None,
                })
                .collect();
            assert!(
                target_ids.contains(&original_speaker_id),
                "Saw in Half should be able to target the original Formidable Speaker"
            );
        }
        other => panic!("expected Saw in Half target prompt, got {other:?}"),
    }

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [{ "kind": "object", "object": original_speaker_id.0 }],
        }))
        .expect("target selection should serialize"),
    )
    .expect("targeting Formidable Speaker should succeed");

    for _ in 0..8 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(_)) => dispatch_pass_priority(&mut wasm),
            _ => break,
        }
    }

    let order_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Order(ctx)) => ctx,
        other => {
            panic!("expected trigger ordering prompt after Saw in Half resolves, got {other:?}")
        }
    };
    assert_eq!(
        order_ctx.items.len(),
        2,
        "Saw in Half should produce exactly two Formidable Speaker ETB triggers"
    );

    dispatch_select_options(&mut wasm, &[0, 1]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(
                ctx.player, alice,
                "after ordering simultaneous triggers, the active player should receive the first new priority window"
            );
        }
        other => {
            panic!("expected a fresh priority window after ordering triggers, got {other:?}")
        }
    }
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "ordering triggers should not auto-resolve any stack entries"
    );

    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(
                ctx.player,
                PlayerId::from_index(1),
                "one pass should hand priority to the opponent without resolving a trigger"
            );
        }
        other => panic!("expected opponent priority after one pass, got {other:?}"),
    }
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "a single pass must not resolve the top trigger in multiplayer-style priority"
    );

    dispatch_pass_priority(&mut wasm);

    let first_boolean_source = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected first resolving Formidable Speaker prompt, got {:?}",
                ctx.description
            );
            ctx.source
        }
        other => panic!("expected first resolving boolean prompt, got {other:?}"),
    };
    assert_eq!(
        wasm.game.stack.len(),
        1,
        "exactly one trigger should have resolved after both players pass"
    );

    dispatch_select_options(&mut wasm, &[0]);

    let next_ctx = wasm
        .pending_decision
        .as_ref()
        .unwrap_or_else(|| panic!("expected another decision after declining the first trigger"));

    match next_ctx {
        DecisionContext::Boolean(ctx) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected the second Formidable Speaker prompt after declining the first, got {:?}",
                ctx.description
            );
            assert_ne!(
                ctx.source, first_boolean_source,
                "declining the first trigger should advance to the second trigger, not reissue the same source"
            );
        }
        other => panic!("expected the second Formidable Speaker boolean prompt, got {other:?}"),
    }
}

#[test]
fn live_resolution_follow_up_prompts_restore_resolving_stack_object() {
    let mut wasm = WasmGame::new();

    wasm.add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("first Forest should be added to hand");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("library filler should be added");

    wasm.add_card_to_zone(
        0,
        "Cultivator Colossus".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("Cultivator Colossus should enter with ETB processing");

    let resolving_checkpoint = wasm
        .pending_live_continuation
        .as_ref()
        .map(|continuation| continuation.checkpoint.clone())
        .expect("Cultivator ETB prompt should retain the committed resolution checkpoint");
    let next_ctx = wasm
        .pending_decision
        .clone()
        .expect("Cultivator ETB prompt should be pending");
    let expected_resolving_id = wasm
        .active_resolving_stack_object
        .as_ref()
        .map(|entry| entry.id)
        .expect("Cultivator ETB prompt should expose the resolving stack entry");

    wasm.clear_active_resolving_stack_object();
    assert!(
        wasm.active_resolving_stack_object.is_none(),
        "test setup should clear the resolving entry before simulating live dispatch"
    );

    wasm.finish_live_priority_dispatch(
        GameProgress::NeedsDecisionCtx(next_ctx),
        None,
        Some(resolving_checkpoint),
    )
    .expect("live follow-up prompt should snapshot cleanly");

    assert_eq!(
        wasm.active_resolving_stack_object
            .as_ref()
            .map(|entry| entry.id),
        Some(expected_resolving_id),
        "live follow-up prompts should restore the resolving stack entry from the committed resolution checkpoint"
    );
}

#[test]
fn tainted_pact_declining_first_card_advances_to_second_prompt_in_live_ui_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Tainted Pact".to_string(), "hand".to_string(), true)
            .expect("Tainted Pact should be added to hand"),
    );
    wasm.add_card_to_zone(0, "Second Card".to_string(), "library".to_string(), true)
        .expect("second library card should be added");
    wasm.add_card_to_zone(0, "First Card".to_string(), "library".to_string(), true)
        .expect("first library card should be added");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("first card"),
                "expected first Tainted Pact prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("second card"),
                "declining the first card should advance to the second prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected second Tainted Pact boolean prompt, got {other:?}"),
    }
}

#[test]
fn tainted_pact_declining_first_revealed_unique_card_prompts_for_second_card() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Tainted Pact".to_string(), "hand".to_string(), true)
            .expect("Tainted Pact should be added to hand"),
    );
    wasm.game
        .create_hidden_card_placeholder(alice, Zone::Library, 0, "alice-slot-0".to_string());
    wasm.game
        .create_hidden_card_placeholder(alice, Zone::Library, 1, "alice-slot-1".to_string());

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    wasm.reveal_hidden_slot(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "slot": 1,
            "cardName": "Tainted Pact",
            "commitment": "alice-slot-1",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("first exiled card should reveal");

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("tainted pact"),
                "expected first revealed Tainted Pact prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("hidden card")
                    || ctx.description.to_ascii_lowercase().contains("swamp"),
                "declining a unique first card should advance to the second prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected second Tainted Pact boolean prompt, got {other:?}"),
    }

    wasm.reveal_hidden_slot(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "slot": 0,
            "cardName": "Swamp",
            "commitment": "alice-slot-0",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("second exiled card should reveal");

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("swamp"),
                "revealing the second unique card should preserve the choice prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected revealed second Tainted Pact prompt, got {other:?}"),
    }
}

#[test]
fn reveal_hidden_position_uses_position_commitment_over_private_slot_collision() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let wrong_private_slot = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        58,
        "alice-slot-58".to_string(),
    );
    let correct_position = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        36,
        "alice-slot-36".to_string(),
    );
    wasm.game.set_hidden_card_info(
        correct_position,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 36,
            commitment: "alice-slot-36".to_string(),
            public_slot: Some(58),
            public_commitment: Some("ziffle:test-deck:58".to_string()),
        },
    );

    wasm.reveal_hidden_position(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "position": 58,
            "originalSlot": 36,
            "cardName": "Swamp",
            "positionCommitment": "ziffle:test-deck:58",
            "commitment": "alice-slot-36",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("ziffle reveal should choose the object with the matching position commitment");

    assert_eq!(
        wasm.game
            .object(correct_position)
            .expect("correct position object should exist")
            .name,
        "Swamp",
        "the public ziffle commitment should select the object at that shuffled position"
    );
    assert_eq!(
        wasm.game
            .object(wrong_private_slot)
            .expect("private-slot collision object should exist")
            .name,
        "Hidden Card",
        "a matching private slot number must not win over a mismatched position commitment"
    );
}

#[test]
fn reveal_hidden_position_preserves_existing_public_identity_for_private_opening() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let hand_id = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Hand,
        10,
        "ziffle:initial-deck:10".to_string(),
    );
    wasm.game.set_hidden_card_info(
        hand_id,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Hand,
            slot: 10,
            commitment: "ziffle:initial-deck:10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:shuffle-deck:51".to_string()),
        },
    );

    wasm.reveal_hidden_position(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "objectId": hand_id.0,
            "position": 10,
            "originalSlot": 40,
            "cardName": "Swamp",
            "positionCommitment": "ziffle:initial-deck:10",
            "commitment": "private-slot-40",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("private position reveal should preserve the public ziffle identity");

    let info = wasm
        .game
        .hidden_card_info(hand_id)
        .expect("revealed hidden card should retain hidden metadata");
    assert_eq!(info.slot, 40);
    assert_eq!(info.commitment, "private-slot-40");
    assert_eq!(info.public_slot, Some(51));
    assert_eq!(
        info.public_commitment.as_deref(),
        Some("ziffle:shuffle-deck:51")
    );
    assert_eq!(
        wasm.game
            .object(hand_id)
            .expect("hand object should still exist")
            .name,
        "Swamp"
    );
}

#[test]
fn reveal_hidden_positions_reveals_multiple_ziffle_positions_in_one_batch() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let first = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        10,
        "private-slot-10".to_string(),
    );
    let second = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        20,
        "private-slot-20".to_string(),
    );
    wasm.game.set_hidden_card_info(
        first,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 10,
            commitment: "private-slot-10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:test-deck:51".to_string()),
        },
    );
    wasm.game.set_hidden_card_info(
        second,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 20,
            commitment: "private-slot-20".to_string(),
            public_slot: Some(52),
            public_commitment: Some("ziffle:test-deck:52".to_string()),
        },
    );

    wasm.reveal_hidden_positions(
        serde_wasm_bindgen::to_value(&json!({
            "reveals": [
                {
                    "owner": 0,
                    "position": 51,
                    "originalSlot": 10,
                    "cardName": "Island",
                    "positionCommitment": "ziffle:test-deck:51",
                    "commitment": "private-slot-10",
                },
                {
                    "owner": 0,
                    "position": 52,
                    "originalSlot": 20,
                    "cardName": "Swamp",
                    "positionCommitment": "ziffle:test-deck:52",
                    "commitment": "private-slot-20",
                },
            ],
        }))
        .expect("batch reveal input should serialize"),
    )
    .expect("batch reveal should apply");

    assert_eq!(
        wasm.game.object(first).expect("first card exists").name,
        "Island"
    );
    assert_eq!(
        wasm.game.object(second).expect("second card exists").name,
        "Swamp"
    );
}

#[test]
fn reveal_hidden_positions_rejects_batch_without_partial_reveals() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let first = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        10,
        "private-slot-10".to_string(),
    );
    let second = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        20,
        "private-slot-20".to_string(),
    );
    wasm.game.set_hidden_card_info(
        first,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 10,
            commitment: "private-slot-10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:test-deck:51".to_string()),
        },
    );
    wasm.game.set_hidden_card_info(
        second,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 20,
            commitment: "private-slot-20".to_string(),
            public_slot: Some(52),
            public_commitment: Some("ziffle:test-deck:52".to_string()),
        },
    );

    let result = wasm.reveal_hidden_positions(
        serde_wasm_bindgen::to_value(&json!({
            "reveals": [
                {
                    "owner": 0,
                    "position": 51,
                    "originalSlot": 10,
                    "cardName": "Island",
                    "positionCommitment": "ziffle:test-deck:51",
                    "commitment": "private-slot-10",
                },
                {
                    "owner": 0,
                    "position": 52,
                    "originalSlot": 20,
                    "cardName": "Swamp",
                    "positionCommitment": "ziffle:test-deck:wrong",
                    "commitment": "private-slot-20",
                },
            ],
        }))
        .expect("batch reveal input should serialize"),
    );

    assert!(result.is_err(), "invalid batch reveal should fail");
    assert_eq!(
        wasm.game.object(first).expect("first card exists").name,
        "Hidden Card",
        "the valid first reveal must not be applied before the invalid second reveal is rejected"
    );
    assert_eq!(
        wasm.game.object(second).expect("second card exists").name,
        "Hidden Card",
        "the invalid second reveal should remain hidden"
    );
}

#[test]
fn demonic_consultation_resolution_prompts_for_card_name_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Demonic Consultation".to_string(),
            "hand".to_string(),
            true,
        )
        .expect("Demonic Consultation should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::TextInput(ctx)) => {
            assert_eq!(ctx.description, "Choose a card name");
            assert_eq!(ctx.placeholder.as_deref(), Some("Enter a card name"));
        }
        other => panic!("expected Demonic Consultation card-name prompt, got {other:?}"),
    }
}

#[test]
fn mystical_tutor_resolution_prompts_for_hidden_library_choice_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Mystical Tutor".to_string(), "hand".to_string(), true)
            .expect("Mystical Tutor should be added to hand"),
    );
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let hidden_library_ids: Vec<ObjectId> = (0..3)
        .map(|slot| {
            wasm.game.create_hidden_card_placeholder(
                alice,
                Zone::Library,
                slot,
                format!("alice-hidden-library-{slot}"),
            )
        })
        .collect();

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .collect::<Vec<_>>(),
                hidden_library_ids,
                "Mystical Tutor should pause on the owner with hidden library candidates"
            );
        }
        other => panic!("expected Mystical Tutor hidden-library prompt, got {other:?}"),
    }

    assert!(
        wasm.active_audit_viewed_cards.iter().any(|view| {
            view.viewer == alice
                && view.subject == alice
                && view.zone == Zone::Library
                && !view.public
                && view.cards == hidden_library_ids
        }),
        "WASM dispatch should retain the private library view for audit material"
    );
}

#[test]
fn krrik_casting_black_spell_surfaces_pay_two_life_option_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "K'rrik, Son of Yawgmoth".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("K'rrik should be added to the battlefield");
    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Demonic Consultation".to_string(),
            "hand".to_string(),
            true,
        )
        .expect("Demonic Consultation should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => {
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "Pay 2 life"),
                "expected K'rrik to surface a pay-2-life payment option in the WASM decision"
            );
        }
        other => panic!("expected mana payment choice after starting the cast, got {other:?}"),
    }
}

#[test]
fn public_reveal_survives_replay_advance_to_next_prompt() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.runner = Some(ironsmith::turn_runner::TurnRunner::new());
    wasm.runner_awaiting_priority = true;

    let revealed_card = CardBuilder::new(CardId::from_raw(90_200), "Bob's Revealed Top")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Library);

    let mut replay_dm = WasmReplayDecisionMaker::new(&[]);
    let view_ctx = ViewCardsContext::new(alice, bob, None, Zone::Library, "Reveal consulted cards")
        .with_public(true);
    DecisionMaker::view_cards(&mut replay_dm, &wasm.game, alice, &[revealed_id], &view_ctx);
    let (_, viewed_cards, audit_viewed_cards) = replay_dm.finish();
    wasm.active_viewed_cards = viewed_cards;
    wasm.active_audit_viewed_cards = audit_viewed_cards;

    wasm.advance_until_decision()
        .expect("advance should produce a priority prompt");

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("publicly revealed cards should still be surfaced at the next prompt");
    assert_eq!(viewed_cards.visibility, "public");
    assert_eq!(viewed_cards.zone, "Library");
    assert_eq!(viewed_cards.card_ids, vec![revealed_id.0]);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Revealed Top");
}

#[test]
fn public_reveal_resolves_stale_replay_ids_to_live_card_names() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed_card = CardBuilder::new(CardId::from_raw(90_201), "Bob's Moving Top")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Library);

    let mut replay_dm = WasmReplayDecisionMaker::new(&[]);
    let view_ctx = ViewCardsContext::new(alice, bob, None, Zone::Library, "Reveal consulted cards")
        .with_public(true);
    DecisionMaker::view_cards(&mut replay_dm, &wasm.game, alice, &[revealed_id], &view_ctx);
    let (_, viewed_cards, audit_viewed_cards) = replay_dm.finish();
    wasm.active_viewed_cards = viewed_cards;
    wasm.active_audit_viewed_cards = audit_viewed_cards;

    let moved_id = wasm
        .game
        .move_object(
            revealed_id,
            Zone::Hand,
            ironsmith::events::cause::EventCause::from_game_rule(),
        )
        .expect("card should move to hand");

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("publicly revealed cards should still be surfaced");
    assert_eq!(viewed_cards.card_ids, vec![moved_id.0]);
    assert_eq!(viewed_cards.cards[0].id, moved_id.0);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Moving Top");
}

#[test]
fn viewed_card_snapshots_follow_stable_identity_when_object_id_is_stale() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed_card = CardBuilder::new(CardId::from_raw(90_202), "Bob's Stable Secret")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Hand);
    let stale_unrelated_id = ObjectId::from_raw(revealed_id.0.saturating_add(10_000));
    wasm.active_viewed_cards = Some(ActiveViewedCards {
        viewer: alice,
        subject: bob,
        zone: Zone::Hand,
        cards: vec![stale_unrelated_id],
        card_stable_ids: stable_ids_for_viewed_cards(&wasm.game, &[revealed_id]),
        public: false,
        source: None,
        description: "Inspect hidden card for decision".to_string(),
    });

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );

    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("view should resolve through the card's stable id");
    assert_eq!(viewed_cards.card_ids, vec![revealed_id.0]);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Stable Secret");
    assert_eq!(
        snapshot.players[bob.index()].hand_cards[0].name,
        "Bob's Stable Secret"
    );
}

#[test]
fn cultivator_colossus_snapshot_tracks_repeat_iteration_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    let forest_a = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("first Forest should be added to hand");
    let forest_b = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("second Forest should be added to hand");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("first library filler should be added");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("second library filler should be added");

    wasm.add_card_to_zone(
        0,
        "Cultivator Colossus".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("Cultivator Colossus should enter with ETB processing");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let resolving_stack_object = snapshot
        .resolving_stack_object
        .as_ref()
        .expect("Cultivator ETB prompt should expose the resolving trigger in the snapshot");
    assert_eq!(resolving_stack_object.name, "Cultivator Colossus");
    assert_eq!(
        resolving_stack_object.ability_kind.as_deref(),
        Some("Triggered"),
        "the pinned resolving entry should surface Cultivator's ETB as a triggered ability"
    );
    assert!(
        snapshot.stack_objects.is_empty(),
        "the real stack should stay empty while the UI-only resolving entry is shown separately"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("yes choice should serialize"),
    )
    .expect("accepting the first Cultivator iteration should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let mut hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    hand_ids.sort_unstable();
    assert_eq!(
        hand_ids,
        vec![forest_a, forest_b],
        "first land-choice snapshot should still show both lands in hand"
    );
    let first_choice = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include first land-choice decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects snapshot, got {other:?}"),
    };
    let mut first_candidates: Vec<u64> = first_choice
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id)
        .collect();
    first_candidates.sort_unstable();
    assert_eq!(
        first_candidates,
        vec![forest_a, forest_b],
        "first land-choice snapshot should offer both lands"
    );
    assert!(
        snapshot.resolving_stack_object.is_some(),
        "the resolving Cultivator trigger should stay visible during the land-choice step"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": [forest_a],
        }))
        .expect("first land selection should serialize"),
    )
    .expect("choosing the first land should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        1,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    assert_eq!(
        hand_ids,
        vec![forest_b],
        "after the first land move, the snapshot hand should only show the remaining land"
    );
    let forest_count = me
        .battlefield
        .iter()
        .filter(|permanent| permanent.name == "Forest")
        .map(|permanent| permanent.count)
        .sum::<usize>();
    assert_eq!(
        forest_count, 1,
        "after the first land move, the snapshot battlefield should already show one Forest"
    );
    match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the repeated may decision")
    {
        super::DecisionView::SelectOptions { options, .. } => {
            let option_text: Vec<&str> = options
                .iter()
                .map(|option| option.description.as_str())
                .collect();
            assert_eq!(option_text, vec!["Yes", "No"]);
        }
        other => panic!("expected repeat yes/no snapshot, got {other:?}"),
    }
    assert!(
        snapshot.resolving_stack_object.is_some(),
        "the resolving Cultivator trigger should stay visible across repeat iterations"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("second yes choice should serialize"),
    )
    .expect("accepting the second Cultivator iteration should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        2,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    assert_eq!(
        hand_ids,
        vec![forest_b],
        "before the second land is chosen, the snapshot hand should still show only the remaining land"
    );
    let second_choice = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include second land-choice decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected second select_objects snapshot, got {other:?}"),
    };
    let second_candidates: Vec<u64> = second_choice
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id)
        .collect();
    assert_eq!(
        second_candidates,
        vec![forest_b],
        "second land-choice snapshot should only offer the remaining land"
    );
}

#[test]
fn pregame_mulligan_prompt_offers_keep_and_mulligan() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    let actions = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => &ctx.actions,
        other => panic!("expected pregame priority decision, got {other:?}"),
    };
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, LegalAction::KeepOpeningHand))
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, LegalAction::TakeMulligan))
    );
}

#[test]
fn pregame_priority_labels_progress_from_keep_hand_to_pregame() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "keep_opening_hand"),
        "Keep hand"
    );

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "continue_pregame"),
        "Pregame"
    );

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "begin_game"),
        "Pregame"
    );

    dispatch_matching_priority_action(&mut wasm, |action| matches!(action, LegalAction::BeginGame));

    assert!(
        wasm.pregame.is_none(),
        "game should leave pregame after the Pregame decision"
    );
}

#[test]
fn commander_first_mulligan_is_free() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
    let alice = PlayerId::from_index(0);

    seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, alice, "pregame should move to opening actions");
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::ContinuePregame))
            );
        }
        other => panic!("expected opening-actions priority prompt, got {other:?}"),
    }
}

#[test]
fn commander_second_mulligan_bottoms_one_card() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
    let alice = PlayerId::from_index(0);

    seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, Some(1));
        }
        other => panic!("expected one-card bottoming prompt, got {other:?}"),
    }
}

#[test]
fn serum_powder_redraws_without_counting_as_a_mulligan() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let alice = PlayerId::from_index(0);

    let serum_id = wasm
        .game
        .create_object_from_definition(&serum_powder(), alice, Zone::Hand);
    seed_filler_cards(&mut wasm, alice, Zone::Hand, 6);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(
            action,
            LegalAction::UsePregameAction { card_id, .. } if *card_id == serum_id
        )
    });

    assert_eq!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .hand
            .len(),
        7,
        "Serum Powder should redraw the same hand size"
    );
    assert_eq!(
        wasm.game.exile.len(),
        7,
        "Serum Powder should exile the original opening hand"
    );
    assert_eq!(
        wasm.pregame
            .as_ref()
            .and_then(|pregame| pregame.mulligans_taken.get(&alice).copied())
            .unwrap_or(0),
        0,
        "Serum Powder should not increment the mulligan count"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "the same player should remain on the mulligan prompt"
    );
}

#[test]
fn gemstone_caverns_appears_for_non_starting_player_in_opening_actions() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let bob = PlayerId::from_index(1);

    seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
    let gemstone_id = wasm
        .game
        .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
    seed_filler_cards(&mut wasm, bob, Zone::Hand, 1);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert!(ctx.actions.iter().any(|action| {
                matches!(
                    action,
                    LegalAction::UsePregameAction { card_id, .. }
                        if *card_id == gemstone_id
                )
            }));
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::BeginGame))
            );
        }
        other => panic!("expected Bob's opening-actions prompt, got {other:?}"),
    }
}

#[test]
fn gemstone_caverns_moves_to_battlefield_and_prompts_for_exile() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let bob = PlayerId::from_index(1);

    seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
    let _gemstone_id =
        wasm.game
            .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
    let exile_card = seed_filler_cards(&mut wasm, bob, Zone::Hand, 1)
        .into_iter()
        .next()
        .expect("expected filler card in hand");
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::UsePregameAction { .. })
    });

    let gemstone_on_battlefield = wasm.game.battlefield.iter().copied().find(|id| {
        wasm.game
            .object(*id)
            .is_some_and(|object| object.name == "Gemstone Caverns")
    });
    let gemstone_on_battlefield =
        gemstone_on_battlefield.expect("Gemstone Caverns should move to the battlefield");
    assert_eq!(
        wasm.game
            .object(gemstone_on_battlefield)
            .and_then(|object| object.counters.get(&CounterType::Luck).copied()),
        Some(1),
        "Gemstone Caverns should enter with a luck counter"
    );
    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, Some(1));
        }
        other => panic!("expected Gemstone exile prompt, got {other:?}"),
    }

    dispatch_select_objects(&mut wasm, &[exile_card.0]);

    assert!(
        wasm.game.exile.iter().any(|id| wasm
            .game
            .object(*id)
            .is_some_and(|object| object.name == "Ornithopter")),
        "the chosen card should be exiled"
    );
    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::BeginGame))
            );
        }
        other => panic!("expected Bob to resume opening actions, got {other:?}"),
    }
}

#[test]
fn custom_card_preview_supports_split_faces_and_fuse() {
    let wasm = WasmGame::new();
    let draft = CustomCardInput {
        layout: CustomCardLayoutInput::Split,
        has_fuse: true,
        faces: vec![
            custom_face(
                "Breaking Forge",
                &["Sorcery"],
                "Target player mills four cards.",
                None,
                None,
            ),
            custom_face(
                "Entering Forge",
                &["Sorcery"],
                "Return target creature card from a graveyard to the battlefield under your control.",
                None,
                None,
            ),
        ],
    };

    let preview = wasm
        .build_custom_card_preview(&draft)
        .expect("split custom preview should compile");

    assert_eq!(preview.faces.len(), 2);
    assert!(preview.has_fuse);
    assert_eq!(preview.faces[0].name, "Breaking Forge");
    assert_eq!(preview.faces[1].name, "Entering Forge");
}

#[test]
fn create_custom_card_registers_runtime_linked_face_lookup() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let payload = serde_wasm_bindgen::to_value(&json!({
        "draft": {
            "layout": "transform_like",
            "hasFuse": false,
            "faces": [
                {
                    "name": "Forge Pup",
                    "manaCost": "{1}{R}",
                    "cardTypes": ["Creature"],
                    "subtypes": ["Wolf"],
                    "oracleText": "Haste",
                    "power": "2",
                    "toughness": "1"
                },
                {
                    "name": "Forge Howler",
                    "cardTypes": ["Creature"],
                    "subtypes": ["Wolf"],
                    "oracleText": "Trample",
                    "power": "4",
                    "toughness": "3"
                }
            ]
        },
        "playerIndex": 0,
        "zoneName": "hand",
        "skipTriggers": true
    }))
    .expect("custom card payload should encode");

    let object_id = wasm
        .create_custom_card(payload)
        .expect("linked custom card should be created");
    let object = wasm
        .game
        .object(ObjectId(object_id))
        .expect("created custom card should exist");

    assert_eq!(object.name, "Forge Pup");
    let linked = wasm
        .game
        .linked_face_definition_by_name_or_id(object.other_face_name.as_deref(), object.other_face)
        .expect("linked custom back face should resolve at runtime");
    assert_eq!(linked.name(), "Forge Howler");
}

#[test]
fn external_linked_card_sources_accept_generated_camel_case_fields() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let sources = json!({
        "version": 1,
        "canonicalName": "Ondu Inversion",
        "aliases": [
            {
                "alias": "Ondu Inversion // Ondu Skyruins",
                "canonical": "Ondu Inversion"
            }
        ],
        "group": {
            "kind": "linked",
            "layout": "transform_like",
            "combinedName": "Ondu Inversion // Ondu Skyruins",
            "hasFuse": false,
            "faces": [
                {
                    "name": "Ondu Inversion",
                    "block": "Mana cost: {6}{W}{W}\nType: Sorcery\nDestroy all nonland permanents.",
                    "score": 1.0
                },
                {
                    "name": "Ondu Skyruins",
                    "block": "Type: Land\nThis land enters tapped.\n{T}: Add {W}.",
                    "score": 1.0
                }
            ]
        }
    });

    wasm.register_external_card_sources_json(sources.to_string())
        .expect("generated linked source JSON should register");
    let object_id = wasm
        .add_card_to_hand(0, "Ondu Inversion".to_string())
        .expect("front face should be addable after registration");
    let object = wasm
        .game
        .object(ObjectId(object_id))
        .expect("added object should exist");
    assert_eq!(object.name, "Ondu Inversion");
    assert_eq!(object.other_face_name.as_deref(), Some("Ondu Skyruins"));
}

#[test]
fn snapshot_shows_foretold_card_only_to_the_player_allowed_to_look() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let def = CardDefinitionBuilder::new(CardId::from_raw(50_001), "Foretell Snapshot Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .foretell(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::Foretell { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("foretell should succeed");

    let alice_snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let bob_snapshot = GameSnapshot::from_game(
        &game,
        bob,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let alice_view = alice_snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice snapshot should exist");
    let bob_view_of_alice = bob_snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice zone snapshot should exist for bob");

    assert_eq!(alice_view.exile_cards.len(), 1);
    assert_eq!(alice_view.exile_cards[0].name, "Foretell Snapshot Probe");
    assert_eq!(bob_view_of_alice.exile_cards.len(), 1);
    assert_eq!(bob_view_of_alice.exile_cards[0].name, "Hidden card");
    assert!(
        bob_view_of_alice.exile_cards[0].card_types.is_empty(),
        "unauthorized players should not learn the face-down exiled card's characteristics"
    );
}

#[test]
fn snapshot_uses_exile_look_permissions_instead_of_card_ownership() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let card_id = game.create_object_from_definition(&ornithopter(), bob, Zone::Exile);
    game.set_face_down(card_id);
    game.grant_face_down_exile_view(card_id, alice);

    let alice_snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let bob_snapshot = GameSnapshot::from_game(
        &game,
        bob,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let alice_view_of_bob = alice_snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist for alice");
    let bob_view = bob_snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist");

    assert_eq!(alice_view_of_bob.exile_cards.len(), 1);
    assert_eq!(alice_view_of_bob.exile_cards[0].name, "Ornithopter");
    assert_eq!(bob_view.exile_cards.len(), 1);
    assert_eq!(bob_view.exile_cards[0].name, "Hidden card");
}

