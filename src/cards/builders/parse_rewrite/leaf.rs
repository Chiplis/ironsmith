use winnow::ascii::{digit1, multispace0, multispace1};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated, terminated};
use winnow::error::{ContextError, Result as WResult};
use winnow::prelude::*;
use winnow::token::{one_of, take_while};

use crate::cards::builders::{CardTextError, ChoiceCount, MetadataLine, parse_subtype_word};
use crate::color::Color;
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::{CardType, Subtype, Supertype};

use super::ported_activation_and_restrictions::parse_activation_cost;
use super::ported_object_filters::parse_object_filter;
use super::util::tokenize_line;

#[derive(Debug, Clone)]
pub(crate) struct TypeLineCst {
    pub(crate) supertypes: Vec<Supertype>,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivationCostCst {
    pub(crate) raw: String,
    pub(crate) segments: Vec<ActivationCostSegmentCst>,
    pub(crate) legacy_lowered: Option<TotalCost>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActivationCostSegmentCst {
    Mana(ManaCost),
    Tap,
    TapChosen {
        count: u32,
        filter_text: String,
        other: bool,
    },
    Untap,
    Life(u32),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    DiscardCard(u32),
    DiscardFiltered {
        count: u32,
        card_types: Vec<CardType>,
        random: bool,
    },
    Mill(u32),
    SacrificeSelf,
    SacrificeCreature,
    SacrificeChosen {
        count: u32,
        filter_text: String,
        other: bool,
    },
    ExileSelf,
    ExileSelfFromGraveyard,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileFromGraveyard {
        count: u32,
        card_type: Option<CardType>,
    },
    ExileChosen {
        choice_count: ChoiceCount,
        filter_text: String,
    },
    ExileTopLibrary {
        count: u32,
    },
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter_text: String,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
    },
    PutCountersChosen {
        counter_type: CounterType,
        count: u32,
        filter_text: String,
    },
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveCountersAmong {
        counter_type: Option<CounterType>,
        count: u32,
        filter_text: String,
        display_x: bool,
    },
    RemoveCountersDynamic {
        counter_type: Option<CounterType>,
        display_x: bool,
    },
}

fn parse_word<'a>(input: &mut &'a str) -> WResult<&'a str> {
    take_while(1.., |ch: char| {
        ch.is_ascii_alphabetic() || ch == '\'' || ch == '-'
    })
    .parse_next(input)
}

fn spaced<'a, O, P>(parser: P) -> impl Parser<&'a str, O, ContextError>
where
    P: Parser<&'a str, O, ContextError>,
{
    delimited(multispace0, parser, multispace0)
}

fn count_word_value(word: &str) -> Option<u32> {
    match word.to_ascii_lowercase().as_str() {
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => None,
    }
}

