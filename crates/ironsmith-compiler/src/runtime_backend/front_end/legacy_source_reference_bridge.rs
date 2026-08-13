//! PR-02 compatibility bridge for parser code that still reads hidden source state.
//!
//! Canonical entry points install this state from an explicit `ParseContext`.
//! Leaf consumers migrate off these functions in later parser checkpoints and
//! PR-33 deletes this module.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::cards::TextSpan;
use crate::parse_context::ParseContext;
use crate::target::{SacrificedObjectKind, SourceReferenceSurface};
use crate::types::{CardType, Subtype};

use super::grammar::leaf;

type SourceReferenceAlias = leaf::LeafSourceReferenceAlias;

#[derive(Clone, Default)]
struct SourceReferenceContext {
    source_name: String,
    aliases: Vec<SourceReferenceAlias>,
    preferred_self_surface: Option<SourceReferenceSurface>,
    surfaces_by_span: HashMap<TextSpan, SourceReferenceSurface>,
    sacrificed_kinds_by_span: HashMap<TextSpan, SacrificedObjectKind>,
}

thread_local! {
    static SOURCE_REFERENCE_CONTEXT: RefCell<SourceReferenceContext> =
        RefCell::new(SourceReferenceContext::default());
}

pub(crate) fn with_parse_context<T>(
    context: &mut ParseContext,
    run: impl FnOnce(&mut ParseContext) -> T,
) -> T {
    let view = context.view();
    let source_name = view.source().card_name.clone();
    let card_types = view.card().card_types.clone();
    let subtypes = view.card().subtypes.clone();
    with_card_identity(&source_name, &card_types, &subtypes, || run(context))
}

pub(crate) fn with_name<T>(card_name: &str, run: impl FnOnce() -> T) -> T {
    with_aliases(card_name, Vec::new(), run)
}

pub(crate) fn with_card_identity<T>(
    card_name: &str,
    card_types: &[CardType],
    subtypes: &[Subtype],
    run: impl FnOnce() -> T,
) -> T {
    with_aliases(
        card_name,
        aliases_for_card_identity(card_types, subtypes),
        run,
    )
}

pub(crate) fn with_token_identity<T>(
    token_name: &str,
    card_types: &[CardType],
    subtypes: &[Subtype],
    run: impl FnOnce() -> T,
) -> T {
    let mut aliases = Vec::new();
    push_alias_words(
        &mut aliases,
        vec!["this".to_string(), "token".to_string()],
        SourceReferenceSurface::ThisPermanentType("this token".to_string()),
    );
    for alias in aliases_for_card_identity(card_types, subtypes) {
        push_alias_words(&mut aliases, alias.words, alias.surface);
    }
    with_aliases(token_name, aliases, run)
}

fn with_aliases<T>(
    card_name: &str,
    extra_aliases: Vec<SourceReferenceAlias>,
    run: impl FnOnce() -> T,
) -> T {
    let preferred_self_surface = extra_aliases.first().map(|alias| alias.surface.clone());
    let mut aliases = aliases_for_name(card_name);
    for alias in extra_aliases {
        push_alias_words(&mut aliases, alias.words, alias.surface);
    }
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.words.len()));
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let previous = context.replace(SourceReferenceContext {
            source_name: card_name.trim().to_string(),
            aliases,
            preferred_self_surface,
            surfaces_by_span: HashMap::new(),
            sacrificed_kinds_by_span: HashMap::new(),
        });
        let result = run();
        context.replace(previous);
        result
    })
}

pub(crate) fn current_name() -> Option<String> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let source_name = context.borrow().source_name.trim().to_string();
        (!source_name.is_empty()).then_some(source_name)
    })
}

pub(crate) fn preferred_self_surface() -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| context.borrow().preferred_self_surface.clone())
}

pub(crate) fn surface_for_span(span: Option<TextSpan>) -> Option<SourceReferenceSurface> {
    let span = span?;
    SOURCE_REFERENCE_CONTEXT.with(|context| context.borrow().surfaces_by_span.get(&span).cloned())
}

