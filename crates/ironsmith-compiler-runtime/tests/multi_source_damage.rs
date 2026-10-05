//! UNVALIDATED simultaneous multi-source damage, source sets and target legality.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/multi_source_damage.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game_with_starting_life(starting: i32) -> GameState {
    let mut game = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Charlie".into()],
        starting,
    );
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    optional: Option<bool>,
    kicker_payments: usize,
    decline_targets: bool,
    bounds: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if let Some(choice) = self.optional {
            return choice;
        }
        let _ = ctx;
        true
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            return context
                .options
                .iter()
                .filter(|option| option.legal)
                .take(self.kicker_payments)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.bounds.extend(
            context
                .requirements
                .iter()
                .map(|r| (r.min_targets, r.max_targets)),
        );
        if self.decline_targets {
            return Vec::new();
        }
        if !self.targets.is_empty() {
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            }));
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if !self.objects.is_empty() {
            let selected = self
                .objects
                .iter()
                .copied()
                .filter(|id| {
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                })
                .take(context.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                selected.len() >= context.min,
                "requested cost objects must satisfy the current payment decision"
            );
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .rev()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

fn game() -> GameState {
    game_with_starting_life(20)
}
fn creature(game: &mut GameState, player: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla(name, "{1}", "Human", p, t),
        player,
        Zone::Battlefield,
    )
}
#[test]
fn eight_exact_cards_keep_metadata_and_one_typed_multi_source_owner_through_artifacts() {
    fn visit(effect: &Effect, found: &mut Vec<ironsmith_core::DealDamageBySourcesEffect>) {
        if let Some(damage) = effect.downcast_ref::<ironsmith::effects::DealDamageBySourcesEffect>()
        {
            found.push(damage.clone());
        }
        effect.visit_child_effects(&mut |child| visit(child, found));
    }
    for row in fixtures()
        .into_iter()
        .filter(|row| row["name"] != "Alpha Brawl")
    {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            assert!(
                !definition.canonical_text.contains("Choose target"),
                "synthetic source declarations stay part of the authored damage instruction: {}",
                definition.canonical_text
            );
            assert!(
                !definition.canonical_text.contains("tagged"),
                "{}",
                definition.canonical_text
            );
            let mut found = Vec::new();
            for effect in definition
                .spell_effect
                .as_ref()
                .unwrap()
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
            {
                visit(effect, &mut found);
            }
            assert_eq!(found.len(), 1, "{name}");
            assert_eq!(found[0].amount, ironsmith_core::Value::SourcePower);
            assert_eq!(
                found[0].source_binding,
                ironsmith_core::DamageSourceSetBinding::LiveMembers
            );
            assert_eq!(
                found[0].recipient_binding,
                ironsmith_core::DamageRecipientSetBinding::SharedSet
            );
            assert!(!found[0].unpreventable);
            assert_eq!(
                found[0].sources.len(),
                if matches!(name, "Friendly Rivalry" | "Graceful Takedown") {
                    2
                } else {
                    1
                }
            );
        }
    }
}
#[test]
fn band_together_reads_every_power_before_lifelink_changes_life_and_records_both_sources() {
    for definition in definitions("Band Together") {
        let mut game = game();
        let first = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Lifelink source",
                "Type: Creature — Human\nPower/Toughness: 3/4\nLifelink",
                false,
            )
            .unwrap(),
            A,
            Zone::Battlefield,
        );
        let second=game.create_object_from_definition(&compile_to_runtime_definition("Life-sized source","Type: Creature — Human\nPower/Toughness: */*\nThis creature's power and toughness are each equal to your life total.",false).unwrap(),A,Zone::Battlefield);
        let recipient = creature(&mut game, B, "Recipient", 1, 100);
        let mut dm = Choices {
            targets: vec![
                Target::Object(first),
                Target::Object(second),
                Target::Object(recipient),
            ],
            ..Default::default()
        };
        let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            game.damage_on(recipient),
            23,
            "later source power cannot grow from earlier lifelink"
        );
        assert_eq!(game.player(A).unwrap().life, 23);
        let receipts = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .is_some_and(|event| event.source == first || event.source == second)
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 2);
        assert_eq!(
            receipts[0].event.simultaneous_batch(),
            receipts[1].event.simultaneous_batch()
        );
        assert!(receipts[0].event.simultaneous_batch().is_some());
        let _ = spell;
    }
}
#[test]
fn required_two_source_targets_keep_the_survivor_after_the_other_leaves_and_returns() {
    for definition in definitions("Combo Attack") {
        let mut game = game();
        game.restore_team_vs_team(vec![vec![A, C], vec![B]], vec![A, C, B], 0, A)
            .unwrap();
        let first = creature(&mut game, A, "First", 2, 4);
        let second = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Teammate's surviving source",
                "Type: Creature — Human\nPower/Toughness: 3/4\nLifelink",
                false,
            )
            .unwrap(),
            C,
            Zone::Battlefield,
        );
        let recipient = creature(&mut game, B, "Recipient", 1, 100);
        let mut dm = Choices {
            targets: vec![
                Target::Object(first),
                Target::Object(second),
                Target::Object(recipient),
            ],
            ..Default::default()
        };
        let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        assert!(
            dm.bounds.contains(&(2, Some(2))),
            "two is an announcement minimum"
        );
        apply(
            &mut game,
            spell,
            Effect::exile(ChooseSpec::SpecificObject(first)),
        );
        let exile = *game.exile.last().unwrap();
        apply(
            &mut game,
            spell,
            Effect::new(
                ironsmith::effects::MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(exile),
                    Zone::Battlefield,
                    false,
                )
                .under_owner_control(),
            ),
        );
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            game.damage_on(recipient),
            3,
            "one survivor still deals damage; the new incarnation does not"
        );
        assert_eq!(
            game.player(C).unwrap().life,
            23,
            "lifelink belongs to the actual source's controller"
        );
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
#[test]
fn zero_optional_sources_are_legal_and_recipient_illegality_prevents_every_source() {
    for definition in definitions("Band Together") {
        for no_sources in [true, false] {
            let mut game = game();
            let first = creature(&mut game, A, "First", 2, 4);
            let second = creature(&mut game, A, "Second", 3, 4);
            let recipient = creature(&mut game, B, "Recipient", 1, 100);
            let mut dm = Choices {
                targets: if no_sources {
                    vec![Target::Object(recipient)]
                } else {
                    vec![
                        Target::Object(first),
                        Target::Object(second),
                        Target::Object(recipient),
                    ]
                },
                ..Default::default()
            };
            let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if !no_sources {
                apply(
                    &mut game,
                    spell,
                    Effect::exile(ChooseSpec::SpecificObject(recipient)),
                );
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(recipient), 0);
        }
    }
}
#[test]
fn one_shield_counter_prevents_the_whole_multi_source_damage_event() {
    for definition in definitions("Band Together") {
        let mut game = game();
        let first = creature(&mut game, A, "First", 2, 4);
        let second = creature(&mut game, A, "Second", 3, 4);
        let recipient = creature(&mut game, B, "Shielded", 1, 100);
        apply(
            &mut game,
            first,
            Effect::new(ironsmith::effects::PutCountersEffect::new(
                ironsmith::CounterType::Shield,
                1,
                ChooseSpec::SpecificObject(recipient),
            )),
        );
        let mut dm = Choices {
            targets: vec![
                Target::Object(first),
                Target::Object(second),
                Target::Object(recipient),
            ],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.damage_on(recipient), 0);
        assert_eq!(
            game.object(recipient)
                .unwrap()
                .counters
                .get(&ironsmith::CounterType::Shield)
                .copied()
                .unwrap_or(0),
            0
        );
    }
}
#[test]
fn tapped_and_pumped_plural_antecedents_keep_all_sources_and_current_post_instruction_power() {
    for name in [
        "Coordinated Clobbering",
        "Tandem Takedown",
        "Terrific Team-Up",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let first = creature(&mut game, A, "First", 2, 4);
            let second = creature(&mut game, A, "Second", 3, 4);
            let recipient = creature(&mut game, B, "Recipient", 1, 100);
            let mut dm = Choices {
                targets: vec![
                    Target::Object(first),
                    Target::Object(second),
                    Target::Object(recipient),
                ],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.damage_on(recipient),
                if name == "Coordinated Clobbering" {
                    5
                } else {
                    7
                },
                "{name}"
            );
            if name == "Coordinated Clobbering" {
                assert!(game.is_tapped(first) && game.is_tapped(second));
            }
        }
    }
}
#[test]
fn independent_source_groups_keep_legendary_and_enchanted_membership_and_other_exclusions() {
    for name in ["Friendly Rivalry", "Graceful Takedown"] {
        for definition in definitions(name) {
            let mut game = game();
            let first = creature(&mut game, A, "First", 2, 4);
            let second = game.create_object_from_definition(
                &compile_to_runtime_definition(
                    "Legendary source",
                    "Type: Legendary Creature — Human\nPower/Toughness: 3/4",
                    false,
                )
                .unwrap(),
                A,
                Zone::Battlefield,
            );
            if name == "Graceful Takedown" {
                let aura=game.create_object_from_definition(&compile_to_runtime_definition("Attached aura","Type: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +0/+1.",false).unwrap(),A,Zone::Battlefield);
                apply(
                    &mut game,
                    aura,
                    Effect::attach_to(ChooseSpec::SpecificObject(first)),
                );
            }
            let recipient = creature(&mut game, B, "Recipient", 1, 100);
            let mut dm = Choices {
                targets: vec![
                    Target::Object(first),
                    Target::Object(second),
                    Target::Object(recipient),
                ],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(game.damage_on(recipient), 5, "{name}");
            assert!(
                dm.bounds
                    .iter()
                    .any(|(min, max)| *min == 0 && *max == Some(1))
            );
            if name == "Graceful Takedown" {
                assert!(
                    dm.bounds
                        .iter()
                        .any(|(min, max)| *min == 0 && max.is_none())
                );
            }
        }
    }
}
#[test]
fn printed_cost_reductions_and_bounded_source_target_bodies_compose() {
    for name in ["Allies at Last", "Terrific Team-Up"] {
        for definition in definitions(name) {
            let mut game = game();
            let first = game.create_object_from_definition(
                &vanilla("First ally", "{1}", "Ally", 2, 4),
                A,
                Zone::Battlefield,
            );
            let second = creature(&mut game, A, "Second source", 3, 4);
            let recipient = creature(&mut game, B, "Recipient", 1, 100);
            game.player_mut(A).unwrap().mana_pool = Default::default();
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Green, 1);
            if name == "Terrific Team-Up" {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 1);
            }
            let card = game.create_object_from_definition(&definition, A, Zone::Hand);
            let cast_action = LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            };
            assert!(
                !compute_legal_actions(&game, A)
                    .unwrap()
                    .contains(&cast_action)
            );
            let qualifier = if name == "Allies at Last" {
                vanilla("Second ally", "{1}", "Ally", 1, 4)
            } else {
                vanilla("Four-value permanent", "{4}", "Human", 1, 4)
            };
            game.create_object_from_definition(&qualifier, A, Zone::Battlefield);
            assert!(
                compute_legal_actions(&game, A)
                    .unwrap()
                    .contains(&cast_action),
                "{name}: printed reduction unlocks the cast"
            );
            let mut dm = Choices {
                targets: vec![
                    Target::Object(first),
                    Target::Object(second),
                    Target::Object(recipient),
                ],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.damage_on(recipient),
                if name == "Allies at Last" { 5 } else { 7 }
            );
        }
    }
}
#[test]
fn explicit_source_to_all_recipients_uses_one_damage_occurrence_and_one_lifelink_gain() {
    for definition in definitions_text(
        "Quantified source damage",
        "Mana cost: {G}\nType: Sorcery\nTarget creature an opponent controls deals damage equal to its power to each other creature that player controls.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Lifelink source",
                "Type: Creature — Human\nPower/Toughness: 5/20\nLifelink",
                false,
            )
            .unwrap(),
            B,
            Zone::Battlefield,
        );
        let first = creature(&mut game, B, "First recipient", 1, 20);
        let second = creature(&mut game, B, "Second recipient", 1, 20);
        let outsider = creature(&mut game, C, "Other controller", 1, 20);
        let mut dm = Choices {
            targets: vec![Target::Object(source)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            (
                game.damage_on(source),
                game.damage_on(first),
                game.damage_on(second),
                game.damage_on(outsider)
            ),
            (0, 5, 5, 0)
        );
        assert_eq!(game.player(B).unwrap().life, 30);
        let gains = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .filter_map(|record| record.event.downcast::<ironsmith::events::LifeGainEvent>())
            .filter(|event| event.player == B)
            .collect::<Vec<_>>();
        assert_eq!(
            gains.len(),
            1,
            "lifelink is once for the complete occurrence"
        );
        assert_eq!(gains[0].amount, 10);
        fn contains_serial_damage(effect: &Effect) -> bool {
            if effect
                .downcast_ref::<ironsmith::effects::ForEachObject>()
                .is_some()
            {
                return true;
            }
            let mut found = false;
            effect.visit_child_effects(&mut |child| found |= contains_serial_damage(child));
            found
        }
        assert!(
            !definition
                .spell_effect
                .as_ref()
                .unwrap()
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .any(contains_serial_damage)
        );
    }
}

