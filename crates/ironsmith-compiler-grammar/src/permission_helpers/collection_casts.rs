//! One-shot casts drawn from a named card collection while a spell or
//! ability resolves (CR 608.2g; each cast follows CR 601.2):
//!
//! - "You may cast any number of spells from among cards exiled this way
//!   without paying their mana costs." (the set an earlier instruction exiled)
//! - "You may cast up to two spells from among the exiled cards ..."
//! - "You may cast a spell with mana value X from among cards exiled with
//!   this creature without paying its mana cost." (cards linked to this
//!   source, CR 607.2a)
//! - "Cast any number of cards exiled with this creature without paying their
//!   mana costs."
//! - "Cast any number of red instant and/or sorcery cards from your graveyard
//!   without paying their mana costs."
//!
//! Choosing the cards is both the authored count and the "may" (choosing none
//! casts nothing); each chosen card is then cast, paying its mana cost unless
//! the clause says otherwise. Lands can't be cast (CR 305.9), so a generic card
//! or spell subject never offers one.
use super::*;
use crate::effect::ChoiceCount;
use crate::filter::Comparison;
use crate::grammar::{leaf, primitives};
use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectionMarker {
    ManaValue,
    FromAmong,
    OwnedExiledWith,
    ExiledWith,
    FromYourGraveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Collection {
    /// "them", "those cards": the set the previous instruction produced.
    PriorResult,
    /// "cards exiled this way", "the exiled cards": the set an earlier exile
    /// instruction produced, even across an intervening action.
    PriorExiled,
    /// Cards exiled with this source (CR 607.2a), optionally only those the
    /// caster owns.
    SourceLinked { owned_by_you: bool },
    /// The caster's graveyard.
    YourGraveyard,
    /// Cards the caster owns outside the game (the sideboard, CR 400.11).
    OwnedOutsideGame,
}

fn collection_marker<'a>(
    input: &mut crate::lexer::LexStream<'a>,
) -> WResult<CollectionMarker> {
    alt((
        primitives::phrase(&["with", "mana", "value"]).value(CollectionMarker::ManaValue),
        primitives::phrase(&["from", "among"]).value(CollectionMarker::FromAmong),
        primitives::phrase(&["you", "own", "exiled", "with"])
            .value(CollectionMarker::OwnedExiledWith),
        primitives::phrase(&["exiled", "with"]).value(CollectionMarker::ExiledWith),
        primitives::phrase(&["from", "your", "graveyard"])
            .value(CollectionMarker::FromYourGraveyard),
    ))
    .parse_next(input)
}

fn mana_value_bound<'a>(input: &mut crate::lexer::LexStream<'a>) -> WResult<Comparison> {
    alt((
        primitives::phrase(&["x", "or", "less"])
            .value(Comparison::LessThanOrEqualExpr(Box::new(Value::X))),
        primitives::kw("x").value(Comparison::EqualExpr(Box::new(Value::X))),
        (
            leaf::parse_leaf_number_token_lexed,
            opt(primitives::phrase(&["or", "less"])),
        )
            .map(|(value, or_less)| {
                if or_less.is_some() {
                    Comparison::LessThanOrEqual(value as i32)
                } else {
                    Comparison::Equal(value as i32)
                }
            }),
    ))
    .parse_next(input)
}

fn parse_cast_count(tokens: &[OwnedLexToken]) -> (Option<ChoiceCount>, &[OwnedLexToken]) {
    if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["any", "number", "of"]))
    {
        return (Some(ChoiceCount::any_number()), rest);
    }
    if let Some((count, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["up", "to"]),
            leaf::parse_leaf_number_token_lexed,
        )
            .map(|((), count)| count),
    ) {
        return (Some(ChoiceCount::up_to(count as usize)), rest);
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("a"), primitives::kw("an"))),
    ) {
        return (Some(ChoiceCount::up_to(1)), rest);
    }
    (None, tokens)
}

/// "this artifact", "this Saga", "this" (a normalized card name), or the
/// card's authored short name ("Jeleva").
fn is_this_source_reference(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = token_word_refs(tokens);
    if crate::util::is_source_reference_words(&words) {
        return true;
    }
    tokens.iter().all(|token| {
        matches!(token.kind, TokenKind::Word)
            && token.slice.chars().next().is_some_and(char::is_uppercase)
    })
}

/// Split the collection reference from the free-cast tail.
fn split_free_cast_tail(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], bool)> {
    let Some((index, (), tail)) = primitives::find_prefix(tokens, || {
        (
            primitives::phrase(&["without", "paying"]),
            alt((
                primitives::phrase(&["its", "mana", "cost"]),
                primitives::phrase(&["their", "mana", "costs"]),
                primitives::phrase(&["their", "mana", "cost"]),
            )),
        )
            .void()
    }) else {
        return Some((trim_lexed_commas(tokens), false));
    };
    primitives::probe_all(
        trim_lexed_commas(tail),
        primitives::sentence_end(),
        "collection free-cast tail",
    )?;
    Some((trim_lexed_commas(&tokens[..index]), true))
}

