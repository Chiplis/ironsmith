//! Snapcaster Mage card definition.

use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Snapcaster Mage - {1}{U}
/// Creature — Human Wizard (2/1)
/// Flash
/// When Snapcaster Mage enters the battlefield, target instant or sorcery card
/// in your graveyard gains flashback until end of turn.
pub fn snapcaster_mage() -> CardDefinition {
    let mut def = CardDefinitionBuilder::new(CardId::new(), "Snapcaster Mage")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("Flash")
        .expect("Card text should be supported");

    let target = ChooseSpec::Object(ObjectFilter {
        zone: Some(Zone::Graveyard),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        owner: Some(PlayerFilter::You),
        ..Default::default()
    });

    def.abilities.push(Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_enters_battlefield(),
            effects: vec![Effect::grant_flashback_until_eot_unified(target)],
            choices: vec![],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "When Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost."
                .to_string(),
        ),
    });

    def.card.oracle_text = format!(
        "{}\n{}",
        def.card.oracle_text,
        "When Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost."
    );

    def
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_snapcaster_mage() {
        let def = snapcaster_mage();
        assert_eq!(def.name(), "Snapcaster Mage");

        // Check Flash
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flash()
            } else {
                false
            }
        }));

        // Check ETB trigger (now using Trigger struct)
        assert!(def.abilities.iter().any(|a| matches!(
            &a.kind,
            AbilityKind::Triggered(t) if t.trigger.display().contains("enters")
        )));
    }

    #[test]
    fn test_replay_snapcaster_mage_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Snapcaster Mage
                "0", // Tap Island 1
                "0", // Tap Island 2
                "0", // Target Lightning Bolt in graveyard (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Snapcaster Mage"])
                .p1_battlefield(vec!["Island", "Island"])
                .p1_graveyard(vec!["Lightning Bolt"]),
        );

        // Snapcaster Mage should be on the battlefield
        assert!(
            game.battlefield_has("Snapcaster Mage"),
            "Snapcaster Mage should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let snapcaster_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Snapcaster Mage" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(snapcaster_id) = snapcaster_id {
            assert_eq!(
                game.calculated_power(snapcaster_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(snapcaster_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Snapcaster Mage on battlefield");
        }
    }
}
