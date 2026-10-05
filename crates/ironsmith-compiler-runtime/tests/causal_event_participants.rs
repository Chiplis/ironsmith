//! Captured cause, affected participant and completed action boundaries.
//! Source proposals only: every scenario is authored but unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/causal_event_participants.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn object(game: &mut GameState, player: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count {
        object(
            game,
            player,
            Zone::Library,
            "Library resource",
            "Type: Land",
        );
    }
}
#[derive(Default)]
struct Choices {
    preferred: Vec<Target>,
    yes: bool,
    rotate: bool,
    required_target_controller: Option<PlayerId>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.yes
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let mut selected = Vec::new();
        for requirement in &context.requirements {
            if let Some(expected) = self.required_target_controller {
                for target in &requirement.legal_targets {
                    if let Target::Object(id) = target {
                        assert_eq!(
                            game.current_controller(*id),
                            Some(expected),
                            "the causing controller is the target antecedent"
                        );
                    }
                }
            }
            let choices: Vec<_> = self
                .preferred
                .iter()
                .copied()
                .filter(|t| requirement.legal_targets.contains(t))
                .take(requirement.max_targets.unwrap_or(usize::MAX))
                .collect();
            if choices.len() >= requirement.min_targets {
                selected.extend(choices);
            } else {
                selected.extend(SelectFirstDecisionMaker.decide_targets(
                    game,
                    &TargetsContext::new(
                        context.player,
                        context.source,
                        "fallback",
                        vec![requirement.clone()],
                    ),
                ));
            }
        }
        if self.rotate {
            self.preferred.retain(|target| !selected.contains(target));
        }
        selected
    }
}
fn action(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending without decision: {progress:?}");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut Choices) -> ObjectId {
    let stable = game.object(spell).unwrap().stable_id;
    action(
        game,
        player,
        LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        },
        dm,
    );
    game.find_object_by_stable_id(stable).unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn activation(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}
fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token)
        })
        .collect()
}
#[test]
fn five_complete_oracle_bodies_keep_direct_artifact_identity() {
    assert_eq!(fixtures().len(), 5);
    assert_eq!(
        fixtures()
            .iter()
            .filter(|r| r["proposed_complete"] == true)
            .count(),
        5
    );
    for row in fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
    {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}
#[test]
fn baral_uses_countering_controller_not_countered_controller_and_keeps_reduction_and_optional_loot()
{
    for definition in definitions("Baral, Chief of Compliance") {
        for (caster, counterer, yes, expected_draws) in [
            (B, A, true, 1),
            (A, A, true, 1),
            (B, B, true, 0),
            (A, B, true, 0),
            (B, A, false, 0),
        ] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            library(&mut game, A, 3);
            library(&mut game, B, 3);
            let target = object(
                &mut game,
                caster,
                Zone::Hand,
                "Countered spell",
                "Mana cost: {0}\nType: Instant\nDraw a card.",
            );
            let mut dm = Choices {
                yes,
                ..Default::default()
            };
            let target = cast(&mut game, caster, target, &mut dm);
            let counter = object(
                &mut game,
                counterer,
                Zone::Hand,
                "Countering spell",
                "Mana cost: {1}{U}\nType: Instant\nCounter target spell.",
            );
            game.player_mut(counterer)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Blue, 1);
            if counterer != A {
                game.player_mut(counterer)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::ManaSymbol::Colorless, 1);
            }
            dm.preferred = vec![Target::Object(target)];
            cast(&mut game, counterer, counter, &mut dm);
            assert_eq!(
                game.player(counterer).unwrap().mana_pool.total(),
                0,
                "Baral reduces only its controller's instant"
            );
            resolve(&mut game, &mut dm);
            assert_eq!(game.stack.len(), usize::from(counterer == A));
            if counterer == A {
                resolve(&mut game, &mut dm);
            }
            assert_eq!(game.player(A).unwrap().library.len(), 3 - expected_draws);
            assert!(
                game.player(A).unwrap().hand.is_empty(),
                "an accepted draw must be followed by its discard"
            );
            assert!(game.stack_is_empty());
        }
    }
}
#[test]
fn lullmage_real_seven_merfolk_cost_counters_then_optionally_creates_its_blue_token() {
    for definition in definitions("Lullmage Mentor") {
        for (yes, uncounterable) in [(true, false), (false, false), (true, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut merfolk = vec![source];
            for _ in 0..6 {
                merfolk.push(object(
                    &mut game,
                    A,
                    Zone::Battlefield,
                    "Merfolk cost resource",
                    "Type: Creature — Merfolk\nPower/Toughness: 1/1",
                ));
            }
            let target = object(
                &mut game,
                B,
                Zone::Hand,
                "Target spell",
                if uncounterable {
                    "Mana cost: {0}\nType: Instant\nThis spell can't be countered.\nDraw a card."
                } else {
                    "Mana cost: {0}\nType: Instant\nDraw a card."
                },
            );
            library(&mut game, B, 2);
            let mut dm = Choices {
                yes,
                ..Default::default()
            };
            let target = cast(&mut game, B, target, &mut dm);
            dm.preferred = vec![Target::Object(target)];
            action(
                &mut game,
                A,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: activation(&definition),
                },
                &mut dm,
            );
            assert!(merfolk.iter().all(|id| game.is_tapped(*id)));
            assert_eq!(game.stack.len(), 2);
            resolve(&mut game, &mut dm);
            if uncounterable {
                assert!(tokens(&game).is_empty());
                assert_eq!(game.stack.len(), 1);
            } else {
                assert_eq!(game.stack.len(), 1);
                resolve(&mut game, &mut dm);
                let made = tokens(&game);
                assert_eq!(made.len(), usize::from(yes));
                if yes {
                    assert_eq!(game.calculated_power(made[0]), Some(1));
                    assert_eq!(game.calculated_toughness(made[0]), Some(1));
                    assert!(game.current_has_subtype(made[0], ironsmith::Subtype::Merfolk));
                    assert!(
                        game.current_colors(made[0])
                            .unwrap()
                            .contains(ironsmith::Color::Blue)
                    );
                }
            }
        }
    }
}
#[test]
fn karmic_justice_sees_its_simultaneous_destruction_and_pins_causing_opponent_even_with_exile_replacement()
 {
    for definition in definitions("Karmic Justice") {
        for exile in [false, true] {
            let mut game = game();
            let justice = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let victim = object(
                &mut game,
                A,
                Zone::Battlefield,
                "Friendly artifact",
                "Type: Artifact",
            );
            let b1 = object(
                &mut game,
                B,
                Zone::Battlefield,
                "First accountable creature",
                "Type: Creature\nPower/Toughness: 2/2",
            );
            let b2 = object(
                &mut game,
                B,
                Zone::Battlefield,
                "Second accountable creature",
                "Type: Creature\nPower/Toughness: 2/2",
            );
            let c = object(
                &mut game,
                C,
                Zone::Battlefield,
                "Other opponent creature",
                "Type: Creature\nPower/Toughness: 2/2",
            );
            if exile {
                object(
                    &mut game,
                    C,
                    Zone::Battlefield,
                    "Exile replacement",
                    "Type: Enchantment\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
                );
            }
            let spell = object(
                &mut game,
                B,
                Zone::Hand,
                "Mass destruction",
                "Mana cost: {0}\nType: Instant\nDestroy all artifacts and enchantments.",
            );
            let mut dm = Choices {
                yes: true,
                preferred: vec![Target::Object(b1), Target::Object(b2)],
                rotate: true,
                required_target_controller: Some(B),
            };
            cast(&mut game, B, spell, &mut dm);
            resolve(&mut game, &mut dm);
            assert!(game.object(justice).is_none() && game.object(victim).is_none());
            assert_eq!(
                game.stack.len(),
                2,
                "Justice sees each noncreature permanent including itself"
            );
            resolve(&mut game, &mut dm);
            resolve(&mut game, &mut dm);
            assert!(game.object(b1).is_none() && game.object(b2).is_none());
            assert!(game.object(c).is_some());
        }
    }
}
#[test]
fn karmic_justice_does_not_remember_an_observer_that_left_in_an_earlier_instruction() {
    for definition in definitions("Karmic Justice") {
        let mut game = game();
        let justice = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = object(
            &mut game,
            A,
            Zone::Battlefield,
            "Later artifact",
            "Type: Artifact",
        );
        let accountable = object(
            &mut game,
            B,
            Zone::Battlefield,
            "Accountable creature",
            "Type: Creature\nPower/Toughness: 2/2",
        );
        let spell = object(
            &mut game,
            B,
            Zone::Hand,
            "Sequential destruction",
            "Mana cost: {0}\nType: Instant\nDestroy target enchantment. Destroy target artifact.",
        );
        let mut dm = Choices {
            yes: true,
            preferred: vec![Target::Object(justice), Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, B, spell, &mut dm);
        dm.preferred = vec![Target::Object(accountable)];
        resolve(&mut game, &mut dm);
        assert_eq!(
            game.stack.len(),
            1,
            "only the first instruction had Justice as an observer"
        );
        resolve(&mut game, &mut dm);
        assert!(game.object(accountable).is_none());
    }
}
#[test]
fn karmic_justice_excludes_own_effects_creatures_and_indestructible_failures() {
    for definition in definitions("Karmic Justice") {
        for (actor, kind, indestructible) in [
            (A, "Artifact", false),
            (B, "Creature", false),
            (B, "Artifact", true),
        ] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let text = format!(
                "Type: {kind}\n{}{}",
                if kind == "Creature" {
                    "Power/Toughness: 2/2\n"
                } else {
                    ""
                },
                if indestructible { "Indestructible" } else { "" }
            );
            let target = object(&mut game, A, Zone::Battlefield, "Excluded victim", &text);
            let spell = object(
                &mut game,
                actor,
                Zone::Hand,
                "Destruction",
                "Mana cost: {0}\nType: Instant\nDestroy target permanent.",
            );
            let mut dm = Choices {
                yes: true,
                preferred: vec![Target::Object(target)],
                ..Default::default()
            };
            cast(&mut game, actor, spell, &mut dm);
            resolve(&mut game, &mut dm);
            assert!(game.stack_is_empty());
            assert_eq!(game.object(target).is_some(), indestructible);
        }
    }
}
#[test]
fn spiritual_focus_counts_each_opponent_caused_discard_and_keeps_optional_draw_independent_of_life()
{
    for definition in definitions("Spiritual Focus") {
        for (actor, yes, expected_life, expected_draws) in
            [(B, true, 24, 2), (B, false, 24, 0), (A, true, 20, 0)]
        {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            library(&mut game, A, 4);
            for _ in 0..2 {
                object(&mut game, A, Zone::Hand, "Discarded card", "Type: Land");
            }
            let spell = object(
                &mut game,
                actor,
                Zone::Hand,
                "Discard effect",
                "Mana cost: {0}\nType: Instant\nTarget player discards two cards.",
            );
            let mut dm = Choices {
                yes,
                preferred: vec![Target::Player(A)],
                ..Default::default()
            };
            cast(&mut game, actor, spell, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.stack.len(), if actor == B { 2 } else { 0 });
            while !game.stack_is_empty() {
                resolve(&mut game, &mut dm);
            }
            assert_eq!(game.player(A).unwrap().life, expected_life);
            assert_eq!(game.player(A).unwrap().hand.len(), expected_draws);
            assert_eq!(game.player(A).unwrap().library.len(), 4 - expected_draws);
        }
    }
}
// Complete-card gate retained while the discard-destination/delayed matcher
// source closure awaits review. Grammar-only qualification earned no credit.
#[test]
fn pure_intentions_complete_card_returns_only_the_exact_discarded_destinations() {
    for definition in definitions("Pure Intentions") {
        let mut game = game();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::White, 1);
        let intention = game.create_object_from_definition(&definition, A, Zone::Hand);
        let discarded = object(&mut game, A, Zone::Hand, "Protected card", "Type: Land");
        let stable = game.object(discarded).unwrap().stable_id;
        let mut dm = Choices {
            yes: true,
            ..Default::default()
        };
        cast(&mut game, A, intention, &mut dm);
        resolve(&mut game, &mut dm);
        let spell = object(
            &mut game,
            B,
            Zone::Hand,
            "Opponent discard",
            "Mana cost: {0}\nType: Instant\nTarget player discards a card.",
        );
        dm.preferred = vec![Target::Player(A)];
        cast(&mut game, B, spell, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut dm);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, discarded);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
    }
}

