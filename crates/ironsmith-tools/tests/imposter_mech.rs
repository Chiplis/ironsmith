//! Imposter Mech: "You may have this Vehicle enter as a copy of a creature an
//! opponent controls, except it's a Vehicle artifact with crew 3 and it loses
//! all other card types. Crew 3"
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Imposter Mech",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

struct Choices {
    copy: bool,
    pick: Vec<&'static str>,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.copy
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        // "Choose which replacement effect to apply": copy or not.
        let wanted = if self.copy {
            "Enter as a copy of Wind Drake"
        } else {
            "Do not apply"
        };
        let matching: Vec<_> = ctx
            .options
            .iter()
            .filter(|o| o.legal && o.description.starts_with(wanted))
            .map(|o| o.index)
            .collect();
        if matching.is_empty() {
            ctx.options
                .iter()
                .filter(|o| o.legal)
                .take(ctx.min.max(1))
                .map(|o| o.index)
                .collect()
        } else {
            matching
        }
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let picked: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal && self.pick.contains(&c.name.as_str()))
            .map(|c| c.id)
            .collect();
        if picked.is_empty() {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(ctx.min)
                .map(|c| c.id)
                .collect()
        } else {
            picked
        }
    }
}

fn creature(name: &str, power: i32, flying: bool) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Drake])
        .power_toughness(PowerToughness::fixed(power, power));
    if flying {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::flying()));
    }
    builder.build()
}

/// Imposter Mech enters under Alice's control while Bob controls a Wind Drake
/// (2/2 flying) and Alice controls a 3/3 Ogre to crew with.
fn enter(copy: bool) -> (GameState, ObjectId, ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&creature("Wind Drake", 2, true), bob, Zone::Battlefield);
    let ogre = game.create_object_from_definition(
        &creature("Crew Ogre", 3, false),
        alice,
        Zone::Battlefield,
    );
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let mut dm = Choices {
        copy,
        pick: vec!["Wind Drake"],
    };
    let mech = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .expect("enters")
        .new_id;
    (game, mech, ogre)
}

fn has_crew(game: &GameState, id: ObjectId) -> bool {
    game.current_characteristics(id).is_some_and(|chars| {
        chars.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Activated(activated)
                if format!("{:?}", activated.mana_cost).contains("Crew"))
        })
    })
}

#[test]
fn copies_an_opponents_creature_as_a_noncreature_vehicle_artifact_with_crew() {
    let (game, mech, _) = enter(true);
    let chars = game.current_characteristics(mech).unwrap();
    assert_eq!(game.object(mech).unwrap().name, "Wind Drake", "copied name");
    assert_eq!(
        chars.card_types,
        vec![CardType::Artifact],
        "loses all other card types"
    );
    assert!(chars.subtypes.contains(&Subtype::Vehicle));
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::Flying),
        "copied abilities are kept"
    );
    assert!(has_crew(&game, mech), "gains crew 3");
    assert!(
        !chars.card_types.contains(&CardType::Creature),
        "not a creature until crewed"
    );
}

#[test]
fn crewing_the_copy_makes_it_an_artifact_creature() {
    let (mut game, mech, ogre) = enter(true);
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == mech))
        .expect("crew is activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        copy: true,
        pick: vec!["Crew Ogre"],
    };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    assert!(game.is_tapped(ogre), "crewed by tapping the Ogre");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let chars = game.current_characteristics(mech).unwrap();
    assert!(chars.card_types.contains(&CardType::Creature));
    assert!(chars.card_types.contains(&CardType::Artifact));
    assert_eq!(game.calculated_power(mech), Some(2), "copied power");
}

#[test]
fn declining_the_copy_leaves_the_printed_vehicle() {
    let (game, mech, _) = enter(false);
    assert_eq!(game.object(mech).unwrap().name, "Imposter Mech");
    let chars = game.current_characteristics(mech).unwrap();
    assert!(chars.subtypes.contains(&Subtype::Vehicle));
    assert!(has_crew(&game, mech));
}
