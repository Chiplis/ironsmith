use super::*;
use crate::target::{ObjectFilter, SourceReferenceSurface, TaggedOpbjectRelation};
use crate::types::CardType;

pub(super) fn normalized_reminder_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let words = if let Some(rest) = common::strip_phrase_prefix(words, &["it", "has"])
        .or_else(|| common::strip_phrase_prefix(words, &["they", "have"]))
    {
        rest
    } else {
        words
    };

    // Possessive apostrophes are removed from parser words, so the authored
    // "This token's power ..." surface arrives as "this tokens power ...".
    // The characteristic sentence may be quoted inside the larger token
    // definition rather than beginning the token slice. Normalize that
    // explicit token reference to the same semantic subject as "Its power".
    for subject in [
        &["this", "tokens"][..],
        &["this", "token"],
        &["thiss", "token"],
    ] {
        if let Some(start) = common::phrase_offset(words, subject)
            && let Some(tail) = words.get(start + subject.len()..)
            && (common::phrase_present(
                tail,
                &["power", "and", "toughness", "are", "each", "equal", "to"],
            ) || (common::phrase_present(tail, &["power", "is", "equal", "to"])
                && common::phrase_present(tail, &["toughness", "is", "equal", "to"])))
        {
            let mut normalized = vec!["its"];
            normalized.extend_from_slice(tail);
            return normalized;
        }
    }

    for prefix in [
        &["when", "it"][..],
        &["whenever", "it"],
        &["when", "they"],
        &["whenever", "they"],
    ] {
        if let Some(rest) = common::strip_phrase_prefix(words, prefix) {
            let mut normalized = vec![words[0], "this", "token"];
            normalized.extend_from_slice(rest);
            return normalized;
        }
    }
    words.to_vec()
}

fn parse_possessive_stat_rhs(words: &[&str], is_power: bool) -> Option<Value> {
    let stat_word = if is_power { "power" } else { "toughness" };
    let owner_words = words.strip_suffix(&[stat_word])?;
    let source_value = || {
        if is_power {
            Value::SourcePower
        } else {
            Value::SourceToughness
        }
    };
    let tagged_value = |tag: crate::tag::CompilerReferenceTag| {
        let spec = Box::new(ChooseSpec::Tagged(tag.bind()));
        if is_power {
            Value::PowerOf(spec)
        } else {
            Value::ToughnessOf(spec)
        }
    };

    if [
        &["this"][..],
        &["thiss"],
        &["this", "creature"],
        &["thiss", "creature"],
        &["this", "creatures"],
        &["thiss", "creatures"],
    ]
    .iter()
    .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(source_value());
    }
    if [&["that", "card"][..], &["that", "cards"]]
        .iter()
        .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(tagged_value(
            crate::tag::CompilerReferenceTag::TokenDynamicThatCard,
        ));
    }
    if [
        &["that", "creature"][..],
        &["that", "creatures"],
        &["that", "object"],
        &["that", "objects"],
    ]
    .iter()
    .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(tagged_value(crate::tag::CompilerReferenceTag::It));
    }
    None
}

