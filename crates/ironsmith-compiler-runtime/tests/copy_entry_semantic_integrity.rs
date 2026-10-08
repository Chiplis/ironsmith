//! Whole source bodies and independent runtime/artifact routes. UNVALIDATED/UNRUN.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::{EnterAsCopyAsEntersSpec, StaticAbility, StaticAbilityId};
use ironsmith::{CardId, CardType, Color, ColorSet, CounterType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_definition, materialize_artifact, materialize_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const CHAMELEON: &str = "Chameleon, Master of Disguise";
const MORITTE: &str = "Moritte of the Frost";
const RAID: &str = "Protean Raider";
const SAKASHIMA: &str = "Sakashima of a Thousand Faces";

// Independently transcribed metadata and full Oracle bodies from frozen
// fixtures/card-failure-campaign/cards-20261003.json.xz. No shortened proxy
// replaces a card's secondary abilities, condition, or printed characteristics.
fn source(name: &str) -> &'static str {
    match name {
        CHAMELEON => "Mana cost: {3}{U}\nType: Legendary Creature — Human Shapeshifter Villain\nPower/Toughness: 2/3\nYou may have Chameleon enter as a copy of a creature you control, except his name is Chameleon, Master of Disguise.\nMayhem {2}{U} (You may cast this card from your graveyard for {2}{U} if you discarded it this turn. Timing rules still apply.)",
        MORITTE => "Mana cost: {2}{G}{U}{U}\nType: Legendary Snow Creature — Shapeshifter\nPower/Toughness: 0/0\nChangeling (This card is every creature type.)\nYou may have Moritte enter as a copy of a permanent you control, except it's legendary and snow in addition to its other types and, if it's a creature, it enters with two additional +1/+1 counters on it and has changeling.",
        RAID => "Mana cost: {1}{U}{R}\nType: Creature — Shapeshifter Pirate\nPower/Toughness: 2/2\nRaid — If you attacked this turn, you may have this creature enter as a copy of any creature on the battlefield.",
        SAKASHIMA => "Mana cost: {3}{U}\nType: Legendary Creature — Human Rogue\nPower/Toughness: 3/1\nYou may have Sakashima enter as a copy of another creature you control, except it has Sakashima's other abilities.\nThe \"legend rule\" doesn't apply to permanents you control.\nPartner (You can have two commanders if both have partner.)",
        _ => panic!("unknown source: {name}"),
    }
}

fn expected_rules(name: &str) -> &'static str {
    match name {
        CHAMELEON => "You may have this creature enter as a copy of a creature you control, except his name is Chameleon, Master of Disguise.\nMayhem {2}{U}",
        MORITTE => "Changeling\nYou may have this creature enter as a copy of a permanent you control, except it's legendary and snow in addition to its other types and, if it's a creature, it enters with two additional +1/+1 counters on it and has changeling.",
        RAID => "Raid — If you attacked this turn, you may have this creature enter as a copy of any creature on the battlefield.",
        SAKASHIMA => "You may have this creature enter as a copy of another creature you control, except it has this creature's other abilities.\nThe \"legend rule\" doesn't apply to permanents you control.\nPartner",
        _ => panic!("missing whole-body expectation"),
    }
}

