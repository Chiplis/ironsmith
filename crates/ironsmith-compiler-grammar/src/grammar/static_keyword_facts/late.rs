//! Typed grammar facts for late static-keyword sentence families.
//!
//! These parsers own surface recognition and token-boundary discovery.  The
//! static-ability family consumes the facts to perform semantic validation and
//! construct runtime abilities.

use crate::types::CardType;

use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas,
};
use super::super::{leaf, primitives, static_keyword_line_shapes};

const MAY_CHOOSE_NOT_UNTAP_PREFIX: &[&str] = &["you", "may", "choose", "not", "to", "untap"];
const DURING_YOUR_UNTAP_STEP: &[&str] = &["during", "your", "untap", "step"];
const SURVEILLED_GRAVEYARD_PLAY_LIFE_COST: &[&str] = &[
    "you",
    "may",
    "play",
    "lands",
    "and",
    "cast",
    "spells",
    "from",
    "among",
    "cards",
    "in",
    "your",
    "graveyard",
    "youve",
    "surveilled",
    "this",
    "turn",
    "if",
    "you",
    "cast",
    "a",
    "spell",
    "this",
    "way",
    "you",
    "pay",
    "life",
    "equal",
    "to",
    "its",
    "mana",
    "value",
    "rather",
    "than",
    "paying",
    "its",
    "mana",
    "cost",
];
const SOURCE_LINKED_EXILE_CAST_PREFIX: &[&str] = &[
    "during", "each", "players", "turn", "that", "player", "may", "cast", "a", "spell", "from",
    "among", "the", "cards", "they", "dont", "own", "exiled", "with",
];
const ANY_MANA_CAST_SUFFIX: &[&str] = &[
    "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "it",
];
const CAST_CREATURE_THIS_WAY_HASTE: &[&str] = &[
    "if", "you", "cast", "a", "creature", "spell", "this", "way", "it", "gains", "haste", "until",
    "end", "of", "turn",
];
const CAST_THIS_WAY_ENTERS_TAPPED: &[&[&str]] = &[
    &[
        "if", "you", "cast", "a", "spell", "this", "way", "that", "artifact", "enters", "tapped",
    ],
    &[
        "if",
        "you",
        "cast",
        "a",
        "spell",
        "this",
        "way",
        "that",
        "permanent",
        "enters",
        "tapped",
    ],
    &[
        "if", "you", "cast", "a", "spell", "this", "way", "that", "creature", "enters", "tapped",
    ],
    &[
        "if", "you", "cast", "a", "spell", "this", "way", "it", "enters", "tapped",
    ],
    &["if", "you", "do", "it", "enters", "tapped"],
];
const CONTROL_OPPONENTS_WHILE_SEARCHING: &[&str] = &[
    "you",
    "control",
    "your",
    "opponents",
    "while",
    "theyre",
    "searching",
    "their",
    "libraries",
];
const OPPONENT_SEARCH_EXILE_FOUND_CARDS: &[&str] = &[
    "while",
    "an",
    "opponent",
    "is",
    "searching",
    "their",
    "library",
    "they",
    "exile",
    "each",
    "card",
    "they",
    "find",
    "you",
    "may",
    "play",
    "those",
    "cards",
    "for",
    "as",
    "long",
    "as",
    "they",
    "remain",
    "exiled",
    "and",
    "you",
    "may",
    "spend",
    "mana",
    "as",
    "though",
    "it",
    "were",
    "mana",
    "of",
    "any",
    "color",
    "to",
    "cast",
    "them",
];
const CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING: &[&str] = &[
    "while",
    "youre",
    "searching",
    "your",
    "library",
    "you",
    "may",
    "cast",
    "this",
    "card",
    "from",
    "your",
    "library",
];
const ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT: &[&[&str]] = &[
    &[
        "all",
        "creatures",
        "attack",
        "enchanted",
        "creatures",
        "controller",
        "each",
        "combat",
        "if",
        "able",
    ],
    &[
        "all",
        "creatures",
        "attack",
        "enchanted",
        "creature",
        "controller",
        "each",
        "combat",
        "if",
        "able",
    ],
];
const DRAW_REPLACEMENT_EXILE_TOP_PREFIX: &[&str] = &[
    "if", "you", "would", "draw", "a", "card", "exile", "the", "top",
];
const DRAW_REPLACEMENT_EXILE_TOP_TAIL: &[&str] = &[
    "of", "your", "library", "instead", "you", "may", "play", "those", "cards", "this", "turn",
];
const ACTIVATE_EACH_OF_THOSE_ONCE: &[&str] = &[
    "you",
    "may",
    "activate",
    "each",
    "of",
    "those",
    "abilities",
    "only",
    "once",
    "each",
    "turn",
];
const PAY_LIFE_ENTER_TAPPED_TAILS: &[&[&str]] = &[
    &["it", "enters", "tapped"],
    &["it", "enter", "tapped"],
    &["it", "enters", "the", "battlefield", "tapped"],
    &["it", "enter", "the", "battlefield", "tapped"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayChooseNotUntapFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub simple_source_subject: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackEachCombatFact<'a> {
    AttachedController,
    Subject(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetraceGrantFact {
    pub card_types: Vec<CardType>,
    /// "nonland permanent cards in your graveyard have retrace" (Six).
    pub nonland_permanents: bool,
    /// "During your turn, ..." scopes the grant to the controller's turn.
    pub during_your_turn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalDrawReplacementFact<'a> {
    pub condition_tokens: &'a [OwnedLexToken],
    pub draw_count: u32,
    pub life_loss: Option<u32>,
}

/// "If you would draw one or more cards, you draw that many cards plus one
/// instead." (Quantum Riddler)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawExtraCardsReplacementFact {
    pub extra: u32,
    /// "except the first one you draw in each of your draw steps"
    pub except_first_of_draw_step: bool,
    /// "one or more cards ... that many" applies per draw instruction;
    /// "a card ... two cards" applies to each card drawn.
    pub per_instruction: bool,
}

/// The player whose life change a doubling replacement watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeChangePlayerFact {
    You,
    Opponent,
    Any,
}

/// "If you would gain life, you gain twice that much life instead." /
/// "If an opponent would lose life during your turn, they lose twice that
/// much life instead."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoubleLifeChangeFact {
    pub player: LifeChangePlayerFact,
    pub loss: bool,
    pub during_your_turn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayLifeOrEnterTappedFact {
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayLifeOrEnterTappedError {
    MissingPay,
    UnsupportedPrefix,
    MissingAmount,
    MissingIfYouDont,
    UnsupportedTail,
}

/// "As this land enters, you may reveal a Plains or Island card from your
/// hand. If you don't, this land enters tapped." (shadow lands, snarls).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealCardOrEnterTappedFact<'a> {
    /// Authored subject phrase ("this land").
    pub subject: String,
    /// The revealed card's filter tokens ("a Plains or Island card").
    pub filter_tokens: &'a [OwnedLexToken],
    /// Authored tail subject phrase ("this land" or "it").
    pub tail_subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyActivatedAbilitiesFact {
    pub marker_token: usize,
    /// Word index of the `has`/`have` marker, for callers that render only the
    /// ability half of a grant ("<subject> have all activated abilities of …").
    pub marker_word_start: usize,
    pub filter_start_token: usize,
    pub filter_end_token: usize,
    pub only_loyalty: bool,
    pub once_each_turn_word_start: Option<usize>,
    pub exclude_source_name: bool,
}

pub fn parse_may_choose_not_untap_tokens(
    tokens: &[OwnedLexToken],
) -> Option<MayChooseNotUntapFact<'_>> {
    parse_semantic_all(tokens, parse_may_choose_not_untap_lexed)
}

fn parse_may_choose_not_untap_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<MayChooseNotUntapFact<'a>> {
    semantic_phrase(MAY_CHOOSE_NOT_UNTAP_PREFIX).parse_next(input)?;
    let subject_tokens = repeat_till(
        1..,
        any.void(),
        peek(semantic_phrase(DURING_YOUR_UNTAP_STEP)),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    semantic_phrase(DURING_YOUR_UNTAP_STEP).parse_next(input)?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let simple_source_subject = parse_semantic_all(
        subject_tokens,
        alt((
            semantic_phrase(&["this", "artifact"]),
            semantic_phrase(&["this", "creature"]),
            semantic_phrase(&["this", "land"]),
            semantic_phrase(&["this", "permanent"]),
            semantic_phrase(&["this", "card"]),
            semantic_kw("this"),
            semantic_kw("it"),
        )),
    )
    .is_some();
    Ok(MayChooseNotUntapFact {
        subject_tokens,
        simple_source_subject,
    })
}

pub fn is_surveilled_graveyard_play_life_cost(tokens: &[OwnedLexToken]) -> bool {
    parse_semantic_all(tokens, semantic_phrase(SURVEILLED_GRAVEYARD_PLAY_LIFE_COST)).is_some()
}

pub fn is_source_linked_exile_cast_with_any_mana(tokens: &[OwnedLexToken]) -> bool {
    parse_semantic_all(tokens, parse_source_linked_exile_cast_lexed).is_some()
}

fn parse_source_linked_exile_cast_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    semantic_phrase(SOURCE_LINKED_EXILE_CAST_PREFIX).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(semantic_phrase(ANY_MANA_CAST_SUFFIX)),
    )
    .void()
    .parse_next(input)?;
    semantic_phrase(ANY_MANA_CAST_SUFFIX).parse_next(input)
}

