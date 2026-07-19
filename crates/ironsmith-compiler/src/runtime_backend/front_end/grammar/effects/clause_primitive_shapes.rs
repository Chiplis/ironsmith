use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{CardTextError, ClashOpponentAst, PlayerAst};
use crate::effect::Value;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenKind, TokenWordView};

#[path = "clause_primitive_shapes/combat_and_duration.rs"]
mod combat_and_duration;
pub(crate) use combat_and_duration::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyTargetsShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackRetargetFilterKind {
    ActivatedAbility,
    SpellOrAbility,
    Ability,
    InstantOrSorcery,
    Spell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StackRetargetFilterShape {
    pub(crate) kind: StackRetargetFilterKind,
    pub(crate) other: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseCardNameShape<'a> {
    pub(crate) player: PlayerAst,
    pub(crate) filter_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PowerDamageTargetShape<'a> {
    EachPlayer,
    EachOpponent,
    Source,
    Tokens(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PowerDamageShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) source_is_tagged: bool,
    pub(crate) amount: Value,
    pub(crate) target: PowerDamageTargetShape<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FightShape<'a> {
    pub(crate) left_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) right_tokens: &'a [OwnedLexToken],
    pub(crate) right_is_tagged_other: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetargetReferenceShape {
    Copy,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetargetConstraintShape {
    SingleTarget,
    SingleCreatureTarget,
    SourceOnlyTarget,
    YouOnlyTarget,
    AnyPlayerTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepeatProcessShape {
    Required,
    Once,
    May,
}

pub(super) fn trim_shape_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

fn exact_phrase(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_all(
        trim_shape_edges(tokens),
        (primitives::phrase(words), eof).void(),
        "clause primitive phrase",
    )
    .is_ok()
}

fn has_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word)).is_some()
}

pub(crate) fn parse_copy_targets_shape(tokens: &[OwnedLexToken]) -> Option<CopyTargetsShape<'_>> {
    let (_, target_tokens) = primitives::parse_prefix(
        trim_shape_edges(tokens),
        alt((
            primitives::phrase(&["the", "copy", "targets"]),
            primitives::phrase(&["that", "copy", "targets"]),
            primitives::phrase(&["copy", "targets"]),
        )),
    )?;
    Some(CopyTargetsShape {
        target_tokens: trim_shape_edges(target_tokens),
    })
}

pub(crate) fn parse_stack_retarget_filter_shape(
    tokens: &[OwnedLexToken],
) -> Option<StackRetargetFilterShape> {
    let has_ability = has_word(tokens, "ability") || has_word(tokens, "abilities");
    let has_spell = has_word(tokens, "spell") || has_word(tokens, "spells");
    let kind = if has_word(tokens, "activated") && has_ability {
        StackRetargetFilterKind::ActivatedAbility
    } else if has_ability && has_spell {
        StackRetargetFilterKind::SpellOrAbility
    } else if has_ability {
        StackRetargetFilterKind::Ability
    } else if (has_word(tokens, "instant") || has_word(tokens, "sorcery")) && has_spell {
        StackRetargetFilterKind::InstantOrSorcery
    } else if has_spell {
        StackRetargetFilterKind::Spell
    } else {
        return None;
    };
    Some(StackRetargetFilterShape {
        kind,
        other: has_word(tokens, "other"),
    })
}

fn choose_name_prefix<'a>(
    input: &mut crate::runtime_backend::lexer::LexStream<'a>,
) -> WResult<PlayerAst> {
    alt((
        primitives::phrase(&["that", "player", "chooses"]).value(PlayerAst::That),
        primitives::phrase(&["you", "choose"]).value(PlayerAst::You),
        primitives::kw("choose").value(PlayerAst::You),
    ))
    .parse_next(input)
}