fn parse_card_type_word(word: &str) -> Option<CardType> {
    match word.to_ascii_lowercase().as_str() {
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

fn parse_color_word(word: &str) -> Option<ColorSet> {
    Color::from_name(word).map(ColorSet::from_color)
}

fn parse_supertype_word_local(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

fn intern_counter_name(word: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" | "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "+1/+0" => Some(CounterType::PlusOnePlusZero),
        "+0/+1" => Some(CounterType::PlusZeroPlusOne),
        "+1/+2" => Some(CounterType::PlusOnePlusTwo),
        "+2/+2" => Some(CounterType::PlusTwoPlusTwo),
        "-0/-2" => Some(CounterType::MinusZeroMinusTwo),
        "-2/-2" => Some(CounterType::MinusTwoMinusTwo),
        "deathtouch" => Some(CounterType::Deathtouch),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "void" => Some(CounterType::Void),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "finality" => Some(CounterType::Finality),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "burden" => Some(CounterType::Named(intern_counter_name("burden"))),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        "luck" => Some(CounterType::Luck),
        "oil" => Some(CounterType::Oil),
        _ => None,
    }
}

fn parse_count_inner(input: &mut &str) -> WResult<u32> {
    alt((
        digit1.try_map(str::parse::<u32>),
        parse_word.verify_map(count_word_value),
    ))
    .parse_next(input)
}

fn parse_mana_symbol_inner(input: &mut &str) -> WResult<ManaSymbol> {
    alt((
        digit1.try_map(|digits: &str| digits.parse::<u8>().map(ManaSymbol::Generic)),
        one_of([
            'W', 'w', 'U', 'u', 'B', 'b', 'R', 'r', 'G', 'g', 'C', 'c', 'S', 's', 'X', 'x', 'P',
            'p',
        ])
        .map(|ch: char| match ch.to_ascii_uppercase() {
            'W' => ManaSymbol::White,
            'U' => ManaSymbol::Blue,
            'B' => ManaSymbol::Black,
            'R' => ManaSymbol::Red,
            'G' => ManaSymbol::Green,
            'C' => ManaSymbol::Colorless,
            'S' => ManaSymbol::Snow,
            'X' => ManaSymbol::X,
            'P' => ManaSymbol::Life(2),
            _ => unreachable!("one_of constrains supported mana-symbol letters"),
        }),
    ))
    .parse_next(input)
}

fn parse_mana_group_inner(input: &mut &str) -> WResult<Vec<ManaSymbol>> {
    delimited(
        spaced("{"),
        separated(1.., parse_mana_symbol_inner, spaced('/')),
        spaced("}"),
    )
    .parse_next(input)
}

fn parse_mana_cost_inner(input: &mut &str) -> WResult<ManaCost> {
    repeat(1.., parse_mana_group_inner)
        .map(ManaCost::from_pips)
        .parse_next(input)
}

fn parse_discard_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    preceded(
        spaced("discard"),
        alt((
            spaced("this card").value(ActivationCostSegmentCst::DiscardSource),
            (
                opt(terminated(parse_count_inner, multispace1)),
                opt(spaced(alt(("a", "an")))),
                spaced(alt(("card", "cards"))),
            )
                .map(|(count, _article, _)| {
                    ActivationCostSegmentCst::DiscardCard(count.unwrap_or(1))
                }),
        )),
    )
    .parse_next(input)
}

fn parse_pay_life_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    "pay".parse_next(input)?;
    multispace1.parse_next(input)?;
    let amount = parse_count_inner.parse_next(input)?;
    multispace1.parse_next(input)?;
    alt(("life", "lives")).parse_next(input)?;
    Ok(ActivationCostSegmentCst::Life(amount))
}

fn parse_sacrifice_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    let original = *input;
    if let Ok(parsed) = preceded(
        spaced("sacrifice"),
        alt((
            spaced(alt((
                "this creature",
                "this artifact",
                "this aura",
                "this enchantment",
                "this equipment",
                "this fortification",
                "this land",
                "this permanent",
                "this card",
            )))
            .value(ActivationCostSegmentCst::SacrificeSelf),
            spaced("a creature").value(ActivationCostSegmentCst::SacrificeCreature),
        )),
    )
    .parse_next(input)
    {
        return Ok(parsed);
    }

    *input = original;
    "sacrifice".parse_next(input)?;
    multispace1.parse_next(input)?;
    let count = parse_count_inner.parse_next(input).unwrap_or(1);
    let mut other = false;
    if input.trim_start().starts_with("another ") {
        multispace0.parse_next(input)?;
        "another".parse_next(input)?;
        other = true;
    }
    multispace0.parse_next(input)?;
    let filter_text = input.trim().to_string();
    if filter_text.is_empty() {
        return Err(ContextError::new());
    }
    *input = "";
    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_sacrifice_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("sacrifice ") else {
        return Err(CardTextError::ParseError(
            "rewrite sacrifice parser expected leading 'sacrifice'".to_string(),
        ));
    };

    if matches!(
        rest,
        "it" | "this"
            | "this creature"
            | "this artifact"
            | "this aura"
            | "this enchantment"
            | "this equipment"
            | "this fortification"
            | "this land"
            | "this permanent"
            | "this card"
    ) {
        return Ok(ActivationCostSegmentCst::SacrificeSelf);
    }
    if rest == "a creature" {
        return Ok(ActivationCostSegmentCst::SacrificeCreature);
    }

    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an") {
            idx = 1;
        }
    }

    if parts.get(idx).is_some_and(|part| *part == "another") {
        other = true;
        idx += 1;
    }

    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_tap_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((spaced("{t}"), spaced("t")))
        .value(ActivationCostSegmentCst::Tap)
        .parse_next(input)
}