pub fn contains_singular_cast_spell(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || {
        alt((
            semantic_phrase(&["cast", "a", "spell"]),
            semantic_phrase(&["cast", "one", "spell"]),
        ))
    })
}

pub fn parse_play_permission_with_haste_followup(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let sentences = split_lexed_sentences(tokens);
    let [permission, followup] = sentences.as_slice() else {
        return None;
    };
    parse_semantic_all(followup, semantic_phrase(CAST_CREATURE_THIS_WAY_HASTE))
        .is_some()
        .then_some(*permission)
}

pub fn parse_play_permission_with_enter_tapped_followup(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let sentences = split_lexed_sentences(tokens);
    let [permission, followup] = sentences.as_slice() else {
        return None;
    };
    parse_semantic_all(
        followup,
        alt((
            semantic_phrase(CAST_THIS_WAY_ENTERS_TAPPED[0]),
            semantic_phrase(CAST_THIS_WAY_ENTERS_TAPPED[1]),
            semantic_phrase(CAST_THIS_WAY_ENTERS_TAPPED[2]),
            semantic_phrase(CAST_THIS_WAY_ENTERS_TAPPED[3]),
            semantic_phrase(CAST_THIS_WAY_ENTERS_TAPPED[4]),
        )),
    )
    .is_some()
    .then_some(*permission)
}

