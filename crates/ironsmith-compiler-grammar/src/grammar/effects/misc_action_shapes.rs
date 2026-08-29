use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::{permission_shapes, primitives, values};
use crate::cards::builders::{CardTextError, PlayerAst};
use crate::effect::{EventValueSpec, Value};
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

#[path = "misc_action_shapes/payment_and_untap.rs"]
mod payment_and_untap;
pub use payment_and_untap::*;
#[path = "misc_action_shapes/die.rs"]
mod die;
pub use die::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BecomePlayerSurface {
    Monarch,
    LifeTotal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchTargetSurface<'a> {
    Source(&'a [OwnedLexToken]),
    Tagged(&'a [OwnedLexToken]),
    Explicit(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchPowerToughnessShape<'a> {
    pub target: SwitchTargetSurface<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipActionKind {
    NextCombatPhaseThisTurn,
    CombatPhases,
    DrawStep,
    Turn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipActionShape {
    pub player: PlayerAst,
    pub action: SkipActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndActionShape {
    Turn,
    CombatPhase,
    EndStepLoseGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipTargetSurface<'a> {
    Source(Option<&'a [OwnedLexToken]>),
    Coin,
    Explicit(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlipActionShape<'a> {
    pub target: FlipTargetSurface<'a>,
    pub delayed_until_next_end_step: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MillActionShape {
    pub count: Value,
}

pub fn parse_misc_word_choice(word: &str, choices: &[&str]) -> bool {
    for choice in choices {
        if permission_shapes::exact_words(&[word], &[*choice]) {
            return true;
        }
    }
    false
}

pub fn parse_become_player_surface(tokens: &[OwnedLexToken]) -> BecomePlayerSurface {
    if [&["the", "monarch"][..], &["monarch"]]
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        BecomePlayerSurface::Monarch
    } else {
        BecomePlayerSurface::LifeTotal
    }
}

pub fn parse_switch_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SwitchPowerToughnessShape<'_>> {
    let (power, _, after_power) =
        primitives::find_prefix(tokens, || primitives::kw("power").void())?;
    let (_, _, after_toughness) =
        primitives::find_prefix(after_power, || primitives::kw("toughness").void())?;
    let leading_target = trim_lexed_commas(tokens.get(..power)?);
    let target_tokens = if permission_shapes::exact_tokens(leading_target, &["the"])
        && let Some(((), trailing_target)) =
            primitives::parse_prefix(after_toughness, primitives::kw("of").void())
        && !trailing_target.is_empty()
    {
        let trailing_target =
            primitives::parse_prefix(trailing_target, primitives::phrase(&["each", "of"]).void())
                .map(|(_, remainder)| remainder)
                .unwrap_or(trailing_target);
        trim_lexed_commas(trailing_target)
    } else {
        leading_target
    };
    let target = if target_tokens.is_empty()
        || [
            &["this"][..],
            &["this", "creature"],
            &["this", "creatures"],
            &["this", "permanent"],
        ]
        .iter()
        .any(|expected| permission_shapes::exact_tokens(target_tokens, expected))
    {
        SwitchTargetSurface::Source(target_tokens)
    } else if permission_shapes::exact_tokens(target_tokens, &["it"]) {
        SwitchTargetSurface::Tagged(target_tokens)
    } else {
        SwitchTargetSurface::Explicit(target_tokens)
    };
    Some(SwitchPowerToughnessShape { target })
}

fn skip_player_prefix(input: &mut LexStream<'_>) -> WResult<(PlayerAst, bool)> {
    alt((
        primitives::kw("your").value((PlayerAst::You, false)),
        primitives::kw("their").value((PlayerAst::That, false)),
        alt((
            primitives::phrase(&["that", "player"]),
            primitives::phrase(&["that", "players"]),
            primitives::phrase(&["his", "or", "her"]),
        ))
        .value((PlayerAst::That, false)),
        alt((
            primitives::phrase(&["target", "player"]),
            primitives::phrase(&["target", "players"]),
        ))
        .value((PlayerAst::Target, false)),
        alt((
            primitives::phrase(&["target", "opponent"]),
            primitives::phrase(&["target", "opponents"]),
        ))
        .value((PlayerAst::TargetOpponent, false)),
        alt((
            primitives::phrase(&["that", "turn"]),
            primitives::kw("turn").void(),
        ))
        .value((PlayerAst::Implicit, true)),
    ))
    .parse_next(input)
}

fn contains_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word).void()).is_some()
}

pub fn parse_skip_action_tokens(
    tokens: &[OwnedLexToken],
    subject_player: Option<PlayerAst>,
) -> Option<SkipActionShape> {
    let (player, action_tokens) = if let Some(player) = subject_player {
        (player, tokens)
    } else {
        let ((player, keep_prefix), tail) = primitives::parse_prefix(tokens, skip_player_prefix)?;
        (player, if keep_prefix { tokens } else { tail })
    };

    let action = if ["combat", "phase", "next", "this", "turn"]
        .iter()
        .all(|word| contains_word(action_tokens, word))
    {
        SkipActionKind::NextCombatPhaseThisTurn
    } else if contains_word(action_tokens, "combat")
        && contains_word(action_tokens, "turn")
        && (contains_word(action_tokens, "phase") || contains_word(action_tokens, "phases"))
    {
        SkipActionKind::CombatPhases
    } else if contains_word(action_tokens, "draw") && contains_word(action_tokens, "step") {
        SkipActionKind::DrawStep
    } else if contains_word(action_tokens, "turn") {
        SkipActionKind::Turn
    } else {
        return None;
    };
    Some(SkipActionShape { player, action })
}

pub fn parse_end_action_tokens(tokens: &[OwnedLexToken]) -> Option<EndActionShape> {
    if [&["the", "combat", "phase"][..], &["combat", "phase"]]
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(EndActionShape::CombatPhase);
    }
    if [&["the", "turn"][..], &["turn"]]
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(EndActionShape::Turn);
    }
    permission_shapes::exact_tokens(tokens, &["step", "you", "lose", "the", "game"])
        .then_some(EndActionShape::EndStepLoseGame)
}

fn next_end_step_suffix(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
        primitives::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
        primitives::phrase(&["at", "beginning", "of", "the", "next", "end", "step"]),
        primitives::phrase(&["at", "beginning", "of", "next", "end", "step"]),
    ))
    .parse_next(input)
}

