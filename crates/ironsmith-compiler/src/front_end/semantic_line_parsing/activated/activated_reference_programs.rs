use super::*;

fn named_surface_for_span(
    info: &LineInfo,
    span: crate::cards::builders::TextSpan,
) -> Option<crate::target::SourceReferenceSurface> {
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
    // Some legacy target spans include the action verb (and a historical
    // source-map can begin one byte into it). Select the authored proper name
    // inside that grammar-proven source-target span rather than treating the
    // action verb as part of the reference surface.
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
    let authored = crate::string_primitives::split_once(&authored_span[authored_start..], " and ")
        .map_or(&authored_span[authored_start..], |(source, _)| source)
        .trim_matches(|ch: char| ch.is_whitespace() || ch == ',' || ch == '.');
    let authored_tokens = crate::lexer::lex_line(authored, info.line_index).ok()?;
    // Source maps produced before the final comma/`then` split can retain a
    // few bytes from the following clause (for example `Jace, th`). Reapply
    // the grammar boundary to the mapped slice before accepting it as a name.
    let name_end = authored_tokens
        .iter()
        .position(|token| {
            matches!(
                token.kind,
                crate::lexer::TokenKind::Comma
                    | crate::lexer::TokenKind::Period
                    | crate::lexer::TokenKind::Semicolon
            ) || token.is_word("then")
        })
        .unwrap_or(authored_tokens.len());
    let name_tokens = authored_tokens.get(..name_end)?;
    if !crate::lexer::is_authored_proper_name_phrase(name_tokens) {
        return None;
    }
    let words = crate::lexer::parser_token_word_refs(name_tokens);
    let authored = crate::lexer::render_token_slice(name_tokens)
        .trim()
        .to_string();
    Some(if words.len() == 1 {
        crate::target::SourceReferenceSurface::ShortName(authored)
    } else {
        crate::target::SourceReferenceSurface::FullName(authored)
    })
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
        let (_, name_tokens) = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| token.is_word("exile"))
            .filter_map(|(exile_index, exile)| {
                let start = exile_index + 1;
                let end = tokens[start..]
                    .iter()
                    .position(|token| {
                        matches!(
                            token.kind,
                            crate::lexer::TokenKind::Comma
                                | crate::lexer::TokenKind::Period
                                | crate::lexer::TokenKind::Semicolon
                        ) || token.is_word("then")
                    })
                    .map_or(tokens.len(), |offset| start + offset);
                let candidate = tokens.get(start..end)?;
                crate::lexer::is_authored_proper_name_phrase(candidate)
                    .then_some((exile.span.start.abs_diff(original_span.start), candidate))
            })
            .min_by_key(|(distance, _)| *distance)?;
        let authored = crate::lexer::render_token_slice(name_tokens)
            .trim()
            .to_string();
        let words = crate::lexer::parser_token_word_refs(name_tokens);
        Some(if words.len() == 1 {
            crate::target::SourceReferenceSurface::ShortName(authored)
        } else {
            crate::target::SourceReferenceSurface::FullName(authored)
        })
    }

    fn named_surface_from_authored_counter_clause(
        info: &LineInfo,
    ) -> Option<crate::target::SourceReferenceSurface> {
        let tokens = crate::lexer::lex_line(&info.normalized.original, info.line_index).ok()?;
        let words = crate::lexer::TokenWordView::new(&tokens);
        let mut surfaces = (0..words.len().saturating_sub(1))
            .filter(|index| {
                words.parses_any_word_at(*index, &["counter", "counters"])
                    && words.parses_word_at(*index + 1, "on")
            })
            .filter_map(|counter_index| {
                let start = words.map_word_to_token_boundary(counter_index + 2)?;
                let end = tokens[start..]
                    .iter()
                    .position(|token| {
                        matches!(
                            token.kind,
                            crate::lexer::TokenKind::Comma
                                | crate::lexer::TokenKind::Period
                                | crate::lexer::TokenKind::Semicolon
                        ) || token.is_word("then")
                    })
                    .map_or(tokens.len(), |offset| start + offset);
                let name_tokens = tokens.get(start..end)?;
                if !crate::lexer::is_authored_proper_name_phrase(name_tokens) {
                    return None;
                }
                let authored = crate::lexer::render_token_slice(name_tokens)
                    .trim()
                    .to_string();
                Some(
                    if crate::lexer::parser_token_word_refs(name_tokens).len() == 1 {
                        crate::target::SourceReferenceSurface::ShortName(authored)
                    } else {
                        crate::target::SourceReferenceSurface::FullName(authored)
                    },
                )
            })
            .collect::<Vec<_>>();
        surfaces.dedup();
        let [surface] = surfaces.as_slice() else {
            return None;
        };
        Some(surface.clone())
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
        let mut surfaces = tokens
            .windows(2)
            .filter(|pair| pair[0].is_word("return"))
            .filter_map(|pair| {
                matches!(
                    pair[1].slice.to_ascii_lowercase().as_str(),
                    "him" | "her" | "it"
                )
                .then(|| {
                    crate::target::SourceReferenceSurface::ThisPermanentType(pair[1].slice.clone())
                })
            })
            .collect::<Vec<_>>();
        surfaces.dedup();
        let [surface] = surfaces.as_slice() else {
            return None;
        };
        Some(surface.clone())
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
            normalized: crate::cards::builders::NormalizedLine {
                original: raw.to_string(),
                normalized: raw.to_ascii_lowercase(),
                char_map: Vec::new(),
            },
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
