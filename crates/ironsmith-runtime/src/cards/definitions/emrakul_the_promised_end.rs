//! Emrakul, the Promised End card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;

/// Emrakul, the Promised End - Legendary Creature — Eldrazi
/// {13}
/// 13/13
/// This spell costs {1} less to cast for each card type among cards in your graveyard.
/// When you cast this spell, gain control of target opponent during that player's next turn.
/// After that turn, that player takes an extra turn.
/// Flying, trample, protection from instants.
pub fn emrakul_the_promised_end() -> CardDefinition {
    let text = "Mana cost: {13}\n\
Type: Legendary Creature — Eldrazi\n\
Power/Toughness: 13/13\n\
This spell costs {1} less to cast for each card type among cards in your graveyard.\n\
When you cast this spell, gain control of target opponent during that player's next turn. After that turn, that player takes an extra turn.\n\
Flying, trample, protection from instants.";

    CardDefinitionBuilder::new(CardId::new(), "Emrakul, the Promised End")
        .parse_text(text)
        .expect("Emrakul text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::{PlayerControlDuration, PlayerControlStart};
    use crate::zone::Zone;

    #[test]
    fn test_emrakul_compiled_text_preserves_cost_reduction_and_after_that_turn() {
        let rendered = crate::compiled_text::compiled_lines(&emrakul_the_promised_end())
            .join("\n")
            .to_ascii_lowercase();

        assert!(rendered.contains(
            "this spell costs {1} less to cast for each card type among cards in your graveyard"
        ));
        assert!(rendered.contains("after that turn"));
        assert!(rendered.contains("takes an extra turn"));
    }

    #[test]
    fn test_emrakul_uses_stack_cost_reduction_and_delayed_extra_turn_effect() {
        let def = emrakul_the_promised_end();

        let cost_ability = &def.abilities[0];
        assert!(
            cost_ability.functional_zones.contains(&Zone::Stack),
            "spell cost reduction should function while casting on the stack"
        );
        assert!(
            !cost_ability.functional_zones.contains(&Zone::Battlefield),
            "spell cost reduction should not be battlefield-only metadata"
        );

        let triggered = &def.abilities[1];
        assert_eq!(triggered.functional_zones, vec![Zone::Stack]);

        let AbilityKind::Triggered(triggered_ability) = &triggered.kind else {
            panic!("expected Emrakul's cast ability to lower as a triggered ability");
        };

        assert_eq!(triggered_ability.effects.len(), 2);

        let control = triggered_ability.effects[0]
            .downcast_ref::<crate::effects::ControlPlayerEffect>()
            .expect("first effect should control the chosen opponent");
        assert_eq!(control.start, PlayerControlStart::NextTurn);
        assert_eq!(control.duration, PlayerControlDuration::UntilEndOfTurn);

        assert!(
            triggered_ability.effects[1]
                .downcast_ref::<crate::effects::ExtraTurnAfterNextTurnEffect>()
                .is_some(),
            "second effect should schedule the opponent's extra turn after their next turn"
        );
    }
}
