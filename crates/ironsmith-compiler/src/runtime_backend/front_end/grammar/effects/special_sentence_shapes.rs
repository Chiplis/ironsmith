use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::{leaf, permission_shapes, primitives};
use crate::cards::builders::PlayerAst;
use crate::effect::{ChoiceCount, Until, Value};
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordBundleShapeError {
    UnsupportedAbility,
    ModifierChanged,
    UnsupportedTrailingList,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeywordBundlePumpShape<'a> {
    pub(crate) duration: Until,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) power: Value,
    pub(crate) toughness: Value,
    pub(crate) abilities: Vec<StaticAbilityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScaleAxes {
    pub(crate) power: bool,
    pub(crate) toughness: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ScaledPowerShape<'a> {
    SetLifeTotal {
        player: PlayerAst,
        player_filter: PlayerFilter,
    },
    DoubleManaPool {
        player: PlayerAst,
    },
    ScaleAll {
        filter_tokens: &'a [OwnedLexToken],
        axes: ScaleAxes,
        multiplier: i32,
    },
    ScaleTarget {
        target_tokens: &'a [OwnedLexToken],
        axes: ScaleAxes,
        multiplier: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SacrificeThenDrawShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) artifact_enchantment_or_token: bool,
}

fn trim_shape_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

fn exact_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    permission_shapes::exact_tokens(trim_shape_edges(tokens), expected)
}

fn bundle_duration<'a>(input: &mut LexStream<'a>) -> WResult<Until> {
    alt((
        primitives::phrase(&["until", "end", "of", "turn"]).value(Until::EndOfTurn),
        primitives::phrase(&["until", "your", "next", "turn"]).value(Until::YourNextTurn),
        primitives::phrase(&["until", "end", "of", "combat"]).value(Until::EndOfCombat),
    ))
    .parse_next(input)
}

fn keyword_bundle_ability<'a>(input: &mut LexStream<'a>) -> WResult<StaticAbilityId> {
    alt((
        primitives::phrase(&["first", "strike"]).value(StaticAbilityId::FirstStrike),
        primitives::phrase(&["double", "strike"]).value(StaticAbilityId::DoubleStrike),
        alt((
            primitives::kw("flying").value(StaticAbilityId::Flying),
            primitives::kw("deathtouch").value(StaticAbilityId::Deathtouch),
            primitives::kw("haste").value(StaticAbilityId::Haste),
            primitives::kw("hexproof").value(StaticAbilityId::Hexproof),
            primitives::kw("indestructible").value(StaticAbilityId::Indestructible),
            primitives::kw("lifelink").value(StaticAbilityId::Lifelink),
        )),
        alt((
            primitives::kw("menace").value(StaticAbilityId::Menace),
            primitives::kw("protection").value(StaticAbilityId::Protection),
            primitives::kw("reach").value(StaticAbilityId::Reach),
            primitives::kw("trample").value(StaticAbilityId::Trample),
            primitives::kw("vigilance").value(StaticAbilityId::Vigilance),
            primitives::kw("partner").value(StaticAbilityId::Partner),
        )),
    ))
    .parse_next(input)
}

fn pt_modifier_prefix(tokens: &[OwnedLexToken]) -> Option<((Value, Value), &[OwnedLexToken])> {
    let token = tokens.first()?;
    let modifier = token.as_word()?;
    let values = leaf::parse_leaf_pt_modifier_values_complete(modifier).ok()?;
    Some((values, &tokens[1..]))
}

fn parse_bundle_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<((Value, Value), StaticAbilityId, &[OwnedLexToken])>, KeywordBundleShapeError> {
    let Some((modifier, after_modifier)) = pt_modifier_prefix(tokens) else {
        return Ok(None);
    };
    let Some(((), after_condition)) =
        primitives::parse_prefix(after_modifier, primitives::phrase(&["if", "it", "has"]))
    else {
        return Ok(None);
    };
    let Some((ability, rest)) = primitives::parse_prefix(after_condition, keyword_bundle_ability)
    else {
        return Err(KeywordBundleShapeError::UnsupportedAbility);
    };
    Ok(Some((modifier, ability, rest)))
}