pub fn is_control_opponents_while_searching(tokens: &[OwnedLexToken]) -> bool {
    parse_semantic_all(tokens, semantic_phrase(CONTROL_OPPONENTS_WHILE_SEARCHING)).is_some()
}

pub fn is_opponent_search_exile_found_cards(tokens: &[OwnedLexToken]) -> bool {
    parse_semantic_all(tokens, semantic_phrase(OPPONENT_SEARCH_EXILE_FOUND_CARDS)).is_some()
}

pub fn is_cast_this_card_from_library_while_searching(tokens: &[OwnedLexToken]) -> bool {
    parse_semantic_all(
        tokens,
        semantic_phrase(CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING),
    )
    .is_some()
}

pub fn parse_attack_each_combat_if_able_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttackEachCombatFact<'_>> {
    parse_semantic_all(tokens, parse_attack_each_combat_if_able_lexed)
}

fn parse_attack_each_combat_if_able_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttackEachCombatFact<'a>> {
    alt((
        alt((
            semantic_phrase(ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT[0]),
            semantic_phrase(ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT[1]),
        ))
        .value(AttackEachCombatFact::AttachedController),
        parse_subject_attack_each_combat_lexed,
    ))
    .parse_next(input)
}

fn parse_subject_attack_each_combat_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttackEachCombatFact<'a>> {
    let subject_tokens = repeat_till(
        0..,
        any.void(),
        peek(alt((semantic_kw("attack"), semantic_kw("attacks")))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    alt((semantic_kw("attack"), semantic_kw("attacks"))).parse_next(input)?;
    semantic_phrase(&["each", "combat", "if", "able"]).parse_next(input)?;
    Ok(AttackEachCombatFact::Subject(trim_lexed_commas(
        subject_tokens,
    )))
}

pub fn parse_additional_land_play_count(tokens: &[OwnedLexToken]) -> Option<u32> {
    parse_semantic_all(tokens, parse_additional_land_play_lexed)
}

fn parse_additional_land_play_lexed(input: &mut LexStream<'_>) -> WResult<u32> {
    semantic_phrase(&["you", "may", "play"]).parse_next(input)?;
    opt(semantic_phrase(&["up", "to"])).parse_next(input)?;
    let count = semantic_number_token.parse_next(input)?;
    if count == 0 {
        return Err(primitives::backtrack_err(
            "additional land play count",
            "positive count",
        ));
    }
    semantic_kw("additional").parse_next(input)?;
    alt((semantic_kw("land"), semantic_kw("lands"))).parse_next(input)?;
    semantic_phrase(&["on", "each", "of", "your", "turns"]).parse_next(input)?;
    Ok(count)
}

pub fn parse_retrace_grant_tokens(tokens: &[OwnedLexToken]) -> Option<RetraceGrantFact> {
    parse_semantic_all(tokens, parse_retrace_grant_lexed)
}

fn parse_retrace_grant_lexed(input: &mut LexStream<'_>) -> WResult<RetraceGrantFact> {
    let during_your_turn = opt((
        semantic_phrase(&["during", "your", "turn"]),
        opt(primitives::comma()),
    ))
    .parse_next(input)?
    .is_some();
    opt(semantic_kw("each")).parse_next(input)?;
    if opt(semantic_phrase(&["nonland", "permanent"]))
        .parse_next(input)?
        .is_some()
    {
        alt((semantic_kw("cards"), semantic_kw("card"))).parse_next(input)?;
        semantic_phrase(&["in", "your", "graveyard", "have", "retrace"]).parse_next(input)?;
        return Ok(RetraceGrantFact {
            card_types: Vec::new(),
            nonland_permanents: true,
            during_your_turn,
        });
    }
    let (atoms, ()) = repeat_till::<_, _, Vec<Option<CardType>>, _, _, _, _>(
        1..,
        parse_retrace_subject_atom,
        peek(semantic_phrase(&["in", "your", "graveyard"])),
    )
    .parse_next(input)?;
    semantic_phrase(&["in", "your", "graveyard", "have", "retrace"]).parse_next(input)?;

    let mut card_types = Vec::new();
    for card_type in atoms.into_iter().flatten() {
        if card_types.iter().all(|existing| *existing != card_type) {
            card_types.push(card_type);
        }
    }
    if card_types.is_empty() {
        return Err(primitives::backtrack_err(
            "retrace grant subject",
            "instant or sorcery card type",
        ));
    }
    Ok(RetraceGrantFact {
        card_types,
        nonland_permanents: false,
        during_your_turn,
    })
}

fn parse_retrace_subject_atom(input: &mut LexStream<'_>) -> WResult<Option<CardType>> {
    alt((
        alt((semantic_kw("instant"), semantic_kw("instants"))).value(Some(CardType::Instant)),
        alt((semantic_kw("sorcery"), semantic_kw("sorceries"))).value(Some(CardType::Sorcery)),
        alt((
            semantic_kw("and"),
            semantic_kw("or"),
            semantic_kw("card"),
            semantic_kw("cards"),
        ))
        .value(None),
    ))
    .parse_next(input)
}

pub fn parse_draw_replacement_exile_top_and_play_count(tokens: &[OwnedLexToken]) -> Option<u32> {
    parse_semantic_all(tokens, parse_draw_replacement_exile_top_and_play_lexed)
}

fn parse_draw_replacement_exile_top_and_play_lexed(input: &mut LexStream<'_>) -> WResult<u32> {
    semantic_phrase(DRAW_REPLACEMENT_EXILE_TOP_PREFIX).parse_next(input)?;
    let count = semantic_number_token.parse_next(input)?;
    alt((semantic_kw("card"), semantic_kw("cards"))).parse_next(input)?;
    semantic_phrase(DRAW_REPLACEMENT_EXILE_TOP_TAIL).parse_next(input)?;
    Ok(count)
}

pub fn parse_conditional_draw_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalDrawReplacementFact<'_>> {
    parse_semantic_all(tokens, parse_conditional_draw_replacement_lexed)
}