fn parse_tap_chosen_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("tap ") else {
        return Err(CardTextError::ParseError(
            "rewrite tap-cost parser expected leading 'tap'".to_string(),
        ));
    };

    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;

    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an") {
            idx = 1;
        }
    }

    if parts.get(idx).is_some_and(|part| *part == "another") {
        other = true;
        idx += 1;
    }

    if !parts.get(idx).is_some_and(|part| *part == "untapped") {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser expected untapped selector in '{raw}'"
        )));
    }
    idx += 1;

    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::TapChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_untap_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((spaced("{q}"), spaced("q")))
        .value(ActivationCostSegmentCst::Untap)
        .parse_next(input)
}

fn parse_pay_energy_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    "pay".parse_next(input)?;
    multispace1.parse_next(input)?;

    let mut count = 0u32;
    loop {
        if spaced("{e}").value(()).parse_next(input).is_ok() {
            count += 1;
        } else if spaced("e").value(()).parse_next(input).is_ok() {
            count += 1;
        } else {
            break;
        }
    }

    if count == 0 {
        return Err(ContextError::new());
    }
    Ok(ActivationCostSegmentCst::Energy(count))
}

fn parse_mill_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    "mill".parse_next(input)?;
    multispace1.parse_next(input)?;
    let count = alt((parse_count_inner, alt(("a", "an")).value(1))).parse_next(input)?;
    multispace1.parse_next(input)?;
    alt(("card", "cards")).parse_next(input)?;
    Ok(ActivationCostSegmentCst::Mill(count))
}

fn parse_counter_type_descriptor(raw: &str) -> Result<CounterType, CardTextError> {
    let words = raw
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| ch == ',' || ch == '.'))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    let counter_idx = words
        .iter()
        .position(|word| matches!(*word, "counter" | "counters"));

    let counter_type = counter_idx.and_then(|counter_idx| {
        if counter_idx == 0 {
            return None;
        }

        let prev = words[counter_idx - 1];
        if let Some(counter_type) = parse_counter_type_word(prev) {
            return Some(counter_type);
        }

        if prev == "strike" && counter_idx >= 2 {
            match words[counter_idx - 2] {
                "double" => return Some(CounterType::DoubleStrike),
                "first" => return Some(CounterType::FirstStrike),
                _ => {}
            }
        }

        if matches!(
            prev,
            "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six" | "another"
        ) {
            return None;
        }

        prev.chars()
            .all(|ch| ch.is_ascii_alphabetic())
            .then(|| CounterType::Named(intern_counter_name(prev)))
    });

    counter_type.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite counter parser could not determine counter type from '{raw}'"
        ))
    })
}

fn parse_loyalty_shorthand_activation_cost_rewrite(raw: &str) -> Option<TotalCost> {
    let normalized = raw.trim().replace('−', "-");
    let prefix = normalized
        .split_once(':')
        .map(|(left, _)| left.trim())
        .unwrap_or(normalized.as_str());

    if let Some(rest) = prefix.strip_prefix('+')
        && let Ok(amount) = rest.parse::<u32>()
    {
        return Some(if amount == 0 {
            TotalCost::free()
        } else {
            TotalCost::from_cost(Cost::add_counters(CounterType::Loyalty, amount))
        });
    }

    if let Some(rest) = prefix.strip_prefix('-') {
        if rest.eq_ignore_ascii_case("x") {
            return Some(TotalCost::from_cost(Cost::remove_any_counters_from_source(
                Some(CounterType::Loyalty),
                true,
            )));
        }
        if let Ok(amount) = rest.parse::<u32>() {
            return Some(TotalCost::from_cost(Cost::remove_counters(
                CounterType::Loyalty,
                amount,
            )));
        }
    }

    (prefix == "0").then(TotalCost::free)
}

