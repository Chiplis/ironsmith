use super::*;
use crate::runtime_backend::effect_sentences::find_verb_words;
use crate::runtime_backend::grammar::choices::{
    ChoiceBattlefieldController, ChoiceBecomeKind, ChoiceBecomeSubject, ChoiceBecomeSyntaxError,
    ChoiceClauseActor, ChoiceObjectClauseKind, ChoiceObjectClauseSyntaxError,
    ChoiceObjectCountSource, ChoiceObjectFilterFacts, ChoicePlayerClauseSyntaxError,
    ChoiceTypePhraseSyntaxError, ChosenCantBlockSyntaxError, TargetPlayerChoiceActor,
    parse_choice_basic_land_type_phrase_words, parse_choice_battlefield_move_shape,
    parse_choice_become_shape,
    parse_choice_card_type_phrase_words as parse_typed_choice_card_type_phrase_words,
    parse_choice_card_type_reveal_shape_words,
    parse_choice_color_phrase_words as parse_typed_choice_color_phrase_words,
    parse_choice_creature_type_phrase_words as parse_typed_choice_creature_type_phrase_words,
    parse_choice_land_type_phrase_words as parse_typed_choice_land_type_phrase_words,
    parse_choice_library_move_shape, parse_choice_object_clause_tokens,
    parse_choice_player_clause_tokens,
    parse_choice_player_phrase_words as parse_typed_choice_player_phrase_words,
    parse_chosen_cant_block_shape, parse_target_player_choice_tokens, parse_that_type_tokens,
};

fn expand_graveyard_or_hand_disjunction_filter(
    mut filter: ObjectFilter,
    facts: ChoiceObjectFilterFacts,
) -> ObjectFilter {
    if !facts.graveyard_and_hand {
        return filter;
    }

    filter.zone = None;
    filter.controller = None;
    filter.any_of = vec![
        ObjectFilter {
            zone: Some(Zone::Graveyard),
            ..ObjectFilter::default()
        },
        ObjectFilter {
            zone: Some(Zone::Hand),
            ..ObjectFilter::default()
        },
    ];
    filter
}

fn expand_tagged_hand_or_graveyard_disjunction_filter(
    mut filter: ObjectFilter,
    facts: ChoiceObjectFilterFacts,
) -> ObjectFilter {
    if !facts.tagged_graveyard_disjunction {
        return filter;
    }
    let graveyard_arm_is_plain_card = facts.graveyard_arm_is_plain_card;

    let mut hand_arm = filter.clone();
    hand_arm.zone = Some(Zone::Hand);
    hand_arm.controller = None;
    hand_arm.owner = None;
    hand_arm.any_of.clear();
    if !hand_arm
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        hand_arm.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let mut graveyard_arm = filter.clone();
    graveyard_arm.zone = Some(Zone::Graveyard);
    graveyard_arm.any_of.clear();
    graveyard_arm
        .tagged_constraints
        .retain(|constraint| constraint.tag.as_str() != IT_TAG);
    if graveyard_arm_is_plain_card {
        graveyard_arm.excluded_card_types.clear();
    }

    filter.zone = None;
    filter.controller = None;
    filter.owner = None;
    filter.tagged_constraints.clear();
    if graveyard_arm_is_plain_card {
        filter.excluded_card_types.clear();
    }
    filter.any_of = vec![hand_arm, graveyard_arm];
    filter
}

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let parsed = match parse_target_player_choice_tokens(tokens) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return Ok(None),
        Err(ChoiceObjectClauseSyntaxError::MissingObject) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object after target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::MissingFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object filter after count in target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };
    if parsed.filter_is_player_target {
        return Ok(None);
    }
    if find_verb(parsed.filter_tokens).is_some() {
        return Ok(None);
    }

    let mut chooser = match parsed.actor {
        TargetPlayerChoiceActor::TargetPlayer => PlayerAst::Target,
        TargetPlayerChoiceActor::TargetOpponent => PlayerAst::TargetOpponent,
        TargetPlayerChoiceActor::ThatPlayer | TargetPlayerChoiceActor::Voter => PlayerAst::That,
    };
    let mut choose_filter = parse_object_filter(parsed.filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    choose_filter = expand_graveyard_or_hand_disjunction_filter(choose_filter, parsed.filter_facts);
    if chooser == PlayerAst::That
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        chooser = PlayerAst::ItsController;
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            PlayerAst::ItsController => {
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
            }
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, parsed.count)))
}

