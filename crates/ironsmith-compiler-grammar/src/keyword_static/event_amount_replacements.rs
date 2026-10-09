//! Amount-modifying replacements (CR 614.1a, 616.1): the watched event still
//! happens, with a different number.
//!
//! - "If an opponent would mill one or more cards, they mill twice that many
//!   cards instead." (Bruvac the Grandiloquent)
//! - "If you would scry a number of cards, scry that many cards plus one
//!   instead." (Kenessos, Priest of Thassa)
//! - "You may look at an additional two cards each time you surveil."
//!   (Enhanced Surveillance)
//! - "If a source would deal 4 or more damage to a permanent or player, that
//!   source deals 3 damage to that permanent or player instead." (Divine
//!   Presence, Forethought Amulet)
//! - "If a source would deal damage to a permanent or player, it deals half
//!   that damage, rounded down, to that permanent or player instead." (Ghosts
//!   of the Innocent)
//!
//! "If you would scry a number of cards, draw that many cards instead."
//! (Eligeth) replaces the scry with a program; it is read here too because it
//! shares the scry header, and becomes a keyword-action instead replacement.

use super::*;
use crate::cards::builders::{
    EffectAst, LifeResourceActionAst, PlayerAst, SubjectVerbActionAst, SubjectVerbRoleAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::events::KeywordActionKind;
use crate::lexer::LexStream;
use ironsmith_core::{AmountEventSpec, AmountModifierSpec};
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;


/// The keyword actions whose magnitude a player-scoped replacement modifies.
fn keyword_action_verb<'a>(input: &mut LexStream<'a>) -> WResult<KeywordActionKind> {
    alt((
        alt((
            crate::grammar::primitives::kw("mill"),
            crate::grammar::primitives::kw("mills"),
        ))
        .value(KeywordActionKind::Mill),
        alt((
            crate::grammar::primitives::kw("scry"),
            crate::grammar::primitives::kw("scries"),
        ))
        .value(KeywordActionKind::Scry),
        alt((
            crate::grammar::primitives::kw("surveil"),
            crate::grammar::primitives::kw("surveils"),
        ))
        .value(KeywordActionKind::Surveil),
    ))
    .parse_next(input)
}

/// "you", "an opponent", "a player".
fn performer<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        crate::grammar::primitives::kw("you").value(PlayerFilter::You),
        crate::grammar::primitives::phrase(&["an", "opponent"]).value(PlayerFilter::Opponent),
        crate::grammar::primitives::phrase(&["a", "player"]).value(PlayerFilter::Any),
    ))
    .parse_next(input)
}

/// "one or more cards", "a number of cards": the whole instruction's number
/// (CR 701.17, 701.22), never one card at a time.
fn instruction_count<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        crate::grammar::primitives::phrase(&["one", "or", "more", "cards"]),
        crate::grammar::primitives::phrase(&["a", "number", "of", "cards"]),
    ))
    .parse_next(input)
}

/// The pronoun repeating the performer ("they mill", "you scry"), when any.
fn repeated_performer<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(alt((
        crate::grammar::primitives::kw("they"),
        crate::grammar::primitives::kw("you"),
        crate::grammar::primitives::kw("that"),
    ))
    .void())
    .parse_next(input)?;
    opt(crate::grammar::primitives::kw("player").void()).parse_next(input)?;
    Ok(())
}

