use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{CardTextError, ClashOpponentAst, PlayerAst};
use crate::effect::Value;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView};

#[path = "clause_primitive_shapes/combat_and_duration.rs"]
mod combat_and_duration;
pub use combat_and_duration::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyTargetsShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackRetargetFilterKind {
    ActivatedAbility,
    SpellOrAbility,
    Ability,
    InstantOrSorcery,
    Spell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackRetargetFilterShape {
    pub kind: StackRetargetFilterKind,
    pub other: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseCardNameShape<'a> {
    pub player: PlayerAst,
    pub filter_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerDamageTargetShape<'a> {
    EachPlayer,
    EachOtherPlayer,
    EachOpponent,
    Source,
    Tokens(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerDamageShape<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub source_is_tagged: bool,
    pub amount: Value,
    pub target: PowerDamageTargetShape<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightShape<'a> {
    pub left_tokens: Option<&'a [OwnedLexToken]>,
    pub right_tokens: &'a [OwnedLexToken],
    pub right_is_tagged_other: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetargetReferenceShape {
    Copy,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetargetConstraintShape {
    SingleTarget,
    SingleCreatureTarget,
    SourceOnlyTarget,
    YouOnlyTarget,
    AnyPlayerTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatProcessShape {
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

pub fn parse_copy_targets_shape(tokens: &[OwnedLexToken]) -> Option<CopyTargetsShape<'_>> {
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

pub fn parse_stack_retarget_filter_shape(
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

fn choose_name_prefix<'a>(input: &mut crate::lexer::LexStream<'a>) -> WResult<PlayerAst> {
    alt((
        primitives::phrase(&["that", "player", "chooses"]).value(PlayerAst::That),
        primitives::phrase(&["you", "choose"]).value(PlayerAst::You),
        primitives::kw("choose").value(PlayerAst::You),
    ))
    .parse_next(input)
}

pub fn parse_choose_card_name_shape(tokens: &[OwnedLexToken]) -> Option<ChooseCardNameShape<'_>> {
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

pub fn is_each_player_exiles_hand_face_down_and_draws_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(
        tokens,
        &[
            "each", "player", "exiles", "all", "cards", "from", "their", "hand", "face", "down",
            "and", "draws", "seven", "cards",
        ],
    )
}

fn characteristic_reference_word_count(words: &[&str]) -> Option<(usize, bool)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let (count, toughness) = alt((
        (
            alt((
                primitives::word_slice_exact("its"),
                primitives::word_slice_exact("that"),
            )),
            alt((
                primitives::word_slice_exact("power").value(false),
                primitives::word_slice_exact("toughness").value(true),
            )),
        )
            .map(|(_, toughness)| (2, toughness)),
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
            alt((
                primitives::word_slice_exact("power").value(false),
                primitives::word_slice_exact("toughness").value(true),
            )),
        )
            .map(|(_, _, toughness)| (3, toughness)),
    ))
    .parse_next(&mut input)
    .ok()?;
    Some((count, toughness))
}

pub fn is_it_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["it"])
}

fn value_references_power_or_toughness(value: &Value) -> bool {
    match value {
        Value::SourcePower | Value::SourceToughness | Value::PowerOf(_) | Value::ToughnessOf(_) => {
            true
        }
        Value::Add(left, right) => {
            value_references_power_or_toughness(left) || value_references_power_or_toughness(right)
        }
        Value::Scaled(value, _) | Value::SurfaceHinted { value, .. } => {
            value_references_power_or_toughness(value)
        }
        _ => false,
    }
}

/// Within a grammatical "A deals damage equal to its ..." clause, the
/// possessive belongs to `A`, even when the amount is an arithmetic
/// expression. The general value grammar intentionally leaves `its` as an
/// antecedent tag; convert only characteristic leaves in this local clause
/// while preserving their authored surface hints and all surrounding math.
fn bind_damage_source_possessive_characteristic(value: Value) -> Value {
    fn local_source_spec(spec: crate::target::ChooseSpec) -> crate::target::ChooseSpec {
        if matches!(
            spec.base(),
            crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ) && matches!(
            spec.source_reference_surface(),
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
                if matches!(surface.as_str(), "it" | "its")
        ) {
            crate::target::ChooseSpec::Source.with_surface_hints(spec.surface_hints().to_vec())
        } else {
            spec
        }
    }

    match value {
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_damage_source_possessive_characteristic(*value)),
            hints,
        },
        Value::Add(left, right) => Value::Add(
            Box::new(bind_damage_source_possessive_characteristic(*left)),
            Box::new(bind_damage_source_possessive_characteristic(*right)),
        ),
        Value::Scaled(value, scale) => Value::Scaled(
            Box::new(bind_damage_source_possessive_characteristic(*value)),
            scale,
        ),
        Value::DividedRoundedDown(value, divisor) => Value::DividedRoundedDown(
            Box::new(bind_damage_source_possessive_characteristic(*value)),
            divisor,
        ),
        Value::HalfRoundedDown(value) => Value::HalfRoundedDown(Box::new(
            bind_damage_source_possessive_characteristic(*value),
        )),
        Value::Min(left, right) => Value::Min(
            Box::new(bind_damage_source_possessive_characteristic(*left)),
            Box::new(bind_damage_source_possessive_characteristic(*right)),
        ),
        Value::PowerOf(spec) => Value::PowerOf(Box::new(local_source_spec(*spec))),
        Value::ToughnessOf(spec) => Value::ToughnessOf(Box::new(local_source_spec(*spec))),
        value => value,
    }
}

