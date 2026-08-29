use std::ops::Range;

use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::mana::ManaCost;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::{leaf, primitives};

#[path = "keyword_action_costs/semantic_shapes.rs"]
mod semantic_shapes;
pub use semantic_shapes::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordUntapRestriction {
    Bare,
    DuringStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaymentAlternativeSplit {
    pub delimiter: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordPaymentLead {
    pub payload_first: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordDynamicPaymentShape {
    Energy {
        value: Range<usize>,
    },
    ManaAmountEqual,
    Mana {
        cost: ManaCost,
        trailing_first: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordDynamicManaTail {
    Life { value: Option<Range<usize>> },
    WhereX { same_name_in_graveyard: bool },
    ForEach,
    Modifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordAbilityHead {
    Other,
    CumulativeUpkeep { cost: Range<usize> },
    Crew,
    Saddle,
    AuraSwap,
    EmergeFrom,
    JobSelect,
    UmbraArmor,
    Echo { cost: Range<usize> },
    ProtectionFrom,
    Toxic,
    FirstStrike,
    DoubleStrike,
    Modular { sunburst: bool },
    ForMirrodin,
    LivingWeapon,
    BattleCry,
    SplitSecond,
    ReadAhead,
    DoctorCompanion,
    Fuse,
    Bolster { amount: Option<u32> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordAbilitySurface {
    pub phrase_first: usize,
    pub word_count: usize,
    pub head: KeywordAbilityHead,
    pub sorcery_speed_reminder: bool,
    pub once_per_turn_reminder: bool,
    pub conjoined: bool,
    pub unblockable_tail: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordTriggerObjectHead {
    CardType(CardType),
    Subtype(Subtype),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicSoulshiftShape {
    pub count_filter: ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialAbilityPhraseKind {
    VariableCasualtyPlaneswalkerCopy,
    StartYourEngines,
    AnyLandwalk,
    NonbasicLandwalk,
    ArtifactLandwalk,
}

pub fn parse_dynamic_soulshift_words(words: &[&str]) -> Option<DynamicSoulshiftShape> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_dynamic_soulshift_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(DynamicSoulshiftShape {
        count_filter: ObjectFilter::default()
            .with_subtype(Subtype::Spirit)
            .you_control()
            .in_zone(Zone::Battlefield),
    })
}

pub fn parse_dynamic_soulshift_tokens(tokens: &[OwnedLexToken]) -> Option<DynamicSoulshiftShape> {
    primitives::parse_prefix(tokens, parse_dynamic_soulshift_lexed)?;
    Some(DynamicSoulshiftShape {
        count_filter: ObjectFilter::default()
            .with_subtype(Subtype::Spirit)
            .you_control()
            .in_zone(Zone::Battlefield),
    })
}

pub fn parse_special_ability_phrase_words(words: &[&str]) -> Option<SpecialAbilityPhraseKind> {
    let mut variable: primitives::WordSliceInput<'_> = words;
    if parse_variable_casualty_planeswalker_copy_words
        .parse_next(&mut variable)
        .is_ok()
    {
        return Some(SpecialAbilityPhraseKind::VariableCasualtyPlaneswalkerCopy);
    }
    primitives::parse_full_word_slice(words, parse_exact_special_ability_phrase_words)
}

fn parse_dynamic_soulshift_word_slice(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("soulshift"),
        primitives::word_slice_exact("x"),
        primitives::word_slice_exact("where"),
        primitives::word_slice_exact("x"),
        primitives::word_slice_exact("is"),
        primitives::word_slice_exact("the"),
        primitives::word_slice_exact("number"),
        primitives::word_slice_exact("of"),
        alt((
            primitives::word_slice_exact("spirit"),
            primitives::word_slice_exact("spirits"),
        )),
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .void()
        .parse_next(input)
}

fn parse_dynamic_soulshift_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["soulshift", "x"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["where", "x", "is", "the", "number", "of"]).parse_next(input)?;
    alt((primitives::kw("spirit"), primitives::kw("spirits"))).parse_next(input)?;
    primitives::phrase(&["you", "control"])
        .void()
        .parse_next(input)
}

fn parse_variable_casualty_planeswalker_copy_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<()> {
    (
        primitives::word_slice_exact("casualty"),
        primitives::word_slice_exact("x"),
        primitives::word_slice_exact("the"),
        primitives::word_slice_exact("copy"),
        primitives::word_slice_exact("isnt"),
        primitives::word_slice_exact("legendary"),
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("has"),
        primitives::word_slice_exact("starting"),
        primitives::word_slice_exact("loyalty"),
        primitives::word_slice_exact("x"),
    )
        .void()
        .parse_next(input)
}

fn parse_exact_special_ability_phrase_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SpecialAbilityPhraseKind> {
    alt((
        (
            primitives::word_slice_exact("start"),
            primitives::word_slice_exact("your"),
            primitives::word_slice_exact("engines"),
        )
            .value(SpecialAbilityPhraseKind::StartYourEngines),
        primitives::word_slice_exact("landwalk").value(SpecialAbilityPhraseKind::AnyLandwalk),
        (
            primitives::word_slice_exact("nonbasic"),
            primitives::word_slice_exact("landwalk"),
        )
            .value(SpecialAbilityPhraseKind::NonbasicLandwalk),
        (
            primitives::word_slice_exact("artifact"),
            primitives::word_slice_exact("landwalk"),
        )
            .value(SpecialAbilityPhraseKind::ArtifactLandwalk),
    ))
    .parse_next(input)
}

pub fn parse_keyword_untap_restriction_words(words: &[&str]) -> Option<KeywordUntapRestriction> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_keyword_untap_restriction_lexed_words
        .parse_next(&mut input)
        .ok()
        .filter(|_| input.is_empty())
}

pub fn parse_payment_alternative_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PaymentAlternativeSplit> {
    primitives::parse_prefix(tokens, parse_payment_alternative_split_lexed).map(|(split, _)| split)
}

pub fn parse_keyword_payment_lead_tokens(tokens: &[OwnedLexToken]) -> KeywordPaymentLead {
    let payload_first = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("pay").value(()),
            primitives::kw("pays").value(()),
        )),
    )
    .map(|(_, rest)| tokens.len().saturating_sub(rest.len()))
    .unwrap_or(0);
    KeywordPaymentLead { payload_first }
}

pub fn parse_keyword_dynamic_payment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordDynamicPaymentShape> {
    // A bare numeric token remains accepted as legacy generic mana elsewhere,
    // but in `pay 3 life` the following life noun proves that the number is a
    // life amount. Leave that complete cost to the activation-cost grammar
    // instead of misclassifying `3` as a dynamic mana prefix.
    if primitives::parse_prefix(
        tokens,
        (
            leaf::parse_leaf_number_prefix_lexed,
            alt((primitives::kw("life"), primitives::kw("lives"))),
        )
            .void(),
    )
    .is_some()
    {
        return None;
    }

    if let Some((value_first, rest)) =
        primitives::parse_prefix(tokens, parse_dynamic_energy_payment_lexed)
    {
        if rest.is_empty() {
            return None;
        }
        return Some(KeywordDynamicPaymentShape::Energy {
            value: value_first..tokens.len(),
        });
    }

    if primitives::parse_prefix(tokens, primitives::kw("mana")).is_some() {
        return Some(KeywordDynamicPaymentShape::ManaAmountEqual);
    }

    let (prefix, rest) = primitives::parse_prefix(tokens, leaf::parse_leaf_mana_cost_prefix_lexed)?;
    if rest.is_empty() {
        return None;
    }
    Some(KeywordDynamicPaymentShape::Mana {
        cost: prefix.cost,
        trailing_first: tokens.len().saturating_sub(rest.len()),
    })
}

pub fn parse_keyword_dynamic_mana_tail_tokens(tokens: &[OwnedLexToken]) -> KeywordDynamicManaTail {
    if let Some(value) = parse_keyword_dynamic_life_tail_tokens(tokens) {
        return KeywordDynamicManaTail::Life { value: Some(value) };
    }
    if primitives::parse_prefix(tokens, primitives::kw("and")).is_some() {
        return KeywordDynamicManaTail::Life { value: None };
    }

    if primitives::parse_prefix(tokens, parse_where_x_prefix_lexed).is_some() {
        let (same_name, graveyard) = primitives::parse_all(
            tokens,
            parse_where_x_reference_facts_lexed,
            "keyword-where-x-reference",
        )
        .unwrap_or((false, false));
        return KeywordDynamicManaTail::WhereX {
            same_name_in_graveyard: same_name && graveyard,
        };
    }

    if primitives::parse_prefix(tokens, parse_for_each_prefix_lexed).is_some() {
        return KeywordDynamicManaTail::ForEach;
    }

    KeywordDynamicManaTail::Modifier
}

pub fn parse_keyword_ability_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordAbilitySurface> {
    let (phrase_first, phrase_tokens) =
        if let Some(((), rest)) = primitives::parse_prefix(tokens, primitives::kw("and").void()) {
            (tokens.len().saturating_sub(rest.len()), rest)
        } else {
            (0, tokens)
        };

    let facts = primitives::parse_all(
        phrase_tokens,
        parse_keyword_ability_facts_lexed,
        "keyword-ability-facts",
    )
    .ok()?;
    if facts.word_count == 0 {
        return None;
    }

    let (tag, head_rest) = primitives::parse_prefix(phrase_tokens, parse_keyword_head_lexed)
        .unwrap_or((KeywordHeadTag::Other, phrase_tokens));
    let head_end = tokens.len().saturating_sub(head_rest.len());
    let head = match tag {
        KeywordHeadTag::Other => KeywordAbilityHead::Other,
        KeywordHeadTag::CumulativeUpkeep => {
            let relative_end = parse_cost_boundary(head_rest, CostBoundaryKind::Period);
            KeywordAbilityHead::CumulativeUpkeep {
                cost: head_end..head_end + relative_end,
            }
        }
        KeywordHeadTag::Crew => KeywordAbilityHead::Crew,
        KeywordHeadTag::Saddle => KeywordAbilityHead::Saddle,
        KeywordHeadTag::AuraSwap => KeywordAbilityHead::AuraSwap,
        KeywordHeadTag::EmergeFrom => KeywordAbilityHead::EmergeFrom,
        KeywordHeadTag::JobSelect => KeywordAbilityHead::JobSelect,
        KeywordHeadTag::UmbraArmor => KeywordAbilityHead::UmbraArmor,
        KeywordHeadTag::Echo => {
            let relative_end = parse_cost_boundary(head_rest, CostBoundaryKind::EchoReminder);
            KeywordAbilityHead::Echo {
                cost: head_end..head_end + relative_end,
            }
        }
        KeywordHeadTag::ProtectionFrom => KeywordAbilityHead::ProtectionFrom,
        KeywordHeadTag::Toxic => KeywordAbilityHead::Toxic,
        KeywordHeadTag::FirstStrike => KeywordAbilityHead::FirstStrike,
        KeywordHeadTag::DoubleStrike => KeywordAbilityHead::DoubleStrike,
        KeywordHeadTag::ModularSunburst => KeywordAbilityHead::Modular { sunburst: true },
        KeywordHeadTag::Modular => KeywordAbilityHead::Modular { sunburst: false },
        KeywordHeadTag::ForMirrodin => KeywordAbilityHead::ForMirrodin,
        KeywordHeadTag::LivingWeapon => KeywordAbilityHead::LivingWeapon,
        KeywordHeadTag::BattleCry => KeywordAbilityHead::BattleCry,
        KeywordHeadTag::SplitSecond => KeywordAbilityHead::SplitSecond,
        KeywordHeadTag::ReadAhead => KeywordAbilityHead::ReadAhead,
        KeywordHeadTag::DoctorCompanion => KeywordAbilityHead::DoctorCompanion,
        KeywordHeadTag::Fuse => KeywordAbilityHead::Fuse,
        KeywordHeadTag::Bolster => KeywordAbilityHead::Bolster {
            amount: leaf::parse_leaf_number_prefix_tokens(head_rest)
                .and_then(leaf::LeafNumberPrefix::into_fixed)
                .map(|(amount, _)| amount),
        },
    };

    Some(KeywordAbilitySurface {
        phrase_first,
        word_count: facts.word_count,
        head,
        sorcery_speed_reminder: facts.sorcery_speed_reminder,
        once_per_turn_reminder: facts.once_per_turn_reminder,
        conjoined: facts.conjoined,
        unblockable_tail: facts.unblockable_tail,
    })
}

pub fn parse_keyword_trigger_object_head(word: &str) -> Option<KeywordTriggerObjectHead> {
    if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
        return Some(KeywordTriggerObjectHead::CardType(card_type));
    }
    leaf::parse_leaf_subtype_flexible_complete(word)
        .ok()
        .map(KeywordTriggerObjectHead::Subtype)
}

fn parse_keyword_untap_restriction_lexed_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<KeywordUntapRestriction> {
    let first: &str = any.parse_next(input)?;
    if !matches!(first, "untap" | "untaps") {
        return Err(primitives::backtrack_err(
            "untap restriction",
            "untap or untaps",
        ));
    }
    if input.is_empty() {
        return Ok(KeywordUntapRestriction::Bare);
    }

    let mut saw_during = false;
    let mut saw_step = false;
    while !input.is_empty() {
        let word: &str = any.parse_next(input)?;
        if !matches!(
            word,
            "untap"
                | "untaps"
                | "during"
                | "its"
                | "their"
                | "your"
                | "controllers"
                | "controller"
                | "step"
                | "steps"
                | "next"
                | "the"
        ) {
            return Err(primitives::backtrack_err(
                "untap restriction",
                "supported untap-step restriction word",
            ));
        }
        saw_during |= word == "during";
        saw_step |= matches!(word, "step" | "steps");
    }
    if saw_during && saw_step {
        Ok(KeywordUntapRestriction::DuringStep)
    } else {
        Err(primitives::backtrack_err(
            "untap restriction",
            "during an untap step",
        ))
    }
}

fn parse_payment_alternative_split_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PaymentAlternativeSplit> {
    let initial_len = input.len();
    let mut previous_word = None;
    while let Some(token) = input.peek_token() {
        if token.is_word("or") {
            let next_word = input.get(1).and_then(OwnedLexToken::as_word);
            let comparison_tail = next_word
                .is_some_and(|word| matches!(word, "less" | "greater" | "more" | "fewer"))
                || (previous_word == Some("than") && next_word == Some("equal"));
            if !comparison_tail {
                let delimiter = initial_len.saturating_sub(input.len());
                primitives::kw("or").parse_next(input)?;
                return Ok(PaymentAlternativeSplit { delimiter });
            }
        }
        let consumed: &'a OwnedLexToken = any.parse_next(input)?;
        if let Some(word) = consumed.as_word() {
            previous_word = Some(word);
        }
    }
    Err(primitives::backtrack_err(
        "payment alternative",
        "non-comparison or delimiter",
    ))
}

fn parse_dynamic_energy_payment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    )))
    .parse_next(input)?;
    primitives::phrase(&["amount", "of"]).parse_next(input)?;
    any.verify(|token: &&OwnedLexToken| {
        token.kind == TokenKind::ManaGroup && token.parser_text() == "{e}"
    })
    .parse_next(input)?;
    primitives::phrase(&["equal", "to"]).parse_next(input)?;
    Ok(initial_len.saturating_sub(input.len()))
}