/// "twice that many cards", "three times that many cards", "that many cards
/// plus one", "that many plus one cards".
fn count_modifier<'a>(input: &mut LexStream<'a>) -> WResult<AmountModifierSpec> {
    alt((
        (
            crate::grammar::primitives::kw("twice"),
            crate::grammar::primitives::phrase(&["that", "many", "cards"]),
        )
            .value(AmountModifierSpec::Multiply(2)),
        (
            crate::grammar::primitives::number_token,
            crate::grammar::primitives::phrase(&["times", "that", "many", "cards"]),
        )
            .map(|(factor, ())| AmountModifierSpec::Multiply(factor)),
        (
            crate::grammar::primitives::phrase(&["that", "many", "cards", "plus"]),
            crate::grammar::primitives::number_token,
        )
            .map(|((), additional)| AmountModifierSpec::Add(additional)),
        (
            crate::grammar::primitives::phrase(&["that", "many", "plus"]),
            crate::grammar::primitives::number_token,
            crate::grammar::primitives::kw("cards"),
        )
            .map(|((), additional, _)| AmountModifierSpec::Add(additional)),
    ))
    .parse_next(input)
}

/// What a keyword-action header reads as: a modified number, or the draw
/// program that replaces the action.
#[derive(Clone, Copy)]
enum KeywordActionReading {
    Amount(AmountModifierSpec),
    DrawThatMany,
}

/// "If <performer> would <mill|scry|surveil> <count>, [<pronoun>] <same verb>
/// <modified count> instead." / "..., draw that many cards instead."
fn keyword_action_line<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(KeywordActionKind, PlayerFilter, KeywordActionReading)> {
    crate::grammar::primitives::kw("if").parse_next(input)?;
    let performer = performer.parse_next(input)?;
    crate::grammar::primitives::kw("would").parse_next(input)?;
    let action = keyword_action_verb.parse_next(input)?;
    instruction_count.parse_next(input)?;
    crate::grammar::primitives::comma().parse_next(input)?;
    let reading = alt((
        (
            crate::grammar::primitives::phrase(&["draw", "that", "many", "cards"]),
            crate::grammar::primitives::kw("instead"),
        )
            .value(KeywordActionReading::DrawThatMany),
        (
            repeated_performer,
            keyword_action_verb.verify(move |verb: &KeywordActionKind| *verb == action),
            count_modifier,
            crate::grammar::primitives::kw("instead"),
        )
            .map(|(_, _, modifier, _)| KeywordActionReading::Amount(modifier)),
    ))
    .parse_next(input)?;
    crate::grammar::primitives::sentence_end().parse_next(input)?;
    Ok((action, performer, reading))
}

/// "You may look at an additional two cards each time you surveil."
fn additional_looked_cards_line<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(KeywordActionKind, u32)> {
    crate::grammar::primitives::phrase(&["you", "may", "look", "at", "an", "additional"])
        .parse_next(input)?;
    let additional = crate::grammar::primitives::number_token.parse_next(input)?;
    alt((
        crate::grammar::primitives::kw("cards"),
        crate::grammar::primitives::kw("card"),
    ))
    .parse_next(input)?;
    crate::grammar::primitives::phrase(&["each", "time", "you"]).parse_next(input)?;
    let action = keyword_action_verb
        .verify(|verb: &KeywordActionKind| {
            matches!(*verb, KeywordActionKind::Scry | KeywordActionKind::Surveil)
        })
        .parse_next(input)?;
    crate::grammar::primitives::sentence_end().parse_next(input)?;
    Ok((action, additional))
}

/// "If you would copy a spell one or more times, instead copy it that many
/// times plus an additional time. You may choose new targets for the
/// additional copy." (Twinning Staff, CR 707.10): the engine offers new
/// targets for each copy the replacement adds.
fn spell_copy_count_line<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    crate::grammar::primitives::phrase(&[
        "if", "you", "would", "copy", "a", "spell", "one", "or", "more", "times",
    ])
    .parse_next(input)?;
    crate::grammar::primitives::comma().parse_next(input)?;
    crate::grammar::primitives::phrase(&[
        "instead", "copy", "it", "that", "many", "times", "plus",
    ])
    .parse_next(input)?;
    let additional = alt((
        crate::grammar::primitives::phrase(&["an", "additional", "time"]).value(1u32),
        (
            crate::grammar::primitives::number_token,
            crate::grammar::primitives::phrase(&["additional", "times"]),
        )
            .map(|(additional, ())| additional),
    ))
    .parse_next(input)?;
    crate::grammar::primitives::period().parse_next(input)?;
    crate::grammar::primitives::phrase(&[
        "you", "may", "choose", "new", "targets", "for", "the", "additional",
    ])
    .parse_next(input)?;
    alt((
        crate::grammar::primitives::kw("copy"),
        crate::grammar::primitives::kw("copies"),
    ))
    .parse_next(input)?;
    crate::grammar::primitives::sentence_end().parse_next(input)?;
    Ok(additional)
}

