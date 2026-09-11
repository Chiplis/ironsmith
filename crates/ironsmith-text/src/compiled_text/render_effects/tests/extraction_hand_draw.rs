use super::*;

struct SelectAll;
impl crate::decision::DecisionMaker for SelectAll {
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect()
    }
}

#[test]
fn extraction_draw_counts_only_successful_hand_exiles() {
    check_extraction_hand_draw(false);
}

#[test]
fn permanent_extraction_draw_counts_only_successful_hand_exiles() {
    check_extraction_hand_draw(true);
}

fn check_extraction_hand_draw(permanent: bool) {
    let spell_oracle = "Counter target instant or sorcery spell. Search its controller's graveyard, hand, and library for any number of cards with the same name as that spell and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.";
    let permanent_oracle = "Exile target creature or planeswalker. Search its controller's graveyard, hand, and library for any number of cards with the same name as that permanent and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.";
    let oracle = if permanent {
        permanent_oracle
    } else {
        spell_oracle
    };
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Extraction Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(oracle)
            .unwrap();
    fn has_outcome_tag(effect: &Effect) -> bool {
        if effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .is_some_and(|tagged| tagged.outcome_only)
        {
            return true;
        }
        let mut found = false;
        effect.0.visit_child_effects(&mut |child| {
            found |= has_outcome_tag(child);
        });
        found
    }
    assert!(
        definition
            .spell_effect
            .as_ref()
            .unwrap()
            .segments
            .iter()
            .flat_map(|segment| &segment.default_effects)
            .any(has_outcome_tag),
        "the compiled artifact must retain affected-result tagging"
    );
    for hand_count in [0, 1, 2] {
        for (redirected, redirect_all) in [(false, false), (true, false), (true, true)] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let matching =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Matching Spell")
                    .card_types(vec![if permanent {
                        CardType::Creature
                    } else {
                        CardType::Instant
                    }])
                    .build();
            let spell = game.create_object_from_card(
                &matching,
                bob,
                if permanent {
                    Zone::Battlefield
                } else {
                    Zone::Stack
                },
            );
            if !permanent {
                game.stack
                    .push(crate::game_state::StackEntry::new(spell, bob));
            }
            let mut hand_ids = Vec::new();
            for _ in 0..hand_count {
                hand_ids.push(game.create_object_from_card(&matching, bob, Zone::Hand));
            }
            for zone in [Zone::Graveyard, Zone::Library] {
                game.create_object_from_card(&matching, bob, zone);
            }
            let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Spell")
                .card_types(vec![CardType::Instant])
                .build();
            for _ in 0..6 {
                game.create_object_from_card(&other, bob, Zone::Library);
            }
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let mut decision = SelectAll;
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_decision_maker(&mut decision)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(spell)]);
            if redirected && let Some(id) = hand_ids.first() {
                let replacement = crate::effects::RegisterFutureZoneReplacementEffect::new(
                    if redirect_all {
                        ObjectFilter::default().named("Matching Spell")
                    } else {
                        ObjectFilter::specific(*id)
                    },
                    if redirect_all { None } else { Some(Zone::Hand) },
                    Some(Zone::Exile),
                    Zone::Graveyard,
                    if redirect_all {
                        crate::effects::ReplacementApplyMode::UntilEndOfTurn
                    } else {
                        crate::effects::ReplacementApplyMode::OneShot
                    },
                );
                crate::effects::execute_effect(&mut game, &Effect::new(replacement), &mut ctx)
                    .unwrap();
            }
            ctx.snapshot_targets(&game);
            for segment in &definition.spell_effect.as_ref().unwrap().segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            assert_eq!(
                game.player(bob).unwrap().hand.len(),
                if redirect_all {
                    0
                } else {
                    hand_count - usize::from(redirected && hand_count > 0)
                },
                "hand={hand_count}, redirected={redirected}, all={redirect_all}"
            );
            assert!(game.player(alice).unwrap().hand.is_empty());
        }
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [oracle]
    );
}
