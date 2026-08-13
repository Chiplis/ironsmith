use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceUnitId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProvenanceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    pub byte: usize,
    pub character: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub unit: SourceUnitId,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub fn byte_range(self) -> Range<usize> {
        self.start.byte..self.end.byte
    }

    pub fn is_empty(self) -> bool {
        self.start.byte == self.end.byte
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceSliceKind {
    FullDocument,
    OracleLine { line: usize },
    CardName,
    SelfReference,
    Quotation,
    Parenthetical,
    ReminderText,
    Symbol,
    FaceSeparator,
    AbilityWord,
    ChapterHeader,
    ClassHeader,
    LevelHeader,
    ModeMarker,
    Punctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReminderTextDecision {
    NotReminderText,
    Preserved,
    ExcludedFromSemantics,
    TreatedAsRulesText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PunctuationKind {
    Period,
    Comma,
    Colon,
    Semicolon,
    Apostrophe,
    Quote,
    Parenthesis,
    Dash,
    Bullet,
    Other(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuoteStyle {
    Straight,
    Curly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DashStyle {
    Hyphen,
    EnDash,
    EmDash,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderingHint {
    PreserveCapitalization,
    PreserveWhitespace,
    Punctuation(PunctuationKind),
    QuoteStyle(QuoteStyle),
    DashStyle(DashStyle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceRecord {
    pub id: ProvenanceId,
    pub kind: SourceSliceKind,
    pub span: Option<SourceSpan>,
    pub authored: String,
    pub normalized: Option<String>,
    pub normalized_to_source_characters: Vec<usize>,
    pub reminder_text: ReminderTextDecision,
    pub rendering_hints: Vec<RenderingHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub id: SourceUnitId,
    text: String,
    line_byte_starts: Vec<usize>,
}

impl SourceUnit {
    pub fn new(id: SourceUnitId, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_byte_starts = vec![0];
        for (byte, ch) in text.char_indices() {
            if ch == '\n' {
                line_byte_starts.push(byte + ch.len_utf8());
            }
        }
        Self {
            id,
            text,
            line_byte_starts,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn span(&self, bytes: Range<usize>) -> Option<SourceSpan> {
        if bytes.start > bytes.end
            || bytes.end > self.text.len()
            || !self.text.is_char_boundary(bytes.start)
            || !self.text.is_char_boundary(bytes.end)
        {
            return None;
        }
        Some(SourceSpan {
            unit: self.id,
            start: SourcePosition {
                byte: bytes.start,
                character: self.text[..bytes.start].chars().count(),
            },
            end: SourcePosition {
                byte: bytes.end,
                character: self.text[..bytes.end].chars().count(),
            },
        })
    }

    pub fn line_span(&self, line: usize) -> Option<SourceSpan> {
        let start = *self.line_byte_starts.get(line)?;
        let mut end = self
            .line_byte_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        while end > start && matches!(self.text.as_bytes()[end - 1], b'\n' | b'\r') {
            end -= 1;
        }
        self.span(start..end)
    }

    pub fn slice(&self, span: SourceSpan) -> Option<&str> {
        (span.unit == self.id)
            .then(|| self.text.get(span.byte_range()))
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceStore {
    source: SourceUnit,
    records: Vec<ProvenanceRecord>,
    next_id: u32,
}

impl ProvenanceStore {
    pub fn capture(unit: SourceUnitId, text: &str, card_name: &str) -> Self {
        let source = SourceUnit::new(unit, text);
        let mut store = Self {
            source,
            records: Vec::new(),
            next_id: 0,
        };
        let document_span = store.source.span(0..text.len());
        store.push_record(
            SourceSliceKind::FullDocument,
            document_span,
            text.to_string(),
            None,
            Vec::new(),
            ReminderTextDecision::NotReminderText,
            vec![RenderingHint::PreserveWhitespace],
        );
        store.push_record(
            SourceSliceKind::CardName,
            None,
            card_name.to_string(),
            None,
            Vec::new(),
            ReminderTextDecision::NotReminderText,
            vec![RenderingHint::PreserveCapitalization],
        );
        for (_, separator) in card_name.match_indices(" // ") {
            store.push_record(
                SourceSliceKind::FaceSeparator,
                None,
                separator.to_string(),
                None,
                Vec::new(),
                ReminderTextDecision::NotReminderText,
                vec![RenderingHint::PreserveWhitespace],
            );
        }
        for line in 0..store.source.line_byte_starts.len() {
            if let Some(span) = store.source.line_span(line) {
                let authored = store.source.slice(span).unwrap_or_default().to_string();
                store.push_record(
                    SourceSliceKind::OracleLine { line },
                    Some(span),
                    authored.clone(),
                    None,
                    Vec::new(),
                    ReminderTextDecision::NotReminderText,
                    rendering_hints(&authored),
                );
            }
        }
        store.capture_structural_boundaries();
        store
    }

    pub fn source(&self) -> &SourceUnit {
        &self.source
    }

    pub fn records(&self) -> &[ProvenanceRecord] {
        &self.records
    }

    pub fn view(&self) -> ProvenanceView<'_> {
        ProvenanceView { store: self }
    }

    pub fn get(&self, id: ProvenanceId) -> Option<&ProvenanceRecord> {
        self.records
            .get(id.0 as usize)
            .filter(|record| record.id == id)
    }

    pub fn record_normalized_line(
        &mut self,
        line: usize,
        authored: &str,
        normalized: &str,
        normalized_to_source_characters: &[usize],
    ) -> ProvenanceId {
        let span = self.source.line_span(line);
        self.push_record(
            SourceSliceKind::OracleLine { line },
            span,
            authored.to_string(),
            Some(normalized.to_string()),
            normalized_to_source_characters.to_vec(),
            reminder_text_decision(authored),
            rendering_hints(authored),
        )
    }

    pub fn record_structural_span(
        &mut self,
        kind: SourceSliceKind,
        span: SourceSpan,
        reminder_text: ReminderTextDecision,
    ) -> Option<ProvenanceId> {
        let authored = self.source.slice(span)?.to_string();
        Some(self.push_record(
            kind,
            Some(span),
            authored.clone(),
            None,
            Vec::new(),
            reminder_text,
            rendering_hints(&authored),
        ))
    }

    fn capture_structural_boundaries(&mut self) {
        let text = self.source.text().to_string();
        capture_delimited(&text, '(', ')', |range| {
            self.record_boundary(SourceSliceKind::Parenthetical, range)
        });
        capture_delimited(&text, '{', '}', |range| {
            self.record_boundary(SourceSliceKind::Symbol, range)
        });
        capture_quotes(&text, |range| {
            self.record_boundary(SourceSliceKind::Quotation, range)
        });
        for (start, _) in text.match_indices(" // ") {
            self.record_boundary(SourceSliceKind::FaceSeparator, start..start + 4);
        }
    }

    fn record_boundary(&mut self, kind: SourceSliceKind, bytes: Range<usize>) {
        let Some(span) = self.source.span(bytes) else {
            return;
        };
        let authored = self.source.slice(span).unwrap_or_default().to_string();
        let reminder_text = if kind == SourceSliceKind::Parenthetical {
            reminder_text_decision(&authored)
        } else {
            ReminderTextDecision::NotReminderText
        };
        self.push_record(
            kind,
            Some(span),
            authored.clone(),
            None,
            Vec::new(),
            reminder_text,
            rendering_hints(&authored),
        );
    }

    fn push_record(
        &mut self,
        kind: SourceSliceKind,
        span: Option<SourceSpan>,
        authored: String,
        normalized: Option<String>,
        normalized_to_source_characters: Vec<usize>,
        reminder_text: ReminderTextDecision,
        rendering_hints: Vec<RenderingHint>,
    ) -> ProvenanceId {
        let id = ProvenanceId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("provenance identifier overflow");
        self.records.push(ProvenanceRecord {
            id,
            kind,
            span,
            authored,
            normalized,
            normalized_to_source_characters,
            reminder_text,
            rendering_hints,
        });
        id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProvenance {
    pub primary: ProvenanceId,
    pub related: Vec<ProvenanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenanced<T> {
    pub value: T,
    pub provenance: SemanticProvenance,
}

impl<T> Provenanced<T> {
    pub fn new(value: T, primary: ProvenanceId) -> Self {
        Self {
            value,
            provenance: SemanticProvenance {
                primary,
                related: Vec::new(),
            },
        }
    }

    pub fn with_related(mut self, related: impl IntoIterator<Item = ProvenanceId>) -> Self {
        self.provenance.related.extend(related);
        self
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Provenanced<U> {
        Provenanced {
            value: map(self.value),
            provenance: self.provenance,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProvenanceView<'a> {
    store: &'a ProvenanceStore,
}

impl<'a> ProvenanceView<'a> {
    pub fn get(self, id: ProvenanceId) -> Option<&'a ProvenanceRecord> {
        self.store.get(id)
    }

    pub fn source(self) -> &'a SourceUnit {
        self.store.source()
    }

    pub fn slice(self, id: ProvenanceId) -> Option<&'a str> {
        let record = self.get(id)?;
        record.span.and_then(|span| self.source().slice(span))
    }
}

fn capture_delimited(text: &str, open: char, close: char, mut capture: impl FnMut(Range<usize>)) {
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

fn capture_quotes(text: &str, mut capture: impl FnMut(Range<usize>)) {
    let mut open = None;
    for (byte, ch) in text.char_indices() {
        if matches!(ch, '"' | '“' | '”') {
            if let Some(start) = open.take() {
                capture(start..byte + ch.len_utf8());
            } else {
                open = Some(byte);
            }
        }
    }
}

fn rendering_hints(text: &str) -> Vec<RenderingHint> {
    let mut hints = Vec::new();
    if text.chars().any(char::is_uppercase) {
        hints.push(RenderingHint::PreserveCapitalization);
    }
    if text.chars().any(char::is_whitespace) {
        hints.push(RenderingHint::PreserveWhitespace);
    }
    for ch in text.chars() {
        let hint = match ch {
            '.' => Some(RenderingHint::Punctuation(PunctuationKind::Period)),
            ',' => Some(RenderingHint::Punctuation(PunctuationKind::Comma)),
            ':' => Some(RenderingHint::Punctuation(PunctuationKind::Colon)),
            ';' => Some(RenderingHint::Punctuation(PunctuationKind::Semicolon)),
            '\'' | '’' | '‘' => Some(RenderingHint::Punctuation(PunctuationKind::Apostrophe)),
            '"' => Some(RenderingHint::QuoteStyle(QuoteStyle::Straight)),
            '“' | '”' => Some(RenderingHint::QuoteStyle(QuoteStyle::Curly)),
            '-' => Some(RenderingHint::DashStyle(DashStyle::Hyphen)),
            '–' => Some(RenderingHint::DashStyle(DashStyle::EnDash)),
            '—' => Some(RenderingHint::DashStyle(DashStyle::EmDash)),
            '−' => Some(RenderingHint::DashStyle(DashStyle::Minus)),
            '•' => Some(RenderingHint::Punctuation(PunctuationKind::Bullet)),
            _ => None,
        };
        if let Some(hint) = hint
            && !hints.contains(&hint)
        {
            hints.push(hint);
        }
    }
    hints
}

fn reminder_text_decision(text: &str) -> ReminderTextDecision {
    if !text.contains('(') || !text.contains(')') {
        return ReminderTextDecision::NotReminderText;
    }
    let normalized = text.to_ascii_lowercase();
    if normalized.contains("it's not a creature") || normalized.contains("its not a creature") {
        ReminderTextDecision::TreatedAsRulesText
    } else if text.trim().starts_with('(') && text.trim().ends_with(')') {
        ReminderTextDecision::Preserved
    } else {
        ReminderTextDecision::ExcludedFromSemantics
    }
}
