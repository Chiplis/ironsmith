use crate::color::ColorSet;
use crate::runtime_backend::front_end::lexer::{
    OwnedLexToken, parser_token_word_refs, render_token_slice, trim_lexed_commas,
};
use crate::target::SourceReferenceSurface;
use crate::types::{Subtype, Supertype};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, permission_shapes, primitives};

const COPY_NAME_PREFIXES: &[&[&str]] = &[&["its", "name", "is"], &["it", "s", "name", "is"]];
const COPY_PRESERVE_TAILS: &[&[&str]] = &[
    &["and", "it", "has", "this", "ability"],
    &["and", "this", "ability"],
];
const COPY_LEGENDARY_TAILS: &[&[&str]] = &[
    &[
        "and",
        "its",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
    &[
        "and",
        "it",
        "s",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
    &[
        "and",
        "it",
        "is",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
];
const COLOR_CHOICES: &[&[&str]] = &[
    &["color", "of", "your", "choice"],
    &["color", "or", "colors", "of", "your", "choice"],
    &["colors", "of", "your", "choice"],
];
const SOURCE_POWER_TOUGHNESS: &[&[&str]] = &[
    &["this", "power", "and", "toughness"],
    &["thiss", "power", "and", "toughness"],
    &["source", "power", "and", "toughness"],
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BecomeCopyExceptionShape {
    pub(crate) preserve_source_abilities: bool,
    pub(crate) name_override: Option<String>,
    pub(crate) name_override_surface: Option<SourceReferenceSurface>,
    pub(crate) add_supertypes: Vec<Supertype>,
}

#[derive(Debug, Clone)]
pub(crate) struct BecomeRestShape {
    pub(crate) rest_tokens: Vec<OwnedLexToken>,
    pub(crate) body_tokens: Vec<OwnedLexToken>,
    pub(crate) copy_exception: Option<BecomeCopyExceptionShape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BecomeExactKind {
    Monarch,
    BasicLandTypeChoice,
    BasicLandType(Subtype),
    ColorChoice,
    CreatureTypeChoice,
    Colorless,
    Saddled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BecomeCopySourceShape<'a> {
    NotCopy,
    Missing,
    Source(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BecomeAuraShape<'a> {
    pub(crate) tail_tokens: &'a [OwnedLexToken],
    pub(crate) attachment_you_control: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BecomeBodySurfaceShape<'a> {
    pub(crate) body_tokens: &'a [OwnedLexToken],
    pub(crate) exact_kind: Option<BecomeExactKind>,
    pub(crate) copy_source: BecomeCopySourceShape<'a>,
    pub(crate) aura: Option<BecomeAuraShape<'a>>,
    pub(crate) equal_to_source_power_toughness: bool,
}

fn split_last_except(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let mut search_tokens = tokens;
    let mut search_offset = 0usize;
    let mut last_offset = None;
    while let Some((relative, _, after_except)) =
        primitives::find_prefix(search_tokens, || primitives::kw("except").void())
    {
        let marker_offset = search_offset + relative;
        last_offset = Some(marker_offset);
        let consumed = search_tokens.len().saturating_sub(after_except.len());
        search_offset += consumed;
        search_tokens = after_except;
    }
    let marker_offset = last_offset?;
    Some((
        trim_lexed_commas(&tokens[..marker_offset]),
        trim_lexed_commas(&tokens[marker_offset + 1..]),
    ))
}

pub(crate) fn parse_become_copy_exception_shape(
    tokens: &[OwnedLexToken],
) -> Option<BecomeCopyExceptionShape> {
    let tokens = trim_lexed_commas(tokens);
    if permission_shapes::exact_tokens(tokens, &["it", "has", "this", "ability"]) {
        return Some(BecomeCopyExceptionShape {
            preserve_source_abilities: true,
            ..Default::default()
        });
    }

    let (_, mut name_tokens) = primitives::strip_lexed_prefix_phrases(tokens, COPY_NAME_PREFIXES)?;
    let mut parsed = BecomeCopyExceptionShape::default();
    if let Some((_, head)) =
        primitives::strip_lexed_suffix_phrases(name_tokens, COPY_PRESERVE_TAILS)
    {
        parsed.preserve_source_abilities = true;
        name_tokens = head;
    }
    if let Some((_, head)) =
        primitives::strip_lexed_suffix_phrases(name_tokens, COPY_LEGENDARY_TAILS)
    {
        parsed.add_supertypes.push(Supertype::Legendary);
        name_tokens = head;
    }

    name_tokens = trim_lexed_commas(name_tokens);
    let name_words = parser_token_word_refs(name_tokens);
    if name_words.is_empty()
        || (parsed.add_supertypes.is_empty()
            && !parsed.preserve_source_abilities
            && permission_shapes::contains_tokens(name_tokens, &["and"]))
    {
        return None;
    }
    let rendered_name = render_token_slice(name_tokens).trim().to_string();
    if rendered_name.is_empty() {
        return None;
    }
    parsed.name_override_surface =
        crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
            &name_words,
        );
    parsed.name_override = Some(rendered_name);
    Some(parsed)
}

pub(crate) fn parse_become_rest_shape(tokens: &[OwnedLexToken]) -> BecomeRestShape {
    let tokens = trim_lexed_commas(tokens);
    let rest_tokens = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("become").void(),
            primitives::kw("becomes").void(),
        )),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
    .unwrap_or(tokens)
    .to_vec();
    let (body_tokens, copy_exception) = split_last_except(&rest_tokens)
        .and_then(|(body, exception)| {
            parse_become_copy_exception_shape(exception).map(|parsed| (body.to_vec(), parsed))
        })
        .map(|(body, exception)| (body, Some(exception)))
        .unwrap_or_else(|| (rest_tokens.clone(), None));
    BecomeRestShape {
        rest_tokens,
        body_tokens,
        copy_exception,
    }
}

fn basic_land_type(words: &[&str]) -> Option<Subtype> {
    let [word] = words else {
        return None;
    };
    let subtype = leaf::parse_leaf_subtype_flexible_complete(word).ok()?;
    matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    )
    .then_some(subtype)
}

pub(crate) fn parse_become_body_surface_shape(
    tokens: &[OwnedLexToken],
) -> BecomeBodySurfaceShape<'_> {
    let tokens = trim_lexed_commas(tokens);
    let body_tokens = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("the").void(),
            primitives::kw("a").void(),
            primitives::kw("an").void(),
        )),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    let words = parser_token_word_refs(body_tokens);
    let exact_kind = if permission_shapes::exact_words(&words, &["monarch"]) {
        Some(BecomeExactKind::Monarch)
    } else if permission_shapes::exact_words(
        &words,
        &["basic", "land", "type", "of", "your", "choice"],
    ) {
        Some(BecomeExactKind::BasicLandTypeChoice)
    } else if let Some(subtype) = basic_land_type(&words) {
        Some(BecomeExactKind::BasicLandType(subtype))
    } else if COLOR_CHOICES
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
    {
        Some(BecomeExactKind::ColorChoice)
    } else if permission_shapes::exact_words(&words, &["creature", "type", "of", "your", "choice"])
    {
        Some(BecomeExactKind::CreatureTypeChoice)
    } else if permission_shapes::exact_words(&words, &["colorless"]) {
        Some(BecomeExactKind::Colorless)
    } else if permission_shapes::exact_words(&words, &["saddled"]) {
        Some(BecomeExactKind::Saddled)
    } else {
        None
    };

    let copy_source = if let Some((_, source_tokens)) =
        primitives::parse_prefix(body_tokens, primitives::phrase(&["copy", "of"]).void())
    {
        let source_tokens = trim_lexed_commas(source_tokens);
        if source_tokens.is_empty() {
            BecomeCopySourceShape::Missing
        } else {
            BecomeCopySourceShape::Source(source_tokens)
        }
    } else {
        BecomeCopySourceShape::NotCopy
    };

    let aura_tail = primitives::parse_prefix(
        body_tokens,
        primitives::phrase(&["aura", "enchantment", "with", "enchant", "creature"]).void(),
    )
    .or_else(|| {
        primitives::parse_prefix(
            body_tokens,
            primitives::phrase(&["aura", "with", "enchant", "creature"]).void(),
        )
    })
    .map(|(_, tail)| tail);
    let aura = aura_tail.map(|tail_tokens| BecomeAuraShape {
        tail_tokens,
        attachment_you_control: permission_shapes::prefix_tokens(tail_tokens, &["you", "control"]),
    });
    let equal_to_source_power_toughness =
        primitives::parse_prefix(body_tokens, primitives::phrase(&["equal", "to"]).void())
            .is_some_and(|(_, rhs)| {
                SOURCE_POWER_TOUGHNESS
                    .iter()
                    .any(|expected| permission_shapes::exact_tokens(rhs, expected))
            });

    BecomeBodySurfaceShape {
        body_tokens,
        exact_kind,
        copy_source,
        aura,
        equal_to_source_power_toughness,
    }
}

