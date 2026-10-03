use super::{NormalizedLine, map_span_to_original};
use crate::diagnostics::TextSpan;

fn mapped(line: &NormalizedLine, start: usize, end: usize) -> TextSpan {
    let span = map_span_to_original(
        TextSpan {
            line: 7,
            start,
            end,
        },
        &line.normalized,
        &line.original,
        &line.char_map,
    );
    assert_eq!(span.line, 7);
    assert!(span.start <= span.end);
    assert!(line.original.get(span.start..span.end).is_some());
    span
}

#[test]
fn character_map_values_are_scalar_indexes_not_utf8_offsets() {
    let line = NormalizedLine::identity("Aï🙂Z");
    for (start, ch) in line.normalized.char_indices() {
        let end = start + ch.len_utf8();
        assert_eq!(
            mapped(&line, start, end),
            TextSpan {
                line: 7,
                start,
                end
            }
        );
    }
    assert_eq!(
        mapped(&line, 1, 7),
        TextSpan {
            line: 7,
            start: 1,
            end: 7
        }
    );
}

#[test]
fn multibyte_whitespace_normalization_preserves_token_and_line_spans() {
    let line = crate::preprocess::normalize_trimmed_line("\u{2003}É \u{2002} ç\t ")
        .expect("nonempty normalized line");
    assert_eq!(line.normalized, "É ç");
    for (start, end, expected) in [(0, 2, "É"), (3, 5, "ç"), (0, 5, "É \u{2002} ç")] {
        let span = mapped(&line, start, end);
        assert_eq!(&line.original[span.start..span.end], expected);
    }
    let source_end = line.original.find('ç').unwrap() + 'ç'.len_utf8();
    let eof = mapped(&line, line.normalized.len(), line.normalized.len());
    assert_eq!(eof.start, source_end);
    assert_eq!(eof.end, source_end);
    let beginning = mapped(&line, 0, 0);
    assert_eq!(beginning.start, '\u{2003}'.len_utf8());
    assert_eq!(beginning.end, beginning.start);
}

#[test]
fn expanding_unicode_case_mapping_retains_original_character() {
    let original = "İstanbul";
    let normalized = original.to_lowercase();
    let line = NormalizedLine::from_char_map(original, normalized, vec![0, 0, 1, 2, 3, 4, 5, 6, 7]);
    for (start, end) in [(0, 1), (1, 3), (0, 3)] {
        let span = mapped(&line, start, end);
        assert_eq!(&line.original[span.start..span.end], "İ");
    }
    let remainder = mapped(&line, 3, line.normalized.len());
    assert_eq!(&line.original[remainder.start..remainder.end], "stanbul");
}

#[test]
fn byte_boundaries_are_rounded_outward_but_empty_spans_stay_empty() {
    let line = NormalizedLine::identity("Aï🙂Z");
    for (start, end, expected_start, expected_end) in [
        (2, 5, 1, 7),
        (3, 6, 3, 7),
        (2, 2, 1, 1),
        (5, 2, 3, 3),
        (8, 8, 8, 8),
        (usize::MAX, usize::MAX, 8, 8),
        (1, usize::MAX, 1, 8),
    ] {
        assert_eq!(
            mapped(&line, start, end),
            TextSpan {
                line: 7,
                start: expected_start,
                end: expected_end,
            }
        );
    }
}

#[test]
fn empty_normalized_and_original_lines_have_a_valid_empty_anchor() {
    for original in ["", "é"] {
        let line = NormalizedLine::from_char_map(original, "", vec![]);
        assert_eq!(
            mapped(&line, 0, 0),
            TextSpan {
                line: 7,
                start: 0,
                end: 0
            }
        );
        assert_eq!(
            mapped(&line, 1, usize::MAX),
            TextSpan {
                line: 7,
                start: 0,
                end: 0
            }
        );
    }
}

#[test]
fn removed_text_after_a_token_is_not_included_in_its_source_span() {
    let line = NormalizedLine::from_char_map("É (reminder) ç", "É ç", vec![0, 1, 13]);
    let first = mapped(&line, 0, 2);
    assert_eq!(&line.original[first.start..first.end], "É");
    let last = mapped(&line, 3, 5);
    assert_eq!(&line.original[last.start..last.end], "ç");
}