pub fn parse_flip_action_tokens(tokens: &[OwnedLexToken]) -> FlipActionShape<'_> {
    if tokens.is_empty() {
        return FlipActionShape {
            target: FlipTargetSurface::Source(None),
            delayed_until_next_end_step: false,
        };
    }
    if let Some((action_tokens, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || next_end_step_suffix)
    {
        let action_tokens = trim_lexed_commas(action_tokens);
        if !action_tokens.is_empty() {
            let mut shape = parse_flip_action_tokens(action_tokens);
            shape.delayed_until_next_end_step = true;
            return shape;
        }
    }
    if [&["a", "coin"][..], &["coin"]]
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return FlipActionShape {
            target: FlipTargetSurface::Coin,
            delayed_until_next_end_step: false,
        };
    }
    if [
        &["it"][..],
        &["this"],
        &["this", "creature"],
        &["this", "permanent"],
    ]
    .iter()
    .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return FlipActionShape {
            target: FlipTargetSurface::Source(Some(tokens)),
            delayed_until_next_end_step: false,
        };
    }
    FlipActionShape {
        target: FlipTargetSurface::Explicit(tokens),
        delayed_until_next_end_step: false,
    }
}

fn player_filter_for_library_count(player: PlayerAst) -> Option<PlayerFilter> {
    Some(match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        PlayerAst::Active => PlayerFilter::Active,
        PlayerAst::Opponent => PlayerFilter::Opponent,
        PlayerAst::PlayerToYourLeft => PlayerFilter::PlayerToYourLeft,
        PlayerAst::PlayerToYourRight => PlayerFilter::PlayerToYourRight,
        PlayerAst::NotYou => PlayerFilter::NotYou,
        PlayerAst::Any => PlayerFilter::Any,
        PlayerAst::Target => PlayerFilter::target_player(),
        PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        PlayerAst::ThatPlayerOrTargetController => PlayerFilter::target_player(),
        PlayerAst::Defending => PlayerFilter::Defending,
        PlayerAst::Attacking => PlayerFilter::Attacking,
        PlayerAst::Chosen => PlayerFilter::ChosenPlayer,
        PlayerAst::MostCardsInHand => PlayerFilter::MostCardsInHand,
        PlayerAst::MostLifeTied => PlayerFilter::MostLifeTied,
        PlayerAst::LowestLifeTied => PlayerFilter::LowestLifeTied,
        PlayerAst::TriggeringSourceController => {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged("triggering_source"))
        }
        PlayerAst::ItsController | PlayerAst::ItsOwner | PlayerAst::Enchanted => return None,
    })
}

