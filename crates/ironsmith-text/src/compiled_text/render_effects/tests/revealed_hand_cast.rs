use super::*;

struct AcceptCast(bool);
impl crate::decision::DecisionMaker for AcceptCast {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.0
    }
}

#[test]
fn revealed_hand_cast_is_optional_free_and_limited_to_the_revealed_opponents_spells() {
    let oracle = "When this creature enters, target opponent reveals their hand. You may cast an instant or sorcery spell from among those cards without paying its mana cost.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Revealed Hand Cast Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .unwrap();
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("entry trigger");
    };
    for accept in [false, true] {
        for has_spell in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let carol = game.players[2].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let spell =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Revealed Spell")
                    .card_types(vec![CardType::Instant])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(7),
                    ]))
                    .parse_text("You gain 3 life.")
                    .unwrap();
            let creature =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Ineligible Creature")
                    .card_types(vec![CardType::Creature])
                    .build();
            let own_spell = game.create_object_from_definition(&spell, alice, Zone::Hand);
            let other_spell = game.create_object_from_definition(&spell, carol, Zone::Hand);
            let ineligible = game.create_object_from_card(&creature, bob, Zone::Hand);
            if has_spell {
                game.create_object_from_definition(&spell, bob, Zone::Hand);
            }
            let mut dm = AcceptCast(accept);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
                .with_decision_maker(&mut dm);
            let [reveal, may_cast] = triggered.effects.segments[0].default_effects.as_slice()
            else {
                panic!("reveal/cast pair");
            };
            crate::effects::execute_effect(&mut game, reveal, &mut ctx).unwrap();
            let unrevealed = game.create_object_from_definition(&spell, bob, Zone::Hand);
            crate::effects::execute_effect(&mut game, may_cast, &mut ctx).unwrap();
            assert_eq!(
                game.stack.len(),
                usize::from(accept && has_spell),
                "accept={accept}, spell={has_spell}"
            );
            for id in [own_spell, other_spell, ineligible, unrevealed] {
                assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
            }
            if let Some(entry) = game.stack.first() {
                assert_eq!(entry.controller, alice);
                assert_eq!(game.object(entry.object_id).unwrap().owner, bob);
            }
        }
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [oracle]
    );
    let [look, may_root] = triggered.effects.segments[0].default_effects.as_slice() else {
        unreachable!()
    };
    let mut may = may_root
        .downcast_ref::<crate::effects::MayEffect>()
        .unwrap()
        .clone();
    let mut cast = may.effects[1]
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .unwrap()
        .clone();
    cast.tag = TagKey::from("unrelated_card");
    may.effects[1] = Effect::new(cast);
    assert!(
        super::super::structural_bundles::describe_revealed_hand_then_optional_free_cast(
            look,
            &Effect::new(may)
        )
        .is_none()
    );
}