/// The tokens of one phrase, up to (not including) the given phrase.
fn phrase_until<'a>(
    input: &mut LexStream<'a>,
    terminator: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(crate::grammar::primitives::phrase(terminator)),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)
}

/// The tokens up to the next comma.
fn phrase_until_comma<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(crate::grammar::primitives::comma()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

/// The tokens up to "instead" at the end of the sentence.
fn phrase_until_instead<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            crate::grammar::primitives::kw("instead"),
            crate::grammar::primitives::sentence_end(),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)
}

/// One damage amount header and body, as token slices.
struct DamageAmountShape<'a> {
    /// The source description between "if" and "would" ("a source", "an
    /// instant or sorcery source").
    source: &'a [OwnedLexToken],
    minimum: Option<u32>,
    recipient: &'a [OwnedLexToken],
    modifier: AmountModifierSpec,
    repeated_recipient: &'a [OwnedLexToken],
}

/// "<that source|it> deals <N> damage to" / "it deals half that damage,
/// rounded down, to".
fn damage_body_modifier<'a>(input: &mut LexStream<'a>) -> WResult<AmountModifierSpec> {
    alt((
        crate::grammar::primitives::phrase(&["that", "source", "deals"]),
        crate::grammar::primitives::phrase(&["it", "deals"]),
    ))
    .parse_next(input)?;
    let modifier = alt((
        (
            crate::grammar::primitives::number_token,
            crate::grammar::primitives::kw("damage"),
        )
            .map(|(amount, _)| AmountModifierSpec::SetTo(amount)),
        (
            crate::grammar::primitives::phrase(&["half", "that", "damage"]),
            opt(crate::grammar::primitives::comma()),
            crate::grammar::primitives::kw("rounded"),
            alt((
                crate::grammar::primitives::kw("down").value(false),
                crate::grammar::primitives::kw("up").value(true),
            )),
            opt(crate::grammar::primitives::comma()),
        )
            .map(|(_, _, _, round_up, _)| AmountModifierSpec::Half { round_up }),
    ))
    .parse_next(input)?;
    crate::grammar::primitives::kw("to").parse_next(input)?;
    Ok(modifier)
}

fn damage_amount_line<'a>(input: &mut LexStream<'a>) -> WResult<DamageAmountShape<'a>> {
    crate::grammar::primitives::kw("if").parse_next(input)?;
    let source = phrase_until(input, &["would", "deal"])?;
    crate::grammar::primitives::phrase(&["would", "deal"]).parse_next(input)?;
    let minimum = opt((
        crate::grammar::primitives::number_token,
        crate::grammar::primitives::phrase(&["or", "more"]),
    )
        .map(|(minimum, ())| minimum))
    .parse_next(input)?;
    crate::grammar::primitives::phrase(&["damage", "to"]).parse_next(input)?;
    let recipient = phrase_until_comma(input)?;
    crate::grammar::primitives::comma().parse_next(input)?;
    let modifier = damage_body_modifier.parse_next(input)?;
    let repeated_recipient = phrase_until_instead(input)?;
    crate::grammar::primitives::kw("instead").parse_next(input)?;
    crate::grammar::primitives::sentence_end().parse_next(input)?;
    Ok(DamageAmountShape {
        source,
        minimum,
        recipient,
        modifier,
        repeated_recipient,
    })
}