fn parse_where_x_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["where", "x", "is"]),
        primitives::phrase(&["where", "x", "equals"]),
    ))
    .parse_next(input)
}

fn parse_for_each_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["for", "each"]),
        primitives::kw("each").void(),
    ))
    .parse_next(input)
}

fn parse_where_x_reference_facts_lexed<'a>(input: &mut LexStream<'a>) -> WResult<(bool, bool)> {
    let mut same_name = false;
    let mut graveyard = false;
    while !input.is_empty() {
        let mut probe = input.clone();
        if alt((
            primitives::phrase(&["same", "name", "as", "the", "spell"]),
            primitives::phrase(&["same", "name", "as", "that", "spell"]),
        ))
        .parse_next(&mut probe)
        .is_ok()
        {
            *input = probe;
            same_name = true;
            continue;
        }
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token
            .as_word()
            .is_some_and(|word| matches!(word, "graveyard" | "graveyards"))
        {
            graveyard = true;
        }
    }
    Ok((same_name, graveyard))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeywordHeadTag {
    Other,
    CumulativeUpkeep,
    Crew,
    Saddle,
    AuraSwap,
    EmergeFrom,
    JobSelect,
    UmbraArmor,
    Echo,
    ProtectionFrom,
    Toxic,
    FirstStrike,
    DoubleStrike,
    ModularSunburst,
    Modular,
    ForMirrodin,
    LivingWeapon,
    BattleCry,
    SplitSecond,
    ReadAhead,
    DoctorCompanion,
    Fuse,
    Bolster,
}

fn parse_keyword_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<KeywordHeadTag> {
    alt((
        alt((
            primitives::phrase(&["cumulative", "upkeep"]).value(KeywordHeadTag::CumulativeUpkeep),
            primitives::phrase(&["aura", "swap"]).value(KeywordHeadTag::AuraSwap),
            primitives::phrase(&["emerge", "from"]).value(KeywordHeadTag::EmergeFrom),
            primitives::phrase(&["job", "select"]).value(KeywordHeadTag::JobSelect),
            primitives::phrase(&["umbra", "armor"]).value(KeywordHeadTag::UmbraArmor),
            primitives::phrase(&["protection", "from"]).value(KeywordHeadTag::ProtectionFrom),
        )),
        alt((
            alt((
                primitives::phrase(&["first", "strike"]).value(KeywordHeadTag::FirstStrike),
                primitives::phrase(&["double", "strike"]).value(KeywordHeadTag::DoubleStrike),
                primitives::phrase(&["modular", "sunburst"]).value(KeywordHeadTag::ModularSunburst),
                primitives::phrase(&["for", "mirrodin"]).value(KeywordHeadTag::ForMirrodin),
                primitives::phrase(&["living", "weapon"]).value(KeywordHeadTag::LivingWeapon),
                primitives::phrase(&["battle", "cry"]).value(KeywordHeadTag::BattleCry),
                primitives::phrase(&["split", "second"]).value(KeywordHeadTag::SplitSecond),
                primitives::phrase(&["read", "ahead"]).value(KeywordHeadTag::ReadAhead),
                primitives::phrase(&["doctor", "companion"]).value(KeywordHeadTag::DoctorCompanion),
            )),
            alt((
                primitives::kw("crew").value(KeywordHeadTag::Crew),
                primitives::kw("saddle").value(KeywordHeadTag::Saddle),
                primitives::kw("echo").value(KeywordHeadTag::Echo),
                primitives::kw("toxic").value(KeywordHeadTag::Toxic),
                primitives::kw("modular").value(KeywordHeadTag::Modular),
                primitives::kw("fuse").value(KeywordHeadTag::Fuse),
                primitives::kw("bolster").value(KeywordHeadTag::Bolster),
            )),
        )),
    ))
    .parse_next(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CostBoundaryKind {
    Period,
    EchoReminder,
}

fn parse_cost_boundary(tokens: &[OwnedLexToken], kind: CostBoundaryKind) -> usize {
    primitives::parse_prefix(tokens, move |input: &mut LexStream<'_>| {
        let initial_len = input.len();
        while let Some(token) = input.peek_token() {
            let boundary = token.kind == TokenKind::Period
                || (kind == CostBoundaryKind::EchoReminder
                    && (token.kind == TokenKind::LParen || token.is_word("at")));
            if boundary {
                break;
            }
            any.parse_next(input)?;
        }
        Ok(initial_len.saturating_sub(input.len()))
    })
    .map(|(length, _)| length)
    .unwrap_or(tokens.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeywordAbilityFacts {
    word_count: usize,
    sorcery_speed_reminder: bool,
    once_per_turn_reminder: bool,
    conjoined: bool,
    unblockable_tail: bool,
}

fn parse_keyword_ability_facts_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordAbilityFacts> {
    let mut word_count = 0usize;
    let mut sorcery_speed_reminder = false;
    let mut once_per_turn_reminder = false;
    let mut conjoined = false;
    let mut unblockable_tail = false;

    while !input.is_empty() {
        let mut probe = input.clone();
        if primitives::phrase(&["activate", "only", "as", "a", "sorcery"])
            .parse_next(&mut probe)
            .is_ok()
        {
            *input = probe;
            word_count += 5;
            sorcery_speed_reminder = true;
            continue;
        }

        let mut probe = input.clone();
        if alt((
            primitives::phrase(&["activate", "only", "once", "each", "turn"]),
            primitives::phrase(&["activate", "only", "once", "per", "turn"]),
        ))
        .parse_next(&mut probe)
        .is_ok()
        {
            *input = probe;
            word_count += 5;
            once_per_turn_reminder = true;
            continue;
        }

        let mut probe = input.clone();
        if alt((
            primitives::phrase(&["cant", "be", "blocked"]),
            primitives::phrase(&["can't", "be", "blocked"]),
            primitives::phrase(&["cannot", "be", "blocked"]),
        ))
        .parse_next(&mut probe)
        .is_ok()
        {
            let mut suffix_probe = probe.clone();
            let mut later_word = false;
            while !suffix_probe.is_empty() {
                let token: &'a OwnedLexToken = any.parse_next(&mut suffix_probe)?;
                later_word |= token.as_word().is_some();
            }
            if !later_word {
                unblockable_tail = true;
            }
        }

        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if let Some(word) = token.as_word() {
            word_count += 1;
            conjoined |= word == "and";
        }
    }

    Ok(KeywordAbilityFacts {
        word_count,
        sorcery_speed_reminder,
        once_per_turn_reminder,
        conjoined,
        unblockable_tail,
    })
}

#[cfg(test)]
mod tests;
