use crate::diagnostics::CardTextError;
use crate::model::provenance::{
    PunctuationKind, QuoteStyle, ReminderTextDecision, SourceSpan, SourceUnit, SourceUnitId,
};

use super::{MetadataLine, NormalizedLine, normalize_trimmed_line, parse_metadata_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfReferenceSurface {
    FullCardName,
    FaceName,
    This,
    ThisCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeMarker {
    Bullet,
    Hyphen,
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralLineKind {
    Blank,
    Metadata(MetadataLine),
    FaceSeparator,
    ReminderOnly,
    AbilityWord { label: String },
    SagaChapter { chapters: Vec<u32> },
    ClassLevel { level: u32 },
    LevelBand { min: u32, max: Option<u32> },
    ModalHeader,
    Mode { marker: ModeMarker },
    RulesText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralNodeKind {
    SelfReference(SelfReferenceSurface),
    Quotation(QuoteStyle),
    AbilityWord { label: String },
    ReminderText(ReminderTextDecision),
    Symbol,
    FaceSeparator,
    ChapterHeader { chapters: Vec<u32> },
    ClassHeader { level: u32 },
    LevelHeader { min: u32, max: Option<u32> },
    ModeMarker(ModeMarker),
    Punctuation(PunctuationKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralNode {
    pub kind: StructuralNodeKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedLine {
    pub line_index: usize,
    pub span: SourceSpan,
    pub content_span: Option<SourceSpan>,
    pub kind: StructuralLineKind,
    pub nodes: Vec<StructuralNode>,
    pub normalized: Option<NormalizedLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedFace {
    pub face_index: usize,
    pub span: SourceSpan,
    pub line_indices: std::ops::Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentStructure {
    pub source: SourceUnit,
    pub lines: Vec<ClassifiedLine>,
    pub faces: Vec<ClassifiedFace>,
}

impl DocumentStructure {
    pub fn reconstruct_source(&self) -> &str {
        self.source.text()
    }

    pub fn line(&self, line_index: usize) -> Option<&ClassifiedLine> {
        self.lines.iter().find(|line| line.line_index == line_index)
    }

    pub fn source_slice(&self, span: SourceSpan) -> Option<&str> {
        self.source.slice(span)
    }
}

pub fn classify_document_structure(
    unit: SourceUnitId,
    text: &str,
    card_name: &str,
) -> Result<DocumentStructure, CardTextError> {
    let source = SourceUnit::new(unit, text);
    let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let mut lines = Vec::with_capacity(line_count);

    for line_index in 0..line_count {
        let Some(span) = source.line_span(line_index) else {
            continue;
        };
        let authored = source.slice(span).unwrap_or_default();
        lines.push(classify_line(
            &source, line_index, span, authored, card_name,
        )?);
    }

    let faces = classify_faces(&source, &lines);
    Ok(DocumentStructure {
        source,
        lines,
        faces,
    })
}

fn classify_faces(source: &SourceUnit, lines: &[ClassifiedLine]) -> Vec<ClassifiedFace> {
    let mut faces = Vec::new();
    let mut face_start = 0usize;
    for (position, line) in lines.iter().enumerate() {
        if !matches!(&line.kind, StructuralLineKind::FaceSeparator) {
            continue;
        }
        push_face(source, lines, face_start, position, &mut faces);
        face_start = position + 1;
    }
    push_face(source, lines, face_start, lines.len(), &mut faces);
    if faces.is_empty()
        && let Some(span) = source.span(0..source.text().len())
    {
        faces.push(ClassifiedFace {
            face_index: 0,
            span,
            line_indices: 0..lines.len(),
        });
    }
    faces
}

fn push_face(
    source: &SourceUnit,
    lines: &[ClassifiedLine],
    start: usize,
    end: usize,
    faces: &mut Vec<ClassifiedFace>,
) {
    let Some(first) = lines.get(start) else {
        return;
    };
    let Some(last) = end.checked_sub(1).and_then(|position| lines.get(position)) else {
        return;
    };
    let Some(span) = source.span(first.span.start.byte..last.span.end.byte) else {
        return;
    };
    faces.push(ClassifiedFace {
        face_index: faces.len(),
        span,
        line_indices: first.line_index..last.line_index + 1,
    });
}

fn classify_line(
    source: &SourceUnit,
    line_index: usize,
    line_span: SourceSpan,
    authored: &str,
    card_name: &str,
) -> Result<ClassifiedLine, CardTextError> {
    let trimmed = trimmed_byte_range(authored);
    let normalized = normalize_trimmed_line(authored);
    let Some(trimmed) = trimmed else {
        return Ok(ClassifiedLine {
            line_index,
            span: line_span,
            content_span: None,
            kind: StructuralLineKind::Blank,
            nodes: Vec::new(),
            normalized,
        });
    };
    let text = &authored[trimmed.clone()];
    let mut nodes = structural_nodes(source, line_span, authored, card_name);

    let (kind, semantic_content) = if let Some(metadata) = parse_metadata_line(text)? {
        (
            StructuralLineKind::Metadata(metadata),
            Some(trimmed.clone()),
        )
    } else if text == "//" {
        push_node(
            &mut nodes,
            source,
            line_span,
            trimmed.clone(),
            StructuralNodeKind::FaceSeparator,
        );
        (StructuralLineKind::FaceSeparator, None)
    } else if is_fully_parenthetical(text) {
        (StructuralLineKind::ReminderOnly, None)
    } else if let Some((chapters, header, body)) = saga_chapter_prefix(text) {
        push_node(
            &mut nodes,
            source,
            line_span,
            offset_range(&trimmed, header),
            StructuralNodeKind::ChapterHeader {
                chapters: chapters.clone(),
            },
        );
        (
            StructuralLineKind::SagaChapter { chapters },
            Some(offset_range(&trimmed, body)),
        )
    } else if let Some((level, header, body)) = class_level_prefix(text) {
        push_node(
            &mut nodes,
            source,
            line_span,
            offset_range(&trimmed, header),
            StructuralNodeKind::ClassHeader { level },
        );
        (
            StructuralLineKind::ClassLevel { level },
            body.map(|body| offset_range(&trimmed, body)),
        )
    } else if let Some((min, max, header, body)) = level_band_prefix(text) {
        push_node(
            &mut nodes,
            source,
            line_span,
            offset_range(&trimmed, header),
            StructuralNodeKind::LevelHeader { min, max },
        );
        (
            StructuralLineKind::LevelBand { min, max },
            body.map(|body| offset_range(&trimmed, body)),
        )
    } else if let Some((marker, marker_range, body)) = mode_prefix(text) {
        push_node(
            &mut nodes,
            source,
            line_span,
            offset_range(&trimmed, marker_range),
            StructuralNodeKind::ModeMarker(marker),
        );
        (
            StructuralLineKind::Mode { marker },
            Some(offset_range(&trimmed, body)),
        )
    } else if is_modal_header(text) {
        (StructuralLineKind::ModalHeader, Some(trimmed.clone()))
    } else if let Some((label, label_range, body)) = ability_word_prefix(text) {
        push_node(
            &mut nodes,
            source,
            line_span,
            offset_range(&trimmed, label_range),
            StructuralNodeKind::AbilityWord {
                label: label.to_string(),
            },
        );
        (
            StructuralLineKind::AbilityWord {
                label: label.to_string(),
            },
            Some(offset_range(&trimmed, body)),
        )
    } else {
        (StructuralLineKind::RulesText, Some(trimmed.clone()))
    };

    nodes.sort_by_key(|node| (node.span.start.byte, node.span.end.byte));
    nodes.dedup();

    Ok(ClassifiedLine {
        line_index,
        span: line_span,
        content_span: semantic_content.and_then(|range| absolute_span(source, line_span, range)),
        kind,
        nodes,
        normalized,
    })
}

fn structural_nodes(
    source: &SourceUnit,
    line_span: SourceSpan,
    authored: &str,
    card_name: &str,
) -> Vec<StructuralNode> {
    let mut nodes = Vec::new();
    capture_delimited(authored, '(', ')', |range| {
        let decision = reminder_text_decision(&authored[range.clone()]);
        push_node(
            &mut nodes,
            source,
            line_span,
            range,
            StructuralNodeKind::ReminderText(decision),
        );
    });
    capture_delimited(authored, '{', '}', |range| {
        push_node(
            &mut nodes,
            source,
            line_span,
            range,
            StructuralNodeKind::Symbol,
        );
    });
    capture_quotes(authored, |range, style| {
        push_node(
            &mut nodes,
            source,
            line_span,
            range,
            StructuralNodeKind::Quotation(style),
        );
    });
    capture_self_references(authored, card_name, |range, surface| {
        push_node(
            &mut nodes,
            source,
            line_span,
            range,
            StructuralNodeKind::SelfReference(surface),
        );
    });
    capture_punctuation(authored, |range, punctuation| {
        push_node(
            &mut nodes,
            source,
            line_span,
            range,
            StructuralNodeKind::Punctuation(punctuation),
        );
    });
    for (start, _) in authored.match_indices(" // ") {
        push_node(
            &mut nodes,
            source,
            line_span,
            start..start + 4,
            StructuralNodeKind::FaceSeparator,
        );
    }
    nodes
}

fn trimmed_byte_range(text: &str) -> Option<std::ops::Range<usize>> {
    let start = text.find(|ch: char| !ch.is_whitespace())?;
    let last = text.rfind(|ch: char| !ch.is_whitespace())?;
    let end = last + text[last..].chars().next()?.len_utf8();
    Some(start..end)
}

fn absolute_span(
    source: &SourceUnit,
    line: SourceSpan,
    local: std::ops::Range<usize>,
) -> Option<SourceSpan> {
    source.span(line.start.byte + local.start..line.start.byte + local.end)
}

fn offset_range(
    outer: &std::ops::Range<usize>,
    inner: std::ops::Range<usize>,
) -> std::ops::Range<usize> {
    outer.start + inner.start..outer.start + inner.end
}

fn push_node(
    nodes: &mut Vec<StructuralNode>,
    source: &SourceUnit,
    line: SourceSpan,
    local: std::ops::Range<usize>,
    kind: StructuralNodeKind,
) {
    if let Some(span) = absolute_span(source, line, local) {
        nodes.push(StructuralNode { kind, span });
    }
}

fn is_fully_parenthetical(text: &str) -> bool {
    if !text.starts_with('(') || !text.ends_with(')') {
        return false;
    }
    let mut depth = 0u32;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && index + ch.len_utf8() != text.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn ability_word_prefix(
    text: &str,
) -> Option<(&str, std::ops::Range<usize>, std::ops::Range<usize>)> {
    let (dash_start, dash_len) = find_spaced_dash(text)?;
    let label = text[..dash_start].trim();
    if label.is_empty()
        || label.split_whitespace().count() > 6
        || !label.chars().any(char::is_alphabetic)
        || label
            .chars()
            .any(|ch| matches!(ch, '.' | ':' | ';' | '{' | '}'))
    {
        return None;
    }
    let label_start = text[..dash_start].find(label)?;
    let body_start = skip_whitespace(text, dash_start + dash_len);
    Some((
        label,
        label_start..label_start + label.len(),
        body_start..text.len(),
    ))
}

fn saga_chapter_prefix(
    text: &str,
) -> Option<(Vec<u32>, std::ops::Range<usize>, std::ops::Range<usize>)> {
    let (dash_start, dash_len) = find_spaced_dash(text)?;
    let header = text[..dash_start].trim();
    let chapters = header
        .split(',')
        .map(|part| roman_numeral(part.trim()))
        .collect::<Option<Vec<_>>>()?;
    if chapters.is_empty() {
        return None;
    }
    Some((
        chapters,
        0..dash_start,
        skip_whitespace(text, dash_start + dash_len)..text.len(),
    ))
}

fn class_level_prefix(
    text: &str,
) -> Option<(u32, std::ops::Range<usize>, Option<std::ops::Range<usize>>)> {
    let lower = text.to_ascii_lowercase();
    let prefix = lower.strip_prefix("class level ")?;
    let digit_len = prefix.bytes().take_while(u8::is_ascii_digit).count();
    let level = prefix[..digit_len].parse().ok()?;
    let header_end = "class level ".len() + digit_len;
    let body = delimiter_body(text, header_end);
    Some((level, 0..header_end, body))
}

fn level_band_prefix(
    text: &str,
) -> Option<(
    u32,
    Option<u32>,
    std::ops::Range<usize>,
    Option<std::ops::Range<usize>>,
)> {
    let lower = text.to_ascii_lowercase();
    let suffix = lower.strip_prefix("level ")?;
    let header_len = suffix
        .bytes()
        .take_while(|byte| byte.is_ascii_digit() || matches!(*byte, b'-' | b'+' | b' '))
        .count();
    let header_text = suffix[..header_len].trim();
    let (min, max) = if let Some((min, max)) = header_text.split_once('-') {
        (min.trim().parse().ok()?, Some(max.trim().parse().ok()?))
    } else if let Some(min) = header_text.strip_suffix('+') {
        (min.trim().parse().ok()?, None)
    } else {
        (header_text.parse().ok()?, Some(header_text.parse().ok()?))
    };
    let header_end = "level ".len() + header_len;
    Some((min, max, 0..header_end, delimiter_body(text, header_end)))
}

fn delimiter_body(text: &str, header_end: usize) -> Option<std::ops::Range<usize>> {
    let remainder = &text[header_end..];
    let delimiter = remainder.find([':', '—', '–'])?;
    let delimiter_byte = header_end + delimiter;
    let delimiter_len = text[delimiter_byte..].chars().next()?.len_utf8();
    let body_start = skip_whitespace(text, delimiter_byte + delimiter_len);
    (body_start < text.len()).then_some(body_start..text.len())
}

fn mode_prefix(text: &str) -> Option<(ModeMarker, std::ops::Range<usize>, std::ops::Range<usize>)> {
    let first = text.chars().next()?;
    let marker = match first {
        '•' => ModeMarker::Bullet,
        '-' => ModeMarker::Hyphen,
        '+' => ModeMarker::Plus,
        '−' => ModeMarker::Minus,
        _ => return None,
    };
    let end = first.len_utf8();
    let body_start = skip_whitespace(text, end);
    (body_start < text.len()).then_some((marker, 0..end, body_start..text.len()))
}

fn is_modal_header(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("choose ")
        && (text.ends_with(':') || text.ends_with('—') || text.ends_with('–'))
}

fn find_spaced_dash(text: &str) -> Option<(usize, usize)> {
    for (byte, ch) in text.char_indices() {
        if !matches!(ch, '—' | '–') {
            continue;
        }
        let before = text[..byte].chars().next_back();
        let after = text[byte + ch.len_utf8()..].chars().next();
        if before.is_some_and(char::is_whitespace) && after.is_some_and(char::is_whitespace) {
            return Some((byte, ch.len_utf8()));
        }
    }
    None
}

fn skip_whitespace(text: &str, mut byte: usize) -> usize {
    while let Some(ch) = text[byte..].chars().next()
        && ch.is_whitespace()
    {
        byte += ch.len_utf8();
    }
    byte
}

fn roman_numeral(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| matches!(byte, b'I' | b'V' | b'X')) {
        return None;
    }
    let values = text
        .bytes()
        .map(|byte| match byte {
            b'I' => 1,
            b'V' => 5,
            b'X' => 10,
            _ => 0,
        })
        .collect::<Vec<_>>();
    // Subtractive forms such as IV and IX necessarily dip below zero before
    // the following numeral is added. Keep the accumulator signed, then
    // convert only a positive final value back to the public representation.
    let mut total: i32 = 0;
    for (index, value) in values.iter().copied().enumerate() {
        if values.get(index + 1).is_some_and(|next| *next > value) {
            total -= value;
        } else {
            total += value;
        }
    }
    (total > 0).then(|| u32::try_from(total).ok()).flatten()
}

fn capture_delimited(
    text: &str,
    open: char,
    close: char,
    mut capture: impl FnMut(std::ops::Range<usize>),
) {
    let mut stack = Vec::new();
    for (byte, ch) in text.char_indices() {
        if ch == open {
            stack.push(byte);
        } else if ch == close
            && let Some(start) = stack.pop()
        {
            capture(start..byte + ch.len_utf8());
        }
    }
}

fn capture_quotes(text: &str, mut capture: impl FnMut(std::ops::Range<usize>, QuoteStyle)) {
    let mut straight = None;
    let mut curly = None;
    for (byte, ch) in text.char_indices() {
        match ch {
            '"' => {
                if let Some(start) = straight.take() {
                    capture(start..byte + ch.len_utf8(), QuoteStyle::Straight);
                } else {
                    straight = Some(byte);
                }
            }
            '“' => curly = Some(byte),
            '”' => {
                if let Some(start) = curly.take() {
                    capture(start..byte + ch.len_utf8(), QuoteStyle::Curly);
                }
            }
            _ => {}
        }
    }
}

fn capture_self_references(
    text: &str,
    card_name: &str,
    mut capture: impl FnMut(std::ops::Range<usize>, SelfReferenceSurface),
) {
    let mut names = Vec::new();
    let full_name = card_name.trim();
    if !full_name.is_empty() {
        names.push((full_name, SelfReferenceSurface::FullCardName));
    }
    for face in full_name.split(" // ").map(str::trim) {
        if !face.is_empty() && face != full_name {
            names.push((face, SelfReferenceSurface::FaceName));
        }
    }
    names.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));

    let mut byte = 0;
    while byte < text.len() {
        if !text.is_char_boundary(byte) {
            byte += 1;
            continue;
        }
        let mut matched = false;
        for (name, surface) in &names {
            let end = byte + name.len();
            if end <= text.len()
                && text.is_char_boundary(end)
                && text[byte..end].eq_ignore_ascii_case(name)
                && has_word_boundaries(text, byte, end)
            {
                capture(byte..end, *surface);
                byte = end;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let ch = text[byte..].chars().next().expect("character boundary");
        byte += ch.len_utf8();
    }

    capture_ascii_phrase(text, "this card", |range| {
        capture(range, SelfReferenceSurface::ThisCard)
    });
    capture_ascii_word(text, "this", |range| {
        if !text[range.end..]
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("card")
        {
            capture(range, SelfReferenceSurface::This);
        }
    });
}

fn capture_ascii_phrase(text: &str, phrase: &str, mut capture: impl FnMut(std::ops::Range<usize>)) {
    let lower = text.to_ascii_lowercase();
    for (start, _) in lower.match_indices(phrase) {
        let end = start + phrase.len();
        if has_word_boundaries(text, start, end) {
            capture(start..end);
        }
    }
}

fn capture_ascii_word(text: &str, word: &str, capture: impl FnMut(std::ops::Range<usize>)) {
    capture_ascii_phrase(text, word, capture)
}

fn has_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

fn capture_punctuation(
    text: &str,
    mut capture: impl FnMut(std::ops::Range<usize>, PunctuationKind),
) {
    for (byte, ch) in text.char_indices() {
        let kind = match ch {
            '.' => PunctuationKind::Period,
            ',' => PunctuationKind::Comma,
            ':' => PunctuationKind::Colon,
            ';' => PunctuationKind::Semicolon,
            '\'' | '’' | '‘' => PunctuationKind::Apostrophe,
            '"' | '“' | '”' => PunctuationKind::Quote,
            '(' | ')' => PunctuationKind::Parenthesis,
            '-' | '–' | '—' | '−' => PunctuationKind::Dash,
            '•' => PunctuationKind::Bullet,
            _ => continue,
        };
        capture(byte..byte + ch.len_utf8(), kind);
    }
}

fn reminder_text_decision(text: &str) -> ReminderTextDecision {
    let normalized = text.to_ascii_lowercase();
    if normalized.contains("it's not a creature") || normalized.contains("its not a creature") {
        ReminderTextDecision::TreatedAsRulesText
    } else if text.trim().starts_with('(') && text.trim().ends_with(')') {
        ReminderTextDecision::Preserved
    } else {
        ReminderTextDecision::ExcludedFromSemantics
    }
}

#[cfg(test)]
mod tests {
    use super::roman_numeral;

    #[test]
    fn roman_numerals_accept_subtractive_forms_without_unsigned_underflow() {
        assert_eq!(roman_numeral("IV"), Some(4));
        assert_eq!(roman_numeral("IX"), Some(9));
        assert_eq!(roman_numeral("XIV"), Some(14));
    }

    #[test]
    fn roman_numerals_reject_empty_and_non_roman_text() {
        assert_eq!(roman_numeral(""), None);
        assert_eq!(roman_numeral("XIY"), None);
    }
}
