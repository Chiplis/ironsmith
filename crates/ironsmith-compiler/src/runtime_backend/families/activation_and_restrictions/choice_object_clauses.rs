use super::*;
use crate::runtime_backend::grammar::choices::{
    ChoiceBattlefieldController, ChoiceBecomeKind, ChoiceBecomeSyntaxError, ChoiceClauseActor,
    ChoiceObjectClauseSyntaxError, ChoiceObjectCountSource, ChoicePlayerClauseSyntaxError,
    ChoiceTypePhraseSyntaxError, ChosenCantBlockSyntaxError, TargetPlayerChoiceActor,
    TypedChoiceBecomeSubject, TypedChoiceObjectClauseKind,
    parse_choice_basic_land_type_phrase_words, parse_choice_battlefield_move_shape,
    parse_choice_card_type_phrase_words as parse_typed_choice_card_type_phrase_words,
    parse_choice_card_type_reveal_shape_words,
    parse_choice_color_phrase_words as parse_typed_choice_color_phrase_words,
    parse_choice_creature_type_phrase_words as parse_typed_choice_creature_type_phrase_words,
    parse_choice_land_type_phrase_words as parse_typed_choice_land_type_phrase_words,
    parse_choice_library_move_shape, parse_choice_player_clause_tokens,
    parse_choice_player_phrase_words as parse_typed_choice_player_phrase_words,
    parse_that_type_tokens, parse_typed_choice_become_shape,
    parse_typed_choice_object_clause_tokens, parse_typed_chosen_cant_block_tokens,
    parse_typed_target_player_choice_tokens,
};

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    Ok(
        parse_target_player_choose_objects_clause_with_count_value(tokens)?
            .map(|(chooser, filter, count, _count_value)| (chooser, filter, count)),
    )
}

pub(crate) fn parse_target_player_choose_objects_clause_with_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount, Option<Value>)>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let parsed = match parse_typed_target_player_choice_tokens(tokens) {
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
        Err(ChoiceObjectClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen object filter in target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    let mut chooser = match parsed.actor {
        TargetPlayerChoiceActor::TargetPlayer => PlayerAst::Target,
        TargetPlayerChoiceActor::TargetOpponent => PlayerAst::TargetOpponent,
        TargetPlayerChoiceActor::Opponent => PlayerAst::Opponent,
        TargetPlayerChoiceActor::ThatPlayer | TargetPlayerChoiceActor::Voter => PlayerAst::That,
    };
    let mut choose_filter = parsed.filter;
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
    // Choosing an unrestricted battlefield object does not imply that the
    // chooser controls it. Preserve an inferred controller only for a tagged
    // antecedent whose actor is explicitly derived from that object; ordinary
    // text must say `they control` when that restriction is intended.
    if choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            PlayerAst::ItsController => {
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
            }
            _ => PlayerFilter::target_player(),
        });
    }

    let count_value = choice_count_value(parsed.count_source, &clause_words)?;

    Ok(Some((chooser, choose_filter, parsed.count, count_value)))
}