#[cfg(test)]
mod causal_discard_wire_tests {
    use ironsmith_core::{PlayerFilter, trigger_model::Trigger};
    #[test]
    fn old_causal_discard_json_remains_singular_and_new_grouped_flag_round_trips() {
        let singular = Trigger::player_discards_card_caused_by_controller(
            PlayerFilter::You,
            None,
            PlayerFilter::Opponent,
            true,
        );
        let mut wire = serde_json::to_value(&singular).unwrap();
        wire["kind"]["PlayerDiscardsCardCausedByController"]
            .as_object_mut()
            .unwrap()
            .remove("one_or_more");
        assert_eq!(serde_json::from_value::<Trigger>(wire).unwrap(), singular);
        let grouped = Trigger::player_discards_cards_caused_by_controller(
            PlayerFilter::You,
            None,
            PlayerFilter::Opponent,
            true,
        );
        assert_eq!(
            serde_json::from_str::<Trigger>(&serde_json::to_string(&grouped).unwrap()).unwrap(),
            grouped
        );
    }
}

fn register_intentions(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::White, 1);
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    cast(game, A, spell, dm);
    resolve(game, dm);
}
fn discard_from_opponent(game: &mut GameState, amount: u32, dm: &mut Choices) {
    let text = format!("Mana cost: {{0}}\nType: Instant\nTarget player discards {amount} cards.");
    let spell = object(game, B, Zone::Hand, "Opponent discard batch", &text);
    dm.preferred = vec![Target::Player(A)];
    cast(game, B, spell, dm);
    resolve(game, dm);
}
fn next_end_step(game: &mut GameState, dm: &mut Choices) {
    game.turn.phase = ironsmith::Phase::Ending;
    game.turn.step = Some(ironsmith::game_state::Step::End);
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
#[test]
fn intentions_delayed_discard_groups_exact_arrivals_and_does_not_follow_an_extra_move() {
    for definition in definitions("Pure Intentions") {
        for moves_again in [false, true] {
            let mut game = game();
            let mut dm = Choices {
                yes: true,
                ..Default::default()
            };
            register_intentions(&mut game, &definition, &mut dm);
            let first = object(
                &mut game,
                A,
                Zone::Hand,
                "First protected card",
                "Type: Land",
            );
            let first_stable = game.object(first).unwrap().stable_id;
            let second = object(
                &mut game,
                A,
                Zone::Hand,
                "Second protected card",
                "Type: Land",
            );
            let second_stable = game.object(second).unwrap().stable_id;
            discard_from_opponent(&mut game, 2, &mut dm);
            assert_eq!(game.stack.len(), 1, "one batch creates one trigger");
            let original_first = game.find_object_by_stable_id(first_stable).unwrap();
            let original_second = game.find_object_by_stable_id(second_stable).unwrap();
            if moves_again {
                let exile = game
                    .move_object_by_effect(original_first, Zone::Exile)
                    .unwrap();
                game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
            }
            resolve(&mut game, &mut dm);
            let current_first = game.find_object_by_stable_id(first_stable).unwrap();
            let current_second = game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(
                game.object(current_first).unwrap().zone,
                if moves_again {
                    Zone::Graveyard
                } else {
                    Zone::Hand
                }
            );
            assert_eq!(game.object(current_second).unwrap().zone, Zone::Hand);
            assert_ne!(current_second, original_second);
            // "Whenever ... this turn" keeps watching after its first batch.
            discard_from_opponent(&mut game, 1, &mut dm);
            assert_eq!(game.stack.len(), 1);
            resolve(&mut game, &mut dm);
            assert!(!game.player(A).unwrap().hand.is_empty());
            game.next_turn();
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            discard_from_opponent(&mut game, 1, &mut dm);
            assert!(
                game.stack_is_empty(),
                "the registration expires at the turn boundary"
            );
        }
    }
}
#[test]
fn intentions_never_returns_a_discard_replaced_with_exile() {
    for definition in definitions("Pure Intentions") {
        let mut game = game();
        let mut dm = Choices {
            yes: true,
            ..Default::default()
        };
        register_intentions(&mut game, &definition, &mut dm);
        object(
            &mut game,
            B,
            Zone::Battlefield,
            "Exile discard destination",
            "Type: Enchantment\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
        );
        let card = object(
            &mut game,
            A,
            Zone::Hand,
            "Protected but exiled",
            "Type: Land",
        );
        let stable = game.object(card).unwrap().stable_id;
        discard_from_opponent(&mut game, 1, &mut dm);
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut dm);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
    }
}
#[test]
fn intentions_self_discard_pins_graveyard_incarnation_before_delayed_registration_and_resolution() {
    for definition in definitions("Pure Intentions") {
        // 0: no extra move; 1: before registration; 2: after registration.
        for extra_move in 0..3 {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let mut dm = Choices {
                yes: true,
                ..Default::default()
            };
            discard_from_opponent(&mut game, 1, &mut dm);
            assert_eq!(
                game.stack.len(),
                1,
                "self-discard trigger functions from the old hand snapshot"
            );
            let move_again = |game: &mut GameState| {
                let id = game.find_object_by_stable_id(stable).unwrap();
                let exiled = game.move_object_by_effect(id, Zone::Exile).unwrap();
                game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
            };
            if extra_move == 1 {
                move_again(&mut game);
            }
            resolve(&mut game, &mut dm);
            assert!(
                game.player(A).unwrap().hand.is_empty(),
                "registration itself does not return the card"
            );
            if extra_move == 2 {
                move_again(&mut game);
            }
            next_end_step(&mut game, &mut dm);
            assert_eq!(game.stack.len(), 1);
            resolve(&mut game, &mut dm);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                if extra_move == 0 {
                    Zone::Hand
                } else {
                    Zone::Graveyard
                }
            );
        }
    }
}

#[test]
fn delayed_discard_wire_keeps_cause_grouping_and_real_matcher() {
    let spec = ironsmith_core::DelayedTriggerSpec::PlayerDiscardsCard {
        player: ironsmith_core::PlayerFilter::You,
        filter: None,
        cause_controller: Some(ironsmith_core::PlayerFilter::Opponent),
        effect_like_only: true,
        one_or_more: true,
    };
    let restored: ironsmith_core::DelayedTriggerSpec =
        serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    assert_eq!(spec, restored);
    let matcher = ironsmith::triggers::Trigger::from_delayed_trigger_spec(restored);
    let native = matcher
        .downcast_ref::<ironsmith::triggers::YouDiscardCardTrigger>()
        .unwrap();
    assert!(native.one_or_more && native.effect_like_only);
    assert_eq!(
        native.cause_controller,
        Some(ironsmith_core::PlayerFilter::Opponent)
    );
}
