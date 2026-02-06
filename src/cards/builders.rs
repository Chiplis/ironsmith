//! Extended card builder with ability support.
//!
//! This module extends the CardBuilder with methods for adding abilities,
//! making it easy to define cards with their complete gameplay mechanics.

use crate::ability::{
    self, Ability, AbilityKind, ActivatedAbility, ActivationTiming, LevelAbility, ManaAbility,
    ManaAbilityCondition, TriggeredAbility,
};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::{CardBuilder, PowerToughness, PtValue};
use crate::color::ColorSet;
use crate::cost::{OptionalCost, TotalCost};
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, Until, Value,
};
use crate::effects::VoteOption;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{
    ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::triggers::Trigger;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use std::collections::HashMap;

use super::CardDefinition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardTextError {
    UnsupportedLine(String),
    ParseError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeywordAction {
    Flying,
    Menace,
    Hexproof,
    Haste,
    Improvise,
    Convoke,
    AffinityForArtifacts,
    Delve,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Lifelink,
    Vigilance,
    Trample,
    Reach,
    Defender,
    Flash,
    Indestructible,
    Shroud,
    Ward(u32),
    Wither,
    Infect,
    Undying,
    Persist,
    Prowess,
    Exalted,
    Storm,
    Toxic(u32),
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Flanking,
    Bushido(u32),
    Changeling,
    ProtectionFrom(ColorSet),
    ProtectionFromAllColors,
    ProtectionFromColorless,
    ProtectionFromCardType(CardType),
    ProtectionFromSubtype(Subtype),
    Unblockable,
    Marker(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    fn synthetic() -> Self {
        Self {
            line: 0,
            start: 0,
            end: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String, TextSpan),
    Comma(TextSpan),
    Period(TextSpan),
    Colon(TextSpan),
    Semicolon(TextSpan),
}

impl Token {
    fn as_word(&self) -> Option<&str> {
        match self {
            Token::Word(word, _) => Some(word.as_str()),
            _ => None,
        }
    }

    fn is_word(&self, value: &str) -> bool {
        matches!(self, Token::Word(word, _) if word == value)
    }

    fn span(&self) -> TextSpan {
        match self {
            Token::Word(_, span)
            | Token::Comma(span)
            | Token::Period(span)
            | Token::Colon(span)
            | Token::Semicolon(span) => *span,
        }
    }
}

#[derive(Debug, Clone)]
enum LineAst {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbility),
    StaticAbilities(Vec<StaticAbility>),
    Ability(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
    },
    Statement {
        effects: Vec<EffectAst>,
    },
    AdditionalCost {
        effects: Vec<EffectAst>,
    },
}

#[derive(Debug, Clone)]
struct ParsedAbility {
    ability: Ability,
    effects_ast: Option<Vec<EffectAst>>,
}

#[derive(Debug, Clone)]
enum TriggerSpec {
    ThisAttacks,
    ThisBlocks,
    ThisBecomesBlocked,
    ThisBlocksOrBecomesBlocked,
    ThisDies,
    ThisLeavesBattlefield,
    ThisBecomesMonstrous,
    ThisBecomesTapped,
    ThisBecomesUntapped,
    ThisDealsDamage,
    ThisIsDealtDamage,
    YouGainLife,
    YouDrawCard,
    Dies(ObjectFilter),
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
    },
    EntersBattlefield(ObjectFilter),
    EntersBattlefieldTapped(ObjectFilter),
    EntersBattlefieldUntapped(ObjectFilter),
    BeginningOfUpkeep(PlayerFilter),
    BeginningOfCombat(PlayerFilter),
    BeginningOfEndStep(PlayerFilter),
    BeginningOfPrecombatMain(PlayerFilter),
    ThisEntersBattlefield,
    ThisDealsCombatDamageToPlayer,
    YouCastThisSpell,
    KeywordAction {
        action: crate::events::KeywordActionKind,
        player: PlayerFilter,
    },
    SagaChapter(Vec<u32>),
    Either(Box<TriggerSpec>, Box<TriggerSpec>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerAst {
    You,
    Defending,
    Target,
    That,
    ItsController,
    Implicit,
}

#[derive(Debug, Clone, PartialEq)]
enum TargetAst {
    Source(Option<TextSpan>),
    AnyTarget(Option<TextSpan>),
    Spell(Option<TextSpan>),
    Player(PlayerFilter, Option<TextSpan>),
    Object(ObjectFilter, Option<TextSpan>, Option<TextSpan>),
    Tagged(TagKey, Option<TextSpan>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectRefAst {
    It,
}

#[derive(Debug, Clone, PartialEq)]
enum PredicateAst {
    ItIsLandCard,
    ItMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlDurationAst {
    UntilEndOfTurn,
    DuringNextTurn,
    Forever,
}

#[derive(Debug, Clone)]
enum EffectAst {
    DealDamage {
        amount: Value,
        target: TargetAst,
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    Draw {
        count: Value,
        player: PlayerAst,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        mana: Vec<ManaSymbol>,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
        target: TargetAst,
    },
    Proliferate,
    Tap {
        target: TargetAst,
    },
    Untap {
        target: TargetAst,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
        player: PlayerAst,
    },
    GainLife {
        amount: Value,
        player: PlayerAst,
    },
    LoseGame {
        player: PlayerAst,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    GrantProtectionChoice {
        target: TargetAst,
        allow_colorless: bool,
    },
    Earthbend {
        counters: u32,
    },
    AddMana {
        mana: Vec<ManaSymbol>,
        player: PlayerAst,
    },
    AddManaAnyColor {
        amount: Value,
        player: PlayerAst,
    },
    AddManaAnyOneColor {
        amount: Value,
        player: PlayerAst,
    },
    AddManaCommanderIdentity {
        amount: Value,
        player: PlayerAst,
    },
    AddManaImprintedColors,
    Scry {
        count: Value,
        player: PlayerAst,
    },
    Surveil {
        count: Value,
        player: PlayerAst,
    },
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
    },
    PlayFromGraveyardUntilEot {
        player: PlayerAst,
    },
    ExileInsteadOfGraveyardThisTurn {
        player: PlayerAst,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
    },
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
    ExtraTurnAfterTurn {
        player: PlayerAst,
    },
    RevealTop {
        player: PlayerAst,
    },
    RevealHand {
        player: PlayerAst,
    },
    PutIntoHand {
        player: PlayerAst,
        object: ObjectRefAst,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    ChooseObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        player: PlayerAst,
        tag: TagKey,
    },
    Sacrifice {
        filter: ObjectFilter,
        player: PlayerAst,
    },
    SacrificeAll {
        filter: ObjectFilter,
        player: PlayerAst,
    },
    DiscardHand {
        player: PlayerAst,
    },
    Discard {
        count: Value,
        player: PlayerAst,
        random: bool,
    },
    Transform {
        target: TargetAst,
    },
    Regenerate {
        target: TargetAst,
    },
    Mill {
        count: Value,
        player: PlayerAst,
    },
    ReturnToHand {
        target: TargetAst,
    },
    ReturnToBattlefield {
        target: TargetAst,
        tapped: bool,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
    },
    ExchangeControl {
        filter: ObjectFilter,
        count: u32,
    },
    SetLifeTotal {
        amount: Value,
        player: PlayerAst,
    },
    SkipTurn {
        player: PlayerAst,
    },
    SkipDrawStep {
        player: PlayerAst,
    },
    PoisonCounters {
        count: Value,
        player: PlayerAst,
    },
    EnergyCounters {
        count: Value,
        player: PlayerAst,
    },
    May {
        effects: Vec<EffectAst>,
    },
    MayByTaggedController {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    IfResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayer {
        effects: Vec<EffectAst>,
    },
    ForEachOpponentDoesNot {
        effects: Vec<EffectAst>,
    },
    ForEachTaggedPlayer {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    Enchant {
        filter: ObjectFilter,
    },
    Investigate,
    Destroy {
        target: TargetAst,
    },
    DestroyAll {
        filter: ObjectFilter,
    },
    Exile {
        target: TargetAst,
    },
    ExileAll {
        filter: ObjectFilter,
    },
    LookAtHand {
        target: TargetAst,
    },
    TargetOnly {
        target: TargetAst,
    },
    #[allow(dead_code)]
    CreateToken {
        name: String,
        count: u32,
        player: PlayerAst,
    },
    CreateTokenCopy {
        object: ObjectRefAst,
        count: u32,
        player: PlayerAst,
        half_power_toughness_round_up: bool,
    },
    CreateTokenCopyFromSource {
        source: TargetAst,
        count: u32,
        player: PlayerAst,
        half_power_toughness_round_up: bool,
    },
    CreateTokenWithMods {
        name: String,
        count: u32,
        player: PlayerAst,
        tapped: bool,
        attacking: bool,
        exile_at_end_of_combat: bool,
    },
    ExileThatTokenAtEndOfCombat,
    Monstrosity {
        amount: Value,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
    },
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    },
    PumpAll {
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<StaticAbility>,
        duration: Until,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<StaticAbility>,
        duration: Until,
    },
    GrantAbilityToSource {
        ability: Ability,
    },
    SearchLibrary {
        filter: ObjectFilter,
        destination: Zone,
        player: PlayerAst,
        reveal: bool,
        shuffle: bool,
        count: ChoiceCount,
    },
    #[allow(dead_code)]
    ShuffleLibrary {
        player: PlayerAst,
    },
    VoteStart {
        options: Vec<String>,
    },
    VoteOption {
        option: String,
        effects: Vec<EffectAst>,
    },
    VoteExtra {
        count: u32,
        optional: bool,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ParseAnnotations {
    pub tag_spans: HashMap<TagKey, Vec<TextSpan>>,
    pub normalized_lines: HashMap<usize, String>,
    pub original_lines: HashMap<usize, String>,
    pub normalized_char_maps: HashMap<usize, Vec<usize>>,
}

impl ParseAnnotations {
    fn record_tag_span(&mut self, tag: &TagKey, span: TextSpan) {
        self.tag_spans.entry(tag.clone()).or_default().push(span);
    }

    fn record_normalized_line(&mut self, line_index: usize, line: &str) {
        self.normalized_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_original_line(&mut self, line_index: usize, line: &str) {
        self.original_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_char_map(&mut self, line_index: usize, map: Vec<usize>) {
        self.normalized_char_maps.entry(line_index).or_insert(map);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IfResultPredicate {
    Did,
    DidNot,
    DiesThisWay,
}

const IT_TAG: &str = "__it__";

fn tokenize_line(line: &str, line_index: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut word_start: Option<usize> = None;
    let mut word_end: usize = 0;

    let flush = |buffer: &mut String,
                 tokens: &mut Vec<Token>,
                 word_start: &mut Option<usize>,
                 word_end: &mut usize| {
        if !buffer.is_empty() {
            let start = word_start.unwrap_or(0);
            tokens.push(Token::Word(
                buffer.clone(),
                TextSpan {
                    line: line_index,
                    start,
                    end: *word_end,
                },
            ));
            buffer.clear();
        }
        *word_start = None;
        *word_end = 0;
    };

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for (idx, (byte_idx, mut ch)) in chars.iter().copied().enumerate() {
        if ch == '−' {
            ch = '-';
        }
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match ch {
            '+' | '-' => next.is_ascii_digit(),
            '/' => prev.is_ascii_digit() && (next.is_ascii_digit() || next == '-' || next == '+'),
            _ => false,
        };

        if ch.is_ascii_alphanumeric() || is_counter_char {
            if word_start.is_none() {
                word_start = Some(byte_idx);
            }
            word_end = byte_idx + ch.len_utf8();
            buffer.push(ch.to_ascii_lowercase());
            continue;
        }

        if ch == '\'' {
            if word_start.is_some() {
                word_end = byte_idx + ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);

        let span = TextSpan {
            line: line_index,
            start: byte_idx,
            end: byte_idx + ch.len_utf8(),
        };

        match ch {
            ',' => tokens.push(Token::Comma(span)),
            '.' => tokens.push(Token::Period(span)),
            ':' => tokens.push(Token::Colon(span)),
            ';' => tokens.push(Token::Semicolon(span)),
            _ => {}
        }
    }

    flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
    tokens
}

fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("mana cost:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::ManaCost(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type line:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("power/toughness:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::PowerToughness(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("loyalty:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Loyalty(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("defense:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Defense(value.to_string())));
    }

    Ok(None)
}

#[derive(Debug, Clone)]
enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().filter_map(Token::as_word).collect()
}

fn span_from_tokens(tokens: &[Token]) -> Option<TextSpan> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    let start = first.span().start;
    let end = last.span().end;
    Some(TextSpan {
        line: first.span().line,
        start,
        end,
    })
}

#[derive(Debug, Clone)]
struct NormalizedLine {
    original: String,
    normalized: String,
    char_map: Vec<usize>,
}

struct LineInfo {
    line_index: usize,
    raw_line: String,
    normalized: NormalizedLine,
}

fn replace_names_with_map(
    line: &str,
    full_name: &str,
    short_name: &str,
    base_offset: usize,
) -> (String, Vec<usize>) {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let full_bytes = full_name.as_bytes();
    let short_bytes = short_name.as_bytes();

    let mut out = String::new();
    let mut map = Vec::new();
    let mut idx = 0;

    while idx < bytes.len() {
        if !full_bytes.is_empty() && bytes[idx..].starts_with(full_bytes) {
            let name_len = full_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += full_bytes.len();
            continue;
        }
        if !short_bytes.is_empty() && bytes[idx..].starts_with(short_bytes) {
            let name_len = short_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += short_bytes.len();
            continue;
        }

        let ch = lower[idx..].chars().next().unwrap();
        out.push(ch);
        map.push(base_offset + idx);
        idx += ch.len_utf8();
    }

    (out, map)
}

fn strip_parenthetical_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut out_map = Vec::new();
    let mut depth = 0u32;
    let mut char_idx = 0usize;

    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            char_idx += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            char_idx += 1;
            continue;
        }
        if depth == 0 {
            out.push(ch);
            if let Some(mapped) = map.get(char_idx).copied() {
                out_map.push(mapped);
            }
        }
        char_idx += 1;
    }

    (out, out_map)
}

fn normalize_line_for_parse(
    line: &str,
    full_name: &str,
    short_name: &str,
) -> Option<NormalizedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (replaced, map) = replace_names_with_map(trimmed, full_name, short_name, 0);
    let (stripped, stripped_map) = strip_parenthetical_with_map(&replaced, &map);

    if stripped.trim().is_empty() {
        let is_wrapped = trimmed.starts_with('(') && trimmed.ends_with(')');
        if !is_wrapped {
            return None;
        }
        let inner = trimmed.trim_start_matches('(').trim_end_matches(')').trim();
        if inner.is_empty() {
            return None;
        }
        let should_parse = inner.contains('{') || inner.contains(':');
        if !should_parse {
            return None;
        }
        let base_offset = trimmed.find(inner).unwrap_or(0);
        let (inner_replaced, inner_map) =
            replace_names_with_map(inner, full_name, short_name, base_offset);
        return Some(NormalizedLine {
            original: trimmed.to_string(),
            normalized: inner_replaced,
            char_map: inner_map,
        });
    }

    Some(NormalizedLine {
        original: trimmed.to_string(),
        normalized: stripped,
        char_map: stripped_map,
    })
}

fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    if byte_idx == 0 {
        return 0;
    }
    let clamped = byte_idx.min(text.len());
    text[..clamped].chars().count()
}

fn map_span_to_original(
    span: TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> TextSpan {
    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char - 1 >= char_map.len() {
        start_orig
    } else {
        let last_char_idx = end_char - 1;
        let last_orig = char_map[last_char_idx];
        let last_len = original_line[last_orig..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        last_orig + last_len
    };

    TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

fn split_on_period(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Period(_)) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn split_on_comma(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn split_on_and(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_word("and") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, &str)> {
    let (prefix, rest) = line.split_once('—').or_else(|| line.split_once(" - "))?;

    let mut chapters = Vec::new();
    for part in prefix.split(',') {
        let roman = part.trim();
        if roman.is_empty() {
            continue;
        }
        let value = roman_to_int(roman)?;
        chapters.push(value);
    }

    if chapters.is_empty() {
        return None;
    }

    Some((chapters, rest.trim()))
}

fn roman_to_int(roman: &str) -> Option<u32> {
    match roman {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let lower = line.trim().to_ascii_lowercase();
    let rest = lower.strip_prefix("level ")?;
    let token = rest.split_whitespace().next()?;
    if let Some(without_plus) = token.strip_suffix('+') {
        let min = without_plus.parse::<u32>().ok()?;
        return Some((min, None));
    }
    if let Some((start, end)) = token.split_once('-') {
        let min = start.parse::<u32>().ok()?;
        let max = end.parse::<u32>().ok()?;
        return Some((min, Some(max)));
    }
    let value = token.parse::<u32>().ok()?;
    Some((value, Some(value)))
}

fn parse_line(line: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    let normalized = line
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    if normalized.contains("creature token") {
        let starts_with_pt = normalized.split_whitespace().next().is_some_and(|token| {
            let mut chars = token.chars();
            let first = chars.next();
            first.is_some_and(|ch| ch.is_ascii_digit() || ch == '*') && token.contains('/')
        });
        if starts_with_pt {
            return Ok(LineAst::Statement {
                effects: Vec::new(),
            });
        }
    }
    if normalized.starts_with("as this saga enters") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.contains("enters the battlefield tapped unless") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("choose one")
        || normalized.starts_with("choose one or more")
        || normalized.starts_with("choose two")
    {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("this spell costs") && normalized.contains("more to cast") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("devoid") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("the ring tempts you") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("play x random fast effects") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("this land enters tapped unless you have two or more opponents") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("activate only as a sorcery") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("activate only once each turn") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("this ability triggers only once each turn") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("at the beginning of your upkeep, you may pay") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if normalized.starts_with("at the beginning of your draw step, if") {
        return Ok(LineAst::Statement {
            effects: Vec::new(),
        });
    }
    if let Some((chapters, rest)) = parse_saga_chapter_prefix(&normalized) {
        let tokens = tokenize_line(rest, line_index);
        let effects = parse_effect_sentences(&tokens)?;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(chapters),
            effects,
        });
    }

    let tokens = tokenize_line(line, line_index);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty line".to_string()));
    }

    if normalized.starts_with("as an additional cost to cast this spell") {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let effect_start = if let Some(idx) = comma_idx {
            idx + 1
        } else if let Some(idx) = tokens.iter().position(|token| token.is_word("spell")) {
            idx + 1
        } else {
            tokens.len()
        };
        let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "additional cost line missing effect clause".to_string(),
            ));
        }
        let effects = parse_effect_sentences(effect_tokens)?;
        return Ok(LineAst::AdditionalCost { effects });
    }

    if let Some(ability) = parse_equip_line(&tokens)? {
        return Ok(LineAst::Ability(ability));
    }

    if let Some(ability) = parse_level_up_line(&tokens)? {
        return Ok(LineAst::Ability(ability));
    }

    if let Some(ability) = parse_cycling_line(&tokens)? {
        return Ok(LineAst::Ability(ability));
    }

    if let Some(trigger_idx) = tokens.iter().enumerate().position(|(idx, token)| {
        token.is_word("whenever")
            || token.is_word("when")
            || (token.is_word("at") && tokens.get(idx + 1).is_some_and(|next| next.is_word("the")))
    }) {
        return parse_triggered_line(&tokens[trigger_idx..]);
    }

    if let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    {
        let cost_tokens = &tokens[..colon_idx];
        if starts_with_activation_cost(cost_tokens) {
            if let Some(ability) = parse_activated_line(&tokens)? {
                return Ok(LineAst::Ability(ability));
            }
        }
    }

    if let Some(abilities) = parse_static_ability_line(&tokens)? {
        if abilities.len() == 1 {
            return Ok(LineAst::StaticAbility(
                abilities.into_iter().next().expect("single static ability"),
            ));
        }
        return Ok(LineAst::StaticAbilities(abilities));
    }

    if let Some(actions) = parse_ability_line(&tokens) {
        return Ok(LineAst::Abilities(actions));
    }

    let effects = parse_effect_sentences(&tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported line: {line}"
        )));
    }

    Ok(LineAst::Statement { effects })
}

fn starts_with_activation_cost(tokens: &[Token]) -> bool {
    let Some(word) = tokens.first().and_then(Token::as_word) else {
        return false;
    };
    if matches!(
        word,
        "tap" | "t" | "pay" | "discard" | "sacrifice" | "put" | "remove" | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    parse_mana_symbol(word).is_ok()
}

fn parse_ability_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let segments = split_on_comma(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(&segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(action) = parse_ability_phrase(&segment) {
            actions.push(action);
        } else {
            return None;
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn parse_protection_chain(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let words = words(tokens);
    if words.len() < 3 {
        return None;
    }
    if words[0] != "protection" || words[1] != "from" {
        return None;
    }

    let mut actions = Vec::new();
    for idx in 0..words.len().saturating_sub(1) {
        if words[idx] != "from" {
            continue;
        }
        let color = match words[idx + 1] {
            "white" => Some(ColorSet::WHITE),
            "blue" => Some(ColorSet::BLUE),
            "black" => Some(ColorSet::BLACK),
            "red" => Some(ColorSet::RED),
            "green" => Some(ColorSet::GREEN),
            _ => None,
        };
        if let Some(color) = color {
            actions.push(KeywordAction::ProtectionFrom(color));
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn keyword_action_to_static_ability(action: KeywordAction) -> Option<StaticAbility> {
    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
        KeywordAction::Haste => Some(StaticAbility::haste()),
        KeywordAction::Improvise => Some(StaticAbility::improvise()),
        KeywordAction::Convoke => Some(StaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => Some(StaticAbility::affinity_for_artifacts()),
        KeywordAction::Delve => Some(StaticAbility::delve()),
        KeywordAction::FirstStrike => Some(StaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(StaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(StaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(StaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(StaticAbility::vigilance()),
        KeywordAction::Trample => Some(StaticAbility::trample()),
        KeywordAction::Reach => Some(StaticAbility::reach()),
        KeywordAction::Defender => Some(StaticAbility::defender()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Ward(_) => None,
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => None,
        KeywordAction::Persist => None,
        KeywordAction::Prowess => None,
        KeywordAction::Exalted => None,
        KeywordAction::Storm => None,
        KeywordAction::Toxic(_) => None,
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::Bushido(_) => None,
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(_subtype) => None,
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Marker(name) => Some(StaticAbility::custom(name, name.to_string())),
    }
}

fn parse_static_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if let Some(ability) = parse_characteristic_defining_pt_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_no_maximum_hand_size_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_library_of_leng_discard_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_toph_first_metalbender_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_discard_or_redirect_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_pay_life_or_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_copy_activated_abilities_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_source_activation_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enchanted_has_activated_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_shuffle_into_library_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_permanents_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_cant_cycle_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_starting_life_bonus_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_buyback_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_spells_cost_modifier_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_skip_upkeep_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_are_artifacts_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_cards_spells_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_creatures_are_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_blood_moon_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_remove_snow_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_creatures_have_haste_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_lose_all_abilities_and_base_pt_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_creatures_lose_flying_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_anthem_and_indestructible_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_have_indestructible_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_doesnt_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_equipped_creature_has_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_attacks_each_combat_if_able_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_anthem_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_flying_restriction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_assign_damage_as_unblocked_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_grant_flash_to_noncreature_spells_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_creatures_cant_block_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_with_additional_counter_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_additional_land_play_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_play_lands_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_cant_clauses(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}

fn parse_characteristic_defining_pt_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let has_this_pt = words.windows(4).any(|window| {
        window == ["this", "power", "and", "toughness"]
            || window == ["thiss", "power", "and", "toughness"]
    });
    if !has_this_pt {
        return Ok(None);
    }
    if !(words.contains(&"equal") && words.contains(&"number") && words.contains(&"of")) {
        return Ok(None);
    }

    let Some(number_idx) = tokens.iter().position(|token| token.is_word("number")) else {
        return Ok(None);
    };
    if !tokens
        .get(number_idx + 1)
        .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[number_idx + 2..];
    while filter_tokens
        .last()
        .is_some_and(|token| token.is_word("respectively") || matches!(token, Token::Period(_)))
    {
        filter_tokens = &filter_tokens[..filter_tokens.len().saturating_sub(1)];
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    let value = Value::Count(filter);
    Ok(Some(StaticAbility::characteristic_defining_pt(
        value.clone(),
        value,
    )))
}

fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_would_be_put = words
        .windows(3)
        .any(|window| window == ["would", "be", "put"]);
    let has_graveyard = words.contains(&"graveyard");
    let has_anywhere = words.contains(&"anywhere");
    let has_shuffle = words.contains(&"shuffle");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");

    if has_would_be_put
        && has_graveyard
        && has_anywhere
        && has_shuffle
        && has_library
        && has_instead
    {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

fn parse_permanents_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["permanents", "enter", "tapped"]
        || words.as_slice() == ["permanents", "enters", "tapped"]
    {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

fn parse_players_cant_cycle_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "cant", "cycle", "cards"] {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

fn parse_starting_life_bonus_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 || !words.starts_with(&["you", "start", "the", "game"]) {
        return Ok(None);
    }
    if !words.contains(&"additional") || !words.contains(&"life") {
        return Ok(None);
    }
    let mut amount = None;
    for (idx, _token) in tokens.iter().enumerate() {
        if let Some((value, _)) = parse_number(&tokens[idx..]) {
            amount = Some(value);
            break;
        }
    }
    let amount = amount
        .ok_or_else(|| CardTextError::ParseError("missing starting life amount".to_string()))?;
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

fn parse_buyback_cost_reduction_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 5 || !words.starts_with(&["buyback", "costs", "cost"]) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !words.contains(&"less") {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

fn parse_spells_cost_modifier_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let spells_idx = clause_words
        .iter()
        .position(|word| *word == "spell" || *word == "spells");
    let Some(spells_idx) = spells_idx else {
        return Ok(None);
    };
    let cost_idx = clause_words
        .iter()
        .position(|word| *word == "cost" || *word == "costs");
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    if cost_idx <= spells_idx {
        return Ok(None);
    }

    let mut filter = parse_spell_filter(&tokens[..spells_idx]);

    let between_words = &clause_words[spells_idx + 1..cost_idx];
    if between_words
        .windows(2)
        .any(|window| window == ["you", "cast"])
    {
        filter.controller = Some(PlayerFilter::You);
    }

    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }

    if let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        amount_value = dynamic_value;
    } else if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    if is_less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

fn parse_cost_modifier_amount(tokens: &[Token]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let word = tokens.first().and_then(Token::as_word)?;
    let symbol = parse_mana_symbol(word).ok()?;
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    None
}

fn parse_dynamic_cost_modifier_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let words_all = words(tokens);
    let Some(each_idx) = words_all.iter().position(|word| *word == "each") else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = words(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }

    if filter_words.windows(2).any(|pair| pair == ["card", "type"])
        && filter_words.contains(&"graveyard")
    {
        let player = if filter_words
            .windows(2)
            .any(|pair| pair == ["your", "graveyard"])
        {
            PlayerFilter::You
        } else if filter_words
            .windows(2)
            .any(|pair| pair == ["opponents", "graveyard"] || pair == ["opponent", "graveyard"])
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

fn parse_players_skip_upkeep_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "skip", "their", "upkeep", "steps"] {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

fn parse_legend_rule_doesnt_apply_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"legend") && words.contains(&"rule") && words.contains(&"doesnt") {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

fn parse_all_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "permanents", "are", "colorless"] {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

fn parse_all_permanents_are_artifacts_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&["all", "permanents", "are", "artifacts"]) {
        return Ok(Some(StaticAbility::add_card_types(
            ObjectFilter::permanent(),
            vec![CardType::Artifact],
        )));
    }
    Ok(None)
}

fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"colorless")
        && words.contains(&"cards")
        && words.contains(&"spells")
        && words.contains(&"permanents")
    {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

fn parse_all_creatures_are_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let are_idx = words.iter().position(|word| *word == "are");
    let Some(are_idx) = are_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..are_idx];
    let filter = parse_object_filter(subject_tokens, false)?;

    let color_word = words.get(are_idx + 1).copied();
    let Some(color_word) = color_word else {
        return Ok(None);
    };
    let Some(color) = parse_color(color_word) else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

fn parse_blood_moon_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["nonbasic", "lands", "are", "mountains"] {
        return Ok(Some(StaticAbility::blood_moon()));
    }
    Ok(None)
}

fn parse_remove_snow_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "lands", "are", "no", "longer", "snow"] {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

fn parse_all_creatures_have_haste_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };

    if !words.iter().skip(have_idx + 1).any(|word| *word == "haste") {
        return Ok(None);
    }

    let subject_words = &words[..have_idx];
    if subject_words.contains(&"equipped") || subject_words.contains(&"enchanted") {
        return Ok(None);
    }
    let filter = parse_object_filter(&tokens[..have_idx], false)?;
    Ok(Some(StaticAbility::grant_ability(
        filter,
        StaticAbility::haste(),
    )))
}

fn parse_all_creatures_lose_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "creatures", "lose", "flying"] {
        return Ok(Some(StaticAbility::remove_ability(
            ObjectFilter::creature(),
            StaticAbility::flying(),
        )));
    }
    Ok(None)
}

fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let lose_idx = words
        .iter()
        .position(|word| *word == "lose" || *word == "loses");
    let Some(lose_idx) = lose_idx else {
        return Ok(None);
    };

    if !words[lose_idx + 1..]
        .windows(2)
        .any(|window| window == ["all", "abilities"])
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..lose_idx];
    let filter = match parse_object_filter(subject_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let mut abilities = vec![StaticAbility::remove_all_abilities(filter.clone())];

    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    if let Some(have_idx) = have_idx {
        let after_have = &words[have_idx + 1..];
        if after_have.starts_with(&["base", "power", "and", "toughness"])
            && let Some(modifier_token) = after_have.iter().find(|word| word.contains('/'))
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

fn parse_all_have_indestructible_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };

    if !words
        .iter()
        .skip(have_idx + 1)
        .any(|word| *word == "indestructible")
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..have_idx], false)?;
    Ok(Some(StaticAbility::grant_ability(
        filter,
        StaticAbility::indestructible(),
    )))
}

fn parse_anthem_and_indestructible_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");

    let (Some(get_idx), Some(have_idx)) = (get_idx, have_idx) else {
        return Ok(None);
    };

    if have_idx <= get_idx {
        return Ok(None);
    }

    if !words
        .iter()
        .skip(have_idx + 1)
        .any(|word| *word == "indestructible")
    {
        return Ok(None);
    }

    let filter_tokens = &tokens[..get_idx];
    let filter = parse_object_filter(filter_tokens, false)?;

    let modifier_token = tokens.get(get_idx + 1).and_then(Token::as_word);
    let Some(modifier_token) = modifier_token else {
        return Ok(None);
    };
    let Ok((power, toughness)) = parse_pt_modifier(modifier_token) else {
        return Ok(None);
    };

    Ok(Some(vec![
        StaticAbility::anthem(filter.clone(), power, toughness),
        StaticAbility::grant_ability(filter, StaticAbility::indestructible()),
    ]))
}

fn parse_anthem_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };

    let filter_tokens = &tokens[..get_idx];
    let filter = parse_object_filter(filter_tokens, false)?;

    let modifier_token = tokens.get(get_idx + 1).and_then(Token::as_word);
    let Some(modifier_token) = modifier_token else {
        return Ok(None);
    };

    let Ok((power, toughness)) = parse_pt_modifier(modifier_token) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::anthem(filter, power, toughness)))
}

fn parse_enters_tapped_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enter_idx = words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_idx) = enter_idx else {
        return Ok(None);
    };
    if !words
        .iter()
        .skip(enter_idx + 1)
        .any(|word| *word == "tapped")
    {
        return Ok(None);
    }
    if words.first().copied() == Some("this") {
        return Ok(None);
    }
    let filter = parse_object_filter(&tokens[..enter_idx], false)?;
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enter_idx = words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_idx) = enter_idx else {
        return Ok(None);
    };

    if words.first().copied() == Some("this") {
        return Ok(None);
    }
    if !words.contains(&"battlefield")
        || !words.contains(&"with")
        || !words.contains(&"additional")
        || !words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..enter_idx], false)?;

    let additional_idx = tokens
        .iter()
        .position(|token| token.is_word("additional"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let mut count = 1u32;
    if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&tokens[additional_idx - 1..additional_idx])
    {
        count = parsed;
    } else if let Some((parsed, _)) = parse_number(&tokens[additional_idx + 1..]) {
        count = parsed;
    }

    let counter_type = parse_counter_type_from_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(StaticAbility::enters_with_counters_for_filter(
        filter,
        counter_type,
        count,
    )))
}

