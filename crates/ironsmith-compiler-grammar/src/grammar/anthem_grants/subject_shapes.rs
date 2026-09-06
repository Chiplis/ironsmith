use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::{
    CardType, ObjectFilter, PlayerFilter, TagKey, TaggedObjectConstraint, TaggedOpbjectRelation,
    Zone,
};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{filters, leaf, primitives};

#[derive(Debug, Clone, PartialEq)]
pub enum AnthemSubjectGrammarMatch {
    Filter(ObjectFilter),
    RejectFragment,
}

pub fn parse_exact_anthem_subject_grammar(
    tokens: &[OwnedLexToken],
) -> Option<AnthemSubjectGrammarMatch> {
    if let Some(filter) = parse_instant_and_sorcery_spells(tokens) {
        return Some(AnthemSubjectGrammarMatch::Filter(filter));
    }
    if let Some(filter) = parse_attachment_state_qualified_subject(trim_lexed_commas(tokens)) {
        return Some(AnthemSubjectGrammarMatch::Filter(filter));
    }
    crate::grammar::primitives::probe_all(
        trim_lexed_commas(tokens),
        alt((
            parse_commander_subject,
            parse_attacking_token_subject,
            parse_distributive_filter_subject,
            parse_shared_suffix_subject,
            parse_leading_counter_threshold_fragment,
            parse_dangling_conjunction_fragment,
            parse_leading_condition_fragment,
        )),
        "anthem subject",
    )
}

fn parse_instant_and_sorcery_spells(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let view = TokenWordView::new(trim_lexed_commas(tokens));
    let words = view.word_refs();
    let (type_words, color) = match words.split_first() {
        Some((color_word, rest))
            if leaf::parse_leaf_color_complete(color_word).is_ok()
                && matches!(
                    rest.get(..4),
                    Some(["instant", "and", "sorcery", "spells"])
                        | Some(["sorcery", "and", "instant", "spells"])
                ) =>
        {
            (
                rest,
                Some(crate::grammar::primitives::probe_shape(
                    leaf::parse_leaf_color_complete(color_word),
                )?),
            )
        }
        _ => (words.as_slice(), None),
    };
    let [first_type, "and", second_type, "spells", suffix @ ..] = type_words else {
        return None;
    };
    let (first_type, second_type) = match (*first_type, *second_type) {
        ("instant", "sorcery") => (CardType::Instant, CardType::Sorcery),
        ("sorcery", "instant") => (CardType::Sorcery, CardType::Instant),
        _ => return None,
    };

    let mut filter = match suffix {
        ["you", "control"] => ObjectFilter::spell().controlled_by(PlayerFilter::You),
        ["you", "cast"] => ObjectFilter::spell().cast_by(PlayerFilter::You),
        ["you", "cast", "from", "your", "hand"] => {
            let mut filter = ObjectFilter::spell().cast_by(PlayerFilter::You);
            filter.zone = Some(Zone::Hand);
            filter
        }
        _ => return None,
    };
    filter.has_mana_cost = true;
    filter.colors = color;
    filter.any_of = vec![
        ObjectFilter::default().with_type(first_type),
        ObjectFilter::default().with_type(second_type),
    ];
    filter.set_conjunctive_set_surface(true);
    Some(filter)
}