fn strip_bundle_separator(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tokens = trim_lexed_commas(tokens);
    if let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("and").void()) {
        trim_lexed_commas(rest)
    } else {
        tokens
    }
}

pub(crate) fn parse_keyword_bundle_pump_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<KeywordBundlePumpShape<'_>>, KeywordBundleShapeError> {
    let tokens = trim_shape_edges(tokens);
    let Some((duration, after_duration)) = primitives::parse_prefix(tokens, bundle_duration) else {
        return Ok(None);
    };
    let Some((get_idx, _, after_get)) = primitives::find_prefix(after_duration, || {
        alt((primitives::kw("get").void(), primitives::kw("gets").void()))
    }) else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }
    let mut filter_tokens = trim_lexed_commas(&after_duration[..get_idx]);
    if let Some(((), rest)) = primitives::parse_prefix(
        filter_tokens,
        alt((primitives::kw("each").void(), primitives::kw("all").void())),
    ) {
        filter_tokens = trim_lexed_commas(rest);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let Some(((power, toughness), first_ability, mut rest)) = parse_bundle_clause(after_get)?
    else {
        return Ok(None);
    };
    let mut abilities = vec![first_ability];
    loop {
        let candidate = strip_bundle_separator(rest);
        let Some(((next_power, next_toughness), ability, next_rest)) =
            parse_bundle_clause(candidate)?
        else {
            break;
        };
        if next_power != power || next_toughness != toughness {
            return Err(KeywordBundleShapeError::ModifierChanged);
        }
        abilities.push(ability);
        rest = next_rest;
    }

    let rest = trim_lexed_commas(rest);
    let Some(((), mut trailing)) =
        primitives::parse_prefix(rest, primitives::phrase(&["and", "so", "on", "for"]))
    else {
        return Ok(None);
    };
    trailing = strip_bundle_separator(trailing);
    while !trailing.is_empty() {
        let Some((ability, rest)) = primitives::parse_prefix(trailing, keyword_bundle_ability)
        else {
            return Err(KeywordBundleShapeError::UnsupportedTrailingList);
        };
        abilities.push(ability);
        trailing = strip_bundle_separator(rest);
    }

    Ok(Some(KeywordBundlePumpShape {
        duration,
        filter_tokens,
        power,
        toughness,
        abilities,
    }))
}

fn scaled_verb<'a>(input: &mut LexStream<'a>) -> WResult<i32> {
    alt((
        primitives::kw("double").value(1),
        primitives::kw("triple").value(2),
    ))
    .parse_next(input)
}

fn scaled_life_player(tokens: &[OwnedLexToken]) -> Option<(PlayerAst, PlayerFilter)> {
    if exact_tokens(tokens, &["your"]) {
        Some((PlayerAst::You, PlayerFilter::You))
    } else if exact_tokens(tokens, &["target", "player"])
        || exact_tokens(tokens, &["target", "players"])
    {
        Some((PlayerAst::Target, PlayerFilter::target_player()))
    } else if exact_tokens(tokens, &["target", "opponent"])
        || exact_tokens(tokens, &["target", "opponents"])
    {
        Some((PlayerAst::TargetOpponent, PlayerFilter::target_opponent()))
    } else if exact_tokens(tokens, &["opponent"])
        || exact_tokens(tokens, &["opponents"])
        || exact_tokens(tokens, &["an", "opponent"])
        || exact_tokens(tokens, &["an", "opponents"])
    {
        Some((PlayerAst::Opponent, PlayerFilter::Opponent))
    } else {
        None
    }
}

fn scaled_mana_player(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    if exact_tokens(tokens, &["you", "have"]) {
        Some(PlayerAst::You)
    } else if exact_tokens(tokens, &["target", "player", "has"])
        || exact_tokens(tokens, &["target", "player", "have"])
    {
        Some(PlayerAst::Target)
    } else if exact_tokens(tokens, &["target", "opponent", "has"])
        || exact_tokens(tokens, &["target", "opponent", "have"])
    {
        Some(PlayerAst::TargetOpponent)
    } else if exact_tokens(tokens, &["opponent", "has"])
        || exact_tokens(tokens, &["opponents", "have"])
    {
        Some(PlayerAst::Opponent)
    } else {
        None
    }
}

