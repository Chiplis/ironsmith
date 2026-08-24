use crate::cards::builders::{CHOSEN_OBJECTS_TAG, CardTextError, IT_TAG, TargetAst};
use crate::grammar::filters::parse_filter_counter_constraint_words;
use crate::grammar::leaf;
use crate::grammar::permission_shapes;
use crate::grammar::primitives::{self, token_slice_span};
use crate::grammar::targets::{
    EnchantedObjectTargetKind, TargetControllerSetConstraint, TargetPreparationFacts,
    TargetUnionShape, TrailingPlayerTargetKind, parse_chosen_object_target,
    parse_dynamic_target_count_prefix, parse_enchanted_object_target_kind,
    parse_object_or_player_union_target, parse_referenced_target_prefix,
    parse_target_controller_set_suffix, parse_target_for_each_suffix,
    parse_target_preparation_facts, parse_target_union_shape,
};
use crate::lexer::{OwnedLexToken, TokenWordView};
use crate::object_filters::parse_object_filter;
use crate::target::{
    ObjectFilter, PlayerFilter, SacrificedObjectKind, SourceReferenceSurface, TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::util::{
    is_article, is_demonstrative_object_head, parse_for_each_count_value_words,
    parse_subtype_flexible, source_reference_surface_for_possessive_words,
    source_reference_surface_for_words, strip_possessive_suffix, this_source_surface_for_words,
};
use crate::zone::Zone;
use crate::{ChoiceCount, TagKey};

use super::aggregate_constraints::lift_total_mana_value_choice_constraint;
use super::reference_shapes;
use super::target_surfaces::*;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

fn typed_demonstrative_reference_surface(
    tokens: &[OwnedLexToken],
) -> Option<SourceReferenceSurface> {
    let words = TokenWordView::new(tokens).to_word_refs();
    if words.len() < 2
        || !matches!(words[0], "that" | "those")
        || !words[1..]
            .iter()
            .any(|word| is_demonstrative_object_head(word))
    {
        return None;
    }

    Some(SourceReferenceSurface::ThisPermanentType(
        crate::lexer::render_token_slice(tokens).to_ascii_lowercase(),
    ))
}

fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

fn apply_target_preparation_facts(filter: &mut ObjectFilter, facts: TargetPreparationFacts) {
    if !facts.clear_source_linked_exile {
        return;
    }
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    });
    filter.zone.get_or_insert(Zone::Exile);
}

fn tagged_it_owner_or_controller_player_filter(word: &str) -> PlayerFilter {
    if matches!(word, "owner" | "owners") {
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
    } else {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
    }
}

fn contextual_other_player_filter(base: PlayerFilter) -> PlayerFilter {
    PlayerFilter::excluding(base, PlayerFilter::IteratedPlayer)
}

fn source_owner_exclusion(words: &[&str]) -> Option<PlayerFilter> {
    let (&owner, source_words) = words.split_last()?;
    if !matches!(owner, "owner" | "owners") {
        return None;
    }
    let normalized = source_words
        .iter()
        .filter_map(|word| match *word {
            "s" | "'" | "’" => None,
            word => Some(strip_possessive_suffix(word)),
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    this_source_surface_for_words(&normalized)?;
    Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
        crate::tag::SOURCE_OBJECT_TAG,
    )))
}

fn explicit_player_exclusion(words: &[&str]) -> Option<PlayerFilter> {
    let split = crate::word_primitives::parse_sequence_start(words, &["other", "than"])?;
    let base = if crate::word_primitives::parse_any_sequence_complete(
        &words[..split],
        &[&["player"], &["players"]],
    ) {
        PlayerFilter::Any
    } else if crate::word_primitives::parse_any_sequence_complete(
        &words[..split],
        &[&["opponent"], &["opponents"]],
    ) {
        PlayerFilter::Opponent
    } else {
        return None;
    };
    let excluded_words = &words[split + 2..];
    let excluded = if crate::word_primitives::parse_sequence_complete(excluded_words, &["you"]) {
        PlayerFilter::You
    } else if crate::word_primitives::parse_any_sequence_complete(
        excluded_words,
        &[&["that", "player"], &["that", "players"]],
    ) {
        PlayerFilter::IteratedPlayer
    } else {
        source_owner_exclusion(excluded_words)?
    };
    Some(PlayerFilter::excluding(base, excluded))
}

fn sacrificed_object_kind(words: &[&str]) -> Option<SacrificedObjectKind> {
    let words = if crate::word_primitives::first_is_any(words, &["the", "a", "an"]) {
        &words[1..]
    } else {
        words
    };
    crate::word_primitives::matching_value(
        words,
        &[
            (&["sacrificed", "creature"], SacrificedObjectKind::Creature),
            (&["sacrificed", "artifact"], SacrificedObjectKind::Artifact),
            (
                &["sacrificed", "enchantment"],
                SacrificedObjectKind::Enchantment,
            ),
            (
                &["sacrificed", "permanent"],
                SacrificedObjectKind::Permanent,
            ),
        ],
    )
}

#[cfg(test)]
#[path = "target_semantics_inline_tests.rs"]
mod tests;

#[path = "target_semantics/reference_programs.rs"]
mod reference_programs;
pub use reference_programs::parse_target_phrase_inner;
