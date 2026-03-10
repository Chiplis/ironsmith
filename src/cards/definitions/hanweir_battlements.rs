//! Hanweir Battlements card definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Hanweir Battlements
/// Land
/// {T}: Add {C}.
/// {R}, {T}: Target creature gains haste until end of turn.
/// {3}{R}{R}, {T}: If you both own and control this land and a creature named
/// Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township.
pub fn hanweir_battlements() -> CardDefinition {
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Hanweir Battlements")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}.\n{R}, {T}: Target creature gains haste until end of turn.")
        .expect("Card text should be supported");

    definition.abilities.push(Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(3)],
                    vec![ManaSymbol::Red],
                    vec![ManaSymbol::Red],
                ])),
                Cost::tap(),
            ]),
            effects: vec![Effect::hanweir_battlements_meld()],
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
        text: Some(
            "{3}{R}{R}, {T}: If you both own and control this land and a creature named \
                 Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township."
                .to_string(),
        ),
    });
    definition.card.oracle_text =
        "{T}: Add {C}.\n{R}, {T}: Target creature gains haste until end of turn.\n{3}{R}{R}, {T}: If you both own and control this land and a creature named Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township.".to_string();

    definition
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::AutoPassDecisionMaker;
    use crate::events::EventKind;
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::target::ChooseSpec;
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(object);
        id
    }

    #[test]
    fn test_hanweir_battlements_basic_properties() {
        let def = hanweir_battlements();
        assert_eq!(def.name(), "Hanweir Battlements");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.abilities.len(), 3);
    }

    #[test]
    fn test_hanweir_battlements_first_ability_is_colorless_mana() {
        let def = hanweir_battlements();
        let ability = &def.abilities[0];
        assert!(ability.is_mana_ability());

        let AbilityKind::Activated(mana_ability) = &ability.kind else {
            panic!("Expected mana ability");
        };
        assert_eq!(mana_ability.mana_symbols(), &[ManaSymbol::Colorless]);
        assert!(mana_ability.has_tap_cost());
    }

    #[test]
    fn test_hanweir_battlements_second_ability_targets_creature() {
        let def = hanweir_battlements();
        let ability = &def.abilities[1];
        assert!(!ability.is_mana_ability());

        let AbilityKind::Activated(activated) = &ability.kind else {
            panic!("Expected activated ability");
        };
        assert_eq!(
            activated.mana_cost.mana_cost(),
            Some(&ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        );
        assert!(activated.has_tap_cost());
        assert_eq!(activated.choices.len(), 1);

        let target_spec = match &activated.choices[0] {
            ChooseSpec::Target(inner) => inner.as_ref(),
            other => other,
        };
        let ChooseSpec::Object(filter) = target_spec else {
            panic!("Expected creature target");
        };
        assert!(filter.card_types.contains(&CardType::Creature));
    }

    #[test]
    fn test_hanweir_battlements_second_ability_grants_haste() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let battlements_id =
            game.create_object_from_definition(&hanweir_battlements(), alice, Zone::Battlefield);
        let creature_id = create_creature(&mut game, "Soldier", alice);

        let def = hanweir_battlements();
        let effects = match &def.abilities[1].kind {
            AbilityKind::Activated(activated) => &activated.effects,
            _ => panic!("Expected activated ability"),
        };

        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(battlements_id, alice, &mut dm);
        ctx.targets = vec![ResolvedTarget::Object(creature_id)];

        for effect in effects {
            execute_effect(&mut game, effect, &mut ctx).expect("haste effect should resolve");
        }

        let chars = game
            .calculated_characteristics(creature_id)
            .expect("Should calculate characteristics");
        assert!(
            chars.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.has_haste())
            }),
            "Target creature should gain haste"
        );
    }

    #[test]
    fn test_hanweir_battlements_third_ability_has_meld_cost() {
        let def = hanweir_battlements();
        let ability = &def.abilities[2];
        assert!(!ability.is_mana_ability());

        let AbilityKind::Activated(activated) = &ability.kind else {
            panic!("Expected activated ability");
        };
        assert_eq!(
            activated.mana_cost.mana_cost(),
            Some(&ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Red],
            ]))
        );
        assert!(activated.has_tap_cost());
        assert_eq!(activated.effects.len(), 1);
    }

    #[test]
    fn test_hanweir_battlements_third_ability_melds_with_garrison() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let battlements_id =
            game.create_object_from_definition(&hanweir_battlements(), alice, Zone::Battlefield);
        let garrison_id = game.create_object_from_definition(
            &crate::cards::builtin_registry()
                .get("Hanweir Garrison")
                .cloned()
                .expect("Hanweir Garrison should be in builtin registry"),
            alice,
            Zone::Battlefield,
        );

        let def = hanweir_battlements();
        let effect = match &def.abilities[2].kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .first()
                .expect("meld ability should have effect"),
            _ => panic!("Expected activated ability"),
        };

        let mut ctx = ExecutionContext::new_default(battlements_id, alice);
        execute_effect(&mut game, effect, &mut ctx).expect("meld effect should resolve");

        assert!(
            game.battlefield_has("Hanweir, the Writhing Township"),
            "Meld result should be on battlefield"
        );
        assert!(
            !game.battlefield.contains(&battlements_id),
            "Battlements should leave the battlefield"
        );
        assert!(
            !game.battlefield.contains(&garrison_id),
            "Garrison should leave the battlefield"
        );
        let pending = game.take_pending_trigger_events();
        assert!(
            pending
                .iter()
                .any(|event| event.kind() == EventKind::EnterBattlefield),
            "Meld result should queue an EnterBattlefield event"
        );
    }

    #[test]
    fn test_hanweir_battlements_oracle_text_includes_meld_ability() {
        let def = hanweir_battlements();
        assert!(def.card.oracle_text.contains("Target creature gains haste"));
        assert!(def.card.oracle_text.contains("Hanweir Garrison"));
        assert!(
            def.card
                .oracle_text
                .contains("Hanweir, the Writhing Township")
        );
    }
}