fn parse_conditional_draw_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalDrawReplacementFact<'a>> {
    semantic_phrase(&["if", "you", "would", "draw"]).parse_next(input)?;
    opt(semantic_kw("a")).parse_next(input)?;
    semantic_phrase(&["card", "while"]).parse_next(input)?;
    let condition_tokens = repeat_till(1.., any.void(), peek(semantic_kw("instead")))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    semantic_kw("instead").parse_next(input)?;
    opt(semantic_kw("you")).parse_next(input)?;
    semantic_kw("draw").parse_next(input)?;
    let draw_count = semantic_number_token.parse_next(input)?;
    alt((semantic_kw("card"), semantic_kw("cards"))).parse_next(input)?;
    opt(semantic_kw("instead")).parse_next(input)?;
    let life_loss = opt((
        semantic_phrase(&["and", "you", "lose"]),
        semantic_number_token,
        semantic_kw("life"),
    )
        .map(|(_, amount, _)| amount))
    .parse_next(input)?;
    Ok(ConditionalDrawReplacementFact {
        condition_tokens: trim_lexed_commas(condition_tokens),
        draw_count,
        life_loss,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectDrawReplacementFact {
    pub drawer_is_opponent: bool,
    pub except_first_of_draw_step: bool,
}

/// "If an opponent would draw a card except the first one they draw in each
/// of their draw steps, instead that player skips that draw and you draw a
/// card." (Notion Thief)
pub fn parse_redirect_draw_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RedirectDrawReplacementFact> {
    parse_semantic_all(tokens, parse_redirect_draw_replacement_lexed)
}

fn parse_redirect_draw_replacement_lexed(
    input: &mut LexStream<'_>,
) -> WResult<RedirectDrawReplacementFact> {
    semantic_kw("if").parse_next(input)?;
    let drawer_is_opponent = alt((
        semantic_phrase(&["an", "opponent"]).value(true),
        semantic_phrase(&["a", "player"]).value(false),
    ))
    .parse_next(input)?;
    semantic_phrase(&["would", "draw", "a", "card"]).parse_next(input)?;
    let except_first_of_draw_step = opt(semantic_phrase(&[
        "except", "the", "first", "one", "they", "draw", "in", "each", "of", "their", "draw",
        "steps",
    ]))
    .parse_next(input)?
    .is_some();
    semantic_phrase(&[
        "instead", "that", "player", "skips", "that", "draw", "and", "you", "draw", "a", "card",
    ])
    .parse_next(input)?;
    Ok(RedirectDrawReplacementFact {
        drawer_is_opponent,
        except_first_of_draw_step,
    })
}

pub fn parse_draw_extra_cards_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DrawExtraCardsReplacementFact> {
    parse_semantic_all(tokens, parse_draw_extra_cards_replacement_lexed)
}

fn parse_draw_extra_cards_replacement_lexed(
    input: &mut LexStream<'_>,
) -> WResult<DrawExtraCardsReplacementFact> {
    semantic_phrase(&["if", "you", "would", "draw"]).parse_next(input)?;
    let per_instruction = alt((
        semantic_phrase(&["one", "or", "more", "cards"]).value(true),
        semantic_phrase(&["a", "card"]).value(false),
    ))
    .parse_next(input)?;
    let except_first_of_draw_step = opt(semantic_phrase(&[
        "except", "the", "first", "one", "you", "draw", "in", "each", "of", "your", "draw",
        "steps",
    ]))
    .parse_next(input)?
    .is_some();
    opt(semantic_kw("you")).parse_next(input)?;
    semantic_kw("draw").parse_next(input)?;
    let extra = alt((
        // "that many cards plus one instead" adds to the instruction.
        (
            semantic_phrase(&["that", "many", "cards", "plus"]),
            semantic_number_token,
        )
            .map(|(_, extra)| extra),
        // "two cards instead" replaces a single-card draw, so the extra is
        // the difference from one.
        (
            semantic_number_token,
            alt((semantic_kw("cards"), semantic_kw("card"))),
        )
            .map(|(count, _)| count.saturating_sub(1)),
    ))
    .parse_next(input)?;
    semantic_kw("instead").parse_next(input)?;
    Ok(DrawExtraCardsReplacementFact {
        extra,
        except_first_of_draw_step,
        per_instruction,
    })
}

pub fn parse_double_life_change_tokens(tokens: &[OwnedLexToken]) -> Option<DoubleLifeChangeFact> {
    parse_semantic_all(tokens, parse_double_life_change_lexed)
}

fn parse_double_life_change_lexed(input: &mut LexStream<'_>) -> WResult<DoubleLifeChangeFact> {
    semantic_kw("if").parse_next(input)?;
    let player = alt((
        semantic_kw("you").value(LifeChangePlayerFact::You),
        semantic_phrase(&["an", "opponent"]).value(LifeChangePlayerFact::Opponent),
        semantic_phrase(&["a", "player"]).value(LifeChangePlayerFact::Any),
    ))
    .parse_next(input)?;
    semantic_kw("would").parse_next(input)?;
    let loss = alt((
        semantic_kw("gain").value(false),
        semantic_kw("lose").value(true),
    ))
    .parse_next(input)?;
    semantic_kw("life").parse_next(input)?;
    let during_your_turn = opt(semantic_phrase(&["during", "your", "turn"]))
        .parse_next(input)?
        .is_some();
    alt((
        semantic_kw("you"),
        semantic_kw("they"),
        semantic_phrase(&["that", "player"]),
    ))
    .parse_next(input)?;
    alt((
        semantic_kw("gain"),
        semantic_kw("gains"),
        semantic_kw("lose"),
        semantic_kw("loses"),
    ))
    .parse_next(input)?;
    semantic_phrase(&["twice", "that", "much", "life", "instead"]).parse_next(input)?;
    Ok(DoubleLifeChangeFact {
        player,
        loss,
        during_your_turn,
    })
}

pub fn parse_pay_life_or_enter_tapped_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<PayLifeOrEnterTappedFact>, PayLifeOrEnterTappedError> {
    if !is_pay_life_candidate(tokens) {
        return Ok(None);
    }

    let Some(shape) = static_keyword_line_shapes::parse_pay_life_etb_shape(tokens) else {
        return Err(PayLifeOrEnterTappedError::MissingPay);
    };
    if !shape.saw_enter {
        return Ok(None);
    }
    // A leading sentence before the payment ("As this land enters, choose a
    // basic land type. Then you may pay 2 life...") carries its own ability;
    // the compound reading owns that line.
    if split_lexed_sentences(tokens)
        .first()
        .is_some_and(|first| first.len() <= shape.pay.token)
    {
        return Ok(None);
    }
    if !shape.saw_may {
        return Err(PayLifeOrEnterTappedError::UnsupportedPrefix);
    }
    let amount = leaf::parse_leaf_number_prefix_tokens(&tokens[shape.pay.token + 1..])
        .and_then(|number| number.into_fixed())
        .map(|(amount, _)| amount)
        .ok_or(PayLifeOrEnterTappedError::MissingAmount)?;

    let (_, (), tail_tokens) =
        primitives::find_prefix(tokens, || semantic_phrase(&["if", "you", "dont"]))
            .ok_or(PayLifeOrEnterTappedError::MissingIfYouDont)?;
    if primitives::parse_prefix(
        tail_tokens,
        alt((
            semantic_phrase(PAY_LIFE_ENTER_TAPPED_TAILS[0]),
            semantic_phrase(PAY_LIFE_ENTER_TAPPED_TAILS[1]),
            semantic_phrase(PAY_LIFE_ENTER_TAPPED_TAILS[2]),
            semantic_phrase(PAY_LIFE_ENTER_TAPPED_TAILS[3]),
        )),
    )
    .is_none()
    {
        return Err(PayLifeOrEnterTappedError::UnsupportedTail);
    }

    Ok(Some(PayLifeOrEnterTappedFact { amount }))
}

pub fn parse_reveal_card_or_enter_tapped_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealCardOrEnterTappedFact<'_>> {
    if !tokens_have_parser(tokens, || semantic_phrase(&["you", "may", "reveal"])) {
        return None;
    }
    let ((), after_as) = primitives::parse_prefix(tokens, semantic_phrase(&["as", "this"]))?;
    let noun = after_as.first()?.as_word()?;
    if matches!(noun, "enters" | "enter") {
        return None;
    }
    let ((), after_enters) = primitives::parse_prefix(
        after_as.get(1..)?,
        alt((
            semantic_phrase(&["enters", "the", "battlefield"]),
            semantic_phrase(&["enters"]),
        )),
    )?;
    let ((), after_reveal) =
        primitives::parse_prefix(after_enters, semantic_phrase(&["you", "may", "reveal"]))?;
    let (from_idx, (), after_from) =
        primitives::find_prefix(after_reveal, || semantic_phrase(&["from", "your", "hand"]))?;
    let filter_tokens = trim_lexed_commas(&after_reveal[..from_idx]);
    if filter_tokens.is_empty() {
        return None;
    }
    let ((), after_if) =
        primitives::parse_prefix(after_from, semantic_phrase(&["if", "you", "dont"]))?;
    let (tail_idx, (), after_tail) = primitives::find_prefix(after_if, || {
        alt((
            semantic_phrase(&["enters", "the", "battlefield", "tapped"]),
            semantic_phrase(&["enters", "tapped"]),
        ))
    })?;
    let tail_subject = trim_lexed_commas(&after_if[..tail_idx])
        .iter()
        .filter_map(|token| token.as_word())
        .collect::<Vec<_>>()
        .join(" ");
    let subject = format!("this {noun}");
    if tail_subject != "it" && tail_subject != subject {
        return None;
    }
    primitives::parse_prefix(after_tail, semantic_finish)?;
    Some(RevealCardOrEnterTappedFact {
        subject,
        filter_tokens,
        tail_subject,
    })
}

fn is_pay_life_candidate(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_phrase(&["as", "this"])).is_some()
        && tokens_have_parser(tokens, || semantic_kw("pay"))
        && tokens_have_parser(tokens, || semantic_kw("life"))
}

