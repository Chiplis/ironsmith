use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{filters, leaf, primitives};
use super::nearby_primitives::{semantic_all, semantic_kw, semantic_noise, semantic_phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterReplacementShape<'a> {
    CounterAdjustment {
        filter_tokens: &'a [OwnedLexToken],
        counter_type: ironsmith_core::CounterType,
        adjustment: i64,
    },
    GenericUnderYourControl,
    EnergyYouGet,
    /// "If you would put one or more counters on a permanent or player, put
    /// twice that many of each of those kinds of counters on that permanent
    /// or player instead." (Innkeeper's Talent); `opponent` reads "If an
    /// opponent would put ... they put half that many ... rounded down"
    /// (Vorinclex, Monstrous Raider).
    ActorAnyKindMultiply { opponent: bool, halve: bool },
    PlusOneAdd {
        filter_tokens: &'a [OwnedLexToken],
        additional: u32,
    },
    PlusOneDouble {
        filter_tokens: &'a [OwnedLexToken],
    },
    /// "If one or more counters would be put on an artifact or creature you
    /// control, that many plus one of each of those kinds of counters are put
    /// on that permanent instead." (Winding Constrictor)
    AnyKindAdd {
        filter_tokens: &'a [OwnedLexToken],
        additional: u32,
    },
    /// "If you would get one or more counters, you get that many plus one of
    /// each of those kinds of counters instead." (Winding Constrictor)
    PlayerAnyKindAdd {
        additional: u32,
    },
    PlayerCounterPerTurnLimit {
        counter_type: ironsmith_core::CounterType,
        maximum: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCreationReplacementShape<'a> {
    GenericUnderYourControl,
    /// "If one or more tokens would be created [under your control], <factor>
    /// that many of those tokens are created instead." (Primal Vigor, Ojer
    /// Taq, Deepest Foundation)
    GenericMultiplied {
        /// Words between "one or more" and "tokens" ("creature" for Ojer
        /// Taq); empty when every token is multiplied.
        descriptor_tokens: &'a [OwnedLexToken],
        under_your_control: bool,
        factor: u32,
    },
    AddTreasure {
        descriptor_tokens: &'a [OwnedLexToken],
    },
    /// "If one or more tokens would be created under your control, those
    /// tokens plus an additional Food token are created instead." (Peregrin
    /// Took). `descriptor_tokens` narrows the replaced creation ("Food
    /// tokens") and is empty when every token creation qualifies;
    /// `additional_kind_word` is the added token's name ("food", "treasure").
    AddNamedToken {
        descriptor_tokens: &'a [OwnedLexToken],
        additional_kind_word: &'static str,
        /// "those tokens plus that many ... tokens" (Chatterfang).
        per_created: bool,
    },
    /// "If you would create a Clue, Food, or Treasure token, instead create
    /// one of each." (Academy Manufactor). `kind_tokens` is the listed kinds.
    OneOfEach {
        kind_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordActionReplacementShape<'a> {
    ProliferateYouTwice,
    ProliferateOpponentTwice,
    ExploreTwice,
    ExploreAfterScry { value_tokens: &'a [OwnedLexToken] },
    AssembleRiggerTwice,
    PlaneswalkAfterPlanarDeckChoice { count: u32 },
    LearnReturnThisFromGraveyard,
}

pub fn parse_noncombat_damage_minus_counter_replacement_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "if",
            "source",
            "you",
            "control",
            "would",
            "deal",
            "noncombat",
            "damage",
            "to",
            "creature",
            "opponent",
            "controls",
            "put",
            "that",
            "many",
            "-1/-1",
            "counters",
            "on",
            "that",
            "creature",
            "instead",
        ]),
        "noncombat damage minus-counter replacement",
    )
}

pub fn parse_counter_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterReplacementShape<'_>> {
    if parse_generic_counter_replacement(tokens) {
        return Some(CounterReplacementShape::GenericUnderYourControl);
    }
    if parse_energy_counter_replacement(tokens) {
        return Some(CounterReplacementShape::EnergyYouGet);
    }
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_player_counter_per_turn_limit_lexed,
            parse_counter_adjustment_lexed,
            parse_plus_one_add_lexed,
            parse_plus_one_double_lexed,
            parse_any_kind_add_lexed,
            parse_player_any_kind_add_lexed,
            parse_actor_any_kind_multiply_lexed,
        )),
        "counter replacement",
    )
}

