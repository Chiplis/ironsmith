use super::*;

pub(super) fn classify_target(
    tokens: &[OwnedLexToken],
    zone: ReturnZoneShape,
) -> Option<ReturnTargetShape> {
    if zone == ReturnZoneShape::Hand
        && let Some(source_subtype) = paired_source_and_exiled(tokens)
    {
        return Some(ReturnTargetShape::PairedSourceAndExiled { source_subtype });
    }
    if let Some((_, tail)) = split_phrase(tokens, &["and"])
        && starts_multi_target(trim_lexed_commas(tail))
    {
        return Some(ReturnTargetShape::MultiTargetUnsupported);
    }
    let has_target = marker_anywhere(tokens, primitives::kw("target"));
    let has_exiled_cards = marker_anywhere(tokens, primitives::kw("exiled"))
        && (marker_anywhere(tokens, primitives::kw("cards"))
            || tokens
                .first()
                .is_some_and(|token| token.is_word("all") || token.is_word("each")));
    if !has_target && has_exiled_cards {
        let quantifier_stripped =
            primitives::parse_prefix(tokens, alt((primitives::kw("all"), primitives::kw("each"))))
                .map(|(_, rest)| rest)
                .unwrap_or(tokens);
        let parsed_count = leaf::parse_leaf_choice_count_prefix_tokens(quantifier_stripped);
        let filter_tokens = parsed_count
            .as_ref()
            .and_then(|parsed| quantifier_stripped.get(parsed.consumed..))
            .unwrap_or(quantifier_stripped);
        return Some(ReturnTargetShape::UntargetedExiledCards {
            filter_tokens: trim_lexed_commas(filter_tokens).to_vec(),
            count: parsed_count.map(|parsed| parsed.count),
        });
    }

    if let Some((set_quantifier_surface, rest)) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("all").value(ironsmith_core::SetQuantifierSurface::All),
            primitives::kw("each").value(ironsmith_core::SetQuantifierSurface::Each),
        )),
    ) {
        let raw_filter_tokens = trim_lexed_commas(rest).to_vec();
        let unsupported_qualifier = marker_anywhere(rest, primitives::kw("dealt"))
            || (marker_anywhere(rest, primitives::kw("without"))
                && marker_anywhere(rest, primitives::kw("counter")));
        let chosen_this_way = [
            (&["not", "chosen", "this", "way"][..], true),
            (&["that", "weren't", "chosen", "this", "way"][..], true),
            (&["that", "werent", "chosen", "this", "way"][..], true),
            (&["that", "were", "not", "chosen", "this", "way"][..], true),
            (&["chosen", "this", "way"][..], false),
            (&["that", "were", "chosen", "this", "way"][..], false),
            (&["that", "was", "chosen", "this", "way"][..], false),
        ];
        let (without_chosen, chosen_this_way_excluded) = split_suffix(rest, &chosen_this_way)
            .map(|(head, excluded)| (head.to_vec(), Some(excluded)))
            .unwrap_or_else(|| (rest.to_vec(), None));
        let chosen_type = [
            (&["of", "the", "chosen", "type"][..], false),
            (&["that", "are", "of", "the", "chosen", "type"][..], false),
            (&["that", "arent", "of", "the", "chosen", "type"][..], true),
            (&["that", "aren't", "of", "the", "chosen", "type"][..], true),
            (
                &["that", "are", "not", "of", "the", "chosen", "type"][..],
                true,
            ),
        ];
        let (without_type, chosen_type_flag) = split_suffix(&without_chosen, &chosen_type)
            .map(|(head, excluded)| (head.to_vec(), Some(excluded)))
            .unwrap_or((without_chosen, None));
        let (filter_tokens, discarded_or_cycled_this_turn_by) =
            match super::super::parse_cycled_or_discarded_this_turn_filter_tail_tokens(
                &without_type,
            )
            .ok()
            .flatten()
            {
                Some(tail) => (tail.base_tokens, Some(tail.player_filter)),
                None => (without_type, None),
            };
        return Some(ReturnTargetShape::All {
            set_quantifier_surface,
            raw_filter_tokens,
            filter_tokens: trim_lexed_commas(&filter_tokens).to_vec(),
            chosen_this_way_excluded,
            chosen_creature_type: chosen_type_flag == Some(false),
            excluded_chosen_creature_type: chosen_type_flag == Some(true),
            discarded_or_cycled_this_turn_by,
            unsupported_qualifier,
        });
    }

    let graveyard_or_exile_tails = [
        (
            &["from", "your", "graveyard", "or", "from", "exile"][..],
            false,
        ),
        (&["from", "your", "graveyard", "or", "exile"][..], false),
    ];
    let source_from_graveyard_or_exile_tokens = if zone == ReturnZoneShape::Battlefield {
        split_suffix(tokens, &graveyard_or_exile_tails).map(|(head, _)| head.to_vec())
    } else {
        None
    };
    let graveyard_tails = [
        (&["from", "your", "graveyard"][..], false),
        (&["from", "its", "owner", "graveyard"][..], false),
        (&["from", "its", "owners", "graveyard"][..], false),
        (&["from", "its", "owner's", "graveyard"][..], false),
        (&["from", "its", "owners'", "graveyard"][..], false),
    ];
    let source_from_graveyard_tokens = if source_from_graveyard_or_exile_tokens.is_none()
        && zone == ReturnZoneShape::Battlefield
    {
        split_suffix(tokens, &graveyard_tails).map(|(head, _)| head.to_vec())
    } else {
        None
    };
    let (target_tokens, dynamic_count) = if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["that", "many"]),
            opt(primitives::kw("of")),
        )
            .void(),
    ) {
        (trim_lexed_commas(rest).to_vec(), true)
    } else {
        (tokens.to_vec(), false)
    };
    let (target_tokens, top_only) = if let Some((_, rest)) =
        primitives::parse_prefix(&target_tokens, primitives::phrase(&["the", "top"]).void())
    {
        (trim_lexed_commas(rest).to_vec(), true)
    } else {
        (target_tokens, false)
    };
    let source_from_graveyard_tokens = source_from_graveyard_tokens.map(|tokens| {
        primitives::parse_prefix(&tokens, primitives::phrase(&["the", "top"]).void())
            .map(|(_, rest)| trim_lexed_commas(rest).to_vec())
            .unwrap_or(tokens)
    });
    Some(ReturnTargetShape::Singular {
        back_reference: is_return_back_reference_shape(&target_tokens),
        target_tokens,
        source_from_graveyard_tokens,
        source_from_graveyard_or_exile_tokens,
        dynamic_count,
        top_only,
    })
}
