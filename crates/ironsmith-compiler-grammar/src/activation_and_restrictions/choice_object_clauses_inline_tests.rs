use super::super::super::util::tokenize_line;
use super::*;
use crate::effect::Restriction;
use crate::zone::Zone;

#[test]
fn parse_negated_object_restriction_clause_supports_attack_or_block_alone() {
    let tokens = tokenize_line("This creature can't attack or block alone.", 0);

    let parsed = parse_negated_object_restriction_clause(&tokens)
        .expect("parse attack-or-block-alone restriction")
        .expect("expected restriction");

    assert!(matches!(
        parsed.restriction,
        Restriction::AttackOrBlockAlone(_)
    ));
}

#[test]
fn normalized_named_source_restriction_targets_the_source() {
    let tokens = tokenize_line("This can't attack.", 0);

    let parsed = parse_negated_object_restriction_clause(&tokens)
        .expect("parse source attack restriction")
        .expect("expected restriction");

    let Restriction::Attack(filter) = parsed.restriction else {
        panic!("expected an attack restriction");
    };
    assert_eq!(filter, ObjectFilter::source());
}

#[test]
fn parse_negated_object_restriction_clause_supports_activated_abilities_of_that_permanent() {
    let tokens = tokenize_line(
        "Activated abilities of that permanent can't be activated.",
        0,
    );

    let parsed = parse_negated_object_restriction_clause(&tokens)
        .expect("parse activated-abilities restriction")
        .expect("expected restriction");

    assert!(matches!(
        parsed.restriction,
        Restriction::ActivateAbilitiesOf(_)
    ));
}

#[test]
fn parse_you_choose_objects_clause_supports_bare_card_from_it() {
    let tokens = tokenize_line("You choose a card from it.", 0);

    let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse choose-a-card-from-it clause")
        .expect("expected choose clause");

    assert_eq!(chooser, PlayerAst::You);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()),
        "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
    );
    assert!(
        filter.controller.is_none(),
        "expected no controller pin, got {filter:?}"
    );
    assert!(
        filter.owner.is_none(),
        "expected no owner pin, got {filter:?}"
    );
}

#[test]
fn parse_you_choose_objects_clause_supports_card_from_it_with_filter_tail() {
    let tokens = tokenize_line("You choose a card from it with mana value 4 or greater.", 0);

    let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse choose-a-card-from-it-with-filter-tail clause")
        .expect("expected choose clause");

    assert_eq!(chooser, PlayerAst::You);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()),
        "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
    );
    assert!(
        filter.controller.is_none(),
        "expected no controller pin, got {filter:?}"
    );
    assert!(
        filter.owner.is_none(),
        "expected no owner pin, got {filter:?}"
    );
}

#[test]
fn parse_you_choose_objects_clause_supports_opponent_graveyard_or_hand() {
    let tokens = tokenize_line(
        "You choose a nonland card from that player's graveyard or hand.",
        0,
    );

    let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse choose from graveyard-or-hand clause")
        .expect("expected choose clause");

    assert_eq!(chooser, PlayerAst::You);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(filter.zone, None);
    assert_eq!(filter.controller, None);
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    assert!(filter.excluded_card_types.contains(&CardType::Land));
    assert_eq!(filter.any_of.len(), 2);
    assert!(
        filter
            .any_of
            .iter()
            .any(|arm| arm.zone == Some(Zone::Graveyard))
    );
    assert!(filter.any_of.iter().any(|arm| arm.zone == Some(Zone::Hand)));
}

#[test]
fn parse_you_choose_objects_clause_container_reference_overrides_permanent_default() {
    let tokens = tokenize_line("You choose an artifact or creature card from it.", 0);

    let (_chooser, filter, _count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse choose-artifact-or-creature-card-from-it clause")
        .expect("expected choose clause");

    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(
        filter.controller.is_none(),
        "expected no battlefield controller default, got {filter:?}"
    );
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()),
        "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
    );
}

#[test]
fn parse_you_choose_objects_clause_supports_serial_card_type_union_from_it() {
    let tokens = tokenize_line(
        "You choose an artifact, instant, or sorcery card from it.",
        0,
    );

    let (_chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse serial card-type choice")
        .expect("expected an object choice rather than a card-type declaration");

    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(
        filter.card_types,
        [CardType::Artifact, CardType::Instant, CardType::Sorcery]
    );
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()),
        "the serial choice must remain tied to the revealed hand: {filter:?}"
    );
}

#[test]
fn parse_you_choose_objects_clause_supports_one_of_them() {
    let tokens = tokenize_line("You choose one of them.", 0);

    let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse choose-one-of-them clause")
        .expect("expected choose clause");

    assert_eq!(chooser, PlayerAst::You);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()),
        "expected one-of-them choice to reference the previous object set, got {filter:?}"
    );
    assert_eq!(filter.zone, None);
    assert!(
        filter.controller.is_none() && filter.owner.is_none(),
        "expected referenced choice not to default to your permanent, got {filter:?}"
    );
}