fn parse_creatures_cant_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["creatures", "cant", "block"] {
        return Ok(Some(StaticAbility::grant_ability(
            ObjectFilter::creature(),
            StaticAbility::cant_block(),
        )));
    }
    Ok(None)
}

fn parse_equipped_creature_has_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 || words[0] != "equipped" || words[1] != "creature" || words[2] != "has" {
        return Ok(None);
    }

    let ability_tokens = &tokens[3..];
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for segment in split_on_and(ability_tokens) {
        if segment.is_empty() {
            continue;
        }
        let Some(action) = parse_ability_phrase(&segment) else {
            return Ok(None);
        };
        if let Some(ability) = keyword_action_to_static_ability(action) {
            abilities.push(ability);
        }
    }

    if abilities.is_empty() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::equipment_grant(abilities)))
}

fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let matches = words
        .starts_with(&["this", "doesnt", "untap", "during", "your", "untap", "step"])
        || words.starts_with(&[
            "this", "doesn't", "untap", "during", "your", "untap", "step",
        ])
        || words.starts_with(&[
            "this", "does", "not", "untap", "during", "your", "untap", "step",
        ]);

    if matches {
        return Ok(Some(StaticAbility::doesnt_untap()));
    }

    Ok(None)
}

fn parse_flying_restriction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ];

    if matches {
        return Ok(Some(StaticAbility::flying_restriction()));
    }

    Ok(None)
}

fn parse_assign_damage_as_unblocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    if normalized.first().copied() != Some("you") {
        return Ok(None);
    }

    let mut idx = 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"have") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) != Some(&"this") {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "assign", "its", "combat", "damage", "as", "though", "it", "werent", "blocked",
        ] || tail
            == [
                "assign", "its", "combat", "damage", "as", "though", "it", "wasnt", "blocked",
            ];

    if matches {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let mut idx = 0;
    if normalized.get(idx) != Some(&"you") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"cast") {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "noncreature",
            "spells",
            "as",
            "though",
            "they",
            "had",
            "flash",
        ] || tail
            == [
                "noncreature",
                "spells",
                "as",
                "though",
                "they",
                "have",
                "flash",
            ];

    if matches {
        return Ok(Some(StaticAbility::grants(
            crate::grant::GrantSpec::flash_to_noncreature_spells(),
        )));
    }

    Ok(None)
}

fn parse_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["this", "attacks", "each", "combat", "if", "able"]
        || words.as_slice() == ["attacks", "each", "combat", "if", "able"]
        || words.len() >= 5
            && words[words.len() - 5..] == ["attacks", "each", "combat", "if", "able"]
    {
        return Ok(Some(StaticAbility::must_attack()));
    }
    Ok(None)
}

fn parse_additional_land_play_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "you",
            "may",
            "play",
            "an",
            "additional",
            "land",
            "on",
            "each",
            "of",
            "your",
            "turns",
        ]
    {
        return Ok(Some(StaticAbility::additional_land_play()));
    }
    Ok(None)
}

fn parse_play_lands_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "may", "play", "lands", "from", "your", "graveyard"] {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            ObjectFilter::land(),
            Zone::Graveyard,
        );
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }
    let power_str = parts[0].trim_start_matches('+');
    let toughness_str = parts[1].trim_start_matches('+');
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

fn parse_no_maximum_hand_size_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "have", "no", "maximum", "hand", "size"] {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

fn parse_library_of_leng_discard_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_effect_causes = words.windows(3).any(|w| w == ["effect", "causes", "you"]);
    let has_discard = words.contains(&"discard");
    let has_top = words.contains(&"top");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_effect_causes && has_discard && has_top && has_library && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::library_of_leng_discard_replacement()));
    }

    Ok(None)
}

fn parse_toph_first_metalbender_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_nontoken = words.contains(&"nontoken");
    let has_artifact = words
        .iter()
        .any(|word| *word == "artifact" || *word == "artifacts");
    let has_you_control = words
        .windows(2)
        .any(|pair| pair == ["you", "control"] || pair == ["you", "controls"]);
    let has_land = words.iter().any(|word| *word == "land" || *word == "lands");
    let has_addition = words
        .windows(4)
        .any(|pair| pair == ["in", "addition", "to", "their"]);

    if has_nontoken && has_artifact && has_you_control && has_land && has_addition {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::TophFirstMetalbender,
        )));
    }

    Ok(None)
}

fn parse_discard_or_redirect_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_enter = words
        .iter()
        .any(|word| *word == "enter" || *word == "enters");
    let has_battlefield = words.contains(&"battlefield");
    let has_discard = words.contains(&"discard");
    let has_land = words.contains(&"land");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_enter && has_battlefield && has_discard && has_land && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::discard_or_redirect_replacement(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )));
    }

    Ok(None)
}

fn parse_pay_life_or_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    let starts_with_as_enters = words.starts_with(&["as", "this", "enters", "the", "battlefield"]);
    let has_pay = words.contains(&"pay");
    let has_life = words.contains(&"life");
    let has_tapped = words.contains(&"tapped");

    if !starts_with_as_enters || !has_pay || !has_life || !has_tapped {
        return Ok(None);
    }

    let pay_idx = tokens.iter().position(|token| token.is_word("pay"));
    let Some(pay_idx) = pay_idx else {
        return Ok(None);
    };
    let amount_tokens = &tokens[pay_idx + 1..];
    let Some((value, _)) = parse_number(amount_tokens) else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::pay_life_or_enter_tapped(value)))
}

fn parse_copy_activated_abilities_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..words.len().saturating_sub(4) {
        if words[idx] == "has"
            && words[idx + 1] == "all"
            && words[idx + 2] == "activated"
            && words[idx + 3] == "abilities"
            && words[idx + 4] == "of"
        {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };

    let mut condition = None;
    let prefix = &words[..has_idx];
    if prefix.starts_with(&["as", "long", "as"])
        && prefix.contains(&"own")
        && prefix.contains(&"exiled")
        && prefix.contains(&"counter")
    {
        if let Some(counter_word) = prefix
            .iter()
            .zip(prefix.iter().skip(1))
            .find_map(|(word, next)| {
                if *next == "counter" {
                    Some(*word)
                } else {
                    None
                }
            })
            .and_then(parse_counter_type_word)
        {
            condition = Some(
                crate::static_abilities::CopyActivatedAbilitiesCondition::OwnsCardExiledWithCounter(
                    counter_word,
                ),
            );
        }
    }

    let after_of = &words[(has_idx + 5)..];
    let mut filter = None;
    if after_of.contains(&"land") || after_of.contains(&"lands") {
        filter = Some(ObjectFilter::land());
    } else if after_of.contains(&"creature") || after_of.contains(&"creatures") {
        let mut base = ObjectFilter::creature();
        if after_of.contains(&"control") {
            base = base.you_control();
        }
        filter = Some(base);
    } else if after_of.contains(&"card") && after_of.contains(&"exiled") {
        filter = Some(ObjectFilter {
            zone: Some(Zone::Exile),
            ..Default::default()
        });
    }

    let Some(filter) = filter else {
        return Ok(None);
    };

    let counter = after_of
        .iter()
        .zip(after_of.iter().skip(1))
        .find_map(|(word, next)| {
            if *next == "counter" {
                parse_counter_type_word(word)
            } else {
                None
            }
        });

    let exclude_source_name = words.windows(5).any(|window| {
        window == ["same", "name", "as", "this", "creature"]
            || window == ["same", "name", "as", "thiss", "creature"]
    });

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(words.join(" "));
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }

    Ok(Some(StaticAbility::copy_activated_abilities(ability)))
}

fn parse_players_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&[
        "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color",
    ]) {
        return Ok(Some(StaticAbility::spend_mana_as_any_color_players()));
    }

    Ok(None)
}

fn parse_source_activation_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&[
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
        "pay",
        "the",
        "activation",
        "costs",
        "of",
    ]) {
        return Ok(None);
    }

    if words
        .iter()
        .any(|word| *word == "abilities" || *word == "ability")
    {
        return Ok(Some(
            StaticAbility::spend_mana_as_any_color_activation_costs(),
        ));
    }

    Ok(None)
}

fn parse_enchanted_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["enchanted"]) || !token_words.contains(&"has") {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let ability_tokens = &tokens[has_idx + 1..];
    let Some(parsed) = parse_activated_line(ability_tokens)? else {
        return Ok(None);
    };

    let mut ability = parsed.ability;
    if ability.text.is_none() {
        let text_words = words(ability_tokens).join(" ");
        ability.text = Some(text_words);
    }

    Ok(Some(StaticAbility::attached_ability_grant(
        ability,
        token_words.join(" "),
    )))
}

fn parse_activated_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    else {
        return Ok(None);
    };

    let cost_tokens = &tokens[..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }

    let mut effect_sentences = split_on_period(effect_tokens);
    let mut timing = ActivationTiming::AnyTime;
    effect_sentences.retain(|sentence| {
        if is_activate_only_as_sorcery(sentence) {
            timing = ActivationTiming::SorcerySpeed;
            false
        } else if is_activate_only_once_each_turn(sentence) {
            timing = ActivationTiming::OncePerTurn;
            false
        } else {
            true
        }
    });
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let effect_words = words(primary_sentence);
        if effect_words.contains(&"add") {
            let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
            let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

            let mut extra_effects = Vec::new();
            let mut extra_effects_ast = Vec::new();
            let mut activation_condition = None;
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    if let Some(condition) = parse_activation_condition(sentence) {
                        activation_condition = Some(condition);
                        continue;
                    }
                    let ast = parse_effect_sentence(sentence)?;
                    let compiled = compile_statement_effects(&ast)?;
                    extra_effects.extend(compiled);
                    extra_effects_ast.extend(ast);
                }
            }

            let add_idx = effect_words
                .iter()
                .position(|word| *word == "add")
                .unwrap_or(0);
            let mana_words = &effect_words[add_idx + 1..];

            let has_imprinted_colors = mana_words.contains(&"exiled")
                && (mana_words.contains(&"card") || mana_words.contains(&"cards"))
                && mana_words
                    .iter()
                    .any(|word| *word == "color" || *word == "colors");
            let has_any_color = mana_words.contains(&"any") && mana_words.contains(&"color");
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && mana_words.contains(&"identity");
            if has_imprinted_colors {
                let mut effects = vec![Effect::new(
                    crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
                )];
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                };
                if let Some(condition) = activation_condition
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let effects_ast = if extra_effects_ast.is_empty() {
                    None
                } else {
                    Some(extra_effects_ast)
                };
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast,
                }));
            }
            if has_any_color {
                let amount = if mana_words.contains(&"two") {
                    2
                } else if mana_words.contains(&"three") {
                    3
                } else {
                    1
                };
                let mut effects = if uses_commander_identity {
                    vec![Effect::add_mana_from_commander_color_identity(amount)]
                } else {
                    vec![Effect::add_mana_of_any_color(amount)]
                };
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                };
                if let Some(condition) = activation_condition
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let effects_ast = if extra_effects_ast.is_empty() {
                    None
                } else {
                    Some(extra_effects_ast)
                };
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast,
                }));
            }

            let mut mana = Vec::new();
            for word in mana_words {
                if *word == "mana" || *word == "to" || *word == "your" || *word == "pool" {
                    continue;
                }
                if let Ok(symbol) = parse_mana_symbol(word) {
                    mana.push(symbol);
                }
            }

            if !mana.is_empty() {
                if extra_effects.is_empty() {
                    let mut ability = Ability {
                        kind: AbilityKind::Mana(ManaAbility {
                            mana_cost,
                            mana,
                            effects: None,
                            activation_condition: None,
                        }),
                        functional_zones: vec![Zone::Battlefield],
                        text: None,
                    };
                    if let Some(condition) = activation_condition
                        && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                    {
                        mana_ability.activation_condition = Some(condition);
                    }
                    let effects_ast = if extra_effects_ast.is_empty() {
                        None
                    } else {
                        Some(extra_effects_ast)
                    };
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast,
                    }));
                }
                let mut effects = vec![Effect::add_mana(mana)];
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                };
                if let Some(condition) = activation_condition
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let effects_ast = if extra_effects_ast.is_empty() {
                    None
                } else {
                    Some(extra_effects_ast)
                };
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
    let mut effects_ast = Vec::new();
    for sentence in &effect_sentences {
        if sentence.is_empty() {
            continue;
        }
        let parsed = parse_effect_sentence(sentence)?;
        effects_ast.extend(parsed);
    }
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let (effects, choices) = compile_trigger_effects(None, &effects_ast)?;
    let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects,
                choices,
                timing,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        },
        effects_ast: Some(effects_ast),
    }))
}

fn is_activate_only_as_sorcery(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["activate", "only", "as", "a", "sorcery"]
}

fn is_activate_only_once_each_turn(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["activate", "only", "once", "each", "turn"]
}

fn parse_level_up_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 3 || words[0] != "level" || words[1] != "up" {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    for word in words.iter().skip(2) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            symbols.push(symbol);
        }
    }

    if symbols.is_empty() {
        return Err(CardTextError::ParseError(
            "level up missing mana cost".to_string(),
        ));
    }

    let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
    let mana_cost = ManaCost::from_pips(pips);

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: vec![Effect::put_counters_on_source(CounterType::Level, 1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some("Level up".to_string()),
        },
        effects_ast: None,
    }))
}

fn parse_cycling_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
    if words_all.is_empty() {
        return Ok(None);
    }

    let Some(cycling_idx) = words_all.iter().position(|word| word.ends_with("cycling")) else {
        return Ok(None);
    };

    let cost_start = cycling_idx + 1;
    if cost_start >= tokens.len() {
        return Ok(None);
    }

    let cost_end = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .unwrap_or(tokens.len());
    if cost_end <= cost_start {
        return Ok(None);
    }

    let (mana_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..cost_end])?;
    let mut full_cost_effects = cost_effects;
    full_cost_effects.push(Effect::discard(1));
    let mana_cost = crate::ability::merge_cost_effects(mana_cost, full_cost_effects);

    let cycling_tokens = &tokens[..cost_start];
    let search_filter = parse_cycling_search_filter(cycling_tokens)?;
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: vec![effect],
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(words_all.join(" ")),
        },
        effects_ast: None,
    }))
}

fn parse_cycling_search_filter(tokens: &[Token]) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let keyword = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("missing cycling keyword".to_string()))?;
    let mut filter = ObjectFilter::default();

    for word in &words[..words.len().saturating_sub(1)] {
        if let Some(supertype) = parse_supertype_word(word)
            && !filter.supertypes.contains(&supertype)
        {
            filter.supertypes.push(supertype);
        }
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    if keyword == "cycling" {
        return Ok(None);
    }

    if keyword == "landcycling" {
        if !filter.card_types.contains(&CardType::Land) {
            filter.card_types.push(CardType::Land);
        }
        return Ok(Some(filter));
    }

    if let Some(root) = keyword.strip_suffix("cycling") {
        if let Some(card_type) = parse_card_type(root)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        } else if let Some(subtype) =
            parse_subtype_word(root).or_else(|| root.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        } else if let Some(color) = parse_color(root) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(Some(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported cycling variant (clause: '{}')",
        words.join(" ")
    )))
}

fn is_land_subtype(subtype: Subtype) -> bool {
    matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    )
}

fn parse_equip_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("equip") {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    let mut saw_zero = false;
    for word in words.iter().skip(1) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            if matches!(symbol, ManaSymbol::Generic(0)) {
                saw_zero = true;
            } else {
                symbols.push(symbol);
            }
        }
    }

    if symbols.is_empty() && !saw_zero {
        return Err(CardTextError::ParseError(
            "equip missing mana cost".to_string(),
        ));
    }

    let mana_cost = if symbols.is_empty() {
        ManaCost::new()
    } else {
        let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
        ManaCost::from_pips(pips)
    };
    let total_cost = if mana_cost.pips().is_empty() {
        TotalCost::free()
    } else {
        TotalCost::mana(mana_cost)
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: total_cost,
                effects: vec![Effect::attach_to(target.clone())],
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some("Equip".to_string()),
        },
        effects_ast: None,
    }))
}

fn parse_activation_cost(tokens: &[Token]) -> Result<(TotalCost, Vec<Effect>), CardTextError> {
    let mut mana_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut cost_effects = Vec::new();
    let mut explicit_costs = Vec::new();
    let mut energy_count: u32 = 0;
    let mut sac_tag_id = 0u32;

    for segment in split_on_comma(tokens) {
        if segment.is_empty() {
            continue;
        }
        let words = words(&segment);
        if words.is_empty() {
            continue;
        }

        if words[0] == "tap" || words[0] == "t" {
            cost_effects.push(Effect::tap_source());
            continue;
        }

        if words[0] == "pay" {
            let amount = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse pay life cost".to_string())
            })?;
            if !words.contains(&"life") {
                return Err(CardTextError::ParseError(
                    "unsupported pay cost (expected life)".to_string(),
                ));
            }
            cost_effects.push(Effect::pay_life(amount.0));
            continue;
        }

        if words[0] == "discard" {
            let count = parse_number(&segment[1..])
                .map(|value| value.0)
                .unwrap_or(1);
            cost_effects.push(Effect::discard(count));
            continue;
        }

        if words[0] == "sacrifice" {
            if words.get(1).copied() == Some("this") {
                cost_effects.push(Effect::sacrifice_source());
                continue;
            }
            let mut idx = 1;
            let mut other = false;
            if segment
                .get(idx)
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                idx += 1;
            }
            let filter_tokens = &segment[idx..];
            let mut filter = parse_object_filter(filter_tokens, other)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            let tag = format!("sacrifice_cost_{sac_tag_id}");
            sac_tag_id += 1;
            cost_effects.push(Effect::choose_objects(
                filter,
                1,
                PlayerFilter::You,
                tag.clone(),
            ));
            cost_effects.push(Effect::sacrifice(ObjectFilter::tagged(tag), 1));
            continue;
        }

        if words[0] == "put" {
            let (count, used) = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse put counter cost amount".to_string())
            })?;
            let counter_type =
                parse_counter_type_from_tokens(&segment[1 + used..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counter type in activation cost (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            explicit_costs.push(crate::costs::Cost::add_counters(counter_type, count));
            continue;
        }

        if words[0] == "remove" {
            let (count, used) = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse remove counter cost amount".to_string())
            })?;
            let mut idx = 1 + used;
            let counter_idx = segment[idx..]
                .iter()
                .position(|token| token.is_word("counter") || token.is_word("counters"))
                .map(|offset| idx + offset)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing counter keyword in activation cost (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            idx = counter_idx + 1;
            if segment.get(idx).is_some_and(|token| token.is_word("from")) {
                idx += 1;
            }
            if segment.get(idx).is_some_and(|token| token.is_word("among")) {
                idx += 1;
            }
            if idx >= segment.len() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter for remove-counter cost (clause: '{}')",
                    words.join(" ")
                )));
            }
            let mut filter = parse_object_filter(&segment[idx..], false)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            explicit_costs.push(crate::costs::Cost::new(
                crate::costs::RemoveAnyCountersAmongCost::new(count, filter),
            ));
            continue;
        }

        // Otherwise, treat as pure mana symbols.
        for word in &words {
            if *word == "e" {
                energy_count = energy_count.saturating_add(1);
                continue;
            }
            if word.contains('/') {
                let alternatives = parse_mana_symbol_group(word)?;
                mana_pips.push(alternatives);
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana_pips.push(vec![symbol]);
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activation cost segment (clause: '{}')",
                words.join(" ")
            )));
        }
    }

    let mut costs = Vec::new();
    if !mana_pips.is_empty() {
        costs.push(crate::costs::Cost::mana(ManaCost::from_pips(mana_pips)));
    }
    if energy_count > 0 {
        costs.push(crate::costs::Cost::energy(energy_count));
    }
    costs.extend(explicit_costs);

    let total_cost = if costs.is_empty() {
        TotalCost::free()
    } else {
        TotalCost::from_costs(costs)
    };

    Ok((total_cost, cost_effects))
}

fn parse_activation_condition(tokens: &[Token]) -> Option<ManaAbilityCondition> {
    let words = words(tokens);
    if words.len() < 5 {
        return None;
    }
    if !words.starts_with(&["activate", "only", "if", "you", "control"]) {
        return None;
    }

    let mut subtypes = Vec::new();
    for word in words {
        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }

    if subtypes.is_empty() {
        return None;
    }

    Some(ManaAbilityCondition::ControlLandWithSubtype(subtypes))
}

fn parse_enters_tapped_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }
    if words.first().copied() == Some("this")
        && words.contains(&"enters")
        && words.contains(&"tapped")
    {
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    Ok(None)
}

fn parse_cost_reduction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if !line_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let amount_fixed = if let Value::Fixed(value) = amount_value {
        value
    } else {
        1
    };

    let remaining_tokens = &tokens[costs_idx + 1 + used..];
    let remaining_words: Vec<&str> = words(remaining_tokens);

    if !remaining_words.contains(&"less") {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        let reduction = crate::static_abilities::CostReduction::new(
            crate::ability::SpellFilter::default(),
            dynamic,
        );
        return Ok(Some(StaticAbility::new(reduction)));
    }

    if parsed_amount.is_none() {
        return Ok(None);
    }

    let has_each = remaining_words.contains(&"each");
    let has_card_type = remaining_words
        .windows(2)
        .any(|pair| pair == ["card", "type"]);
    let has_graveyard = remaining_words.contains(&"graveyard");

    if has_each && has_card_type && has_graveyard {
        if amount_fixed != 1 {
            return Ok(None);
        }
        let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
        let cost_reduction = crate::static_abilities::CostReduction::new(
            crate::ability::SpellFilter::default(),
            reduction,
        );
        return Ok(Some(StaticAbility::new(cost_reduction)));
    }

    Ok(None)
}

fn parse_cant_clauses(tokens: &[Token]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }

        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Ok(None);
            };
            abilities.push(ability);
        }

        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