pub fn parse_token_creation_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenCreationReplacementShape<'_>> {
    if parse_generic_token_replacement(tokens) {
        return Some(TokenCreationReplacementShape::GenericUnderYourControl);
    }
    if let Some(shape) = crate::grammar::primitives::probe_all(
        tokens,
        parse_multiplied_token_replacement_lexed,
        "multiplied token replacement",
    ) {
        return Some(shape);
    }
    if let Some(shape) = crate::grammar::primitives::probe_all(
        tokens,
        parse_add_named_token_replacement_lexed,
        "additional named token replacement",
    ) {
        return Some(shape);
    }
    if let Some(shape) = crate::grammar::primitives::probe_all(
        tokens,
        parse_one_of_each_token_replacement_lexed,
        "one of each token replacement",
    ) {
        return Some(shape);
    }
    crate::grammar::primitives::probe_all(
        tokens,
        parse_add_treasure_token_replacement_lexed,
        "additional treasure token replacement",
    )
}

fn parse_multiplied_token_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenCreationReplacementShape<'a>> {
    primitives::phrase(&["if", "one", "or", "more"]).parse_next(input)?;
    let descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::kw("tokens")),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["tokens", "would", "be", "created"]).parse_next(input)?;
    let under_your_control = opt(primitives::phrase(&["under", "your", "control"]))
        .parse_next(input)?
        .is_some();
    opt(primitives::comma()).parse_next(input)?;
    let factor = alt((
        primitives::kw("twice").value(2),
        primitives::phrase(&["two", "times"]).value(2),
        primitives::phrase(&["three", "times"]).value(3),
        primitives::kw("thrice").value(3),
    ))
    .parse_next(input)?;
    primitives::phrase(&[
        "that", "many", "of", "those", "tokens", "are", "created", "instead",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TokenCreationReplacementShape::GenericMultiplied {
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
        under_your_control,
        factor,
    })
}

fn parse_one_of_each_token_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenCreationReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "create", "a"]).parse_next(input)?;
    let kind_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["instead", "create", "one", "of", "each"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TokenCreationReplacementShape::OneOfEach {
        kind_tokens: trim_lexed_commas(kind_tokens),
    })
}

fn parse_add_named_token_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenCreationReplacementShape<'a>> {
    primitives::phrase(&["if", "one", "or", "more"]).parse_next(input)?;
    let descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    primitives::phrase(&["would", "be", "created", "under", "your", "control"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["those", "tokens", "plus"]).parse_next(input)?;
    let per_created = alt((
        primitives::phrase(&["that", "many"]).value(true),
        (
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
            primitives::kw("additional"),
        )
            .value(false),
    ))
    .parse_next(input)?;
    let repeated_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    primitives::phrase(&["are", "created", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let descriptor_words = TokenWordView::new(descriptor_tokens).word_refs();
    let repeated_words = TokenWordView::new(repeated_descriptor).word_refs();
    let additional_kind_word = match repeated_words.as_slice() {
        ["food"] => "food",
        ["treasure"] => "treasure",
        ["1/1", "green", "squirrel", "creature"] if per_created => "squirrel",
        _ => {
            return Err(primitives::backtrack_err(
                "additional named token replacement",
                "a Food or Treasure token",
            ));
        }
    };
    if !per_created && !descriptor_words.is_empty() && descriptor_words != repeated_words {
        return Err(primitives::backtrack_err(
            "additional named token replacement",
            "a repeated token descriptor",
        ));
    }
    Ok(TokenCreationReplacementShape::AddNamedToken {
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
        additional_kind_word,
        per_created,
    })
}

pub fn parse_keyword_action_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordActionReplacementShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_learn_return_from_graveyard_replacement_lexed,
            parse_planeswalk_planar_deck_replacement_lexed,
            parse_proliferate_you_replacement_lexed,
            parse_proliferate_opponent_replacement_lexed,
            parse_explore_replacement_lexed,
            parse_assemble_rigger_replacement_lexed,
        )),
        "keyword-action replacement",
    )
}

fn parse_learn_return_from_graveyard_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&[
        "as",
        "long",
        "as",
        "this",
        "card",
        "is",
        "in",
        "your",
        "graveyard",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["if", "you", "would", "learn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "you",
        "may",
        "instead",
        "return",
        "this",
        "card",
        "to",
        "the",
        "battlefield",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::LearnReturnThisFromGraveyard)
}