pub(crate) fn parse_choose_card_name_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseCardNameShape<'_>> {
    let tokens = trim_shape_edges(tokens);
    let (player, tail) = primitives::parse_prefix(tokens, choose_name_prefix)?;
    // "choose the name of a nonland card revealed this way"
    if let Some(((), filter_tokens)) =
        primitives::parse_prefix(tail, primitives::phrase(&["the", "name", "of"]))
    {
        let filter_tokens = trim_shape_edges(filter_tokens);
        if filter_tokens.is_empty() {
            return None;
        }
        return Some(ChooseCardNameShape {
            player,
            filter_tokens: Some(filter_tokens),
        });
    }
    let (filter_tokens, ()) = primitives::split_lexed_once_before_suffix(tail, 0, || {
        (primitives::phrase(&["card", "name"]), eof).void()
    })?;
    let filter_tokens = trim_shape_edges(filter_tokens);
    let filter_tokens = if filter_tokens.is_empty()
        || exact_phrase(filter_tokens, &["any"])
        || exact_phrase(filter_tokens, &["a"])
        || exact_phrase(filter_tokens, &["an"])
    {
        None
    } else {
        Some(filter_tokens)
    };
    Some(ChooseCardNameShape {
        player,
        filter_tokens,
    })
}

pub(crate) fn is_each_player_exiles_hand_face_down_and_draws_shape(
    tokens: &[OwnedLexToken],
) -> bool {
    exact_phrase(
        tokens,
        &[
            "each", "player", "exiles", "all", "cards", "from", "their", "hand", "face", "down",
            "and", "draws", "seven", "cards",
        ],
    )
}

fn power_reference_word_count(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let count = alt((
        (
            alt((
                primitives::word_slice_exact("its"),
                primitives::word_slice_exact("that"),
            )),
            primitives::word_slice_exact("power"),
        )
            .value(2),
        (
            alt((
                primitives::word_slice_exact("this"),
                primitives::word_slice_exact("that"),
            )),
            alt((
                primitives::word_slice_exact("source"),
                primitives::word_slice_exact("creature"),
                primitives::word_slice_exact("objects"),
            )),
            primitives::word_slice_exact("power"),
        )
            .value(3),
    ))
    .parse_next(&mut input)
    .ok()?;
    Some(count)
}

pub(crate) fn is_it_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["it"])
}

fn value_references_power(value: &Value) -> bool {
    match value {
        Value::SourcePower | Value::PowerOf(_) => true,
        Value::Add(left, right) => value_references_power(left) || value_references_power(right),
        Value::Scaled(value, _) | Value::SurfaceHinted { value, .. } => {
            value_references_power(value)
        }
        _ => false,
    }
}

fn source_is_tagged(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["it"])
        || exact_phrase(tokens, &["that", "creature"])
        || exact_phrase(tokens, &["that", "permanent"])
        || exact_phrase(tokens, &["that", "card"])
        || TokenWordView::new(tokens)
            .to_word_refs()
            .starts_with(&["each", "of", "those"])
        || TokenWordView::new(tokens)
            .to_word_refs()
            .ends_with(&["tapped", "this", "way"])
}

fn target_shape(tokens: &[OwnedLexToken], allow_self: bool) -> PowerDamageTargetShape<'_> {
    let tokens = trim_shape_edges(tokens);
    if exact_phrase(tokens, &["each", "player"]) || exact_phrase(tokens, &["each", "players"]) {
        PowerDamageTargetShape::EachPlayer
    } else if exact_phrase(tokens, &["each", "opponent"])
        || exact_phrase(tokens, &["each", "opponents"])
        || exact_phrase(tokens, &["each", "other", "player"])
        || exact_phrase(tokens, &["each", "other", "players"])
    {
        PowerDamageTargetShape::EachOpponent
    } else if allow_self && (exact_phrase(tokens, &["itself"]) || exact_phrase(tokens, &["it"])) {
        PowerDamageTargetShape::Source
    } else {
        PowerDamageTargetShape::Tokens(tokens)
    }
}