#[test]
fn parse_bare_choose_objects_clause_keeps_implicit_chooser() {
    let tokens = tokenize_line("Choose an artifact.", 0);

    let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
        .expect("parse bare choose-artifact clause")
        .expect("expected choose clause");

    assert_eq!(chooser, PlayerAst::Implicit);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(filter.card_types, vec![CardType::Artifact]);
    assert!(
        filter.controller.is_none(),
        "implicit choose should let lowering bind the controller to the chooser, got {filter:?}"
    );
}

#[test]
fn parse_that_player_chooses_one_of_those_uses_last_object_controller() {
    let tokens = tokenize_line("That player chooses one of those creatures.", 0);

    let (chooser, filter, count) = parse_target_player_choose_objects_clause(&tokens)
        .expect("parse that-player chooses one-of-those clause")
        .expect("expected target-player choose clause");

    assert_eq!(chooser, PlayerAst::ItsController);
    assert_eq!(count, ChoiceCount::exactly(1));
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject),
        "expected one-of-those choice to stay tied to tagged objects, got {filter:?}"
    );
}

#[test]
fn parse_you_choose_player_clause_supports_choose_an_opponent() {
    let tokens = tokenize_line("Choose an opponent.", 0);

    let (chooser, filter, random, exclude_previous_choices) =
        parse_you_choose_player_clause(&tokens)
            .expect("parse choose-an-opponent clause")
            .expect("expected choose-player clause");

    assert_eq!(chooser, PlayerAst::You);
    assert_eq!(filter, PlayerFilter::Opponent);
    assert!(!random);
    assert_eq!(exclude_previous_choices, 0);
}

#[test]
fn parse_choose_card_type_phrase_words_supports_limited_type_lists() {
    let parsed =
        parse_choose_card_type_phrase_words(&["choose", "artifact", "creature", "or", "land"])
            .expect("limited choose-card-type phrase should parse")
            .expect("expected choose-card-type phrase");

    assert_eq!(
        parsed,
        (
            5,
            vec![CardType::Artifact, CardType::Creature, CardType::Land]
        )
    );
}

#[test]
fn parse_choose_card_type_phrase_words_supports_permanent_types() {
    let parsed = parse_choose_card_type_phrase_words(&["choose", "a", "permanent", "type"])
        .expect("permanent-type choice phrase should parse")
        .expect("expected choose-card-type phrase");

    assert_eq!(
        parsed,
        (
            4,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ]
        )
    );
}

#[test]
fn typed_choice_sequences_preserve_resolution_choice_ast() {
    let first = tokenize_line("Target opponent chooses a creature.", 0);
    let second = tokenize_line("Other creatures can't block this turn.", 0);
    let cant_block = parse_target_player_chooses_then_other_cant_block(&first, &second)
        .expect("parse choose-then-cant-block sequence")
        .expect("expected choose-then-cant-block sequence");
    assert!(matches!(
        cant_block.as_slice(),
        [
            EffectAst::ChooseObjects {
                player: PlayerAst::TargetOpponent,
                ..
            },
            _
        ]
    ));

    let library = tokenize_line(
        "Target player chooses a creature and puts it on top of their library.",
        0,
    );
    let put_on_top = parse_sentence_target_player_chooses_then_puts_on_top_of_library(&library)
        .expect("parse choose-then-library sequence")
        .expect("expected choose-then-library sequence");
    assert!(matches!(
        put_on_top.as_slice(),
        [EffectAst::ChooseObjects { .. }, _]
    ));
}

#[test]
fn typed_choice_become_and_battlefield_sequences_build_existing_ast() {
    let first = tokenize_line("Choose a creature type other than Dragon.", 0);
    let second = tokenize_line("All creatures become that type.", 0);
    let become_effects = parse_choose_creature_type_then_become_type(&first, &second)
        .expect("parse choose-type-then-become sequence")
        .expect("expected choose-type-then-become sequence");
    assert_eq!(become_effects.len(), 1);

    let battlefield = tokenize_line(
        "Target opponent chooses a card, then you put that card onto the battlefield tapped under its owner's control.",
        0,
    );
    let put_onto_battlefield =
        parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(&battlefield)
            .expect("parse choose-then-battlefield sequence")
            .expect("expected choose-then-battlefield sequence");
    assert!(matches!(
        put_onto_battlefield.as_slice(),
        [EffectAst::ChooseObjects { .. }, _]
    ));
}

#[test]
fn parse_cant_restriction_clause_supports_that_player_cant_cast_spells() {
    let tokens = tokenize_line("That player can't cast spells.", 0);

    let parsed = parse_cant_restriction_clause(&tokens)
        .expect("parse that-player cant-cast clause")
        .expect("expected cant restriction");

    assert_eq!(
        parsed.restriction,
        Restriction::cast_spells(PlayerFilter::IteratedPlayer)
    );
}
