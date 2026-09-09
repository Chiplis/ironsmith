use super::*;
const TEXT: &str = "This creature has defender as long as it's a Wall.\n{1}: This creature becomes the creature type of your choice until end of turn.";
struct ChooseType(Subtype);
impl crate::decision::DecisionMaker for ChooseType {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![
            ctx.options
                .iter()
                .find(|option| option.description == self.0.to_string())
                .expect("chosen creature type must be offered")
                .index,
        ]
    }
}
#[test]
fn self_subtype_defender_tracks_type_choice_and_expiry() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mistform Wall")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Illusion, Subtype::Wall])
        .power_toughness(crate::card::PowerToughness::fixed(1, 5))
        .parse_text(TEXT)
        .unwrap();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap();
    for choices in [
        vec![Subtype::Elf],
        vec![Subtype::Wall],
        vec![Subtype::Elf, Subtype::Wall],
        vec![Subtype::Wall, Subtype::Elf],
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let vanilla = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Wall")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wall])
            .build();
        let other = game.create_object_from_card(&vanilla, bob, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Defender
        ));
        assert!(!game.current_has_static_ability_id(
            other,
            crate::static_abilities::StaticAbilityId::Defender
        ));
        for chosen in choices {
            let mut dm = ChooseType(chosen);
            let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm)
                .with_targets(vec![crate::ResolvedTarget::Object(other)]);
            for effect in &activated.effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.calculated_subtypes(source), vec![chosen]);
            assert_eq!(
                game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::Defender
                ),
                chosen == Subtype::Wall
            );
            assert_eq!(game.calculated_subtypes(other), vec![Subtype::Wall]);
            assert!(!game.current_has_static_ability_id(
                other,
                crate::static_abilities::StaticAbilityId::Defender
            ));
        }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert!(game.current_has_subtype(source, Subtype::Wall));
        assert!(game.current_has_subtype(source, Subtype::Illusion));
        assert!(game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Defender
        ));
    }
}
#[test]
fn self_subtype_defender_renders_self_condition() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mistform Wall")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Illusion, Subtype::Wall])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