fn amount_display(tokens: &[OwnedLexToken]) -> String {
    let mut display = render_token_slice(tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }
    display
}

/// The damage source a header names: "a source" / "any source" is every
/// source; "<description> source" restricts it ("an instant or sorcery
/// source").
fn damage_source_filter(
    source: &[OwnedLexToken],
) -> Result<Option<Option<ObjectFilter>>, CardTextError> {
    let words = parser_token_word_refs(source);
    let Some((last, description)) = words.split_last() else {
        return Ok(None);
    };
    if *last != "source" {
        return Ok(None);
    }
    let description = strip_leading_word_refs_any(description, &["a", "an", "any"]);
    if description.is_empty() {
        return Ok(Some(None));
    }
    // The description is the source's leading words; map them back onto the
    // header tokens so the object-filter grammar reads real tokens.
    let start = source.len().saturating_sub(description.len() + 1);
    let description_tokens = &source[start..source.len() - 1];
    if parser_token_word_refs(description_tokens) != description {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(description_tokens, false)?;
    if filter == ObjectFilter::default() {
        return Ok(None);
    }
    Ok(Some(Some(filter)))
}

/// "If <performer> would <keyword action> ..., <modified action> instead."
/// and the damage set/halve forms. See the module documentation.
pub fn parse_if_event_would_happen_amount_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if let Some(additional) = crate::grammar::primitives::probe_all(
        &tokens,
        spell_copy_count_line,
        "spell copy count replacement",
    ) {
        return Ok(Some(StaticAbility::event_amount_replacement(
            AmountEventSpec::KeywordAction {
                action: KeywordActionKind::CopySpell,
                performer: PlayerFilter::You,
            },
            AmountModifierSpec::Add(additional),
            false,
            amount_display(&tokens),
        )));
    }
    if let Some((action, performer, reading)) = crate::grammar::primitives::probe_all(
        &tokens,
        keyword_action_line,
        "keyword-action amount replacement",
    ) {
        let display = amount_display(&tokens);
        return Ok(Some(match reading {
            KeywordActionReading::Amount(modifier) => StaticAbility::event_amount_replacement(
                AmountEventSpec::KeywordAction { action, performer },
                modifier,
                false,
                display,
            ),
            // "draw that many cards instead": the replaced scry's number,
            // read from the replaced keyword-action event (CR 614.6).
            KeywordActionReading::DrawThatMany => {
                StaticAbility::keyword_action_replacement_for_player(
                    action,
                    performer,
                    vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::You,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                            count: Value::EventValue(EventValueSpec::Amount),
                        }),
                    )],
                    display,
                )
            }
        }));
    }
    let Some(shape) =
        crate::grammar::primitives::probe_all(&tokens, damage_amount_line, "damage amount replacement")
    else {
        return Ok(None);
    };
    let Some(source_filter) = damage_source_filter(shape.source)? else {
        return Ok(None);
    };
    let recipient_words = parser_token_word_refs(shape.recipient);
    let (player, object) = super::parse_damage_amount_replacement_target_filters(&recipient_words)?;
    if player.is_none() && object.is_none() {
        return Ok(None);
    }
    // The body names the same recipients again: "that permanent or player",
    // or the same words ("to you").
    let repeated = parser_token_word_refs(shape.repeated_recipient);
    let demonstrative = match repeated.as_slice() {
        ["that", "permanent", "or", "player"] | ["that", "player", "or", "permanent"] => {
            player.is_some() && object.is_some()
        }
        ["that", "player"] => player.is_some() && object.is_none(),
        ["that", "permanent"] | ["that", "creature"] => object.is_some() && player.is_none(),
        _ => repeated == recipient_words,
    };
    if !demonstrative {
        return Ok(None);
    }
    // "deals 3 damage instead" for "4 or more damage" is a set-to; without a
    // minimum the set-to would also raise smaller amounts, which no card says.
    if matches!(shape.modifier, AmountModifierSpec::SetTo(_)) && shape.minimum.is_none() {
        return Ok(None);
    }
    Ok(Some(StaticAbility::event_amount_replacement(
        AmountEventSpec::Damage {
            source_filter,
            player,
            object,
            combat_only: false,
            minimum: shape.minimum,
        },
        shape.modifier,
        false,
        amount_display(&tokens),
    )))
}