fn parse_cant_clause(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let ability = match normalized.as_slice() {
        ["players", "cant", "gain", "life"] => StaticAbility::players_cant_gain_life(),
        ["players", "cant", "search", "libraries"] => StaticAbility::players_cant_search(),
        ["damage", "cant", "be", "prevented"] => StaticAbility::damage_cant_be_prevented(),
        ["you", "cant", "lose", "the", "game"] => StaticAbility::you_cant_lose_game(),
        ["your", "opponents", "cant", "win", "the", "game"] => {
            StaticAbility::opponents_cant_win_game()
        }
        ["your", "life", "total", "cant", "change"] => StaticAbility::your_life_total_cant_change(),
        ["your", "opponents", "cant", "cast", "spells"] => {
            StaticAbility::opponents_cant_cast_spells()
        }
        [
            "your",
            "opponents",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => StaticAbility::opponents_cant_draw_extra_cards(),
        ["counters", "cant", "be", "put", "on", "this", "permanent"] => {
            StaticAbility::cant_have_counters_placed()
        }
        ["this", "spell", "cant", "be", "countered"] => StaticAbility::cant_be_countered_ability(),
        ["this", "creature", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "block"] => StaticAbility::cant_block(),
        ["permanents", "you", "control", "cant", "be", "sacrificed"] => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        ["this", "creature", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        _ => return Ok(None),
    };

    Ok(Some(ability))
}

fn parse_cant_restrictions(
    tokens: &[Token],
) -> Result<Option<Vec<crate::effect::Restriction>>, CardTextError> {
    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }

        let mut restrictions = Vec::new();
        for segment in segments {
            let Some(restriction) = parse_cant_restriction_clause(&segment)? else {
                return Ok(None);
            };
            restrictions.push(restriction);
        }

        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

fn parse_cant_restriction_clause(
    tokens: &[Token],
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let restriction = match normalized.as_slice() {
        ["players", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Any),
        ["players", "cant", "search", "libraries"] => {
            Restriction::search_libraries(PlayerFilter::Any)
        }
        ["players", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::Any),
        ["players", "cant", "cast", "spells"] => Restriction::cast_spells(PlayerFilter::Any),
        [
            "players",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => Restriction::draw_extra_cards(PlayerFilter::Any),
        ["damage", "cant", "be", "prevented"] => Restriction::prevent_damage(),
        ["you", "cant", "lose", "the", "game"] => Restriction::lose_game(PlayerFilter::You),
        ["your", "opponents", "cant", "win", "the", "game"] => {
            Restriction::win_game(PlayerFilter::Opponent)
        }
        ["your", "life", "total", "cant", "change"] => {
            Restriction::change_life_total(PlayerFilter::You)
        }
        ["your", "opponents", "cant", "cast", "spells"] => {
            Restriction::cast_spells(PlayerFilter::Opponent)
        }
        [
            "your",
            "opponents",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
        ["you", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::You),
        ["you", "cant", "search", "libraries"] => Restriction::search_libraries(PlayerFilter::You),
        ["you", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::You),
        ["opponents", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Opponent),
        ["opponents", "cant", "cast", "spells"] => Restriction::cast_spells(PlayerFilter::Opponent),
        _ => return Ok(None),
    };

    Ok(Some(restriction))
}

fn parse_ability_phrase(tokens: &[Token]) -> Option<KeywordAction> {
    let mut words = words(tokens);
    if words.is_empty() {
        return None;
    }

    if words.first().copied() == Some("and") {
        words.remove(0);
    }

    // Bushido appears as "Bushido N" and is often followed by reminder text.
    if words.first().copied() == Some("bushido") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Bushido(amount));
        }
        return Some(KeywordAction::Marker("bushido"));
    }

    let action = match words.as_slice() {
        ["flying"] => KeywordAction::Flying,
        ["menace"] => KeywordAction::Menace,
        ["hexproof"] => KeywordAction::Hexproof,
        ["haste"] => KeywordAction::Haste,
        ["improvise"] => KeywordAction::Improvise,
        ["convoke"] => KeywordAction::Convoke,
        ["affinity", "for", "artifacts"] => KeywordAction::AffinityForArtifacts,
        ["delve"] => KeywordAction::Delve,
        ["first", "strike"] => KeywordAction::FirstStrike,
        ["double", "strike"] => KeywordAction::DoubleStrike,
        ["deathtouch"] => KeywordAction::Deathtouch,
        ["lifelink"] => KeywordAction::Lifelink,
        ["vigilance"] => KeywordAction::Vigilance,
        ["trample"] => KeywordAction::Trample,
        ["reach"] => KeywordAction::Reach,
        ["defender"] => KeywordAction::Defender,
        ["flash"] => KeywordAction::Flash,
        ["indestructible"] => KeywordAction::Indestructible,
        ["shroud"] => KeywordAction::Shroud,
        ["ward", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Ward(value)
        }
        ["wither"] => KeywordAction::Wither,
        ["infect"] => KeywordAction::Infect,
        ["undying"] => KeywordAction::Undying,
        ["persist"] => KeywordAction::Persist,
        ["prowess"] => KeywordAction::Prowess,
        ["exalted"] => KeywordAction::Exalted,
        ["cascade"] => KeywordAction::Marker("cascade"),
        ["storm"] => KeywordAction::Storm,
        ["ascend"] => KeywordAction::Marker("ascend"),
        ["daybound"] => KeywordAction::Marker("daybound"),
        ["nightbound"] => KeywordAction::Marker("nightbound"),
        ["islandwalk"] => KeywordAction::Marker("islandwalk"),
        ["swampwalk"] => KeywordAction::Marker("swampwalk"),
        ["mountainwalk"] => KeywordAction::Marker("mountainwalk"),
        ["forestwalk"] => KeywordAction::Marker("forestwalk"),
        ["plainswalk"] => KeywordAction::Marker("plainswalk"),
        ["fear"] => KeywordAction::Fear,
        ["intimidate"] => KeywordAction::Intimidate,
        ["shadow"] => KeywordAction::Shadow,
        ["horsemanship"] => KeywordAction::Horsemanship,
        ["flanking"] => KeywordAction::Flanking,
        ["changeling"] => KeywordAction::Changeling,
        ["protection", "from", "all", "colors"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "all", "color"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "colorless"] => KeywordAction::ProtectionFromColorless,
        ["protection", "from", value] => match *value {
            "white" => KeywordAction::ProtectionFrom(ColorSet::WHITE),
            "blue" => KeywordAction::ProtectionFrom(ColorSet::BLUE),
            "black" => KeywordAction::ProtectionFrom(ColorSet::BLACK),
            "red" => KeywordAction::ProtectionFrom(ColorSet::RED),
            "green" => KeywordAction::ProtectionFrom(ColorSet::GREEN),
            _ => {
                if let Some(card_type) = parse_card_type(value) {
                    KeywordAction::ProtectionFromCardType(card_type)
                } else if let Some(subtype) = parse_subtype_word(value)
                    .or_else(|| value.strip_suffix('s').and_then(parse_subtype_word))
                {
                    KeywordAction::ProtectionFromSubtype(subtype)
                } else {
                    return None;
                }
            }
        },
        _ => {
            if !words.is_empty() {
                match words[0] {
                    "flying" => return Some(KeywordAction::Flying),
                    "menace" => return Some(KeywordAction::Menace),
                    "hexproof" => return Some(KeywordAction::Hexproof),
                    "haste" => return Some(KeywordAction::Haste),
                    "improvise" => return Some(KeywordAction::Improvise),
                    "convoke" => return Some(KeywordAction::Convoke),
                    "delve" => return Some(KeywordAction::Delve),
                    "deathtouch" => return Some(KeywordAction::Deathtouch),
                    "lifelink" => return Some(KeywordAction::Lifelink),
                    "vigilance" => return Some(KeywordAction::Vigilance),
                    "trample" => return Some(KeywordAction::Trample),
                    "reach" => return Some(KeywordAction::Reach),
                    "defender" => return Some(KeywordAction::Defender),
                    "flash" => return Some(KeywordAction::Flash),
                    "indestructible" => return Some(KeywordAction::Indestructible),
                    "shroud" => return Some(KeywordAction::Shroud),
                    "wither" => return Some(KeywordAction::Wither),
                    "infect" => return Some(KeywordAction::Infect),
                    "undying" => return Some(KeywordAction::Undying),
                    "persist" => return Some(KeywordAction::Persist),
                    "prowess" => return Some(KeywordAction::Prowess),
                    "exalted" => return Some(KeywordAction::Exalted),
                    "cascade" => return Some(KeywordAction::Marker("cascade")),
                    "storm" => return Some(KeywordAction::Storm),
                    "ascend" => return Some(KeywordAction::Marker("ascend")),
                    "daybound" => return Some(KeywordAction::Marker("daybound")),
                    "nightbound" => return Some(KeywordAction::Marker("nightbound")),
                    "islandwalk" => return Some(KeywordAction::Marker("islandwalk")),
                    "swampwalk" => return Some(KeywordAction::Marker("swampwalk")),
                    "mountainwalk" => return Some(KeywordAction::Marker("mountainwalk")),
                    "forestwalk" => return Some(KeywordAction::Marker("forestwalk")),
                    "plainswalk" => return Some(KeywordAction::Marker("plainswalk")),
                    "toxic" => {
                        let amount = words
                            .get(1)
                            .and_then(|w| w.parse::<u32>().ok())
                            .unwrap_or(1);
                        return Some(KeywordAction::Toxic(amount));
                    }
                    "fear" => return Some(KeywordAction::Fear),
                    "intimidate" => return Some(KeywordAction::Intimidate),
                    "shadow" => return Some(KeywordAction::Shadow),
                    "horsemanship" => return Some(KeywordAction::Horsemanship),
                    "flanking" => return Some(KeywordAction::Flanking),
                    "changeling" => return Some(KeywordAction::Changeling),
                    _ => {}
                }
            }
            if words.len() >= 2 {
                if words.starts_with(&["first", "strike"]) {
                    return Some(KeywordAction::FirstStrike);
                }
                if words.starts_with(&["double", "strike"]) {
                    return Some(KeywordAction::DoubleStrike);
                }
                if words.starts_with(&["protection", "from"]) && words.len() >= 3 {
                    let value = words[2];
                    return match value {
                        "white" => Some(KeywordAction::ProtectionFrom(ColorSet::WHITE)),
                        "blue" => Some(KeywordAction::ProtectionFrom(ColorSet::BLUE)),
                        "black" => Some(KeywordAction::ProtectionFrom(ColorSet::BLACK)),
                        "red" => Some(KeywordAction::ProtectionFrom(ColorSet::RED)),
                        "green" => Some(KeywordAction::ProtectionFrom(ColorSet::GREEN)),
                        _ => parse_card_type(value)
                            .map(KeywordAction::ProtectionFromCardType)
                            .or_else(|| {
                                parse_subtype_word(value)
                                    .or_else(|| {
                                        value.strip_suffix('s').and_then(parse_subtype_word)
                                    })
                                    .map(KeywordAction::ProtectionFromSubtype)
                            }),
                    };
                }
            }
            if words.len() >= 3 {
                let suffix = &words[words.len() - 3..];
                if suffix == ["cant", "be", "blocked"] || suffix == ["cannot", "be", "blocked"] {
                    return Some(KeywordAction::Unblockable);
                }
            }
            return None;
        }
    };

    Some(action)
}

fn parse_triggered_line(tokens: &[Token]) -> Result<LineAst, CardTextError> {
    let start_idx = if tokens.first().is_some_and(|token| {
        token.is_word("whenever") || token.is_word("at") || token.is_word("when")
    }) {
        1
    } else {
        0
    };

    if let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .or_else(|| tokens.iter().position(|token| token.is_word("then")))
    {
        let trigger_tokens = &tokens[start_idx..comma_idx];
        let trigger = parse_trigger_clause(trigger_tokens)?;
        let effects_tokens = &tokens[comma_idx + 1..];
        let effects = parse_effect_sentences(effects_tokens)?;
        return Ok(LineAst::Triggered { trigger, effects });
    }

    // Some oracle lines omit the comma after the trigger clause.
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = parse_trigger_clause(trigger_tokens)
            && let Ok(effects) = parse_effect_sentences(effects_tokens)
        {
            return Ok(LineAst::Triggered { trigger, effects });
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing comma in triggered line (clause: '{}')",
        words(tokens).join(" ")
    )))
}

fn parse_trigger_clause(tokens: &[Token]) -> Result<TriggerSpec, CardTextError> {
    let words = words(tokens);

    if let Some(or_idx) = tokens.iter().position(|token| token.is_word("or"))
        && words.last().copied() == Some("dies")
        && tokens.first().is_some_and(|token| token.is_word("this"))
    {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..tokens.len() - 1];
        if left_tokens.len() == 1
            && left_tokens[0].is_word("this")
            && let Ok(filter) = parse_object_filter(right_tokens, false)
        {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisDies),
                Box::new(TriggerSpec::Dies(filter)),
            ));
        }
    }

    if let Some(or_idx) = tokens.iter().position(|token| token.is_word("or")) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause(left_tokens),
                parse_trigger_clause(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if words.contains(&"cast")
        && words.contains(&"this")
        && words.contains(&"spell")
        && words.contains(&"you")
    {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if (words.contains(&"cast") || words.contains(&"casts")) && words.contains(&"spell") {
        let caster = if words.contains(&"you") {
            PlayerFilter::You
        } else if words.contains(&"opponent") || words.contains(&"opponents") {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let cast_idx = tokens
            .iter()
            .position(|token| token.is_word("cast") || token.is_word("casts"))
            .unwrap_or(0);
        let filter_tokens = if cast_idx + 1 < tokens.len() {
            &tokens[cast_idx + 1..]
        } else {
            &[]
        };
        let filter = if filter_tokens.is_empty() {
            None
        } else {
            Some(parse_object_filter(filter_tokens, false)?)
        };

        return Ok(TriggerSpec::SpellCast { filter, caster });
    }

    if let Some(enters_idx) = tokens
        .iter()
        .position(|token| token.is_word("enters") || token.is_word("enter"))
    {
        if enters_idx == 0 {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        let subject_tokens = &tokens[..enters_idx];
        if subject_tokens
            .first()
            .is_some_and(|token| token.is_word("this"))
        {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        if let Ok(mut filter) = parse_object_filter(subject_tokens, false) {
            if words.contains(&"under") && words.contains(&"your") && words.contains(&"control") {
                filter.controller = Some(PlayerFilter::You);
            } else if words.contains(&"under")
                && (words.contains(&"opponent") || words.contains(&"opponents"))
                && words.contains(&"control")
            {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if words.contains(&"untapped") {
                return Ok(TriggerSpec::EntersBattlefieldUntapped(filter));
            }
            if words.contains(&"tapped") {
                return Ok(TriggerSpec::EntersBattlefieldTapped(filter));
            }
            return Ok(TriggerSpec::EntersBattlefield(filter));
        }
    }

    if words.as_slice() == ["players", "finish", "voting"]
        || words.as_slice() == ["players", "finished", "voting"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
        });
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        let player = if subject == ["you"] {
            Some(PlayerFilter::You)
        } else if subject == ["a", "player"]
            || subject == ["any", "player"]
            || subject == ["player"]
        {
            Some(PlayerFilter::Any)
        } else if subject == ["an", "opponent"] || subject == ["opponent"] {
            Some(PlayerFilter::Opponent)
        } else {
            None
        };
        if let Some(player) = player {
            return Ok(TriggerSpec::KeywordAction { action, player });
        }
    }

    let has_deal = words.iter().any(|word| *word == "deal" || *word == "deals");
    if has_deal
        && words.contains(&"combat")
        && words.contains(&"damage")
        && words.contains(&"player")
    {
        return Ok(TriggerSpec::ThisDealsCombatDamageToPlayer);
    }

    if words.as_slice() == ["this", "becomes", "monstrous"] {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if words.as_slice() == ["this", "creature", "blocks"] || words.as_slice() == ["this", "blocks"]
    {
        return Ok(TriggerSpec::ThisBlocks);
    }

    if words.as_slice() == ["this", "creature", "becomes", "blocked"]
        || words.as_slice() == ["this", "becomes", "blocked"]
    {
        return Ok(TriggerSpec::ThisBecomesBlocked);
    }

    if words.as_slice() == ["this", "creature", "attacks", "or", "blocks"]
        || words.as_slice() == ["this", "attacks", "or", "blocks"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisAttacks),
            Box::new(TriggerSpec::ThisBlocks),
        ));
    }

    if words.starts_with(&["this", "creature", "blocks", "or", "becomes", "blocked"])
        || words.starts_with(&["this", "blocks", "or", "becomes", "blocked"])
    {
        return Ok(TriggerSpec::ThisBlocksOrBecomesBlocked);
    }

    if words.as_slice() == ["this", "creature", "leaves", "the", "battlefield"]
        || words.as_slice() == ["this", "leaves", "the", "battlefield"]
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if words.as_slice() == ["this", "creature", "becomes", "tapped"]
        || words.as_slice() == ["this", "becomes", "tapped"]
    {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "untapped"]
        || words.as_slice() == ["this", "becomes", "untapped"]
    {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if words.starts_with(&["this", "creature", "is", "dealt", "damage"])
        || words.starts_with(&["this", "is", "dealt", "damage"])
    {
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if words.starts_with(&["this", "creature", "deals", "damage"])
        || words.starts_with(&["this", "deals", "damage"])
    {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if words.as_slice() == ["you", "gain", "life"] {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.as_slice() == ["you", "draw", "a", "card"] {
        return Ok(TriggerSpec::YouDrawCard);
    }

    let last = words
        .last()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    match *last {
        "attacks" => Ok(TriggerSpec::ThisAttacks),
        "dies" => {
            let mut subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };

            if subject_tokens.is_empty()
                || subject_tokens
                    .first()
                    .is_some_and(|token| token.is_word("this"))
            {
                return Ok(TriggerSpec::ThisDies);
            }

            let mut other = false;
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }

            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if let Ok(filter) = parse_object_filter(subject_tokens, other) {
                return Ok(TriggerSpec::Dies(filter));
            }

            Ok(TriggerSpec::ThisDies)
        }
        _ if words.contains(&"beginning") && words.contains(&"end") && words.contains(&"step") => {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfEndStep(player))
        }
        _ if words.contains(&"beginning") && words.contains(&"upkeep") => {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfUpkeep(player))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"combat")
            && words.contains(&"turn") =>
        {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfCombat(player))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"precombat")
            && words.contains(&"main") =>
        {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfPrecombatMain(player))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

fn parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();

    for sentence in split_on_period(tokens) {
        if sentence.is_empty() {
            continue;
        }

        let mut sentence_effects = parse_effect_sentence(&sentence)?;
        let has_instead = sentence.iter().any(|token| token.is_word("instead"));
        if has_instead && sentence_effects.len() == 1 && effects.len() >= 1 {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let previous = effects.pop().expect("effects length checked above");
                if let Some(EffectAst::Conditional {
                    predicate,
                    if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if_false.insert(0, previous);
                    effects.push(EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    });
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
    }

    Ok(effects)
}

fn parse_effect_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let sentence_words = words(tokens);
    if sentence_words.starts_with(&["activate", "only"]) {
        return Ok(Vec::new());
    }
    if sentence_words.starts_with(&["this", "ability", "triggers", "only"]) {
        return Ok(Vec::new());
    }
    if sentence_words.starts_with(&["round", "up", "each", "time"]) {
        return Ok(Vec::new());
    }
    if let Some(effect) = parse_each_player_choose_and_sacrifice_rest(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_exile_instead_of_graveyard_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if tokens.first().is_some_and(|token| token.is_word("if")) {
        return parse_conditional_sentence(tokens);
    }
    if let Some(effect) = parse_monstrosity_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_for_each_counter_removed_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if is_exile_that_token_at_end_of_combat(tokens) {
        return Ok(vec![EffectAst::ExileThatTokenAtEndOfCombat]);
    }
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_earthbend_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_enchant_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_cant_effect_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effect) = parse_prevent_damage_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_gain_ability_to_source_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_gain_ability_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_you_and_each_opponent_voted_with_you_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_gain_life_equal_to_power_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_gain_x_plus_life_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_search_library_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effect) = parse_play_from_graveyard_sentence(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_exile_instead_of_graveyard_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_look_at_hand_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_gain_life_equal_to_age_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effect) = parse_for_each_opponent_doesnt(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_vote_start_sentence(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_for_each_vote_clause(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_vote_extra_sentence(tokens) {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_after_turn_sentence(tokens)? {
        return Ok(vec![effect]);
    }

    if is_ring_tempts_sentence(tokens) {
        return Ok(Vec::new());
    }

    if let Some(effects) = parse_destroy_or_exile_all_split_sentence(tokens)? {
        return Ok(effects);
    }

    if tokens.first().is_some_and(|token| token.is_word("if")) {
        return parse_conditional_sentence(tokens);
    }

    parse_effect_chain(tokens)
}

fn parse_each_player_choose_and_sacrifice_rest(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let all_words = words(tokens);
    if all_words.len() < 6 {
        return Ok(None);
    }

    if !all_words.starts_with(&["each", "player", "chooses"])
        && !all_words.starts_with(&["each", "player", "choose"])
    {
        return Ok(None);
    }

    let then_idx = tokens.iter().position(|token| token.is_word("then"));
    let Some(then_idx) = then_idx else {
        return Ok(None);
    };

    let after_then = &tokens[then_idx + 1..];
    let after_words = words(after_then);
    if !(after_words.starts_with(&["sacrifice", "the", "rest"])
        || after_words.starts_with(&["sacrifices", "the", "rest"]))
    {
        return Ok(None);
    }

    let choose_tokens = &tokens[3..then_idx];
    if choose_tokens.is_empty() {
        return Ok(None);
    }

    let from_idx = find_from_among(choose_tokens);
    let Some(from_idx) = from_idx else {
        return Ok(None);
    };

    let (list_tokens, base_tokens) = if from_idx == 0 {
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    } else {
        (
            choose_tokens.get(..from_idx).unwrap_or_default(),
            choose_tokens.get(from_idx + 2..).unwrap_or_default(),
        )
    };

    let list_tokens = trim_commas(list_tokens);
    let base_tokens = trim_commas(base_tokens);
    if list_tokens.is_empty() || base_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = match parse_object_filter(&base_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }

    let mut effects = Vec::new();
    let keep_tag: TagKey = "keep".into();

    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        let segment_filter = match parse_object_filter(&segment, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        };
        let mut combined = merge_filters(&base_filter, &segment_filter);
        combined = combined.not_tagged(keep_tag.clone());
        effects.push(EffectAst::ChooseObjects {
            filter: combined,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Implicit,
            tag: keep_tag.clone(),
        });
    }

    if effects.is_empty() {
        return Ok(None);
    }

    let sacrifice_filter = base_filter.clone().not_tagged(keep_tag.clone());
    effects.push(EffectAst::SacrificeAll {
        filter: sacrifice_filter,
        player: PlayerAst::Implicit,
    });

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

fn find_from_among(tokens: &[Token]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.is_word("from") && tokens.get(idx + 1).is_some_and(|t| t.is_word("among")) {
            Some(idx)
        } else {
            None
        }
    })
}

fn find_list_start(tokens: &[Token]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

fn trim_commas(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && matches!(tokens[start], Token::Comma(_)) {
        start += 1;
    }
    while end > start && matches!(tokens[end - 1], Token::Comma(_)) {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

fn strip_leading_articles(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    while start < tokens.len() {
        if let Some(word) = tokens[start].as_word()
            && is_article(word)
        {
            start += 1;
            continue;
        }
        break;
    }
    tokens[start..].to_vec()
}

fn split_choose_list(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    for segment in split_on_and(tokens) {
        for sub in split_on_comma(&segment) {
            let trimmed = trim_commas(&sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.is_commander |= specific.is_commander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }

    merged
}

fn parse_monstrosity_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("monstrosity") {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let (amount, _) = parse_value(amount_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Monstrosity { amount }))
}

fn parse_for_each_counter_removed_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    if words_all.len() < 6 {
        return Ok(None);
    }
    if !words_all.starts_with(&["for", "each", "counter", "removed", "this", "way"]) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[6..]
    };

    let remainder_words = words(remainder);
    if remainder_words.is_empty() {
        return Ok(None);
    }

    let gets_idx = remainder_words
        .iter()
        .position(|word| *word == "gets" || *word == "get");
    let Some(gets_idx) = gets_idx else {
        return Ok(None);
    };

    let subject_tokens = &remainder[..gets_idx];
    let subject = parse_subject(subject_tokens);
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let after_gets = &remainder[gets_idx + 1..];
    let modifier_token = after_gets.first().and_then(Token::as_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing power/toughness modifier (clause: '{}')",
            remainder_words.join(" ")
        ))
    })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    let duration = if remainder_words.contains(&"until")
        && remainder_words.contains(&"end")
        && remainder_words.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    Ok(Some(EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    }))
}

fn is_exile_that_token_at_end_of_combat(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["exile", "that", "token", "at", "end", "of", "combat"]
        || words.as_slice() == ["exile", "that", "token", "at", "the", "end", "of", "combat"]
}

fn parse_take_extra_turn_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["take", "an", "extra", "turn", "after", "this", "one"] {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::You,
        }));
    }
    Ok(None)
}

fn is_ring_tempts_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["the", "ring", "tempts", "you"]
}

fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let verb = if words[0] == "destroy" {
        Some(Verb::Destroy)
    } else if words[0] == "exile" {
        Some(Verb::Exile)
    } else {
        None
    };
    let Some(verb) = verb else {
        return Ok(None);
    };
    if words[1] != "all" || !words.contains(&"and") {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment in split_on_and(&tokens[2..]) {
        if segment.is_empty() {
            continue;
        }
        let filter = match parse_object_filter(&segment, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        };
        let effect = match verb {
            Verb::Destroy => EffectAst::DestroyAll { filter },
            Verb::Exile => EffectAst::ExileAll { filter },
            _ => return Ok(None),
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

fn parse_look_at_hand_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["look", "at", "target", "players", "hand"]
        || words.as_slice() == ["look", "at", "target", "player", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::Any, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
}

fn parse_gain_life_equal_to_age_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"age") && words.contains(&"life") && words.contains(&"gain") {
        return Ok(Some(vec![EffectAst::GainLife {
            amount: Value::Fixed(0),
            player: PlayerAst::You,
        }]));
    }
    Ok(None)
}

fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let pattern = [
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for", "may", "scry",
    ];

    if words.len() < pattern.len() {
        return Ok(None);
    }

    if !words.starts_with(&pattern) {
        return Ok(None);
    }

    let scry_index = pattern.len() - 1;
    let value_tokens = &tokens[(scry_index + 1)..];
    let Some((count, _)) = parse_value(value_tokens) else {
        return Ok(None);
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::Scry {
            count: count.clone(),
            player: PlayerAst::You,
        }],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::Scry {
                count,
                player: PlayerAst::Implicit,
            }],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

fn parse_gain_life_equal_to_power_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.get(gain_idx + 1) != Some(&"life")
        || words.get(gain_idx + 2) != Some(&"equal")
        || words.get(gain_idx + 3) != Some(&"to")
    {
        return Ok(None);
    }

    let tail = &words[gain_idx + 4..];
    let has_its_power = tail.windows(2).any(|pair| pair == ["its", "power"]);
    if !has_its_power {
        return Ok(None);
    }

    let subject = if gain_idx > 0 {
        Some(parse_subject(&tokens[..gain_idx]))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
    Ok(Some(vec![EffectAst::GainLife { amount, player }]))
}

fn parse_prevent_damage_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&["prevent", "all", "combat", "damage"])
        && words.contains(&"this")
        && words.contains(&"turn")
    {
        return Ok(Some(EffectAst::PreventAllCombatDamage {
            duration: Until::EndOfTurn,
        }));
    }

    Ok(None)
}

fn parse_gain_x_plus_life_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.len() <= gain_idx + 4 {
        return Ok(None);
    }

    if words[gain_idx + 1] != "x" || words[gain_idx + 2] != "plus" {
        return Ok(None);
    }

    let number_token = tokens.get(gain_idx + 3).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let number_word = number_token
        .as_word()
        .ok_or_else(|| CardTextError::ParseError("missing life gain amount".to_string()))?;
    let (bonus, _) = parse_number(&[Token::Word(number_word.to_string(), TextSpan::synthetic())])
        .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let subject_tokens = &tokens[..gain_idx];
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let effects = vec![
        EffectAst::GainLife {
            amount: Value::X,
            player,
        },
        EffectAst::GainLife {
            amount: Value::Fixed(bonus as i32),
            player,
        },
    ];

    Ok(Some(effects))
}

fn parse_gain_ability_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let gain_idx = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let after_gain = &words[gain_idx + 1..];
    if after_gain.contains(&"life") || after_gain.contains(&"control") {
        return Ok(None);
    }

    let duration = if after_gain.contains(&"until")
        && after_gain.contains(&"end")
        && after_gain.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    let subject_tokens = &tokens[..gain_idx];

    let ability_tokens = if let Some(until_idx) = tokens.iter().position(|t| t.is_word("until")) {
        if until_idx > gain_idx + 1 {
            &tokens[gain_idx + 1..until_idx]
        } else {
            &tokens[gain_idx + 1..]
        }
    } else {
        &tokens[gain_idx + 1..]
    };

    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };

    let abilities: Vec<StaticAbility> = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect();
    if abilities.is_empty() {
        return Ok(None);
    }

    if words[..gain_idx].contains(&"target") {
        let target = parse_target_phrase(subject_tokens)?;
        return Ok(Some(vec![EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }]));
    }

    let filter = match parse_object_filter(subject_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    Ok(Some(vec![EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    }]))
}

fn parse_gain_ability_to_source_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let gain_idx = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..gain_idx];
    if !matches!(parse_subject(subject_tokens), SubjectAst::This) {
        return Ok(None);
    }

    let ability_tokens = &tokens[gain_idx + 1..];
    if let Some(ability) = parse_activated_line(ability_tokens)? {
        return Ok(Some(EffectAst::GrantAbilityToSource {
            ability: ability.ability,
        }));
    }

    Ok(None)
}

fn parse_search_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["search", "your", "library", "for"]) {
        return Ok(None);
    }

    let for_idx = tokens
        .iter()
        .position(|token| token.is_word("for"))
        .unwrap_or(3);
    let put_idx = tokens.iter().position(|token| token.is_word("put"));
    let Some(put_idx) = put_idx else {
        return Ok(None);
    };

    let filter_end = tokens[for_idx + 1..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| for_idx + 1 + idx)
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("reveal") || token.is_word("then"))
        })
        .unwrap_or(put_idx);

    if filter_end <= for_idx + 1 {
        return Ok(None);
    }

    let count_tokens = &tokens[for_idx + 1..filter_end];
    let mut count = ChoiceCount::up_to(1);
    let mut count_used = 0usize;

    if count_tokens.len() >= 2
        && count_tokens[0].is_word("any")
        && count_tokens[1].is_word("number")
    {
        count = ChoiceCount::any_number();
        count_used = 2;
    } else if count_tokens.len() >= 2
        && count_tokens[0].is_word("up")
        && count_tokens[1].is_word("to")
    {
        if let Some((value, used)) = parse_number(&count_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            count_used = 2 + used;
        }
    } else if let Some((value, used)) = parse_number(count_tokens) {
        count = ChoiceCount::up_to(value as usize);
        count_used = used;
    }

    if count_used < count_tokens.len() && count_tokens[count_used].is_word("of") {
        count_used += 1;
    }

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Ok(None);
    }

    let filter_tokens = &tokens[filter_start..filter_end];
    let filter_words: Vec<&str> = words(filter_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let mut filter =
        if filter_words.len() == 1 && (filter_words[0] == "card" || filter_words[0] == "cards") {
            ObjectFilter::default()
        } else {
            match parse_object_filter(filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) => return Ok(None),
            }
        };
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    }) && !filter.card_types.contains(&CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    if words_all.contains(&"mana") && words_all.contains(&"cost") {
        filter.has_mana_cost = true;
        filter.no_x_in_cost = true;
        let mut max_value: Option<u32> = None;
        for word in words_all.iter() {
            if let Ok(value) = word.parse::<u32>() {
                max_value = Some(max_value.map_or(value, |max| max.max(value)));
            }
        }
        if let Some(max_value) = max_value {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(max_value as i32));
        }
    }

    let destination = if words_all.contains(&"graveyard") {
        Zone::Graveyard
    } else if words_all.contains(&"hand") {
        Zone::Hand
    } else if words_all.contains(&"top") && words_all.contains(&"library") {
        Zone::Library
    } else {
        Zone::Battlefield
    };

    let reveal = words_all.contains(&"reveal");
    let shuffle = words_all.contains(&"shuffle");
    let effects = vec![EffectAst::SearchLibrary {
        filter,
        destination,
        player: PlayerAst::You,
        reveal,
        shuffle,
        count,
    }];

    Ok(Some(effects))
}

