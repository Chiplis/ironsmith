//! UNVALIDATED: intrinsic graveyard permissions retain complete alternative costs.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/intrinsic_zone_alternative_costs.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows = rows();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn printed(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    printed(
        game,
        owner,
        Zone::Battlefield,
        name,
        "Type: Creature — Bear\nPower/Toughness: 2/2",
    )
}
fn grave_card(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    printed(
        game,
        owner,
        Zone::Graveyard,
        name,
        "Mana cost: {1}\nType: Artifact",
    )
}
fn action(
    game: &GameState,
    player: PlayerId,
    spell: ObjectId,
    alternative: bool,
) -> Option<LegalAction> {
    compute_legal_actions(game,player).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==spell && if alternative {matches!(casting_method,CastingMethod::Alternative(_))} else {matches!(casting_method,CastingMethod::Normal)}))
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    choosers: Vec<PlayerId>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for target in &self.targets {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            );
        }
        self.targets.clone()
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.choosers.push(ctx.player);
        let selected: Vec<_> = self
            .objects
            .iter()
            .copied()
            .filter(|id| {
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            })
            .collect();
        if selected.is_empty() {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        } else {
            selected
        }
    }
}
fn announce(
    game: &mut GameState,
    player: PlayerId,
    spell: ObjectId,
    alternative: bool,
    dm: &mut Choices,
) -> ObjectId {
    game.turn.priority_player = Some(player);
    let action = action(game, player, spell, alternative).unwrap();
    let mut state = PriorityLoopState::new(2);
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
        if state.pending_cast.is_none() {
            return game.stack.last().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("pending cast did not finish")
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn named(game: &GameState, zone: Zone, name: &str) -> ObjectId {
    game.objects_in_deterministic_order()
        .into_iter()
        .find(|object| object.zone == zone && object.name.as_ref() == name)
        .unwrap()
        .id
}
#[test]
fn four_complete_payloads_round_trip_and_one_entry_rider_remains_partial() {
    let rows = rows();
    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter()
            .filter(|row| row["proposed_coverage"] == "partial_not_counted")
            .count(),
        1
    );
    for row in rows
        .iter()
        .filter(|row| row["proposed_coverage"] == "source_complete_unvalidated")
    {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.alternative_casts.len(), 1);
            assert_eq!(
                definition.alternative_casts[0].cast_from_zone(),
                Zone::Graveyard
            );
            let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(text.contains("from your graveyard"), "{text}");
            assert!(text.contains("rather than paying its mana cost"), "{text}");
        }
    }
}
#[test]
fn raffine_keeps_its_ordinary_hand_price_and_uses_the_full_graveyard_price_with_real_aura_targets()
{
    for definition in definitions("Raffine's Guidance") {
        for graveyard in [false, true] {
            let mut game = new_game();
            let host = creature(&mut game, B, "Host");
            let zone = if graveyard {
                Zone::Graveyard
            } else {
                Zone::Hand
            };
            let aura = game.create_object_from_definition(&definition, A, zone);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::White, 1);
            if graveyard {
                assert!(action(&game, A, aura, true).is_none());
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 2);
            }
            assert!(action(&game, A, aura, !graveyard).is_none());
            let mut dm = Choices {
                targets: vec![Target::Object(host)],
                ..Default::default()
            };
            announce(&mut game, A, aura, graveyard, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.current_power(host), Some(3));
            assert_eq!(game.current_toughness(host), Some(3));
            assert_eq!(
                game.object(named(&game, Zone::Battlefield, "Raffine's Guidance"))
                    .unwrap()
                    .attached_to,
                Some(ironsmith::object::AttachmentTarget::Object(host))
            );
        }
    }
}
#[test]
fn scourge_requires_two_owned_controlled_creatures_and_pays_both_components_once() {
    for definition in definitions("Scourge of Nel Toth") {
        let mut game = new_game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 2);
        let first = creature(&mut game, A, "First");
        let opponent = creature(&mut game, B, "Opponent");
        assert!(action(&game, A, spell, true).is_none());
        let second = creature(&mut game, A, "Second");
        let spare = creature(&mut game, A, "Spare");
        let mut dm = Choices {
            objects: vec![first, second],
            ..Default::default()
        };
        announce(&mut game, A, spell, true, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.object(first).is_none());
        assert!(game.object(second).is_none());
        assert!(game.object(opponent).is_some());
        assert!(game.object(spare).is_some());
        assert!(dm.choosers.iter().all(|player| *player == A));
        resolve(&mut game, &mut dm);
        let dragon = named(&game, Zone::Battlefield, "Scourge of Nel Toth");
        assert!(game.object_has_ability(
            dragon,
            &ironsmith::static_abilities::StaticAbility::flying()
        ));
    }
}
#[test]
fn squee_exiles_exactly_four_other_cards_from_its_casters_graveyard_then_attacks_with_haste() {
    for definition in definitions("Squee, Dubious Monarch") {
        let mut game = new_game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let mut payment: Vec<_> = (0..3)
            .map(|n| grave_card(&mut game, A, &format!("Pay {n}")))
            .collect();
        let theirs = grave_card(&mut game, B, "Not yours");
        assert!(
            action(&game, A, spell, true).is_none(),
            "the cast card and opponent cards cannot fill the four-card cost"
        );
        payment.push(grave_card(&mut game, A, "Pay fourth"));
        let mut dm = Choices {
            objects: payment.clone(),
            ..Default::default()
        };
        announce(&mut game, A, spell, true, &mut dm);
        assert_eq!(game.exile.len(), 4);
        assert!(game.object(theirs).is_some());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut dm);
        let squee = named(&game, Zone::Battlefield, "Squee, Dubious Monarch");
        assert!(
            game.object_has_ability(squee, &ironsmith::static_abilities::StaticAbility::haste())
        );
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(squee, ironsmith::combat_state::AttackTarget::Player(B))],
        )
        .unwrap();
        game.combat = Some(combat);
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut dm);
        let tokens: Vec<_> = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| object.zone == Zone::Battlefield && object.is_token())
            .map(|object| object.id)
            .collect();
        assert_eq!(tokens.len(), 1);
        assert!(game.is_tapped(tokens[0]));
        assert!(
            game.combat
                .as_ref()
                .unwrap()
                .attackers
                .iter()
                .any(|attacker| attacker.creature == tokens[0])
        );
    }
}
fn glimpse_library(game: &mut GameState) -> Vec<ObjectId> {
    (0..3)
        .map(|n| {
            printed(
                game,
                A,
                Zone::Library,
                &format!("Library {n}"),
                "Mana cost: {1}\nType: Artifact",
            )
        })
        .collect()
}
#[test]
fn glimpse_condition_uses_the_caster_and_is_locked_at_announcement_while_its_rider_tracks_the_method()
 {
    for definition in definitions("Glimpse the Cosmos") {
        for graveyard in [false, true] {
            let mut game = new_game();
            let library = glimpse_library(&mut game);
            let spell = game.create_object_from_definition(
                &definition,
                A,
                if graveyard {
                    Zone::Graveyard
                } else {
                    Zone::Hand
                },
            );
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 1);
            let giant = printed(
                &mut game,
                B,
                Zone::Battlefield,
                "Giant witness",
                "Type: Creature — Giant\nPower/Toughness: 3/3",
            );
            if graveyard {
                assert!(action(&game, A, spell, true).is_none());
                game.object_mut(giant).unwrap().controller = A;
            } else {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 1);
            }
            let mut dm = Choices {
                objects: vec![library[1]],
                ..Default::default()
            };
            announce(&mut game, A, spell, graveyard, &mut dm);
            game.move_object_by_effect(giant, Zone::Graveyard).unwrap();
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.player(A).unwrap().library.len(), 2);
            named(
                &game,
                if graveyard {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                },
                "Glimpse the Cosmos",
            );
        }
    }
}
#[test]
fn glimpse_exiles_if_countered_but_a_stack_bounce_still_returns_to_hand() {
    for definition in definitions("Glimpse the Cosmos") {
        for counter in [false, true] {
            let mut game = new_game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            let source = printed(
                &mut game,
                A,
                Zone::Battlefield,
                "Giant source",
                "Type: Creature — Giant\nPower/Toughness: 3/3",
            );
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 1);
            let mut dm = Choices::default();
            let stack = announce(&mut game, A, spell, true, &mut dm);
            let effect = if counter {
                Effect::counter(ChooseSpec::SpecificObject(stack))
            } else {
                Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                    ChooseSpec::SpecificObject(stack),
                ))
            };
            execute_effect(
                &mut game,
                &effect,
                &mut EffectContext::new(source, B, &mut dm),
            )
            .unwrap();
            named(
                &game,
                if counter { Zone::Exile } else { Zone::Hand },
                "Glimpse the Cosmos",
            );
            assert!(game.stack.is_empty());
        }
    }
}
#[test]
fn alternative_price_taxes_apply_without_reintroducing_the_printed_price_or_unlocking_other_zones()
{
    for definition in definitions("Raffine's Guidance") {
        let mut game = new_game();
        let host = creature(&mut game, A, "Aura host");
        printed(
            &mut game,
            A,
            Zone::Battlefield,
            "Tax",
            "Type: Artifact\nSpells you cast cost {1} more to cast.",
        );
        let grave = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let exile = game.create_object_from_definition(&definition, A, Zone::Exile);
        let foreign = game.create_object_from_definition(&definition, B, Zone::Graveyard);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        assert!(action(&game, A, grave, true).is_none());
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        assert!(action(&game, A, grave, true).is_some());
        assert!(action(&game, A, exile, true).is_none());
        assert!(action(&game, A, foreign, true).is_none());
        let mut dm = Choices {
            targets: vec![Target::Object(host)],
            ..Default::default()
        };
        announce(&mut game, A, grave, true, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut dm);
        assert_eq!(game.current_power(host), Some(3));
    }
}