pub fn parse_copy_activated_abilities_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CopyActivatedAbilitiesFact> {
    let ((subject_tokens, only_loyalty), filter_tokens) =
        primitives::parse_prefix(tokens, parse_copy_activated_marker_prefix_lexed)?;
    let marker_token = subject_tokens.len();
    let filter_start_token = tokens.len().checked_sub(filter_tokens.len())?;
    let once_tail = primitives::find_prefix(filter_tokens, || {
        semantic_phrase(ACTIVATE_EACH_OF_THOSE_ONCE)
    });
    let once_each_turn_token_start =
        once_tail.map(|(relative, (), _)| filter_start_token + relative);
    let filter_end_token = once_each_turn_token_start.unwrap_or(tokens.len());
    let once_each_turn_word_start = once_each_turn_token_start
        .and_then(|token_start| semantic_word_count(&tokens[..token_start]));
    let exclude_source_name = tokens_have_parser(tokens, || {
        alt((
            semantic_phrase(&["same", "name", "as", "this", "creature"]),
            semantic_phrase(&["same", "name", "as", "thiss", "creature"]),
        ))
    });

    Some(CopyActivatedAbilitiesFact {
        marker_token,
        marker_word_start: semantic_word_count(&tokens[..marker_token])?,
        filter_start_token,
        filter_end_token,
        only_loyalty,
        once_each_turn_word_start,
        exclude_source_name,
    })
}