fn parse_planeswalk_planar_deck_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "planeswalk"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["instead", "look", "at", "the", "top"]).parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["cards", "of", "your", "planar", "deck"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "put", "one", "on", "the", "bottom", "of", "your", "planar", "deck", "and", "the", "other",
        "on", "top",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "planeswalk"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::PlaneswalkAfterPlanarDeckChoice { count })
}

fn parse_generic_counter_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "if",
            "effect",
            "would",
            "put",
            "one",
            "or",
            "more",
            "counters",
            "on",
            "permanent",
            "you",
            "control",
            "it",
            "puts",
            "twice",
            "that",
            "many",
            "of",
            "those",
            "counters",
            "on",
            "that",
            "permanent",
            "instead",
        ]),
        "generic double-counter replacement",
    )
}

fn parse_proliferate_you_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "proliferate"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["proliferate", "twice", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::ProliferateYouTwice)
}

fn parse_proliferate_opponent_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "an", "opponent", "would", "proliferate"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "player", "proliferates", "twice", "instead"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::ProliferateOpponentTwice)
}

fn parse_explore_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "a", "creature", "you", "control", "would", "explore"])
        .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    alt((
        (
            primitives::kw("you"),
            primitives::kw("scry"),
            repeat_till::<_, _, (), _, _, _, _>(
                1..,
                any.void(),
                peek((opt(primitives::comma()), primitives::kw("then"))),
            )
            .map(|((), _)| ())
            .take(),
            opt(primitives::comma()),
            primitives::kw("then"),
            alt((
                primitives::phrase(&["it", "explores"]),
                primitives::phrase(&["that", "creature", "explores"]),
            )),
            primitives::sentence_end(),
        )
            .map(|(_, _, value_tokens, _, _, _, _)| {
                KeywordActionReplacementShape::ExploreAfterScry {
                    value_tokens: trim_lexed_commas(value_tokens),
                }
            }),
        (
            primitives::phrase(&["it", "explores"]),
            opt(primitives::comma()),
            primitives::phrase(&["then", "it", "explores", "again"]),
            primitives::sentence_end(),
        )
            .value(KeywordActionReplacementShape::ExploreTwice),
    ))
    .parse_next(input)
}

fn parse_assemble_rigger_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&[
        "if",
        "a",
        "rigger",
        "you",
        "control",
        "would",
        "assemble",
        "a",
        "contraption",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "assembles", "two", "contraptions", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::AssembleRiggerTwice)
}

#[cfg(test)]
mod learn_replacement_tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(text: &str) -> Option<KeywordActionReplacementShape<'_>> {
        let tokens = Box::leak(Box::new(lex_line(text, 0).expect("line should lex")));
        parse_keyword_action_replacement_tokens(tokens)
    }

    #[test]
    fn graveyard_learn_replacement_is_exact_and_optional_surface_is_required() {
        assert!(matches!(
            parse(
                "As long as this card is in your graveyard, if you would learn, you may instead return this card to the battlefield."
            ),
            Some(KeywordActionReplacementShape::LearnReturnThisFromGraveyard)
        ));
        for near_miss in [
            "As long as this card is in your hand, if you would learn, you may instead return this card to the battlefield.",
            "As long as this card is in your graveyard, whenever you learn, return this card to the battlefield.",
            "As long as this card is in your graveyard, if you would learn, instead return this card to the battlefield.",
        ] {
            assert!(parse(near_miss).is_none(), "overclaimed: {near_miss}");
        }
    }
}

fn parse_energy_counter_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        (
            semantic_phrase(&["if", "you", "would", "get", "one", "or", "more"]),
            semantic_energy_symbol,
            opt(semantic_phrase(&["energy", "counters"])),
            semantic_phrase(&["you", "get", "twice", "that", "many"]),
            semantic_energy_symbol,
            semantic_kw("instead"),
        )
            .void(),
        "double energy-counter replacement",
    )
}