fn parse_discard_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("discard ") else {
        return Err(CardTextError::ParseError(
            "rewrite discard parser expected leading 'discard'".to_string(),
        ));
    };

    if rest == "your hand" {
        return Ok(ActivationCostSegmentCst::DiscardHand);
    }

    if rest == "this card" {
        return Ok(ActivationCostSegmentCst::DiscardSource);
    }

    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }

    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an"))
    {
        idx += 1;
    }

    let mut card_types = Vec::new();
    while let Some(part) = parts.get(idx).copied() {
        if matches!(part, "card" | "cards") {
            break;
        }
        if matches!(part, "and" | "or" | "a" | "an") {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type_word(part) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support selector '{raw}'"
            )));
        };
        if !card_types.contains(&card_type) {
            card_types.push(card_type);
        }
        idx += 1;
    }

    if !parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "card" | "cards"))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected card selector in '{raw}'"
        )));
    }
    idx += 1;

    let random = match parts.get(idx..) {
        None | Some([]) => false,
        Some(["at", "random"]) => true,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support trailing clause in '{raw}'"
            )));
        }
    };

    if card_types.is_empty() && !random {
        return Ok(ActivationCostSegmentCst::DiscardCard(count));
    }

    Ok(ActivationCostSegmentCst::DiscardFiltered {
        count,
        card_types,
        random,
    })
}

fn parse_exile_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("exile ") else {
        return Err(CardTextError::ParseError(
            "rewrite exile parser expected leading 'exile'".to_string(),
        ));
    };

    if rest.starts_with("target ") {
        return Err(CardTextError::ParseError(
            "unsupported targeted exile cost segment".to_string(),
        ));
    }

    if matches!(
        rest,
        "this"
            | "this card"
            | "this spell"
            | "this permanent"
            | "this creature"
            | "this artifact"
            | "this enchantment"
            | "this land"
            | "this aura"
            | "this vehicle"
    ) || rest.starts_with("this card from your ")
        || rest.starts_with("this spell from your ")
        || rest.starts_with("this creature from your ")
        || rest.starts_with("this artifact from your ")
        || rest.starts_with("this enchantment from your ")
        || rest.starts_with("this land from your ")
        || rest.starts_with("this aura from your ")
        || rest.starts_with("this vehicle from your ")
    {
        if rest.contains("from your graveyard") {
            return Ok(ActivationCostSegmentCst::ExileSelfFromGraveyard);
        }
        return Ok(ActivationCostSegmentCst::ExileSelf);
    }

    if let Some(top_suffix) = rest
        .strip_prefix("the top ")
        .and_then(|tail| tail.strip_suffix(" cards of your library"))
        .or_else(|| {
            rest.strip_prefix("the top ")
                .and_then(|tail| tail.strip_suffix(" card of your library"))
        })
    {
        let count = parse_count_word_rewrite(top_suffix.trim())?;
        return Ok(ActivationCostSegmentCst::ExileTopLibrary { count });
    }

    if let Some(hand_suffix) = rest.strip_suffix(" from your hand") {
        let parts = hand_suffix.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-hand parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let mut color_filter = None;
        if let Some(word) = parts.get(idx).copied()
            && let Some(color) = parse_color_word(word)
        {
            color_filter = Some(color);
            idx += 1;
        }

        if !parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "card" | "cards"))
        {
            return Err(CardTextError::ParseError(format!(
                "rewrite exile-from-hand parser expected card selector in '{raw}'"
            )));
        }

        return Ok(ActivationCostSegmentCst::ExileFromHand {
            count,
            color_filter,
        });
    }

    if let Some(graveyard_suffix) = rest.strip_suffix(" from your graveyard") {
        if let Some((choice_count, filter_text)) = parse_generic_choice_prefix(graveyard_suffix) {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter_text: format!("{filter_text} from your graveyard"),
            });
        }

        let parts = graveyard_suffix.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-graveyard parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let mut card_type = None;
        if let Some(word) = parts.get(idx).copied()
            && let Some(parsed) = parse_card_type_word(word)
        {
            card_type = Some(parsed);
            idx += 1;
        }

        if !parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "card" | "cards"))
        {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count: ChoiceCount::exactly(count as usize),
                filter_text: format!("{} from your graveyard", parts[idx..].join(" ")),
            });
        }

        idx += 1;
        if idx < parts.len() {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count: ChoiceCount::exactly(count as usize),
                filter_text: format!("{} from your graveyard", parts[idx - 1..].join(" ")),
            });
        }

        return Ok(ActivationCostSegmentCst::ExileFromGraveyard { count, card_type });
    }

    let (choice_count, mut filter_text) = parse_generic_choice_prefix(rest).ok_or_else(|| {
        CardTextError::ParseError(format!("rewrite exile parser does not yet support '{raw}'"))
    })?;
    if filter_text.ends_with(" from a single graveyard") {
        filter_text = filter_text.replace(" from a single graveyard", " from a graveyard");
    }
    Ok(ActivationCostSegmentCst::ExileChosen {
        choice_count,
        filter_text,
    })
}

