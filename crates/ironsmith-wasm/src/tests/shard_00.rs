#![allow(unused_imports)]
use super::shard_01::*;
use super::shard_02::*;
use super::*;

#[test]
pub(super) fn validate_match_setup_accepts_loadable_normal_decks() {
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
pub(super) fn start_match_loads_sideboards_outside_the_game() {
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
pub(super) fn validate_match_setup_reports_invalid_cards() {
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
pub(super) fn test_action_drag_metadata_links_suspend_special_action_to_card_and_exile() {
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

pub(super) fn custom_face(
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

pub(super) fn start_pregame(
    wasm: &mut WasmGame,
    opening_hand_size: usize,
    format: MatchFormatInput,
) {
    wasm.pregame = Some(PregameState::new(
        &wasm.game.turn_store.turn_order,
        opening_hand_size,
        format,
    ));
    wasm.advance_until_decision()
        .expect("pregame should produce a decision");
}

pub(super) fn dispatch_matching_priority_action<F>(wasm: &mut WasmGame, predicate: F)
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

pub(super) fn snapshot_priority_action_label(wasm: &mut WasmGame, action_ref_kind: &str) -> String {
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

pub(super) fn dispatch_select_objects(wasm: &mut WasmGame, object_ids: &[u64]) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": object_ids,
        }))
        .expect("select_objects should serialize"),
    )
    .expect("select_objects should succeed");
}

pub(super) fn dispatch_select_options(wasm: &mut WasmGame, option_indices: &[usize]) {
    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": option_indices,
        }))
        .expect("select_options should serialize"),
    )
    .expect("select_options should succeed");
}

