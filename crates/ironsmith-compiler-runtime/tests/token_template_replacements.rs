//! Source-authored only. No build, compilation or execution in this campaign stage.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor, IncubateEffect};
use ironsmith::events::CreateTokensEvent;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ObjectFilter, PlayerFilter};
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId {
    PlayerId::from_index(0)
}
fn b() -> PlayerId {
    PlayerId::from_index(1)
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, materialize_artifact(&decoded).unwrap()]
}
fn fixture(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/token_template_replacements.json.fixture"
    ))
    .unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap_or(""),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    definitions(name, &text)
}
fn soldier() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Soldier")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}
fn create(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    token: CardDefinition,
    count: i32,
) -> Vec<ObjectId> {
    let outcome = CreateTokenEffect::you(token, count)
        .execute(game, &mut EffectContext::new_default(source, player))
        .unwrap();
    let ids = outcome.result_objects().unwrap_or_default().to_vec();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
    ids
}
fn subtype_count(game: &GameState, ids: &[ObjectId], subtype: Subtype) -> usize {
    ids.iter()
        .filter(|id| game.current_has_subtype(**id, subtype))
        .count()
}
#[test]
fn frozen_full_cards_round_trip_including_all_secondary_bodies() {
    for name in [
        "Bilbo, Fellow Conspirator",
        "Divine Visitation",
        "Donatello, the Brains",
        "Draconic Visitor",
        "Jinnie Fay, Jetmir's Second",
        "Jolene, the Plunder Queen",
        "Queen Allenal of Ruadach",
        "Quina, Qu Gourmet",
        "Stridehangar Automaton",
        "Tippy-Toe, Terrific Partner",
        "Worldwalker Helm",
    ] {
        for definition in fixture(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn every_addition_is_once_per_creation_and_uses_the_live_controller() {
    for (name, added) in [
        ("Queen Allenal of Ruadach", Subtype::Soldier),
        ("Quina, Qu Gourmet", Subtype::Frog),
        ("Tippy-Toe, Terrific Partner", Subtype::Food),
        ("Donatello, the Brains", Subtype::Mutagen),
    ] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let ids = create(&mut game, host, a(), soldier(), 3);
            assert_eq!(ids.len(), 4, "{name}");
            assert_eq!(
                subtype_count(&game, &ids, added),
                if added == Subtype::Soldier { 4 } else { 1 }
            );
            assert!(create(&mut game, host, a(), soldier(), 0).is_empty());
            assert_eq!(create(&mut game, host, b(), soldier(), 2).len(), 2);
            game.set_current_controller(host, b()).unwrap();
            assert_eq!(create(&mut game, host, b(), soldier(), 2).len(), 3);
            game.phase_out(host);
            assert_eq!(create(&mut game, host, b(), soldier(), 2).len(), 2);
            game.phase_in(host);
            game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            assert_eq!(create(&mut game, host, b(), soldier(), 2).len(), 2);
        }
    }
}
#[test]
fn bilbo_replaces_each_food_with_two_definitions_in_one_creation_event() {
    use ironsmith::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
    use ironsmith::triggers::tokens::TokensCreatedTrigger;
    for definition in fixture("Bilbo, Fellow Conspirator") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let ids = create(
            &mut game,
            host,
            a(),
            ironsmith::cards::tokens::food_token_definition(),
            3,
        );
        assert_eq!(ids.len(), 6);
        assert_eq!(subtype_count(&game, &ids, Subtype::Food), 3);
        assert_eq!(subtype_count(&game, &ids, Subtype::Treasure), 3);
        let events = game.take_pending_trigger_events();
        let created: Vec<_> = events
            .iter()
            .filter(|event| event.downcast::<CreateTokensEvent>().is_some())
            .collect();
        assert_eq!(created.len(), 1);
        assert_eq!(
            created[0]
                .downcast::<CreateTokensEvent>()
                .unwrap()
                .total_count(),
            6
        );
        let ctx = TriggerContext::for_source(host, a(), &game);
        for (filter, expected) in [
            (ObjectFilter::default(), 6),
            (ObjectFilter::default().with_subtype(Subtype::Treasure), 3),
        ] {
            let trigger = TokensCreatedTrigger::new(PlayerFilter::You, filter, false);
            assert_eq!(
                trigger.trigger_count_with_context(created[0], &ctx),
                expected
            );
        }
    }
}
#[test]
fn substitutions_keep_intrinsic_keywords_and_outer_tap_cleanup_instructions() {
    for (name, token, expected, power) in [
        ("Divine Visitation", soldier(), Subtype::Angel, 4),
        (
            "Draconic Visitor",
            ironsmith::cards::tokens::treasure_token_definition(),
            Subtype::Dragon,
            5,
        ),
    ] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let effect = CreateTokenEffect::you(token.clone(), 3)
                .tapped()
                .exile_at_next_end_step();
            let ids = effect
                .execute(&mut game, &mut EffectContext::new_default(host, a()))
                .unwrap()
                .result_objects()
                .unwrap()
                .to_vec();
            assert_eq!(ids.len(), 3);
            assert_eq!(subtype_count(&game, &ids, expected), 3);
            for id in ids {
                assert!(game.is_tapped(id));
                assert_eq!(game.current_power(id), Some(power));
                assert!(game.current_has_static_ability_id(id, StaticAbilityId::Flying));
            }
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                3,
                "replacement tokens inherit the original cleanup instruction"
            );
        }
    }
}
struct Prefer {
    text: &'static str,
    source: Option<ObjectId>,
    pause: bool,
    pending: bool,
    calls: usize,
}
impl DecisionMaker for Prefer {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(ctx.player, a());
        self.calls += 1;
        self.pending = self.pause;
        let option = ctx
            .options
            .iter()
            .find(|option| {
                option.legal
                    && option.description.contains(self.text)
                    && self
                        .source
                        .is_none_or(|source| option.object_id == Some(source))
            })
            .or_else(|| ctx.options.iter().find(|option| option.legal))
            .unwrap();
        vec![option.index]
    }
}
#[test]
fn jinnie_cat_dog_or_decline_share_one_application_and_pending_choice_is_atomic() {
    for definition in fixture("Jinnie Fay, Jetmir's Second") {
        for (text, subtype, keyword) in [
            ("Cat", Subtype::Cat, Some(StaticAbilityId::Haste)),
            ("Dog", Subtype::Dog, Some(StaticAbilityId::Vigilance)),
            ("Do not apply", Subtype::Treasure, None),
        ] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let mut dm = Prefer {
                text,
                source: None,
                pause: true,
                pending: false,
                calls: 0,
            };
            let effect =
                CreateTokenEffect::you(ironsmith::cards::tokens::treasure_token_definition(), 2);
            let before = game.battlefield.len();
            let out = effect
                .execute(&mut game, &mut EffectContext::new(host, a(), &mut dm))
                .unwrap();
            assert!(out.result_objects().is_none_or(|ids| ids.is_empty()));
            assert_eq!(game.battlefield.len(), before);
            assert!(game.take_pending_trigger_events().is_empty());
            dm.pause = false;
            dm.pending = false;
            let ids = effect
                .execute(&mut game, &mut EffectContext::new(host, a(), &mut dm))
                .unwrap()
                .result_objects()
                .unwrap()
                .to_vec();
            assert_eq!(
                dm.calls, 2,
                "one pending query then exactly one accepted/declined choice"
            );
            assert_eq!(ids.len(), 2);
            assert_eq!(subtype_count(&game, &ids, subtype), 2);
            if let Some(keyword) = keyword {
                assert!(
                    ids.iter()
                        .all(|id| game.current_has_static_ability_id(*id, keyword))
                );
            }
        }
    }
}
#[test]
fn addition_and_doubling_see_modified_groups_in_affected_player_order() {
    let doubles = definitions(
        "Token multiplier probe",
        "Type: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",
    );
    for (adder, doubler) in fixture("Quina, Qu Gourmet").into_iter().zip(doubles) {
        for add_first in [false, true] {
            let mut game = game();
            let add = game.create_object_from_definition(&adder, a(), Zone::Battlefield);
            let double = game.create_object_from_definition(&doubler, a(), Zone::Battlefield);
            let mut dm = Prefer {
                text: "",
                source: Some(if add_first { add } else { double }),
                pause: false,
                pending: false,
                calls: 0,
            };
            let ids = CreateTokenEffect::you(soldier(), 2)
                .execute(&mut game, &mut EffectContext::new(add, a(), &mut dm))
                .unwrap()
                .result_objects()
                .unwrap()
                .to_vec();
            assert_eq!(ids.len(), if add_first { 6 } else { 5 });
            assert_eq!(
                subtype_count(&game, &ids, Subtype::Frog),
                if add_first { 2 } else { 1 }
            );
        }
    }
}
#[test]
fn artifact_templates_keep_map_actions_thopter_anthems_and_incubate_counters() {
    for (name, subtype) in [
        ("Worldwalker Helm", Subtype::Map),
        ("Stridehangar Automaton", Subtype::Thopter),
    ] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(create(&mut game, host, a(), soldier(), 2).len(), 2);
            let ids = create(
                &mut game,
                host,
                a(),
                ironsmith::cards::tokens::treasure_token_definition(),
                2,
            );
            assert_eq!(ids.len(), 3);
            assert_eq!(subtype_count(&game, &ids, subtype), 1);
            let added = *ids
                .iter()
                .find(|id| game.current_has_subtype(**id, subtype))
                .unwrap();
            if subtype == Subtype::Map {
                assert!(!game.object(added).unwrap().abilities.is_empty());
            } else {
                assert_eq!(game.current_power(added), Some(2));
                assert!(game.current_has_static_ability_id(added, StaticAbilityId::Flying));
            }
        }
    }
    for definition in fixture("Draconic Visitor") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let ids = IncubateEffect::you(3, 1)
            .execute(&mut game, &mut EffectContext::new_default(host, a()))
            .unwrap()
            .result_objects()
            .unwrap()
            .to_vec();
        assert_eq!(subtype_count(&game, &ids, Subtype::Dragon), 1);
        assert_eq!(
            game.object(ids[0])
                .unwrap()
                .counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne),
            Some(&3)
        );
        assert_eq!(game.current_power(ids[0]), Some(8));
    }
}