fn parse_player_counter_per_turn_limit_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "get", "one", "or", "more"]).parse_next(input)?;
    let first_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["instead", "you", "get"]).parse_next(input)?;
    let maximum = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let replacement_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["and", "you", "can't", "get", "additional"]).parse_next(input)?;
    let additional_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let first = filters::parse_counter_type_from_tokens(trim_lexed_commas(first_descriptor));
    let replacement =
        filters::parse_counter_type_from_tokens(trim_lexed_commas(replacement_descriptor));
    let additional =
        filters::parse_counter_type_from_tokens(trim_lexed_commas(additional_descriptor));
    let Some(counter_type) =
        first.filter(|kind| Some(*kind) == replacement && replacement == additional)
    else {
        return Err(primitives::backtrack_err(
            "player counter per-turn replacement",
            "matching counter types",
        ));
    };

    Ok(CounterReplacementShape::PlayerCounterPerTurnLimit {
        counter_type,
        maximum,
    })
}

fn semantic_energy_symbol<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify(|token: &&OwnedLexToken| {
        token
            .mana_group_inner()
            .is_some_and(|inner| inner.eq_ignore_ascii_case("e"))
    })
    .void()
    .parse_next(input)
}

fn parse_any_kind_add_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterReplacementShape<'a>> {
    primitives::phrase(&["if", "one", "or", "more", "counters", "would", "be", "put", "on"])
        .parse_next(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["that", "many", "plus"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "many", "plus"]).parse_next(input)?;
    let additional = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&[
        "of", "each", "of", "those", "kinds", "of", "counters", "are", "put", "on",
    ])
    .parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::AnyKindAdd {
        filter_tokens: trim_lexed_commas(filter_tokens),
        additional,
    })
}

fn parse_player_any_kind_add_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "get", "one", "or", "more", "counters"])
        .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "get", "that", "many", "plus"]).parse_next(input)?;
    let additional = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["of", "each", "of", "those", "kinds", "of", "counters", "instead"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::PlayerAnyKindAdd { additional })
}

fn parse_actor_any_kind_multiply_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterReplacementShape<'a>> {
    primitives::kw("if").parse_next(input)?;
    let opponent = alt((
        primitives::kw("you").value(false),
        primitives::phrase(&["an", "opponent"]).value(true),
    ))
    .parse_next(input)?;
    primitives::phrase(&["would", "put", "one", "or", "more", "counters", "on"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["permanent", "or", "player"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    opt(alt((primitives::kw("they"), primitives::kw("you")))).parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    let halve = alt((
        primitives::kw("twice").value(false),
        primitives::kw("half").value(true),
    ))
    .parse_next(input)?;
    primitives::phrase(&["that", "many", "of", "each", "of", "those", "kinds", "of", "counters", "on", "that"])
        .parse_next(input)?;
    primitives::phrase(&["permanent", "or", "player", "instead"]).parse_next(input)?;
    if halve {
        opt(primitives::comma()).parse_next(input)?;
        primitives::phrase(&["rounded", "down"]).parse_next(input)?;
    }
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::ActorAnyKindMultiply { opponent, halve })
}

fn parse_counter_adjustment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterReplacementShape<'a>> {
    primitives::phrase(&["if", "one", "or", "more"]).parse_next(input)?;
    let counter_tokens = take_until_replacement_phrase(input, &["counters", "would", "be", "put", "on"])?;
    let counter_type = filters::parse_counter_type_from_tokens(counter_tokens)
        .ok_or_else(|| primitives::backtrack_err("counter adjustment", "counter kind"))?;
    primitives::phrase(&["counters", "would", "be", "put", "on"]).parse_next(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["that", "many"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "many"]).parse_next(input)?;
    let repeated = take_until_replacement_phrase(input, &["counters"])?;
    if filters::parse_counter_type_from_tokens(repeated) != Some(counter_type) {
        return Err(primitives::backtrack_err("counter adjustment", "matching counter kind"));
    }
    primitives::kw("counters").parse_next(input)?;
    let sign = alt((primitives::kw("minus").value(-1i64), primitives::kw("plus").value(1i64))).parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["are", "put", "on"]).parse_next(input)?;
    alt((primitives::kw("it").void(), primitives::phrase(&["that", "permanent"]), primitives::phrase(&["that", "creature"]))).parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::CounterAdjustment { filter_tokens: trim_lexed_commas(filter_tokens), counter_type, adjustment: sign * i64::from(count) })
}

fn parse_plus_one_add_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterReplacementShape<'a>> {
    parse_plus_one_counter_prefix(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["that", "many", "plus"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "many", "plus"]).parse_next(input)?;
    let additional = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    opt((
        primitives::kw("+1/+1"),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    ))
    .parse_next(input)?;
    primitives::phrase(&["are", "put", "on"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::PlusOneAdd {
        filter_tokens: trim_lexed_commas(filter_tokens),
        additional,
    })
}

