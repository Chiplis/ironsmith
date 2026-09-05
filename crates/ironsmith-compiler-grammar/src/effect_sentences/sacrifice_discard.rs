use super::*;
use crate::effect_sentences::parse_artifact_enchantment_or_token_filter;
use crate::effect_sentences::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, rewrite_unless_cost_source_values_to_it_tag, try_build_unless,
};
use crate::grammar::effects::sacrifice_discard_shapes as sacrifice_discard_grammar;
use crate::grammar::filters::preserve_branch_scoped_card_type_union;

fn trim_trailing_discard_alternative_action(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let discard_tokens = sacrifice_discard_grammar::parse_discard_alternative_shape(tokens)
        .map(|shape| shape.discard_tokens)
        .unwrap_or(tokens);
    trim_commas(discard_tokens)
}

fn parse_trailing_discard_unless_predicate(
    trailing_tokens: &[OwnedLexToken],
    player: PlayerAst,
    count: Value,
    any_number: bool,
    discard_filter: Option<ObjectFilter>,
) -> Result<Option<EffectAst>, CardTextError> {
    let predicate_tokens =
        match sacrifice_discard_grammar::parse_discard_unless_shape(trailing_tokens) {
            sacrifice_discard_grammar::DiscardUnlessShape::None => return Ok(None),
            sacrifice_discard_grammar::DiscardUnlessShape::MissingPredicate => {
                return Err(CardTextError::ParseError(
                    "missing predicate after trailing discard unless".to_string(),
                ));
            }
            sacrifice_discard_grammar::DiscardUnlessShape::Predicate(predicate_tokens) => {
                predicate_tokens
            }
        };
    let predicate =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)?;
    let discard =
        EffectAst::subject_verb_discard(player, count, false, any_number, discard_filter, None);

    Ok(Some(EffectAst::Conditional {
        predicate: PredicateAst::Not(Box::new(predicate)),
        if_true: vec![discard],
        if_false: Vec::new(),
    }))
}

fn wrap_unless_escaped(effect: EffectAst, unless_escaped: bool) -> EffectAst {
    if unless_escaped {
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellEscaped,
            if_true: Vec::new(),
            if_false: vec![effect],
        }
    } else {
        effect
    }
}

fn triggering_same_mana_value_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::Triggering.bind(),
            relation: crate::target::TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    filter
}

/// Keep an adjective attached to its terminal noun in a serialized union.
///
/// The broad card-type parser can flatten `enchantments and nonbasic lands`
/// into one branch, which incorrectly applies `nonbasic` to both types.  When
/// the source tokens explicitly anchor the adjective immediately before
/// `land(s)`, split that mixed branch and retain the Basic exclusion only on
/// the land arm.
pub fn preserve_terminal_nonbasic_land_union(tokens: &[OwnedLexToken], filter: &mut ObjectFilter) {
    let adjective_is_land_local = crate::slice_primitives::find_window_by(tokens, 2, |window| {
        window[0].is_word("nonbasic") && (window[1].is_word("land") || window[1].is_word("lands"))
    })
    .is_some();
    if !adjective_is_land_local {
        return;
    }

    // Some whole-sentence subject/verb routes flatten the type list before
    // the later correlated-result pass sees it. The source adjacency still
    // proves `nonbasic` belongs only to the land arm, so recover the same
    // inclusive union from the flattened selector.
    if filter.any_of.is_empty()
        && filter.card_types.len() > 1
        && crate::slice_primitives::contains(&filter.card_types, &crate::types::CardType::Land)
        && crate::slice_primitives::contains(
            &filter.excluded_supertypes,
            &crate::types::Supertype::Basic,
        )
    {
        let card_types = std::mem::take(&mut filter.card_types);
        filter
            .excluded_supertypes
            .retain(|supertype| *supertype != crate::types::Supertype::Basic);
        filter.any_of = card_types
            .into_iter()
            .map(|card_type| {
                let mut branch = ObjectFilter::default().with_type(card_type);
                if card_type == crate::types::CardType::Land {
                    branch
                        .excluded_supertypes
                        .push(crate::types::Supertype::Basic);
                }
                branch
            })
            .collect();
        return;
    }

    if filter.any_of.is_empty() {
        return;
    }

    let mut branches = Vec::new();
    for branch in std::mem::take(&mut filter.any_of) {
        if branch.card_types.len() > 1
            && crate::slice_primitives::contains(&branch.card_types, &crate::types::CardType::Land)
            && crate::slice_primitives::contains(
                &branch.excluded_supertypes,
                &crate::types::Supertype::Basic,
            )
        {
            for card_type in &branch.card_types {
                let mut split = branch.clone();
                split.card_types = vec![*card_type];
                if *card_type != crate::types::CardType::Land {
                    split
                        .excluded_supertypes
                        .retain(|supertype| *supertype != crate::types::Supertype::Basic);
                }
                branches.push(split);
            }
        } else {
            branches.push(branch);
        }
    }
    filter.any_of = branches;
}

#[cfg(test)]
#[path = "sacrifice_discard_inline_selected_sacrifice_tests.rs"]
mod selected_sacrifice_tests;

#[path = "sacrifice_discard/library.rs"]
mod library_programs;
pub use library_programs::{discard_subject_owner_filter, parse_discard};
#[path = "sacrifice_discard/resource.rs"]
mod resource_programs;
pub use resource_programs::parse_sacrifice;