#[test]
fn manufactor_substitutes_custom_definitions_and_retains_added_template_groups() {
    let cards = definitions(
        "Academy Manufactor",
        "Mana cost: {3}\nType: Artifact Creature — Assembly-Worker\nPower/Toughness: 1/3\nIf you would create a Clue, Food, or Treasure token, instead create one of each.",
    );
    for definition in cards {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let custom = CardDefinitionBuilder::new(CardId::new(), "Custom Food creature")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Food])
            .power_toughness(PowerToughness::fixed(9, 9))
            .build();
        let ids = create(&mut game, host, a(), custom, 2);
        assert_eq!(ids.len(), 6);
        for subtype in [Subtype::Clue, Subtype::Food, Subtype::Treasure] {
            assert_eq!(subtype_count(&game, &ids, subtype), 2);
        }
        assert!(
            ids.iter().all(|id| !game.current_is_creature(*id)),
            "one-of-each replaces the definition rather than preserving the custom prototype"
        );
    }
}

#[test]
fn subtype_replacement_chain_sees_previous_template_and_each_identity_once() {
    let definitions = definitions(
        "Token subtype chain probe",
        "Type: Enchantment\nIf you would create a Fish token, create a 3/3 blue Shark creature token instead.\nIf you would create a Shark token, create an 8/8 blue Octopus creature token instead.",
    );
    for definition in definitions {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let fish = CardDefinitionBuilder::new(CardId::new(), "Fish")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Fish])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let ids = create(&mut game, host, a(), fish, 2);
        assert_eq!(ids.len(), 2);
        assert_eq!(subtype_count(&game, &ids, Subtype::Octopus), 2);
        assert!(ids.iter().all(|id| game.current_power(*id) == Some(8)));
    }
}

