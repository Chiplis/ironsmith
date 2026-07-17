use crate::cards::builders::{CHOSEN_OBJECTS_TAG, CardTextError, IT_TAG, TargetAst};
use crate::runtime_backend::grammar::filters::parse_filter_counter_constraint_words;
use crate::runtime_backend::grammar::leaf;
use crate::runtime_backend::grammar::permission_shapes;
use crate::runtime_backend::grammar::primitives::{self, token_slice_span};
use crate::runtime_backend::grammar::targets::{
    EnchantedObjectTargetKind, TargetControllerSetConstraint, TargetPreparationFacts,
    TargetUnionShape, parse_chosen_object_target, parse_dynamic_target_count_prefix,
    parse_enchanted_object_target_kind, parse_referenced_target_prefix,
    parse_target_controller_set_suffix, parse_target_for_each_suffix,
    parse_target_preparation_facts, parse_target_union_shape,
};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView};
use crate::runtime_backend::object_filters::parse_object_filter;
use crate::runtime_backend::util::{
    is_article, is_demonstrative_object_head, parse_for_each_count_value_words,
    record_sacrificed_object_kind, record_source_reference_surface,
    source_reference_surface_for_possessive_words, source_reference_surface_for_words,
    strip_possessive_suffix, this_source_surface_for_words,
};
use crate::target::{
    ObjectFilter, PlayerFilter, SacrificedObjectKind, SourceReferenceSurface, TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;
use crate::{ChoiceCount, TagKey};

use super::reference_shapes;
use super::target_surfaces::*;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

fn typed_demonstrative_reference_surface(
    tokens: &[OwnedLexToken],
) -> Option<SourceReferenceSurface> {
    let words = TokenWordView::new(tokens).to_word_refs();
    if words.len() != 2
        || !matches!(words[0], "that" | "those")
        || !is_demonstrative_object_head(words[1])
    {
        return None;
    }

    Some(SourceReferenceSurface::ThisPermanentType(words.join(" ")))
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
    let split = words
        .windows(2)
        .position(|window| window == ["other", "than"])?;
    let base = match &words[..split] {
        ["player"] | ["players"] => PlayerFilter::Any,
        ["opponent"] | ["opponents"] => PlayerFilter::Opponent,
        _ => return None,
    };
    let excluded_words = &words[split + 2..];
    let excluded = match excluded_words {
        ["you"] => PlayerFilter::You,
        ["that", "player"] | ["that", "players"] => PlayerFilter::IteratedPlayer,
        _ => source_owner_exclusion(excluded_words)?,
    };
    Some(PlayerFilter::excluding(base, excluded))
}

fn sacrificed_object_kind(words: &[&str]) -> Option<SacrificedObjectKind> {
    let words = match words {
        [article @ ("the" | "a" | "an"), rest @ ..] => {
            let _ = article;
            rest
        }
        words => words,
    };
    match words {
        ["sacrificed", "creature"] => Some(SacrificedObjectKind::Creature),
        ["sacrificed", "artifact"] => Some(SacrificedObjectKind::Artifact),
        ["sacrificed", "enchantment"] => Some(SacrificedObjectKind::Enchantment),
        ["sacrificed", "permanent"] => Some(SacrificedObjectKind::Permanent),
        _ => None,
    }
}

pub(crate) fn parse_target_phrase_inner(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    while permission_shapes::prefix_tokens(tokens, &["then"]) {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    // `each` is a set quantifier rather than part of the object filter. Let
    // the ordinary target-head parser see a following `other` so it can retain
    // the source-exclusion bit (for example, "each other creature").
    if tokens.first().and_then(OwnedLexToken::as_word) == Some("each")
        && tokens.get(1).and_then(OwnedLexToken::as_word) == Some("other")
    {
        return parse_target_phrase_inner(&tokens[1..]);
    }

    if let Some(dynamic) = parse_dynamic_target_count_prefix(tokens) {
        let target = parse_target_phrase_inner(dynamic.target_tokens)?;
        return Ok(TargetAst::WithCountValue(
            Box::new(target),
            dynamic.count,
            dynamic.value,
        ));
    }

    let token_word_view = TokenWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();
    if let Some(kind) = sacrificed_object_kind(&token_words) {
        let span = token_slice_span(tokens);
        record_sacrificed_object_kind(span, kind);
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), span));
    }
    if matches_surface(token_words.as_slice(), YOUR_OPPONENTS_TARGET_PATTERN) {
        return Ok(TargetAst::Player(
            PlayerFilter::Opponent,
            token_slice_span(tokens),
        ));
    }
    if matches_surface(
        token_words.as_slice(),
        DEFENDING_PLAYER_CHOICE_TARGET_PATTERN,
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported defending player's choice target phrase '{}'",
            token_words.join(" ")
        )));
    }

    // Recognize an exact `this <permanent type>` source surface before the
    // generic target head consumes `this` as a demonstrative prefix.  Once
    // consumed, only the object noun remains and the phrase would otherwise
    // widen from the source permanent to every matching permanent.
    if let Some(surface) = this_source_surface_for_words(&token_words) {
        let span = token_slice_span(tokens);
        record_source_reference_surface(span, surface);
        return Ok(TargetAst::Source(span));
    }
    if let Some(surface) = source_reference_surface_for_possessive_words(&token_words) {
        let span = token_slice_span(tokens);
        record_source_reference_surface(span, surface);
        return Ok(TargetAst::Source(span));
    }

    let target_head = leaf::parse_leaf_target_head_tokens(tokens)?;
    tokens = target_head.tokens();
    let random_choice = target_head.prefix.random.is_some();
    let span = target_head.prefix.phrase_span;
    let target_count: Option<ChoiceCount> = None;

    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if matches_surface(&all_words, ANY_TARGET_PATTERN) {
        return Ok(TargetAst::AnyTarget(span));
    }
    if matches_surface(&all_words, ANY_OTHER_TARGET_PATTERN) {
        return Ok(TargetAst::AnyOtherTarget(span));
    }
    if let Some(reference) = parse_referenced_target_prefix(tokens) {
        let mut filter = parse_object_filter(reference.object_tokens, reference.other)?;
        filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        let mut count = ChoiceCount::exactly(reference.count as usize);
        if random_choice {
            count = count.at_random();
        }
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            Some(count),
        ));
    }
    if matches_surface(&all_words, IT_OR_THEM_WITH_PREFIX_PATTERN)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[2..])
        && consumed == all_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if matches_surface(&all_words, ALL_REFERENCED_WITH_THAT_NAME_PATTERN) {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if matches_surface(&all_words, TAGGED_OBJECT_TARGET_PATTERN) {
        if let Some(surface) = typed_demonstrative_reference_surface(tokens) {
            record_source_reference_surface(span, surface);
        } else if all_words == ["the", "card"] {
            record_source_reference_surface(
                span,
                SourceReferenceSurface::ThisPermanentType("the card".to_string()),
            );
        }
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if matches_surface(&all_words, REST_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("rest"), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if let Some(chosen) = parse_chosen_object_target(tokens) {
        let filter_tokens = chosen.filter_tokens;
        let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
        let mut filter = if matches_surface(&filter_words, CARDS_TARGET_SHORTHAND_PATTERN) {
            ObjectFilter::default()
        } else {
            parse_object_filter(&filter_tokens, false)?
        };
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_OBJECTS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, EQUIPPED_OBJECT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("equipped"), span),
            target_count,
        ));
    }
    if let Some(enchanted) = parse_enchanted_object_target_kind(&remaining_words) {
        if enchanted == EnchantedObjectTargetKind::Creature {
            let mut filter = ObjectFilter::tagged(TagKey::from("enchanted"));
            filter.card_types.push(CardType::Creature);
            return Ok(wrap_target_count(
                TargetAst::Object(filter, None, span),
                target_count,
            ));
        }
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("enchanted"), span),
            target_count,
        ));
    }
    if matches_surface(
        &remaining_words,
        CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN,
    ) {
        record_source_reference_surface(
            span,
            SourceReferenceSurface::ThisPermanentType(
                "the creature tapped to pay this spell's additional cost".to_string(),
            ),
        );
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("tap_cost_0"), span),
            target_count,
        ));
    }

    let target_count = target_head.prefix.count;
    let idx = target_head.prefix.consumed;
    let other = target_head.prefix.other;
    let explicit_target = target_head.prefix.explicit_target_span.is_some();
    let saw_top_prefix = target_head.prefix.top.is_some();

    let words_all = crate::runtime_backend::token_word_refs(&tokens[idx..]);
    if matches_surface(&words_all, ANY_TARGET_PATTERN) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }
    if matches_surface(&words_all, ANY_OTHER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = crate::runtime_backend::token_word_refs(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.is_empty() && explicit_target {
        return Ok(wrap_target_count(
            if other {
                TargetAst::AnyOtherTarget(span)
            } else {
                TargetAst::AnyTarget(span)
            },
            target_count,
        ));
    }
    if other && matches_surface(&remaining_words, TARGET_OR_TARGETS_WORD_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TARGET_OR_TARGETS_WORD_PATTERN) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let bare_top_library_shorthand = saw_top_prefix
        && !remaining_words
            .iter()
            .any(|word| matches_surface_word(word, LIBRARY_WORD_PATTERN))
        && (matches_surface(&remaining_words, TOP_CARD_TARGET_SHORTHAND_PATTERN)
            || (target_count.is_some()
                && matches_surface(&remaining_words, CARDS_TARGET_SHORTHAND_PATTERN)));
    if bare_top_library_shorthand {
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    if let Some(filter) = reference_shapes::parse_hand_advantage_player(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if let Some(filter) = reference_shapes::parse_life_advantage_player(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, PLAYER_ON_YOUR_TEAM_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if let Some(filter) = explicit_player_exclusion(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }
    if other && matches_surface(&remaining_words, ANY_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                contextual_other_player_filter(PlayerFilter::Any),
                target_span,
            ),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ANY_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ENCHANTED_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::TaggedPlayer(TagKey::from("enchanted")),
                target_span,
            ),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, CHOSEN_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::ChosenPlayer, target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_OPPONENT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, DEFENDING_PLAYER_EDGE_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Defending, target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words.get(1).is_some_and(|word| {
        let normalized = strip_possessive_suffix(word);
        leaf::parse_leaf_object_reference_head_complete(normalized).is_ok()
    });
    if remaining_words.len() >= 3
        && matches_surface_word(remaining_words[0], THAT_OR_THE_WORD_PATTERN)
        && second_word_is_object_head
        && matches_surface_word(remaining_words[2], CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN)
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[2]);
        return Ok(wrap_target_count(
            // The referenced object may have been targeted earlier, but its
            // controller/owner is an ordinary resolution-time reference. The
            // possessive phrase does not create another target requirement.
            TargetAst::Player(player, None),
            target_count,
        ));
    }
    if remaining_words.len() >= 5
        && matches_surface_word(remaining_words[0], THAT_WORD_PATTERN)
        && second_word_is_object_head
        && matches_surface_word(remaining_words[2], OR_WORD_PATTERN)
        && is_demonstrative_object_head(remaining_words[3])
        && matches_surface_word(remaining_words[4], CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN)
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[4]);
        return Ok(wrap_target_count(
            TargetAst::Player(player, None),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                None,
            ),
            target_count,
        ));
    }
    if remaining_words.len() >= 2 {
        let object_head = strip_possessive_suffix(remaining_words[0]);
        if matches!(
            remaining_words[1],
            "controller" | "controllers" | "owner" | "owners"
        ) && leaf::parse_leaf_object_reference_head_complete(object_head).is_ok()
        {
            let player = tagged_it_owner_or_controller_player_filter(remaining_words[1]);
            return Ok(wrap_target_count(
                TargetAst::Player(player, None),
                target_count,
            ));
        }
    }
    if matches_surface(&remaining_words, ITS_OR_THEIR_OWNER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                None,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, YOU_OR_YOUR_PREFIX_PATTERN) && remaining_words.len() == 1 {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                if other {
                    contextual_other_player_filter(PlayerFilter::Opponent)
                } else {
                    PlayerFilter::Opponent
                },
                target_span,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, OPPONENT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                if other {
                    contextual_other_player_filter(PlayerFilter::Opponent)
                } else {
                    PlayerFilter::Opponent
                },
                target_span,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, SPELL_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Spell(target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TRIGGERING_SPELL_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("triggering"), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("triggering_source"), span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, IT_OR_THEM_WITH_PREFIX_PATTERN)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, span),
            target_count,
        ));
    }

    if reference_shapes::is_source_from_your_graveyard(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Graveyard);
        source_filter.owner = Some(PlayerFilter::You);
        if let Some(surface) = source_reference_surface_for_words(&remaining_words)
            .or_else(|| this_source_surface_for_words(&remaining_words))
        {
            source_filter = source_filter.with_source_surface(surface);
        }
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if reference_shapes::is_source_from_exile(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Exile);
        if let Some(surface) = source_reference_surface_for_words(&remaining_words)
            .or_else(|| this_source_surface_for_words(&remaining_words))
        {
            source_filter = source_filter.with_source_surface(surface);
        }
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if let Some(surface) = source_reference_surface_for_words(&remaining_words)
        .or_else(|| this_source_surface_for_words(&remaining_words))
    {
        let source_span = target_span.or(span);
        record_source_reference_surface(source_span, surface);
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, SOURCE_PT_REFERENCE_PREFIX_PATTERN)
        || matches_surface(&remaining_words, SOURCE_PT_REFERENCE_TARGET_PATTERN)
    {
        let source_span = target_span.or(span);
        record_source_reference_surface(
            source_span,
            SourceReferenceSurface::ThisPermanentType(remaining_words.join(" ")),
        );
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, IT_INSTEAD_THIS_WAY_PREFIX_PATTERN)
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| matches_surface_word(word, INSTEAD_THIS_WAY_WORD_PATTERN))
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TOKEN_CREATED_THIS_WAY_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ITSELF_TARGET_PATTERN) {
        record_source_reference_surface(
            span,
            SourceReferenceSurface::ThisPermanentType("itself".to_string()),
        );
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches_surface(&remaining_words, HIM_OR_HER_TARGET_PATTERN) {
        record_source_reference_surface(
            span,
            SourceReferenceSurface::ThisPermanentType(remaining_words[0].to_string()),
        );
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches_surface(&remaining_words, THEM_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let attacking_you_or_your_planeswalker = [
        &[
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
        &[
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
        &[
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if attacking_you_or_your_planeswalker {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Creature);
        filter.attacking = true;
        filter.controller = Some(PlayerFilter::Opponent);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    let opponent_or_planeswalker = [
        &["opponent", "or", "planeswalker"][..],
        &["opponents", "or", "planeswalkers"][..],
        &["planeswalker", "or", "opponent"][..],
        &["planeswalkers", "or", "opponents"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let prior_player_or_planeswalker = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::PriorPlayerOrPlaneswalker)
    );
    if prior_player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(
                PlayerFilter::TargetPlayerOrControllerOfTarget,
                target_span,
            ),
            target_count,
        ));
    }

    let player_or_planeswalker_its_attacking = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::AttackedPlayerOrPlaneswalker)
    );
    if player_or_planeswalker_its_attacking {
        return Ok(wrap_target_count(
            TargetAst::AttackedPlayerOrPlaneswalker(target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = [
        &["player", "or", "planeswalker"][..],
        &["players", "or", "planeswalkers"][..],
        &["planeswalker", "or", "player"][..],
        &["planeswalkers", "or", "players"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::BattleOrOpponent)
    ) {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Battle);
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let creature_or_player = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::CreatureOrPlayer)
    );
    if creature_or_player {
        let mut filter = ObjectFilter::creature();
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::PermanentOrPlayer)
    ) {
        let mut filter = ObjectFilter::permanent();
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    let mixed_object_player_target =
        matches_surface(&remaining_words, MIXED_PLAYER_PLANESWALKER_TOKEN_PATTERN);
    if mixed_object_player_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-token/player/planeswalker target phrase (clause: '{}')",
            remaining_words.join(" ")
        )));
    }

    let controller_set = parse_target_controller_set_suffix(remaining);
    let target_set_same_controller = matches!(
        controller_set.constraint,
        TargetControllerSetConstraint::SameController
    );
    let target_set_different_controllers = matches!(
        controller_set.constraint,
        TargetControllerSetConstraint::DifferentControllers
    );
    let remaining = controller_set.core_tokens.as_slice();
    if target_count.is_none_or(|count| count.is_single())
        && let Some(for_each) = parse_target_for_each_suffix(remaining)
        && let Some((count_value, used_words)) =
            parse_for_each_count_value_words(&for_each.count_words)
        && used_words == for_each.count_words.len()
    {
        let object_tokens = for_each.object_tokens;
        if !object_tokens.is_empty() {
            let mut filter = parse_object_filter(object_tokens, other)?;
            filter.target_set_same_controller = target_set_same_controller;
            filter.target_set_different_controllers = target_set_different_controllers;
            return Ok(TargetAst::WithCountValue(
                Box::new(TargetAst::Object(filter, target_span, None)),
                ChoiceCount::dynamic_x(),
                count_value,
            ));
        }
    }

    let mut filter = parse_object_filter(remaining, other)?;
    apply_target_preparation_facts(
        &mut filter,
        parse_target_preparation_facts(remaining, explicit_target),
    );
    filter.target_set_same_controller = target_set_same_controller;
    filter.target_set_different_controllers = target_set_different_controllers;
    if filter.with_counter.is_none()
        && remaining_words
            .first()
            .is_some_and(|word| matches_surface_word(word, IT_OR_THEM_WORD_PATTERN))
        && remaining_words
            .get(1)
            .is_some_and(|word| matches_surface_word(word, WITH_WORD_PATTERN))
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        filter.with_counter = Some(counter_constraint);
    }
    let reference_span = if let Some(surface) = typed_demonstrative_reference_surface(remaining) {
        filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        let span = token_slice_span(remaining);
        record_source_reference_surface(span, surface);
        span
    } else if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mut idx = tokens.len();
        let mut found_span = None;
        while idx > 0 {
            idx -= 1;
            if token_matches_surface(&tokens[idx], IT_WORD_PATTERN) {
                found_span = Some(tokens[idx].span());
                break;
            }
        }
        found_span
    } else {
        None
    };
    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, reference_span),
        target_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorSet;
    use crate::events::KeywordActionKind;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse(raw: &str) -> TargetAst {
        let tokens = lex_line(raw, 0).expect("lex target");
        parse_target_phrase_inner(&tokens).expect(raw)
    }

    #[test]
    fn target_other_than_source_remains_an_explicit_object_target() {
        let TargetAst::Object(filter, explicit_target, _) =
            parse("target creature other than this creature")
        else {
            panic!("expected object target");
        };
        assert!(explicit_target.is_some());
        assert!(filter.other);
        assert!(!filter.source);
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert!(filter.source_surface.is_some());
    }

    #[test]
    fn another_stickered_target_does_not_turn_reflexive_it_into_a_reference() {
        let TargetAst::Object(filter, explicit_target, it_span) =
            parse("another target creature with an art sticker on it")
        else {
            panic!("expected object target");
        };
        assert!(explicit_target.is_some());
        assert!(filter.other);
        assert!(!filter.source);
        assert_eq!(filter.sticker, Some(KeywordActionKind::ArtSticker));
        assert!(filter.tagged_constraints.is_empty());
        assert!(it_span.is_none());
    }

    #[test]
    fn each_other_object_preserves_source_exclusion() {
        let TargetAst::Object(filter, _, _) = parse("each other creature") else {
            panic!("expected object filter");
        };
        assert!(filter.other);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn battle_or_opponent_preserves_both_target_domains_and_source_exclusion() {
        let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
            parse("another target battle or opponent")
        else {
            panic!("expected object/player union target");
        };
        assert!(explicit_target.is_some());
        assert!(filter.other);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Battle]);
        assert_eq!(player, PlayerFilter::Opponent);
    }

    #[test]
    fn non_target_permanent_or_player_union_remains_non_targeting() {
        let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
            parse("a permanent or player")
        else {
            panic!("expected object/player union reference");
        };
        assert!(explicit_target.is_none());
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(player, PlayerFilter::Any);
    }

    #[test]
    fn attacked_player_or_planeswalker_remains_a_combat_reference() {
        assert!(matches!(
            parse("the player or planeswalker it's attacking"),
            TargetAst::AttackedPlayerOrPlaneswalker(_)
        ));
    }

    #[test]
    fn both_color_target_records_an_all_of_color_constraint() {
        let TargetAst::Object(filter, explicit_target, _) =
            parse("target spell thats both blue and black")
        else {
            panic!("expected spell object target");
        };
        assert!(explicit_target.is_some());
        assert_eq!(
            filter.required_colors,
            Some(ColorSet::BLUE.union(ColorSet::BLACK))
        );
        assert_eq!(filter.colors, None);
    }

    #[test]
    fn typed_demonstrative_target_records_its_exact_surface() {
        let tokens = lex_line("that creature", 0).expect("lex target");
        let target = parse_target_phrase_inner(&tokens).expect("parse target");
        let TargetAst::Object(_, _, Some(span)) = target else {
            panic!("expected typed demonstrative object target with reference span");
        };
        assert_eq!(
            crate::runtime_backend::util::source_reference_surface_for_span(Some(span)),
            Some(SourceReferenceSurface::ThisPermanentType(
                "that creature".to_string()
            ))
        );
    }

    #[test]
    fn definite_card_target_is_a_tagged_reference() {
        let TargetAst::Tagged(tag, Some(span)) = parse("the card") else {
            panic!("expected definite card reference");
        };
        assert_eq!(tag.as_str(), IT_TAG);
        assert_eq!(
            crate::runtime_backend::util::source_reference_surface_for_span(Some(span)),
            Some(SourceReferenceSurface::ThisPermanentType(
                "the card".to_string()
            ))
        );
    }

    #[test]
    fn sacrificed_object_target_is_a_typed_tagged_reference() {
        let TargetAst::Tagged(tag, Some(span)) = parse("the sacrificed creature") else {
            panic!("expected tagged sacrificed-object reference");
        };
        assert_eq!(tag.as_str(), IT_TAG);
        assert_eq!(
            crate::runtime_backend::util::sacrificed_object_kind_for_span(Some(span)),
            Some(SacrificedObjectKind::Creature)
        );
    }

    #[test]
    fn named_possessive_source_target_preserves_short_name_surface() {
        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Casey Jones, Asphalt Hooligan",
            || {
                let TargetAst::Source(Some(span)) = parse("Casey Jones's") else {
                    panic!("expected named possessive to resolve to the source");
                };
                assert_eq!(
                    crate::runtime_backend::util::source_reference_surface_for_span(Some(span)),
                    Some(SourceReferenceSurface::ShortName("Casey Jones".to_string()))
                );
            },
        );
    }

    #[test]
    fn full_name_possessive_source_target_preserves_full_name_surface() {
        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Tifa Lockhart",
            || {
                let TargetAst::Source(Some(span)) = parse("Tifa Lockhart's") else {
                    panic!("expected named possessive to resolve to the source");
                };
                assert_eq!(
                    crate::runtime_backend::util::source_reference_surface_for_span(Some(span)),
                    Some(SourceReferenceSurface::FullName(
                        "Tifa Lockhart".to_string()
                    ))
                );
            },
        );
    }
}