fn parse_copy_activated_marker_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], bool)> {
    let subject_tokens = repeat_till(0.., any.void(), peek(parse_copy_activated_marker_lexed))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let only_loyalty = parse_copy_activated_marker_lexed.parse_next(input)?;
    Ok((subject_tokens, only_loyalty))
}

fn parse_copy_activated_marker_lexed(input: &mut LexStream<'_>) -> WResult<bool> {
    alt((
        (
            alt((strict_kw("has"), strict_kw("have"))),
            semantic_phrase(&["all", "activated", "abilities", "of"]),
        )
            .value(false),
        (
            alt((strict_kw("has"), strict_kw("have"))),
            semantic_phrase(&["all", "loyalty", "abilities", "of"]),
        )
            .value(true),
    ))
    .parse_next(input)
}

fn semantic_word_count(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let counts = crate::grammar::primitives::take_leaf(
        &mut input,
        repeat::<_, _, Vec<usize>, ErrMode<ContextError>, _>(
            0..,
            any.map(|token: &OwnedLexToken| token.parser_word_pieces().len()),
        ),
    )?;
    crate::grammar::primitives::take_leaf(&mut input, semantic_finish)?;
    Some(counts.into_iter().sum())
}

fn parse_semantic_all<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> Option<O>
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    let output = crate::grammar::primitives::take_leaf(&mut input, parser)?;
    crate::grammar::primitives::take_leaf(&mut input, semantic_finish)?;
    Some(output)
}