fn activate_source(game: &mut GameState, source: ObjectId) {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm, resolve_stack_entry,
    };
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = a();
    game.turn.priority_player = Some(a());
    let action = compute_legal_actions(game, a()).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
        if state.pending_activation.is_none() {
            break;
        }
        if let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress {
            progress =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
                    .unwrap();
        } else {
            break;
        }
    }
    assert!(!game.stack_is_empty());
    resolve_stack_entry(game).unwrap();
}
#[test]
fn jolene_and_quina_secondary_sacrifice_costs_pay_real_created_tokens() {
    for (name, input, count, counters) in [
        (
            "Jolene, the Plunder Queen",
            ironsmith::cards::tokens::treasure_token_definition(),
            4,
            5,
        ),
        ("Quina, Qu Gourmet", soldier(), 1, 1),
    ] {
        for definition in fixture(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let ids = create(&mut game, host, a(), input.clone(), count);
            game.player_mut(a())
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 2);
            activate_source(&mut game, host);
            assert_eq!(
                game.object(host)
                    .unwrap()
                    .counters
                    .get(&ironsmith::object::CounterType::PlusOnePlusOne),
                Some(&counters)
            );
            let expected_remaining = if name.starts_with("Jolene") { 0 } else { 1 };
            assert_eq!(
                ids.iter()
                    .filter(|id| game
                        .object(**id)
                        .is_some_and(|object| object.zone == Zone::Battlefield))
                    .count(),
                expected_remaining
            );
        }
    }
}
#[test]
fn solved_and_class_level_guards_are_live_for_template_replacements() {
    let solved = definitions(
        "Case token gate probe",
        "Type: Enchantment — Case\nTo solve — You control three or more Detectives.\nSolved — If one or more tokens would be created under your control, those tokens plus a Clue token are created instead.",
    );
    for definition in solved {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert_eq!(create(&mut game, host, a(), soldier(), 2).len(), 2);
        game.solve_case(host);
        let ids = create(&mut game, host, a(), soldier(), 2);
        assert_eq!(ids.len(), 3);
        assert_eq!(subtype_count(&game, &ids, Subtype::Clue), 1);
    }
    let leveled = definitions(
        "Class token gate probe",
        "Type: Enchantment — Class\n{G}{U}: Level 2\nIf you would create a Fish token, create a 3/3 blue Shark creature token instead.\n{2}{G}{U}: Level 3\nIf you would create a Shark token, create an 8/8 blue Octopus creature token instead.",
    );
    for definition in leveled {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        for (level, subtype) in [
            (1, Subtype::Fish),
            (2, Subtype::Shark),
            (3, Subtype::Octopus),
        ] {
            game.set_class_level(host, level);
            let fish = CardDefinitionBuilder::new(CardId::new(), "Fish")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Fish])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            let ids = create(&mut game, host, a(), fish, 2);
            assert_eq!(subtype_count(&game, &ids, subtype), 2);
        }
    }
}