#[test]
fn alpha_brawl_captures_two_different_sets_and_keeps_both_damage_phases_complete() {
    for definition in definitions("Alpha Brawl") {
        assert!(
            definition
                .canonical_text
                .contains("then each of those creatures"),
            "{}",
            definition.canonical_text
        );
        assert!(
            !definition.canonical_text.contains("tagged"),
            "{}",
            definition.canonical_text
        );
        for mode in 0..6 {
            let mut game = game();
            let primary = game.create_object_from_definition(
                &compile_to_runtime_definition(
                    "Primary creature",
                    "Type: Creature — Human\nPower/Toughness: 4/50\nLifelink",
                    false,
                )
                .unwrap(),
                B,
                Zone::Battlefield,
            );
            let first = creature(&mut game, B, "First other", 2, 50);
            let second = creature(&mut game, B, "Second other", 5, 50);
            let outsider = creature(&mut game, C, "Unrelated controller", 17, 50);
            if mode == 1 {
                apply(
                    &mut game,
                    primary,
                    Effect::exile(ChooseSpec::SpecificObject(first)),
                );
                apply(
                    &mut game,
                    primary,
                    Effect::exile(ChooseSpec::SpecificObject(second)),
                );
            }
            if mode >= 2 {
                let changed = if mode == 3 { primary } else { first };
                let mut additions = Vec::new();
                if mode == 2 {
                    additions.push(Effect::pump(
                        3,
                        0,
                        ChooseSpec::SpecificObject(changed),
                        Until::EndOfTurn,
                    ));
                }
                if mode >= 4 {
                    additions.push(Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                        ironsmith::continuous::EffectTarget::Specific(changed),
                        ironsmith::continuous::Modification::SetCardTypes(vec![
                            ironsmith::CardType::Artifact,
                        ]),
                        Until::EndOfTurn,
                    )));
                }
                if mode != 4 {
                    additions.push(
                        Effect::exile(ChooseSpec::SpecificObject(changed)).tag("reciprocal_move"),
                    );
                    // Mode 5 departs while already a noncreature. Its actual
                    // departure receipt, rather than capture-time P/T, applies.
                    if mode != 5 {
                        additions.push(Effect::new(
                            ironsmith::effects::MoveToZoneEffect::new(
                                ChooseSpec::Tagged("reciprocal_move".into()),
                                Zone::Battlefield,
                                false,
                            )
                            .under_owner_control(),
                        ));
                    }
                }
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(
                        primary,
                        B,
                        ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                            ironsmith::target::ObjectFilter::specific(first),
                        ),
                        ironsmith::replacement::ReplacementAction::Additionally(additions),
                    ),
                );
            }
            let mut dm = Choices {
                targets: vec![Target::Object(primary)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                dm.bounds,
                vec![(1, Some(1))],
                "only the original creature is a target"
            );
            assert_eq!(game.damage_on(outsider), 0);
            let history = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some()
                })
                .collect::<Vec<_>>();
            if mode == 1 {
                assert!(history.is_empty());
                assert_eq!(game.damage_on(primary), 0);
                continue;
            }
            let first_phase = history
                .iter()
                .filter(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .unwrap()
                        .source
                        == primary
                })
                .collect::<Vec<_>>();
            assert_eq!(first_phase.len(), 2);
            assert!(first_phase.iter().all(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .unwrap()
                    .amount
                    == 4
            }));
            assert_eq!(
                first_phase[0].event.simultaneous_batch(),
                first_phase[1].event.simultaneous_batch()
            );
            assert_eq!(
                game.player(B).unwrap().life,
                28,
                "first source gains life once from its complete outgoing set"
            );
            if mode == 3 {
                let returned = game
                    .battlefield
                    .iter()
                    .copied()
                    .find(|id| {
                        game.object(*id)
                            .is_some_and(|object| object.name == "Primary creature")
                    })
                    .unwrap();
                assert_ne!(returned, primary);
                assert_eq!(
                    game.damage_on(returned),
                    0,
                    "the second phase cannot hit the new incarnation"
                );
                assert_eq!(history.len(), 2);
                continue;
            }
            assert_eq!(
                game.damage_on(primary),
                match mode {
                    2 => 10,
                    4 | 5 => 5,
                    _ => 7,
                }
            );
            let second_phase = history
                .iter()
                .filter(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .unwrap()
                        .source
                        != primary
                })
                .collect::<Vec<_>>();
            assert_eq!(second_phase.len(), if mode >= 4 { 1 } else { 2 });
            if mode >= 4 {
                assert!(
                    second_phase.iter().all(|record| record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some_and(|event| event.source == second && event.amount == 5)),
                    "a known noncreature source has zero power, whether present or departed"
                );
            } else {
                assert_eq!(
                    second_phase[0].event.simultaneous_batch(),
                    second_phase[1].event.simultaneous_batch()
                );
            }
            assert_ne!(
                first_phase[0].event.simultaneous_batch(),
                second_phase[0].event.simultaneous_batch()
            );
            assert!(second_phase.iter().all(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .unwrap()
                    .target
                    == ironsmith::events::DamageTarget::Object(primary)
            }));
            if mode == 2 {
                let returned = game
                    .battlefield
                    .iter()
                    .copied()
                    .find(|id| {
                        game.object(*id)
                            .is_some_and(|object| object.name == "First other")
                    })
                    .unwrap();
                assert_ne!(returned, first);
                assert_eq!(game.calculated_power(returned), Some(2));
                assert!(
                    second_phase.iter().any(|record| record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some_and(|event| event.source == first && event.amount == 5)),
                    "the old source uses actual departure power, not earlier2 or the returned creature"
                );
                assert!(second_phase.iter().all(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .unwrap()
                        .source
                        != returned
                }));
            }
        }
    }
}

