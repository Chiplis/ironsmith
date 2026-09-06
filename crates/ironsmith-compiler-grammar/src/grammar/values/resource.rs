use super::*;

pub fn parse_add_mana_equal_amount_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    fn canonical_add_mana_equal_amount_value(value: Value) -> Value {
        match value {
            Value::SourcePower => Value::PowerOf(Box::new(ChooseSpec::Source)),
            Value::SourceToughness => Value::ToughnessOf(Box::new(ChooseSpec::Source)),
            Value::Add(left, right) => Value::Add(
                Box::new(canonical_add_mana_equal_amount_value(*left)),
                Box::new(canonical_add_mana_equal_amount_value(*right)),
            ),
            other => other,
        }
    }

    let (_, _, tail_tokens) =
        primitives::find_prefix(tokens, || primitives::phrase(EQUAL_TO_PHRASE))?;
    let tail_clause = LexedClause::new(tail_tokens).trimmed();
    let tail = tail_clause.word_refs();
    if tail.is_empty() {
        return None;
    }

    let segment_clause = |start: usize, end: usize| -> Option<LexedClause<'_>> {
        tail_clause.between_word_range(start, end)
    };

    let sacrificed_object_kind = |words: &[&str]| -> Option<SacrificedObjectKind> {
        let sacrificed = crate::slice_primitives::find_window_by(words, 2, |pair| {
            pair.first().copied() == Some("sacrificed")
        })?;
        match words.get(sacrificed + 1).copied()? {
            "creature" | "creatures" | "creature's" => Some(SacrificedObjectKind::Creature),
            "artifact" | "artifacts" | "artifact's" => Some(SacrificedObjectKind::Artifact),
            "enchantment" | "enchantments" | "enchantment's" => {
                Some(SacrificedObjectKind::Enchantment)
            }
            "permanent" | "permanents" | "permanent's" => Some(SacrificedObjectKind::Permanent),
            _ => None,
        }
    };

    let parse_power_or_toughness_segment =
        |segment: &[&str], segment_clause: LexedClause<'_>| -> Option<Value> {
            if segment.last().copied() == Some(POWER_WORD)
                && let Some(surface) =
                    source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::PowerOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }
            if segment.last().copied() == Some(TOUGHNESS_WORD)
                && let Some(surface) =
                    source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::ToughnessOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }

            parse_value_stat_segment(segment_clause)
        };

    let parse_mana_value_segment =
        |segment: &[&str], segment_clause: LexedClause<'_>| -> Option<Value> {
            let is_tagged_that_object_mana_value = || {
                if segment.len() < 4 || segment[0] != THAT_WORD {
                    return false;
                }
                let suffix_start = segment.len().saturating_sub(MANA_VALUE_SUFFIX.len());
                for (idx, expected) in MANA_VALUE_SUFFIX.iter().copied().enumerate() {
                    if segment.get(suffix_start + idx).copied() != Some(expected) {
                        return false;
                    }
                }

                !segment[1..segment.len() - 2].is_empty()
            };

            if let Some(value) = parse_value_mana_value_segment(segment_clause) {
                return Some(value);
            }
            if is_tagged_that_object_mana_value() {
                return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                    (crate::tag::CompilerReferenceTag::It.bind()).into(),
                ))));
            }
            None
        };

    let parse_amount_segment = |start: usize, end: usize| -> Option<Value> {
        let segment = &tail[start..end];
        let segment_clause = segment_clause(start, end)?;
        let value = parse_mana_value_segment(segment, segment_clause)
            .or_else(|| {
                parse_value_expr_words(segment)
                    .and_then(|(value, used)| (used == segment.len()).then_some(value))
            })
            .or_else(|| parse_power_or_toughness_segment(segment, segment_clause))
            .or_else(|| {
                if segment.len() == 1 {
                    parse_number_word_i32(segment[0]).map(Value::Fixed)
                } else {
                    None
                }
            })?;
        Some(match sacrificed_object_kind(segment) {
            Some(kind) => value.with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
            None => value,
        })
    };

    let difference_prefix_len = if crate::word_primitives::parse_sequence_prefix(
        &tail,
        &["the", "difference", "between"],
    ) {
        Some(3)
    } else if crate::word_primitives::parse_sequence_prefix(&tail, &["difference", "between"]) {
        Some(2)
    } else {
        None
    };
    if let Some(prefix_len) = difference_prefix_len
        && let Some(and_offset) =
            crate::slice_primitives::select_position(&tail[prefix_len..], |word| *word == "and")
    {
        let and_idx = prefix_len + and_offset;
        let parse_difference_segment = |start: usize, end: usize| -> Option<Value> {
            let segment = &tail[start..end];
            if segment.first().copied() == Some("that")
                && segment
                    .iter()
                    .any(|word| matches!(*word, "spell" | "spells" | "spell's"))
                && crate::word_primitives::parse_sequence_suffix(segment, &["mana", "value"])
            {
                return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                    (crate::tag::CompilerReferenceTag::Triggering.bind()).into(),
                ))));
            }
            if segment.first().copied() == Some("that")
                && segment.len() > 3
                && crate::word_primitives::parse_sequence_suffix(segment, &["mana", "value"])
            {
                let mut reference_words = segment[..segment.len() - 2].to_vec();
                if let Some(noun) = reference_words.last_mut()
                    && let Some(singular) = crate::word_primitives::strip_word_suffix(noun, "s")
                {
                    *noun = singular;
                }
                let surface = reference_words.join(" ");
                return Some(Value::ManaValueOf(Box::new(
                    ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into())
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(surface),
                        )),
                )));
            }
            parse_amount_segment(start, end)
        };
        if and_idx > prefix_len
            && and_idx + 1 < tail.len()
            && let Some(left) = parse_difference_segment(prefix_len, and_idx)
            && let Some(right) = parse_difference_segment(and_idx + 1, tail.len())
        {
            let absolute = Value::absolute_difference(left, right);
            return Some(absolute.with_surface_hint(ValueSurfaceHint::Difference));
        }
    }

    let mut plus_idx = None;
    for (idx, word) in tail.iter().copied().enumerate() {
        if word == PLUS_WORD {
            plus_idx = Some(idx);
            break;
        }
    }

    if let Some(plus_idx) = plus_idx
        && plus_idx > 0
        && plus_idx + 1 < tail.len()
        && let Some(left) = parse_amount_segment(0, plus_idx)
        && let Some(right) = parse_amount_segment(plus_idx + 1, tail.len())
    {
        return Some(canonical_add_mana_equal_amount_value(Value::Add(
            Box::new(left),
            Box::new(right),
        )));
    }

    if let Some(value) = parse_amount_segment(0, tail.len()) {
        return Some(canonical_add_mana_equal_amount_value(value));
    }

    None
}