#[test]
fn copy_owner_preserves_outer_instructions_without_copying_inline_exceptions_to_added_tokens() {
    use ironsmith::effects::{CreateTokenCopyEffect, TokenCopyReferenceSurface};
    use ironsmith::target::ChooseSpec;
    for definition in fixture("Quina, Qu Gourmet") {
        for followup_haste in [false, true] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let model = game.create_object_from_definition(&soldier(), a(), Zone::Battlefield);
            let mut effect = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(model));
            effect.has_haste = true;
            effect.enters_tapped = true;
            effect.exile_at_next_end_step = true;
            effect.haste_followup_reference_surface =
                followup_haste.then_some(TokenCopyReferenceSurface::ThoseTokens);
            let ids = effect
                .execute(&mut game, &mut EffectContext::new_default(host, a()))
                .unwrap()
                .result_objects()
                .unwrap()
                .to_vec();
            assert_eq!(ids.len(), 2);
            assert!(ids.iter().all(|id| game.is_tapped(*id)));
            assert_eq!(game.effect_store.delayed_triggers.len(), 2);
            let frog = *ids
                .iter()
                .find(|id| game.current_has_subtype(**id, Subtype::Frog))
                .unwrap();
            assert_eq!(
                game.current_has_static_ability_id(frog, StaticAbilityId::Haste),
                followup_haste,
                "new template inherits a separate grant instruction but not the original copy's intrinsic exception"
            );
        }
    }
}

