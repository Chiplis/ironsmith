use std::cell::{Cell, Ref, RefCell, RefMut};

use crate::cards::builders::CardDefinitionBuilder;
use crate::diagnostics::TextSpan;
use crate::model::provenance::{ProvenanceId, ProvenanceStore};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceQuery, ReferenceRole, SymbolId, SymbolResolutionError,
    SymbolScopeId, SymbolScopeKind, SymbolTable,
};
use crate::types::{CardType, Subtype, Supertype};

pub use crate::model::provenance::SourceUnitId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseArenaId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdentity {
    pub unit: SourceUnitId,
    pub card_name: String,
    pub face_index: usize,
    pub source_len: usize,
    pub source_line_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardFaceMetadata {
    pub supertypes: Vec<Supertype>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub other_face_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseFeatures {
    pub allow_unsupported: bool,
    pub preserve_reminder_text: bool,
    pub capture_trace: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseScopeKind {
    Root,
    Document,
    Line { source_line: usize },
    NestedAbility,
    ModalMode { mode_index: usize },
    TokenDefinition,
    CleaveBranch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextDiagnostic {
    pub rule_path: Vec<String>,
    pub span: Option<TextSpan>,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ParseDiagnosticSink {
    diagnostics: RefCell<Vec<ContextDiagnostic>>,
}

impl ParseDiagnosticSink {
    pub fn push(&self, diagnostic: ContextDiagnostic) {
        self.diagnostics.borrow_mut().push(diagnostic);
    }

    pub fn snapshot(&self) -> Vec<ContextDiagnostic> {
        self.diagnostics.borrow().clone()
    }

    pub fn take(&self) -> Vec<ContextDiagnostic> {
        std::mem::take(&mut *self.diagnostics.borrow_mut())
    }
}

#[derive(Debug)]
pub struct ParseArenas {
    next_scope: Cell<u32>,
    next_provenance: Cell<u32>,
}

impl Default for ParseArenas {
    fn default() -> Self {
        Self {
            next_scope: Cell::new(1),
            next_provenance: Cell::new(0),
        }
    }
}

impl ParseArenas {
    fn allocate(cell: &Cell<u32>) -> ParseArenaId {
        let id = cell.get();
        cell.set(id.checked_add(1).expect("parse arena identifier overflow"));
        ParseArenaId(id)
    }

    pub fn allocate_scope(&self) -> ParseScopeId {
        let ParseArenaId(id) = Self::allocate(&self.next_scope);
        ParseScopeId(id)
    }

    pub fn allocate_provenance(&self) -> ProvenanceId {
        let ParseArenaId(id) = Self::allocate(&self.next_provenance);
        ProvenanceId(id)
    }
}

#[derive(Debug)]
pub struct ParseContext {
    source: SourceIdentity,
    card: CardFaceMetadata,
    features: ParseFeatures,
    diagnostics: ParseDiagnosticSink,
    arenas: ParseArenas,
    provenance: ProvenanceStore,
    symbols: RefCell<SymbolTable>,
}

impl ParseContext {
    pub fn new(source: SourceIdentity, card: CardFaceMetadata, features: ParseFeatures) -> Self {
        let provenance = ProvenanceStore::capture(source.unit, "", &source.card_name);
        let mut symbols = SymbolTable::default();
        let root_scope = symbols.root_scope();
        symbols
            .bind(
                root_scope,
                ReferenceRole::Source,
                Cardinality::ExactlyOne,
                ObjectDomain::Object,
                None,
            )
            .expect("root symbol scope must exist");
        Self {
            source,
            card,
            features,
            diagnostics: ParseDiagnosticSink::default(),
            arenas: ParseArenas::default(),
            provenance,
            symbols: RefCell::new(symbols),
        }
    }

    pub(crate) fn for_builder(
        builder: &CardDefinitionBuilder,
        text: &str,
        allow_unsupported: bool,
    ) -> Self {
        let mut context = Self::new(
            SourceIdentity {
                unit: SourceUnitId(0),
                card_name: builder.card_builder.name_ref().trim().to_string(),
                face_index: 0,
                source_len: text.len(),
                source_line_count: text.lines().count(),
            },
            CardFaceMetadata {
                supertypes: builder.card_builder.supertypes_ref().to_vec(),
                card_types: builder.card_builder.card_types_ref().to_vec(),
                subtypes: builder.card_builder.subtypes_ref().to_vec(),
                other_face_name: None,
            },
            ParseFeatures {
                allow_unsupported,
                preserve_reminder_text: false,
                capture_trace: crate::parse_trace::is_enabled(),
            },
        );
        context.provenance =
            ProvenanceStore::capture(context.source.unit, text, &context.source.card_name);
        context
    }

    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }

    pub fn card(&self) -> &CardFaceMetadata {
        &self.card
    }

    pub fn features(&self) -> ParseFeatures {
        self.features
    }

    pub fn diagnostics(&self) -> &ParseDiagnosticSink {
        &self.diagnostics
    }

    pub fn arenas(&self) -> &ParseArenas {
        &self.arenas
    }

    pub fn provenance(&self) -> &ProvenanceStore {
        &self.provenance
    }

    pub(crate) fn replace_provenance(&mut self, provenance: ProvenanceStore) {
        self.provenance = provenance;
    }

    pub fn symbols(&self) -> Ref<'_, SymbolTable> {
        self.symbols.borrow()
    }

    pub fn symbols_mut(&self) -> RefMut<'_, SymbolTable> {
        self.symbols.borrow_mut()
    }

    pub fn view(&self) -> ParseContextView<'_> {
        ParseContextView {
            source: &self.source,
            card: &self.card,
            features: self.features,
            diagnostics: &self.diagnostics,
            arenas: &self.arenas,
            provenance: &self.provenance,
            symbols: &self.symbols,
            symbol_scope: SymbolScopeId(0),
            scope: ParseScopeId(0),
            scope_kind: ParseScopeKind::Root,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParseContextView<'a> {
    source: &'a SourceIdentity,
    card: &'a CardFaceMetadata,
    features: ParseFeatures,
    diagnostics: &'a ParseDiagnosticSink,
    arenas: &'a ParseArenas,
    provenance: &'a ProvenanceStore,
    symbols: &'a RefCell<SymbolTable>,
    symbol_scope: SymbolScopeId,
    scope: ParseScopeId,
    scope_kind: ParseScopeKind,
}

impl<'a> ParseContextView<'a> {
    pub fn source(self) -> &'a SourceIdentity {
        self.source
    }

    pub fn card(self) -> &'a CardFaceMetadata {
        self.card
    }

    pub fn features(self) -> ParseFeatures {
        self.features
    }

    pub fn diagnostics(self) -> &'a ParseDiagnosticSink {
        self.diagnostics
    }

    pub fn arenas(self) -> &'a ParseArenas {
        self.arenas
    }

    pub fn provenance(self) -> &'a ProvenanceStore {
        self.provenance
    }

    pub fn symbol_scope(self) -> SymbolScopeId {
        self.symbol_scope
    }

    pub fn symbols(self) -> Ref<'a, SymbolTable> {
        self.symbols.borrow()
    }

    pub fn bind_symbol(
        self,
        role: ReferenceRole,
        cardinality: Cardinality,
        domain: ObjectDomain,
        provenance: Option<ProvenanceId>,
    ) -> Result<SymbolId, SymbolResolutionError> {
        self.symbols
            .borrow_mut()
            .bind(self.symbol_scope, role, cardinality, domain, provenance)
    }

    pub fn resolve_symbol(
        self,
        role: ReferenceRole,
        domain: ObjectDomain,
        required_cardinality: Option<Cardinality>,
    ) -> Result<SymbolId, SymbolResolutionError> {
        self.symbols.borrow().resolve(ReferenceQuery {
            scope: self.symbol_scope,
            role,
            domain,
            required_cardinality,
        })
    }

    pub fn scope(self) -> ParseScopeId {
        self.scope
    }

    pub fn scope_kind(self) -> ParseScopeKind {
        self.scope_kind
    }

    pub fn child(self, scope_kind: ParseScopeKind) -> Self {
        let symbol_scope_kind = match scope_kind {
            ParseScopeKind::Root => SymbolScopeKind::Root,
            ParseScopeKind::Document => SymbolScopeKind::Document,
            ParseScopeKind::Line { .. } => SymbolScopeKind::Line,
            ParseScopeKind::NestedAbility => SymbolScopeKind::NestedAbility,
            ParseScopeKind::ModalMode { .. } => SymbolScopeKind::ModalMode,
            ParseScopeKind::TokenDefinition => SymbolScopeKind::TokenDefinition,
            ParseScopeKind::CleaveBranch => SymbolScopeKind::Branch,
        };
        let symbol_scope = self
            .symbols
            .borrow_mut()
            .create_scope(self.symbol_scope, symbol_scope_kind)
            .expect("parent parse symbol scope must exist");
        Self {
            scope: self.arenas.allocate_scope(),
            scope_kind,
            symbol_scope,
            ..self
        }
    }

    pub fn record_diagnostic<I, S>(
        self,
        rule_path: I,
        span: Option<TextSpan>,
        message: impl Into<String>,
    ) where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.diagnostics.push(ContextDiagnostic {
            rule_path: rule_path.into_iter().map(Into::into).collect(),
            span,
            message: message.into(),
        });
    }
}