/// "You may look at an additional two cards each time you surveil."
/// (Enhanced Surveillance): an optional additive replacement on the
/// controller's surveil (CR 614.1a, 701.25).
pub fn parse_you_may_look_at_additional_cards_each_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((action, additional)) = crate::grammar::primitives::probe_all(
        &tokens,
        additional_looked_cards_line,
        "additional looked cards replacement",
    ) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::event_amount_replacement(
        AmountEventSpec::KeywordAction {
            action,
            performer: PlayerFilter::You,
        },
        AmountModifierSpec::Add(additional),
        true,
        amount_display(&tokens),
    )))
}

/// "If you control a creature, damage that would reduce your life total to
/// less than 1 reduces it to 1 instead." (Worship): a static life floor for
/// damage (CR 614.1a). The damage is still dealt — lifelink and other
/// results see its full amount — and only the life total is floored, through
/// the shared `DamageReduceLifeBelowOne` rule the resolving form (Angel's
/// Grace) already uses. A leading "if" condition gates the static ability.
pub fn parse_damage_life_floor_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    const CLAUSE: &[&str] = &[
        "damage", "that", "would", "reduce", "your", "life", "total", "to", "less", "than", "1",
        "reduces", "it", "to", "1", "instead",
    ];
    let tokens = trim_edge_punctuation(tokens);
    let (condition_tokens, clause_tokens) = match tokens.first() {
        Some(first) if first.is_word("if") => {
            let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else {
                return Ok(None);
            };
            (Some(&tokens[1..comma]), &tokens[comma + 1..])
        }
        _ => (None, &tokens[..]),
    };
    if parser_token_word_refs(clause_tokens).as_slice() != CLAUSE {
        return Ok(None);
    }
    let ability = StaticAbility::restriction(
        crate::effect::Restriction::damage_reduce_life_below_one(PlayerFilter::You),
        amount_display(&tokens),
    );
    Ok(Some(match condition_tokens {
        Some(condition) if !condition.is_empty() => {
            ability.with_condition(parse_static_condition_clause(condition)?)
        }
        Some(_) => return Ok(None),
        None => ability,
    }))
}

/// "Polukranos enters with six +1/+1 counters on it. It escapes with twelve
/// +1/+1 counters on it instead." (Polukranos, Unchained): the escaped entry
/// replaces the ordinary one (CR 702.138c, 614.1c). Read as two entry
/// replacements: the first unless it escaped, the second if it did.
pub fn parse_enters_or_escapes_instead_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [enters, escapes] = sentences.as_slice() else {
        return Ok(None);
    };
    let escapes = trim_edge_punctuation(escapes);
    let Some((last, escapes_body)) = escapes.split_last() else {
        return Ok(None);
    };
    if !last.is_word("instead")
        || !escapes_body.iter().any(|token| token.is_word("escapes"))
        || !enters.iter().any(|token| token.is_word("enters"))
    {
        return Ok(None);
    }
    let enters = trim_edge_punctuation(enters);
    let mut unless_escaped = enters.clone();
    let span = enters.last().map(OwnedLexToken::span).unwrap_or_else(TextSpan::synthetic);
    for word in ["unless", "it", "escaped"] {
        unless_escaped.push(OwnedLexToken::word(word.to_string(), span));
    }
    let Some(mut abilities) = super::parse_enters_with_counters_line(&unless_escaped)? else {
        return Ok(None);
    };
    let Some(escaped) = super::parse_enters_with_counters_line(escapes_body)? else {
        return Ok(None);
    };
    abilities.extend(escaped);
    Ok(Some(abilities))
}