fn parse_earthbend_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("earthbend") {
        return Ok(None);
    }

    let count_tokens = &tokens[1..];
    let (count, _) = parse_number(count_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing earthbend count (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Earthbend { counters: count }))
}

fn parse_enchant_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() || words[0] != "enchant" {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = parse_object_filter(remaining, false)?;
    Ok(Some(EffectAst::Enchant { filter }))
}

fn parse_cant_effect_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let (duration, clause_tokens) = if words.starts_with(&["until", "end", "of", "turn"]) {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let remainder = if let Some(idx) = comma_idx {
            &tokens[idx + 1..]
        } else {
            &tokens[4..]
        };
        (crate::effect::Until::EndOfTurn, remainder.to_vec())
    } else if words.ends_with(&["until", "end", "of", "turn"]) {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = tokens[..end_idx]
            .iter()
            .filter(|&token| !matches!(token, Token::Comma(_)))
            .cloned()
            .collect::<Vec<_>>();
        (crate::effect::Until::EndOfTurn, remainder)
    } else if words.ends_with(&["until", "your", "next", "turn"])
        || (words.ends_with(&["next", "turn"]) && words.contains(&"until"))
    {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = tokens[..end_idx]
            .iter()
            .filter(|&token| !matches!(token, Token::Comma(_)))
            .cloned()
            .collect::<Vec<_>>();
        (crate::effect::Until::YourNextTurn, remainder)
    } else {
        return Ok(None);
    };

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Ok(None);
    };

    let effects = restrictions
        .into_iter()
        .map(|restriction| EffectAst::Cant {
            restriction,
            duration: duration.clone(),
        })
        .collect();

    Ok(Some(effects))
}

fn parse_play_from_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 8 || !line_words.starts_with(&["until", "end", "of", "turn"]) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[4..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = [
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "spells",
        "from",
        "your",
        "graveyard",
    ];

    if remaining_words == expected {
        return Ok(Some(EffectAst::PlayFromGraveyardUntilEot {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

fn parse_exile_instead_of_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.first().copied() != Some("if") {
        return Ok(None);
    }

    let has_graveyard_clause = line_words
        .windows(4)
        .any(|w| w == ["into", "your", "graveyard", "from"])
        || line_words
            .windows(3)
            .any(|w| w == ["your", "graveyard", "from"])
        || (line_words.contains(&"your") && line_words.contains(&"graveyard"));
    let has_would_put = line_words
        .windows(4)
        .any(|w| w == ["card", "would", "be", "put"]);
    let has_this_turn = line_words.contains(&"this") && line_words.contains(&"turn");
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        return Ok(None);
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = ["exile", "that", "card", "instead"];
    if remaining_words == expected {
        return Ok(Some(EffectAst::ExileInsteadOfGraveyardThisTurn {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Ok(ManaCost::new());
    }

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut current = String::new();
    let mut in_brace = false;

    for ch in trimmed.chars() {
        if ch == '{' {
            in_brace = true;
            current.clear();
            continue;
        }
        if ch == '}' {
            if !in_brace {
                continue;
            }
            in_brace = false;
            if current.is_empty() {
                continue;
            }
            let alternatives = parse_mana_symbol_group(&current)?;
            if !alternatives.is_empty() {
                pips.push(alternatives);
            }
            continue;
        }
        if in_brace {
            current.push(ch);
        }
    }

    Ok(ManaCost::from_pips(pips))
}

fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let mut alternatives = Vec::new();
    for part in raw.split('/') {
        let symbol = parse_mana_symbol(part)?;
        alternatives.push(symbol);
    }
    Ok(alternatives)
}

fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    let upper = part.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return Err(CardTextError::ParseError("empty mana symbol".to_string()));
    }

    if upper.chars().all(|c| c.is_ascii_digit()) {
        let value = upper.parse::<u8>().map_err(|_| {
            CardTextError::ParseError(format!("invalid generic mana symbol '{part}'"))
        })?;
        return Ok(ManaSymbol::Generic(value));
    }

    match upper.as_str() {
        "W" => Ok(ManaSymbol::White),
        "U" => Ok(ManaSymbol::Blue),
        "B" => Ok(ManaSymbol::Black),
        "R" => Ok(ManaSymbol::Red),
        "G" => Ok(ManaSymbol::Green),
        "C" => Ok(ManaSymbol::Colorless),
        "S" => Ok(ManaSymbol::Snow),
        "X" => Ok(ManaSymbol::X),
        "P" => Ok(ManaSymbol::Life(2)),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported mana symbol '{part}'"
        ))),
    }
}

fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();

    let parts: Vec<&str> = raw.split('—').collect();
    let left = parts[0].trim();
    let right = parts.get(1).map(|s| s.trim());

    for word in left.split_whitespace() {
        if let Some(supertype) = parse_supertype_word(word) {
            supertypes.push(supertype);
            continue;
        }
        if let Some(card_type) = parse_card_type(&word.to_ascii_lowercase()) {
            card_types.push(card_type);
        }
    }

    if let Some(right) = right {
        for word in right.split_whitespace() {
            if let Some(subtype) = parse_subtype_word(word) {
                subtypes.push(subtype);
            }
        }
    }

    Ok((supertypes, card_types, subtypes))
}

fn parse_supertype_word(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

fn parse_subtype_word(word: &str) -> Option<Subtype> {
    match word.to_ascii_lowercase().as_str() {
        "plains" => Some(Subtype::Plains),
        "island" => Some(Subtype::Island),
        "swamp" => Some(Subtype::Swamp),
        "mountain" => Some(Subtype::Mountain),
        "forest" => Some(Subtype::Forest),
        "urzas" => Some(Subtype::Urzas),
        "advisor" => Some(Subtype::Advisor),
        "ally" => Some(Subtype::Ally),
        "angel" => Some(Subtype::Angel),
        "ape" => Some(Subtype::Ape),
        "archer" => Some(Subtype::Archer),
        "artificer" => Some(Subtype::Artificer),
        "assassin" => Some(Subtype::Assassin),
        "astartes" => Some(Subtype::Astartes),
        "avatar" => Some(Subtype::Avatar),
        "barbarian" => Some(Subtype::Barbarian),
        "bard" => Some(Subtype::Bard),
        "bear" => Some(Subtype::Bear),
        "beast" => Some(Subtype::Beast),
        "berserker" => Some(Subtype::Berserker),
        "bird" => Some(Subtype::Bird),
        "boar" => Some(Subtype::Boar),
        "cat" => Some(Subtype::Cat),
        "centaur" => Some(Subtype::Centaur),
        "changeling" => Some(Subtype::Changeling),
        "cleric" => Some(Subtype::Cleric),
        "construct" => Some(Subtype::Construct),
        "crab" => Some(Subtype::Crab),
        "crocodile" => Some(Subtype::Crocodile),
        "dauthi" => Some(Subtype::Dauthi),
        "demon" => Some(Subtype::Demon),
        "dinosaur" => Some(Subtype::Dinosaur),
        "djinn" => Some(Subtype::Djinn),
        "dog" => Some(Subtype::Dog),
        "dragon" => Some(Subtype::Dragon),
        "drake" => Some(Subtype::Drake),
        "druid" => Some(Subtype::Druid),
        "dwarf" => Some(Subtype::Dwarf),
        "eldrazi" => Some(Subtype::Eldrazi),
        "elemental" => Some(Subtype::Elemental),
        "elephant" => Some(Subtype::Elephant),
        "elf" | "elves" => Some(Subtype::Elf),
        "faerie" => Some(Subtype::Faerie),
        "fish" => Some(Subtype::Fish),
        "fox" => Some(Subtype::Fox),
        "frog" => Some(Subtype::Frog),
        "fungus" => Some(Subtype::Fungus),
        "gargoyle" => Some(Subtype::Gargoyle),
        "giant" => Some(Subtype::Giant),
        "gnome" => Some(Subtype::Gnome),
        "goat" => Some(Subtype::Goat),
        "goblin" => Some(Subtype::Goblin),
        "god" => Some(Subtype::God),
        "golem" => Some(Subtype::Golem),
        "gorgon" => Some(Subtype::Gorgon),
        "griffin" => Some(Subtype::Griffin),
        "hag" => Some(Subtype::Hag),
        "halfling" => Some(Subtype::Halfling),
        "harpy" => Some(Subtype::Harpy),
        "hippo" => Some(Subtype::Hippo),
        "horror" => Some(Subtype::Horror),
        "horse" => Some(Subtype::Horse),
        "hound" => Some(Subtype::Hound),
        "human" => Some(Subtype::Human),
        "hydra" => Some(Subtype::Hydra),
        "illusion" => Some(Subtype::Illusion),
        "imp" => Some(Subtype::Imp),
        "insect" => Some(Subtype::Insect),
        "jellyfish" => Some(Subtype::Jellyfish),
        "kavu" => Some(Subtype::Kavu),
        "kirin" => Some(Subtype::Kirin),
        "kithkin" => Some(Subtype::Kithkin),
        "knight" => Some(Subtype::Knight),
        "kobold" => Some(Subtype::Kobold),
        "kor" => Some(Subtype::Kor),
        "kraken" => Some(Subtype::Kraken),
        "leviathan" => Some(Subtype::Leviathan),
        "lizard" => Some(Subtype::Lizard),
        "manticore" => Some(Subtype::Manticore),
        "mercenary" => Some(Subtype::Mercenary),
        "merfolk" => Some(Subtype::Merfolk),
        "minion" => Some(Subtype::Minion),
        "minotaur" => Some(Subtype::Minotaur),
        "mole" => Some(Subtype::Mole),
        "monk" => Some(Subtype::Monk),
        "moonfolk" => Some(Subtype::Moonfolk),
        "mouse" => Some(Subtype::Mouse),
        "mutant" => Some(Subtype::Mutant),
        "myr" => Some(Subtype::Myr),
        "naga" => Some(Subtype::Naga),
        "nightmare" => Some(Subtype::Nightmare),
        "ninja" => Some(Subtype::Ninja),
        "noble" => Some(Subtype::Noble),
        "octopus" => Some(Subtype::Octopus),
        "ogre" => Some(Subtype::Ogre),
        "ooze" => Some(Subtype::Ooze),
        "orc" => Some(Subtype::Orc),
        "otter" => Some(Subtype::Otter),
        "ox" => Some(Subtype::Ox),
        "oyster" => Some(Subtype::Oyster),
        "peasant" => Some(Subtype::Peasant),
        "pegasus" => Some(Subtype::Pegasus),
        "phyrexian" => Some(Subtype::Phyrexian),
        "phoenix" => Some(Subtype::Phoenix),
        "pilot" => Some(Subtype::Pilot),
        "pirate" => Some(Subtype::Pirate),
        "plant" => Some(Subtype::Plant),
        "praetor" => Some(Subtype::Praetor),
        "raccoon" => Some(Subtype::Raccoon),
        "rabbit" => Some(Subtype::Rabbit),
        "rat" => Some(Subtype::Rat),
        "rebel" => Some(Subtype::Rebel),
        "rhino" => Some(Subtype::Rhino),
        "rogue" => Some(Subtype::Rogue),
        "robot" => Some(Subtype::Robot),
        "salamander" => Some(Subtype::Salamander),
        "samurai" => Some(Subtype::Samurai),
        "satyr" => Some(Subtype::Satyr),
        "scarecrow" => Some(Subtype::Scarecrow),
        "scout" => Some(Subtype::Scout),
        "serpent" => Some(Subtype::Serpent),
        "shade" => Some(Subtype::Shade),
        "shaman" => Some(Subtype::Shaman),
        "shapeshifter" => Some(Subtype::Shapeshifter),
        "shark" => Some(Subtype::Shark),
        "sheep" => Some(Subtype::Sheep),
        "skeleton" => Some(Subtype::Skeleton),
        "slith" => Some(Subtype::Slith),
        "sliver" => Some(Subtype::Sliver),
        "slug" => Some(Subtype::Slug),
        "snake" => Some(Subtype::Snake),
        "soldier" => Some(Subtype::Soldier),
        "sorcerer" => Some(Subtype::Sorcerer),
        "spacecraft" => Some(Subtype::Spacecraft),
        "sphinx" => Some(Subtype::Sphinx),
        "specter" => Some(Subtype::Specter),
        "spider" => Some(Subtype::Spider),
        "spike" => Some(Subtype::Spike),
        "spirit" => Some(Subtype::Spirit),
        "sponge" => Some(Subtype::Sponge),
        "squid" => Some(Subtype::Squid),
        "squirrel" => Some(Subtype::Squirrel),
        "starfish" => Some(Subtype::Starfish),
        "surrakar" => Some(Subtype::Surrakar),
        "thopter" => Some(Subtype::Thopter),
        "thrull" => Some(Subtype::Thrull),
        "tiefling" => Some(Subtype::Tiefling),
        "toy" => Some(Subtype::Toy),
        "treefolk" => Some(Subtype::Treefolk),
        "trilobite" => Some(Subtype::Trilobite),
        "troll" => Some(Subtype::Troll),
        "turtle" => Some(Subtype::Turtle),
        "unicorn" => Some(Subtype::Unicorn),
        "vampire" => Some(Subtype::Vampire),
        "vedalken" => Some(Subtype::Vedalken),
        "viashino" => Some(Subtype::Viashino),
        "wall" => Some(Subtype::Wall),
        "warlock" => Some(Subtype::Warlock),
        "warrior" => Some(Subtype::Warrior),
        "weird" => Some(Subtype::Weird),
        "werewolf" => Some(Subtype::Werewolf),
        "whale" => Some(Subtype::Whale),
        "wizard" => Some(Subtype::Wizard),
        "wolf" => Some(Subtype::Wolf),
        "wolverine" => Some(Subtype::Wolverine),
        "wombat" => Some(Subtype::Wombat),
        "worm" => Some(Subtype::Worm),
        "wraith" => Some(Subtype::Wraith),
        "wurm" => Some(Subtype::Wurm),
        "yeti" => Some(Subtype::Yeti),
        "zombie" => Some(Subtype::Zombie),
        "zubera" => Some(Subtype::Zubera),
        "clue" => Some(Subtype::Clue),
        "contraption" => Some(Subtype::Contraption),
        "equipment" => Some(Subtype::Equipment),
        "food" => Some(Subtype::Food),
        "fortification" => Some(Subtype::Fortification),
        "gold" => Some(Subtype::Gold),
        "treasure" => Some(Subtype::Treasure),
        "vehicle" => Some(Subtype::Vehicle),
        "aura" => Some(Subtype::Aura),
        "background" => Some(Subtype::Background),
        "cartouche" => Some(Subtype::Cartouche),
        "class" => Some(Subtype::Class),
        "curse" => Some(Subtype::Curse),
        "role" => Some(Subtype::Role),
        "rune" => Some(Subtype::Rune),
        "saga" => Some(Subtype::Saga),
        "shard" => Some(Subtype::Shard),
        "shrine" => Some(Subtype::Shrine),
        "adventure" => Some(Subtype::Adventure),
        "arcane" => Some(Subtype::Arcane),
        "lesson" => Some(Subtype::Lesson),
        "trap" => Some(Subtype::Trap),
        "ajani" => Some(Subtype::Ajani),
        "ashiok" => Some(Subtype::Ashiok),
        "chandra" => Some(Subtype::Chandra),
        "elspeth" => Some(Subtype::Elspeth),
        "garruk" => Some(Subtype::Garruk),
        "gideon" => Some(Subtype::Gideon),
        "jace" => Some(Subtype::Jace),
        "karn" => Some(Subtype::Karn),
        "liliana" => Some(Subtype::Liliana),
        "nissa" => Some(Subtype::Nissa),
        "sorin" => Some(Subtype::Sorin),
        "teferi" => Some(Subtype::Teferi),
        "ugin" => Some(Subtype::Ugin),
        "vraska" => Some(Subtype::Vraska),
        _ => None,
    }
}

fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let power = parse_pt_value(parts[0].trim())?;
    let toughness = parse_pt_value(parts[1].trim())?;
    Some(PowerToughness::new(power, toughness))
}

fn parse_pt_value(raw: &str) -> Option<PtValue> {
    if raw == ".5" || raw == "0.5" {
        return Some(PtValue::Fixed(0));
    }
    if raw == "*" {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = raw.strip_prefix("*+") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = raw.strip_suffix("+*") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Ok(value) = raw.parse::<i32>() {
        return Some(PtValue::Fixed(value));
    }
    None
}

fn parse_for_each_opponent_doesnt(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if !words.starts_with(&["for", "each", "opponent"])
        && !words.starts_with(&["for", "each", "opponents"])
    {
        return Ok(None);
    }

    let has_doesnt =
        words.contains(&"doesnt") || words.windows(2).any(|pair| pair == ["do", "not"]);
    if !has_doesnt {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each opponent clause".to_string())
        })?;

    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot { effects }))
}

fn parse_vote_start_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };

    let has_each = words[..vote_idx].contains(&"each");
    let has_player = words[..vote_idx]
        .iter()
        .any(|word| *word == "player" || *word == "players");
    if !has_each || !has_player {
        return Ok(None);
    }

    let for_idx = words
        .iter()
        .position(|word| *word == "for")
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let option_words = &words[for_idx + 1..];
    let mut options = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in option_words {
        if *word == "or" {
            if !current.is_empty() {
                options.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        if is_article(word) {
            continue;
        }
        current.push(word);
    }
    if !current.is_empty() {
        options.push(current.join(" "));
    }

    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    }

    Ok(Some(EffectAst::VoteStart { options }))
}

fn parse_for_each_vote_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if !words.starts_with(&["for", "each"]) {
        return Ok(None);
    }

    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };
    if vote_idx <= 2 {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }

    let option_words: Vec<&str> = words[2..vote_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if option_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }
    let option = option_words.join(" ");

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each vote clause".to_string())
        })?;

    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

fn parse_vote_extra_sentence(tokens: &[Token]) -> Option<EffectAst> {
    let words = words(tokens);
    if words.len() < 3 || words.first().copied() != Some("you") {
        return None;
    }

    let has_vote = words.iter().any(|word| *word == "vote" || *word == "votes");
    let has_additional = words.contains(&"additional");
    let has_time = words.iter().any(|word| *word == "time" || *word == "times");
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = words.contains(&"may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}

fn parse_after_turn_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 3
        || line_words[0] != "after"
        || line_words[1] != "that"
        || line_words[2] != "turn"
    {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[3..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.len() < 4 {
        return Err(CardTextError::ParseError(
            "unsupported after turn clause".to_string(),
        ));
    }

    let player = if remaining_words.starts_with(&["that", "player"]) {
        PlayerAst::That
    } else if remaining_words.starts_with(&["target", "player"]) {
        PlayerAst::Target
    } else if remaining_words.starts_with(&["you"]) {
        PlayerAst::You
    } else {
        return Err(CardTextError::ParseError(
            "unsupported after turn player".to_string(),
        ));
    };

    if remaining_words.contains(&"extra") && remaining_words.contains(&"turn") {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn { player }));
    }

    Err(CardTextError::ParseError(
        "unsupported after turn clause".to_string(),
    ))
}

fn parse_conditional_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| CardTextError::ParseError("missing comma in if clause".to_string()))?;

    let predicate_tokens = &tokens[1..comma_idx];
    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;

    if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
        return Ok(vec![EffectAst::IfResult { predicate, effects }]);
    }

    let predicate = parse_predicate(predicate_tokens)?;
    Ok(vec![EffectAst::Conditional {
        predicate,
        if_true: effects,
        if_false: Vec::new(),
    }])
}

fn parse_if_result_predicate(tokens: &[Token]) -> Option<IfResultPredicate> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if words.len() >= 2 && words[0] == "you" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2 && words[0] == "they" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && (words[1] == "creature" || words[1] == "permanent" || words[1] == "card")
        && words[2] == "dies"
        && words[3] == "this"
        && words[4] == "way"
    {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if words.len() >= 2 && words[0] == "you" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }
    if words.len() >= 2 && words[0] == "they" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }

    None
}

fn parse_predicate(tokens: &[Token]) -> Result<PredicateAst, CardTextError> {
    let mut filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "is")
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }

    if filtered.len() >= 2 {
        let tag = if filtered.starts_with(&["equipped", "creature"]) {
            Some("equipped")
        } else if filtered.starts_with(&["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let is_it = filtered.first().is_some_and(|word| *word == "it");
    let has_land = filtered.contains(&"land");
    let has_card = filtered.contains(&"card");

    if is_it && has_land && has_card {
        return Ok(PredicateAst::ItIsLandCard);
    }

    if is_it {
        let mut card_types = Vec::new();
        for word in &filtered {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
        }
        if !card_types.is_empty() {
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                card_types,
                ..Default::default()
            }));
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

fn parse_effect_chain(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let words = words(tokens);
    let starts_with_each_opponent =
        words.starts_with(&["each", "opponent"]) || words.starts_with(&["each", "opponents"]);

    if tokens.first().is_some_and(|token| token.is_word("they"))
        && tokens.get(1).is_some_and(|token| token.is_word("may"))
    {
        let inner_tokens = &tokens[2..];
        let effects = parse_effect_chain_inner(inner_tokens)?;
        return Ok(vec![EffectAst::MayByTaggedController {
            tag: TagKey::from("triggering"),
            effects,
        }]);
    }

    if tokens.iter().any(|token| token.is_word("may")) && !starts_with_each_opponent {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_inner(&stripped)?;
        return Ok(vec![EffectAst::May { effects }]);
    }

    parse_effect_chain_inner(tokens)
}

fn parse_effect_chain_inner(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let raw_segments = split_on_and(tokens);
    let mut segments: Vec<Vec<Token>> = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segments.is_empty() {
            segments.push(segment);
            continue;
        }
        if find_verb(&segment).is_none() {
            let last = segments.last_mut().expect("non-empty segments");
            last.push(Token::Word("and".to_string(), TextSpan::synthetic()));
            last.extend(segment);
            continue;
        }
        segments.push(segment);
    }
    while segments.len() > 1 && find_verb(&segments[0]).is_none() {
        let mut first = segments.remove(0);
        first.push(Token::Word("and".to_string(), TextSpan::synthetic()));
        let mut next = segments.remove(0);
        first.append(&mut next);
        segments.insert(0, first);
    }
    for segment in segments {
        effects.push(parse_effect_clause(&segment)?);
    }
    Ok(effects)
}

fn remove_first_word(tokens: &[Token], word: &str) -> Vec<Token> {
    let mut removed = false;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        if !removed && token.is_word(word) {
            removed = true;
            continue;
        }
        out.push(token.clone());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Remove,
    Return,
    Exchange,
    Become,
    Skip,
    Surveil,
}

fn parse_effect_clause(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    if let Some(effect) = parse_for_each_opponent_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_for_each_player_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_verb_first_clause(tokens)? {
        return Ok(effect);
    }

    if tokens.first().is_some_and(|token| token.is_word("target")) && find_verb(tokens).is_none() {
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::TargetOnly { target });
    }

    let (verb, verb_idx) = find_verb(tokens).ok_or_else(|| {
        let clause = words(tokens).join(" ");
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "remove",
            "return",
            "exchange",
            "become",
            "skip",
            "surveil",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;

    if matches!(verb, Verb::Get) {
        let subject_tokens = &tokens[..verb_idx];
        if !subject_tokens.is_empty() {
            let subject_words = words(subject_tokens);
            if !subject_words.contains(&"this")
                && !subject_words.contains(&"that")
                && !subject_words.contains(&"it")
            {
                if let Ok(filter) = parse_object_filter(subject_tokens, false)
                    && filter != ObjectFilter::default()
                {
                    if let Some(mod_token) = tokens.get(verb_idx + 1).and_then(Token::as_word)
                        && let Ok((power, toughness)) = parse_pt_modifier(mod_token)
                    {
                        let rest_words = words(&tokens[verb_idx + 1..]);
                        let duration = if rest_words.contains(&"until")
                            && rest_words.contains(&"end")
                            && rest_words.contains(&"turn")
                        {
                            Until::EndOfTurn
                        } else {
                            Until::EndOfTurn
                        };
                        return Ok(EffectAst::PumpAll {
                            filter,
                            power: Value::Fixed(power),
                            toughness: Value::Fixed(toughness),
                            duration,
                        });
                    }
                }
            }
        }
    }

    let subject_tokens = &tokens[..verb_idx];
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        let rest_words = words(&tokens[verb_idx + 1..]);
        let has_protection = rest_words.contains(&"protection");
        let has_choice = rest_words.contains(&"choice");
        let has_color = rest_words.contains(&"color");
        let has_colorless = rest_words.contains(&"colorless");
        if has_protection && has_choice && (has_color || has_colorless) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::GrantProtectionChoice {
                target,
                allow_colorless: has_colorless,
            });
        }
    }
    let subject = parse_subject(subject_tokens);
    let rest = &tokens[verb_idx + 1..];

    parse_effect_with_verb(verb, Some(subject), rest)
}

fn parse_for_each_opponent_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 2 {
        return Ok(None);
    }

    if !words.starts_with(&["each", "opponent"]) && !words.starts_with(&["each", "opponents"]) {
        return Ok(None);
    }

    let inner_tokens = &tokens[2..];
    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachOpponent { effects }))
}

fn parse_for_each_player_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 2 {
        return Ok(None);
    }

    if !words.starts_with(&["each", "player"]) && !words.starts_with(&["each", "players"]) {
        return Ok(None);
    }

    let inner_tokens = &tokens[2..];
    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain_inner(inner_tokens)?
    };

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

fn parse_verb_first_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(Token::Word(word, _)) = tokens.first() else {
        return Ok(None);
    };

    let verb = match word.as_str() {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

fn find_verb(tokens: &[Token]) -> Option<(Verb, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        let verb = match word {
            "adds" | "add" => Verb::Add,
            "moves" | "move" => Verb::Move,
            "deals" | "deal" => Verb::Deal,
            "draws" | "draw" => Verb::Draw,
            "counters" | "counter" => Verb::Counter,
            "destroys" | "destroy" => Verb::Destroy,
            "exiles" | "exile" => Verb::Exile,
            "reveals" | "reveal" => Verb::Reveal,
            "loses" | "lose" => Verb::Lose,
            "gains" | "gain" => Verb::Gain,
            "puts" | "put" => Verb::Put,
            "sacrifices" | "sacrifice" => Verb::Sacrifice,
            "creates" | "create" => Verb::Create,
            "investigates" | "investigate" => Verb::Investigate,
            "proliferates" | "proliferate" => Verb::Proliferate,
            "taps" | "tap" => Verb::Tap,
            "untaps" | "untap" => Verb::Untap,
            "scries" | "scry" => Verb::Scry,
            "discards" | "discard" => Verb::Discard,
            "transforms" | "transform" => Verb::Transform,
            "regenerates" | "regenerate" => Verb::Regenerate,
            "mills" | "mill" => Verb::Mill,
            "gets" | "get" => Verb::Get,
            "removes" | "remove" => Verb::Remove,
            "returns" | "return" => Verb::Return,
            "exchanges" | "exchange" => Verb::Exchange,
            "becomes" | "become" => Verb::Become,
            "skips" | "skip" => Verb::Skip,
            "surveils" | "surveil" => Verb::Surveil,
            _ => continue,
        };
        return Some((verb, idx));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubjectAst {
    This,
    Player(PlayerAst),
}

fn parse_subject(tokens: &[Token]) -> SubjectAst {
    let words = words(tokens);
    if words.is_empty() {
        return SubjectAst::This;
    }

    if words.starts_with(&["you"]) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if words.starts_with(&["target", "player"]) || words.starts_with(&["target", "players"]) {
        return SubjectAst::Player(PlayerAst::Target);
    }

    if words.starts_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }

    if words.starts_with(&["that", "player"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if words.starts_with(&["its", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }

    if words.starts_with(&["this"]) || words.starts_with(&["thiss"]) {
        return SubjectAst::This;
    }

    SubjectAst::This
}

fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[Token],
) -> Result<EffectAst, CardTextError> {
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Lose => parse_lose_life(tokens, subject),
        Verb::Gain => {
            if tokens.first().is_some_and(|token| token.is_word("control")) {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            if tokens
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"))
            {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens),
        Verb::Proliferate => Ok(EffectAst::Proliferate),
        Verb::Tap => parse_tap(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens),
        Verb::Become => parse_become(tokens, subject),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
    }
}

fn parse_deal_damage(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    let clause_words = words(tokens);
    if clause_words.starts_with(&["damage", "to", "each", "opponent"])
        && clause_words.contains(&"number")
        && clause_words.contains(&"cards")
        && clause_words.contains(&"hand")
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: value,
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_deal_damage_with_amount(
    tokens: &[Token],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(Token::Word(word, _)) = rest.first() else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != "damage" {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }

    let target_words = words(target_tokens);
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all")) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DealDamageEach {
            amount: amount.clone(),
            filter,
        });
    }

    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::DealDamage { amount, target })
}

fn parse_move(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["all", "counters", "from"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let from_idx = tokens
        .iter()
        .position(|token| token.is_word("from"))
        .unwrap_or(2);
    let onto_idx = tokens
        .iter()
        .position(|token| token.is_word("onto"))
        .or_else(|| tokens.iter().position(|token| token.is_word("to")))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing move destination (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let from_tokens = &tokens[from_idx + 1..onto_idx];
    let to_tokens = &tokens[onto_idx + 1..];
    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(EffectAst::MoveAllCounters { from, to })
}

fn parse_draw(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Draw { count, player })
}

fn parse_counter(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) {
        let target_tokens = &tokens[..unless_idx];
        let target = parse_target_phrase(target_tokens)?;

        let unless_tokens = &tokens[unless_idx + 1..];
        let pays_idx = unless_tokens
            .iter()
            .position(|token| token.is_word("pays"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;

        let mut mana = Vec::new();
        for token in &unless_tokens[pays_idx + 1..] {
            if let Some(word) = token.as_word()
                && let Ok(symbol) = parse_mana_symbol(word)
            {
                mana.push(symbol);
            }
        }

        if mana.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        return Ok(EffectAst::CounterUnlessPays { target, mana });
    }

    let mut target_tokens = tokens;
    if let Some(that_idx) = tokens.iter().position(|token| token.is_word("that")) {
        target_tokens = &tokens[..that_idx];
    }
    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::Counter { target })
}

fn parse_reveal(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    if words.contains(&"hand") {
        return Ok(EffectAst::RevealHand { player });
    }

    let has_top = words.contains(&"top");
    let has_card = words.contains(&"card");

    if !has_top || !has_card {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::RevealTop { player })
}

fn parse_lose_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    if words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    let (amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life loss amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }

    Ok(EffectAst::LoseLife { amount, player })
}

fn parse_gain_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let (amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }

    Ok(EffectAst::GainLife { amount, player })
}

