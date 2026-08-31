use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};

use super::super::{primitives, trigger_surface};
use crate::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::model::ast::TriggerIntroSurfaceAst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleDocumentLineShape {
    StartYourEngines,
    Learn,
    SplitTopAndFaceDownLook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialLineShape {
    SplitTopLookAndLandPlay,
    AssignDamageAsUnblockedEnchanted,
    GraveyardOrExileCast,
    AdditionalCombatAfterMainPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftRuleLineShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChampionedWithThisTriggerShape<'a> {
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxSpeedLineShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub trigger_intro: Option<TriggerIntroSurfaceAst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChampionLineShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationKeywordLineShape {
    pub creature_threshold: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationThresholdLineShape<'a> {
    pub threshold: i32,
    pub body_tokens: &'a [OwnedLexToken],
    pub trigger_intro: Option<TriggerIntroSurfaceAst>,
    pub needs_terminal_punctuation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscapeEntersWithLineShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlternativeCostKeywordLineShape<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedStatementPreference {
    ChooseTwoShuffleRestBattlefield,
    ExiledCardCostsMore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadingUnlessLineShape<'a> {
    pub condition_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

fn parse_visible_all<'a, O, P>(tokens: &'a [OwnedLexToken], mut parser: P) -> Option<O>
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let visible = super::parse_visible_line_tokens(tokens);
    let visible = if visible.last().is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Period | TokenKind::Bang | TokenKind::Question
        )
    }) {
        visible.get(..visible.len().saturating_sub(1))?
    } else {
        visible
    };
    crate::grammar::primitives::probe_shape(parser.parse(LexStream::new(visible)))
}

fn simple_document_line(input: &mut LexStream<'_>) -> WResult<SimpleDocumentLineShape> {
    alt((
        primitives::phrase(&["start", "your", "engines"])
            .value(SimpleDocumentLineShape::StartYourEngines),
        primitives::kw("learn").value(SimpleDocumentLineShape::Learn),
        primitives::phrase(&[
            "you",
            "may",
            "look",
            "at",
            "the",
            "top",
            "card",
            "of",
            "your",
            "library",
            "and",
            "at",
            "face-down",
            "creatures",
            "you",
            "don't",
            "control",
            "any",
            "time",
        ])
        .value(SimpleDocumentLineShape::SplitTopAndFaceDownLook),
    ))
    .parse_next(input)
}

pub fn parse_simple_document_line(tokens: &[OwnedLexToken]) -> Option<SimpleDocumentLineShape> {
    parse_visible_all(tokens, simple_document_line)
}

fn special_line(input: &mut LexStream<'_>) -> WResult<SpecialLineShape> {
    alt((
        split_top_look_and_land_play,
        primitives::phrase(&[
            "enchanted",
            "creature's",
            "controller",
            "may",
            "have",
            "it",
            "assign",
            "its",
            "combat",
            "damage",
            "as",
            "though",
            "it",
            "weren't",
            "blocked",
        ])
        .value(SpecialLineShape::AssignDamageAsUnblockedEnchanted),
        primitives::phrase(&[
            "you",
            "may",
            "cast",
            "this",
            "card",
            "from",
            "your",
            "graveyard",
            "or",
            "from",
            "exile",
        ])
        .value(SpecialLineShape::GraveyardOrExileCast),
        additional_combat_after_main_phase,
    ))
    .parse_next(input)
}

fn split_top_look_and_land_play(input: &mut LexStream<'_>) -> WResult<SpecialLineShape> {
    primitives::phrase(&[
        "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "and", "you", "may", "play", "lands", "from", "the", "top", "of", "your", "library",
    ])
    .parse_next(input)?;
    Ok(SpecialLineShape::SplitTopLookAndLandPlay)
}

fn additional_combat_after_main_phase(input: &mut LexStream<'_>) -> WResult<SpecialLineShape> {
    primitives::phrase(&["after", "this", "main", "phase"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "there",
        "is",
        "an",
        "additional",
        "combat",
        "phase",
        "followed",
        "by",
        "an",
        "additional",
        "main",
        "phase",
    ])
    .parse_next(input)?;
    Ok(SpecialLineShape::AdditionalCombatAfterMainPhase)
}

pub fn parse_special_line(tokens: &[OwnedLexToken]) -> Option<SpecialLineShape> {
    parse_visible_all(tokens, special_line)
}

fn draft_rule_prefix(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["reveal", "this", "card", "as", "you", "draft", "it"]),
        primitives::phrase(&["as", "you", "draft"]),
        primitives::phrase(&["during", "the", "draft"]),
        primitives::phrase(&["immediately", "after", "the", "draft"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_draft_rule_line(tokens: &[OwnedLexToken]) -> Option<DraftRuleLineShape> {
    let visible = super::parse_visible_line_tokens(tokens);
    if parse_visible_all(
        visible,
        primitives::phrase(&["draft", "this", "card", "face", "up"]),
    )
    .is_some()
        || primitives::parse_prefix(visible, draft_rule_prefix).is_some()
    {
        return Some(DraftRuleLineShape);
    }
    primitives::parse_prefix(visible, primitives::phrase(&["each", "player", "passes"]))?;
    primitives::find_prefix(visible, || primitives::phrase(&["booster", "pack"]).void())?;
    Some(DraftRuleLineShape)
}

pub fn parse_championed_with_this_trigger(
    tokens: &[OwnedLexToken],
) -> Option<ChampionedWithThisTriggerShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("when"))?;
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["is", "championed", "with", "this"]).void()
    })?;
    let split = super::parse_comma_split(tokens)?;
    (!split.after.is_empty()).then_some(ChampionedWithThisTriggerShape {
        effect_tokens: split.after,
    })
}

