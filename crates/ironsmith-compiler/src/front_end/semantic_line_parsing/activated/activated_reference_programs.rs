use super::*;

/// Restore an authored named-source surface on source exile actions after
/// source-reference normalization has turned the semantic target into
/// `TargetAst::Source`. The retained span maps back to the original line, so
/// this stays structural and also covers short aliases on other activated
/// abilities without depending on a card name.
pub(super) fn reconcile_named_source_exile_surfaces(info: &LineInfo, effects: &mut [EffectAst]) {
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
        // Some legacy target spans include the exile verb (and a historical
        // source-map can begin one byte into it). Select the authored proper
        // name inside that grammar-proven source-target span rather than
        // treating the action verb as part of the reference surface.
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
                (!word.eq_ignore_ascii_case("exile")).then_some(index)
            })?;
        let authored =
            crate::string_primitives::split_once(&authored_span[authored_start..], " and ")
                .map_or(&authored_span[authored_start..], |(source, _)| source)
                .trim_matches(|ch: char| ch.is_whitespace() || ch == ',' || ch == '.');
        let authored_tokens = crate::lexer::lex_line(authored, info.line_index).ok()?;
        let words = crate::lexer::parser_token_word_refs(&authored_tokens);
        if words.is_empty()
            || words
                .first()
                .is_some_and(|word| matches!(*word, "this" | "it"))
            || !authored.chars().next().is_some_and(char::is_uppercase)
        {
            return None;
        }
        Some(if words.len() == 1 {
            crate::target::SourceReferenceSurface::ShortName(authored.to_string())
        } else {
            crate::target::SourceReferenceSurface::FullName(authored.to_string())
        })
    }

    fn apply_target(info: &LineInfo, target: &mut TargetAst) {
        let TargetAst::Source(Some(span)) = target else {
            return;
        };
        let Some(surface) = named_surface_for_span(info, *span) else {
            return;
        };
        *target = TargetAst::Object(
            ObjectFilter::source_with_surface(surface),
            None,
            Some(*span),
        );
    }

    fn apply(info: &LineInfo, effects: &mut [EffectAst]) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                match &mut subject_verb.action {
                    SubjectVerbActionAst::Exile { target, .. } => apply_target(info, target),
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
}