fn choice_count_value(
    count_source: Option<ChoiceObjectCountSource>,
    clause_words: &[&str],
) -> Result<Option<Value>, CardTextError> {
    match count_source {
        Some(ChoiceObjectCountSource::CardsDiscardedThisWay) => Ok(Some(Value::Count(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
        ))),
        Some(ChoiceObjectCountSource::ThatMany) => Ok(Some(Value::EventValue(
            crate::effect::EventValueSpec::Amount,
        ))),
        Some(ChoiceObjectCountSource::ForEach(count_words)) => {
            let count_word_refs = count_words.iter().map(String::as_str).collect::<Vec<_>>();
            let Some((value, consumed)) =
                crate::runtime_backend::util::parse_for_each_count_value_words(&count_word_refs)
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported for-each object-choice count (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            if consumed != count_word_refs.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing words in object-choice count (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            Ok(Some(value.with_surface_hint(
                ironsmith_core::ValueSurfaceHint::ForEach,
            )))
        }
        None => Ok(None),
    }
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
    let parsed = match parse_typed_choice_object_clause_tokens(tokens) {
        Ok(Some(TypedChoiceObjectClauseKind::Object(parsed))) => parsed,
        Ok(Some(TypedChoiceObjectClauseKind::CardName)) | Ok(None) => return Ok(None),
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
        Err(ChoiceObjectClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen object filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };
    let references_it = parsed.references.references_it;
    let count_value = choice_count_value(parsed.count_source, &clause_words)?;
    let mut choose_filter = parsed.filter;
    let chooser = match parsed.actor {
        // Preserve the implicit actor until the enclosing sentence shape is
        // known. At top level lowering resolves it to you; inside
        // `Each player/opponent chooses ...`, the participant loop binds it
        // to the iterated player.
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };

    if references_it
        && !choose_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    {
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    if !references_it
        && chooser == PlayerAst::You
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
    let shape = match parse_typed_chosen_cant_block_tokens(second) {
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
        Err(ChosenCantBlockSyntaxError::UnsupportedObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-block subject filter (clause: '{}')",
                second_words.join(" ")
            )));
        }
    };

    let mut restriction_filter = shape.filter;
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
    let shape = match parse_typed_choice_become_shape(first, second) {
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
        Err(ChoiceBecomeSyntaxError::UnsupportedObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
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
        TypedChoiceBecomeSubject::AllObjects(filter) => TargetAst::Object(filter, None, None),
        TypedChoiceBecomeSubject::Target(subject_tokens) => parse_target_phrase(subject_tokens)?,
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

#[cfg(test)]
mod result_choice_tests {
    use super::*;

    #[test]
    fn implicit_choice_actor_is_preserved_for_enclosing_sentence_binding() {
        let tokens = crate::runtime_backend::lex_line("Choose a nonland card exiled this way.", 0)
            .expect("lex choice");
        let (chooser, filter, count, _) = parse_you_choose_objects_clause_with_count_value(&tokens)
            .expect("parse choice")
            .expect("match choice");
        assert_eq!(chooser, PlayerAst::Implicit);
        assert!(count.is_single());
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn up_to_prior_amount_choice_preserves_count_and_value_source() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose up to that many target creatures you control.",
            0,
        )
        .expect("lex choice");
        let (_, _, count, count_value) = parse_you_choose_objects_clause_with_count_value(&tokens)
            .expect("parse choice")
            .expect("match choice");

        assert!(count.is_up_to_dynamic_x());
        assert_eq!(
            count_value,
            Some(Value::EventValue(crate::effect::EventValueSpec::Amount))
        );
    }

    #[test]
    fn for_each_choice_count_lowers_to_a_typed_dynamic_value() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose a permanent for each card in their graveyard.",
            0,
        )
        .expect("lex choice");
        let (_, _, count, count_value) = parse_you_choose_objects_clause_with_count_value(&tokens)
            .expect("parse choice")
            .expect("match choice");

        assert!(count.is_dynamic_x());
        let value = count_value.expect("for-each count value");
        assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        let Value::Count(filter) = value.unhinted() else {
            panic!("expected an object-count value: {value:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn that_player_for_each_choice_keeps_its_dynamic_count_basis() {
        let tokens = crate::runtime_backend::lex_line(
            "That player chooses a permanent for each card in their graveyard.",
            0,
        )
        .expect("lex participant choice");
        let (chooser, filter, count, count_value) =
            parse_target_player_choose_objects_clause_with_count_value(&tokens)
                .expect("parse participant choice")
                .expect("match participant choice");

        assert_eq!(chooser, PlayerAst::That);
        assert!(count.is_dynamic_x());
        assert_eq!(filter.controller, None);
        let value = count_value.expect("for-each count value");
        assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        let Value::Count(filter) = value.unhinted() else {
            panic!("expected an object-count value: {value:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn participant_choice_only_restricts_control_when_oracle_says_so() {
        let unrestricted =
            crate::runtime_backend::lex_line("That player chooses up to two Plains.", 0)
                .expect("lex unrestricted choice");
        let (_, unrestricted_filter, _, _) =
            parse_target_player_choose_objects_clause_with_count_value(&unrestricted)
                .expect("parse unrestricted choice")
                .expect("match unrestricted choice");
        assert_eq!(unrestricted_filter.controller, None);

        let controlled = crate::runtime_backend::lex_line(
            "That player chooses up to two Plains they control.",
            0,
        )
        .expect("lex controlled choice");
        let (_, controlled_filter, _, _) =
            parse_target_player_choose_objects_clause_with_count_value(&controlled)
                .expect("parse controlled choice")
                .expect("match controlled choice");
        assert_eq!(
            controlled_filter.controller,
            Some(PlayerFilter::IteratedPlayer)
        );

        let negative_tag_only = crate::runtime_backend::lex_line(
            "That player chooses a permanent that hasn't been chosen this way.",
            0,
        )
        .expect("lex complement-only choice");
        let (_, negative_filter, _, _) =
            parse_target_player_choose_objects_clause_with_count_value(&negative_tag_only)
                .expect("parse complement-only choice")
                .expect("match complement-only choice");
        assert_eq!(negative_filter.controller, None);
        assert!(
            negative_filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
        );
    }
}
