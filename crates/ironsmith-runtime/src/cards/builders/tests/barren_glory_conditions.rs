#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn upkeep_intervening_if(definition: &CardDefinition) -> &crate::effect::Condition {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.intervening_if.as_ref(),
            _ => None,
        })
        .expect("expected an upkeep trigger with an intervening-if condition")
}

#[test]
fn independent_positive_control_and_hand_conditions_lower_separately() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Positive condition")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, if you control an artifact and have a card in hand, draw a card.",
        )
        .expect("the positive compound condition should parse");

    let crate::effect::Condition::And(left, right) = upkeep_intervening_if(&definition) else {
        panic!(
            "expected two independently lowered conditions, got {:#?}",
            upkeep_intervening_if(&definition)
        );
    };
    assert!(matches!(
        left.as_ref(),
        crate::effect::Condition::PlayerControls {
            player: PlayerFilter::You,
            filter,
        } if filter.card_types == [CardType::Artifact]
    ));
    assert_eq!(
        right.as_ref(),
        &crate::effect::Condition::PlayerCardsInHandOrMore {
            player: PlayerFilter::You,
            count: 1,
        }
    );
}

#[test]
fn barren_glory_lowers_both_authored_negatives_and_renders_exactly() {
    let oracle = "At the beginning of your upkeep, if you control no permanents other than this enchantment and have no cards in hand, you win the game.";
    let definition = parse_oracle_card_definition("Barren Glory");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let crate::effect::Condition::And(left, right) = upkeep_intervening_if(&definition) else {
        panic!(
            "expected two independently lowered negative conditions, got {:#?}",
            upkeep_intervening_if(&definition)
        );
    };
    let crate::effect::Condition::Not(controlled) = left.as_ref() else {
        panic!("the permanent-control condition must remain negated: {left:#?}");
    };
    let crate::effect::Condition::PlayerControls { player, filter } = controlled.as_ref() else {
        panic!("expected a negated player-control condition: {controlled:#?}");
    };

    assert_eq!(player, &PlayerFilter::You);
    assert!(filter.other, "the source permanent must be excluded");
    assert!(filter.has_all_permanent_card_types());
    assert_eq!(
        filter.source_surface,
        Some(SourceReferenceSurface::ThisPermanentType(
            "this enchantment".to_string()
        ))
    );
    assert_eq!(
        right.as_ref(),
        &crate::effect::Condition::Not(Box::new(crate::effect::Condition::CardsInHandOrMore(1)))
    );
}