fn parse_gain_control(
    tokens: &[Token],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let mut idx = 0;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("control"))
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let duration_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("during") || token.is_word("until"))
        .map(|offset| idx + offset);

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };

    let target_ast = parse_target_phrase(target_tokens)?;
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    match target_ast {
        TargetAst::Player(filter, _) => Ok(EffectAst::ControlPlayer {
            player: PlayerFilter::Target(Box::new(filter)),
            duration,
        }),
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            Ok(EffectAst::GainControl {
                target: target_ast,
                duration: until,
            })
        }
    }
}

fn parse_control_duration(tokens: &[Token]) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    let words = words(tokens);
    let has_during = words.contains(&"during");
    let has_next = words.contains(&"next");
    let has_turn = words.contains(&"turn");
    if has_during && has_next && has_turn {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    let has_until = words.contains(&"until");
    let has_end = words.contains(&"end");
    if has_until && has_end && has_turn {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

fn parse_put_into_hand(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    let has_it = words.contains(&"it");
    let has_them = words.contains(&"them");
    let has_hand = words.contains(&"hand");
    let has_into = words.contains(&"into");

    if !has_hand || !has_into || (!has_it && !has_them) {
        return Err(CardTextError::ParseError(format!(
            "unsupported put clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::PutIntoHand {
        player,
        object: ObjectRefAst::It,
    })
}

fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" => Some(CounterType::MinusOneMinusOne),
        "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "brain" => Some(CounterType::Brain),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        _ => None,
    }
}

fn parse_counter_type_from_tokens(tokens: &[Token]) -> Option<CounterType> {
    for token in tokens {
        if let Some(word) = token.as_word()
            && let Some(parsed) = parse_counter_type_word(word)
        {
            return Some(parsed);
        }
    }

    let token_words = words(tokens);
    for window in token_words.windows(2) {
        match window {
            ["-1", "-1"] => return Some(CounterType::MinusOneMinusOne),
            ["-0", "-1"] => return Some(CounterType::MinusOneMinusOne),
            ["+1", "+1"] => return Some(CounterType::PlusOnePlusOne),
            _ => {}
        }
    }
    None
}

fn parse_put_counters(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let (count, used) = parse_number(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let on_idx = rest
        .iter()
        .position(|token| token.is_word("on"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter target (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let target_tokens = &rest[on_idx + 1..];
    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::PutCounters {
        counter_type,
        count,
        target,
    })
}

fn parse_tap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Tap { target })
}

fn parse_sacrifice(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let mut idx = 0;
    let mut other = false;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }

    let filter_tokens = &tokens[idx..];
    let filter = parse_object_filter(filter_tokens, other)?;
    Ok(EffectAst::Sacrifice { filter, player })
}

fn parse_discard(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);
    if clause_words.contains(&"hand") {
        return Ok(EffectAst::DiscardHand { player });
    }

    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing discard count (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }

    let rest_words = words(rest);
    let random = rest_words.ends_with(&["at", "random"]);

    Ok(EffectAst::Discard {
        count,
        player,
        random,
    })
}

fn parse_return(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let to_idx = tokens
        .iter()
        .rposition(|token| token.is_word("to"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let target_tokens = &tokens[..to_idx];
    let destination_words = words(&tokens[to_idx + 1..]);
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    if !is_hand && !is_battlefield {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let target_words = words(target_tokens);
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::ReturnAllToHand { filter });
    }

    let target = parse_target_phrase(target_tokens)?;
    if is_battlefield {
        Ok(EffectAst::ReturnToBattlefield { target, tapped })
    } else {
        Ok(EffectAst::ReturnToHand { target })
    }
}

fn parse_exchange(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["control", "of"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut idx = 2usize;
    let mut count = 2u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing exchange target filter".to_string(),
        ));
    }

    let filter = parse_object_filter(&tokens[idx..], false)?;
    Ok(EffectAst::ExchangeControl { filter, count })
}

fn parse_become(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let amount = parse_value(tokens).map(|(value, _)| value).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life total amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    Ok(EffectAst::SetLifeTotal { amount, player })
}

fn parse_skip(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported skip clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let words = words(tokens);
    if words.contains(&"draw") && words.contains(&"step") {
        return Ok(EffectAst::SkipDrawStep { player });
    }
    if words.contains(&"turn") {
        return Ok(EffectAst::SkipTurn { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_transform(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(None),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Transform { target })
}

fn parse_regenerate(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Regenerate { target })
}

fn parse_mill(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing mill count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Mill { count, player })
}

fn parse_get(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if words.contains(&"poison") && words.contains(&"counter") {
        let player = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::Implicit,
        };
        return Ok(EffectAst::PoisonCounters {
            count: Value::Fixed(1),
            player,
        });
    }

    let energy_count = tokens.iter().filter(|token| token.is_word("e")).count();
    if energy_count > 0 {
        let player = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::Implicit,
        };
        return Ok(EffectAst::EnergyCounters {
            count: Value::Fixed(energy_count as i32),
            player,
        });
    }

    if let Some(mod_token) = tokens.first().and_then(Token::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier(mod_token)
    {
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        let duration =
            if words.contains(&"until") && words.contains(&"end") && words.contains(&"turn") {
                Until::EndOfTurn
            } else {
                Until::EndOfTurn
            };
        return Ok(EffectAst::Pump {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            duration,
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_add_mana(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let mut mana = Vec::new();
    for token in tokens {
        if let Some(word) = token.as_word() {
            if word == "mana" || word == "to" || word == "your" || word == "pool" {
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana.push(symbol);
            }
        }
    }

    if mana.is_empty() {
        let clause_words = words(tokens);
        let has_card_word = clause_words
            .iter()
            .any(|word| *word == "card" || *word == "cards");
        if clause_words.contains(&"exiled") && has_card_word && clause_words.contains(&"colors") {
            return Ok(EffectAst::AddManaImprintedColors);
        }

        if clause_words.contains(&"commander")
            && clause_words.contains(&"color")
            && clause_words.contains(&"identity")
        {
            let amount = parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1));
            return Ok(EffectAst::AddManaCommanderIdentity { amount, player });
        }

        if clause_words.contains(&"any") && clause_words.contains(&"color") {
            let amount = parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1));
            let any_one = clause_words
                .windows(3)
                .any(|window| window == ["any", "one", "color"]);
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor { amount, player });
        }

        return Err(CardTextError::ParseError(format!(
            "missing mana symbols (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::AddMana { mana, player })
}

fn parse_create(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    let mut idx = 0;
    let mut count = 1;
    if let Some((value, used)) = parse_number(tokens) {
        count = value;
        idx = used;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        idx += 1;
    }

    let remaining_words = words(&tokens[idx..]);
    let token_idx = remaining_words
        .iter()
        .position(|word| *word == "token" || *word == "tokens")
        .ok_or_else(|| CardTextError::ParseError("create clause missing token".to_string()))?;

    let name_words: Vec<&str> = remaining_words[..token_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    let tail_tokens = &tokens[idx + token_idx + 1..];
    let tail_words = remaining_words[token_idx + 1..].to_vec();
    if name_words.is_empty() {
        if tail_words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
        {
            let half_pt = tail_words.contains(&"half")
                && tail_words.contains(&"power")
                && tail_words.contains(&"toughness");
            if let Some(of_idx) = tail_tokens.iter().position(|token| token.is_word("of")) {
                let source_tokens = &tail_tokens[of_idx + 1..];
                let source_end = source_tokens
                    .iter()
                    .position(|token| matches!(token, Token::Comma(_)) || token.is_word("except"))
                    .unwrap_or(source_tokens.len());
                let source_tokens = &source_tokens[..source_end];
                if !source_tokens.is_empty() {
                    let source = parse_target_phrase(source_tokens)?;
                    return Ok(EffectAst::CreateTokenCopyFromSource {
                        source,
                        count,
                        player,
                        half_power_toughness_round_up: half_pt,
                    });
                }
            }
            return Ok(EffectAst::CreateTokenCopy {
                object: ObjectRefAst::It,
                count,
                player,
                half_power_toughness_round_up: half_pt,
            });
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    let mut name = remaining_words.join(" ");
    if name.is_empty() {
        name = normalize_token_name(&name_words);
        if tail_words.contains(&"lifelink") {
            name.push_str(" lifelink");
        }
    }

    let tapped = tail_words.contains(&"tapped");
    let attacking = tail_words.contains(&"attacking");

    Ok(EffectAst::CreateTokenWithMods {
        name,
        count,
        player,
        tapped,
        attacking,
        exile_at_end_of_combat: false,
    })
}

fn normalize_token_name(words: &[&str]) -> String {
    let mut filtered = Vec::new();
    for word in words {
        if word.contains('/')
            && word
                .chars()
                .all(|c| c.is_ascii_digit() || c == '/' || c == '+' || c == '-')
        {
            continue;
        }
        if matches!(
            *word,
            "white" | "blue" | "black" | "red" | "green" | "colorless" | "creature"
        ) {
            continue;
        }
        filtered.push(*word);
    }
    if filtered.is_empty() {
        words.join(" ")
    } else {
        filtered.join(" ")
    }
}

fn parse_investigate(_tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    Ok(EffectAst::Investigate)
}

fn parse_remove(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let mut idx = 0;
    let mut up_to = false;
    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        up_to = true;
        idx += 2;
    }

    let (amount, used) = parse_value(&tokens[idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .map(|offset| idx + offset)
        .ok_or_else(|| CardTextError::ParseError("missing counter keyword".to_string()))?;
    if counter_idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing counter keyword".to_string(),
        ));
    }
    idx = counter_idx + 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("from")) {
        idx += 1;
    }

    let target_tokens = &tokens[idx..];
    let target = parse_target_phrase(target_tokens)?;

    let _ = up_to;
    Ok(EffectAst::RemoveUpToAnyCounters { amount, target })
}

fn parse_destroy(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if words.first().copied() == Some("all") {
        let filter_tokens = &tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DestroyAll { filter });
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Destroy { target })
}

fn parse_exile(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if words.first().copied() == Some("all") {
        let filter_tokens = &tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::ExileAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Exile { target })
}

fn parse_untap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    if words(tokens).as_slice() == ["them"] {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::UntapAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Untap { target })
}

fn parse_scry(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Scry { count, player })
}

fn parse_surveil(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Surveil { count, player })
}

fn parse_target_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);

    let all_words = words(tokens);
    if all_words.as_slice() == ["that", "permanent"] || all_words.as_slice() == ["that", "creature"]
    {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), span));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if remaining_words.as_slice() == ["equipped", "creature"]
        || remaining_words.as_slice() == ["equipped", "creatures"]
    {
        return Ok(TargetAst::Tagged(TagKey::from("equipped"), span));
    }
    if remaining_words.as_slice() == ["enchanted", "creature"]
        || remaining_words.as_slice() == ["enchanted", "creatures"]
    {
        return Ok(TargetAst::Tagged(TagKey::from("enchanted"), span));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((_, used)) = parse_number(&tokens[idx..]) {
            idx += used;
        }
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }

    let words_all = words(tokens);
    if words_all.as_slice() == ["any", "target"] {
        return Ok(TargetAst::AnyTarget(span));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = words(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"] {
        return Ok(TargetAst::Player(PlayerFilter::Any, span));
    }

    if remaining_words.as_slice() == ["you"] {
        return Ok(TargetAst::Player(PlayerFilter::You, span));
    }

    if remaining_words.as_slice() == ["opponent"] || remaining_words.as_slice() == ["opponents"] {
        return Ok(TargetAst::Player(PlayerFilter::Opponent, span));
    }

    if remaining_words.as_slice() == ["spell"] || remaining_words.as_slice() == ["spells"] {
        return Ok(TargetAst::Spell(span));
    }

    if remaining_words.as_slice() == ["this"]
        || remaining_words.as_slice() == ["thiss"]
        || remaining_words.as_slice() == ["this", "creature"]
        || remaining_words.as_slice() == ["thiss", "creature"]
        || remaining_words.as_slice() == ["this", "permanent"]
        || remaining_words.as_slice() == ["thiss", "permanent"]
    {
        return Ok(TargetAst::Source(span));
    }
    if remaining_words.starts_with(&["thiss", "power", "and", "toughness"]) {
        return Ok(TargetAst::Source(span));
    }

    if remaining_words.first().is_some_and(|word| *word == "it")
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| *word == "instead" || *word == "this" || *word == "way")
    {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), span));
    }

    let has_creature =
        remaining_words.contains(&"creature") || remaining_words.contains(&"creatures");
    let has_player = remaining_words.contains(&"player") || remaining_words.contains(&"players");
    if has_creature && has_player {
        return Ok(TargetAst::AnyTarget(span));
    }

    let filter = parse_object_filter(remaining, other)?;
    let it_span = if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        tokens
            .iter()
            .rev()
            .find(|token| token.is_word("it"))
            .map(Token::span)
    } else {
        None
    };
    Ok(TargetAst::Object(filter, span, it_span))
}

fn parse_object_filter(tokens: &[Token], other: bool) -> Result<ObjectFilter, CardTextError> {
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut base_tokens: Vec<Token> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("targets") || token.is_word("target") {
            if idx > 0 && tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let target_words = words(target_tokens);
        if target_words.starts_with(&["you"]) {
            target_player = Some(PlayerFilter::You);
        } else if target_words.starts_with(&["opponent"])
            || target_words.starts_with(&["opponents"])
        {
            target_player = Some(PlayerFilter::Opponent);
        } else if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
            target_player = Some(PlayerFilter::Any);
        } else {
            let mut target_filter_tokens = target_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if !target_filter_tokens.is_empty() {
                target_object = Some(parse_object_filter(target_filter_tokens, false)?);
            }
        }
    }

    let all_words: Vec<&str> = words(&base_tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "instead")
        .collect();

    if all_words.len() == 1 && (all_words[0] == "it" || all_words[0] == "them") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(filter);
    }

    let has_share_card_type = all_words.contains(&"shares")
        && all_words.contains(&"card")
        && all_words.contains(&"type")
        && all_words.contains(&"it");

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }

    if all_words.len() >= 2 {
        for window in all_words.windows(2) {
            match window {
                ["you", "control"] | ["you", "controls"] => {
                    filter.controller = Some(PlayerFilter::You);
                }
                ["opponent", "control"]
                | ["opponent", "controls"]
                | ["opponents", "control"]
                | ["opponents", "controls"] => {
                    filter.controller = Some(PlayerFilter::Opponent);
                }
                ["they", "control"] | ["they", "controls"] => {
                    filter.controller = Some(PlayerFilter::IteratedPlayer);
                }
                _ => {}
            }
        }
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if all_words.contains(&"spell") {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    _ => {}
                }
            }
        }
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    for word in &all_words {
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => saw_spell = true,
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "commander" | "commanders" => filter.is_commander = true,
            "attacking" | "blocking" => {}
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            _ => {}
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(card_type) = parse_card_type(word)
            && is_permanent_type(card_type)
        {
            saw_permanent_type = true;
        }

        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            saw_subtype = true;
        }
    }

    let segments = split_on_or(&base_tokens);
    let mut segment_types = Vec::new();

    for segment in &segments {
        let segment_words: Vec<&str> = words(segment)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        let mut types = Vec::new();
        for word in segment_words {
            if let Some(card_type) = parse_card_type(word)
                && !types.contains(&card_type)
            {
                types.push(card_type);
            }
        }
        if !types.is_empty() {
            segment_types.push(types);
        }
    }

    if segments.len() > 1 {
        let mut any_types = Vec::new();
        for types in segment_types {
            if types.len() != 1 {
                return Err(CardTextError::ParseError(
                    "unsupported target type list".to_string(),
                ));
            }
            let card_type = types[0];
            if !any_types.contains(&card_type) {
                any_types.push(card_type);
            }
        }
        if !any_types.is_empty() {
            filter.card_types = any_types;
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        if types.len() > 1 {
            filter.all_card_types = types;
        } else if types.len() == 1 {
            filter.card_types = types;
        }
    }

    if saw_spell && saw_permanent {
        return Err(CardTextError::ParseError(format!(
            "cannot mix spell and permanent targets (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if let Some(zone) = filter.zone {
        if saw_spell && zone != Zone::Stack {
            return Err(CardTextError::ParseError(
                "spell targets must be on the stack".to_string(),
            ));
        }
    } else if saw_spell {
        filter.zone = Some(Zone::Stack);
    } else if saw_permanent || saw_permanent_type || saw_subtype {
        filter.zone = Some(Zone::Battlefield);
    }

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.tapped
        || filter.untapped
        || filter.is_commander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.targets_player.is_some()
        || filter.targets_object.is_some();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    Ok(filter)
}

fn parse_spell_filter(tokens: &[Token]) -> crate::ability::SpellFilter {
    let mut filter = crate::ability::SpellFilter::default();
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    for word in words {
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }

        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    filter
}

fn split_on_or(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) || token.is_word("or") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn parse_card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

fn parse_non_type(word: &str) -> Option<CardType> {
    let rest = word.strip_prefix("non")?;
    parse_card_type(rest)
}

fn parse_non_color(word: &str) -> Option<ColorSet> {
    let rest = word.strip_prefix("non")?;
    match rest {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

fn parse_color(word: &str) -> Option<ColorSet> {
    match word {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

fn parse_zone_word(word: &str) -> Option<Zone> {
    match word {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

fn is_permanent_type(card_type: CardType) -> bool {
    matches!(
        card_type,
        CardType::Artifact
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
            | CardType::Battle
    )
}

fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

fn parse_number(tokens: &[Token]) -> Option<(u32, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }

    let value = match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    };

    Some((value, 1))
}

fn parse_value(tokens: &[Token]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if word == "x" {
        return Some((Value::X, 1));
    }

    let (number, used) = parse_number(tokens)?;
    Some((Value::Fixed(number as i32), used))
}

#[derive(Debug, Clone)]
struct CompileContext {
    next_effect_id: u32,
    next_tag_id: u32,
    last_effect_id: Option<EffectId>,
    last_object_tag: Option<String>,
    last_player_filter: Option<PlayerFilter>,
    iterated_player: bool,
    auto_tag_object_targets: bool,
}

impl CompileContext {
    fn new() -> Self {
        Self {
            next_effect_id: 0,
            next_tag_id: 0,
            last_effect_id: None,
            last_object_tag: None,
            last_player_filter: None,
            iterated_player: false,
            auto_tag_object_targets: false,
        }
    }

    fn next_effect_id(&mut self) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        id
    }

    fn next_tag(&mut self, prefix: &str) -> String {
        let tag = format!("{prefix}_{}", self.next_tag_id);
        self.next_tag_id += 1;
        tag
    }
}

fn compile_trigger_spec(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::ThisBlocks => Trigger::this_blocks(),
        TriggerSpec::ThisBecomesBlocked => Trigger::this_becomes_blocked(),
        TriggerSpec::ThisBlocksOrBecomesBlocked => Trigger::this_blocks_or_becomes_blocked(),
        TriggerSpec::ThisDies => Trigger::this_dies(),
        TriggerSpec::ThisLeavesBattlefield => Trigger::this_leaves_battlefield(),
        TriggerSpec::ThisBecomesMonstrous => Trigger::this_becomes_monstrous(),
        TriggerSpec::ThisBecomesTapped => Trigger::becomes_tapped(),
        TriggerSpec::ThisBecomesUntapped => Trigger::becomes_untapped(),
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::SpellCast { filter, caster } => Trigger::spell_cast(filter, caster),
        TriggerSpec::EntersBattlefield(filter) => Trigger::enters_battlefield(filter),
        TriggerSpec::EntersBattlefieldTapped(filter) => Trigger::enters_battlefield_tapped(filter),
        TriggerSpec::EntersBattlefieldUntapped(filter) => {
            Trigger::enters_battlefield_untapped(filter)
        }
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::ThisEntersBattlefield => Trigger::this_enters_battlefield(),
        TriggerSpec::ThisDealsCombatDamageToPlayer => Trigger::this_deals_combat_damage_to_player(),
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction { action, player } => Trigger::keyword_action(action, player),
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::Either(left, right) => {
            Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right))
        }
    }
}

fn compile_statement_effects(effects: &[EffectAst]) -> Result<Vec<Effect>, CardTextError> {
    let mut ctx = CompileContext::new();
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(effects, tag) {
            if ctx.last_object_tag.is_none() {
                ctx.last_object_tag = Some(tag.to_string());
            }
            prelude.push(Effect::tag_attached_to_source(tag));
        }
    }
    let (mut compiled, _) = compile_effects(effects, &mut ctx)?;
    if !prelude.is_empty() {
        prelude.append(&mut compiled);
        Ok(prelude)
    } else {
        Ok(compiled)
    }
}

fn inferred_trigger_player_filter(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    match trigger {
        TriggerSpec::SpellCast { caster, .. } => Some(caster.clone()),
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::KeywordAction { player, .. } => Some(player.clone()),
        TriggerSpec::Either(left, right) => {
            let left_filter = inferred_trigger_player_filter(left);
            let right_filter = inferred_trigger_player_filter(right);
            if left_filter == right_filter {
                left_filter
            } else {
                None
            }
        }
        _ => None,
    }
}

fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut ctx = CompileContext::new();
    ctx.last_player_filter = trigger.and_then(inferred_trigger_player_filter);
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(effects, tag) {
            if ctx.last_object_tag.is_none() {
                ctx.last_object_tag = Some(tag.to_string());
            }
            prelude.push(Effect::tag_attached_to_source(tag));
        }
    }
    if ctx.last_object_tag.is_none() && effects_reference_it_tag(effects) {
        ctx.last_object_tag = Some("triggering".to_string());
    }
    let (mut compiled, choices) = compile_effects(effects, &mut ctx)?;
    if !prelude.is_empty() {
        prelude.append(&mut compiled);
        compiled = prelude;
    }
    if effects_reference_tag(effects, "triggering")
        || matches!(ctx.last_object_tag.as_deref(), Some("triggering"))
    {
        compiled.insert(0, Effect::tag_triggering_object("triggering"));
    }
    Ok((compiled, choices))
}

fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

fn effect_references_tag(effect: &EffectAst, tag: &str) -> bool {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::ReturnToHand { target }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::PumpByLastEffect { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::CreateTokenCopyFromSource { source: target, .. } => {
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::SearchLibrary { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        EffectAst::MoveAllCounters { from, to } => {
            matches!(from, TargetAst::Tagged(t, _) if t.as_str() == tag)
                || matches!(to, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::PutIntoHand { object, .. } => {
            matches!(object, ObjectRefAst::It) && tag == IT_TAG
        }
        EffectAst::CreateTokenCopy { object, .. } => {
            matches!(object, ObjectRefAst::It) && tag == IT_TAG
        }
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects } => effects_reference_tag(effects, tag),
        EffectAst::VoteOption { effects, .. } => effects_reference_tag(effects, tag),
        _ => false,
    }
}

fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
}

fn effect_references_its_controller(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseGame { player }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateToken { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. } => matches!(player, PlayerAst::ItsController),
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            effects_reference_its_controller(if_true) || effects_reference_its_controller(if_false)
        }
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachTaggedPlayer { effects, .. } => {
            effects_reference_its_controller(effects)
        }
        EffectAst::VoteOption { effects, .. } => effects_reference_its_controller(effects),
        _ => false,
    }
}

