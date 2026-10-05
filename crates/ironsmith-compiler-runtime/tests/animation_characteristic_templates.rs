//! Frozen animation descriptors and independent characteristic retention.
//! Authored only; campaign compilation and execution remain deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{AttachToEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::CounterType;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{
    CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/animation_characteristic_templates.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    if let Some(n) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {n}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn witness(game: &mut GameState, types: Vec<CardType>, colors: ColorSet, token: bool) -> ObjectId {
    let mut card = CardBuilder::new(CardId::new(), "Unlisted witness")
        .card_types(types)
        .subtypes(vec![Subtype::Elf])
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(2, 2));
    if token {
        card = card.token();
    }
    game.create_object_from_card(&card.build(), A, Zone::Battlefield)
}
fn source(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.take_pending_trigger_events();
    source
}
fn program(
    game: &mut GameState,
    source: ObjectId,
    program: ResolutionProgram,
    targets: Vec<ObjectId>,
) {
    game.refresh_continuous_state().unwrap();
    let requirements =
        extract_target_requirements_from_program_with_modes(game, &program, A, Some(source), None);
    assert_eq!(
        requirements.len(),
        targets.len(),
        "one authored target must not multiply across its characteristic changes"
    );
    let assignments = requirements
        .iter()
        .zip(&targets)
        .enumerate()
        .map(|(i, (requirement, id))| {
            assert!(requirement.legal_targets.contains(&Target::Object(*id)));
            TargetAssignment {
                spec: requirement.spec.clone(),
                range: i..i + 1,
            }
        })
        .collect();
    game.push_to_stack(
        StackEntry::ability(source, A, program)
            .with_targets(targets.into_iter().map(Target::Object).collect())
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    game.refresh_continuous_state().unwrap();
}
fn activated(definition: &CardDefinition, index: usize) -> ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a.effects.clone()),
            _ => None,
        })
        .nth(index)
        .unwrap()
}
fn event(game: &mut GameState, event: TriggerEvent) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    game.refresh_continuous_state().unwrap();
}
fn legendary(game: &GameState, id: ObjectId) -> bool {
    game.current_supertypes(id)
        .unwrap()
        .contains(&Supertype::Legendary)
}