pub(crate) fn parse_you_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    Ok(parse_you_choose_objects_clause_with_count_value(tokens)?
        .map(|(chooser, filter, count, _count_value)| (chooser, filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause_with_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount, Option<Value>)>, CardTextError> {
    let trimmed_tokens = trim_edge_punctuation(tokens);
    let tokens = trimmed_tokens.as_slice();
    let clause_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let parsed = match parse_choice_object_clause_tokens(tokens) {
        Ok(Some(ChoiceObjectClauseKind::Object(parsed))) => parsed,
        Ok(Some(ChoiceObjectClauseKind::CardName)) | Ok(None) => return Ok(None),
        Err(ChoiceObjectClauseSyntaxError::MissingObject) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object after choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::MissingFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };
    let choose_words_storage = parsed.filter_words;
    let choose_words = choose_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let references_it = parsed.references.references_it;
    let references_container_it = parsed.references.references_container_it;
    let explicit_container_reference = parsed.references.explicit_container_reference;
    let filter_facts = parsed.filter_facts;
    let count_value = parsed.count_source.map(|source| match source {
        ChoiceObjectCountSource::CardsDiscardedThisWay => {
            Value::Count(ObjectFilter::tagged(TagKey::from(IT_TAG)))
        }
    });

    let controller_tail = crate::runtime_backend::object_filters::parse_simple_object_filter_words(
        &choose_words,
        false,
    )
    .is_some_and(|filter| filter.controller.is_some());
    if find_verb_words(&choose_words).is_some() && !controller_tail {
        return Ok(None);
    }

    let mut choose_filter = if references_it && filter_facts.bare_card {
        ObjectFilter::default()
    } else {
        crate::runtime_backend::object_filters::parse_object_filter_words(&choose_words, false)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported chosen object filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
    };
    choose_filter = expand_graveyard_or_hand_disjunction_filter(choose_filter, filter_facts);
    if references_it {
        if explicit_container_reference
            && matches!(choose_filter.zone, None | Some(Zone::Battlefield))
        {
            choose_filter.zone = Some(Zone::Hand);
        } else if references_container_it && choose_filter.zone.is_none() {
            choose_filter.zone = Some(Zone::Hand);
        }
        if !choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
        choose_filter =
            expand_tagged_hand_or_graveyard_disjunction_filter(choose_filter, filter_facts);
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    let chooser = match parsed.actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
    };

    if references_it {
        choose_filter.controller = None;
        choose_filter.owner = None;
    } else if chooser == PlayerAst::You
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter.could_be_targeted_by.is_none()
    {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((chooser, choose_filter, parsed.count, count_value)))
}

pub(crate) fn parse_you_choose_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, PlayerFilter, bool, usize)>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let trimmed_tokens = trim_edge_punctuation(tokens);
    let parsed = match parse_choice_player_clause_tokens(&trimmed_tokens) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return Ok(None),
        Err(ChoicePlayerClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    Ok(Some((
        PlayerAst::You,
        parsed.filter,
        parsed.random,
        parsed.exclude_previous_choices,
    )))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = crate::runtime_backend::token_word_refs(second);
    let shape = match parse_chosen_cant_block_shape(second) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(ChosenCantBlockSyntaxError::MissingSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing subject in cant-block clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChosenCantBlockSyntaxError::MissingObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in cant-block clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
    };

    let mut restriction_filter = if shape.bare_other_reference {
        ObjectFilter::default()
    } else {
        parse_object_filter(shape.subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported cant-block subject filter (clause: '{}')",
                second_words.join(" ")
            ))
        })?
    };
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if shape.exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(restriction_filter),
            Until::EndOfTurn,
            None,
        ),
    ]))
}

#[cfg(test)]
mod tests {
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
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
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
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
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
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
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
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
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
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::runtime_backend::token_word_refs(first);
    let second_words = crate::runtime_backend::token_word_refs(second);
    let Some(shape) = parse_choice_card_type_reveal_shape_words(&first_words, &second_words) else {
        return Ok(None);
    };

    Ok(Some(vec![
        compose_reveal_top_choose_card_type_put_to_hand_rest_bottom(first, shape.count),
    ]))
}

/// Composes the "choose a card type, reveal the top N, put all of that type into
/// your hand and the rest on the bottom" effect as a player modal
/// (`EffectAst::ChooseOneOf`) over the nine card types, mirroring the runtime
/// `Effect::choose_one` the retired `RevealTopChooseCardTypePutToHandRestBottom`
/// recipe lowered to. Each mode looks at the top N cards, reveals them, and for
/// each looked card moves it to hand if it matches the mode's card type, else to
/// the bottom of the library.
fn compose_reveal_top_choose_card_type_put_to_hand_rest_bottom(
    first: &[OwnedLexToken],
    count: u32,
) -> EffectAst {
    let card_type_modes = [
        ("Artifact", CardType::Artifact),
        ("Battle", CardType::Battle),
        ("Creature", CardType::Creature),
        ("Enchantment", CardType::Enchantment),
        ("Instant", CardType::Instant),
        ("Kindred", CardType::Kindred),
        ("Land", CardType::Land),
        ("Planeswalker", CardType::Planeswalker),
        ("Sorcery", CardType::Sorcery),
    ];

    let modes = card_type_modes
        .into_iter()
        .map(|(label, card_type)| {
            let looked_tag = crate::runtime_backend::util::helper_tag_for_tokens(
                first,
                &format!("revealed_{label}"),
            );
            let mut card_type_filter = ObjectFilter::default();
            card_type_filter.card_types.push(card_type);

            let effects = vec![
                EffectAst::subject_verb_look_at_top_cards(
                    PlayerAst::You,
                    Value::Fixed(count as i32),
                    looked_tag.clone(),
                ),
                EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
                EffectAst::ForEachTagged {
                    tag: looked_tag,
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::TaggedMatches(
                            TagKey::from(IT_TAG),
                            card_type_filter,
                        ),
                        if_true: vec![EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(TagKey::from(IT_TAG), None),
                            Zone::Hand,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )],
                        if_false: vec![EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(TagKey::from(IT_TAG), None),
                            Zone::Library,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )],
                    }],
                },
            ];

            crate::cards::builders::ChooseOneModeAst {
                description: label.to_string(),
                effects,
            }
        })
        .collect();

    EffectAst::ChooseOneOf { modes }
}