fn strip_scaled_duration(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::split_lexed_once_before_suffix(tokens, 0, || {
        primitives::phrase(&["until", "end", "of", "turn"])
    })
    .map(|(head, ())| trim_lexed_commas(head))
    .unwrap_or(tokens)
}

fn scale_axes_prefix(tokens: &[OwnedLexToken]) -> Option<(ScaleAxes, &[OwnedLexToken])> {
    [
        (
            &["the", "power", "and", "toughness", "of"][..],
            ScaleAxes {
                power: true,
                toughness: true,
            },
        ),
        (
            &["the", "power", "of"][..],
            ScaleAxes {
                power: true,
                toughness: false,
            },
        ),
        (
            &["the", "toughness", "of"][..],
            ScaleAxes {
                power: false,
                toughness: true,
            },
        ),
    ]
    .into_iter()
    .find_map(|(prefix, axes)| {
        primitives::parse_prefix(tokens, primitives::phrase(prefix)).map(|((), rest)| (axes, rest))
    })
}

fn scale_axes_suffix(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], ScaleAxes)> {
    [
        (
            &["power", "and", "toughness"][..],
            ScaleAxes {
                power: true,
                toughness: true,
            },
        ),
        (
            &["power"][..],
            ScaleAxes {
                power: true,
                toughness: false,
            },
        ),
        (
            &["toughness"][..],
            ScaleAxes {
                power: false,
                toughness: true,
            },
        ),
    ]
    .into_iter()
    .find_map(|(suffix, axes)| {
        primitives::split_lexed_once_before_suffix(tokens, 1, || primitives::phrase(suffix))
            .map(|(head, ())| (trim_lexed_commas(head), axes))
    })
}

fn scaled_target_surface<'a>(
    tokens: &'a [OwnedLexToken],
    axes: ScaleAxes,
    multiplier: i32,
) -> Option<ScaledPowerShape<'a>> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return None;
    }
    if let Some(((), filter_tokens)) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("each").void(), primitives::kw("all").void())),
    ) {
        let filter_tokens = trim_lexed_commas(filter_tokens);
        return (!filter_tokens.is_empty()).then_some(ScaledPowerShape::ScaleAll {
            filter_tokens,
            axes,
            multiplier,
        });
    }
    Some(ScaledPowerShape::ScaleTarget {
        target_tokens: tokens,
        axes,
        multiplier,
    })
}

pub(crate) fn parse_scaled_power_shape(tokens: &[OwnedLexToken]) -> Option<ScaledPowerShape<'_>> {
    let mut tokens = trim_shape_edges(tokens);
    if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["until", "end", "of", "turn"]))
    {
        tokens = trim_shape_edges(rest);
    }
    let (multiplier, rest) = primitives::parse_prefix(tokens, scaled_verb)?;
    let rest = strip_scaled_duration(rest);

    if multiplier == 1
        && let Some((player_tokens, ())) =
            primitives::split_lexed_once_before_suffix(rest, 1, || {
                primitives::phrase(&["life", "total"])
            })
        && let Some((player, player_filter)) = scaled_life_player(player_tokens)
    {
        return Some(ScaledPowerShape::SetLifeTotal {
            player,
            player_filter,
        });
    }

    if multiplier == 1
        && let Some(((), player_tokens)) = primitives::parse_prefix(
            rest,
            primitives::phrase(&[
                "the", "amount", "of", "each", "type", "of", "unspent", "mana",
            ]),
        )
        && let Some(player) = scaled_mana_player(player_tokens)
    {
        return Some(ScaledPowerShape::DoubleManaPool { player });
    }

    if let Some((axes, subject_tokens)) = scale_axes_prefix(rest) {
        return scaled_target_surface(subject_tokens, axes, multiplier);
    }
    let (subject_tokens, axes) = scale_axes_suffix(rest)?;
    scaled_target_surface(subject_tokens, axes, multiplier)
}

pub(crate) fn parses_spell_this_way_pay_life(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_shape_edges(tokens);
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["if", "you", "cast", "a", "spell", "this", "way"]),
    )
    .is_some()
        && primitives::contains_word(tokens, "rather")
        && primitives::contains_word(tokens, "mana")
        && primitives::contains_word(tokens, "cost")
}