// Presentation-only tolerance: case, commas, terminal periods, and captured
// source-name aliases. Every rules word, number, modal and printed-name
// exception remains; model assertions below independently check literal names.
fn rules_surface(text: &str, name: &str) -> String {
    let mut text = text.to_ascii_lowercase();
    let short = match name { CHAMELEON => "chameleon", MORITTE => "moritte", SAKASHIMA => "sakashima", _ => "" };
    if !short.is_empty() {
        text = text.replace(&format!("have {short} enter"), "have this creature enter")
            .replace(&format!("has {short}'s other abilities"), "has this creature's other abilities");
    }
    text.replace([',', '.'], "").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn definitions_text(name: &str, text: &str) -> [CardDefinition; 3] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    // The companion definition is also artifact-materialized. It must not be
    // mislabeled as the independent direct source route above.
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let native = encode_runtime_definition(direct.clone()).unwrap();
    let native = materialize_definition(serde_json::from_value(serde_json::to_value(native).unwrap()).unwrap()).unwrap();
    [direct, materialize_artifact(&decoded).unwrap(), native]
}
fn definitions(name: &str) -> [CardDefinition; 3] { definitions_text(name, source(name)) }
fn spec(definition: &CardDefinition) -> &EnterAsCopyAsEntersSpec {
    let copies = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => ability.enter_as_copy_as_enters(), _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(copies.len(), 1, "one copy occurrence, including conditional wrappers");
    copies[0]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn donor(game: &mut GameState, controller: PlayerId, creature: bool) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Copy donor")
        .card_types(vec![if creature { CardType::Creature } else { CardType::Artifact }])
        .power_toughness(PowerToughness::fixed(2, 3))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .with_ability(Ability::static_ability(StaticAbility::hexproof()))
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}
#[derive(Default)]
struct ChooseCopy { source: Option<ObjectId>, selected: usize, offered: Vec<ObjectId> }
impl DecisionMaker for ChooseCopy {
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        for option in context.options.iter().filter(|option| option.legal) {
            if let Some(objects) = &option.related_object_ids { self.offered.extend(objects); }
        }
        if let Some(source) = self.source
            && let Some(option) = context.options.iter().find(|option| option.legal
                && option.related_object_ids.as_ref().is_some_and(|objects| objects.contains(&source))) {
            self.selected += 1;
            vec![option.index]
        } else { SelectFirstDecisionMaker.decide_options(game, context) }
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, choice: &mut ChooseCopy) -> ObjectId {
    let entrant = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, choice).unwrap();
    assert!(!receipt.pending && receipt.programs.is_empty(), "entry must finish, not project a pending receipt");
    receipt.original.into_result().unwrap().new_id
}

#[test]
fn four_whole_bodies_preserve_rendered_words_metadata_and_typed_copy_payloads() {
    for (name, mana_value, pt, subtypes) in [
        (CHAMELEON, 4, (2, 3), vec![Subtype::Human, Subtype::Shapeshifter, Subtype::Villain]),
        (MORITTE, 5, (0, 0), vec![Subtype::Shapeshifter]),
        (RAID, 3, (2, 2), vec![Subtype::Shapeshifter, Subtype::Pirate]),
        (SAKASHIMA, 4, (3, 1), vec![Subtype::Human, Subtype::Rogue]),
    ] {
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert_eq!(rules_surface(&rendered, name), rules_surface(expected_rules(name), name), "{name}: {rendered}");
            assert!(rendered.to_ascii_lowercase().contains("enter as a copy"));
            assert_eq!(definition.card.name, name);
            assert_eq!(definition.card.mana_value(), mana_value);
            let (mana_cost, colors) = match name {
                CHAMELEON | SAKASHIMA => ("{3}{U}", ColorSet::BLUE),
                MORITTE => ("{2}{G}{U}{U}", ColorSet::GREEN.with(Color::Blue)),
                RAID => ("{1}{U}{R}", ColorSet::BLUE.with(Color::Red)),
                _ => unreachable!(),
            };
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), mana_cost);
            assert_eq!(definition.card.colors(), colors);
            assert_eq!(definition.card.card_types, [CardType::Creature]);
            assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(pt.0, pt.1)));
            assert_eq!(definition.card.subtypes, subtypes);
            assert_eq!(definition.card.supertypes.contains(&Supertype::Legendary), name != RAID);
            assert_eq!(definition.card.supertypes.contains(&Supertype::Snow), name == MORITTE);
            let copy = spec(&definition);
            assert!(copy.may && copy.affected_filter.is_none());
            assert_eq!(copy.filter.controller, (name != RAID).then_some(ironsmith::target::PlayerFilter::You));
            assert!(copy.copy_duration.is_none() && copy.copy_followups.is_empty());
            assert_eq!(copy.keep_other_source_abilities, name == SAKASHIMA);
            assert_eq!(copy.name_override.as_deref(), (name == CHAMELEON).then_some(CHAMELEON));
            if name == MORITTE {
                assert_eq!(copy.added_supertypes, [Supertype::Legendary, Supertype::Snow]);
                assert_eq!(copy.additional_counters, [(CounterType::PlusOnePlusOne, 2)]);
                assert_eq!(copy.additional_counters_source_filter.as_ref().unwrap().card_types, [CardType::Creature]);
                assert_eq!(copy.added_abilities_source_filter, copy.additional_counters_source_filter);
            }
            if name == CHAMELEON {
                assert_eq!(definition.alternative_casts.len(), 1);
                let method = &definition.alternative_casts[0];
                assert_eq!(method.name(), "Mayhem");
                assert_eq!(method.mana_cost().unwrap().mana_value(), 3);
                assert_eq!(method.mana_cost().unwrap().to_oracle(), "{2}{U}");
                assert_eq!(method.cast_from_zone(), Zone::Graveyard);
                assert!(method.cast_condition().is_some());
                assert!(!method.exiles_after_resolution());
            }
        }
    }
}