fn parse_half_library_count(
    tokens: &[OwnedLexToken],
    subject_player: PlayerAst,
) -> Option<(Value, usize, bool)> {
    let words = TokenWordView::new(tokens);
    let refs = words.word_refs();
    if refs.first().copied() != Some("half") {
        return None;
    }
    let (player, library_word) = match refs.get(1..4) {
        Some(["their", "library", _]) => (player_filter_for_library_count(subject_player)?, 2),
        Some(["your", "library", _]) => (PlayerFilter::You, 2),
        Some(["target", "player", "library"]) | Some(["target", "players", "library"]) => {
            (PlayerFilter::target_player(), 3)
        }
        Some(["target", "opponent", "library"]) | Some(["target", "opponents", "library"]) => {
            (PlayerFilter::target_opponent(), 3)
        }
        Some(["that", "player", "library"]) | Some(["that", "players", "library"]) => {
            (player_filter_for_library_count(subject_player)?, 3)
        }
        _ => return None,
    };
    let base = Value::CardsInLibrary(player);
    let value = match refs.get(library_word + 1..library_word + 3)? {
        ["rounded", "down"] => Value::HalfRoundedDown(Box::new(base)),
        ["rounded", "up"] => Value::HalfRoundedDown(Box::new(Value::Add(
            Box::new(base),
            Box::new(Value::Fixed(1)),
        ))),
        _ => return None,
    };
    let consumed_words = library_word + 3;
    let consumed = words.token_index_after_words(consumed_words)?;
    Some((value, consumed, true))
}

fn parse_trailing_for_each_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = TokenWordView::new(tokens);
    let refs = words.word_refs();
    let mut start = usize::from(matches!(refs.first().copied(), Some("card" | "cards")));
    if permission_shapes::prefix_words(&refs[start..], &["for", "each"]) {
        start += 2;
    } else if refs.get(start).copied() == Some("each") {
        start += 1;
    } else {
        return None;
    }
    let after_each = refs.get(start..)?;
    if let Some(on) = permission_shapes::find_words(after_each, &["on"])
        && on > 0
    {
        let counter_words = &after_each[..on];
        let reference = &after_each[on + 1..];
        if matches!(counter_words.last().copied(), Some("counter" | "counters"))
            && ([&["this"][..], &["it"]]
                .iter()
                .any(|prefix| permission_shapes::prefix_words(reference, prefix)))
        {
            let counter_start = words.token_start_indices().get(start).copied()?;
            let counter_end = words.token_start_indices().get(start + on).copied()?;
            if let Some(counter_type) =
                crate::util::parse_counter_type_from_tokens(&tokens[counter_start..counter_end])
            {
                return Some(Value::CountersOnSource(counter_type));
            }
        }
    }

    let mut number_of_words = vec!["the", "number", "of"];
    number_of_words.extend_from_slice(after_each);
    if let Some((value, used)) = crate::util::parse_value_expr_words(&number_of_words)
        && used == number_of_words.len()
    {
        return Some(value);
    }
    crate::util::parse_for_each_count_value_words(&refs).map(|(value, _)| value)
}

fn trailing_is_instead(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens(tokens, &["instead"])
}

