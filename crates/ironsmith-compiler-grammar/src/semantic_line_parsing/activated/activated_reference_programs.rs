use super::*;

fn authored_name_text_for_span(
    info: &LineInfo,
    span: crate::cards::builders::TextSpan,
) -> Option<String> {
    let original_span = crate::util::map_span_to_original(
        span,
        info.normalized.normalized.as_str(),
        info.normalized.original.as_str(),
        &info.normalized.char_map,
    );
    let authored_span = info
        .normalized
        .original
        .get(original_span.start..original_span.end)?
        .trim_matches(|ch: char| ch.is_whitespace() || ch == ',' || ch == '.');
    let authored_start = authored_span
        .char_indices()
        .filter(|(index, _)| {
            *index == 0
                || authored_span[..*index]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
        })
        .find_map(|(index, ch)| {
            if !ch.is_uppercase() {
                return None;
            }
            let word = authored_span[index..]
                .split(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '’')
                .next()
                .unwrap_or_default();
            let parser_word = word.to_ascii_lowercase();
            (!["exile", "put", "remove"]
                .iter()
                .any(|verb| verb.starts_with(parser_word.as_str())))
            .then_some(index)
        })?;
    Some(
        crate::string_primitives::split_once(&authored_span[authored_start..], " and ")
            .map_or(&authored_span[authored_start..], |(source, _)| source)
            .trim_matches(|ch: char| ch.is_whitespace() || ch == ',' || ch == '.')
            .to_string(),
    )
}

fn named_surface_for_span(
    info: &LineInfo,
    span: crate::cards::builders::TextSpan,
) -> Option<crate::target::SourceReferenceSurface> {
    // Some legacy target spans include the action verb (and a historical
    // source-map can begin one byte into it). Select the authored proper name
    // inside that grammar-proven source-target span rather than treating the
    // action verb as part of the reference surface.
    let authored = authored_name_text_for_span(info, span)?;
    let authored_tokens = crate::lexer::lex_line(&authored, info.line_index).ok()?;
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
pub(super) fn reconcile_named_source_action_surfaces(info: &LineInfo, effects: &mut [EffectAst]) {
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
        let tokens = crate::lexer::lex_line(&info.normalized.original, info.line_index).ok()?;
        crate::grammar::source_surface_shapes::parse_named_operand_nearest_to(
            &tokens,
            "exile",
            original_span.start,
        )
        .map(|shape| shape.surface)
    }

    fn named_surface_from_authored_counter_clause(
        info: &LineInfo,
    ) -> Option<crate::target::SourceReferenceSurface> {
        let tokens = crate::lexer::lex_line(&info.normalized.original, info.line_index).ok()?;
        crate::grammar::source_surface_shapes::parse_unique_named_counter_on_operand(&tokens)
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
        let tokens = crate::lexer::lex_line(&info.normalized.original, info.line_index).ok()?;
        if !tokens.iter().any(|token| token.is_word("exile")) {
            return None;
        }
        crate::grammar::source_surface_shapes::parse_unique_pronoun_operand_after(&tokens, "return")
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

        reconcile_named_source_action_surfaces(&info, &mut effects);

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

        reconcile_named_source_action_surfaces(&info, &mut effects);

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
