use ironsmith::card::CardBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::{
    ChooseObjectsEffect, DiscardEffect, EffectExecutor, SacrificeEffect, UnlessActionEffect,
};
use ironsmith::filter::ObjectFilter;
use ironsmith::ids::CardId;
use ironsmith::target::PlayerFilter;
use ironsmith::types::CardType;
use ironsmith::zone::Zone;
use ironsmith::{GameState, ObjectId, PlayerId};
const A: PlayerId = PlayerId::from_index(0);
#[derive(Default)]
struct Pick {
    ids: Vec<ObjectId>,
    prompts: Vec<SelectObjectsContext>,
    booleans: usize,
    pending: bool,
}
impl DecisionMaker for Pick {
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        self.prompts.push(context.clone());
        self.ids.clone()
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.booleans += 1;
        false
    }
    fn awaiting_choice(&self) -> bool {
        self.pending && !self.prompts.is_empty()
    }
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn card(g: &mut GameState, zone: Zone, kind: CardType) -> ObjectId {
    let c = CardBuilder::new(CardId::new(), format!("{kind:?}"))
        .card_types(vec![kind])
        .build();
    g.create_object_from_card(&c, A, zone)
}
fn discard() -> UnlessActionEffect {
    let mut alternative = DiscardEffect::you(1);
    alternative.card_filter = Some(ObjectFilter::creature().in_zone(Zone::Hand));
    UnlessActionEffect::new(
        vec![Effect::new(DiscardEffect::you(2))],
        vec![Effect::new(alternative)],
        PlayerFilter::You,
    )
}
#[test]
fn combined_discard_selects_one_creature_or_any_two_including_creatures() {
    for selected in [vec![0], vec![0, 1], vec![1, 2]] {
        let mut g = game();
        let ids = [
            card(&mut g, Zone::Hand, CardType::Creature),
            card(&mut g, Zone::Hand, CardType::Land),
            card(&mut g, Zone::Hand, CardType::Instant),
        ];
        let chosen = selected.iter().map(|i| ids[*i]).collect::<Vec<_>>();
        let mut dm = Pick {
            ids: chosen.clone(),
            ..Default::default()
        };
        let source = g.new_object_id();
        discard()
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(dm.booleans, 0);
        assert_eq!(dm.prompts.len(), 1);
        assert_eq!(dm.prompts[0].alternatives.len(), 2);
        for id in ids {
            assert_eq!(
                g.player(A).unwrap().hand.contains(&id),
                !chosen.contains(&id)
            );
        }
        assert_eq!(g.player(A).unwrap().graveyard.len(), chosen.len());
    }
}
#[test]
fn combined_discard_rejects_invalid_duplicate_and_foreign_selections_atomically() {
    for mode in 0..3 {
        let mut g = game();
        let creature = card(&mut g, Zone::Hand, CardType::Creature);
        let land = card(&mut g, Zone::Hand, CardType::Land);
        let foreign = card(&mut g, Zone::Battlefield, CardType::Creature);
        let mut dm = Pick {
            ids: match mode {
                0 => vec![land],
                1 => vec![creature, creature],
                _ => vec![foreign],
            },
            ..Default::default()
        };
        let source = g.new_object_id();
        assert!(
            discard()
                .execute(
                    &mut g,
                    &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm)
                )
                .is_err()
        );
        assert_eq!(g.player(A).unwrap().hand.len(), 2);
        assert!(g.player(A).unwrap().graveyard.is_empty());
    }
}
#[test]
fn combined_discard_preserves_do_as_much_as_possible_and_pending_choices() {
    for kind in [None, Some(CardType::Land), Some(CardType::Creature)] {
        let mut g = game();
        let ids = kind
            .map(|k| card(&mut g, Zone::Hand, k))
            .into_iter()
            .collect::<Vec<_>>();
        let mut dm = Pick {
            ids: ids.clone(),
            ..Default::default()
        };
        let source = g.new_object_id();
        discard()
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert!(g.player(A).unwrap().hand.is_empty());
        assert_eq!(dm.booleans, usize::from(kind == Some(CardType::Creature)));
    }
    let mut g = game();
    let creature = card(&mut g, Zone::Hand, CardType::Creature);
    let _ = card(&mut g, Zone::Hand, CardType::Land);
    let mut dm = Pick {
        ids: vec![creature],
        pending: true,
        ..Default::default()
    };
    let source = g.new_object_id();
    discard()
        .execute(
            &mut g,
            &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(g.player(A).unwrap().hand.len(), 2);
    assert_eq!(dm.prompts.len(), 1);
}
#[test]
fn combined_sacrifice_uses_native_action_and_payment_owners() {
    for two in [false, true] {
        let mut g = game();
        let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
        let land = card(&mut g, Zone::Battlefield, CardType::Land);
        let chosen = if two {
            vec![creature, land]
        } else {
            vec![creature]
        };
        let effect = UnlessActionEffect::new(
            vec![Effect::new(SacrificeEffect::you(
                ObjectFilter::default(),
                2,
            ))],
            vec![Effect::new(SacrificeEffect::you_creature(1))],
            PlayerFilter::You,
        );
        let mut dm = Pick {
            ids: chosen.clone(),
            ..Default::default()
        };
        let source = g.new_object_id();
        effect
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(dm.prompts.len(), 1);
        assert_eq!(dm.booleans, 0);
        assert_eq!(g.player(A).unwrap().graveyard.len(), chosen.len());
    }
}
#[test]
fn combined_choose_preserves_tags_and_does_not_prompt_again() {
    let mut g = game();
    let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
    let _ = card(&mut g, Zone::Battlefield, CardType::Land);
    let effect = UnlessActionEffect::new(
        vec![Effect::new(ChooseObjectsEffect::new(
            ObjectFilter::default(),
            2,
            PlayerFilter::You,
            "chosen",
        ))],
        vec![Effect::new(ChooseObjectsEffect::new(
            ObjectFilter::creature(),
            1,
            PlayerFilter::You,
            "chosen",
        ))],
        PlayerFilter::You,
    );
    let mut dm = Pick {
        ids: vec![creature],
        ..Default::default()
    };
    let source = g.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, A).with_decision_maker(&mut dm);
    effect.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(
        ctx.tagged_objects.get("chosen").unwrap()[0].object_id,
        creature
    );
    assert_eq!(dm.prompts.len(), 1);
    assert_eq!(dm.booleans, 0);
    assert_eq!(g.battlefield.len(), 2);
}
#[test]
fn incompatible_actions_keep_explicit_branch_choice() {
    let mut g = game();
    let _ = card(&mut g, Zone::Hand, CardType::Creature);
    let effect = UnlessActionEffect::new(
        vec![Effect::gain_life(2)],
        vec![Effect::new(DiscardEffect::you(1))],
        PlayerFilter::You,
    );
    let mut dm = Pick::default();
    let source = g.new_object_id();
    effect
        .execute(
            &mut g,
            &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(dm.booleans, 1);
    assert!(dm.prompts.is_empty());
    assert_eq!(g.player(A).unwrap().life, 22);
}
#[test]
fn compiled_winternight_stories_uses_combined_selection() {
    let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Winternight Stories",
        "Mana cost: {2}{U}\nType: Sorcery\nDraw three cards. Then you discard two cards unless you discard a creature card.\nHarmonize {4}{U}.",false).unwrap();
    let mut g = game();
    let creature = card(&mut g, Zone::Hand, CardType::Creature);
    let _ = card(&mut g, Zone::Hand, CardType::Land);
    for _ in 0..3 {
        card(&mut g, Zone::Library, CardType::Land);
    }
    let source = g.new_object_id();
    let mut dm = Pick {
        ids: vec![creature],
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new_default(source, A).with_decision_maker(&mut dm);
    for effect in definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        effect.0.execute(&mut g, &mut ctx).unwrap();
    }
    assert_eq!(dm.booleans, 0);
    assert_eq!(dm.prompts.len(), 1);
    assert_eq!(dm.prompts[0].alternatives.len(), 2);
    assert_eq!(g.player(A).unwrap().hand.len(), 4);
}