fn parse_dynamic_rhs(words: &[&str]) -> Option<Value> {
    if common::phrase_exact(
        words,
        &["the", "total", "power", "of", "those", "creatures"],
    ) {
        let filter = ObjectFilter {
            card_types: vec![CardType::Creature],
            ..Default::default()
        }
        .match_tagged(
            crate::tag::CompilerReferenceTag::ZoneChangeGroup.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        return Some(Value::TotalPower(filter));
    }
    if let Some(value) =
        parse_possessive_stat_rhs(words, true).or_else(|| parse_possessive_stat_rhs(words, false))
    {
        return Some(value);
    }
    let (value, used) = value_expr::parse_value_expr_words(words)?;
    (used == words.len()).then_some(value)
}

pub(super) fn parse_dynamic_power_toughness(words: &[&str]) -> Option<(Value, Value)> {
    if let Some(rhs) = common::strip_phrase_prefix(
        words,
        &[
            "with",
            "base",
            "power",
            "and",
            "toughness",
            "each",
            "equal",
            "to",
        ],
    ) {
        let value = parse_dynamic_rhs(rhs)?;
        return Some((value.clone(), value));
    }

    if let Some(start) = common::phrase_offset(words, &["with", "power", "equal", "to"])
        && let Some(power_rhs) =
            common::strip_phrase_prefix(words.get(start..)?, &["with", "power", "equal", "to"])
    {
        let and_idx = common::phrase_offset(power_rhs, &["and"])?;
        let power = parse_dynamic_rhs(power_rhs.get(..and_idx)?)?;
        let toughness_rhs = common::strip_phrase_prefix(
            power_rhs.get(and_idx + 1..)?,
            &["toughness", "equal", "to"],
        )?;
        return Some((power, parse_dynamic_rhs(toughness_rhs)?));
    }

    if let Some(rhs) = common::strip_phrase_prefix(
        words,
        &[
            "its",
            "power",
            "and",
            "toughness",
            "are",
            "each",
            "equal",
            "to",
        ],
    ) {
        let value = parse_dynamic_rhs(rhs)?;
        return Some((value.clone(), value));
    }

    let power_rhs = common::strip_phrase_prefix(words, &["its", "power", "is", "equal", "to"])?;
    let and_idx = common::phrase_offset(power_rhs, &["and"])?;
    let power = parse_dynamic_rhs(power_rhs.get(..and_idx)?)?;
    let toughness_rhs = common::strip_phrase_prefix(
        power_rhs.get(and_idx + 1..)?,
        &["its", "toughness", "is", "equal", "to"],
    )?;
    let toughness = if let Some(offset_words) =
        common::strip_phrase_prefix(toughness_rhs, &["that", "number", "plus"])
            .or_else(|| common::strip_phrase_prefix(toughness_rhs, &["that", "amount", "plus"]))
    {
        Value::Add(
            Box::new(power.clone()),
            Box::new(parse_dynamic_rhs(offset_words)?),
        )
    } else if common::phrase_exact(toughness_rhs, &["that", "number"])
        || common::phrase_exact(toughness_rhs, &["that", "amount"])
    {
        power.clone()
    } else {
        parse_dynamic_rhs(toughness_rhs)?
    };
    Some((power, toughness))
}

fn parse_named_source_counter_dynamic_power_toughness(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<(Value, Value)> {
    let rhs = common::strip_phrase_prefix(
        words,
        &[
            "its",
            "power",
            "and",
            "toughness",
            "are",
            "each",
            "equal",
            "to",
        ],
    )?;
    let descriptor = common::strip_phrase_prefix(rhs, &["the", "number", "of"])?;
    let counter_idx = crate::word_primitives::select_word_position(descriptor, |word| {
        matches!(word, "counter" | "counters")
    })?;
    if counter_idx == 0 || descriptor.get(counter_idx + 1) != Some(&"on") {
        return None;
    }
    let reference_words = descriptor.get(counter_idx + 2..)?;
    if reference_words.is_empty() {
        return None;
    }
    let counter_type =
        crate::grammar::filters::parse_counter_type_words(descriptor.get(..=counter_idx)?)?;

    // The normalized P/T prefix can collapse `this token's` into `its`, so
    // locate the unchanged proper-name suffix from the end of the authored
    // token stream rather than trying to map every normalized prefix word.
    let positions = crate::lexer::parser_token_word_positions(tokens);
    let reference_word_start = positions.len().checked_sub(reference_words.len())?;
    let reference_token_start = positions.get(reference_word_start)?.0;
    let reference_token_end = positions.last()?.0.checked_add(1)?;
    let reference_tokens = tokens.get(reference_token_start..reference_token_end)?;
    if !crate::lexer::is_authored_proper_name_phrase(reference_tokens)
        || !crate::lexer::is_bare_card_name_phrase(reference_tokens)
    {
        return None;
    }
    let surface = SourceReferenceSurface::FullName(crate::lexer::render_bare_card_name_surface(
        reference_tokens,
    ));
    let value = Value::CountersOn(
        Box::new(crate::util::source_choose_spec_for_surface(surface)),
        Some(counter_type),
    );
    Some((value.clone(), value))
}

pub fn parse_token_dynamic_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    let raw_words = parser_token_word_refs(tokens);
    let words = normalized_reminder_words(&raw_words);
    parse_dynamic_power_toughness(&words)
        .or_else(|| parse_named_source_counter_dynamic_power_toughness(tokens, &words))
}