pub(crate) fn record_surface(span: Option<TextSpan>, surface: SourceReferenceSurface) {
    let Some(span) = span else {
        return;
    };
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let surface = canonical_surface(&context.aliases, surface);
        context.surfaces_by_span.insert(span, surface);
    });
}

pub(crate) fn sacrificed_kind_for_span(span: Option<TextSpan>) -> Option<SacrificedObjectKind> {
    let span = span?;
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        context
            .borrow()
            .sacrificed_kinds_by_span
            .get(&span)
            .copied()
    })
}

pub(crate) fn record_sacrificed_kind(span: Option<TextSpan>, kind: SacrificedObjectKind) {
    let Some(span) = span else {
        return;
    };
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        context
            .borrow_mut()
            .sacrificed_kinds_by_span
            .insert(span, kind);
    });
}

pub(crate) fn surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        leaf::parse_leaf_source_reference_alias_words(&context.borrow().aliases, words)
    })
}

pub(crate) fn surface_for_possessive_words(words: &[&str]) -> Option<SourceReferenceSurface> {
    SOURCE_REFERENCE_CONTEXT.with(|context| {
        leaf::parse_leaf_source_reference_possessive_alias_words(&context.borrow().aliases, words)
    })
}

pub(crate) fn aliases_for_name(name: &str) -> Vec<SourceReferenceAlias> {
    leaf::parse_leaf_source_reference_aliases_for_name(name)
}

pub(crate) fn canonical_surface(
    aliases: &[SourceReferenceAlias],
    surface: SourceReferenceSurface,
) -> SourceReferenceSurface {
    let surface_text = match &surface {
        SourceReferenceSurface::FullName(text) | SourceReferenceSurface::ShortName(text) => {
            text.as_str()
        }
        SourceReferenceSurface::ThisPermanentType(_) => return surface,
    };
    aliases
        .iter()
        .find_map(|alias| match &alias.surface {
            SourceReferenceSurface::FullName(alias_text)
            | SourceReferenceSurface::ShortName(alias_text)
                if alias_text.eq_ignore_ascii_case(surface_text) =>
            {
                Some(alias.surface.clone())
            }
            _ => None,
        })
        .unwrap_or(surface)
}

fn aliases_for_card_identity(
    card_types: &[CardType],
    subtypes: &[Subtype],
) -> Vec<SourceReferenceAlias> {
    let mut aliases = Vec::new();
    for card_type in card_types {
        if let Some(type_name) = self_card_type_name(*card_type) {
            push_this_alias(&mut aliases, type_name);
        }
    }
    for subtype in subtypes {
        if is_permanent_subtype(*subtype) {
            push_this_alias(&mut aliases, &subtype.display_name());
        }
    }
    aliases
}

fn self_card_type_name(card_type: CardType) -> Option<&'static str> {
    match card_type {
        CardType::Artifact
        | CardType::Battle
        | CardType::Creature
        | CardType::Enchantment
        | CardType::Land
        | CardType::Plane
        | CardType::Phenomenon
        | CardType::Vanguard
        | CardType::Scheme
        | CardType::Conspiracy
        | CardType::Planeswalker => Some(card_type.name()),
        CardType::Instant | CardType::Kindred | CardType::Sorcery => None,
    }
}

fn is_permanent_subtype(subtype: Subtype) -> bool {
    subtype.is_land_subtype()
        || subtype.is_creature_type()
        || subtype.is_artifact_subtype()
        || subtype.is_enchantment_subtype()
        || subtype.is_planeswalker_subtype()
        || subtype.is_battle_subtype()
}

fn push_this_alias(aliases: &mut Vec<SourceReferenceAlias>, permanent_type: &str) {
    let Some(surface) = leaf::parse_leaf_this_source_reference_surface(permanent_type) else {
        return;
    };
    let SourceReferenceSurface::ThisPermanentType(surface_text) = &surface else {
        return;
    };
    let surface_text = surface_text.clone();
    leaf::push_leaf_source_reference_alias(aliases, &surface_text, surface);
}

fn push_alias_words(
    aliases: &mut Vec<SourceReferenceAlias>,
    words: Vec<String>,
    surface: SourceReferenceSurface,
) {
    leaf::push_leaf_source_reference_alias_words(aliases, words, surface);
}