pub(super) fn dispatch_select_target_object(wasm: &mut WasmGame, object_id: ObjectId) {
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

pub(super) fn dispatch_select_target_player(wasm: &mut WasmGame, player: PlayerId) {
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

pub(super) fn dispatch_pass_priority(wasm: &mut WasmGame) {
    dispatch_matching_priority_action(wasm, |action| matches!(action, LegalAction::PassPriority));
}

#[test]
pub(super) fn battlefield_lane_prefers_artifact_over_land() {
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
pub(super) fn battlefield_lane_prefers_creature_over_artifact() {
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
pub(super) fn battlefield_lane_prefers_enchantment_over_creature_and_sorts_after_creatures() {
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
pub(super) fn convert_and_validate_targets_rejects_wrong_requirement_order() {
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
                legal_target_sets: Vec::new(),
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
            },
            TargetRequirementContext {
                description: "second target".to_string(),
                legal_targets: vec![second],
                legal_target_sets: Vec::new(),
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
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
pub(super) fn convert_and_validate_targets_accepts_unbounded_then_fixed_sequence() {
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
                legal_target_sets: Vec::new(),
                min_targets: 0,
                max_targets: None,
                distinct_player_group: None,
            },
            TargetRequirementContext {
                description: "last target".to_string(),
                legal_targets: vec![c],
                legal_target_sets: Vec::new(),
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
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
pub(super) fn object_details_reports_calculated_battlefield_power_toughness() {
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
pub(super) fn object_details_reports_current_granted_abilities() {
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
pub(super) fn object_details_compacts_changeling_type_line_display() {
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
pub(super) fn object_details_include_compiled_spell_effects_for_spells_with_static_abilities() {
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
pub(super) fn object_details_debug_compiled_text_keeps_spell_effects_when_oracle_fallback_would_apply()
 {
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
pub(super) fn object_details_compiled_text_uses_normalized_surface_for_possessive_self_reference() {
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
pub(super) fn object_details_include_convoke_for_builtin_cards() {
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
pub(super) fn resolving_spell_snapshot_uses_current_source_object_name_after_stack_exit() {
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
pub(super) fn delayed_trigger_snapshot_keeps_source_name_after_source_changes_zones() {
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
pub(super) fn card_load_diagnostics_include_compilation_context_for_builtin_cards() {
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
pub(super) fn card_load_diagnostics_report_parse_error_for_unsupported_generated_cards() {
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
pub(super) fn load_decks_reports_threshold_and_parse_failures_separately() {
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
pub(super) fn add_card_to_zone_allows_below_threshold_cards_for_manual_injection() {
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
pub(super) fn load_decks_accepts_alternative_card_names() {
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
pub(super) fn add_card_to_hand_accepts_alternative_card_names() {
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
pub(super) fn add_card_to_zone_battlefield_applies_etb_replacement_effects() {
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
pub(super) fn add_card_to_zone_battlefield_adds_initial_saga_lore_counter() {
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
pub(super) fn add_card_to_zone_battlefield_surfaces_roaming_throne_type_choice() {
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
pub(super) fn add_card_to_zone_day_of_the_moon_goads_chosen_name_after_text_choice() {
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
pub(super) fn add_card_to_zone_battlefield_commits_roaming_throne_after_choice() {
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
pub(super) fn playing_urzas_saga_from_hand_adds_initial_lore_counter_and_surfaces_snapshot_counters()
 {
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
pub(super) fn cancelability_allows_locked_pending_mana_ability_while_decision_open() {
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
pub(super) fn cancelability_allows_mana_undo_when_not_locked() {
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
pub(super) fn cancelability_allows_epoch_undo_without_pending_chain() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;

    assert!(
        wasm.is_cancelable(),
        "cancel should stay available during a reversible priority epoch"
    );
}

#[test]
pub(super) fn cancelability_blocks_epoch_undo_without_user_action() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = false;

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled when no undoable action happened in this epoch"
    );
}

#[test]
pub(super) fn cancelability_allows_irreversible_mana_replay_while_decision_open() {
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
pub(super) fn cancelability_blocks_irreversible_mana_replay_without_open_decision() {
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
pub(super) fn cancelability_blocks_when_land_played_in_epoch() {
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
pub(super) fn cancelability_blocks_land_play_replay_even_with_open_decision() {
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
pub(super) fn cancelability_blocks_when_epoch_is_mana_locked() {
    let mut wasm = WasmGame::new();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_undo_locked_by_mana = true;

    assert!(
        !wasm.is_cancelable(),
        "cancel should be disabled once epoch is locked by irreversible mana activation"
    );
}

#[test]
pub(super) fn dispatch_disables_cancel_when_mana_tap_trigger_adds_stack_object() {
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
        swamp.abilities_mut().push(Ability::mana(
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
        source.abilities_mut().push(Ability::triggered(
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
pub(super) fn snapshot_surfaces_undo_land_stable_id_for_reversible_land_tap() {
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
        swamp.abilities_mut().push(Ability::mana(
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
pub(super) fn phyrexian_tower_sacrifice_mana_action_uses_selected_creature_without_rollback() {
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
pub(super) fn cleanup_auto_discard_only_applies_for_non_perspective_player() {
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
pub(super) fn cleanup_auto_discard_respects_toggle_and_cleanup_step() {
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
pub(super) fn snapshot_perspective_hand_cards_are_not_truncated() {
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
pub(super) fn snapshot_visible_zone_cards_include_oracle_text() {
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
pub(super) fn snapshot_public_top_library_static_shows_each_players_top_card() {
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
pub(super) fn crypto_requirements_include_static_public_top_library_opening() {
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
pub(super) fn crypto_requirements_include_static_private_own_top_library_opening() {
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
pub(super) fn snapshot_courser_static_shows_only_controllers_top_library_card() {
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
pub(super) fn snapshot_telepathy_static_shows_opponents_hands_to_all_players() {
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
pub(super) fn crypto_requirements_include_static_public_hand_openings() {
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
pub(super) fn snapshot_redacts_hidden_opponent_select_object_candidates() {
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
pub(super) fn redacted_select_object_choice_ids_resolve_to_real_candidates() {
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
pub(super) fn select_object_view_exposes_stable_identity_for_public_candidates() {
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
pub(super) fn select_object_view_keeps_synthetic_candidates_on_object_ids() {
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
pub(super) fn select_object_view_uses_hidden_reference_without_card_identity_for_hidden_candidates()
{
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
pub(super) fn select_object_view_uses_hidden_reference_for_face_down_exile() {
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
pub(super) fn select_object_choice_ids_remap_by_stable_id_and_reject_invalid_stable_id() {
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
pub(super) fn select_object_choice_ids_remap_by_hidden_ref_only_for_legal_candidates() {
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
pub(super) fn snapshot_shows_hidden_zone_select_object_candidates_to_decision_player() {
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
pub(super) fn snapshot_routes_controlled_player_decision_to_controller() {
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
pub(super) fn snapshot_redacts_hidden_opponent_priority_hand_actions() {
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
pub(super) fn snapshot_surfaces_cross_owner_play_from_cards_in_pseudo_hand() {
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
pub(super) fn snapshot_grouped_battlefield_count_matches_total() {
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
pub(super) fn snapshot_grouped_battlefield_includes_mana_cost() {
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
pub(super) fn canceling_spell_chain_after_land_play_keeps_land_on_battlefield() {
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