fn effect_references_it_tag(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::ReturnToHand { target }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::PumpByLastEffect { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::CreateTokenCopyFromSource { source: target, .. } => {
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(
                predicate,
                PredicateAst::ItIsLandCard | PredicateAst::ItMatches(_)
            ) || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::SearchLibrary { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        EffectAst::MoveAllCounters { from, to } => {
            matches!(from, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
                || matches!(to, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::PutIntoHand { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::CreateTokenCopy { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects } => effects_reference_it_tag(effects),
        EffectAst::VoteOption { effects, .. } => effects_reference_it_tag(effects),
        _ => false,
    }
}

fn compile_effects(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut idx = 0;

    while idx < effects.len() {
        if let Some((effect_sequence, effect_choices, consumed)) =
            compile_vote_sequence(&effects[idx..], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += consumed;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_if_do_with_opponent_doesnt(&effects[idx], &effects[idx + 1], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        if idx + 1 < effects.len()
            && let EffectAst::CreateTokenWithMods {
                name,
                count,
                player,
                tapped,
                attacking,
                ..
            } = &effects[idx]
            && matches!(effects[idx + 1], EffectAst::ExileThatTokenAtEndOfCombat)
        {
            let effect = EffectAst::CreateTokenWithMods {
                name: name.clone(),
                count: *count,
                player: *player,
                tapped: *tapped,
                attacking: *attacking,
                exile_at_end_of_combat: true,
            };
            let (effect_list, effect_choices) = compile_effect(&effect, ctx)?;
            compiled.extend(effect_list);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        let remaining = if idx + 1 < effects.len() {
            &effects[idx + 1..]
        } else {
            &[]
        };
        ctx.auto_tag_object_targets =
            effects_reference_it_tag(remaining) || effects_reference_its_controller(remaining);

        let next_is_if_result =
            idx + 1 < effects.len() && matches!(effects[idx + 1], EffectAst::IfResult { .. });
        let next_is_if_result_with_opponent_doesnt = next_is_if_result
            && idx + 2 < effects.len()
            && matches!(effects[idx + 2], EffectAst::ForEachOpponentDoesNot { .. });
        if next_is_if_result && !next_is_if_result_with_opponent_doesnt {
            let (mut effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
            if !effect_list.is_empty() {
                let id = ctx.next_effect_id();
                let last = effect_list.pop().expect("non-empty effect list");
                effect_list.push(Effect::with_id(id.0, last));
                ctx.last_effect_id = Some(id);
            } else {
                ctx.last_effect_id = None;
            }

            compiled.extend(effect_list);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }

            let if_remaining = if idx + 2 < effects.len() {
                &effects[idx + 2..]
            } else {
                &[]
            };
            ctx.auto_tag_object_targets = effects_reference_it_tag(if_remaining)
                || effects_reference_its_controller(if_remaining);
            let (if_effects, if_choices) = compile_effect(&effects[idx + 1], ctx)?;
            compiled.extend(if_effects);
            for choice in if_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        let (effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
        compiled.extend(effect_list);
        for choice in effect_choices {
            push_choice(&mut choices, choice);
        }
        idx += 1;
    }

    Ok((compiled, choices))
}

fn collect_tag_spans_from_line(
    line: &LineAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    match line {
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::Ability(_)
        | LineAst::Abilities(_) => {}
    }
}

fn collect_tag_spans_from_effects_with_context(
    effects: &[EffectAst],
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    for effect in effects {
        collect_tag_spans_from_effect(effect, annotations, ctx);
    }
}

fn collect_tag_spans_from_effect(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::PumpByLastEffect { target, .. } => {
            collect_tag_spans_from_target(target, annotations, ctx);
        }
        EffectAst::MoveAllCounters { from, to } => {
            collect_tag_spans_from_target(from, annotations, ctx);
            collect_tag_spans_from_target(to, annotations, ctx);
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            collect_tag_spans_from_effects_with_context(if_true, annotations, ctx);
            collect_tag_spans_from_effects_with_context(if_false, annotations, ctx);
        }
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        EffectAst::VoteOption { effects, .. } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        _ => {}
    }
}

fn collect_tag_spans_from_target(
    target: &TargetAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    if let TargetAst::Tagged(tag, Some(span)) = target {
        let mapped = map_span_to_original(*span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(tag, mapped);
    }
    if let TargetAst::Object(filter, _, Some(it_span)) = target
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mapped = map_span_to_original(*it_span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(&TagKey::from(IT_TAG), mapped);
    }
}

fn compile_if_do_with_opponent_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged = EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        }],
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn compile_vote_sequence(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(EffectAst::VoteStart { options }) = effects.first() else {
        return Ok(None);
    };

    let mut option_effects: HashMap<String, Vec<EffectAst>> = HashMap::new();
    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let mut consumed = 1;

    while consumed < effects.len() {
        match &effects[consumed] {
            EffectAst::VoteOption { option, effects } => {
                if option_effects
                    .insert(option.clone(), effects.clone())
                    .is_some()
                {
                    return Err(CardTextError::ParseError(format!(
                        "duplicate vote option clause for '{option}'"
                    )));
                }
                consumed += 1;
            }
            EffectAst::VoteExtra { count, optional } => {
                if *optional {
                    extra_optional = extra_optional.saturating_add(*count);
                } else {
                    extra_mandatory = extra_mandatory.saturating_add(*count);
                }
                consumed += 1;
            }
            _ => break,
        }
    }

    let saved_iterated = ctx.iterated_player;
    let saved_last_effect = ctx.last_effect_id;
    let saved_last_tag = ctx.last_object_tag.clone();
    let saved_last_player = ctx.last_player_filter.clone();
    ctx.iterated_player = true;

    let mut vote_options = Vec::new();
    let mut choices = Vec::new();
    for option in options {
        let option_effects_ast = option_effects.get(option).ok_or_else(|| {
            CardTextError::ParseError(format!("missing effects for vote option '{option}'"))
        })?;
        ctx.last_effect_id = None;
        ctx.last_object_tag = None;
        ctx.last_player_filter = None;
        let (compiled, option_choices) = compile_effects(option_effects_ast, ctx)?;
        for choice in option_choices {
            push_choice(&mut choices, choice);
        }
        vote_options.push(VoteOption::new(option.clone(), compiled));
    }

    ctx.iterated_player = saved_iterated;
    ctx.last_effect_id = saved_last_effect;
    ctx.last_object_tag = saved_last_tag;
    ctx.last_player_filter = saved_last_player;

    let effect = if extra_optional > 0 {
        Effect::vote_with_optional_extra(vote_options, extra_mandatory, extra_optional)
    } else {
        Effect::vote(vote_options, extra_mandatory)
    };

    Ok(Some((vec![effect], choices, consumed)))
}

fn compile_effect(
    effect: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::deal_damage(amount.clone(), spec.clone()),
                &spec,
                ctx,
                "damaged",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            if let TargetAst::Player(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DealDamageEach { amount, filter } => {
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::deal_damage(amount.clone(), ChooseSpec::Iterated)],
            );
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PutCounters {
            counter_type,
            count,
            target,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::put_counters(*counter_type, *count, spec.clone()),
                &spec,
                ctx,
                "counters",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Proliferate => Ok((vec![Effect::proliferate()], Vec::new())),
        EffectAst::Tap { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Untap { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::UntapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let effect = Effect::untap_all(resolved_filter);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::GrantProtectionChoice {
            target,
            allow_colorless,
        } => {
            let spec = choose_spec_for_target(target);
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    description: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];

            for (name, color) in colors {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Color(
                    ColorSet::from(color),
                ));
                modes.push(EffectMode {
                    description: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let effect = Effect::choose_one(modes);
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = Effect::new(crate::effects::EarthbendEffect::new(
                spec.clone(),
                *counters,
            ));
            Ok((vec![effect], vec![spec]))
        }
        EffectAst::Draw { count, player } => match player {
            PlayerAst::Target => {
                let effect = Effect::target_draws(count.clone(), PlayerFilter::target_player());
                ctx.last_player_filter = Some(PlayerFilter::target_player());
                Ok((vec![effect], vec![ChooseSpec::target_player()]))
            }
            _ => {
                let filter = resolve_non_target_player_filter(*player, ctx)?;
                let effect = if matches!(&filter, PlayerFilter::You) {
                    Effect::draw(count.clone())
                } else {
                    Effect::target_draws(count.clone(), filter.clone())
                };
                if !matches!(*player, PlayerAst::Implicit) {
                    ctx.last_player_filter = Some(filter);
                }
                Ok((vec![effect], Vec::new()))
            }
        },
        EffectAst::Counter { target } => {
            let spec = choose_spec_for_target(target);
            let effect =
                tag_object_target_effect(Effect::counter(spec.clone()), &spec, ctx, "countered");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::CounterUnlessPays { target, mana } => {
            let spec = choose_spec_for_target(target);
            let effect = tag_object_target_effect(
                Effect::counter_unless_pays(spec.clone(), mana.clone()),
                &spec,
                ctx,
                "countered",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::LoseLife { amount, player } => match player {
            PlayerAst::Target => Ok((vec![Effect::lose_life_target(amount.clone())], {
                ctx.last_player_filter = Some(PlayerFilter::target_player());
                vec![ChooseSpec::target_player()]
            })),
            _ => {
                let filter = resolve_non_target_player_filter(*player, ctx)?;
                let effect = if matches!(&filter, PlayerFilter::You) {
                    Effect::lose_life(amount.clone())
                } else {
                    Effect::lose_life_player(amount.clone(), filter.clone())
                };
                if !matches!(*player, PlayerAst::Implicit) {
                    ctx.last_player_filter = Some(filter);
                }
                Ok((vec![effect], Vec::new()))
            }
        },
        EffectAst::GainLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            match player {
                PlayerAst::Target => Ok((vec![Effect::gain_life_target(amount.clone())], {
                    ctx.last_player_filter = Some(PlayerFilter::target_player());
                    vec![ChooseSpec::target_player()]
                })),
                _ => {
                    let filter = resolve_non_target_player_filter(*player, ctx)?;
                    let effect = if matches!(&filter, PlayerFilter::You) {
                        Effect::gain_life(amount.clone())
                    } else {
                        Effect::gain_life_player(amount.clone(), ChooseSpec::Player(filter.clone()))
                    };
                    if !matches!(*player, PlayerAst::Implicit) {
                        ctx.last_player_filter = Some(filter);
                    }
                    Ok((vec![effect], Vec::new()))
                }
            }
        }
        EffectAst::LoseGame { player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::lose_the_game()
            } else {
                Effect::lose_the_game_player(filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PreventAllCombatDamage { duration } => Ok((
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        )),
        EffectAst::AddMana { mana, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana(mana.clone())
            } else {
                Effect::add_mana_player(mana.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaAnyColor { amount, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_of_any_color(amount.clone())
            } else {
                Effect::add_mana_of_any_color_player(amount.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaAnyOneColor { amount, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_of_any_one_color(amount.clone())
            } else {
                Effect::add_mana_of_any_one_color_player(amount.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaCommanderIdentity { amount, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_from_commander_color_identity(amount.clone())
            } else {
                Effect::add_mana_from_commander_color_identity_player(
                    amount.clone(),
                    filter.clone(),
                )
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaImprintedColors => Ok((
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        )),
        EffectAst::Scry { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::scry(count.clone())
            } else {
                Effect::scry_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::Surveil { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::surveil(count.clone())
            } else {
                Effect::surveil_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::Cant {
            restriction,
            duration,
        } => Ok((
            vec![Effect::cant_until(restriction.clone(), duration.clone())],
            Vec::new(),
        )),
        EffectAst::PlayFromGraveyardUntilEot { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::grant_play_from_graveyard_until_eot(player_filter);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::ExileInsteadOfGraveyardThisTurn { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::exile_instead_of_graveyard_this_turn(player_filter);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::GainControl { target, duration } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let mut effect = Effect::gain_control_with_duration(spec.clone(), duration.clone());
            let mut choices = Vec::new();
            if spec.is_target() {
                let tag = ctx.next_tag("controlled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ControlPlayer { player, duration } => {
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(player.clone());
            }

            let effect = Effect::control_player(player.clone(), start, duration);
            Ok((vec![effect], choices))
        }
        EffectAst::ExtraTurnAfterTurn { player } => {
            let (player_filter, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(player_filter.clone()),
                vec![Effect::extra_turn_player(player_filter.clone())],
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            Ok((vec![effect], choices))
        }
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            tag,
        } => {
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let effect =
                Effect::choose_objects(resolved_filter, *count, chooser.clone(), tag.clone());
            ctx.last_object_tag = Some(tag.as_str().to_string());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Sacrifice { filter, player } => {
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let tag = ctx.next_tag("sacrificed");
            ctx.last_object_tag = Some(tag.clone());
            let choose = Effect::choose_objects(resolved_filter, 1, chooser.clone(), tag.clone());
            let sacrifice = Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, chooser.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![choose, sacrifice], choices))
        }
        EffectAst::SacrificeAll { filter, player } => {
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DiscardHand { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::discard_hand()
            } else {
                Effect::discard_hand_player(filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Discard {
            count,
            player,
            random,
        } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::discard_player(count.clone(), PlayerFilter::You, *random)
            } else {
                Effect::discard_player(count.clone(), filter.clone(), *random)
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnToHand { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec.clone())),
                &spec,
                ctx,
                "returned",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnToBattlefield { target, tapped } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::return_from_graveyard_to_battlefield(spec.clone(), *tapped),
                &spec,
                ctx,
                "returned",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec.clone());
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnAllToHand { filter } => {
            Ok((vec![Effect::return_all_to_hand(filter.clone())], Vec::new()))
        }
        EffectAst::ExchangeControl { filter, count } => {
            let first = ChooseSpec::Object(filter.clone());
            let second = ChooseSpec::Object(filter.clone());
            let effect = Effect::exchange_control(first, second);
            let target_spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
                .with_count(ChoiceCount::exactly(*count as usize));
            Ok((vec![effect], vec![target_spec]))
        }
        EffectAst::SetLifeTotal { amount, player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::set_life_total_player(amount.clone(), filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::SkipTurn { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::skip_turn_player(filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::SkipDrawStep { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::skip_draw_step_player(filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Regenerate { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Mill { count, player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::mill(count.clone())
            } else {
                Effect::mill_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::PoisonCounters { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::poison_counters(count.clone())
            } else {
                Effect::poison_counters_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::EnergyCounters { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::energy_counters(count.clone())
            } else {
                Effect::energy_counters_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::May { effects } => {
            let saved_last_effect = ctx.last_effect_id;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let effect = Effect::with_id(id.0, Effect::may(inner_effects));
            Ok((vec![effect], inner_choices))
        }
        EffectAst::MayByTaggedController { tag, effects } => {
            let saved_last_effect = ctx.last_effect_id;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let effect = Effect::with_id(
                id.0,
                Effect::for_each_controller_of_tagged(
                    tag.clone(),
                    vec![Effect::may(inner_effects)],
                ),
            );
            Ok((vec![effect], inner_choices))
        }
        EffectAst::IfResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for if clause".to_string())
            })?;
            let saved_last_effect = ctx.last_effect_id;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
            let predicate = match predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
            };
            let effect = Effect::if_then(condition, predicate, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponent { effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_each_opponent(inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachPlayer { effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_players(PlayerFilter::Any, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponentDoesNot { .. } => Err(CardTextError::ParseError(
            "for each opponent who doesn't must follow an opponent clause".to_string(),
        )),
        EffectAst::Destroy { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let mut effect = Effect::destroy(spec.clone());
            if spec.is_target() {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DestroyAll { filter } => {
            Ok((vec![Effect::destroy_all(filter.clone())], Vec::new()))
        }
        EffectAst::ExileAll { filter } => Ok((vec![Effect::exile_all(filter.clone())], Vec::new())),
        EffectAst::Exile { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let mut effect = if spec.count().is_single() {
                Effect::move_to_zone(spec.clone(), Zone::Exile, true)
            } else {
                Effect::exile(spec.clone())
            };
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::LookAtHand { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = Effect::new(crate::effects::LookAtHandEffect::new(spec.clone()));
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            if let TargetAst::Player(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((vec![effect], choices))
        }
        EffectAst::TargetOnly { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::TargetOnlyEffect::new(spec.clone())),
                &spec,
                ctx,
                "targeted",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::RevealTop { player } => {
            let mut choices = Vec::new();
            let player_filter = match player {
                PlayerAst::Target => {
                    choices.push(ChooseSpec::target_player());
                    PlayerFilter::target_player()
                }
                _ => resolve_non_target_player_filter(*player, ctx)?,
            };
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::reveal_top(player_filter, tag);
            Ok((vec![effect], choices))
        }
        EffectAst::RevealHand { player } => {
            let mut choices = Vec::new();
            let spec = match player {
                PlayerAst::Target => {
                    choices.push(ChooseSpec::target_player());
                    ChooseSpec::target_player()
                }
                _ => {
                    let filter = resolve_non_target_player_filter(*player, ctx)?;
                    ChooseSpec::Player(filter)
                }
            };
            if !matches!(*player, PlayerAst::Implicit) {
                if let ChooseSpec::Player(filter) = &spec {
                    ctx.last_player_filter = Some(filter.clone());
                } else if matches!(*player, PlayerAst::Target) {
                    ctx.last_player_filter = Some(PlayerFilter::target_player());
                }
            }
            let effect = Effect::new(crate::effects::LookAtHandEffect::new(spec));
            Ok((vec![effect], choices))
        }
        EffectAst::PutIntoHand { player, object } => {
            let tag = match object {
                ObjectRefAst::It => ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?,
            };
            let mut choices = Vec::new();
            if matches!(player, PlayerAst::Target) {
                choices.push(ChooseSpec::target_player());
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else if !matches!(*player, PlayerAst::Implicit) {
                let resolved = resolve_non_target_player_filter(*player, ctx)?;
                ctx.last_player_filter = Some(resolved);
            }
            let effect = Effect::move_to_zone(ChooseSpec::tagged(tag), Zone::Hand, false);
            Ok((vec![effect], choices))
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let saved_last_tag = ctx.last_object_tag.clone();
            let (true_effects, true_choices) = compile_effects(if_true, ctx)?;
            ctx.last_object_tag = saved_last_tag.clone();
            let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
            ctx.last_object_tag = saved_last_tag.clone();
            let condition = match predicate {
                PredicateAst::ItIsLandCard => {
                    let tag = saved_last_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "conditional requires prior reference".to_string(),
                        )
                    })?;
                    Condition::TaggedObjectMatches(
                        tag.into(),
                        ObjectFilter {
                            zone: None,
                            card_types: vec![CardType::Land],
                            ..Default::default()
                        },
                    )
                }
                PredicateAst::ItMatches(filter) => {
                    let tag = saved_last_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "conditional requires prior reference".to_string(),
                        )
                    })?;
                    let mut resolved = filter.clone();
                    resolved.zone = None;
                    Condition::TaggedObjectMatches(tag.into(), resolved)
                }
                PredicateAst::TaggedMatches(tag, filter) => {
                    let mut resolved = filter.clone();
                    resolved.zone = None;
                    Condition::TaggedObjectMatches(tag.clone(), resolved)
                }
            };
            let effect = if false_effects.is_empty() {
                Effect::conditional_only(condition, true_effects)
            } else {
                Effect::conditional(condition, true_effects, false_effects)
            };
            let mut choices = true_choices;
            choices.extend(false_choices);
            Ok((vec![effect], choices))
        }
        EffectAst::Enchant { filter } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()));
            let effect = Effect::attach_to(spec.clone());
            Ok((vec![effect], vec![spec]))
        }
        EffectAst::Investigate => Ok((vec![Effect::investigate(1)], Vec::new())),
        EffectAst::CreateTokenWithMods {
            name,
            count,
            player,
            tapped,
            attacking,
            exile_at_end_of_combat,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let player_filter = if matches!(*player, PlayerAst::Implicit) {
                PlayerFilter::You
            } else {
                resolve_non_target_player_filter(*player, ctx)?
            };
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, *count)
            } else {
                crate::effects::CreateTokenEffect::new(token, *count, player_filter.clone())
            };
            if *tapped {
                effect = effect.tapped();
            }
            if *attacking {
                effect = effect.attacking();
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_end_of_combat();
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        EffectAst::CreateToken {
            name,
            count,
            player,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let player_filter = if matches!(*player, PlayerAst::Implicit) {
                PlayerFilter::You
            } else {
                resolve_non_target_player_filter(*player, ctx)?
            };
            let effect = if matches!(player_filter, PlayerFilter::You) {
                Effect::create_tokens(token, *count)
            } else {
                Effect::create_tokens_player(token, *count, player_filter)
            };
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::CreateTokenCopy {
            object,
            count,
            player,
            half_power_toughness_round_up,
        } => {
            let tag = match object {
                ObjectRefAst::It => ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'that creature' without prior reference".to_string(),
                    )
                })?,
            };
            let tag: TagKey = tag.into();
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::Tagged(tag),
                *count,
                player_filter,
            );
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        EffectAst::CreateTokenCopyFromSource {
            source,
            count,
            player,
            half_power_toughness_round_up,
        } => {
            let source_spec = choose_spec_for_target(source);
            let source_spec = resolve_choose_spec_it_tag(&source_spec, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                source_spec.clone(),
                *count,
                player_filter,
            );
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            let mut choices = Vec::new();
            if source_spec.is_target() {
                choices.push(source_spec);
            }
            Ok((vec![Effect::new(effect)], choices))
        }
        EffectAst::ExileThatTokenAtEndOfCombat => Ok((Vec::new(), Vec::new())),
        EffectAst::Monstrosity { amount } => {
            Ok((vec![Effect::monstrosity(amount.clone())], Vec::new()))
        }
        EffectAst::RemoveUpToAnyCounters { amount, target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let effect = tag_object_target_effect(
                Effect::with_id(
                    id.0,
                    Effect::remove_up_to_any_counters(amount.clone(), spec.clone()),
                ),
                &spec,
                ctx,
                "counters",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::MoveAllCounters { from, to } => {
            let from_spec = choose_spec_for_target(from);
            let from_spec = resolve_choose_spec_it_tag(&from_spec, ctx)?;
            let to_spec = choose_spec_for_target(to);
            let to_spec = resolve_choose_spec_it_tag(&to_spec, ctx)?;
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            let mut choices = Vec::new();
            if from_spec.is_target() {
                choices.push(from_spec);
            }
            if to_spec.is_target() {
                choices.push(to_spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::pump(
                    power.clone(),
                    toughness.clone(),
                    spec.clone(),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "pumped",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::pump_all(
                resolved_filter.clone(),
                power.clone(),
                toughness.clone(),
                duration.clone(),
            )
            .tag_all(tag.clone());
            ctx.last_object_tag = Some(tag);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            let effect = tag_object_target_effect(
                Effect::pump(
                    power_value,
                    Value::Fixed(*toughness),
                    spec.clone(),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "pumped",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => Ok((
            vec![Effect::grant_abilities_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            )],
            Vec::new(),
        )),
        EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::GrantAbilitiesTargetEffect::new(
                    spec.clone(),
                    abilities.clone(),
                    duration.clone(),
                )),
                &spec,
                ctx,
                "granted",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Transform { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::transform(spec.clone()),
                &spec,
                ctx,
                "transformed",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::GrantAbilityToSource { ability } => Ok((
            vec![Effect::grant_object_ability_to_source(ability.clone())],
            Vec::new(),
        )),
        EffectAst::SearchLibrary {
            filter,
            destination,
            player,
            reveal: _reveal,
            shuffle,
            count,
        } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let count = *count;
            let mut filter = filter.clone();
            if filter.owner.is_none() {
                filter.owner = Some(player_filter.clone());
            }
            let tag = ctx.next_tag("searched");
            let choose = crate::effects::ChooseObjectsEffect::new(
                filter,
                count,
                player_filter.clone(),
                tag.clone(),
            )
            .in_zone(Zone::Library)
            .with_description("cards")
            .as_search();

            let to_top = matches!(destination, Zone::Library);
            let move_effect = Effect::move_to_zone(ChooseSpec::Iterated, *destination, to_top);
            let mut sequence_effects = vec![Effect::new(choose)];
            if *shuffle && *destination == Zone::Library {
                sequence_effects.push(Effect::shuffle_library_player(player_filter.clone()));
                sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
            } else {
                sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                if *shuffle {
                    sequence_effects.push(Effect::shuffle_library_player(player_filter));
                }
            }

            let sequence = crate::effects::SequenceEffect::new(sequence_effects);
            Ok((vec![Effect::new(sequence)], Vec::new()))
        }
        EffectAst::ShuffleLibrary { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            Ok((
                vec![Effect::shuffle_library_player(player_filter)],
                Vec::new(),
            ))
        }
        EffectAst::VoteStart { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. } => Err(CardTextError::ParseError(
            "vote clauses must appear together".to_string(),
        )),
    }
}

fn resolve_non_target_player_filter(
    player: PlayerAst,
    ctx: &CompileContext,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
        PlayerAst::Defending => Ok(PlayerFilter::Defending),
        PlayerAst::Target => Err(CardTextError::ParseError(
            "target player requires explicit targeting".to_string(),
        )),
        PlayerAst::That => {
            if ctx.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else if let Some(filter) = &ctx.last_player_filter {
                Ok(filter.clone())
            } else {
                Err(CardTextError::ParseError(
                    "cannot resolve 'that player' without context".to_string(),
                ))
            }
        }
        PlayerAst::ItsController => {
            let tag = ctx.last_object_tag.as_ref().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot resolve 'its controller' without prior reference".to_string(),
                )
            })?;
            Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)))
        }
        PlayerAst::Implicit => {
            if ctx.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else {
                Ok(PlayerFilter::You)
            }
        }
    }
}

fn resolve_player_filter_with_target(
    player: PlayerAst,
    ctx: &CompileContext,
) -> Result<(PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    match player {
        PlayerAst::Target => Ok((
            PlayerFilter::target_player(),
            vec![ChooseSpec::target_player()],
        )),
        _ => Ok((resolve_non_target_player_filter(player, ctx)?, Vec::new())),
    }
}

fn resolve_it_tag(
    filter: &ObjectFilter,
    ctx: &CompileContext,
) -> Result<ObjectFilter, CardTextError> {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        return Ok(filter.clone());
    }

    let tag = ctx.last_object_tag.as_ref().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'it' without prior reference".to_string())
    })?;

    let mut resolved = filter.clone();
    for constraint in &mut resolved.tagged_constraints {
        if constraint.tag.as_str() == IT_TAG {
            constraint.tag = tag.into();
        }
    }
    Ok(resolved)
}

fn resolve_choose_spec_it_tag(
    spec: &ChooseSpec,
    ctx: &CompileContext,
) -> Result<ChooseSpec, CardTextError> {
    match spec {
        ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG => {
            let resolved = ctx.last_object_tag.as_ref().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'it' without prior reference".to_string(),
                )
            })?;
            Ok(ChooseSpec::Tagged(TagKey::from(resolved.as_str())))
        }
        ChooseSpec::Tagged(tag) => Ok(ChooseSpec::Tagged(tag.clone())),
        ChooseSpec::Object(filter) => Ok(ChooseSpec::Object(resolve_it_tag(filter, ctx)?)),
        ChooseSpec::Target(inner) => Ok(ChooseSpec::Target(Box::new(resolve_choose_spec_it_tag(
            inner, ctx,
        )?))),
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag(inner, ctx)?),
            count.clone(),
        )),
        ChooseSpec::All(filter) => Ok(ChooseSpec::All(resolve_it_tag(filter, ctx)?)),
        ChooseSpec::Player(filter) => Ok(ChooseSpec::Player(filter.clone())),
        ChooseSpec::SpecificObject(id) => Ok(ChooseSpec::SpecificObject(*id)),
        ChooseSpec::SpecificPlayer(id) => Ok(ChooseSpec::SpecificPlayer(*id)),
        ChooseSpec::AnyTarget => Ok(ChooseSpec::AnyTarget),
        ChooseSpec::Source => Ok(ChooseSpec::Source),
        ChooseSpec::SourceController => Ok(ChooseSpec::SourceController),
        ChooseSpec::SourceOwner => Ok(ChooseSpec::SourceOwner),
        ChooseSpec::EachPlayer(filter) => Ok(ChooseSpec::EachPlayer(filter.clone())),
        ChooseSpec::Iterated => Ok(ChooseSpec::Iterated),
    }
}

fn resolve_value_it_tag(value: &Value, ctx: &CompileContext) -> Result<Value, CardTextError> {
    match value {
        Value::PowerOf(spec) => Ok(Value::PowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        _ => Ok(value.clone()),
    }
}

fn choose_spec_targets_object(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::Object(_)
        | ChooseSpec::Tagged(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source => true,
        _ => false,
    }
}

fn tag_object_target_effect(
    effect: Effect,
    spec: &ChooseSpec,
    ctx: &mut CompileContext,
    prefix: &str,
) -> Effect {
    if ctx.auto_tag_object_targets && spec.is_target() && choose_spec_targets_object(spec) {
        let tag = ctx.next_tag(prefix);
        ctx.last_object_tag = Some(tag.clone());
        effect.tag(tag)
    } else {
        effect
    }
}

fn token_definition_for(name: &str) -> Option<CardDefinition> {
    let lower = name.trim().to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let has_word = |needle: &str| words.iter().any(|word| *word == needle);

    if has_word("treasure") {
        return Some(crate::cards::tokens::treasure_token_definition());
    }
    if has_word("clue") {
        return Some(crate::cards::tokens::clue_token_definition());
    }
    if has_word("food") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Food")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food]);
        return Some(builder.build());
    }
    if has_word("blood") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Blood")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("powerstone") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Powerstone")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("angel") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(4, 4))
            .flying();
        return Some(builder.build());
    }
    if has_word("wall")
        && lower.contains("0/4")
        && lower.contains("artifact")
        && lower.contains("creature")
    {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Wall")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Wall])
            .power_toughness(PowerToughness::fixed(0, 4))
            .defender();
        return Some(builder.build());
    }
    if has_word("squirrel") && lower.contains("1/1") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Squirrel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Squirrel])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(1, 1));
        return Some(builder.build());
    }
    if has_word("elephant") && lower.contains("3/3") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Elephant")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3));
        return Some(builder.build());
    }
    if has_word("construct") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Construct")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Construct])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(
                StaticAbility::characteristic_defining_pt(
                    Value::Count(ObjectFilter::artifact().you_control()),
                    Value::Count(ObjectFilter::artifact().you_control()),
                ),
            ));
        return Some(builder.build());
    }
    if has_word("vampire") && lower.contains("1/1") && lower.contains("white") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Vampire")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Vampire])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(1, 1));
        if lower.contains("lifelink") {
            builder = builder.lifelink();
        }
        return Some(builder.build());
    }
    if has_word("human") && lower.contains("1/1") && lower.contains("white") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Human")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(1, 1));
        return Some(builder.build());
    }
    if has_word("shapeshifter") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(3, 2));
        if lower.contains("changeling") || lower == "shapeshifter" {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        return Some(builder.build());
    }
    if has_word("astartes")
        && has_word("warrior")
        && lower.contains("2/2")
        && lower.contains("white")
    {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Astartes Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(2, 2));
        if lower.contains("vigilance") {
            builder = builder.vigilance();
        }
        return Some(builder.build());
    }
    if words.contains(&"creature") {
        let mut card_types = vec![CardType::Creature];
        if words.contains(&"artifact") {
            card_types.insert(0, CardType::Artifact);
        }

        let (power, toughness) = words.iter().find_map(|word| parse_token_pt(word))?;

        let mut subtypes = Vec::new();
        for word in &words {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }

        let token_name = subtypes
            .first()
            .map(|subtype| format!("{subtype:?}"))
            .unwrap_or_else(|| "Token".to_string());

        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(power, toughness));

        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }

        let mut colors = ColorSet::new();
        if words.contains(&"white") {
            colors = colors.union(ColorSet::WHITE);
        }
        if words.contains(&"blue") {
            colors = colors.union(ColorSet::BLUE);
        }
        if words.contains(&"black") {
            colors = colors.union(ColorSet::BLACK);
        }
        if words.contains(&"red") {
            colors = colors.union(ColorSet::RED);
        }
        if words.contains(&"green") {
            colors = colors.union(ColorSet::GREEN);
        }
        if !colors.is_empty() {
            builder = builder.color_indicator(colors);
        }

        if words.contains(&"flying") {
            builder = builder.flying();
        }
        if words.contains(&"vigilance") {
            builder = builder.vigilance();
        }
        if words.contains(&"trample") {
            builder = builder.trample();
        }
        if words.contains(&"lifelink") {
            builder = builder.lifelink();
        }
        if words.contains(&"deathtouch") {
            builder = builder.deathtouch();
        }
        if words.contains(&"haste") {
            builder = builder.haste();
        }
        if words.contains(&"menace") {
            builder = builder.menace();
        }
        if words.contains(&"reach") {
            builder = builder.reach();
        }
        if words.contains(&"hexproof") {
            builder = builder.hexproof();
        }
        if words.contains(&"indestructible") {
            builder = builder.indestructible();
        }
        if words.contains(&"first") && words.contains(&"strike") {
            builder = builder.first_strike();
        }
        if words.contains(&"double") && words.contains(&"strike") {
            builder = builder.double_strike();
        }
        if words.contains(&"changeling") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }

        return Some(builder.build());
    }
    None
}

fn parse_token_pt(word: &str) -> Option<(i32, i32)> {
    let (left, right) = word.split_once('/')?;
    let power = left.parse::<i32>().ok()?;
    let toughness = right.parse::<i32>().ok()?;
    Some((power, toughness))
}

fn choose_spec_for_target(target: &TargetAst) -> ChooseSpec {
    match target {
        TargetAst::Source(_) => ChooseSpec::Source,
        TargetAst::AnyTarget(_) => ChooseSpec::AnyTarget,
        TargetAst::Spell(_) => ChooseSpec::target_spell(),
        TargetAst::Player(filter, _) => {
            if *filter == PlayerFilter::You {
                ChooseSpec::SourceController
            } else if *filter == PlayerFilter::IteratedPlayer {
                ChooseSpec::Player(filter.clone())
            } else {
                ChooseSpec::target(ChooseSpec::Player(filter.clone()))
            }
        }
        TargetAst::Object(filter, _, _) => ChooseSpec::target(ChooseSpec::Object(filter.clone())),
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone()),
    }
}

fn push_choice(choices: &mut Vec<ChooseSpec>, choice: ChooseSpec) {
    if !choices.iter().any(|existing| existing == &choice) {
        choices.push(choice);
    }
}

/// Builder for creating CardDefinitions with abilities.
#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    /// The underlying card builder
    card_builder: CardBuilder,

    /// Abilities to add to the card
    abilities: Vec<Ability>,

    /// Spell effects for instants/sorceries
    spell_effect: Option<Vec<Effect>>,

    /// Alternative casting methods (flashback, escape, etc.)
    alternative_casts: Vec<AlternativeCastingMethod>,

    /// Optional costs (kicker, buyback, etc.)
    optional_costs: Vec<OptionalCost>,

    /// For sagas: the maximum chapter number
    max_saga_chapter: Option<u32>,

    /// Cost effects (new unified model) - effects that are executed as part of paying costs.
    cost_effects: Vec<Effect>,

    /// For Auras: what this card can enchant (used for non-target attachments)
    aura_attach_filter: Option<ObjectFilter>,
}

impl CardDefinitionBuilder {
    fn pt_value_text(value: PtValue) -> String {
        match value {
            PtValue::Fixed(n) => n.to_string(),
            PtValue::Star => "*".to_string(),
            PtValue::StarPlus(n) => {
                if n >= 0 {
                    format!("*+{n}")
                } else {
                    format!("*{n}")
                }
            }
        }
    }

    fn type_line_text(
        supertypes: &[Supertype],
        card_types: &[CardType],
        subtypes: &[Subtype],
    ) -> Option<String> {
        if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        let mut left = Vec::new();
        for supertype in supertypes {
            left.push(format!("{:?}", supertype));
        }
        for card_type in card_types {
            left.push(format!("{:?}", card_type));
        }

        let mut line = left.join(" ");
        if !subtypes.is_empty() {
            let right = subtypes
                .iter()
                .map(|subtype| format!("{:?}", subtype))
                .collect::<Vec<_>>()
                .join(" ");
            if !line.is_empty() {
                line.push_str(" — ");
            }
            line.push_str(&right);
        }
        Some(line)
    }

    fn build_text_with_metadata(&self, rules: &str) -> String {
        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = Self::type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                Self::pt_value_text(pt.power),
                Self::pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        lines.join("\n")
    }

    /// Create a new card definition builder.
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
            aura_attach_filter: None,
        }
    }

    // === Card properties (delegated to CardBuilder) ===

    /// Set the mana cost.
    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
        self
    }

    /// Set the color indicator.
    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.card_builder = self.card_builder.color_indicator(colors);
        self
    }

    /// Set the supertypes.
    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.card_builder = self.card_builder.supertypes(supertypes);
        self
    }

    /// Set the card types.
    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_builder = self.card_builder.card_types(types);
        self
    }

    /// Set the subtypes.
    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.card_builder = self.card_builder.subtypes(subtypes);
        self
    }

    /// Set the oracle text.
    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.oracle_text(text);
        self
    }

    fn apply_keyword_action(self, action: KeywordAction) -> Self {
        match action {
            KeywordAction::Flying => self.flying(),
            KeywordAction::Menace => self.menace(),
            KeywordAction::Hexproof => self.hexproof(),
            KeywordAction::Haste => self.haste(),
            KeywordAction::Improvise => self.improvise(),
            KeywordAction::Convoke => self.convoke(),
            KeywordAction::AffinityForArtifacts => self.affinity_for_artifacts(),
            KeywordAction::Delve => self.delve(),
            KeywordAction::FirstStrike => self.first_strike(),
            KeywordAction::DoubleStrike => self.double_strike(),
            KeywordAction::Deathtouch => self.deathtouch(),
            KeywordAction::Lifelink => self.lifelink(),
            KeywordAction::Vigilance => self.vigilance(),
            KeywordAction::Trample => self.trample(),
            KeywordAction::Reach => self.reach(),
            KeywordAction::Defender => self.defender(),
            KeywordAction::Flash => self.flash(),
            KeywordAction::Indestructible => self.indestructible(),
            KeywordAction::Shroud => self.shroud(),
            KeywordAction::Ward(amount) => self.ward_generic(amount),
            KeywordAction::Wither => self.wither(),
            KeywordAction::Infect => self.infect(),
            KeywordAction::Undying => self.undying(),
            KeywordAction::Persist => self.persist(),
            KeywordAction::Prowess => self.prowess(),
            KeywordAction::Exalted => self.exalted(),
            KeywordAction::Storm => self.storm(),
            KeywordAction::Toxic(amount) => self.toxic(amount),
            KeywordAction::Fear => self.fear(),
            KeywordAction::Intimidate => self.intimidate(),
            KeywordAction::Shadow => self.shadow(),
            KeywordAction::Horsemanship => self.horsemanship(),
            KeywordAction::Flanking => {
                self.with_ability(Ability::static_ability(StaticAbility::flanking()))
            }
            KeywordAction::Bushido(amount) => self.bushido(amount),
            KeywordAction::Changeling => {
                self.with_ability(Ability::static_ability(StaticAbility::changeling()))
            }
            KeywordAction::ProtectionFrom(colors) => self.protection_from(colors),
            KeywordAction::ProtectionFromAllColors => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::AllColors),
            )),
            KeywordAction::ProtectionFromColorless => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::Colorless),
            )),
            KeywordAction::ProtectionFromCardType(card_type) => {
                self.protection_from_card_type(card_type)
            }
            KeywordAction::ProtectionFromSubtype(subtype) => self.protection_from_subtype(subtype),
            KeywordAction::Unblockable => self.unblockable(),
            KeywordAction::Marker(name) => self.with_ability(Ability::static_ability(
                StaticAbility::custom(name, name.to_string()),
            )),
        }
    }

    /// Build a CardDefinition from oracle text.
    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        let (definition, _) = self.parse_text_with_annotations(text)?;
        Ok(definition)
    }

    /// Build a CardDefinition from oracle text, returning parse annotations.
    pub fn parse_text_with_annotations(
        mut self,
        text: impl Into<String>,
    ) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
        let text = text.into();

        let mut oracle_lines = Vec::new();

        let card_name = self.card_builder.name_ref().to_string();
        let short_name = card_name
            .split(',')
            .next()
            .unwrap_or(card_name.as_str())
            .trim()
            .to_string();
        let full_lower = card_name.to_ascii_lowercase();
        let short_lower = short_name.to_ascii_lowercase();

        let mut annotations = ParseAnnotations::default();

        let mut line_infos = Vec::new();
        for (line_index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(meta) = parse_metadata_line(line)? {
                self = self.apply_metadata(meta)?;
                continue;
            }

            oracle_lines.push(line.to_string());
            let Some(normalized) =
                normalize_line_for_parse(line, full_lower.as_str(), short_lower.as_str())
            else {
                continue;
            };
            annotations.record_original_line(line_index, &normalized.original);
            annotations.record_normalized_line(line_index, &normalized.normalized);
            annotations.record_char_map(line_index, normalized.char_map.clone());
            line_infos.push(LineInfo {
                line_index,
                raw_line: line.to_string(),
                normalized,
            });
        }

        #[derive(Clone)]
        struct ModalHeader {
            min: u32,
            max: Option<u32>,
            commander_allows_both: bool,
            trigger: Option<TriggerSpec>,
            line_text: String,
        }

        struct PendingModal {
            header: ModalHeader,
            modes: Vec<EffectMode>,
        }

        let mut level_abilities: Vec<LevelAbility> = Vec::new();

        let is_bullet_line = |line: &str| {
            let trimmed = line.trim_start();
            trimmed.starts_with('•') || trimmed.starts_with('*') || trimmed.starts_with('-')
        };

        let parse_modal_header = |info: &LineInfo| -> Result<Option<ModalHeader>, CardTextError> {
            let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
            let words = words(&tokens);
            let Some(choose_idx) = tokens.iter().position(|token| token.is_word("choose")) else {
                return Ok(None);
            };

            let mut min = None;
            let mut max = None;
            let choose_tokens = &tokens[choose_idx + 1..];
            if choose_tokens.len() >= 3
                && choose_tokens[0].is_word("one")
                && choose_tokens[1].is_word("or")
                && choose_tokens[2].is_word("more")
            {
                min = Some(1);
                max = None;
            } else if choose_tokens.len() >= 3
                && choose_tokens[0].is_word("one")
                && choose_tokens[1].is_word("or")
                && choose_tokens[2].is_word("both")
            {
                min = Some(1);
                max = Some(2);
            } else if choose_tokens.len() >= 2
                && choose_tokens[0].is_word("up")
                && choose_tokens[1].is_word("to")
            {
                if let Some((value, _)) = parse_number(&choose_tokens[2..]) {
                    min = Some(0);
                    max = Some(value);
                }
            } else if let Some((value, _)) = parse_number(choose_tokens) {
                min = Some(value);
                max = Some(value);
            }

            let Some(min) = min else {
                return Ok(None);
            };

            let commander_allows_both = words.contains(&"commander") && words.contains(&"both");

            let mut trigger = None;
            if let Some(comma_idx) = tokens
                .iter()
                .position(|token| matches!(token, Token::Comma(_)))
            {
                if choose_idx > comma_idx {
                    let start_idx = if tokens.first().is_some_and(|token| {
                        token.is_word("whenever") || token.is_word("when") || token.is_word("at")
                    }) {
                        1
                    } else {
                        0
                    };
                    if comma_idx > start_idx {
                        let trigger_tokens = &tokens[start_idx..comma_idx];
                        if !trigger_tokens.is_empty() {
                            trigger = Some(parse_trigger_clause(trigger_tokens)?);
                        }
                    }
                }
            }

            Ok(Some(ModalHeader {
                min,
                max,
                commander_allows_both,
                trigger,
                line_text: info.raw_line.clone(),
            }))
        };

        let mut pending_modal: Option<PendingModal> = None;
        let mut idx = 0usize;
        while idx < line_infos.len() {
            let info = &line_infos[idx];
            if let Some((min_level, max_level)) = parse_level_header(&info.normalized.normalized) {
                let mut level = LevelAbility::new(min_level, max_level);
                idx += 1;
                while idx < line_infos.len() {
                    let next = &line_infos[idx];
                    if parse_level_header(&next.normalized.normalized).is_some() {
                        break;
                    }

                    let normalized_line = next.normalized.normalized.as_str();
                    if let Some(pt) = parse_power_toughness(normalized_line) {
                        if let (PtValue::Fixed(power), PtValue::Fixed(toughness)) =
                            (pt.power, pt.toughness)
                        {
                            level = level.with_pt(power, toughness);
                        }
                        idx += 1;
                        continue;
                    }

                    let tokens = tokenize_line(normalized_line, next.line_index);
                    if let Some(actions) = parse_ability_line(&tokens) {
                        for action in actions {
                            if let Some(ability) = keyword_action_to_static_ability(action) {
                                level.abilities.push(ability);
                            }
                        }
                        idx += 1;
                        continue;
                    }

                    if let Some(abilities) = parse_static_ability_line(&tokens)? {
                        level.abilities.extend(abilities);
                        idx += 1;
                        continue;
                    }

                    idx += 1;
                }

                level_abilities.push(level);
                continue;
            }

            if let Some(pending) = pending_modal.as_mut() {
                if is_bullet_line(&info.raw_line) {
                    let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
                    let effects_ast = parse_effect_sentences(&tokens)?;
                    if !effects_ast.is_empty() {
                        collect_tag_spans_from_effects_with_context(
                            &effects_ast,
                            &mut annotations,
                            &info.normalized,
                        );
                    }
                    let effects = compile_statement_effects(&effects_ast)?;
                    let description = info
                        .raw_line
                        .trim_start()
                        .trim_start_matches(|c: char| c == '•' || c == '*' || c == '-')
                        .trim()
                        .to_string();
                    pending.modes.push(EffectMode {
                        description,
                        effects,
                    });
                    idx += 1;
                    continue;
                }

                let pending = pending_modal.take().expect("pending modal");
                let modes = pending.modes;
                if !modes.is_empty() {
                    let mode_count = modes.len() as u32;
                    let max = pending.header.max.unwrap_or(mode_count).min(mode_count);
                    let min = pending.header.min.min(max);

                    let modal_effect = if pending.header.commander_allows_both {
                        let max_both = mode_count.min(2).max(1);
                        let choose_both = if max_both == 1 {
                            Effect::choose_one(modes.clone())
                        } else {
                            Effect::choose_up_to(max_both, 1, modes.clone())
                        };
                        let choose_one = Effect::choose_one(modes.clone());
                        Effect::conditional(
                            Condition::YouControlCommander,
                            vec![choose_both],
                            vec![choose_one],
                        )
                    } else if min == 1 && max == 1 {
                        Effect::choose_one(modes)
                    } else if min == max {
                        Effect::choose_exactly(max, modes)
                    } else {
                        Effect::choose_up_to(max, min, modes)
                    };

                    if let Some(trigger) = pending.header.trigger {
                        let compiled_trigger = compile_trigger_spec(trigger);
                        self = self.with_ability(Ability {
                            kind: AbilityKind::Triggered(TriggeredAbility {
                                trigger: compiled_trigger,
                                effects: vec![modal_effect],
                                choices: Vec::new(),
                                intervening_if: None,
                            }),
                            functional_zones: vec![Zone::Battlefield],
                            text: Some(pending.header.line_text),
                        });
                    } else if let Some(ref mut existing) = self.spell_effect {
                        existing.push(modal_effect);
                    } else {
                        self.spell_effect = Some(vec![modal_effect]);
                    }
                }
                continue;
            }

            let next_is_bullet = line_infos
                .get(idx + 1)
                .is_some_and(|next| is_bullet_line(&next.raw_line));
            if next_is_bullet {
                if let Some(header) = parse_modal_header(info)? {
                    pending_modal = Some(PendingModal {
                        header,
                        modes: Vec::new(),
                    });
                    idx += 1;
                    continue;
                }
            }

            let parsed = parse_line(&info.normalized.normalized, info.line_index)?;
            collect_tag_spans_from_line(&parsed, &mut annotations, &info.normalized);
            match parsed {
                LineAst::Abilities(actions) => {
                    for action in actions {
                        self = self.apply_keyword_action(action);
                    }
                }
                LineAst::StaticAbility(ability) => {
                    self = self.with_ability(
                        Ability::static_ability(ability).with_text(info.raw_line.as_str()),
                    );
                }
                LineAst::StaticAbilities(abilities) => {
                    for ability in abilities {
                        self = self.with_ability(
                            Ability::static_ability(ability).with_text(info.raw_line.as_str()),
                        );
                    }
                }
                LineAst::Ability(parsed_ability) => {
                    if let Some(ref effects_ast) = parsed_ability.effects_ast {
                        collect_tag_spans_from_effects_with_context(
                            effects_ast,
                            &mut annotations,
                            &info.normalized,
                        );
                    }
                    let ability = parsed_ability.ability;
                    let line_lower = info.raw_line.to_ascii_lowercase();
                    if let AbilityKind::Mana(mana_ability) = &ability.kind
                        && mana_ability.effects.is_none()
                        && mana_ability.mana.len() > 1
                        && line_lower.contains(" or ")
                    {
                        for symbol in &mana_ability.mana {
                            let mut split = ability.clone();
                            if let AbilityKind::Mana(ref mut inner) = split.kind {
                                inner.mana = vec![*symbol];
                            }
                            self = self.with_ability(split.with_text(info.raw_line.as_str()));
                        }
                        idx += 1;
                        continue;
                    }
                    self = self.with_ability(ability.with_text(info.raw_line.as_str()));
                }
                LineAst::Statement { effects } => {
                    if effects.is_empty() {
                        idx += 1;
                        continue;
                    }
                    if let Some(enchant_filter) = effects.iter().find_map(|effect| {
                        if let EffectAst::Enchant { filter } = effect {
                            Some(filter.clone())
                        } else {
                            None
                        }
                    }) {
                        self.aura_attach_filter = Some(enchant_filter);
                    }
                    let compiled = compile_statement_effects(&effects)?;
                    if let Some(ref mut existing) = self.spell_effect {
                        existing.extend(compiled);
                    } else {
                        self.spell_effect = Some(compiled);
                    }
                }
                LineAst::AdditionalCost { effects } => {
                    if effects.is_empty() {
                        idx += 1;
                        continue;
                    }
                    let compiled = compile_statement_effects(&effects)?;
                    self.cost_effects.extend(compiled);
                }
                LineAst::Triggered { trigger, effects } => {
                    let (compiled_effects, choices) =
                        compile_trigger_effects(Some(&trigger), &effects)?;
                    let compiled_trigger = compile_trigger_spec(trigger);
                    self = self.with_ability(Ability {
                        kind: AbilityKind::Triggered(TriggeredAbility {
                            trigger: compiled_trigger,
                            effects: compiled_effects,
                            choices,
                            intervening_if: None,
                        }),
                        functional_zones: vec![Zone::Battlefield],
                        text: Some(info.raw_line.clone()),
                    });
                }
            }
            idx += 1;
        }

        if let Some(pending) = pending_modal.take() {
            let modes = pending.modes;
            if !modes.is_empty() {
                let mode_count = modes.len() as u32;
                let max = pending.header.max.unwrap_or(mode_count).min(mode_count);
                let min = pending.header.min.min(max);
                let modal_effect = if pending.header.commander_allows_both {
                    let max_both = mode_count.min(2).max(1);
                    let choose_both = if max_both == 1 {
                        Effect::choose_one(modes.clone())
                    } else {
                        Effect::choose_up_to(max_both, 1, modes.clone())
                    };
                    let choose_one = Effect::choose_one(modes.clone());
                    Effect::conditional(
                        Condition::YouControlCommander,
                        vec![choose_both],
                        vec![choose_one],
                    )
                } else if min == 1 && max == 1 {
                    Effect::choose_one(modes)
                } else if min == max {
                    Effect::choose_exactly(max, modes)
                } else {
                    Effect::choose_up_to(max, min, modes)
                };

                if let Some(trigger) = pending.header.trigger {
                    let compiled_trigger = compile_trigger_spec(trigger);
                    self = self.with_ability(Ability {
                        kind: AbilityKind::Triggered(TriggeredAbility {
                            trigger: compiled_trigger,
                            effects: vec![modal_effect],
                            choices: Vec::new(),
                            intervening_if: None,
                        }),
                        functional_zones: vec![Zone::Battlefield],
                        text: Some(pending.header.line_text),
                    });
                } else if let Some(ref mut existing) = self.spell_effect {
                    existing.push(modal_effect);
                } else {
                    self.spell_effect = Some(vec![modal_effect]);
                }
            }
        }

        if !oracle_lines.is_empty() {
            self = self.oracle_text(oracle_lines.join("\n"));
        }
        if !level_abilities.is_empty() {
            self = self.with_level_abilities(level_abilities);
        }
        Ok((self.build(), annotations))
    }

    /// Build a CardDefinition from oracle text, prepending metadata lines
    /// derived from the builder's current fields (mana cost, type line, etc.).
    pub fn from_text_with_metadata(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        let rules = text.into();
        let combined = self.build_text_with_metadata(rules.as_str());

        let parse_builder = self.clone();
        let mut parse_builder = parse_builder;
        parse_builder.cost_effects.clear();
        match parse_builder.parse_text(combined.clone()) {
            Ok(def) => Ok(def),
            Err(err) => {
                let is_token_error = matches!(
                    &err,
                    CardTextError::ParseError(msg) if msg.to_ascii_lowercase().contains("token")
                );
                if is_token_error {
                    Err(err)
                } else {
                    Ok(self.oracle_text(combined).build())
                }
            }
        }
    }

    /// Backwards-compatible wrapper for prepending metadata to rules text.
    pub fn text_box(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        let rules = text.into();
        let combined = self.build_text_with_metadata(rules.as_str());

        // Treat the text box as authoritative: drop any previously added abilities if parsing succeeds.
        let mut parse_builder = self.clone();
        parse_builder.abilities.clear();
        parse_builder.cost_effects.clear();
        match parse_builder.parse_text(combined.clone()) {
            Ok(def) => Ok(def),
            Err(err) => {
                let is_token_error = matches!(
                    &err,
                    CardTextError::ParseError(msg) if msg.to_ascii_lowercase().contains("token")
                );
                if is_token_error {
                    Err(err)
                } else {
                    Ok(self.oracle_text(combined).build())
                }
            }
        }
    }

    /// Build a CardDefinition from oracle text with metadata, without parsing rules text.
    /// Useful for cards with custom/manual abilities where parsing may be incomplete.
    pub fn from_text_with_metadata_oracle_only(self, text: impl Into<String>) -> CardDefinition {
        fn pt_value_text(value: PtValue) -> String {
            match value {
                PtValue::Fixed(n) => n.to_string(),
                PtValue::Star => "*".to_string(),
                PtValue::StarPlus(n) => {
                    if n >= 0 {
                        format!("*+{n}")
                    } else {
                        format!("*{n}")
                    }
                }
            }
        }

        fn type_line_text(
            supertypes: &[Supertype],
            card_types: &[CardType],
            subtypes: &[Subtype],
        ) -> Option<String> {
            if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
                return None;
            }

            let mut left = Vec::new();
            for supertype in supertypes {
                left.push(format!("{:?}", supertype));
            }
            for card_type in card_types {
                left.push(format!("{:?}", card_type));
            }

            let mut line = left.join(" ");
            if !subtypes.is_empty() {
                let right = subtypes
                    .iter()
                    .map(|subtype| format!("{:?}", subtype))
                    .collect::<Vec<_>>()
                    .join(" ");
                if !line.is_empty() {
                    line.push_str(" — ");
                }
                line.push_str(&right);
            }
            Some(line)
        }

        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                pt_value_text(pt.power),
                pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        let rules = text.into();
        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        let combined = lines.join("\n");
        self.oracle_text(combined).build()
    }

    fn apply_metadata(mut self, meta: MetadataLine) -> Result<Self, CardTextError> {
        match meta {
            MetadataLine::ManaCost(raw) => {
                let cost = parse_scryfall_mana_cost(&raw)?;
                if !cost.is_empty() {
                    self.card_builder = self.card_builder.mana_cost(cost);
                }
            }
            MetadataLine::TypeLine(raw) => {
                let (supertypes, card_types, subtypes) = parse_type_line(&raw)?;
                if !supertypes.is_empty() {
                    self.card_builder = self.card_builder.supertypes(supertypes);
                }
                if !card_types.is_empty() {
                    self.card_builder = self.card_builder.card_types(card_types);
                }
                if !subtypes.is_empty() {
                    self.card_builder = self.card_builder.subtypes(subtypes);
                }
            }
            MetadataLine::PowerToughness(raw) => {
                if let Some(pt) = parse_power_toughness(&raw) {
                    self.card_builder = self.card_builder.power_toughness(pt);
                }
            }
            MetadataLine::Loyalty(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.loyalty(value);
                }
            }
            MetadataLine::Defense(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.defense(value);
                }
            }
        }

        Ok(self)
    }

    /// Set the power/toughness.
    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.card_builder = self.card_builder.power_toughness(pt);
        self
    }

    /// Set the starting loyalty.
    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.card_builder = self.card_builder.loyalty(loyalty);
        self
    }

    /// Set the defense value.
    pub fn defense(mut self, defense: u32) -> Self {
        self.card_builder = self.card_builder.defense(defense);
        self
    }

    /// Mark this card as a token.
    ///
    /// Tokens are not real cards - they are created by effects and cease to exist
    /// when they leave the battlefield.
    pub fn token(mut self) -> Self {
        self.card_builder = self.card_builder.token();
        self
    }

    // === Ability methods ===

    /// Add abilities to the card.
    pub fn with_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    /// Add a single ability to the card.
    pub fn with_ability(mut self, ability: Ability) -> Self {
        self.abilities.push(ability);
        self
    }

    // === Keyword shortcuts ===

    /// Add flying.
    pub fn flying(self) -> Self {
        self.with_ability(ability::flying())
    }

    /// Add first strike.
    pub fn first_strike(self) -> Self {
        self.with_ability(ability::first_strike())
    }

    /// Add double strike.
    pub fn double_strike(self) -> Self {
        self.with_ability(ability::double_strike())
    }

    /// Add deathtouch.
    pub fn deathtouch(self) -> Self {
        self.with_ability(ability::deathtouch())
    }

    /// Add lifelink.
    pub fn lifelink(self) -> Self {
        self.with_ability(ability::lifelink())
    }

    /// Add vigilance.
    pub fn vigilance(self) -> Self {
        self.with_ability(ability::vigilance())
    }

    /// Add trample.
    pub fn trample(self) -> Self {
        self.with_ability(ability::trample())
    }

    /// Add haste.
    pub fn haste(self) -> Self {
        self.with_ability(ability::haste())
    }

    /// Add reach.
    pub fn reach(self) -> Self {
        self.with_ability(ability::reach())
    }

    /// Add defender.
    pub fn defender(self) -> Self {
        self.with_ability(ability::defender())
    }

    /// Add hexproof.
    pub fn hexproof(self) -> Self {
        self.with_ability(ability::hexproof())
    }

    /// Add ward with a mana cost.
    ///
    /// Ward is a triggered ability that counters spells or abilities that target
    /// this permanent unless the opponent pays the ward cost.
    ///
    /// Example: `ward(TotalCost::mana("{3}"))` for "Ward {3}"
    pub fn ward(self, cost: TotalCost) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::ward(cost)).with_text("Ward"))
    }

    /// Add ward with a generic mana cost.
    ///
    /// Convenience method for the common case of ward with just generic mana.
    /// Example: `ward_generic(3)` for "Ward {3}"
    pub fn ward_generic(self, amount: u32) -> Self {
        use crate::mana::{ManaCost, ManaSymbol};
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(amount as u8)]]);
        self.ward(TotalCost::mana(mana))
    }

    /// Add indestructible.
    pub fn indestructible(self) -> Self {
        self.with_ability(ability::indestructible())
    }

    /// Add menace.
    pub fn menace(self) -> Self {
        self.with_ability(ability::menace())
    }

    /// Add flash.
    pub fn flash(self) -> Self {
        self.with_ability(ability::flash())
    }

    /// Add shroud.
    pub fn shroud(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shroud()).with_text("Shroud"))
    }

    /// Add wither.
    pub fn wither(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::wither()).with_text("Wither"))
    }

    /// Add infect.
    pub fn infect(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::infect()).with_text("Infect"))
    }

    /// Add undying.
    ///
    /// Undying is a triggered ability: "When this creature dies, if it had no +1/+1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a +1/+1 counter on it."
    pub fn undying(self) -> Self {
        use crate::effect::Effect;
        use crate::object::CounterType;
        use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose = Effect::choose_objects(filter, 1, PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::undying(),
                effects,
                choices: vec![],
                intervening_if: None,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Undying".to_string()),
        })
    }

    /// Add persist.
    ///
    /// Persist is a triggered ability: "When this creature dies, if it had no -1/-1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a -1/-1 counter on it."
    pub fn persist(self) -> Self {
        use crate::effect::Effect;
        use crate::object::CounterType;
        use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose = Effect::choose_objects(filter, 1, PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::persist(),
                effects,
                choices: vec![],
                intervening_if: None,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Persist".to_string()),
        })
    }

    /// Add prowess.
    ///
    /// Prowess means "Whenever you cast a noncreature spell, this creature gets +1/+1 until
    /// end of turn."
    pub fn prowess(self) -> Self {
        self.with_ability(
            Ability::triggered(
                Trigger::spell_cast(Some(ObjectFilter::noncreature_spell()), PlayerFilter::You),
                vec![Effect::pump(1, 1, ChooseSpec::Source, Until::EndOfTurn)],
            )
            .with_text("Prowess"),
        )
    }

    /// Add exalted.
    ///
    /// Exalted means "Whenever a creature you control attacks alone, that creature gets +1/+1
    /// until end of turn."
    pub fn exalted(self) -> Self {
        let attacker_tag = "exalted_attacker";
        self.with_ability(
            Ability::triggered(
                Trigger::attacks_alone(ObjectFilter::creature().you_control()),
                vec![
                    Effect::tag_triggering_object(attacker_tag),
                    Effect::pump(
                        1,
                        1,
                        ChooseSpec::Tagged(attacker_tag.into()),
                        Until::EndOfTurn,
                    ),
                ],
            )
            .with_text("Exalted"),
        )
    }

    /// Add toxic N.
    ///
    /// Toxic N means "Players dealt combat damage by this creature also get N poison counters."
    pub fn toxic(self, amount: u32) -> Self {
        let text = format!("Toxic {amount}");
        self.with_ability(
            Ability::triggered(
                Trigger::this_deals_combat_damage_to_player(),
                vec![Effect::poison_counters_player(
                    amount as i32,
                    PlayerFilter::DamagedPlayer,
                )],
            )
            .with_text(&text),
        )
    }

    /// Add storm.
    ///
    /// Storm means "When you cast this spell, copy it for each spell cast before it this turn."
    pub fn storm(self) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: vec![
                    Effect::with_id(
                        0,
                        Effect::copy_spell_n(
                            ChooseSpec::Source,
                            Value::SpellsCastBeforeThisTurn(PlayerFilter::You),
                        ),
                    ),
                    Effect::may_choose_new_targets(EffectId(0)),
                ],
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Stack],
            text: Some("Storm".to_string()),
        })
    }

    /// Add fear.
    pub fn fear(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::fear()).with_text("Fear"))
    }

    /// Add intimidate.
    pub fn intimidate(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::intimidate()).with_text("Intimidate"),
        )
    }

    /// Add shadow.
    pub fn shadow(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shadow()).with_text("Shadow"))
    }

    /// Add horsemanship.
    pub fn horsemanship(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::horsemanship()).with_text("Horsemanship"),
        )
    }

    /// Add bushido N.
    ///
    /// Bushido means "Whenever this creature blocks or becomes blocked, it gets +N/+N until
    /// end of turn."
    pub fn bushido(self, amount: u32) -> Self {
        use crate::effect::Until;
        let text = format!("Bushido {amount}");
        self.with_ability(
            Ability::triggered(
                Trigger::this_blocks_or_becomes_blocked(),
                vec![Effect::pump(
                    amount,
                    amount,
                    ChooseSpec::Source,
                    Until::EndOfTurn,
                )],
            )
            .with_text(&text),
        )
    }

    /// Add unblockable (can't be blocked).
    pub fn unblockable(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::unblockable())
                .with_text("This creature can't be blocked."),
        )
    }

    /// Add "may assign combat damage as though unblocked" (Thorn Elemental ability).
    pub fn may_assign_damage_as_unblocked(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::may_assign_damage_as_unblocked())
                .with_text("You may have ~ assign its combat damage as though it weren't blocked."),
        )
    }

    /// Add "shuffle into library from graveyard" (Darksteel Colossus ability).
    pub fn shuffle_into_library_from_graveyard(self) -> Self {
        use crate::zone::Zone;
        self.with_ability(
            Ability::static_ability(StaticAbility::shuffle_into_library_from_graveyard())
                .in_zones(vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Graveyard,
                    Zone::Exile,
                ])
                .with_text("If ~ would be put into a graveyard from anywhere, reveal it and shuffle it into its owner's library instead."),
        )
    }

    // === Cost Modifier Abilities ===

    /// Add affinity for artifacts (cost reduction based on artifacts you control).
    pub fn affinity_for_artifacts(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::affinity_for_artifacts())
                .with_text("Affinity for artifacts"),
        )
    }

    /// Add delve (exile cards from graveyard to pay generic mana).
    pub fn delve(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::delve()).with_text("Delve"))
    }

    /// Add convoke (tap creatures to help pay for this spell).
    pub fn convoke(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::convoke()).with_text("Convoke"))
    }

    /// Add improvise (tap artifacts to pay generic mana).
    pub fn improvise(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::improvise()).with_text("Improvise"),
        )
    }

    /// Add protection from a color.
    pub fn protection_from(self, colors: ColorSet) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(colors)))
                .with_text(&format!("Protection from {:?}", colors)),
        )
    }

    /// Add protection from a card type.
    pub fn protection_from_card_type(self, card_type: CardType) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::CardType(
                card_type,
            )))
            .with_text(&format!("Protection from {:?}s", card_type)),
        )
    }

    /// Add protection from a creature subtype (e.g., "Protection from Humans").
    pub fn protection_from_subtype(self, subtype: Subtype) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::Permanents(
                ObjectFilter::default().with_subtype(subtype),
            )))
            .with_text(&format!("Protection from {:?}", subtype)),
        )
    }

    // === Triggered ability shortcuts ===

    /// Add an enters-the-battlefield trigger.
    pub fn with_etb(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::etb_trigger(effects))
    }

    /// Add a dies trigger.
    pub fn with_dies_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::dies_trigger(effects))
    }

    /// Add an upkeep trigger.
    pub fn with_upkeep_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::upkeep_trigger(effects))
    }

    /// Add a custom triggered ability.
    pub fn with_trigger(self, trigger: crate::triggers::Trigger, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::triggered(trigger, effects))
    }

    /// Add a targeted ETB trigger (e.g., Snapcaster Mage).
    pub fn with_targeted_etb(
        self,
        target_spec: crate::target::ChooseSpec,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects,
                choices: vec![target_spec],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        };
        self.with_ability(ability)
    }

    /// Add an optional triggered ability ("you may").
    pub fn with_optional_trigger(
        self,
        trigger: crate::triggers::Trigger,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects,
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        };
        self.with_ability(ability)
    }

    // === Activated ability shortcuts ===

    /// Add an activated ability.
    pub fn with_activated(self, cost: TotalCost, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated(cost, effects))
    }

    /// Add a tap ability that does something.
    pub fn with_tap_ability(self, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated_with_cost_effects(
            TotalCost::free(),
            vec![Effect::tap_source()],
            effects,
        ))
    }

    // === Mana ability shortcuts ===

    /// Add a mana ability that taps for a single color.
    pub fn taps_for(self, mana: ManaSymbol) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), vec![mana]))
    }

    /// Add a mana ability that taps for multiple mana.
    pub fn taps_for_mana(self, mana: Vec<ManaSymbol>) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), mana))
    }

    // === Spell effect shortcuts ===

    /// Set the spell effects (for instants/sorceries).
    pub fn with_spell_effect(mut self, effects: Vec<Effect>) -> Self {
        self.spell_effect = Some(effects);
        self
    }

    // === Alternative Casting Methods ===

    /// Add flashback with the given cost.
    pub fn flashback(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Flashback { cost });
        self
    }

    /// Add jump-start (cast from graveyard, discard a card).
    pub fn jump_start(mut self) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::JumpStart);
        self
    }

    /// Add escape with the given cost and exile count.
    pub fn escape(mut self, cost: ManaCost, exile_count: u32) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Escape {
                cost: Some(cost),
                exile_count,
            });
        self
    }

    /// Add madness with the given cost.
    pub fn madness(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Madness { cost });
        self
    }

    /// Add miracle with the given cost.
    ///
    /// Miracle is both an alternative casting method and a triggered ability:
    /// "When you draw this card, if it's the first card you've drawn this turn,
    /// you may reveal it. If you do, you may cast it for its miracle cost."
    pub fn miracle(mut self, cost: ManaCost) -> Self {
        use crate::effect::Effect;
        use crate::triggers::Trigger;

        // Add the alternative casting method
        self.alternative_casts
            .push(AlternativeCastingMethod::Miracle { cost });

        // Add the miracle trigger
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::miracle(),
                effects: vec![Effect::may_cast_for_miracle_cost()],
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![crate::zone::Zone::Hand], // Only triggers from hand
            text: Some("Miracle".to_string()),
        })
    }

    /// Add a custom alternative casting method.
    pub fn alternative_cast(mut self, method: AlternativeCastingMethod) -> Self {
        self.alternative_casts.push(method);
        self
    }

    // === Optional Costs (Kicker, Buyback, etc.) ===

    /// Add a kicker cost (can pay once for additional effect).
    pub fn kicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::kicker(cost));
        self
    }

    /// Add a kicker cost using just mana.
    pub fn kicker_mana(self, cost: ManaCost) -> Self {
        self.kicker(TotalCost::mana(cost))
    }

    /// Add a multikicker cost (can pay any number of times).
    pub fn multikicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::multikicker(cost));
        self
    }

    /// Add a multikicker cost using just mana.
    pub fn multikicker_mana(self, cost: ManaCost) -> Self {
        self.multikicker(TotalCost::mana(cost))
    }

    /// Add a buyback cost (return spell to hand after resolution).
    pub fn buyback(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::buyback(cost));
        self
    }

    /// Add a buyback cost using just mana.
    pub fn buyback_mana(self, cost: ManaCost) -> Self {
        self.buyback(TotalCost::mana(cost))
    }

    /// Add an entwine cost (for modal spells, choose all modes).
    pub fn entwine(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::entwine(cost));
        self
    }

    /// Add an entwine cost using just mana.
    pub fn entwine_mana(self, cost: ManaCost) -> Self {
        self.entwine(TotalCost::mana(cost))
    }

    /// Add a custom optional cost.
    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.optional_costs.push(cost);
        self
    }

    /// Set cost effects (new unified model).
    ///
    /// Cost effects are executed as part of paying costs, with `EventCause::from_cost()`.
    /// This enables triggers like "Whenever a creature is sacrificed to pay a cost".
    pub fn cost_effects(mut self, effects: Vec<Effect>) -> Self {
        self.cost_effects = effects;
        self
    }

    // === Saga Support ===

    /// Configure this card as a saga with the given number of chapters.
    ///
    /// Sagas automatically gain a lore counter at the start of each precombat main phase.
    /// When a lore counter is added, any chapters at or below that number that haven't
    /// triggered yet will trigger.
    pub fn saga(mut self, max_chapters: u32) -> Self {
        self.max_saga_chapter = Some(max_chapters);
        self
    }

    /// Add a saga chapter ability that triggers on a single chapter.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapter(1, vec![Effect::sacrifice(ObjectFilter::creature(), 1)])  // Chapter I
    /// ```
    pub fn with_chapter(self, chapter: u32, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(vec![chapter]), effects)
    }

    /// Add a saga chapter ability that triggers on multiple chapters.
    ///
    /// Use this for "I, II" style abilities that trigger on multiple chapters.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapters(vec![1, 2], vec![Effect::draw(1)])  // Chapters I, II
    /// ```
    pub fn with_chapters(self, chapters: Vec<u32>, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(chapters), effects)
    }

    // === Level-Up Support ===

    /// Add a level-up activated ability.
    ///
    /// Level-up is an activated ability that can only be activated at sorcery speed.
    /// It adds a level counter to the creature.
    ///
    /// # Arguments
    /// * `cost` - The mana cost to level up
    ///
    /// # Example
    /// ```ignore
    /// .level_up(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
    /// ```
    pub fn level_up(self, cost: ManaCost) -> Self {
        use crate::ability::{AbilityKind, ActivatedAbility};
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(cost),
                effects: vec![Effect::put_counters_on_source(CounterType::Level, 1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some("Level up".to_string()),
        };
        self.with_ability(ability)
    }

    /// Add level-based abilities.
    ///
    /// Level abilities grant different P/T and abilities based on the number of
    /// level counters on the creature. Only one tier applies at a time.
    ///
    /// # Example
    /// ```ignore
    /// .with_level_abilities(vec![
    ///     LevelAbility::new(2, Some(6)).with_pt(3, 3).with_ability(StaticAbility::first_strike()),
    ///     LevelAbility::new(7, None).with_pt(4, 4).with_ability(StaticAbility::double_strike()),
    /// ])
    /// ```
    pub fn with_level_abilities(self, levels: Vec<LevelAbility>) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::with_level_abilities(levels),
        ))
    }

    // === Build ===

    /// Build the card definition.
    pub fn build(self) -> CardDefinition {
        CardDefinition {
            card: self.card_builder.build(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            optional_costs: self.optional_costs,
            max_saga_chapter: self.max_saga_chapter,
            cost_effects: self.cost_effects,
        }
    }
}

