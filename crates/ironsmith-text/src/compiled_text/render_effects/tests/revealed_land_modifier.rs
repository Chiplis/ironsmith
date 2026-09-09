use super::*;
const TEXT: &str = "{1}: Look at the top card of your library.\n{2}: Reveal the top card of your library. If it's a land card, this creature gets +1/+0 and gains flying until end of turn. Activate only once each turn.";
#[derive(Default)]
struct Views(Vec<(crate::ids::PlayerId, bool, Vec<crate::ids::ObjectId>)>);
impl crate::decision::DecisionMaker for Views {
    fn view_cards(
        &mut self,
        _: &crate::game_state::GameState,
        viewer: crate::ids::PlayerId,
        cards: &[crate::ids::ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.0.push((viewer, ctx.public, cards.to_vec()));
    }
}
#[test]
fn revealed_land_modifier_checks_top_card_and_expires() {
    use crate::static_abilities::StaticAbilityId;
    for (name, power, toughness, dp, dt, keyword, ability_id) in [
        (
            "Callous Deceiver",
            1,
            3,
            1,
            0,
            "flying",
            StaticAbilityId::Flying,
        ),
        (
            "Feral Deceiver",
            3,
            2,
            2,
            2,
            "trample",
            StaticAbilityId::Trample,
        ),
        (
            "Brutal Deceiver",
            2,
            2,
            1,
            0,
            "first strike",
            StaticAbilityId::FirstStrike,
        ),
    ] {
        let text = TEXT
            .replace("+1/+0", &format!("+{dp}/+{dt}"))
            .replace("flying", keyword);
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(crate::card::PowerToughness::fixed(power, toughness))
            .parse_text(&text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            text
        );
        let abilities: Vec<_> = definition
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                crate::ability::AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(abilities.len(), 2);
        assert!(format!("{:?}", abilities[1].timing).contains("OncePerTurn"));
        for kind in [Some(CardType::Land), Some(CardType::Sorcery), None] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let top = kind.map(|kind| {
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Top card")
                    .card_types(vec![kind])
                    .build();
                game.create_object_from_card(&card, alice, Zone::Library)
            });
            let mut views = Views::default();
            {
                let mut ctx = crate::effects::EffectContext::new(source, alice, &mut views);
                for effect in &abilities[0].effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            if let Some(top) = top {
                assert!(views.0.iter().any(|v| v == &(alice, false, vec![top])));
            }
            assert!(views.0.iter().all(|v| v.0 == alice && !v.1));
            views.0.clear();
            {
                let mut ctx = crate::effects::EffectContext::new(source, alice, &mut views);
                for effect in &abilities[1].effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            if let Some(top) = top {
                for player in [alice, bob] {
                    assert!(views.0.iter().any(|v| v == &(player, true, vec![top])));
                }
                assert_eq!(game.object(top).unwrap().zone, Zone::Library);
            }
            let land = kind == Some(CardType::Land);
            assert_eq!(
                game.calculated_power(source),
                Some(power + if land { dp } else { 0 })
            );
            assert_eq!(
                game.calculated_toughness(source),
                Some(toughness + if land { dt } else { 0 })
            );
            assert_eq!(game.current_has_static_ability_id(source, ability_id), land);
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            assert_eq!(game.calculated_power(source), Some(power));
            assert!(!game.current_has_static_ability_id(source, ability_id));
        }
    }
}
#[test]
fn revealed_land_modifier_renders_complete_program() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Callous Deceiver")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