fn parse_tagged_collection(tokens: &[OwnedLexToken]) -> Option<Collection> {
    if let Some(((), rest)) = primitives::parse_prefix(tokens, primitives::kw("cards").void()) {
        let (owned_by_you, rest) =
            match primitives::parse_prefix(rest, primitives::phrase(&["you", "own"])) {
                Some(((), rest)) => (true, rest),
                None => (false, rest),
            };
        if owned_by_you
            && primitives::probe_all(
                rest,
                primitives::phrase(&["outside", "the", "game"]),
                "cast collection outside the game",
            )
            .is_some()
        {
            return Some(Collection::OwnedOutsideGame);
        }
        if let Some(((), source)) =
            primitives::parse_prefix(rest, primitives::phrase(&["exiled", "with"]))
        {
            return is_this_source_reference(source)
                .then_some(Collection::SourceLinked { owned_by_you });
        }
        if owned_by_you {
            return None;
        }
    }
    primitives::probe_all(
        tokens,
        alt((
            primitives::any_phrase(&[&["them"], &["those", "cards"]])
                .value(Collection::PriorResult),
            primitives::any_phrase(&[
                &["the", "exiled", "cards"],
                &["those", "exiled", "cards"],
                &["cards", "exiled", "this", "way"],
                &["the", "cards", "exiled", "this", "way"],
            ])
            .value(Collection::PriorExiled),
        )),
        "cast collection reference",
    )
}

/// Read one complete collection cast clause.
///
/// Two owners reach it, so no clause is ever claimed twice: the
/// `cast`-headed clause primitive (imperative "Cast any number of ...",
/// which no other primitive reads), and the final fallback of
/// [`parse_cast_or_play_tagged_clause`] for "you may cast ..." clauses,
/// reached only when every established permission shape declined.
pub(crate) fn parse_collection_cast_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = trim_commas(tokens);
    let mut body: &[OwnedLexToken] = &trimmed;
    let mut mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::Normal;
    if let Some(fact) = strip_allow_any_color_for_cast_suffix_tokens(body) {
        mana_spend_mode = fact.mana_spend_mode;
        body = trim_lexed_commas(fact.body_tokens);
    }
    let rest = match primitives::parse_prefix(body, primitives::phrase(&["you", "may", "cast"]))
    {
        Some(((), rest)) => rest,
        None => match primitives::parse_prefix(body, primitives::kw("cast")) {
            Some((_, rest)) => rest,
            None => return Ok(None),
        },
    };
    let (authored_count, rest) = parse_cast_count(rest);
    let Some((marker_index, marker, after_marker)) =
        primitives::find_prefix(rest, || collection_marker)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_lexed_commas(&rest[..marker_index]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let (mana_value, marker, after_marker) = if marker == CollectionMarker::ManaValue {
        let Some((bound, after_bound)) = primitives::parse_prefix(after_marker, mana_value_bound)
        else {
            return Ok(None);
        };
        let Some((marker, after_marker)) = primitives::parse_prefix(after_bound, collection_marker)
        else {
            return Ok(None);
        };
        if marker == CollectionMarker::ManaValue {
            return Ok(None);
        }
        (Some(bound), marker, after_marker)
    } else {
        (None, marker, after_marker)
    };
    let Some((reference_tokens, without_paying_mana_cost)) = split_free_cast_tail(after_marker)
    else {
        return Ok(None);
    };
    let collection = match marker {
        CollectionMarker::FromAmong => match parse_tagged_collection(reference_tokens) {
            Some(collection) => collection,
            None => return Ok(None),
        },
        CollectionMarker::OwnedExiledWith | CollectionMarker::ExiledWith => {
            if !is_this_source_reference(reference_tokens) {
                return Ok(None);
            }
            Collection::SourceLinked {
                owned_by_you: marker == CollectionMarker::OwnedExiledWith,
            }
        }
        CollectionMarker::FromYourGraveyard => {
            if !reference_tokens.is_empty() {
                return Ok(None);
            }
            Collection::YourGraveyard
        }
        CollectionMarker::ManaValue => return Ok(None),
    };

    // An uncounted permission over the source's linked cards is an
    // ongoing static permission. It must not be folded into a neighboring
    // activated instruction as an immediate collection cast.
    if matches!(collection, Collection::SourceLinked { .. }) && authored_count.is_none() {
        return Ok(None);
    }
    let subject_words = token_word_refs(subject_tokens);
    let plural_subject = matches!(subject_words.last(), Some(&"spells" | &"cards"));
    let mut filter = if matches!(subject_words.as_slice(), ["card"] | ["cards"]) {
        ObjectFilter::default()
    } else {
        let Some(mut filter) =
            permission_subject_facts::parse_cast_permission_filter_tokens(subject_tokens)?
        else {
            return Ok(None);
        };
        filter.zone = None;
        filter
    };
    if matches!(subject_words.as_slice(), ["card"] | ["cards"])
        || permission_subject_facts::generic_spell_subject_requires_nonland(subject_tokens)
    {
        exclude_lands_from_spell_filter(&mut filter);
    }
    filter.mana_value = mana_value;
    let count = match authored_count {
        Some(count) => count,
        None if plural_subject => ChoiceCount::any_number(),
        None => return Ok(None),
    };
    match collection {
        Collection::PriorResult => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.bind().into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        Collection::PriorExiled => {
            filter.zone = Some(Zone::Exile);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.bind().into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Exiled));
        }
        Collection::SourceLinked { owned_by_you } => {
            filter.zone = Some(Zone::Exile);
            if owned_by_you {
                filter.owner = Some(PlayerFilter::You);
            }
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        Collection::YourGraveyard => {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        Collection::OwnedOutsideGame => {
            filter.zone = Some(Zone::OutsideGame);
            filter.owner = Some(PlayerFilter::You);
        }
    }

    let chosen = super::super::util::helper_tag_for_tokens(tokens, "cast_from_collection");
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count,
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(chosen.clone()),
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(chosen),
                effects: vec![
                    EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        PlayerAst::You,
                        false,
                        false,
                        without_paying_mana_cost,
                        None,
                        None,
                        mana_spend_mode,
                    ),
                ],
            }),
        ],
    }))
}