#[cfg(test)]
mod target_parse_tests {
    use super::*;

    #[test]
    fn parse_target_creature() {
        let tokens = tokenize_line("target creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse target creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_nonland_permanent() {
        let tokens = tokenize_line("target nonland permanent", 0);
        let target = parse_target_phrase(&tokens).expect("parse target nonland permanent");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::nonland_permanent());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_artifact_or_enchantment() {
        let tokens = tokenize_line("target artifact or enchantment", 0);
        let target = parse_target_phrase(&tokens).expect("parse target artifact or enchantment");
        match target {
            TargetAst::Object(filter, _, _) => {
                let expected =
                    ObjectFilter::any_of_types(&[CardType::Artifact, CardType::Enchantment]);
                assert_eq!(filter, expected);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_creature_you_control() {
        let tokens = tokenize_line("target creature you control", 0);
        let target = parse_target_phrase(&tokens).expect("parse target creature you control");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature().you_control());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_another_target_creature_you_control() {
        let tokens = tokenize_line("another target creature you control", 0);
        let target = parse_target_phrase(&tokens).expect("parse another target creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature().you_control().other());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_nonblack_creature() {
        let tokens = tokenize_line("target nonblack creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse target nonblack creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                let expected = ObjectFilter::creature().without_colors(ColorSet::BLACK);
                assert_eq!(filter, expected);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_on_it() {
        let tokens = tokenize_line("on it", 0);
        let target = parse_target_phrase(&tokens).expect("parse on it");
        match target {
            TargetAst::Tagged(tag, _) => {
                assert_eq!(tag.as_str(), IT_TAG);
            }
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter.tagged_constraints.len(), 1);
                let constraint = &filter.tagged_constraints[0];
                assert_eq!(constraint.tag.as_str(), IT_TAG);
                assert_eq!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_this_as_source() {
        let tokens = tokenize_line("this", 0);
        let target = parse_target_phrase(&tokens).expect("parse this");
        assert!(matches!(target, TargetAst::Source(_)));
    }

    #[test]
    fn parse_target_this_creature_as_source() {
        let tokens = tokenize_line("this creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse this creature");
        assert!(matches!(target, TargetAst::Source(_)));
    }

    #[test]
    fn parse_permanent_shares_card_type_with_it() {
        let tokens = tokenize_line("a permanent that shares a card type with it", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse shared card type filter");
        assert_eq!(filter.tagged_constraints.len(), 1);
        let constraint = &filter.tagged_constraints[0];
        assert_eq!(constraint.tag.as_str(), IT_TAG);
        assert_eq!(constraint.relation, TaggedOpbjectRelation::SharesCardType);
    }
}

#[cfg(test)]
mod effect_parse_tests {
    use super::*;
    use crate::effects::CantEffect;
    use crate::effects::{
        CounterEffect, CreateTokenCopyEffect, DiscardEffect, ExchangeControlEffect,
        ExileInsteadOfGraveyardEffect, GainControlEffect, GrantPlayFromGraveyardEffect,
        LookAtHandEffect, RemoveUpToAnyCountersEffect, ReturnFromGraveyardToBattlefieldEffect,
        ReturnToHandEffect, SetLifeTotalEffect, SkipDrawStepEffect, SkipTurnEffect, SurveilEffect,
        TransformEffect,
    };
    use crate::ids::CardId;

    #[test]
    fn parse_yawgmoths_will_from_text() {
        let text = "Until end of turn, you may play lands and cast spells from your graveyard.\n\
If a card would be put into your graveyard from anywhere this turn, exile that card instead.";
        let def = CardDefinitionBuilder::new(CardId::new(), "Yawgmoth's Will")
            .parse_text(text)
            .expect("parse yawgmoth's will");

        let effects = def.spell_effect.expect("spell effect");
        assert_eq!(effects.len(), 2);
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<GrantPlayFromGraveyardEffect>().is_some()),
            "should include play-from-graveyard effect"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ExileInsteadOfGraveyardEffect>().is_some()),
            "should include exile-instead replacement effect"
        );
    }

    #[test]
    fn parse_cant_gain_life_until_eot_from_text() {
        let text = "Until end of turn, players can't gain life.";
        let def = CardDefinitionBuilder::new(CardId::new(), "No Life")
            .parse_text(text)
            .expect("parse cant gain life");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<CantEffect>().is_some()),
            "should include cant effect"
        );
    }

    #[test]
    fn parse_return_to_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Unsummon")
            .parse_text("Return target creature to its owner's hand.")
            .expect("parse return to hand");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ReturnToHandEffect>().is_some()),
            "should include return-to-hand effect"
        );
    }

    #[test]
    fn parse_return_up_to_cards_from_graveyard_to_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Macabre Reconstruction")
            .parse_text("Return up to two target creature cards from your graveyard to your hand.")
            .expect("parse up-to return to hand");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ReturnToHandEffect>().is_some()),
            "should include return-to-hand effect"
        );
    }

    #[test]
    fn parse_return_to_battlefield_from_graveyard_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Reanimate Variant")
            .parse_text(
                "Return target creature card from your graveyard to the battlefield tapped.",
            )
            .expect("parse return to battlefield");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| e
                .downcast_ref::<ReturnFromGraveyardToBattlefieldEffect>()
                .is_some()),
            "should include return-to-battlefield effect"
        );
    }

    #[test]
    fn parse_exchange_control_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Switcheroo")
            .parse_text("Exchange control of two target creatures.")
            .expect("parse exchange control");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ExchangeControlEffect>().is_some()),
            "should include exchange control effect"
        );
    }

    #[test]
    fn parse_counter_spell_with_graveyard_reference_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Drown in the Loch Variant")
            .parse_text(
                "Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard.",
            )
            .expect("parse counter spell with graveyard reference");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<CounterEffect>().is_some()),
            "should include counter effect"
        );
    }

    #[test]
    fn parse_remove_typed_counter_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Power Conduit Variant")
            .parse_text("Remove a +1/+1 counter from target creature.")
            .expect("parse typed counter removal");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<RemoveUpToAnyCountersEffect>().is_some()
                    || format!("{e:?}").contains("RemoveUpToAnyCountersEffect")
            }),
            "should include remove counters effect"
        );
    }

    #[test]
    fn parse_create_token_copy_of_target_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Copy Variant")
            .parse_text("Create a token that's a copy of target artifact.")
            .expect("parse copy token from target");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<CreateTokenCopyEffect>().is_some()
                    || format!("{e:?}").contains("CreateTokenCopyEffect")
            }),
            "should include create-token-copy effect"
        );
    }

    #[test]
    fn parse_set_life_total_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Blessed Wind")
            .parse_text("Target player's life total becomes 20.")
            .expect("parse set life total");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SetLifeTotalEffect>().is_some()),
            "should include set life total effect"
        );
    }

    #[test]
    fn parse_discard_random_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Specter's Wail")
            .parse_text("Target player discards a card at random.")
            .expect("parse random discard");

        let effects = def.spell_effect.expect("spell effect");
        let discard = effects
            .iter()
            .find_map(|e| e.downcast_ref::<DiscardEffect>())
            .expect("should include discard effect");
        assert!(discard.random, "discard should be random");
    }

    #[test]
    fn parse_skip_turn_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Meditate")
            .parse_text("You skip your next turn.")
            .expect("parse skip turn");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SkipTurnEffect>().is_some()),
            "should include skip turn effect"
        );
    }

    #[test]
    fn parse_skip_draw_step_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Fatigue")
            .parse_text("Target player skips their next draw step.")
            .expect("parse skip draw step");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SkipDrawStepEffect>().is_some()),
            "should include skip draw step effect"
        );
    }

    #[test]
    fn parse_gain_control_target_creature_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Threaten")
            .parse_text("Gain control of target creature until end of turn.")
            .expect("parse gain control");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<GainControlEffect>().is_some()
                    || format!("{e:?}").contains("GainControlEffect")
            }),
            "should include gain control effect"
        );
    }

    #[test]
    fn parse_reveal_targets_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Peek Variant")
            .parse_text("Target player reveals their hand.")
            .expect("parse reveal hand");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtHandEffect>().is_some()),
            "should include look-at-hand effect"
        );
    }

    #[test]
    fn parse_surveil_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Surveil Card")
            .parse_text("Surveil 1.")
            .expect("parse surveil");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SurveilEffect>().is_some()),
            "should include surveil effect"
        );
    }

    #[test]
    fn parse_transform_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Werewolf Shift")
            .parse_text("Transform this creature.")
            .expect("parse transform");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<TransformEffect>().is_some()),
            "should include transform effect"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;
    use crate::static_abilities::StaticAbilityId;
    use crate::target::ChooseSpec;

    #[test]
    fn test_creature_with_keywords() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .power_toughness(PowerToughness::fixed(3, 3))
            .flying()
            .vigilance()
            .build();

        assert_eq!(def.name(), "Test Creature");
        assert!(def.is_creature());
        assert_eq!(def.abilities.len(), 2);
    }

    #[test]
    fn test_creature_with_mana_ability() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mana Dork")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf, Subtype::Druid])
            .power_toughness(PowerToughness::fixed(1, 1))
            .taps_for(ManaSymbol::Green)
            .build();

        assert!(def.is_creature());
        assert_eq!(def.abilities.len(), 1);
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_spell_with_effects() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::deal_damage(3, ChooseSpec::AnyTarget)])
            .build();

        assert!(def.is_spell());
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_creature_with_etb() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "ETB Creature")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_etb(vec![Effect::draw(1)])
            .build();

        assert_eq!(def.abilities.len(), 1);
        let ability = &def.abilities[0];
        // Check that the trigger is an ETB trigger (now using Trigger struct)
        if let AbilityKind::Triggered(t) = &ability.kind {
            assert!(t.trigger.display().contains("enters"));
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_protection_from_color() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Protected")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .protection_from(ColorSet::from(Color::Red))
            .build();

        assert_eq!(def.abilities.len(), 1);
    }

    #[test]
    fn test_land_with_mana_ability() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Forest")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .taps_for(ManaSymbol::Green)
            .build();

        assert!(def.card.is_land());
        assert_eq!(def.abilities.len(), 1);
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_complex_creature() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Complex Creature")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Vampire])
            .power_toughness(PowerToughness::fixed(2, 3))
            .flying()
            .deathtouch()
            .lifelink()
            .build();

        assert_eq!(def.abilities.len(), 3);
        assert!(def.is_creature());
    }

    #[test]
    fn test_parse_cant_gain_life_from_text() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Life")
            .parse_text("Players can't gain life.")
            .expect("parse players can't gain life");

        let has_cant_gain = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::PlayersCantGainLife
            )
        });

        assert!(has_cant_gain);
    }

    #[test]
    fn test_parse_uncounterable_from_text() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Counter")
            .parse_text("This spell can't be countered.")
            .expect("parse this spell can't be countered");

        let has_uncounterable = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::CantBeCountered
            )
        });

        assert!(has_uncounterable);
    }

    #[test]
    fn test_parse_double_cant_clause_from_text() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Win")
            .parse_text("You can't lose the game and your opponents can't win the game.")
            .expect("parse dual can't clause");

        let has_cant_lose = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::YouCantLoseGame
            )
        });
        let has_cant_win = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::OpponentsCantWinGame
            )
        });

        assert!(has_cant_lose);
        assert!(has_cant_win);
    }

    #[test]
    fn test_parse_keyword_action_trigger_you_earthbend() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Earthbend Watcher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .parse_text("Whenever you earthbend, draw a card.")
            .expect("parse keyword action trigger");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected triggered ability");

        assert_eq!(triggered.trigger.display(), "Whenever you earthbend");
    }

    #[test]
    fn test_parse_keyword_action_trigger_any_player() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Investigation Watcher")
            .card_types(vec![CardType::Enchantment])
            .parse_text("Whenever a player investigates, draw a card.")
            .expect("parse keyword action trigger");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected triggered ability");

        assert_eq!(
            triggered.trigger.display(),
            "Whenever a player investigates"
        );
    }

    #[test]
    fn test_parse_keyword_action_trigger_players_finish_voting() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vote Watcher")
            .card_types(vec![CardType::Enchantment])
            .parse_text("Whenever players finish voting, draw a card.")
            .expect("parse keyword action trigger");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected triggered ability");

        assert_eq!(
            triggered.trigger.display(),
            "Whenever players finish voting"
        );
    }

    #[test]
    fn test_parse_prowess_keyword_line() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prowess Creature")
            .card_types(vec![CardType::Creature])
            .parse_text("Prowess")
            .expect("parse prowess keyword");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected prowess triggered ability");

        assert_eq!(def.abilities.len(), 1);
        assert_eq!(def.abilities[0].text.as_deref(), Some("Prowess"));
        assert!(triggered.trigger.display().contains("you cast"));
        assert!(
            format!("{:?}", triggered.effects[0]).contains("ModifyPowerToughnessEffect"),
            "expected pump effect, got {:?}",
            triggered.effects
        );
    }

    #[test]
    fn test_parse_bushido_keyword_line() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Samurai")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Bushido 1 (Whenever this creature blocks or becomes blocked, it gets +1/+1 until end of turn.)",
            )
            .expect("parse bushido keyword");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected bushido triggered ability");

        assert_eq!(def.abilities.len(), 1);
        assert_eq!(def.abilities[0].text.as_deref(), Some("Bushido 1"));
        assert!(triggered.trigger.display().contains("blocks"));
        assert!(
            format!("{:?}", triggered.effects[0]).contains("ModifyPowerToughnessEffect"),
            "expected pump effect, got {:?}",
            triggered.effects
        );
    }

    #[test]
    fn test_parse_exalted_keyword_line() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exalted Creature")
            .card_types(vec![CardType::Creature])
            .parse_text("Exalted")
            .expect("parse exalted keyword");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected exalted triggered ability");

        assert_eq!(def.abilities[0].text.as_deref(), Some("Exalted"));
        assert!(triggered.trigger.display().contains("attacks alone"));
        assert_eq!(triggered.effects.len(), 2);
    }

    #[test]
    fn test_parse_toxic_keyword_line() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Toxic Creature")
            .card_types(vec![CardType::Creature])
            .parse_text("Toxic 2")
            .expect("parse toxic keyword");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected toxic triggered ability");

        assert_eq!(def.abilities[0].text.as_deref(), Some("Toxic 2"));
        assert!(
            triggered
                .trigger
                .display()
                .contains("combat damage to a player")
        );
        assert!(
            format!("{:?}", triggered.effects[0]).contains("PoisonCountersEffect"),
            "expected poison counters effect, got {:?}",
            triggered.effects
        );
    }

    #[test]
    fn test_parse_storm_keyword_line() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Storm Spell")
            .card_types(vec![CardType::Instant])
            .parse_text("Storm")
            .expect("parse storm keyword");

        let triggered = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .expect("expected storm triggered ability");

        assert_eq!(def.abilities[0].text.as_deref(), Some("Storm"));
        assert_eq!(def.abilities[0].functional_zones, vec![Zone::Stack]);
        assert!(triggered.trigger.display().contains("cast this spell"));
        assert!(
            format!("{:?}", triggered.effects[0]).contains("CopySpellEffect"),
            "expected copy spell effect wrapper, got {:?}",
            triggered.effects
        );
        assert!(
            format!("{:?}", triggered.effects[1]).contains("ChooseNewTargetsEffect"),
            "expected choose-new-targets effect, got {:?}",
            triggered.effects
        );
    }

    #[test]
    fn test_parse_trigger_without_comma() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Comma Trigger")
            .card_types(vec![CardType::Enchantment])
            .parse_text("At the beginning of the next end step draw a card.")
            .expect("parse trigger without comma");

        let has_triggered = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(has_triggered, "expected triggered ability");
    }
}
