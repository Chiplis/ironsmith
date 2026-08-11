use winnow::combinator::{alt, opt, preceded, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{EventValueSpec, Value};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

use super::super::super::super::lexer::{OwnedLexToken, trim_lexed_commas};
use super::super::super::primitives;
use super::{CreationPhrase, CreationTokens, CreationWordClass, CreationWords};

fn parse_counter_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    let mut idx = usize::from(surface.first_is(CreationWordClass::SourceCounterLeading));
    let counter_type = words
        .get(idx)
        .and_then(|word| {
            crate::runtime_backend::front_end::shared::util::parse_counter_type_word(word)
        })
        .or_else(|| {
            if !surface.class_at(idx + 1, CreationWordClass::Counter) {
                return None;
            }
            let tokens = token_surface.word_range(idx..idx + 2)?;
            crate::runtime_backend::front_end::shared::util::parse_counter_type_from_tokens(tokens)
        });
    if counter_type.is_some() {
        idx += 1;
    }
    if !surface.class_at(idx, CreationWordClass::Counter)
        || !surface.class_at(idx + 1, CreationWordClass::On)
    {
        return None;
    }
    let reference = words.get(idx + 2..)?;
    if CreationWords::new(reference).exact(CreationPhrase::SourceCounterReference) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }
    crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(reference)
        .map(|reference| {
            Value::CountersOn(
                Box::new(
                    crate::runtime_backend::front_end::shared::util::source_choose_spec_for_surface(
                        reference,
                    ),
                ),
                counter_type,
            )
        })
}

fn parse_static_ability(input: &mut primitives::WordSliceInput<'_>) -> WResult<StaticAbilityId> {
    alt((
        (
            primitives::word_slice_exact("first"),
            primitives::word_slice_exact("strike"),
        )
            .value(StaticAbilityId::FirstStrike),
        (
            primitives::word_slice_exact("double"),
            primitives::word_slice_exact("strike"),
        )
            .value(StaticAbilityId::DoubleStrike),
        primitives::word_slice_exact("flying").value(StaticAbilityId::Flying),
        primitives::word_slice_exact("deathtouch").value(StaticAbilityId::Deathtouch),
        primitives::word_slice_exact("haste").value(StaticAbilityId::Haste),
        primitives::word_slice_exact("hexproof").value(StaticAbilityId::Hexproof),
        primitives::word_slice_exact("indestructible").value(StaticAbilityId::Indestructible),
        primitives::word_slice_exact("lifelink").value(StaticAbilityId::Lifelink),
        alt((
            primitives::word_slice_exact("menace").value(StaticAbilityId::Menace),
            primitives::word_slice_exact("reach").value(StaticAbilityId::Reach),
            primitives::word_slice_exact("trample").value(StaticAbilityId::Trample),
            primitives::word_slice_exact("vigilance").value(StaticAbilityId::Vigilance),
        )),
    ))
    .parse_next(input)
}

fn parse_ability_separator(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("or"),
    ))
    .void()
    .parse_next(input)
}

fn parse_static_abilities_among(tokens: &[OwnedLexToken]) -> Option<Value> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    if !surface.starts(CreationPhrase::AbilityFromAmong) {
        return None;
    }
    let found = surface.phrase_location(CreationPhrase::FoundAmong)?;
    if found <= 3 {
        return None;
    }
    let mut ability_words: primitives::WordSliceInput<'_> = &words[3..found];
    let ability_ids: Vec<StaticAbilityId> = repeat(
        1..,
        preceded(opt(parse_ability_separator), parse_static_ability),
    )
    .parse_next(&mut ability_words)
    .ok()?;
    if !ability_words.is_empty() || ability_ids.is_empty() {
        return None;
    }
    let mut unique = Vec::new();
    for ability in ability_ids {
        if !unique.iter().any(|existing| existing == &ability) {
            unique.push(ability);
        }
    }
    let scope_start = token_surface.boundary(found + 2)?;
    let scope_tokens = trim_lexed_commas(tokens.get(scope_start..)?);
    if scope_tokens.is_empty() {
        return None;
    }
    let filter =
        crate::runtime_backend::object_filters::parse_object_filter(scope_tokens, false).ok()?;
    Some(
        Value::StaticAbilitiesAmong {
            filter,
            abilities: unique,
        }
        .with_surface_hint(ValueSurfaceHint::ForEach),
    )
}