pub(crate) fn parse_become_attack_color(words: &[&str]) -> Option<ColorSet> {
    let [
        color_word,
        "until",
        "end",
        "of",
        "turn",
        "and",
        "attacks",
        tail @ ..,
    ] = words
    else {
        return None;
    };
    if !matches!(tail, ["if", "able"] | ["this", "turn", "if", "able"]) {
        return None;
    }
    leaf::parse_leaf_color_complete(color_word).ok()
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::lex_line;

    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn rest_shape_strips_verb_and_returns_typed_copy_exception() {
        let shape = parse_become_rest_shape(&lex(
            "becomes a copy of target creature except its name is Relic and it has this ability",
        ));
        assert!(shape.copy_exception.unwrap().preserve_source_abilities);
        assert_eq!(
            parser_token_word_refs(&shape.body_tokens),
            ["a", "copy", "of", "target", "creature"]
        );
    }

    #[test]
    fn body_shape_classifies_exact_copy_aura_and_equal_surfaces() {
        let copy_tokens = lex("a copy of target creature");
        assert!(matches!(
            parse_become_body_surface_shape(&copy_tokens).copy_source,
            BecomeCopySourceShape::Source(_)
        ));
        let aura_tokens = lex("an Aura with enchant creature you control");
        assert!(
            parse_become_body_surface_shape(&aura_tokens)
                .aura
                .unwrap()
                .attachment_you_control
        );
        let equal_tokens = lex("equal to this power and toughness");
        assert!(parse_become_body_surface_shape(&equal_tokens).equal_to_source_power_toughness);
    }
}
