use super::*;

/// How far the authored line was indented: `source_tokens` are lexed from the
/// trimmed line, while the source map speaks in untrimmed offsets.
fn authored_leading_whitespace(info: &LineInfo) -> usize {
    let original = info.normalized.original.as_str();
    original.len() - original.trim_start().len()
}

/// The authored proper name inside a source-target span, as tokens.
///
/// Some legacy target spans include the action verb (and a historical
/// source-map can begin one byte into it). The name is the first capitalized
/// word inside the mapped span that is not the start of that verb, through
/// the end of the span or the "and" that joins a second operand.
fn authored_name_tokens_for_span(
    info: &LineInfo,
    span: crate::cards::builders::TextSpan,
) -> Option<Vec<OwnedLexToken>> {
    let original_span = crate::util::map_span_to_original(
        span,
        info.normalized.normalized.as_str(),
        info.normalized.original.as_str(),
        &info.normalized.char_map,
    );
    let offset = authored_leading_whitespace(info);
    let start = original_span.start.saturating_sub(offset);
    let end = original_span.end.saturating_sub(offset);
    let covered: Vec<&OwnedLexToken> = info
        .source_tokens
        .iter()
        .filter(|token| token.span.start < end && token.span.end > start)
        .collect();
    let mut first_name = None;
    for (index, token) in covered.iter().enumerate() {
        let starts_capitalized = token.slice.chars().next().is_some_and(char::is_uppercase);
        let is_verb_start = ["exile", "put", "remove"]
            .iter()
            .any(|verb| crate::string_primitives::starts_with(verb, token.parser_text.as_str()));
        if starts_capitalized && !is_verb_start {
            first_name = Some(index);
            break;
        }
    }
    let first_name = first_name?;
    let name: Vec<OwnedLexToken> = covered[first_name..]
        .iter()
        .take_while(|token| !token.is_word("and"))
        .filter(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
        .map(|token| (*token).clone())
        .collect();
    (!name.is_empty()).then_some(name)
}

fn named_surface_for_span(
    info: &LineInfo,
    span: crate::cards::builders::TextSpan,
) -> Option<crate::target::SourceReferenceSurface> {
    let authored_tokens = authored_name_tokens_for_span(info, span)?;
    // Source maps produced before the final comma/`then` split can retain a
    // few bytes from the following clause (for example `Jace, th`). Reapply
    // the grammar boundary to the mapped slice before accepting it as a name.
    crate::grammar::source_surface_shapes::parse_leading_named_surface(&authored_tokens)
        .map(|shape| shape.surface)
}

/// Restore an authored named-source surface on source exile actions after
/// source-reference normalization has turned the semantic target into
/// `TargetAst::Source`. The retained span maps back to the original line, so
/// this stays structural and also covers short aliases on other activated
/// abilities without depending on a card name.
pub(super) fn recognize_named_source_action_surfaces(info: &LineInfo, effects: &mut [EffectAst]) {
    fn named_surface_from_authored_exile_clause(
        info: &LineInfo,
        span: crate::cards::builders::TextSpan,
    ) -> Option<crate::target::SourceReferenceSurface> {
        let original_span = crate::util::map_span_to_original(
            span,
            info.normalized.normalized.as_str(),
            info.normalized.original.as_str(),
            &info.normalized.char_map,
        );
        // `source_tokens` are lexed from the trimmed line; the source map
        // speaks in untrimmed offsets.
        let offset = authored_leading_whitespace(info);
        crate::grammar::source_surface_shapes::parse_named_operand_nearest_to(
            &info.source_tokens,
            "exile",
            original_span.start.saturating_sub(offset),
        )
        .map(|shape| shape.surface)
    }

    fn named_surface_from_authored_counter_clause(
        info: &LineInfo,
    ) -> Option<crate::target::SourceReferenceSurface> {
        crate::grammar::source_surface_shapes::parse_unique_named_counter_on_operand(
            &info.source_tokens,
        )
        .map(|shape| shape.surface)
    }

    fn apply_target(info: &LineInfo, target: &mut TargetAst) {
        let TargetAst::Source(Some(span)) = target else {
            return;
        };
        let Some(surface) = named_surface_for_span(info, *span)
            .or_else(|| named_surface_from_authored_exile_clause(info, *span))
        else {
            return;
        };
        *target = TargetAst::Object(
            ObjectFilter::source_with_surface(surface),
            None,
            Some(*span),
        );
    }

    fn apply_counter_target(info: &LineInfo, target: &mut TargetAst) {
        let TargetAst::Source(span) = target else {
            return;
        };
        let surface = span
            .and_then(|span| named_surface_for_span(info, span))
            .or_else(|| named_surface_from_authored_counter_clause(info));
        let Some(surface) = surface else {
            return;
        };
        *target = TargetAst::Object(ObjectFilter::source_with_surface(surface), None, *span);
    }

    fn authored_return_surface(info: &LineInfo) -> Option<crate::target::SourceReferenceSurface> {
        let tokens = info.source_tokens.as_slice();
        if !tokens.iter().any(|token| token.is_word("exile")) {
            return None;
        }
        crate::grammar::source_surface_shapes::parse_unique_pronoun_operand_after(tokens, "return")
            .map(|shape| shape.surface)
    }

    fn transformed_source_return_count(effects: &[EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect
                && matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::ReturnToBattlefield {
                        target: TargetAst::Source(_),
                        controller: ReturnControllerAst::Owner,
                        transformed: true,
                        ..
                    } | SubjectVerbActionAst::MoveToZone {
                        target: TargetAst::Source(_),
                        zone: Zone::Battlefield,
                        battlefield_controller: ReturnControllerAst::Owner,
                        battlefield_transformed: true,
                        ..
                    }
                )
            {
                count += 1;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += transformed_source_return_count(nested)
            });
        }
        count
    }

    fn apply_return_surface(
        effects: &mut [EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                let target = match &mut subject_verb.action {
                    SubjectVerbActionAst::ReturnToBattlefield {
                        target,
                        controller: ReturnControllerAst::Owner,
                        transformed: true,
                        ..
                    }
                    | SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: Zone::Battlefield,
                        battlefield_controller: ReturnControllerAst::Owner,
                        battlefield_transformed: true,
                        ..
                    } => Some(target),
                    _ => None,
                };
                if let Some(target) = target
                    && let TargetAst::Source(span) = target
                {
                    let span = *span;
                    *target = TargetAst::Object(
                        ObjectFilter::source_with_surface(surface.clone()),
                        None,
                        span,
                    );
                }
            }
            for_each_nested_effects_mut(effect, true, |nested| {
                apply_return_surface(nested, surface)
            });
        }
    }

    fn apply(info: &LineInfo, effects: &mut [EffectAst]) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                match &mut subject_verb.action {
                    SubjectVerbActionAst::Exile { target, .. } => apply_target(info, target),
                    SubjectVerbActionAst::PutCounters { target, .. } => {
                        apply_counter_target(info, target)
                    }
                    SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: Zone::Exile,
                        ..
                    } => apply_target(info, target),
                    _ => {}
                }
            }
            for_each_nested_effects_mut(effect, true, |nested| apply(info, nested));
        }
    }

    apply(info, effects);
    if transformed_source_return_count(effects) == 1
        && let Some(surface) = authored_return_surface(info)
    {
        apply_return_surface(effects, &surface);
    }
}