pub(crate) fn parse_sacrifice_then_draw_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeThenDrawShape<'_>> {
    let tokens = trim_shape_edges(tokens);
    let ((), after_sacrifice) =
        primitives::parse_prefix(tokens, primitives::kw("sacrifice").void())?;
    let (choice_tokens, draw_tokens) =
        primitives::split_lexed_once_on_separator(after_sacrifice, || {
            primitives::kw("then").void()
        })?;
    if !exact_tokens(draw_tokens, &["draw", "that", "many", "cards"]) {
        return None;
    }
    let choice = leaf::parse_leaf_choice_count_prefix_tokens(choice_tokens)?;
    if choice.count != ChoiceCount::any_number() {
        return None;
    }
    let filter_tokens = trim_lexed_commas(choice_tokens.get(choice.consumed..)?);
    if filter_tokens.is_empty() {
        return Some(SacrificeThenDrawShape {
            filter_tokens,
            artifact_enchantment_or_token: false,
        });
    }
    let artifact_enchantment_or_token = (primitives::contains_word(filter_tokens, "token")
        || primitives::contains_word(filter_tokens, "tokens"))
        && (primitives::contains_word(filter_tokens, "artifact")
            || primitives::contains_word(filter_tokens, "artifacts"))
        && (primitives::contains_word(filter_tokens, "enchantment")
            || primitives::contains_word(filter_tokens, "enchantments"));
    Some(SacrificeThenDrawShape {
        filter_tokens,
        artifact_enchantment_or_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_scaled_target_and_sweep_shapes() {
        let tokens = lex_line(
            "double the power and toughness of each creature you control until end of turn",
            0,
        )
        .unwrap();
        let ScaledPowerShape::ScaleAll {
            axes, multiplier, ..
        } = parse_scaled_power_shape(&tokens).unwrap()
        else {
            panic!("expected sweep");
        };
        assert_eq!(
            axes,
            ScaleAxes {
                power: true,
                toughness: true
            }
        );
        assert_eq!(multiplier, 1);

        let tokens = lex_line(
            "triple target creature's power and toughness until end of turn",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_scaled_power_shape(&tokens),
            Some(ScaledPowerShape::ScaleTarget { multiplier: 2, .. })
        ));

        let tokens = lex_line("until end of turn, double target creature's power", 0).unwrap();
        assert!(matches!(
            parse_scaled_power_shape(&tokens),
            Some(ScaledPowerShape::ScaleTarget {
                axes: ScaleAxes {
                    power: true,
                    toughness: false,
                },
                multiplier: 1,
                ..
            })
        ));
    }

    #[test]
    fn parses_keyword_bundle_shape() {
        let tokens = lex_line(
            "until end of turn each other creature you control gets +1/+1 if it has flying +1/+1 if it has first strike and so on for double strike deathtouch and haste",
            0,
        )
        .unwrap();
        let shape = parse_keyword_bundle_pump_shape(&tokens).unwrap().unwrap();
        assert_eq!(shape.power, Value::Fixed(1));
        assert_eq!(shape.toughness, Value::Fixed(1));
        assert_eq!(shape.abilities.len(), 5);
    }

    #[test]
    fn parses_punctuated_keyword_bundle_shape_without_truncating_the_trailing_list() {
        let tokens = lex_line(
            "until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner",
            0,
        )
        .unwrap();
        let shape = parse_keyword_bundle_pump_shape(&tokens).unwrap().unwrap();

        assert_eq!(shape.abilities.len(), 14);
        assert_eq!(shape.abilities.first(), Some(&StaticAbilityId::Flying));
        assert_eq!(shape.abilities.last(), Some(&StaticAbilityId::Partner));
    }

    #[test]
    fn parses_sacrifice_then_draw_shape() {
        let tokens = lex_line(
            "sacrifice any number of artifacts enchantments and tokens then draw that many cards",
            0,
        )
        .unwrap();
        let shape = parse_sacrifice_then_draw_shape(&tokens).unwrap();
        assert!(shape.artifact_enchantment_or_token);
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            vec!["artifacts", "enchantments", "and", "tokens"]
        );
    }
}
