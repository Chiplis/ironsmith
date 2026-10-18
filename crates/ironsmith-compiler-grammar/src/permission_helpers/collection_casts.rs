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
    /// "from your graveyard and/or hand" (Invoke Calamity).
    FromYourGraveyardOrHand,
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
    /// The caster's graveyard and hand together: each chosen card may come
    /// from either zone.
    YourGraveyardOrHand,
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
        primitives::any_phrase(&[
            &["from", "your", "graveyard", "and/or", "hand"],
            &["from", "your", "graveyard", "or", "hand"],
            &["from", "your", "hand", "and/or", "graveyard"],
            &["from", "your", "hand", "or", "graveyard"],
        ])
        .value(CollectionMarker::FromYourGraveyardOrHand),
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
    // "cards exiled with it" (Kylox's Voltstrider): inside the source's own
    // ability the pronoun names the source (CR 607.2a linked pool), the same
    // reading the source-exiled permission tail already gives it.
    if words.as_slice() == ["it"] {
        return true;
    }
    tokens.iter().all(|token| {
        matches!(token.kind, TokenKind::Word)
            && token.slice.chars().next().is_some_and(char::is_uppercase)
    })
}

/// Strip a trailing "with total mana value N or less" / "X or less" cap on the
/// whole selection (Rod of Absorption, Primeval Spawn).
fn split_total_mana_value_cap(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Value>) {
    let Some((index, (), bound)) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["with", "total", "mana", "value"])
    }) else {
        return (tokens, None);
    };
    let words = token_word_refs(trim_lexed_commas(bound));
    let value = match words.as_slice() {
        ["x", "or", "less"] => Some(Value::X),
        [count, "or", "less"] => crate::util::parse_number_word_i32(count).map(Value::Fixed),
        _ => None,
    };
    match value {
        Some(value) => (trim_lexed_commas(&tokens[..index]), Some(value)),
        None => (tokens, None),
    }
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
    // "cast an artifact spell from your hand or graveyard by paying life
    // equal to its mana value rather than paying its mana cost" (Anrakyr the
    // Traveller): the price replaces the chosen card's mana cost.
    let priced_tokens = trim_commas(tokens);
    if let Some((body, price)) =
        super::effect_cast_prices::replacement_price_suffix(&priced_tokens)?
    {
        let Some(mut effect) = parse_unpriced_collection_cast_clause(body)? else {
            return Ok(None);
        };
        if super::effect_cast_prices::attach_price(&mut effect, &price) != 1 {
            return Ok(None);
        }
        return Ok(Some(effect));
    }
    parse_unpriced_collection_cast_clause(tokens)
}

fn parse_unpriced_collection_cast_clause(
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
            // "For each player, you may cast ...": the quantified wrapper
            // owns the optionality and leaves the bare "you cast" body.
            None => match primitives::parse_prefix(body, primitives::phrase(&["you", "cast"])) {
                Some(((), rest)) => rest,
                None => return Ok(None),
            },
        },
    };
    let (authored_count, rest) = parse_cast_count(rest);
    // "An opponent chooses one of those cards. Shuffle that card into your
    // library. You may cast the other cards without paying their mana
    // costs." (Emergent Ultimatum): the remainder of the collection the
    // earlier choice was made from. It is the ordinary "the rest" complement
    // (bound by lowering to the collection's cards still in its zone), and
    // choosing any number of them is the "may"; each is cast in turn.
    if authored_count.is_none()
        && let Some((reference_tokens, without_paying_mana_cost)) = split_free_cast_tail(rest)
        && matches!(
            token_word_refs(reference_tokens).as_slice(),
            ["the", "other", "cards"]
                | ["the", "rest"]
                | ["the", "rest", "of", "the", "cards"]
                | ["the", "rest", "of", "those", "cards"]
        )
    {
        let mut filter = ObjectFilter::default();
        exclude_lands_from_spell_filter(&mut filter);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::Rest.bind().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let chosen = super::super::util::helper_tag_for_tokens(tokens, "cast_from_collection");
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::ObjectChoices(
                    crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        count: ChoiceCount::any_number(),
                        count_value: None,
                        player: PlayerAst::You,
                        tag: crate::tag::TagRef::of(chosen.clone()),
                    },
                ),
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
        }));
    }
    // "For each player, you may cast a card that player milled this way
    // without paying its mana cost." (The Ur-Sphinx): one card from the
    // iterated player's share of the prior mill.
    if let Some((reference_tokens, without_paying_mana_cost)) = split_free_cast_tail(rest)
        && matches!(
            token_word_refs(reference_tokens).as_slice(),
            ["card" | "spell", "that", "player", "milled", "this", "way"]
        )
        && let Some(count) = authored_count
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        exclude_lands_from_spell_filter(&mut filter);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::It.bind().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Milled));
        let chosen = super::super::util::helper_tag_for_tokens(tokens, "cast_from_collection");
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::ObjectChoices(
                    crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        count,
                        count_value: None,
                        player: PlayerAst::You,
                        tag: crate::tag::TagRef::of(chosen.clone()),
                    },
                ),
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
        }));
    }
    let Some((marker_index, marker, after_marker)) =
        primitives::find_prefix(rest, || collection_marker)
    else {
        return Ok(None);
    };
    let (subject_tokens, subject_total_cap) =
        split_total_mana_value_cap(trim_lexed_commas(&rest[..marker_index]));
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
    let (reference_tokens, reference_total_cap) = split_total_mana_value_cap(reference_tokens);
    let total_mana_value_cap = subject_total_cap.or(reference_total_cap);
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
        CollectionMarker::FromYourGraveyardOrHand => {
            if !reference_tokens.is_empty() {
                return Ok(None);
            }
            Collection::YourGraveyardOrHand
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
        Collection::YourGraveyardOrHand => {
            // One candidate pool spanning both zones: a union whose
            // branches each carry the authored characteristics and a zone.
            let mut graveyard = filter.clone();
            graveyard.zone = Some(Zone::Graveyard);
            graveyard.owner = Some(PlayerFilter::You);
            let mut hand = filter.clone();
            hand.zone = Some(Zone::Hand);
            hand.owner = Some(PlayerFilter::You);
            filter = ObjectFilter {
                any_of: vec![graveyard, hand],
                ..ObjectFilter::default()
            };
        }
        Collection::OwnedOutsideGame => {
            filter.zone = Some(Zone::OutsideGame);
            filter.owner = Some(PlayerFilter::You);
        }
    }

    let chosen = super::super::util::helper_tag_for_tokens(tokens, "cast_from_collection");
    let choose = match total_mana_value_cap {
        Some(maximum) => EffectAst::ObjectChoices(
            crate::cards::builders::ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
                filter,
                count,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(chosen.clone()),
                constraint: crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(
                    maximum,
                ),
            },
        ),
        None => EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen.clone()),
        }),
    };
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            choose,
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