fn parse_attachment_state_qualified_subject(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if let Some(relative_start) =
        crate::slice_primitives::find_last_window_by(&words, 4, |window| {
            crate::word_primitives::parse_choice_sequence_complete(
                window,
                &[&["that"], &["is", "are"], &["enchanted"], &["by"]],
            )
        })
    {
        let base_token_end = view.token_index_after_words(relative_start)?;
        let attachment_token_start = view.token_index_after_words(relative_start + 4)?;
        let base_tokens = trim_lexed_commas(tokens.get(..base_token_end)?);
        let attachment_tokens = trim_lexed_commas(tokens.get(attachment_token_start..)?);
        if base_tokens.is_empty() || attachment_tokens.is_empty() {
            return None;
        }
        let mut filter = crate::grammar::primitives::probe_shape(
            filters::parse_object_filter_with_grammar_entrypoint_lexed(base_tokens, false),
        )?;
        let attachment = crate::grammar::primitives::probe_shape(
            filters::parse_object_filter_with_grammar_entrypoint_lexed(attachment_tokens, false),
        )?;
        filter.with_attached_object = Some(Box::new(attachment));
        filter.set_relative_attachment_state_surface(true);
        return Some(filter);
    }

    let attachment_tags: &[crate::tag::CompilerReferenceTag] =
        if crate::word_primitives::parse_choice_sequence_suffix(
            &words,
            &[
                &["that"],
                &["is", "are"],
                &["enchanted"],
                &["or"],
                &["equipped"],
            ],
        ) {
            &[
                crate::tag::CompilerReferenceTag::Enchanted,
                crate::tag::CompilerReferenceTag::Equipped,
            ]
        } else if crate::word_primitives::parse_choice_sequence_suffix(
            &words,
            &[
                &["that"],
                &["is", "are"],
                &["equipped"],
                &["or"],
                &["enchanted"],
            ],
        ) {
            &[
                crate::tag::CompilerReferenceTag::Equipped,
                crate::tag::CompilerReferenceTag::Enchanted,
            ]
        } else if crate::word_primitives::parse_choice_sequence_suffix(
            &words,
            &[&["that"], &["is", "are"], &["enchanted"]],
        ) {
            &[crate::tag::CompilerReferenceTag::Enchanted]
        } else if crate::word_primitives::parse_choice_sequence_suffix(
            &words,
            &[&["that"], &["is", "are"], &["equipped"]],
        ) {
            &[crate::tag::CompilerReferenceTag::Equipped]
        } else {
            return None;
        };

    let suffix_word_count = if attachment_tags.len() == 2 { 5 } else { 3 };
    let base_word_count = words.len().checked_sub(suffix_word_count)?;
    let base_token_end = view.token_index_after_words(base_word_count)?;
    let base_tokens = trim_lexed_commas(tokens.get(..base_token_end)?);
    if base_tokens.is_empty() {
        return None;
    }
    let base_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(base_tokens, false),
    )?;
    let mut branches = attachment_tags
        .iter()
        .map(|tag| {
            let mut branch = base_filter.clone();
            branch.tagged_constraints.push(TaggedObjectConstraint {
                tag: tag.bind().into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            branch
        })
        .collect::<Vec<_>>();
    let mut filter = if branches.len() == 1 {
        branches.pop()?
    } else {
        let mut union = ObjectFilter::default();
        union.any_of = branches;
        union
    };
    filter.set_relative_attachment_state_surface(true);
    Some(filter)
}

fn parse_distributive_filter_subject(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    primitives::kw("each").parse_next(input)?;
    let filter_tokens = rest.parse_next(input)?;
    let filter = parse_shared_suffix_filter(filter_tokens)
        .or_else(|| {
            crate::grammar::primitives::probe_shape(
                filters::parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, false),
            )
        })
        .ok_or_else(|| primitives::backtrack_err("distributive anthem subject", "object filter"))?;
    Ok(AnthemSubjectGrammarMatch::Filter(filter))
}

fn parse_shared_suffix_subject(input: &mut LexStream<'_>) -> WResult<AnthemSubjectGrammarMatch> {
    let tokens = rest.parse_next(input)?;
    let filter = parse_shared_suffix_filter(tokens).ok_or_else(|| {
        primitives::backtrack_err("shared-suffix anthem subject", "object-filter disjunction")
    })?;
    Ok(AnthemSubjectGrammarMatch::Filter(filter))
}

fn parse_shared_suffix_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;

    for candidate in super::parse_shared_suffix_candidates(tokens) {
        let left_branch = trim_lexed_commas(&tokens[..candidate.and_token]);
        let right_branch =
            trim_lexed_commas(&tokens[candidate.and_token + 1..candidate.split_token]);
        let shared_suffix = trim_lexed_commas(&tokens[candidate.split_token..]);
        if left_branch.is_empty() || right_branch.is_empty() || shared_suffix.is_empty() {
            continue;
        }

        let leading_other = left_branch
            .first()
            .is_some_and(|token| token.is_word("other") || token.is_word("another"));
        let left_branch_body = if leading_other {
            trim_lexed_commas(&left_branch[1..])
        } else {
            left_branch
        };
        let Ok(left_branch_filter) = filters::parse_object_filter_with_grammar_entrypoint_lexed(
            left_branch_body,
            leading_other,
        ) else {
            continue;
        };
        let Ok(right_branch_filter) =
            filters::parse_object_filter_with_grammar_entrypoint_lexed(right_branch, false)
        else {
            continue;
        };
        if !subject_branch_looks_type_like(&left_branch_filter)
            || !subject_branch_looks_type_like(&right_branch_filter)
        {
            continue;
        }

        let mut left_full = left_branch_body.to_vec();
        left_full.extend_from_slice(shared_suffix);
        let mut right_full = right_branch.to_vec();
        right_full.extend_from_slice(shared_suffix);

        let Ok(mut left_filter) =
            filters::parse_object_filter_with_grammar_entrypoint_lexed(&left_full, leading_other)
        else {
            continue;
        };
        let Ok(mut right_filter) =
            filters::parse_object_filter_with_grammar_entrypoint_lexed(&right_full, false)
        else {
            continue;
        };
        if left_filter == right_filter {
            continue;
        }

        let score = object_filter_specificity_score(&left_filter)
            + object_filter_specificity_score(&right_filter)
            + shared_suffix.len();
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            let mut disjunction = ObjectFilter::default();
            if !left_filter.card_types.is_empty()
                && left_filter.card_types == right_filter.card_types
            {
                disjunction.card_types = std::mem::take(&mut left_filter.card_types);
                right_filter.card_types.clear();
            }
            if !left_filter.all_card_types.is_empty()
                && left_filter.all_card_types == right_filter.all_card_types
            {
                disjunction.all_card_types = std::mem::take(&mut left_filter.all_card_types);
                right_filter.all_card_types.clear();
            }
            if left_filter.zone == right_filter.zone {
                disjunction.zone = left_filter.zone;
                left_filter.zone = None;
                right_filter.zone = None;
            }
            if left_filter.controller == right_filter.controller {
                disjunction.controller = left_filter.controller.clone();
                left_filter.controller = None;
                right_filter.controller = None;
            }
            if left_filter.owner == right_filter.owner {
                disjunction.owner = left_filter.owner.clone();
                left_filter.owner = None;
                right_filter.owner = None;
            }
            if left_filter.mana_value == right_filter.mana_value {
                disjunction.mana_value = left_filter.mana_value.take();
                right_filter.mana_value = None;
            }
            if left_filter.other == right_filter.other || leading_other {
                disjunction.other = left_filter.other || right_filter.other;
                left_filter.other = false;
                right_filter.other = false;
            }
            disjunction.any_of = vec![left_filter, right_filter];
            disjunction.set_conjunctive_set_surface(true);
            best = Some((score, disjunction));
        }
    }

    best.map(|(_, filter)| filter)
}