pub(crate) fn parse_creation_for_each_dynamic_count_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    // Keep the canonical exact surface on the dedicated value.  The shared
    // turn-history parser also understands the broader `... that died this
    // turn` family, but creation counts use this value directly in lowering
    // and runtime metrics.
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    // Creation parsing has already removed the authored `for each` marker.
    // Restore it before the ordinary object-count fallback so provenance
    // counts such as "mana from a Cave spent to cast it" remain tied to the
    // spell's actual mana payment rather than becoming battlefield counts.
    let mut for_each_words = Vec::with_capacity(words.len() + 2);
    for_each_words.extend(["for", "each"]);
    for_each_words.extend(words.iter().copied());
    if let Some((value, used)) =
        super::super::super::shared_util::count_shapes::parse_for_each_count_value_words(
            &for_each_words,
        )
        && used == for_each_words.len()
        && !matches!(value.unhinted(), Value::Count(_))
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if surface.starts(CreationPhrase::CreatureDiedThisTurn) {
        return Some(Value::CreaturesDiedThisTurn.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(tokens)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if let Some(value) = parse_counter_count(tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if let Some(value) = parse_static_abilities_among(tokens) {
        return Some(value);
    }

    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    // Creation parsing has already removed the authored `for each` marker.
    // Restore that marker only for the shared colored-mana-symbol parser so
    // Chroma counts are recognized before the generic object-filter fallback.
    if let Some((value, used)) =
        super::super::super::shared_util::value_expr::colored_mana_symbols_in_costs(&for_each_words)
        && used == for_each_words.len()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if let Some(player) = crate::runtime_backend::front_end::grammar::shared_util::value_helper_shapes::parse_party_size_player(&words)
    {
        return Some(Value::PartySize(player).with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if surface.starts(CreationPhrase::CardExiledThisWay) {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::Count,
        )
        .with_action(ironsmith_core::PriorEffectAction::Exiled);
        return Some(Value::PendingPriorEffectMetric(query).with_surface_hints([
            ValueSurfaceHint::ForEach,
            ValueSurfaceHint::CardsExiledThisWay,
        ]));
    }
    if surface.starts(CreationPhrase::ObjectExiledThisWay) {
        let filter = if matches!(words.first().copied(), Some("permanent" | "permanents")) {
            ObjectFilter::permanent()
        } else {
            ObjectFilter::default()
        };
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::Count,
        )
        .with_action(ironsmith_core::PriorEffectAction::Exiled)
        .with_filter(filter);
        return Some(
            Value::PendingPriorEffectMetric(query).with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if surface.starts(CreationPhrase::GraveyardOrHandThisWay) {
        if surface.has(CreationWordClass::Put) && surface.has_phrase(CreationPhrase::ThisWay) {
            return Some(
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::AffectedObjects,
                    metric: ironsmith_core::EffectMetric::Count,
                }
                .with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
        let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if surface.starts(CreationPhrase::RegeneratedThisTurn) {
        return Some(
            Value::SourceRegeneratedThisTurnCount.with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if surface.has(CreationWordClass::Spell)
        && surface.has(CreationWordClass::Cast)
        && surface.has(CreationWordClass::Turn)
    {
        let player = if surface.has(CreationWordClass::YouReference) {
            PlayerFilter::You
        } else if surface.has(CreationWordClass::OpponentReference) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };
        if surface.has_phrase(CreationPhrase::OtherThanFirst) {
            return Some(
                Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                )
                .with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
        if surface.has(CreationWordClass::This) {
            return Some(
                Value::SpellsCastThisTurn(player).with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
    }
    if surface.starts(CreationPhrase::ColorsOfMana) {
        return Some(
            Value::ColorsOfManaSpentToCastThisSpell.with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if surface.starts(CreationPhrase::BasicLandTypes) {
        return Some(
            Value::BasicLandTypesAmong(ObjectFilter::land().you_control())
                .with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if surface.starts(CreationPhrase::CardTypesAmong) {
        let scope_start = token_surface.boundary(3)?;
        let scope_tokens = trim_lexed_commas(tokens.get(scope_start..)?);
        let filter =
            crate::runtime_backend::object_filters::parse_object_filter(scope_tokens, false)
                .ok()?;
        return Some(Value::CardTypesAmong(filter).with_surface_hint(ValueSurfaceHint::ForEach));
    }
    None
}

fn reject_lossy_count(tokens: &[OwnedLexToken], words: &[&str]) -> Result<(), CardTextError> {
    let surface = CreationWords::new(words);
    let clause = CreationTokens::new(tokens).words().join(" ");
    if surface.has_phrase(CreationPhrase::CardTypesAmong) {
        return Err(CardTextError::ParseError(format!(
            "unsupported card-types-among create count (clause: '{clause}')"
        )));
    }
    if surface.has_phrase(CreationPhrase::ThisWay) {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-way create count (clause: '{clause}')"
        )));
    }
    Ok(())
}

pub(crate) fn validate_creation_count_fallback_tokens(
    tokens: &[OwnedLexToken],
    full_clause_words: &[&str],
) -> Result<(), CardTextError> {
    let words = CreationTokens::new(tokens).words();
    let surface = CreationWords::new(&words);
    if surface.has_phrase(CreationPhrase::CardTypesAmong) {
        return Err(CardTextError::ParseError(format!(
            "unsupported card-types-among create count (clause: '{}')",
            full_clause_words.join(" ")
        )));
    }
    if surface.has_phrase(CreationPhrase::ThisWay) {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-way create count (clause: '{}')",
            full_clause_words.join(" ")
        )));
    }
    Ok(())
}

pub(crate) fn parse_investigate_for_each_count_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Value, CardTextError> {
    let token_surface = CreationTokens::new(tokens);
    let words = token_surface.words();
    let surface = CreationWords::new(&words);
    if let Some(exiled) = surface.phrase_location(CreationPhrase::ExiledThisWay) {
        let filter_tokens = token_surface
            .word_range(0..exiled)
            .map(trim_lexed_commas)
            .unwrap_or_default();
        let mut filter = if filter_tokens.is_empty() {
            ObjectFilter::default()
        } else {
            crate::runtime_backend::object_filters::parse_object_filter(filter_tokens, false)?
        };
        filter.zone = Some(Zone::Exile);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(Value::Count(filter).with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if surface.has_phrase(CreationPhrase::ThisWay) {
        return Ok(
            Value::EventValue(EventValueSpec::Amount).with_surface_hint(ValueSurfaceHint::ForEach)
        );
    }
    if let Some(value) = parse_creation_for_each_dynamic_count_tokens(tokens) {
        return Ok(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    reject_lossy_count(tokens, &words)?;
    Ok(
        Value::Count(crate::runtime_backend::object_filters::parse_object_filter(
            tokens, false,
        )?)
        .with_surface_hint(ValueSurfaceHint::ForEach),
    )
}

#[cfg(test)]
mod tests {
    use super::super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_dynamic_creation_counts() {
        let tokens = lex_line("creatures that died this turn", 0).unwrap();
        assert!(matches!(
            parse_creation_for_each_dynamic_count_tokens(&tokens).map(Value::into_unhinted),
            Some(Value::CreaturesDiedThisTurn)
        ));
        let tokens = lex_line("colors of mana spent to cast this spell", 0).unwrap();
        assert!(matches!(
            parse_creation_for_each_dynamic_count_tokens(&tokens).map(Value::into_unhinted),
            Some(Value::ColorsOfManaSpentToCastThisSpell)
        ));

        let tokens = lex_line("creature in your party", 0).unwrap();
        let party = parse_creation_for_each_dynamic_count_tokens(&tokens)
            .expect("party creation count should parse");
        assert!(party.has_surface_hint(ValueSurfaceHint::ForEach));
        assert_eq!(party.into_unhinted(), Value::PartySize(PlayerFilter::You));

        let tokens = lex_line("permanent exiled this way", 0).unwrap();
        let prior_exile = parse_creation_for_each_dynamic_count_tokens(&tokens)
            .expect("typed prior-exile creation count should parse");
        assert!(prior_exile.has_surface_hint(ValueSurfaceHint::ForEach));
        let Value::PendingPriorEffectMetric(query) = prior_exile.into_unhinted() else {
            panic!("expected typed prior-effect metric");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Exiled)
        );
        assert_eq!(
            query.filter.expect("permanent filter").card_types,
            ObjectFilter::permanent().card_types
        );

        let tokens = lex_line("mana from a Cave spent to cast it", 0).unwrap();
        let spent_mana = parse_creation_for_each_dynamic_count_tokens(&tokens)
            .expect("creation count should retain mana-payment provenance");
        let Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            reference,
        } = spent_mana.unhinted()
        else {
            panic!("expected typed mana-source count, got {spent_mana:#?}");
        };
        assert!(!include_source_noun);
        assert_eq!(
            *reference,
            ironsmith_core::ManaSpentCastReferenceSurface::It
        );
        assert_eq!(source_filter.subtypes, [crate::Subtype::Cave]);
    }

    #[test]
    fn parses_colored_mana_symbols_as_dynamic_creation_count() {
        let tokens = lex_line(
            "white mana symbol in the mana costs of permanents you control",
            0,
        )
        .unwrap();
        let value = parse_creation_for_each_dynamic_count_tokens(&tokens)
            .expect("colored mana-symbol creation count should parse");
        assert!(value.has_surface_hint(ValueSurfaceHint::ForEach));
        let Value::ManaSymbolsInManaCostOf { spec, color } = value.into_unhinted() else {
            panic!("expected structured mana-symbol creation count");
        };
        assert_eq!(color, crate::color::Color::White);
        let ChooseSpec::All(filter) = spec.unhinted() else {
            panic!("expected aggregate permanent scope");
        };
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }
}