fn parse_return_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("return ") else {
        return Err(CardTextError::ParseError(
            "rewrite return-cost parser expected leading 'return'".to_string(),
        ));
    };
    let Some(target) = rest
        .strip_suffix(" to its owner's hand")
        .or_else(|| rest.strip_suffix(" to their owner's hand"))
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser expected owner-hand suffix in '{raw}'"
        )));
    };

    if matches!(
        target,
        "it" | "this"
            | "this card"
            | "this permanent"
            | "this creature"
            | "this artifact"
            | "this enchantment"
            | "this land"
    ) {
        return Ok(ActivationCostSegmentCst::ReturnSelfToHand);
    }

    let parts = target.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }
    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text })
}

fn parse_put_counter_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("put ") else {
        return Err(CardTextError::ParseError(
            "rewrite put-counter parser expected leading 'put'".to_string(),
        ));
    };
    let Some(on_idx) = rest.find(" on ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing 'on' in '{raw}'"
        )));
    };
    let descriptor = rest[..on_idx].trim();
    let target = rest[on_idx + 4..].trim();
    if !matches!(
        target,
        "this"
            | "this creature"
            | "this permanent"
            | "this artifact"
            | "this aura"
            | "this card"
            | "this land"
    ) {
        let parts = descriptor.split_whitespace().collect::<Vec<_>>();
        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let counter_descriptor = parts[idx..].join(" ");
        let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;
        return Ok(ActivationCostSegmentCst::PutCountersChosen {
            counter_type,
            count,
            filter_text: target.to_string(),
        });
    }

    let parts = descriptor.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }

    let counter_descriptor = parts[idx..].join(" ");
    let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;
    Ok(ActivationCostSegmentCst::PutCounters {
        counter_type,
        count,
    })
}

fn parse_remove_counter_segment_rewrite(
    raw: &str,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("remove ") else {
        return Err(CardTextError::ParseError(
            "rewrite remove-counter parser expected leading 'remove'".to_string(),
        ));
    };
    let Some(from_idx) = rest.find(" from ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing 'from' in '{raw}'"
        )));
    };
    let descriptor = rest[..from_idx].trim();
    let target = rest[from_idx + 6..].trim();
    let parts = descriptor.split_whitespace().collect::<Vec<_>>();
    if parts.starts_with(&["x"]) {
        let counter_descriptor = parts[1..].join(" ");
        let counter_type = (!counter_descriptor.is_empty())
            .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
            .transpose()?;
        return if let Some(filter_text) = target.strip_prefix("among ") {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: filter_text.to_string(),
                display_x: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: true,
            })
        };
    }
    if parts.starts_with(&["any", "number", "of"]) {
        let counter_descriptor = parts[3..].join(" ");
        let counter_type = (!counter_descriptor.is_empty())
            .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
            .transpose()?;
        return if let Some(filter_text) = target.strip_prefix("among ") {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: filter_text.to_string(),
                display_x: false,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
            })
        };
    }
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }

    let counter_descriptor = parts[idx..].join(" ");
    let counter_type = (!counter_descriptor.is_empty())
        .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
        .transpose()?;
    if let Some(filter_text) = target.strip_prefix("among ") {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: filter_text.to_string(),
            display_x: false,
        });
    }

    if !matches!(
        target,
        "this"
            | "this creature"
            | "this permanent"
            | "this artifact"
            | "this enchantment"
            | "this card"
            | "this land"
            | "it"
    ) {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target.to_string(),
            display_x: false,
        });
    }

    let counter_type = counter_type.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing counter type in '{raw}'"
        ))
    })?;
    Ok(ActivationCostSegmentCst::RemoveCounters {
        counter_type,
        count,
    })
}