fn subject_branch_looks_type_like(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
}

pub fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    if !filter.any_of.is_empty() {
        score += 12;
        score += filter
            .any_of
            .iter()
            .map(object_filter_specificity_score)
            .sum::<usize>();
    }
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += filter.supertypes.len() * 8;
    score += filter.excluded_supertypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken || filter.foretold) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(
        filter.attacking
            || filter.nonattacking
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.unblocked,
    ) * 2;
    score += usize::from(filter.is_commander || filter.noncommander) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(filter.entered_battlefield_this_turn) * 2;
    score += usize::from(filter.entered_battlefield_controller.is_some()) * 2;
    score += usize::from(filter.was_dealt_damage_this_turn) * 2;
    score += usize::from(filter.dealt_damage_to_player_this_turn.is_some()) * 2;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_ability_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.required_colors.is_some()) * 3;
    score += usize::from(filter.sticker.is_some()) * 3;
    score += usize::from(filter.chosen_color) * 3;
    score += usize::from(filter.chosen_creature_type) * 3;
    score += usize::from(filter.excluded_chosen_creature_type) * 3;
    score += usize::from(filter.excluded_any_chosen_creature_type) * 3;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

fn parse_commander_subject(input: &mut LexStream<'_>) -> WResult<AnthemSubjectGrammarMatch> {
    alt((primitives::kw("commander"), primitives::kw("commanders")))
        .void()
        .parse_next(input)?;
    let controller = parse_controller_clause(input)?;
    Ok(AnthemSubjectGrammarMatch::Filter(
        ObjectFilter::permanent()
            .commander()
            .controlled_by(controller),
    ))
}

fn parse_attacking_token_subject(input: &mut LexStream<'_>) -> WResult<AnthemSubjectGrammarMatch> {
    primitives::kw("attacking").void().parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens")))
        .void()
        .parse_next(input)?;
    let controller = parse_controller_clause(input)?;
    let mut filter = ObjectFilter::permanent().token().controlled_by(controller);
    filter.attacking = true;
    Ok(AnthemSubjectGrammarMatch::Filter(filter))
}

fn parse_controller_clause(input: &mut LexStream<'_>) -> WResult<PlayerFilter> {
    alt((
        primitives::phrase(&["you", "control"]).value(PlayerFilter::You),
        primitives::phrase(&["opponents", "control"]).value(PlayerFilter::Opponent),
        primitives::phrase(&["an", "opponent", "controls"]).value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

fn parse_dangling_conjunction_fragment(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek((primitives::kw("and"), eof)).void())
        .map(|((), ())| ())
        .parse_next(input)?;
    primitives::kw("and").void().parse_next(input)?;
    Ok(AnthemSubjectGrammarMatch::RejectFragment)
}

fn parse_leading_counter_threshold_fragment(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((
        primitives::phrase(&["or", "more"]),
        primitives::phrase(&["or", "fewer"]),
        primitives::phrase(&["or", "less"]),
    ))
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))).void(),
    )
    .void()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    let trailing_tokens: &[OwnedLexToken] = rest.parse_next(input)?;
    if trailing_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "counter-threshold fragment",
            "trailing subject fragment",
        ));
    }
    Ok(AnthemSubjectGrammarMatch::RejectFragment)
}

fn parse_leading_condition_fragment(
    input: &mut LexStream<'_>,
) -> WResult<AnthemSubjectGrammarMatch> {
    primitives::phrase(&["as", "long", "as"])
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek((primitives::kw("it"), eof)).void())
        .map(|((), ())| ())
        .parse_next(input)?;
    primitives::kw("it").void().parse_next(input)?;
    Ok(AnthemSubjectGrammarMatch::RejectFragment)
}

#[cfg(test)]
#[path = "subject_shapes/tests.rs"]
mod tests;