fn source_is_tagged(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["it"])
        || exact_phrase(tokens, &["that", "creature"])
        || exact_phrase(tokens, &["that", "permanent"])
        || exact_phrase(tokens, &["that", "card"])
        || crate::word_primitives::parse_sequence_prefix(
            &TokenWordView::new(tokens).to_word_refs(),
            &["each", "of", "those"],
        )
        || crate::word_primitives::parse_sequence_suffix(
            &TokenWordView::new(tokens).to_word_refs(),
            &["tapped", "this", "way"],
        )
}

fn target_shape(tokens: &[OwnedLexToken], allow_self: bool) -> PowerDamageTargetShape<'_> {
    let tokens = trim_shape_edges(tokens);
    if exact_phrase(tokens, &["each", "player"]) || exact_phrase(tokens, &["each", "players"]) {
        PowerDamageTargetShape::EachPlayer
    } else if exact_phrase(tokens, &["each", "other", "player"])
        || exact_phrase(tokens, &["each", "other", "players"])
    {
        PowerDamageTargetShape::EachOtherPlayer
    } else if exact_phrase(tokens, &["each", "opponent"])
        || exact_phrase(tokens, &["each", "opponents"])
    {
        PowerDamageTargetShape::EachOpponent
    } else if allow_self && (exact_phrase(tokens, &["itself"]) || exact_phrase(tokens, &["it"])) {
        PowerDamageTargetShape::Source
    } else {
        PowerDamageTargetShape::Tokens(tokens)
    }
}

pub fn parse_power_damage_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<PowerDamageShape<'_>>, CardTextError> {
    if tokens.iter().filter(|token| token.is_any_word(&["deal", "deals"])).count() > 1 {
        return Ok(None);
    }
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
    if source_tokens.is_empty()
        || super::chain_splitting::has_authored_comma_then_surface_tokens(source_tokens)
    {
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
    let (amount, used_words) = if crate::word_primitives::parse_any_sequence_prefix(
        &word_refs,
        &[&["its", "power"], &["its", "toughness"]],
    ) {
        let Some((value, used)) = crate::util::parse_value_expr_words(&word_refs) else {
            return Ok(None);
        };
        if !value_references_power_or_toughness(&value) {
            return Ok(None);
        }
        (bind_damage_source_possessive_characteristic(value), used)
    } else if let Some((value, used)) = crate::util::parse_value_expr_words(&word_refs)
        && value_references_power_or_toughness(&value)
    {
        (value, used)
    } else if let Some((used, toughness)) = characteristic_reference_word_count(&word_refs) {
        let value = if toughness {
            Value::ToughnessOf(Box::new(crate::target::ChooseSpec::Source))
        } else {
            Value::PowerOf(Box::new(crate::target::ChooseSpec::Source))
        };
        (value, used)
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

pub fn parse_fight_shape(tokens: &[OwnedLexToken]) -> Option<FightShape<'_>> {
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

fn clash_opponent<'a>(input: &mut crate::lexer::LexStream<'a>) -> WResult<ClashOpponentAst> {
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

#[cfg(test)]
#[path = "clause_primitive_shapes/tests.rs"]
mod tests;

#[path = "clause_primitive_shapes/resource.rs"]
mod resource_programs;
pub use resource_programs::is_dont_lose_mana_between_steps_shape;
#[path = "clause_primitive_shapes/core.rs"]
mod core_programs;
use core_programs::parse_repeat_process;
pub use core_programs::{parse_clash_shape, parse_repeat_process_shape};
#[path = "clause_primitive_shapes/reference.rs"]
mod reference_programs;
pub use reference_programs::{parse_retarget_constraint_shapes, parse_retarget_reference_shape};