#[test]
fn reversible_sakashima_alias_has_two_identical_full_body_faces_with_one_oracle_identity() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/enter_copy_exceptions.json.fixture")).unwrap();
    let alias = rows.iter().find(|row| row["name"] == "Sakashima of a Thousand Faces // Sakashima of a Thousand Faces").unwrap();
    let faces = alias["card_faces"].as_array().unwrap();
    assert_eq!(faces.len(), 2);
    for face in faces {
        assert_eq!(face["oracle_id"], "8ecdaf4b-4442-42da-9714-4257a83faf50");
        let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", face["mana_cost"].as_str().unwrap(), face["type_line"].as_str().unwrap(), face["power"].as_str().unwrap(), face["toughness"].as_str().unwrap(), face["oracle_text"].as_str().unwrap());
        for definition in definitions_text(face["name"].as_str().unwrap(), &text) {
            assert!(spec(&definition).keep_other_source_abilities);
            assert_eq!(rules_surface(&ironsmith_text::compiled_text_lines(&definition).join("\n"), SAKASHIMA), rules_surface(expected_rules(SAKASHIMA), SAKASHIMA));
        }
    }
}

#[test]
fn own_controller_copy_is_optional_nontargeted_and_excludes_opponent_and_off_battlefield_sources() {
    for name in [CHAMELEON, MORITTE, SAKASHIMA] {
        for definition in definitions(name) {
            for accept in [false, true] {
                let mut game = game();
                let own = donor(&mut game, A, true);
                let opponent = donor(&mut game, B, true);
                let dead = donor(&mut game, A, true);
                let dead = game.move_object_by_effect(dead, Zone::Graveyard).unwrap();
                let mut choice = ChooseCopy { source: accept.then_some(own), ..Default::default() };
                let entered = enter(&mut game, &definition, &mut choice);
                assert_eq!(choice.selected, usize::from(accept));
                assert!(!choice.offered.contains(&opponent) && !choice.offered.contains(&dead));
                assert_eq!(game.current_controller(entered), Some(A));
                assert_eq!(game.current_name(entered).as_deref(), Some(if !accept || name == CHAMELEON { name } else { "Copy donor" }));
                assert_eq!(game.current_has_static_ability_id(entered, StaticAbilityId::Vigilance), accept);
                if accept && name == CHAMELEON {
                    assert!(!game.object(entered).unwrap().supertypes.contains(&Supertype::Legendary), "only the name is excepted");
                    assert_eq!(game.current_power(entered), Some(2));
                    assert_eq!(game.current_toughness(entered), Some(3));
                }
            }
        }
    }
}

