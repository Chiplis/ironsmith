use crate::TagKey;
use crate::cards::builders::IT_TAG;
use crate::effect::Value;
use crate::grammar::filters::{parse_counter_type_from_tokens, parse_counter_type_words};
use crate::lexer::synthetic_word_tokens;
use crate::object_filters::parse_object_filter_words;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::util::{
    is_article, source_choose_spec_for_surface, source_reference_surface_for_words,
    this_source_surface_for_words,
};

use super::super::permission_shapes;
use super::value_helper_shapes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ForEachHead {
    item_start: usize,
    other: bool,
}

fn parse_for_each_object_filter_words(
    words: &[&str],
    leading_other: bool,
) -> Option<crate::target::ObjectFilter> {
    // Route all count filters through the lexed grammar entrypoint so
    // independently scoped repeated-`each` domains become typed union arms
    // before the permissive legacy word parser can collapse them. If the
    // count head consumed a leading `other`, restore it as an authored token:
    // in "other Assassins you control and Assassin cards in your graveyard"
    // that qualifier belongs only to the first arm.
    let mut restored = Vec::with_capacity(words.len() + usize::from(leading_other));
    if leading_other {
        restored.push("other");
    }
    restored.extend_from_slice(words);
    let tokens = synthetic_word_tokens(&restored);
    if let Some(filter) =
        crate::grammar::filters::parse_subtype_color_shared_card_union_lexed(&tokens, false)
    {
        return Some(filter);
    }
    crate::object_filters::parse_object_filter_lexed(&tokens, false).ok()
}

pub fn mana_from_source_spent_to_cast_value(source_words: &[&str]) -> Option<Value> {
    mana_from_source_spent_to_cast_value_with_reference(
        source_words,
        ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
    )
}

pub fn mana_from_source_spent_to_cast_value_with_reference(
    source_words: &[&str],
    reference: ironsmith_core::ManaSpentCastReferenceSurface,
) -> Option<Value> {
    if source_words.is_empty() {
        return None;
    }
    let include_source_noun = crate::word_primitives::last_is(source_words, "source");
    let source_words = if include_source_noun {
        &source_words[..source_words.len() - 1]
    } else {
        source_words
    };
    if source_words.is_empty() {
        return None;
    }
    let source_filter = parse_object_filter_words(source_words, false).ok()?;
    Some(Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun,
        reference,
    })
}

fn parse_mana_from_source_spent_count(words: &[&str], item_start: usize) -> Option<(Value, usize)> {
    if !crate::word_primitives::parse_sequence_prefix(&words[item_start..], &["mana", "from"]) {
        return None;
    }

    for spent_idx in item_start + 3..words.len() {
        if words[spent_idx] != "spent" {
            continue;
        }
        let (consumed, reference) = if crate::word_primitives::parse_sequence_prefix(
            &words[spent_idx..],
            &["spent", "to", "cast", "this", "spell"],
        ) {
            (
                spent_idx + 5,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
            )
        } else if crate::word_primitives::parse_sequence_prefix(
            &words[spent_idx..],
            &["spent", "to", "cast", "this", "creature"],
        ) {
            (
                spent_idx + 5,
                ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature,
            )
        } else if crate::word_primitives::parse_any_sequence_prefix(
            &words[spent_idx..],
            &[
                &["spent", "to", "cast", "it"],
                &["spent", "to", "cast", "them"],
            ],
        ) {
            (
                spent_idx + 4,
                ironsmith_core::ManaSpentCastReferenceSurface::It,
            )
        } else {
            continue;
        };

        let mut source_end = spent_idx;
        if crate::word_primitives::parse_sequence_suffix(&words[..source_end], &["that", "was"]) {
            source_end -= 2;
        }
        let source_words = words.get(item_start + 2..source_end)?;
        let value = mana_from_source_spent_to_cast_value_with_reference(source_words, reference)?;
        return Some((value, consumed));
    }
    None
}

#[cfg(test)]
#[path = "count_shapes_inline_tests.rs"]
mod tests;

#[path = "count_shapes/count_shapes_core_programs.rs"]
mod count_shapes_core_programs;
pub use count_shapes_core_programs::parse_for_each_count_value_words;
use count_shapes_core_programs::{
    exact_one_of, is_kick_count, parse_exact_dynamic_count_basis, parse_for_each_head,
    value_boundary,
};
#[path = "count_shapes/count_shapes_counter_programs.rs"]
mod count_shapes_counter_programs;
use count_shapes_counter_programs::{
    first_counter_word, is_source_counter_reference, is_tagged_counter_reference,
};