fn parse_plus_one_double_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterReplacementShape<'a>> {
    parse_plus_one_counter_prefix(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["twice", "that", "many"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["twice", "that", "many"]).parse_next(input)?;
    opt((
        primitives::kw("+1/+1"),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    ))
    .parse_next(input)?;
    primitives::phrase(&["are", "put", "on"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::PlusOneDouble {
        filter_tokens: trim_lexed_commas(filter_tokens),
    })
}

fn parse_plus_one_counter_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "if", "one", "or", "more", "+1/+1", "counters", "would", "be", "put", "on",
    ])
    .parse_next(input)
}

fn take_until_replacement_phrase<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((opt(primitives::comma()), primitives::phrase(phrase))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)
}

fn parse_generic_token_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        alt((
            semantic_phrase(&[
                "if", "effect", "would", "create", "one", "or", "more", "tokens", "under", "your",
                "control", "it", "creates", "twice", "that", "many", "of", "those", "tokens",
                "instead",
            ]),
            semantic_phrase(&[
                "if", "one", "or", "more", "tokens", "would", "be", "created", "under", "your",
                "control", "twice", "that", "many", "of", "those", "tokens", "are", "created",
                "instead",
            ]),
        )),
        "generic double-token replacement",
    )
}

fn parse_add_treasure_token_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenCreationReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "create", "one", "or", "more"]).parse_next(input)?;
    let descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["instead", "create", "those", "tokens", "plus"]).parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    let repeated_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let descriptor_words = TokenWordView::new(descriptor_tokens).word_refs();
    let repeated_words = TokenWordView::new(repeated_descriptor).word_refs();
    if descriptor_words != repeated_words
        || primitives::find_prefix(descriptor_tokens, || primitives::kw("treasure").void())
            .is_none()
    {
        return Err(primitives::backtrack_err(
            "additional treasure replacement",
            "matching Treasure token descriptors",
        ));
    }
    Ok(TokenCreationReplacementShape::AddTreasure {
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_counter_and_token_replacements() {
        let tokens = lex_line(
            "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_counter_replacement_tokens(&tokens),
            Some(CounterReplacementShape::PlusOneAdd { additional: 1, .. })
        ));
        let permanent = lex_line(
            "If one or more +1/+1 counters would be put on a permanent you control, that many plus one +1/+1 counters are put on that permanent instead.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_counter_replacement_tokens(&permanent),
            Some(CounterReplacementShape::PlusOneAdd { additional: 1, .. })
        ));
        let tokens = lex_line(
            "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_token_creation_replacement_tokens(&tokens),
            Some(TokenCreationReplacementShape::AddTreasure { .. })
        ));

        let energy = lex_line(
            "If you would get one or more {E} (energy counters), you get twice that many {E} instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_counter_replacement_tokens(&energy),
            Some(CounterReplacementShape::EnergyYouGet)
        );

        let per_turn_limit = lex_line(
            "If you would get one or more poison counters, instead you get one poison counter and you can't get additional poison counters this turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_counter_replacement_tokens(&per_turn_limit),
            Some(CounterReplacementShape::PlayerCounterPerTurnLimit {
                counter_type: ironsmith_core::CounterType::Poison,
                maximum: 1,
            })
        );
    }

    #[test]
    fn parses_comma_separated_double_explore_replacement() {
        let tokens = lex_line(
            "If a creature you control would explore, instead it explores, then it explores again.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_action_replacement_tokens(&tokens),
            Some(KeywordActionReplacementShape::ExploreTwice)
        );
    }

    #[test]
    fn parses_rigger_assemble_replacement() {
        let tokens = lex_line(
            "If a Rigger you control would assemble a Contraption, it assembles two Contraptions instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_action_replacement_tokens(&tokens),
            Some(KeywordActionReplacementShape::AssembleRiggerTwice)
        );
    }

    #[test]
    fn parses_planeswalk_planar_deck_replacement() {
        let tokens = lex_line(
            "If you would planeswalk, instead look at the top two cards of your planar deck, put one on the bottom of your planar deck and the other on top, then planeswalk.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_action_replacement_tokens(&tokens),
            Some(KeywordActionReplacementShape::PlaneswalkAfterPlanarDeckChoice { count: 2 })
        );
    }
}
