use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::mana::ManaSymbol;
use ironsmith_compiler::static_abilities::StaticAbilityPayload;
use ironsmith_compiler::types::CardType;

#[test]
fn typed_attack_tax_preserves_alternative_mana_filtered_attackers_and_bound_x() {
    for (name, text) in [
        (
            "Norn's Annex",
            "Creatures can't attack you or planeswalkers you control unless their controller pays {W/P} for each of those creatures.",
        ),
        (
            "Elephant Grass",
            "Nonblack creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
        ),
        (
            "Sphere of Safety",
            "Creatures can't attack you or planeswalkers you control unless their controller pays {X} for each of those creatures, where X is the number of enchantments you control.",
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .unwrap();
        assert_eq!(definition.abilities.len(), 1);
        let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
            panic!("{definition:#?}");
        };
        let StaticAbilityPayload::AttackCost {
            attackers,
            covers_planeswalkers,
            cost,
            ..
        } = &ability.payload
        else {
            panic!("{ability:#?}");
        };
        assert_eq!(*covers_planeswalkers, name != "Elephant Grass");
        assert_eq!(attackers.card_types, vec![CardType::Creature]);
        match name {
            "Norn's Annex" => assert_eq!(
                cost.mana_cost().unwrap().pips(),
                &[vec![ManaSymbol::White, ManaSymbol::Life(2)]]
            ),
            "Elephant Grass" => assert_eq!(
                attackers.excluded_colors,
                ironsmith_compiler::color::ColorSet::BLACK
            ),
            _ => {
                let dynamic = cost.dynamic_mana_cost().unwrap();
                assert!(dynamic.base.has_x());
                let ironsmith_compiler::effect::Value::Count(filter) =
                    dynamic.x_value.as_ref().unwrap().unhinted()
                else {
                    panic!("{dynamic:#?}");
                };
                assert_eq!(filter.card_types, vec![CardType::Enchantment]);
                assert_eq!(
                    filter.controller,
                    Some(ironsmith_compiler::target::PlayerFilter::You)
                );
            }
        }
    }
}