#[test]
fn combined_choose_exiles_selected_objects_through_shared_continuation() {
    use ironsmith::target::ChooseSpec;
    let mut g = game();
    let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
    let land = card(&mut g, Zone::Battlefield, CardType::Land);
    let tail = Effect::exile(ChooseSpec::tagged("chosen"));
    let effect = UnlessActionEffect::new(
        vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::default(),
                2,
                PlayerFilter::You,
                "chosen",
            )),
            tail.clone(),
        ],
        vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::creature(),
                1,
                PlayerFilter::You,
                "chosen",
            )),
            tail,
        ],
        PlayerFilter::You,
    );
    let mut dm = Pick {
        ids: vec![creature],
        ..Default::default()
    };
    let source = g.new_object_id();
    effect
        .execute(
            &mut g,
            &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(dm.prompts.len(), 1);
    assert_eq!(dm.booleans, 0);
    assert!(!g.battlefield.contains(&creature));
    assert!(g.battlefield.contains(&land));
    assert_eq!(g.exile.len(), 1);
}

#[test]
fn simultaneous_sacrifice_keeps_selection_until_commit() {
    let mut g = game();
    let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
    let land = card(&mut g, Zone::Battlefield, CardType::Land);
    let effect = UnlessActionEffect::new(
        vec![Effect::new(SacrificeEffect::player(
            ObjectFilter::default(),
            2,
            PlayerFilter::IteratedPlayer,
        ))],
        vec![Effect::new(SacrificeEffect::player(
            ObjectFilter::creature(),
            1,
            PlayerFilter::IteratedPlayer,
        ))],
        PlayerFilter::IteratedPlayer,
    );
    let mut dm = Pick {
        ids: vec![creature],
        ..Default::default()
    };
    let source = g.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, A).with_decision_maker(&mut dm);
    ctx.iteration.iterated_player = Some(A);
    let proposal = effect
        .prepare_simultaneous_player_action(&g, &mut ctx)
        .unwrap();
    assert!(g.battlefield.contains(&creature));
    proposal.commit(&mut g, &mut ctx).unwrap();
    assert_eq!(dm.prompts.len(), 1);
    assert_eq!(dm.booleans, 0);
    assert!(!g.battlefield.contains(&creature));
    assert!(g.battlefield.contains(&land));
}