#[test]
fn fight_keeps_an_empty_original_target_slot_instead_of_reusing_the_other_friendly_target() {
    for definition in definitions_text(
        "Exact fight slots",
        "Mana cost: {G}\nType: Sorcery\nTarget creature you control fights target creature.",
    ) {
        for blink in [false, true] {
            let mut game = game();
            let first = creature(&mut game, A, "First fighter", 3, 50);
            let second = creature(&mut game, A, "Second friendly fighter", 5, 50);
            let mut dm = Choices {
                targets: vec![Target::Object(first), Target::Object(second)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if blink {
                let moved = game.move_object_by_effect(first, Zone::Exile).unwrap();
                game.move_object_by_effect(moved, Zone::Battlefield)
                    .unwrap();
            } else {
                let mut first_dm = SelectFirstDecisionMaker;
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::GainControlEffect::permanent(
                        ChooseSpec::SpecificObject(first),
                    )),
                    &mut EffectContext::new(first, B, &mut first_dm),
                )
                .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(
                game.damage_on(second),
                0,
                "the remaining friendly target cannot fill both fight operands"
            );
            assert!(
                !game
                    .turn_store
                    .turn_history
                    .event_records
                    .iter()
                    .chain(game.turn_store.turn_history.staged_event_records.iter())
                    .any(|record| record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some())
            );
        }
    }
}

#[test]
fn fight_commits_both_original_sides_and_captures_observers_before_first_side_additions() {
    for definition in definitions_text(
        "Complete fight batch",
        "Mana cost: {G}\nType: Sorcery\nTarget creature you control fights target creature.",
    ) {
        let mut game = game();
        let first = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Lifelink fighter",
                "Type: Creature — Human\nPower/Toughness: 3/50\nLifelink",
                false,
            )
            .unwrap(),
            A,
            Zone::Battlefield,
        );
        let second = creature(&mut game, B, "Other fighter", 4, 50);
        let observer = game.create_object_from_definition(
            &compile_to_runtime_definition(
                "Damage observer",
                "Type: Enchantment\nWhenever a creature is dealt damage, you gain 1 life.",
                false,
            )
            .unwrap(),
            A,
            Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                first,
                A,
                ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                    ironsmith::target::ObjectFilter::specific(second),
                ),
                ironsmith::replacement::ReplacementAction::Additionally(vec![
                    Effect::exile(ChooseSpec::SpecificObject(first)),
                    Effect::exile(ChooseSpec::SpecificObject(observer)),
                ]),
            ),
        );
        let mut dm = Choices {
            targets: vec![Target::Object(first), Target::Object(second)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert!(!game.battlefield.contains(&first));
        assert!(!game.battlefield.contains(&observer));
        assert_eq!(game.damage_on(second), 3);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "one lifelink gain and both already-matched recipient triggers survive additions"
        );
        let damage = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .is_some()
            })
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 2);
        assert_eq!(
            damage
                .iter()
                .map(|record| record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .unwrap()
                    .amount)
                .sum::<u32>(),
            7
        );
        assert!(damage[0].event.simultaneous_batch().is_some());
        assert_eq!(
            damage[0].event.simultaneous_batch(),
            damage[1].event.simultaneous_batch()
        );
        assert_ne!(damage[0].event.provenance(), damage[1].event.provenance());
        assert!(damage.iter().any(|record| {
            record
                .event
                .downcast::<ironsmith::events::DamageEvent>()
                .is_some_and(|event| {
                    event.source == second
                        && event.target == ironsmith::events::DamageTarget::Object(first)
                        && event.amount == 4
                })
        }));
    }
}