fn tokens_have_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn semantic_number_token(input: &mut LexStream<'_>) -> WResult<u32> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    leaf::parse_leaf_number_prefix_lexed.parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        strict_kw(expected),
    )
        .void()
}

fn strict_kw<'a>(expected: &'static str) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    any.verify(move |token: &&OwnedLexToken| {
        token.is_word(expected)
            || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
    })
    .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise(input: &mut LexStream<'_>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| token.parser_word_pieces().is_empty())
        .void()
        .parse_next(input)
}

fn semantic_finish(input: &mut LexStream<'_>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    fn lex(line: &str) -> Vec<OwnedLexToken> {
        lex_line(line, 0).unwrap()
    }

    #[test]
    fn captures_untap_and_followup_permissions() {
        let tokens = lex("You may choose not to untap this creature during your untap step.");
        let parsed = parse_may_choose_not_untap_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.subject_tokens), "this creature");

        let tokens = lex(
            "You may cast creature spells from your graveyard. If you cast a creature spell this way, it gains haste until end of turn.",
        );
        let permission = parse_play_permission_with_haste_followup(&tokens).unwrap();
        assert_eq!(
            render_token_slice(permission),
            "You may cast creature spells from your graveyard"
        );
    }

    #[test]
    fn parses_attack_land_and_retrace_facts() {
        let tokens = lex("Goblins you control attack each combat if able.");
        let Some(AttackEachCombatFact::Subject(subject)) =
            parse_attack_each_combat_if_able_tokens(&tokens)
        else {
            panic!("expected subject attack fact");
        };
        assert_eq!(render_token_slice(subject), "Goblins you control");

        let tokens = lex("You may play up to two additional lands on each of your turns.");
        assert_eq!(parse_additional_land_play_count(&tokens), Some(2));

        let tokens = lex("Instant and sorcery cards in your graveyard have retrace.");
        assert_eq!(
            parse_retrace_grant_tokens(&tokens).unwrap().card_types,
            vec![CardType::Instant, CardType::Sorcery]
        );
    }

    #[test]
    fn parses_draw_replacement_facts() {
        let tokens = lex(
            "If you would draw a card, exile the top two cards of your library instead. You may play those cards this turn.",
        );
        assert_eq!(
            parse_draw_replacement_exile_top_and_play_count(&tokens),
            Some(2)
        );

        let tokens = lex(
            "If you would draw a card while you have no cards in hand, instead draw three cards and you lose 3 life.",
        );
        let parsed = parse_conditional_draw_replacement_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.condition_tokens),
            "you have no cards in hand"
        );
        assert_eq!(parsed.draw_count, 3);
        assert_eq!(parsed.life_loss, Some(3));
    }

    #[test]
    fn parses_pay_life_and_copy_ability_facts() {
        let tokens =
            lex("As this land enters, you may pay 2 life. If you don't, it enters tapped.");
        assert_eq!(
            parse_pay_life_or_enter_tapped_tokens(&tokens),
            Ok(Some(PayLifeOrEnterTappedFact { amount: 2 }))
        );

        let tokens = lex(
            "Creatures you control have all activated abilities of creatures with counters on them. You may activate each of those abilities only once each turn.",
        );
        let parsed = parse_copy_activated_abilities_tokens(&tokens).unwrap();
        assert!(!parsed.only_loyalty);
        assert!(parsed.once_each_turn_word_start.is_some());
        assert!(parsed.filter_end_token < tokens.len());
    }
}