pub(crate) fn parse_power_damage_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<PowerDamageShape<'_>>, CardTextError> {
    if primitives::find_prefix(tokens, || primitives::kw("divided").void()).is_some() {
        return Ok(None);
    }
    let tokens = trim_shape_edges(tokens);
    let Some((source_tokens, after_deal)) =
        primitives::split_lexed_once_on_separator(tokens, || {
            alt((primitives::kw("deal"), primitives::kw("deals"))).void()
        })
    else {
        return Ok(None);
    };
    let source_tokens = trim_shape_edges(source_tokens);
    if source_tokens.is_empty() {
        return Ok(None);
    }
    let after_deal = trim_shape_edges(after_deal);
    let Some((pre_equal, after_equal)) =
        primitives::split_lexed_once_on_separator(after_deal, || {
            primitives::phrase(&["equal", "to"]).void()
        })
    else {
        return Ok(None);
    };
    let pre_equal = trim_shape_edges(pre_equal);
    if !has_word(pre_equal, "damage") {
        return Ok(None);
    }
    let after_equal = trim_shape_edges(after_equal);
    let power_words = TokenWordView::new(after_equal);
    let word_refs = power_words.to_word_refs();
    let (amount, used_words) = if let Some((value, used)) =
        crate::runtime_backend::util::parse_value_expr_words(&word_refs)
        && value_references_power(&value)
    {
        (value, used)
    } else if let Some(used) = power_reference_word_count(&word_refs) {
        (
            Value::PowerOf(Box::new(crate::target::ChooseSpec::Source)),
            used,
        )
    } else {
        return Ok(None);
    };
    let tail_start = power_words
        .token_index_after_words(used_words)
        .unwrap_or(after_equal.len());
    let tail = trim_shape_edges(&after_equal[tail_start..]);

    let target = if exact_phrase(pre_equal, &["damage"]) {
        let target_tokens = primitives::parse_prefix(tail, primitives::kw("to"))
            .map(|(_, rest)| trim_shape_edges(rest))
            .unwrap_or(tail);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing damage target after power reference".to_string(),
            ));
        }
        let target_tokens = if let Some(((), after_each_of)) =
            primitives::parse_prefix(target_tokens, primitives::phrase(&["each", "of"]).void())
            && has_word(after_each_of, "target")
        {
            trim_shape_edges(after_each_of)
        } else {
            target_tokens
        };
        target_shape(target_tokens, false)
    } else if let Some(((), target_tokens)) =
        primitives::parse_prefix(pre_equal, primitives::phrase(&["damage", "to"]).void())
    {
        if !tail.is_empty() {
            return Err(CardTextError::ParseError(
                "unsupported trailing target after explicit power-damage target".to_string(),
            ));
        }
        target_shape(target_tokens, true)
    } else {
        return Ok(None);
    };

    Ok(Some(PowerDamageShape {
        source_tokens,
        source_is_tagged: source_is_tagged(source_tokens),
        amount,
        target,
    }))
}

pub(crate) fn parse_fight_shape(tokens: &[OwnedLexToken]) -> Option<FightShape<'_>> {
    let tokens = trim_shape_edges(tokens);
    let (left, right) = primitives::split_lexed_once_on_separator(tokens, || {
        alt((primitives::kw("fight"), primitives::kw("fights"))).void()
    })?;
    let left = trim_shape_edges(left);
    let right = trim_shape_edges(right);
    Some(FightShape {
        left_tokens: (!left.is_empty()).then_some(left),
        right_tokens: right,
        right_is_tagged_other: exact_phrase(right, &["each", "other"])
            || exact_phrase(right, &["one", "another"]),
    })
}

fn clash_opponent<'a>(
    input: &mut crate::runtime_backend::lexer::LexStream<'a>,
) -> WResult<ClashOpponentAst> {
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    )))
    .parse_next(input)?;
    alt((
        primitives::phrase(&["target", "opponent"]).value(ClashOpponentAst::TargetOpponent),
        primitives::phrase(&["defending", "player"]).value(ClashOpponentAst::DefendingPlayer),
        primitives::kw("opponent").value(ClashOpponentAst::Opponent),
    ))
    .parse_next(input)
}