fn parse_generic_choice_prefix(raw: &str) -> Option<(ChoiceCount, String)> {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("one or more ") {
        return Some((ChoiceCount::at_least(1), rest.trim().to_string()));
    }
    if let Some(rest) = trimmed.strip_prefix("any number of ") {
        return Some((ChoiceCount::any_number(), rest.trim().to_string()));
    }
    if let Some(rest) = trimmed.strip_prefix("x ") {
        return Some((ChoiceCount::dynamic_x(), rest.trim().to_string()));
    }

    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an" | "the") {
            idx = 1;
        }
    }

    let filter_text = parts[idx..].join(" ");
    (!filter_text.is_empty()).then_some((ChoiceCount::exactly(count as usize), filter_text))
}

fn parse_pay_energy_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("pay ") else {
        return Err(CardTextError::ParseError(
            "rewrite energy parser expected leading 'pay'".to_string(),
        ));
    };
    let rest = rest.trim();
    if rest.chars().all(|ch| matches!(ch, '{' | '}' | 'e' | ' ')) {
        let count = rest.matches("{e}").count();
        if count > 0 {
            return Ok(ActivationCostSegmentCst::Energy(count as u32));
        }
    }
    if rest == "{e}" || rest == "e" {
        return Ok(ActivationCostSegmentCst::Energy(1));
    }
    let Some(count_text) = rest
        .strip_suffix(" {e}")
        .or_else(|| rest.strip_suffix(" e"))
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite energy parser expected energy symbol in '{raw}'"
        )));
    };
    Ok(ActivationCostSegmentCst::Energy(parse_count_word_rewrite(
        count_text.trim(),
    )?))
}

fn parse_activation_cost_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((
        parse_tap_segment_inner,
        parse_untap_segment_inner,
        parse_pay_life_segment_inner,
        parse_pay_energy_segment_inner,
        parse_discard_segment_inner,
        parse_mill_segment_inner,
        parse_sacrifice_segment_inner,
        parse_mana_cost_inner.map(ActivationCostSegmentCst::Mana),
    ))
    .parse_next(input)
}

fn finish_parse<'a, O>(
    raw: &'a str,
    mut parser: impl Parser<&'a str, O, ContextError>,
    label: &str,
) -> Result<O, CardTextError> {
    let mut input = raw.trim();
    let parsed = parser
        .parse_next(&mut input)
        .map_err(|err| CardTextError::ParseError(format!("rewrite {label} parse failed: {err}")))?;
    if !input.trim().is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite {label} parser left trailing input: '{}'",
            input.trim()
        )));
    }
    Ok(parsed)
}

pub(crate) fn parse_count_word_rewrite(raw: &str) -> Result<u32, CardTextError> {
    finish_parse(raw, spaced(parse_count_inner), "count-word")
}

pub(crate) fn parse_mana_symbol_group_rewrite(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let trimmed = raw.trim().trim_matches('{').trim_matches('}');
    finish_parse(
        trimmed,
        separated(1.., parse_mana_symbol_inner, spaced('/')),
        "mana-group",
    )
}

pub(crate) fn parse_mana_cost_rewrite(raw: &str) -> Result<ManaCost, CardTextError> {
    finish_parse(raw, spaced(parse_mana_cost_inner), "mana-cost")
}

fn parse_shard_style_mana_or_tap_cost_rewrite(raw: &str) -> Option<(ManaSymbol, ManaSymbol)> {
    let normalized = raw.trim().to_ascii_lowercase();
    let (left_raw, right_raw) = normalized.split_once(" or ")?;

    fn parse_branch(branch: &str) -> Option<ManaSymbol> {
        let parts = branch
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() != 2 || parts[1] != "{t}" {
            return None;
        }

        let mana_cost = parse_mana_cost_rewrite(parts[0]).ok()?;
        let [pip] = mana_cost.pips() else {
            return None;
        };
        let [symbol] = pip.as_slice() else {
            return None;
        };
        Some(*symbol)
    }

    let left = parse_branch(left_raw)?;
    let right = parse_branch(right_raw)?;
    Some((left, right))
}