#[test]
fn moritte_tests_copiable_creature_type_not_layer_four_animation_and_does_not_copy_counters() {
    for definition in definitions(MORITTE) {
        for creature in [false, true] {
            let mut game = game();
            let selected = donor(&mut game, A, creature);
            game.object_mut(selected).unwrap().counters.insert(CounterType::PlusOnePlusOne, 7);
            if !creature {
                game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::new(
                    selected, A, ironsmith::continuous::EffectTarget::Specific(selected),
                    ironsmith::continuous::Modification::AddCardTypes(vec![CardType::Creature])));
                assert!(game.current_is_creature(selected));
            }
            let mut choice = ChooseCopy { source: Some(selected), ..Default::default() };
            let entered = enter(&mut game, &definition, &mut choice);
            assert_eq!(choice.selected, 1);
            assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), if creature { 2 } else { 0 });
            assert_eq!(game.current_has_static_ability_id(entered, StaticAbilityId::Changeling), creature);
            assert_eq!(game.current_is_creature(entered), creature);
            assert!(game.object(entered).unwrap().supertypes.contains(&Supertype::Legendary));
            assert!(game.object(entered).unwrap().supertypes.contains(&Supertype::Snow));
        }
    }
}

#[test]
fn sakashima_copies_partner_and_legend_exception_but_not_its_entry_occurrence_or_layer_six_grants() {
    for definition in definitions(SAKASHIMA) {
        let mut game = game();
        let selected = donor(&mut game, A, true);
        game.object_mut(selected).unwrap().supertypes.push(Supertype::Legendary);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        // Keep a real layer-six grant active before and throughout entry in
        // both relevant zones. Removing it afterward distinguishes copied
        // abilities from an ongoing grant or ordinary zone-change expiry.
        let grants = [Zone::Hand, Zone::Battlefield].map(|zone| {
            game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::new(
                selected, A, ironsmith::continuous::EffectTarget::Filter(
                    ironsmith::ObjectFilter::creature().in_zone(zone)),
                ironsmith::continuous::Modification::AddAbility(StaticAbility::flying())))
        });
        game.refresh_continuous_state().unwrap();
        assert!(game.current_has_static_ability_id(entrant, StaticAbilityId::Flying));
        assert!(game.current_has_static_ability_id(selected, StaticAbilityId::Flying));
        let mut choice = ChooseCopy { source: Some(selected), ..Default::default() };
        let receipt = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut choice).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let entered = receipt.original.into_result().unwrap().new_id;
        assert_eq!(choice.selected, 1);
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Partner));
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Flying));
        for grant in grants { game.effect_store.continuous_effects.remove_effect(grant); }
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_static_ability_id(selected, StaticAbilityId::Flying));
        assert!(!game.current_has_static_ability_id(entered, StaticAbilityId::Flying));
        assert!(!game.object(entered).unwrap().abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.enter_as_copy_as_enters().is_some())));
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        assert!(game.battlefield.contains(&selected) && game.battlefield.contains(&entered));
    }
}

#[test]
fn raid_uses_the_entrants_controller_actual_current_turn_attack_and_declines_without_copy() {
    for definition in definitions(RAID) {
        for attacker_controller in [None, Some(A), Some(B)] {
            for accept in [false, true] {
                let mut game = game();
                let selected = donor(&mut game, B, true);
                if let Some(controller) = attacker_controller {
                    let attacker = donor(&mut game, controller, true);
                    game.remove_summoning_sickness(attacker);
                    game.turn.active_player = controller;
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::DeclareAttackers);
                    game.mark_combat_phase_started();
                    let mut combat = ironsmith::combat_state::CombatState::default();
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
                        &[ironsmith::decision::AttackerDeclaration { creature: attacker,
                            target: ironsmith::combat_state::AttackTarget::Player(if controller == A { B } else { A }) }]).unwrap();
                    game.combat = Some(combat);
                }
                let mut choice = ChooseCopy { source: accept.then_some(selected), ..Default::default() };
                let entered = enter(&mut game, &definition, &mut choice);
                let copied = accept && attacker_controller == Some(A);
                assert_eq!(choice.selected, usize::from(copied));
                assert_eq!(game.current_name(entered).as_deref(), Some(if copied { "Copy donor" } else { RAID }));
                if attacker_controller != Some(A) { assert!(choice.offered.is_empty()); }
            }
        }
    }
}