pub(crate) fn parse_clash_shape(tokens: &[OwnedLexToken]) -> Option<ClashOpponentAst> {
    let tokens = trim_shape_edges(tokens);
    let (_, tail) = primitives::parse_prefix(
        tokens,
        (
            alt((primitives::kw("clash"), primitives::kw("clashes"))),
            opt(primitives::kw("with")),
        ),
    )?;
    let target_tokens = primitives::split_lexed_once_on_separator(tail, || {
        alt((primitives::kw("then").void(), primitives::comma().void()))
    })
    .map(|(head, _)| head)
    .unwrap_or(tail);
    primitives::parse_all(
        trim_shape_edges(target_tokens),
        (clash_opponent, eof).map(|(opponent, _)| opponent),
        "clash opponent",
    )
    .ok()
}

pub(crate) fn parse_retarget_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<RetargetReferenceShape> {
    let tokens = trim_shape_edges(tokens);
    if primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["the", "copy"]),
            primitives::phrase(&["the", "copies"]),
            primitives::phrase(&["that", "copy"]),
            primitives::phrase(&["those", "copies"]),
        )),
    )
    .is_some()
    {
        Some(RetargetReferenceShape::Copy)
    } else if primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("it").void(),
            primitives::kw("them").void(),
            primitives::phrase(&["the", "spell"]).void(),
            primitives::phrase(&["that", "spell"]).void(),
        )),
    )
    .is_some()
    {
        Some(RetargetReferenceShape::Other)
    } else {
        None
    }
}

pub(crate) fn parse_retarget_constraint_shapes(
    tokens: &[OwnedLexToken],
) -> Vec<RetargetConstraintShape> {
    let tokens = trim_shape_edges(tokens);
    let mut constraints = Vec::new();
    let candidates: &'static [(&'static [&'static str], RetargetConstraintShape)] = &[
        (
            &["with", "a", "single", "target"],
            RetargetConstraintShape::SingleTarget,
        ),
        (
            &["targets", "only", "a", "single", "creature"],
            RetargetConstraintShape::SingleCreatureTarget,
        ),
        (
            &["targets", "only", "this", "creature"],
            RetargetConstraintShape::SourceOnlyTarget,
        ),
        (
            &["targets", "only", "this", "permanent"],
            RetargetConstraintShape::SourceOnlyTarget,
        ),
        (
            &["targets", "only", "you"],
            RetargetConstraintShape::YouOnlyTarget,
        ),
        (
            &["targets", "only", "a", "player"],
            RetargetConstraintShape::AnyPlayerTarget,
        ),
        (
            &["if", "that", "target", "is", "you"],
            RetargetConstraintShape::YouOnlyTarget,
        ),
    ];
    for &(phrase, constraint) in candidates {
        if primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some() {
            constraints.push(constraint);
        }
    }
    constraints
}

fn parse_repeat_process<'a>(
    input: &mut crate::runtime_backend::lexer::LexStream<'a>,
) -> WResult<(bool, RepeatProcessShape)> {
    opt(primitives::kw("and")).parse_next(input)?;
    let explicit_may = opt(primitives::phrase(&["you", "may"]))
        .parse_next(input)?
        .is_some();
    primitives::phrase(&["repeat", "this", "process"]).parse_next(input)?;
    let shape = alt((
        primitives::phrase(&["any", "number", "of", "times"]).value(RepeatProcessShape::May),
        primitives::kw("once").value(RepeatProcessShape::Once),
        eof.value(RepeatProcessShape::Required),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((explicit_may, shape))
}

pub(crate) fn parse_repeat_process_shape(tokens: &[OwnedLexToken]) -> Option<RepeatProcessShape> {
    let (explicit_may, shape) = primitives::parse_all(
        trim_shape_edges(tokens),
        parse_repeat_process,
        "repeat process clause",
    )
    .ok()?;
    match (explicit_may, shape) {
        (true, RepeatProcessShape::Required | RepeatProcessShape::May) => {
            Some(RepeatProcessShape::May)
        }
        (false, shape) => Some(shape),
        (true, RepeatProcessShape::Once) => None,
    }
}

pub(crate) fn is_dont_lose_mana_between_steps_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(
        tokens,
        &[
            "you", "dont", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
        ],
    )
}

#[cfg(test)]
#[path = "clause_primitive_shapes/tests.rs"]
mod tests;