pub(crate) fn parse_type_line_rewrite(raw: &str) -> Result<TypeLineCst, CardTextError> {
    fn parse_type_words(segment: &str, context: &str) -> Result<Vec<String>, CardTextError> {
        let words = segment
            .split_whitespace()
            .map(|word| {
                word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '\'' && ch != '-')
            })
            .filter(|word| !word.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if words.is_empty() && !segment.trim().is_empty() {
            return Err(CardTextError::ParseError(format!(
                "rewrite {context} parser found no word tokens in '{}'",
                segment.trim()
            )));
        }
        Ok(words)
    }

    let normalized = raw.trim();
    let front_face = normalized.split("//").next().unwrap_or(normalized).trim();
    let mut emdash_parts = front_face.splitn(2, '—');
    let left = emdash_parts.next().unwrap_or("").trim();
    let right = emdash_parts.next().unwrap_or("").trim();

    let left_words = parse_type_words(left, "type-line-left")?;
    let right_words: Vec<String> = if right.is_empty() {
        Vec::new()
    } else {
        parse_type_words(right, "type-line-right")?
    };

    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    for word in left_words {
        if let Some(supertype) = parse_supertype_word_local(word.as_str()) {
            supertypes.push(supertype);
            continue;
        }
        if let Some(card_type) = parse_card_type_word(word.as_str()) {
            card_types.push(card_type);
        }
    }

    let mut subtypes = Vec::new();
    for word in right_words {
        if let Some(subtype) = parse_subtype_word(word.as_str()) {
            subtypes.push(subtype);
        }
    }

    Ok(TypeLineCst {
        supertypes,
        card_types,
        subtypes,
    })
}