fn mayhem(game: &GameState, id: ObjectId) -> bool {
    compute_legal_actions(game, A).unwrap().iter().any(|action| matches!(action,
        LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, casting_method: CastingMethod::Alternative(0) } if *spell_id == id))
}
#[test]
fn chameleons_other_printed_ability_requires_real_discard_mana_and_sorcery_timing() {
    for definition in definitions(CHAMELEON) {
        let mut game = game();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let plain = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        assert!(!mayhem(&game, plain));
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        execute_effect(&mut game, &ironsmith::Effect::discard(1), &mut EffectContext::new(hand, A, &mut dm)).unwrap();
        let discarded = *game.player(A).unwrap().graveyard.last().unwrap();
        assert!(mayhem(&game, discarded));
        let mut insufficient = game.clone();
        insufficient.player_mut(A).unwrap().mana_pool.empty();
        assert!(!mayhem(&insufficient, discarded));
        insufficient.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        assert!(!mayhem(&insufficient, discarded), "three colorless cannot pay the blue pip");
        let mut combat = game.clone(); combat.turn.phase = Phase::Combat;
        assert!(!mayhem(&combat, discarded));
        let mut opponent = game.clone(); opponent.turn.active_player = B;
        assert!(!mayhem(&opponent, discarded));
        let mut later = game.clone(); later.next_turn(); later.turn.active_player = A;
        later.turn.priority_player = Some(A); later.turn.phase = Phase::FirstMain;
        later.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        later.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        assert!(!mayhem(&later, discarded));
        let mut casting = game.clone();
        let stable = casting.object(discarded).unwrap().stable_id;
        let action = compute_legal_actions(&casting, A).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == discarded)).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut priority = ironsmith::game_loop::PriorityLoopState::new(casting.players_in_game());
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut casting, &mut queue,
            &mut priority, &ironsmith::game_loop::PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..24 {
            if !casting.stack.is_empty() { break; }
            let ironsmith::decision::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("cast did not reach its paid stack entry: {progress:?}");
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut casting, &mut queue,
                &mut priority, &context, &mut dm).unwrap();
        }
        assert_eq!(casting.stack.len(), 1);
        assert_eq!(casting.player(A).unwrap().mana_pool.total(), 0, "actual 2U payment");
        ironsmith::game_loop::resolve_stack_entry_with(&mut casting, &mut dm).unwrap();
        let permanent = casting.find_object_by_stable_id(stable).unwrap();
        assert_eq!(casting.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(casting.current_name(permanent).as_deref(), Some(CHAMELEON));
        assert!(casting.exile.is_empty(), "Mayhem has no flashback-style exile rider");
        let exile = game.move_object_by_effect(discarded, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        assert!(!mayhem(&game, returned), "discard permission does not follow a later incarnation");
    }
}

#[test]
fn copy_metadata_cannot_silently_default_away_own_ability_retention() {
    fn remove_retention(value: &mut serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(fields) => {
                let removed = usize::from(fields.remove("keep_other_source_abilities").is_some());
                removed + fields.values_mut().map(remove_retention).sum::<usize>()
            }
            serde_json::Value::Array(values) => values.iter_mut().map(remove_retention).sum(),
            _ => 0,
        }
    }
    for definition in definitions(SAKASHIMA) {
        let wire = encode_runtime_definition(definition).unwrap();
        let mut json = serde_json::to_value(wire).unwrap();
        assert_eq!(remove_retention(&mut json), 1);
        assert!(serde_json::from_value::<ironsmith_compiled_artifact::WireCardDefinition>(json).is_err(),
            "required existing copy metadata must survive the codec");
    }
}
