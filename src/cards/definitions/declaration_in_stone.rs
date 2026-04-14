//! Declaration in Stone card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Declaration in Stone - {1}{W}
/// Sorcery
/// Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.
pub fn declaration_in_stone() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Declaration in Stone")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::grizzly_bears;
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::filter::{ObjectRef, TaggedOpbjectRelation};
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn named_bear_token(name: &str) -> CardDefinition {
        CardDefinition::new(
            CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Bear])
                .power_toughness(PowerToughness::fixed(2, 2))
                .token()
                .build(),
        )
    }

    #[test]
    fn declaration_in_stone_compiles_same_name_exile_and_nontoken_investigate_followup() {
        let def = declaration_in_stone();
        let effects = def.spell_effect.expect("spell effect");
        let investigate = effects
            .iter()
            .find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::InvestigateEffect>()
                    .cloned()
            })
            .expect("should include investigate effect");

        assert_eq!(
            investigate.count,
            crate::effect::Value::Count(
                ObjectFilter::creature()
                    .nontoken()
                    .in_zone(Zone::Exile)
                    .match_tagged(
                        crate::cards::builders::TagKey::from("__sentence_helper_exiled_l0_s0_e0"),
                        TaggedOpbjectRelation::IsTaggedObject,
                    )
            ),
            "investigate should count only tagged nontoken creatures exiled this way"
        );
        assert_eq!(
            investigate.player,
            PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::cards::builders::TagKey::from("__sentence_helper_exiled_l0_s0_e0")
            )),
            "investigate should be performed by the exiled creature's controller"
        );

        let rendered = crate::compiled_text::oracle_like_lines(&declaration_in_stone()).join(" ");
        assert!(
            rendered
                .contains("That player investigates for each nontoken creature exiled this way"),
            "expected oracle-like investigate follow-up wording, got {rendered}"
        );
    }

    #[test]
    fn declaration_in_stone_exiles_same_name_creatures_and_only_counts_nontokens_for_investigate() {
        let declaration = declaration_in_stone();
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);

        let target_bear =
            game.create_object_from_definition(&grizzly_bears(), bob, Zone::Battlefield);
        let bob_other_bear =
            game.create_object_from_definition(&grizzly_bears(), bob, Zone::Battlefield);
        let bob_token_bear = game.create_object_from_definition(
            &named_bear_token("Grizzly Bears"),
            bob,
            Zone::Battlefield,
        );
        let alice_bear =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target_bear)]);
        ctx.snapshot_targets(&game);

        for effect in declaration.spell_effect.as_ref().expect("spell effects") {
            execute_effect(&mut game, effect, &mut ctx)
                .expect("declaration in stone effect should resolve");
        }

        let exiled_bears = game
            .exile
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Grizzly Bears")
            })
            .count();
        assert_eq!(
            exiled_bears, 3,
            "Bob's target, same-name nontoken, and same-name token should all be exiled"
        );
        assert!(
            game.object(alice_bear)
                .is_some_and(|obj| obj.zone == Zone::Battlefield),
            "same-name creatures controlled by other players should remain on the battlefield"
        );
        assert!(
            game.object(bob_other_bear).is_none(),
            "Bob's matching nontoken should be exiled"
        );
        assert!(
            game.object(bob_token_bear).is_none(),
            "Bob's matching token should be exiled"
        );

        let bob_clues = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.controller == bob && obj.name == "Clue")
            })
            .count();
        let alice_clues = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.controller == alice && obj.name == "Clue")
            })
            .count();
        assert_eq!(
            bob_clues, 2,
            "Bob should investigate only for the two nontoken creatures exiled this way"
        );
        assert_eq!(
            alice_clues, 0,
            "the caster should not receive Clues from Declaration in Stone"
        );
    }
}