pub fn parse_mill_action_tokens(
    tokens: &[OwnedLexToken],
    subject_player: PlayerAst,
) -> Result<Option<MillActionShape>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    if super::chain_splitting::has_authored_comma_then_surface_tokens(tokens) {
        return Ok(None);
    }
    let starts_with_cards = tokens
        .first()
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"));
    let (mut count, used, count_includes_library_noun) = if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["that", "many"]))
    {
        (
            Value::EventValue(EventValueSpec::Amount),
            tokens.len().saturating_sub(rest.len()),
            false,
        )
    } else if starts_with_cards {
        let after_cards = &tokens[1..];
        if let Some((value, used)) = values::parse_value_prefix_lexed(after_cards) {
            (value, 1 + used, false)
        } else if let Some((value, used, includes)) =
            parse_half_library_count(after_cards, subject_player)
        {
            (value, 1 + used, includes)
        } else if let Some(value) = values::parse_add_mana_equal_amount_value_lexed(after_cards) {
            (
                value.with_surface_hint(ValueSurfaceHint::EqualTo),
                tokens.len(),
                false,
            )
        } else {
            return Ok(None);
        }
    } else if let Some(parsed) = parse_half_library_count(tokens, subject_player) {
        parsed
    } else if let Some((value, used)) = values::parse_value_prefix_lexed(tokens) {
        (value, used, false)
    } else {
        return Ok(None);
    };

    let rest = tokens.get(used..).unwrap_or_default();
    if !starts_with_cards && rest.is_empty() && !count_includes_library_noun {
        return Ok(None);
    }
    let trailing = if rest.is_empty() {
        rest
    } else if rest
        .first()
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        &rest[1..]
    } else if !count_includes_library_noun {
        return Ok(None);
    } else {
        rest
    };
    let trailing = trim_lexed_commas(trailing);
    if !trailing.is_empty() && !trailing_is_instead(trailing) {
        if matches!(count, Value::Fixed(1)) {
            count = parse_trailing_for_each_count(trailing).ok_or_else(|| {
                CardTextError::ParseError("unsupported trailing mill clause".to_string())
            })?;
        } else {
            return Err(CardTextError::ParseError(
                "unsupported trailing mill clause".to_string(),
            ));
        }
    }
    Ok(Some(MillActionShape { count }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_switch_skip_flip_and_roll_shapes() {
        let switch = lex_line("target creature's power and toughness", 0).expect("lex");
        assert!(parse_switch_power_toughness_tokens(&switch).is_some());

        let trailing_target = lex_line(
            "the power and toughness of each of up to two target creatures",
            0,
        )
        .expect("lex trailing switch target");
        let parsed = parse_switch_power_toughness_tokens(&trailing_target)
            .expect("trailing switch target should parse");
        assert!(matches!(
            parsed.target,
            SwitchTargetSurface::Explicit(tokens)
                if crate::lexer::token_word_refs(tokens)
                    == ["up", "to", "two", "target", "creatures"]
        ));

        let skip = lex_line("your next combat phase this turn", 0).expect("lex");
        assert_eq!(
            parse_skip_action_tokens(&skip, None).map(|shape| shape.action),
            Some(SkipActionKind::NextCombatPhaseThisTurn)
        );

        let flip = lex_line("it at the beginning of the next end step", 0).expect("lex");
        assert!(parse_flip_action_tokens(&flip).delayed_until_next_end_step);

        let roll = lex_line("a six-sided die", 0).expect("lex");
        assert_eq!(
            parse_roll_die_tokens(&roll).map(|shape| shape.sides),
            Some(6)
        );
    }

    #[test]
    fn parses_mill_counts() {
        let mill = lex_line("three cards", 0).expect("lex");
        assert_eq!(
            parse_mill_action_tokens(&mill, PlayerAst::You)
                .expect("mill parse")
                .map(|shape| shape.count),
            Some(Value::Fixed(3))
        );

        let equal_to = lex_line("cards equal to the number of lands you control", 0)
            .expect("lex equal-to mill count");
        let equal_to = parse_mill_action_tokens(&equal_to, PlayerAst::You)
            .expect("equal-to mill parse")
            .expect("equal-to mill shape");
        assert!(equal_to.count.has_surface_hint(ValueSurfaceHint::EqualTo));
    }
}