pub(crate) fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let parse_rewrite_only = || -> Result<ActivationCostCst, CardTextError> {
        if let Some((left, right)) = parse_shard_style_mana_or_tap_cost_rewrite(raw) {
            return Ok(ActivationCostCst {
                raw: raw.trim().to_string(),
                segments: vec![
                    ActivationCostSegmentCst::Mana(ManaCost::from_pips(vec![vec![left, right]])),
                    ActivationCostSegmentCst::Tap,
                ],
                legacy_lowered: None,
            });
        }

        let mut segments = Vec::new();
        let mut raw_segments = Vec::new();
        for raw_segment in raw.split(',') {
            let segment = raw_segment.trim();
            if segment.is_empty() {
                continue;
            }
            if let Some((left, right)) = segment.split_once(" and sacrifice ") {
                raw_segments.push(left.trim().to_string());
                raw_segments.push(format!("sacrifice {}", right.trim()));
                continue;
            }
            raw_segments.push(segment.to_string());
        }

        for raw_segment in raw_segments {
            let segment = raw_segment.trim().trim_start_matches("and ").trim();
            if segment.is_empty() {
                continue;
            }
            let mut normalized_segment = segment.to_ascii_lowercase();
            if let Some(rest) = normalized_segment.strip_prefix("waterbend ") {
                normalized_segment = rest.trim().to_string();
            }
            let parsed = if normalized_segment.starts_with("exile ") {
                parse_exile_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("pay ")
                && (normalized_segment.contains("{e}") || normalized_segment.ends_with(" e"))
            {
                parse_pay_energy_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("return ") {
                parse_return_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("discard ") {
                parse_discard_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("sacrifice ") {
                parse_sacrifice_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("tap ")
                && normalized_segment.contains(" untapped ")
            {
                parse_tap_chosen_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("put ") {
                parse_put_counter_segment_rewrite(normalized_segment.as_str())
            } else if normalized_segment.starts_with("remove ") {
                parse_remove_counter_segment_rewrite(normalized_segment.as_str())
            } else {
                finish_parse(
                    normalized_segment.as_str(),
                    spaced(parse_activation_cost_segment_inner),
                    "activation-cost-segment",
                )
            }?;
            segments.push(parsed);
        }

        if segments.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite activation-cost parser found no segments".to_string(),
            ));
        }

        Ok(ActivationCostCst {
            raw: raw.trim().to_string(),
            segments,
            legacy_lowered: None,
        })
    };

    match parse_rewrite_only() {
        Ok(cst) => Ok(cst),
        Err(rewrite_err) => {
            if let Some(total_cost) = parse_loyalty_shorthand_activation_cost_rewrite(raw) {
                return Ok(ActivationCostCst {
                    raw: raw.trim().to_string(),
                    segments: Vec::new(),
                    legacy_lowered: Some(total_cost),
                });
            }
            let legacy_tokens = tokenize_line(raw, 0);
            if let Ok(total_cost) = parse_activation_cost(&legacy_tokens) {
                return Ok(ActivationCostCst {
                    raw: raw.trim().to_string(),
                    segments: Vec::new(),
                    legacy_lowered: Some(total_cost),
                });
            }
            Err(rewrite_err)
        }
    }
}

pub(crate) fn lower_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<TotalCost, CardTextError> {
    if let Some(total_cost) = &cst.legacy_lowered {
        return Ok(total_cost.clone());
    }

    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    for segment in &cst.segments {
        match segment {
            ActivationCostSegmentCst::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            ActivationCostSegmentCst::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            ActivationCostSegmentCst::TapChosen {
                count,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tokens = tokenize_line(filter_text, 0);
                let mut filter = parse_object_filter(&tokens, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                filter.untapped = true;
                let tag = format!("tap_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            ActivationCostSegmentCst::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::life(*amount));
            }
            ActivationCostSegmentCst::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            ActivationCostSegmentCst::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            ActivationCostSegmentCst::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            ActivationCostSegmentCst::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types,
                random,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random {
                    let card_filter = if card_types.is_empty() {
                        None
                    } else {
                        Some(ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            ..Default::default()
                        })
                    };
                    costs.push(Cost::validated_effect(Effect::discard_player_filtered(
                        *count as i32,
                        PlayerFilter::You,
                        true,
                        card_filter,
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            ActivationCostSegmentCst::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            ActivationCostSegmentCst::SacrificeSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::sacrifice_self());
            }
            ActivationCostSegmentCst::SacrificeCreature => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    1,
                )));
            }
            ActivationCostSegmentCst::SacrificeChosen {
                count,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter_text = if *count == 1 {
                    filter_text
                        .trim()
                        .strip_prefix("a ")
                        .or_else(|| filter_text.trim().strip_prefix("an "))
                        .unwrap_or(filter_text.trim())
                } else {
                    filter_text.trim()
                };
                let tokens = tokenize_line(normalized_filter_text, 0);
                let mut filter = parse_object_filter(&tokens, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    *count,
                )));
            }
            ActivationCostSegmentCst::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            ActivationCostSegmentCst::ExileFromGraveyard { count, card_type } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = ObjectFilter::default()
                    .owned_by(PlayerFilter::You)
                    .in_zone(crate::zone::Zone::Graveyard);
                if let Some(card_type) = card_type {
                    filter = filter.with_type(*card_type);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tokens = tokenize_line(filter_text, 0);
                let mut filter = parse_object_filter(&tokens, false)?;
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                )));
            }
            ActivationCostSegmentCst::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tokens = tokenize_line(filter_text, 0);
                let mut filter = parse_object_filter(&tokens, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if *count == 1 {
                    costs.push(Cost::return_to_hand(filter));
                } else {
                    let tag = format!("return_cost_{return_tag_id}");
                    return_tag_id += 1;
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        ChoiceCount::exactly(*count as usize),
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::return_to_hand(
                        ObjectFilter::tagged(tag),
                    )));
                }
            }
            ActivationCostSegmentCst::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type,
                count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter = filter_text.trim().to_ascii_lowercase();
                if matches!(
                    normalized_filter.as_str(),
                    "a creature you control" | "creature you control"
                ) {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let tokens = tokenize_line(filter_text, 0);
                let mut filter = parse_object_filter(&tokens, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("put_counter_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter_text,
                display_x,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tokens = tokenize_line(filter_text, 0);
                let mut filter = parse_object_filter(&tokens, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let max_count = if *display_x { u32::MAX / 4 } else { *count };
                costs.push(Cost::validated_effect(Effect::remove_any_counters_among(
                    max_count,
                    filter,
                    *counter_type,
                )));
            }
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_any_counters_from_source(
                    *counter_type,
                    *display_x,
                ));
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}

#[allow(dead_code)]
pub(crate) fn metadata_type_line_cst(
    value: &MetadataLine,
) -> Result<Option<TypeLineCst>, CardTextError> {
    match value {
        MetadataLine::TypeLine(raw) => parse_type_line_rewrite(raw).map(Some),
        _ => Ok(None),
    }
}
