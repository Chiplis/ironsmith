//! Actual Mayar source: existing kind-specific inheritance with a designation selector.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn compile(name: &str, text: &str, keyword: Option<&str>) -> [CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text,"custom_keyword_identities":keyword.into_iter().collect::<Vec<_>>()})).unwrap()
}
fn definitions() -> [CardDefinition; 2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-recursive-next-wave.json")).unwrap();
    let card = &fixture["mechanics"][0]["representative_cards"][0];
    assert_eq!(card["id"], "Mayar, Nullcrowned_BTB");
    compile(
        card["name"].as_str().unwrap(),
        card["executable_text"].as_str().unwrap(),
        Some("breach"),
    )
}
struct Accept;
impl DecisionMaker for Accept {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}
fn settle(game: &mut GameState) {
    let mut dm = Accept;
    for _ in 0..16 {
        let mut q = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
        }
    }
    panic!("recursive collection did not settle");
}
fn add(
    game: &mut GameState,
    name: &str,
    text: &str,
    owner: PlayerId,
    keyword: Option<&str>,
) -> ObjectId {
    game.create_object_from_definition(&compile(name, text, keyword)[0], owner, Zone::Battlefield)
}
#[test]
fn actual_recursive_inherits_each_eligible_kind_but_excludes_designated_abilities_and_ineligible_donors()
 {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for definition in definitions() {
        eprintln!("RECURSIVE_TRACE abilities={:#?}",definition.abilities);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.turn_number = 3;
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add(
            &mut game,
            "Other friendly Curse",
            "Type: Enchantment — Curse\nFlying\nRecursive — Menace\n{0}: You gain 1 life.\nRecursive — {0}: You lose 7 life.\nAt the beginning of your end step, you gain 2 life.\nRecursive — At the beginning of your end step, you lose 9 life.",
            A,
            None,
        );
        add(
            &mut game,
            "Friendly breach donor",
            "Type: Enchantment Creature — Demon\nPower/Toughness: 2/2\nReach\nWhenever this creature deals combat damage to a player, you may have it become an Aura Curse enchantment with enchant opponent until your next turn. If you do, attach it to that player.",
            A,
            Some("breach"),
        );
        add(
            &mut game,
            "Friendly ordinary creature",
            "Type: Creature — Beast\nPower/Toughness: 2/2\nTrample",
            A,
            None,
        );
        add(
            &mut game,
            "Opponent Curse",
            "Type: Enchantment — Curse\nHexproof",
            B,
            None,
        );
        assert!(
            !game.object_has_static_ability_id(source, Id::Flying),
            "Recursive's condition requires being a Curse"
        );
        game.queue_trigger_event(
            Default::default(),
            ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::DamageEvent::with_cause(
                    source,
                    ironsmith::events::DamageTarget::Player(B),
                    1,
                    true,
                    ironsmith::events::EventCause::combat_damage(source),
                ),
                Default::default(),
            ),
        );
        settle(&mut game);
        assert!(
            game.calculated_subtypes(source)
                .contains(&ironsmith::Subtype::Curse)
        );
        assert!(
            game.object_has_static_ability_id(source, Id::Flying),
            "other Curse donor does not need breach"
        );
        assert!(
            game.object_has_static_ability_id(source, Id::Reach),
            "creature donor with breach does not need Curse subtype"
        );
        assert!(
            !game.object_has_static_ability_id(source, Id::Menace),
            "Recursive-labelled static excluded"
        );
        assert!(
            !game.object_has_static_ability_id(source, Id::Trample),
            "creature without breach excluded"
        );
        assert!(
            !game.object_has_static_ability_id(source, Id::Hexproof),
            "opponent's Curse excluded"
        );
        let chars = game.calculated_characteristics(source).unwrap();
        let mut gain_index = None;
        let mut copied_trigger = false;
        for (index, ability) in chars.abilities.iter().enumerate() {
            if matches!(
                chars.abilities.origin(index),
                Some(ironsmith::continuous::AbilityOrigin::Borrowed { .. })
            ) {
                assert!(
                    !ability.has_designation("recursive"),
                    "all borrowed kinds exclude Recursive provenance"
                );
                match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(activated) => {
                        assert!(!activated.effects.all_effects().iter().any(|effect| {
                            effect
                                .downcast_ref::<ironsmith::effects::LoseLifeEffect>()
                                .is_some()
                        }));
                        if activated.effects.all_effects().iter().any(|effect| {
                            effect
                                .downcast_ref::<ironsmith::effects::GainLifeEffect>()
                                .is_some()
                        }) {
                            gain_index = Some(index);
                        }
                    }
                    ironsmith::ability::AbilityKind::Triggered(triggered) => {
                        assert!(!triggered.effects.all_effects().iter().any(|effect| {
                            effect
                                .downcast_ref::<ironsmith::effects::LoseLifeEffect>()
                                .is_some()
                        }));
                        copied_trigger |= triggered.effects.all_effects().iter().any(|effect| {
                            effect
                                .downcast_ref::<ironsmith::effects::GainLifeEffect>()
                                .is_some()
                        });
                    }
                    _ => {}
                }
            }
        }
        assert!(copied_trigger, "ordinary triggered program inherited");
        let index = gain_index.expect("ordinary activated program inherited");
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==index)).unwrap();
        use ironsmith::game_loop::{
            PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
            apply_priority_response_with_dm,
        };
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        assert!(!state.has_pending_action());
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.player(A).unwrap().life,
            21,
            "borrowed activated program executes for its new controller"
        );
    }
}