#[test]
fn affordable_501_originals_plus_added_template_are_exact_and_keep_cleanup() {
    for definition in fixture("Quina, Qu Gourmet") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        game.take_pending_trigger_events();
        let out = CreateTokenEffect::you(soldier(), 501)
            .tapped()
            .exile_at_next_end_step()
            .execute(&mut game, &mut EffectContext::new_default(host, a()))
            .unwrap();
        let ids = out.result_objects().unwrap();
        assert_eq!(ids.len(), 502);
        assert_eq!(subtype_count(&game, ids, Subtype::Soldier), 501);
        assert_eq!(subtype_count(&game, ids, Subtype::Frog), 1);
        assert!(ids.iter().all(|id| game.is_tapped(*id)));
        assert_eq!(game.effect_store.delayed_triggers.len(), 502);
        let creations: Vec<_> = out
            .events
            .iter()
            .filter_map(|event| event.downcast::<CreateTokensEvent>())
            .collect();
        assert_eq!(creations.len(), 1);
        assert_eq!(creations[0].total_count(), 502);
    }
}

#[test]
fn affordable_501_substituted_templates_are_not_truncated() {
    for definition in fixture("Divine Visitation") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let ids = create(&mut game, host, a(), soldier(), 501);
        assert_eq!(ids.len(), 501);
        assert_eq!(subtype_count(&game, &ids, Subtype::Angel), 501);
        assert!(
            ids.iter()
                .all(|id| game.current_has_static_ability_id(*id, StaticAbilityId::Flying))
        );
    }
}

#[test]
fn affordable_501_copies_keep_original_and_additional_instruction_scopes() {
    use ironsmith::effects::{CreateTokenCopyEffect, TokenCopyReferenceSurface};
    use ironsmith::target::ChooseSpec;
    for definition in fixture("Quina, Qu Gourmet") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let model = game.create_object_from_definition(&soldier(), a(), Zone::Battlefield);
        let mut effect = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(model));
        effect.count = ironsmith::effect::Value::Fixed(501);
        effect.has_haste = true;
        effect.haste_followup_reference_surface = Some(TokenCopyReferenceSurface::ThoseTokens);
        effect.enters_tapped = true;
        effect.exile_at_next_end_step = true;
        let out = effect
            .execute(&mut game, &mut EffectContext::new_default(host, a()))
            .unwrap();
        let ids = out.result_objects().unwrap();
        assert_eq!(ids.len(), 502);
        assert_eq!(subtype_count(&game, ids, Subtype::Soldier), 501);
        assert_eq!(subtype_count(&game, ids, Subtype::Frog), 1);
        assert!(ids.iter().all(|id| game.is_tapped(*id)
            && game.current_has_static_ability_id(*id, StaticAbilityId::Haste)));
        assert_eq!(game.effect_store.delayed_triggers.len(), 502);
    }
}

#[test]
fn affordable_501_incubate_iterations_keep_counters_and_separate_creation_events() {
    let mut game = game();
    let source = game.new_object_id();
    let out = IncubateEffect::you(3, 501)
        .execute(&mut game, &mut EffectContext::new_default(source, a()))
        .unwrap();
    let ids = out.result_objects().unwrap();
    assert_eq!(ids.len(), 501);
    assert!(
        ids.iter()
            .all(|id| game.counter_count(*id, ironsmith::object::CounterType::PlusOnePlusOne) == 3)
    );
    let creations: Vec<_> = out
        .events
        .iter()
        .filter_map(|event| event.downcast::<CreateTokensEvent>())
        .collect();
    assert_eq!(creations.len(), 501);
    assert!(creations.iter().all(|event| event.total_count() == 1));
}