#[test]
fn disjoint_actions_can_share_a_choice_across_zones() {
    for discard_card in [false, true] {
        let mut g = game();
        let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
        let hand = card(&mut g, Zone::Hand, CardType::Land);
        let effect = UnlessActionEffect::new(
            vec![Effect::new(SacrificeEffect::you_creature(1))],
            vec![Effect::new(DiscardEffect::you(1))],
            PlayerFilter::You,
        );
        let chosen = if discard_card { hand } else { creature };
        let mut dm = Pick {
            ids: vec![chosen],
            ..Default::default()
        };
        let source = g.new_object_id();
        effect
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(dm.booleans, 0);
        assert_eq!(dm.prompts.len(), 1);
        assert_eq!(g.battlefield.contains(&creature), discard_card);
        assert_eq!(g.player(A).unwrap().hand.contains(&hand), !discard_card);
    }
}

#[test]
fn overlapping_selections_with_different_consequences_keep_branch_choice() {
    let mut g = game();
    let creature = card(&mut g, Zone::Battlefield, CardType::Creature);
    let _ = card(&mut g, Zone::Battlefield, CardType::Land);
    let effect = UnlessActionEffect::new(
        vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::default(),
                1,
                PlayerFilter::You,
                "chosen",
            )),
            Effect::gain_life(1),
        ],
        vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::creature(),
                1,
                PlayerFilter::You,
                "chosen",
            )),
            Effect::gain_life(2),
        ],
        PlayerFilter::You,
    );
    let mut dm = Pick {
        ids: vec![creature],
        ..Default::default()
    };
    let source = g.new_object_id();
    effect
        .execute(
            &mut g,
            &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(dm.booleans, 1);
    assert_eq!(g.player(A).unwrap().life, 21);
}

#[test]
fn a_mixed_group_must_satisfy_one_complete_alternative() {
    let mut g = game();
    let c1 = card(&mut g, Zone::Battlefield, CardType::Creature);
    let _ = card(&mut g, Zone::Battlefield, CardType::Creature);
    let l1 = card(&mut g, Zone::Battlefield, CardType::Land);
    let _ = card(&mut g, Zone::Battlefield, CardType::Land);
    let effect = UnlessActionEffect::new(
        vec![Effect::new(ChooseObjectsEffect::new(
            ObjectFilter::creature(),
            2,
            PlayerFilter::You,
            "chosen",
        ))],
        vec![Effect::new(ChooseObjectsEffect::new(
            ObjectFilter::land(),
            2,
            PlayerFilter::You,
            "chosen",
        ))],
        PlayerFilter::You,
    );
    let mut dm = Pick {
        ids: vec![c1, l1],
        ..Default::default()
    };
    let source = g.new_object_id();
    assert!(
        effect
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm)
            )
            .is_err()
    );
    assert_eq!(dm.booleans, 0);
    assert_eq!(dm.prompts.len(), 1);
    assert_eq!(g.battlefield.len(), 4);
}

#[test]
fn private_hand_knowledge_does_not_change_the_combined_prompt_stream() {
    let mut peer = game();
    let ids = (0..3)
        .map(|slot| {
            peer.create_hidden_card_placeholder(A, Zone::Hand, slot, format!("commitment-{slot}"))
        })
        .collect::<Vec<_>>();
    let mut owner = peer.clone();
    let land = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Private land",
        "Type: Land",
        false,
    )
    .unwrap();
    for id in &ids {
        owner
            .reveal_hidden_card_with_definition(*id, &land)
            .unwrap();
    }
    for mut g in [owner, peer] {
        let source = g.new_object_id();
        let mut dm = Pick {
            pending: true,
            ..Default::default()
        };
        discard()
            .execute(
                &mut g,
                &mut ExecutionContext::new_default(source, A).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(dm.booleans, 0);
        assert_eq!(dm.prompts.len(), 1);
        assert_eq!(dm.prompts[0].alternatives.len(), 2);
        assert_eq!(g.player(A).unwrap().hand.len(), 3);
    }
}