pub(crate) fn parse_choose_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<Subtype>)>, CardTextError> {
    match parse_typed_choice_creature_type_phrase_words(words) {
        Ok(Some(parsed)) => Ok(Some((parsed.consumed, parsed.excluded_subtypes))),
        Ok(None) => Ok(None),
        Err(ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion) => {
            Err(CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion) => {
            Err(CardTextError::ParseError(format!(
                "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(
            ChoiceTypePhraseSyntaxError::MissingColorExclusion
            | ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion,
        ) => Ok(None),
    }
}

pub(crate) fn parse_choose_color_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Option<ColorSet>)>, CardTextError> {
    match parse_typed_choice_color_phrase_words(words) {
        Ok(Some(parsed)) => Ok(Some((parsed.consumed, parsed.excluded))),
        Ok(None) => Ok(None),
        Err(ChoiceTypePhraseSyntaxError::MissingColorExclusion) => {
            Err(CardTextError::ParseError(format!(
                "missing color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion) => {
            Err(CardTextError::ParseError(format!(
                "unsupported color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(
            ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion
            | ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion,
        ) => Ok(None),
    }
}

pub(crate) fn parse_choose_card_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<CardType>)>, CardTextError> {
    Ok(parse_typed_choice_card_type_phrase_words(words)
        .map(|parsed| (parsed.consumed, parsed.options)))
}

pub(crate) fn parse_choose_player_phrase_words(words: &[&str]) -> Option<usize> {
    parse_typed_choice_player_phrase_words(words).map(|parsed| parsed.consumed)
}

pub(crate) fn parse_choose_basic_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    parse_choice_basic_land_type_phrase_words(words).map(|parsed| parsed.consumed)
}

pub(crate) fn parse_choose_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    parse_typed_choice_land_type_phrase_words(words).map(|parsed| parsed.consumed)
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::runtime_backend::token_word_refs(first);
    let second_words = crate::runtime_backend::token_word_refs(second);
    let shape = match parse_choice_become_shape(first, second) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(ChoiceBecomeSyntaxError::MissingCreatureSubtypeExclusion) => {
            return Err(CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedCreatureSubtypeExclusion) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedCreatureTypeClause) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedBasicLandTypeClause) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::MissingSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing target in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::MissingObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
    };

    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(shape.tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, shape.tail_tokens.to_vec())
        };
    if !parse_that_type_tokens(&become_tokens) {
        return Ok(None);
    }

    let target = match shape.subject {
        ChoiceBecomeSubject::AllObjects(filter_tokens) => {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported object filter in creature-type become clause (clause: '{}')",
                    second_words.join(" ")
                ))
            })?;
            TargetAst::Object(filter, None, None)
        }
        ChoiceBecomeSubject::Target(subject_tokens) => parse_target_phrase(subject_tokens)?,
    };

    let effect = match shape.kind {
        ChoiceBecomeKind::CreatureType { excluded_subtypes } => {
            EffectAst::subject_verb_become_creature_type_choice(target, duration, excluded_subtypes)
        }
        ChoiceBecomeKind::BasicLandType => {
            EffectAst::subject_verb_become_basic_land_type_choice(target, duration)
        }
    };

    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_choice_library_move_shape(tokens) else {
        return Ok(None);
    };

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(shape.first_clause)?
    else {
        return Ok(None);
    };

    let target = if shape.moved_is_tagged_choice {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(shape.second_clause))
    } else {
        parse_target_phrase(shape.moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_choice_battlefield_move_shape(tokens) else {
        return Ok(None);
    };

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(shape.first_clause)?
    else {
        return Ok(None);
    };
    let battlefield_controller = match shape.controller {
        ChoiceBattlefieldController::Preserve => ReturnControllerAst::Preserve,
        ChoiceBattlefieldController::You => ReturnControllerAst::You,
        ChoiceBattlefieldController::Owner => ReturnControllerAst::Owner,
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(shape.second_clause)),
            Zone::Battlefield,
            false,
            battlefield_controller,
            shape.tapped,
            None,
        ),
    ]))
}
