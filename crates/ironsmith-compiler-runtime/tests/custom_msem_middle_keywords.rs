//! Source-backed MSEM reminder programs; parent exclusively runs this target.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn routes(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Middle keyword probe", text, false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn fixture_card(id: &str) -> serde_json::Value {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-middle-keywords.json"
    ))
    .unwrap();
    fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == id)
        .unwrap()["representative_cards"][0]
        .clone()
}
#[test]
fn sleight_preserves_sorcery_timing_and_charges_extra_only_for_instant_timing() {
    let card = fixture_card("msem-604");
    for definition in routes(card["executable_text"].as_str().unwrap()) {
        for your_turn in [false, true] {
            for extra in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.turn.active_player = if your_turn { A } else { B };
                game.turn.priority_player = Some(A);
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Green, 2);
                if extra {
                    game.player_mut(A)
                        .unwrap()
                        .mana_pool
                        .add(ManaSymbol::Colorless, 1);
                }
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                let available = compute_legal_actions(&game, A)
                    .unwrap()
                    .iter()
                    .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell));
                assert_eq!(
                    available,
                    your_turn || extra,
                    "your turn {your_turn}, paid extra {extra}"
                );
                assert!(
                    !format!("{:?}", definition.abilities).contains("Keyword(Flash)"),
                    "permission must not grant the flash keyword"
                );
            }
        }
    }
}
struct Choices {
    yes: bool,
    seen: Vec<ObjectId>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.yes
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.seen
            .extend(ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id));
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
#[test]
fn desecrate_optional_sacrifice_selects_only_friendly_artifacts_or_lands() {
    for definition in routes(
        "Type: Creature — Rogue\nPower/Toughness: 3/2\nWhen this creature enters, you may sacrifice an artifact or land.",
    ) {
        for yes in [false, true] {
            for eligible in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let permanent = |text: &str| {
                    ironsmith_compiler_runtime::compile_to_runtime_definition(
                        "Sacrifice probe",
                        text,
                        false,
                    )
                    .unwrap()
                };
                let creature = game.create_object_from_definition(
                    &permanent("Type: Creature\nPower/Toughness: 1/1"),
                    A,
                    Zone::Battlefield,
                );
                let opposing = game.create_object_from_definition(
                    &permanent("Type: Artifact"),
                    B,
                    Zone::Battlefield,
                );
                let friendly = eligible.then(|| {
                    game.create_object_from_definition(
                        &permanent("Type: Land"),
                        A,
                        Zone::Battlefield,
                    )
                });
                let effects = definition
                    .abilities
                    .iter()
                    .find_map(|a| match &a.kind {
                        AbilityKind::Triggered(t) => Some(t.effects.all_effects_owned()),
                        _ => None,
                    })
                    .unwrap();
                let mut dm = Choices { yes, seen: vec![] };
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::SequenceEffect::new(effects)),
                    &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
                )
                .unwrap();
                assert!(game.object(creature).is_some());
                assert!(game.object(opposing).is_some());
                assert!(game.object(source).is_some());
                if let Some(id) = friendly {
                    assert_eq!(game.object(id).is_none(), yes);
                }
                assert!(!dm.seen.contains(&creature));
                assert!(!dm.seen.contains(&opposing));
            }
        }
    }
}
#[test]
fn bloodprice_requires_both_complete_mana_price_and_a_friendly_creature() {
    let card = fixture_card("msem-657");
    for definition in routes(card["executable_text"].as_str().unwrap()) {
        for mana in [2, 3] {
            for creature_owner in [None, Some(A), Some(B)] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.turn.active_player = A;
                game.turn.priority_player = Some(A);
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::White, mana);
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                if let Some(owner) = creature_owner {
                    let body = ironsmith_compiler_runtime::compile_to_runtime_definition(
                        "Bloodprice offering",
                        "Type: Creature\nPower/Toughness: 1/1",
                        false,
                    )
                    .unwrap();
                    game.create_object_from_definition(&body, owner, Zone::Battlefield);
                }
                let available = compute_legal_actions(&game, A)
                    .unwrap()
                    .iter()
                    .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell));
                assert_eq!(
                    available,
                    mana == 3 && creature_owner == Some(A),
                    "mana {mana}, sacrifice owner {creature_owner:?}"
                );
            }
        }
    }
}
#[test]
fn harvest_affordability_counts_only_untapped_friendly_power_and_keeps_colored_mana() {
    let text = "Mana cost: {5}{B}\nType: Sorcery\nAs an additional cost to cast this spell, you may tap any number of untapped creatures you control. This spell costs {X} less to cast, where X is the total power of creatures tapped this way.\nDraw a card.";
    for definition in routes(text) {
        for owner in [A, B] {
            for tapped in [false, true] {
                for colored in [false, true] {
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.turn.active_player = A;
                    game.turn.priority_player = Some(A);
                    if colored {
                        game.player_mut(A)
                            .unwrap()
                            .mana_pool
                            .add(ManaSymbol::Black, 1);
                    }
                    let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                    let body = ironsmith_compiler_runtime::compile_to_runtime_definition(
                        "Harvest body",
                        "Type: Creature\nPower/Toughness: 5/5",
                        false,
                    )
                    .unwrap();
                    let id = game.create_object_from_definition(&body, owner, Zone::Battlefield);
                    if tapped {
                        game.tap(id);
                    }
                    let available = compute_legal_actions(&game, A).unwrap().iter().any(
                        |a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell),
                    );
                    assert_eq!(
                        available,
                        owner == A && !tapped && colored,
                        "owner {owner:?}, tapped {tapped}, colored {colored}"
                    );
                }
            }
        }
    }
}
#[test]
fn harvest_sums_multiple_creatures_but_cannot_reuse_their_tap_for_mana() {
    let text = "Mana cost: {5}{B}\nType: Sorcery\nAs an additional cost to cast this spell, you may tap any number of untapped creatures you control. This spell costs {X} less to cast, where X is the total power of creatures tapped this way.\nDraw a card.";
    for definition in routes(text) {
        for split in [false, true] {
            for floating_black in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.turn.active_player = A;
                game.turn.priority_player = Some(A);
                if floating_black {
                    game.player_mut(A)
                        .unwrap()
                        .mana_pool
                        .add(ManaSymbol::Black, 1);
                }
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                for power in if split { vec![2, 3] } else { vec![5] } {
                    let text =
                        format!("Type: Creature\nPower/Toughness: {power}/5\n{{T}}: Add {{B}}.");
                    let body = ironsmith_compiler_runtime::compile_to_runtime_definition(
                        "Harvest mana body",
                        &text,
                        false,
                    )
                    .unwrap();
                    let id = game.create_object_from_definition(&body, A, Zone::Battlefield);
                    game.remove_summoning_sickness(id);
                }
                let available = compute_legal_actions(&game, A)
                    .unwrap()
                    .iter()
                    .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell));
                assert_eq!(
                    available, floating_black,
                    "split power {split}, floating black {floating_black}"
                );
            }
        }
    }
}
#[test]
fn cadence_rechecks_power_and_filters_the_complete_spell_type_union() {
    use ironsmith::object::CounterType;
    use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
    let text = "Type: Creature — Bard\nPower/Toughness: 1/2\nWhenever you cast an instant, sorcery or Dragon spell, if that spell's mana value is greater than this creature's power, put a +1/+1 counter on this creature.";
    for definition in routes(text) {
        for spell_type in ["Instant", "Sorcery", "Creature — Dragon", "Creature — Fox"] {
            for caster in [A, B] {
                for boost in [false, true] {
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let source =
                        game.create_object_from_definition(&definition, A, Zone::Battlefield);
                    let spell_definition =
                        ironsmith_compiler_runtime::compile_to_runtime_definition(
                            "Cadence spell",
                            &format!("Mana cost: {{3}}\nType: {spell_type}\nPower/Toughness: 1/1"),
                            false,
                        )
                        .unwrap();
                    let spell =
                        game.create_object_from_definition(&spell_definition, caster, Zone::Stack);
                    let event = TriggerEvent::new_with_provenance(
                        ironsmith::events::SpellCastEvent::from_completed_cast(
                            spell,
                            caster,
                            Zone::Hand,
                            &game,
                        ),
                        Default::default(),
                    );
                    let entries = check_triggers(&game, &event);
                    let matches = caster == A && spell_type != "Creature — Fox";
                    assert_eq!(
                        entries.len(),
                        usize::from(matches),
                        "{spell_type}, {caster:?}"
                    );
                    let mut queue = TriggerQueue::new();
                    for entry in entries {
                        queue.add(entry);
                    }
                    let mut dm = SelectFirstDecisionMaker;
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(
                        &mut game, &mut queue, &mut dm,
                    )
                    .unwrap();
                    if boost {
                        game.add_counters(source, CounterType::PlusOnePlusOne, 2);
                    }
                    while !game.stack.is_empty() {
                        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    }
                    assert_eq!(
                        game.counter_count(source, CounterType::PlusOnePlusOne),
                        if boost { 2 } else { u32::from(matches) },
                        "intervening condition is checked at resolution"
                    );
                }
            }
        }
    }
}