pub fn parse_max_speed_line(tokens: &[OwnedLexToken]) -> Option<MaxSpeedLineShape<'_>> {
    primitives::parse_prefix(tokens, primitives::phrase(&["max", "speed"]))?;
    let body_tokens = super::parse_max_speed_body(tokens)
        .map(|shape| shape.body_tokens)
        .unwrap_or(tokens);
    Some(MaxSpeedLineShape {
        body_tokens,
        trigger_intro: trigger_surface::parse_trigger_intro_surface_tokens(body_tokens),
    })
}

fn parse_keyword_tail<'a>(
    tokens: &'a [OwnedLexToken],
    keyword: &'static str,
) -> Option<&'a [OwnedLexToken]> {
    let visible = super::parse_visible_line_tokens(tokens);
    let (_, mut tail) = primitives::parse_prefix(visible, primitives::kw(keyword))?;
    if tail.first().is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Dash | TokenKind::EmDash | TokenKind::Colon
        )
    }) {
        tail = tail.get(1..)?;
    }
    Some(super::trim_commas(tail))
}

pub fn parse_champion_line(tokens: &[OwnedLexToken]) -> Option<ChampionLineShape<'_>> {
    let mut filter_tokens = parse_keyword_tail(tokens, "champion")?;
    if primitives::parse_prefix(
        filter_tokens,
        alt((primitives::kw("a"), primitives::kw("an"))).void(),
    )
    .is_some()
    {
        filter_tokens = filter_tokens.get(1..)?;
    }
    Some(ChampionLineShape { filter_tokens })
}

pub fn parse_station_keyword_line(
    tokens: &[OwnedLexToken],
    source_tokens: &[OwnedLexToken],
) -> Option<StationKeywordLineShape> {
    primitives::parse_prefix(tokens, primitives::kw("station"))?;
    Some(StationKeywordLineShape {
        creature_threshold: super::parse_station_creature_threshold(tokens)
            .or_else(|| super::parse_station_creature_threshold(source_tokens)),
    })
}

pub fn parse_station_threshold_line(
    tokens: &[OwnedLexToken],
) -> Option<StationThresholdLineShape<'_>> {
    let shape = super::parse_station_threshold(tokens)?;
    let last_kind = shape.body_tokens.last().map(|token| token.kind);
    Some(StationThresholdLineShape {
        threshold: shape.threshold,
        body_tokens: shape.body_tokens,
        trigger_intro: trigger_surface::parse_trigger_intro_surface_tokens(shape.body_tokens),
        needs_terminal_punctuation: !matches!(
            last_kind,
            Some(TokenKind::Period | TokenKind::Bang | TokenKind::Question)
        ),
    })
}

pub fn parse_escape_enters_with_line(
    tokens: &[OwnedLexToken],
) -> Option<EscapeEntersWithLineShape> {
    primitives::find_prefix(tokens, || primitives::phrase(&["escapes", "with"]).void())?;
    Some(EscapeEntersWithLineShape)
}

pub fn parse_surge_line(tokens: &[OwnedLexToken]) -> Option<AlternativeCostKeywordLineShape<'_>> {
    Some(AlternativeCostKeywordLineShape {
        cost_tokens: parse_keyword_tail(tokens, "surge")?,
    })
}

pub fn parse_freerunning_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCostKeywordLineShape<'_>> {
    Some(AlternativeCostKeywordLineShape {
        cost_tokens: parse_keyword_tail(tokens, "freerunning")?,
    })
}

pub fn parse_linked_statement_preference(
    tokens: &[OwnedLexToken],
) -> Option<LinkedStatementPreference> {
    parse_choose_two_shuffle_rest_battlefield(tokens)
        .or_else(|| parse_exiled_card_costs_more(tokens))
}

fn parse_choose_two_shuffle_rest_battlefield(
    tokens: &[OwnedLexToken],
) -> Option<LinkedStatementPreference> {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["chooses", "two", "of", "those", "cards"]).void()
    })?;
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["shuffle", "the", "chosen", "cards"]).void()
    })?;
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["put", "the", "rest", "onto", "the", "battlefield"]).void()
    })?;
    Some(LinkedStatementPreference::ChooseTwoShuffleRestBattlefield)
}

fn parse_exiled_card_costs_more(tokens: &[OwnedLexToken]) -> Option<LinkedStatementPreference> {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&[
            "for", "as", "long", "as", "that", "card", "remains", "exiled",
        ])
        .void()
    })?;
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["more", "to", "cast"]).void()
    })?;
    Some(LinkedStatementPreference::ExiledCardCostsMore)
}

pub fn parse_leading_unless_line(tokens: &[OwnedLexToken]) -> Option<LeadingUnlessLineShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("unless"))?;
    let split = super::parse_comma_split(tokens)?;
    (split.before.len() >= 2 && !split.after.is_empty()).then_some(LeadingUnlessLineShape {
        condition_tokens: split.before,
        effect_tokens: split.after,
    })
}