#[cfg(test)]
mod named_source_counter_surface_tests {
    use super::*;

    fn line_info(raw: &str) -> LineInfo {
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: raw.to_string(),
            source_tokens: crate::lexer::lex_line(raw, 0).expect("test line should lex"),
            normalized: crate::cards::builders::NormalizedLine::from_char_map(
                raw,
                raw.to_ascii_lowercase(),
                Vec::new(),
            ),
            semantic_facts: Default::default(),
        }
    }

    fn put_counters(target: TargetAst) -> EffectAst {
        EffectAst::subject_verb_put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            crate::effect::Value::Fixed(2),
            target,
            None,
            false,
        )
    }

    #[test]
    fn named_counter_recipient_restores_its_authored_short_source_surface() {
        let info = line_info("Power-up — {4}{R}: Put two +1/+1 counters on Adept.");
        let mut effects = vec![put_counters(TargetAst::Source(None))];

        recognize_named_source_action_surfaces(&info, &mut effects);

        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("expected one counter action: {effects:#?}");
        };
        let SubjectVerbActionAst::PutCounters {
            target: TargetAst::Object(filter, _, _),
            ..
        } = &subject_verb.action
        else {
            panic!("expected a typed named source target: {subject_verb:#?}");
        };
        assert_eq!(
            filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ShortName(
                "Adept".to_string()
            ))
        );
    }

    #[test]
    fn ordinary_this_creature_counter_recipient_remains_type_relative() {
        let info = line_info("{4}{R}: Put two +1/+1 counters on this creature.");
        let mut effects = vec![put_counters(TargetAst::Source(None))];

        recognize_named_source_action_surfaces(&info, &mut effects);

        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("expected one counter action: {effects:#?}");
        };
        assert!(
            matches!(
                &subject_verb.action,
                SubjectVerbActionAst::PutCounters {
                    target: TargetAst::Source(None),
                    ..
                }
            ),
            "ordinary type-relative source should not acquire a name: {effects:#?}"
        );
    }
}