#[test]
fn twelve_full_frozen_cards_round_trip_without_loss_and_partial_mask_is_uncounted() {
    let complete: Vec<_> = fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect();
    assert_eq!(complete.len(), 12);
    for row in complete {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn awakening_keeps_land_and_color_names_the_result_and_adds_counters_over_zero_base() {
    for definition in definitions("Awakening of Vitu-Ghazi") {
        let mut game = game();
        let land = witness(&mut game, vec![CardType::Land], ColorSet::BLUE, false);
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        program(
            &mut game,
            spell,
            definition.spell_effect.clone().unwrap(),
            vec![land],
        );
        assert_eq!(game.current_name(land).as_deref(), Some("Vitu-Ghazi"));
        assert!(legendary(&game, land));
        assert!(game.object_has_card_type(land, CardType::Land));
        assert!(game.object_has_card_type(land, CardType::Creature));
        assert_eq!(game.current_colors(land), Some(ColorSet::BLUE));
        assert_eq!(game.current_power(land), Some(9));
        assert_eq!(game.current_toughness(land), Some(9));
        assert!(game.object_has_ability(land, &StaticAbility::haste()));
    }
}
#[test]
fn genju_uses_the_enchanted_land_and_expires_only_the_animation() {
    for definition in definitions("Genju of the Realm") {
        let mut game = game();
        let land = witness(&mut game, vec![CardType::Land], ColorSet::BLUE, false);
        let aura = source(&mut game, &definition);
        AttachToEffect::new(ChooseSpec::SpecificObject(land))
            .execute(&mut game, &mut EffectContext::new_default(aura, A))
            .unwrap();
        program(&mut game, aura, activated(&definition, 0), vec![]);
        assert!(legendary(&game, land));
        assert_eq!(game.current_power(land), Some(8));
        assert_eq!(game.current_toughness(land), Some(12));
        assert!(game.object_has_card_type(land, CardType::Land));
        assert!(game.object_has_ability(land, &StaticAbility::trample()));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert!(!game.object_has_card_type(land, CardType::Creature));
        assert!(!legendary(&game, land));
        assert!(game.object(aura).is_some());
    }
}
#[test]
fn sarkhan_changes_card_type_color_and_all_granted_keywords_without_losing_loyalty() {
    for definition in definitions("Sarkhan, the Dragonspeaker") {
        let mut game = game();
        let s = source(&mut game, &definition);
        let loyalty = game
            .object(s)
            .unwrap()
            .counters
            .get(&CounterType::Loyalty)
            .copied();
        program(&mut game, s, activated(&definition, 0), vec![]);
        assert_eq!(game.current_power(s), Some(4));
        assert_eq!(game.current_colors(s), Some(ColorSet::RED));
        assert!(legendary(&game, s));
        assert!(!game.object_has_card_type(s, CardType::Planeswalker));
        for a in [
            StaticAbility::flying(),
            StaticAbility::haste(),
            StaticAbility::indestructible(),
        ] {
            assert!(game.object_has_ability(s, &a));
        }
        assert_eq!(
            game.object(s)
                .unwrap()
                .counters
                .get(&CounterType::Loyalty)
                .copied(),
            loyalty
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(game.object_has_card_type(s, CardType::Planeswalker));
    }
}
#[test]
fn halsin_and_relic_preserve_other_types_but_only_explicit_color_retention_keeps_old_colors() {
    for name in ["Halsin, Emerald Archdruid", "Relic's Roar"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = witness(
                &mut game,
                vec![CardType::Artifact, CardType::Creature],
                ColorSet::BLUE,
                true,
            );
            let s = source(&mut game, &definition);
            let body = if name == "Relic's Roar" {
                definition.spell_effect.clone().unwrap()
            } else {
                activated(&definition, 0)
            };
            program(&mut game, s, body, vec![target]);
            assert!(game.object_has_card_type(target, CardType::Artifact));
            assert!(
                game.current_subtypes(target)
                    .unwrap()
                    .contains(&Subtype::Elf)
            );
            assert_eq!(
                game.current_colors(target),
                Some(if name == "Relic's Roar" {
                    ColorSet::BLUE
                } else {
                    ColorSet::BLUE.union(ColorSet::GREEN)
                })
            );
            assert_eq!(game.current_power(target), Some(4));
        }
    }
}
#[test]
fn indigo_adds_a_color_while_scrapbasket_sets_all_five_and_both_expire() {
    for definition in definitions("Indigo Faerie") {
        let mut game = game();
        let target = witness(&mut game, vec![CardType::Creature], ColorSet::RED, false);
        let s = source(&mut game, &definition);
        program(&mut game, s, activated(&definition, 0), vec![target]);
        assert_eq!(
            game.current_colors(target),
            Some(ColorSet::RED.union(ColorSet::BLUE))
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_colors(target), Some(ColorSet::RED));
    }
    for definition in definitions("Scrapbasket") {
        let mut game = game();
        let s = source(&mut game, &definition);
        program(&mut game, s, activated(&definition, 0), vec![]);
        assert_eq!(game.current_colors(s).unwrap().count(), 5);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_colors(s), Some(ColorSet::COLORLESS));
    }
}
#[test]
fn unctus_adds_blue_artifact_while_xathrid_sets_colorless_and_keeps_creature_types() {
    for name in ["Unctus, Grand Metatect", "Xathrid Gorgon"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = witness(&mut game, vec![CardType::Creature], ColorSet::RED, false);
            let s = source(&mut game, &definition);
            program(&mut game, s, activated(&definition, 0), vec![target]);
            assert!(game.object_has_card_type(target, CardType::Creature));
            assert!(game.object_has_card_type(target, CardType::Artifact));
            assert!(
                game.current_subtypes(target)
                    .unwrap()
                    .contains(&Subtype::Elf)
            );
            assert_eq!(
                game.current_colors(target),
                Some(if name == "Unctus, Grand Metatect" {
                    ColorSet::RED.union(ColorSet::BLUE)
                } else {
                    ColorSet::COLORLESS
                })
            );
            if name == "Xathrid Gorgon" {
                assert_eq!(
                    game.object(target)
                        .unwrap()
                        .counters
                        .get(&CounterType::Petrification),
                    Some(&1)
                );
                assert!(game.object_has_ability(target, &StaticAbility::defender()));
                assert!(!game.can_activate_abilities_of(target));
            }
        }
    }
}
#[test]
fn shadow_attack_retains_the_triggering_flyers_prior_color_and_creature_type() {
    for definition in definitions("Shadow Puppeteers") {
        let mut game = game();
        let s = source(&mut game, &definition);
        let flyer = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Unlisted flyer",
            "Mana cost: {U}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying",
            false,
        )
        .unwrap();
        let target = game.create_object_from_definition(&flyer, A, Zone::Battlefield);
        let source_power = game.current_power(s);
        event(
            &mut game,
            TriggerEvent::new(
                ironsmith::events::combat::CreatureAttackedEvent::new(
                    target,
                    ironsmith::triggers::event::AttackEventTarget::Player(B),
                ),
                Default::default(),
            ),
        );
        assert_eq!(game.current_power(target), Some(4));
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Bird)
        );
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Dragon)
        );
        assert_eq!(
            game.current_colors(target),
            Some(ColorSet::RED.union(ColorSet::BLUE))
        );
        assert_eq!(game.current_power(s), source_power);
    }
}
#[test]
fn cacophony_preserves_enchantment_and_figure_grants_opponent_protection_in_final_form() {
    for definition in definitions("Cacophony Unleashed") {
        let mut game = game();
        // Keep the actual entry events emitted by the engine. The compiler's
        // native entry matcher observes zone changes, not a synthetic legacy
        // EnterBattlefieldEvent after the producer events were discarded.
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let s = game
            .move_object(
                card,
                Zone::Battlefield,
                ironsmith::events::cause::EventCause::effect(),
            )
            .unwrap();
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        while !game.stack_is_empty() {
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(s), Some(6));
        assert!(legendary(&game, s));
        assert!(game.object_has_card_type(s, CardType::Enchantment));
        assert!(game.object_has_ability(s, &StaticAbility::menace()));
        assert!(game.object_has_ability(s, &StaticAbility::deathtouch()));
    }
    for definition in definitions("Figure of Fable") {
        let mut game = game();
        let s = source(&mut game, &definition);
        for index in 0..3 {
            program(&mut game, s, activated(&definition, index), vec![]);
        }
        assert_eq!(game.current_power(s), Some(7));
        assert_eq!(game.current_toughness(s), Some(8));
        assert!(game.current_subtypes(s).unwrap().contains(&Subtype::Avatar));
        assert!(!game.current_subtypes(s).unwrap().contains(&Subtype::Scout));
        let spell = CardBuilder::new(CardId::new(), "Opponent spell")
            .card_types(vec![CardType::Instant])
            .build();
        let enemy = game.create_object_from_card(&spell, B, Zone::Stack);
        assert!(!ironsmith::targeting::can_target_object(&game, s, enemy, B).is_legal());
    }
}
