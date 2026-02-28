use super::*;


pub(crate) fn parse_effect_chain(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let words = words(tokens);
    let starts_with_each_opponent =
        words.starts_with(&["each", "opponent"]) || words.starts_with(&["each", "opponents"]);
    let starts_with_each_player =
        words.starts_with(&["each", "player"]) || words.starts_with(&["each", "players"]);

    if tokens.first().is_some_and(|token| token.is_word("they"))
        && tokens.get(1).is_some_and(|token| token.is_word("may"))
    {
        let inner_tokens = &tokens[2..];
        let effects = parse_effect_chain_with_sentence_primitives(inner_tokens)?;
        return Ok(vec![EffectAst::MayByTaggedController {
            tag: TagKey::from("triggering"),
            effects,
        }]);
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if tokens.first().is_some_and(|token| token.is_word("may"))
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain(&stripped)?;
        return Ok(vec![EffectAst::May { effects }]);
    }

    if let Some(unless_action) = parse_or_action_clause(tokens)? {
        return Ok(vec![unless_action]);
    }

    parse_effect_chain_with_sentence_primitives(tokens)
}

pub(crate) fn parse_or_action_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let mut option_tokens = split_on_or(tokens);
    if option_tokens.len() != 2 {
        return Ok(None);
    }

    let normalize_option = |mut option: Vec<Token>| {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        trim_commas(&option).to_vec()
    };

    let first = normalize_option(option_tokens.remove(0));
    let second = normalize_option(option_tokens.remove(0));
    if first.is_empty() || second.is_empty() {
        return Ok(None);
    }

    let first_starts_effect = find_verb(&first).is_some_and(|(_, verb_idx)| verb_idx == 0)
        || has_effect_head_without_verb(&first);
    let second_starts_effect = find_verb(&second).is_some_and(|(_, verb_idx)| verb_idx == 0)
        || has_effect_head_without_verb(&second);
    if !first_starts_effect || !second_starts_effect {
        return Ok(None);
    }

    let first_effects = match parse_effect_chain_with_sentence_primitives(&first) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };
    let second_effects = match parse_effect_chain_with_sentence_primitives(&second) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };

    Ok(Some(EffectAst::UnlessAction {
        effects: first_effects,
        alternative: second_effects,
        player: PlayerAst::Implicit,
    }))
}

pub(crate) fn parse_effect_chain_with_sentence_primitives(
    tokens: &[Token],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = run_sentence_primitives(tokens, PRE_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    if let Some(effects) = run_sentence_primitives(tokens, POST_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    parse_effect_chain_inner(tokens)
}

pub(crate) fn parse_effect_chain_inner(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_search_library_sentence(tokens)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let raw_segments = split_effect_chain_on_and(tokens);
    let mut segments: Vec<Vec<Token>> = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segments.is_empty() {
            segments.push(segment);
            continue;
        }
        if !segment_has_effect_head(&segment) {
            if let Some(previous) = segments.last()
                && let Some(expanded) = expand_missing_verb_segment(previous, &segment)
            {
                segments.push(expanded);
                continue;
            }
            let last = segments.last_mut().expect("non-empty segments");
            last.push(Token::Word("and".to_string(), TextSpan::synthetic()));
            last.extend(segment);
            continue;
        }
        segments.push(segment);
    }
    while segments.len() > 1 && !segment_has_effect_head(&segments[0]) {
        let mut first = segments.remove(0);
        first.push(Token::Word("and".to_string(), TextSpan::synthetic()));
        let mut next = segments.remove(0);
        first.append(&mut next);
        segments.insert(0, first);
    }
    // Split segments on ", then" when the part after "then" doesn't
    // back-reference the first part (no "that", "it", "them", "its").
    // This handles patterns like "discard your hand, then draw four cards".
    segments = split_segments_on_comma_then(segments);
    segments = split_segments_on_comma_effect_head(segments);
    segments = expand_segments_with_comma_action_clauses(segments);
    segments = expand_segments_with_multi_create_clauses(segments);
    let mut carried_context: Option<CarryContext> = None;
    for segment in segments {
        let segment_effects =
            if let Some(effects) = parse_sentence_return_with_counters_on_it(&segment)? {
                Some(effects)
            } else if let Some(effects) =
                parse_sentence_put_onto_battlefield_with_counters_on_it(&segment)?
            {
                Some(effects)
            } else {
                parse_sentence_exile_source_with_counters(&segment)?
            };
        if let Some(segment_effects) = segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) = parse_search_library_sentence(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) = parse_cant_effect_sentence(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        let mut effect = parse_effect_clause_with_trailing_if(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player_with_clause(&mut effect, context, &segment);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        effects.push(effect);
    }
    // If an "each player ..." clause is followed by additional implicit per-player
    // clauses that reference "it/that <object>", we must keep them inside the same
    // per-player iteration. Otherwise, tag-based "it" references will be overwritten
    // across players (for example: Duskmantle Seer).
    collapse_for_each_player_it_tag_followups(&mut effects);
    collapse_token_copy_next_end_step_exile_followup(&mut effects, tokens);
    collapse_token_copy_end_of_combat_exile_followup(&mut effects, tokens);
    Ok(effects)
}

pub(crate) fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(crate) fn parse_effect_clause_with_trailing_if(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let Some(if_idx) = tokens.iter().rposition(|token| token.is_word("if")) else {
        return parse_effect_clause(tokens);
    };
    if if_idx == 0 || if_idx + 1 >= tokens.len() {
        return parse_effect_clause(tokens);
    }

    let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
    if predicate_tokens.is_empty() {
        return parse_effect_clause(tokens);
    }
    let Ok(predicate) = parse_predicate(&predicate_tokens) else {
        return parse_effect_clause(tokens);
    };
    if !matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
    ) {
        return parse_effect_clause(tokens);
    }

    let leading = trim_commas(&tokens[..if_idx]);
    if leading.is_empty() {
        return parse_effect_clause(tokens);
    }
    let base_effect = if let Ok(effect) = parse_effect_clause(&leading) {
        effect
    } else if let Some(effect) = parse_simple_lose_ability_clause(&leading)? {
        effect
    } else if let Some(effect) = parse_simple_gain_ability_clause(&leading)? {
        effect
    } else {
        return parse_effect_clause(tokens);
    };

    Ok(EffectAst::Conditional {
        predicate,
        if_true: vec![base_effect],
        if_false: Vec::new(),
    })
}

pub(crate) fn is_beginning_of_end_step_words(words: &[&str]) -> bool {
    words
        .windows(5)
        .any(|window| window == ["beginning", "of", "the", "end", "step"])
        || words
            .windows(5)
            .any(|window| window == ["beginning", "of", "next", "end", "step"])
        || words
            .windows(6)
            .any(|window| window == ["beginning", "of", "the", "next", "end", "step"])
}

pub(crate) fn is_end_of_combat_words(words: &[&str]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["end", "of", "combat"])
}

pub(crate) fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[Token],
) {
    let chain_words = words(tokens);
    if !chain_words.contains(&"exile")
        || !chain_words.contains(&"token")
        || !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::Exile { target, .. },
            ) => target_is_generic_token_filter(target),
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                exile_at_next_end_step,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                exile_at_next_end_step,
                ..
            } => {
                *exile_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[Token],
) {
    let chain_words = words(tokens);
    if !chain_words.contains(&"exile")
        || !chain_words.contains(&"token")
        || !is_end_of_combat_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. }
                | EffectAst::CreateTokenCopyFromSource { .. }
                | EffectAst::CreateTokenWithMods { .. },
                EffectAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. }
                | EffectAst::CreateTokenCopyFromSource { .. }
                | EffectAst::CreateTokenWithMods { .. },
                EffectAst::Exile { target, .. },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. }
                | EffectAst::CreateTokenCopyFromSource { .. }
                | EffectAst::CreateTokenWithMods { .. },
                EffectAst::ExileThatTokenAtEndOfCombat,
            ) => true,
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                exile_at_end_of_combat,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                exile_at_end_of_combat,
                ..
            }
            | EffectAst::CreateTokenWithMods {
                exile_at_end_of_combat,
                ..
            } => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn expand_segments_with_comma_action_clauses(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let segment_words = words(&segment);
        let looks_like_sac_discard_chain = (segment_words.contains(&"sacrifice")
            || segment_words.contains(&"sacrifices"))
            && (segment_words.contains(&"discard") || segment_words.contains(&"discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<Token>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_commas(&raw_part).to_vec();
            while part.first().is_some_and(|token| token.is_word("and")) {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn starts_like_create_fragment(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    let starts_like_count = words.first().is_some_and(|word| {
        matches!(
            *word,
            "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six"
        )
    }) || words.first().is_some_and(|word| {
        parse_number(&[Token::Word((*word).to_string(), TextSpan::synthetic())]).is_some()
    }) || words
        .first()
        .is_some_and(|word| word.contains('/') || word == &"x");
    starts_like_count && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

pub(crate) fn expand_segments_with_multi_create_clauses(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb(&segment) else {
            expanded.push(segment);
            continue;
        };
        let segment_words = words(&segment);
        let has_token_rules_tail = segment_words.windows(3).any(|window| {
            matches!(
                window,
                ["when", "this", "token"] | ["whenever", "this", "token"]
            )
        }) || segment_words.windows(2).any(|window| {
            matches!(
                window,
                ["this", "token"] | ["that", "token"] | ["those", "tokens"]
            )
        }) || segment_words
            .windows(2)
            .any(|window| matches!(window, ["it", "has"] | ["they", "have"]));
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let token_mentions = segment_words
            .into_iter()
            .filter(|word| matches!(*word, "token" | "tokens"))
            .count();
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<Token>> = Vec::new();
        for part in comma_parts {
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(&words(previous))
                && starts_with_inline_token_rules_tail(&words(&part))
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(Token::Comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(Token::Comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_missing_verb_segment(previous: &[Token], segment: &[Token]) -> Option<Vec<Token>> {
    let (verb, verb_idx) = find_verb(previous)?;
    match verb {
        Verb::Deal => {
            let segment_words = words(segment);
            if parse_value(segment).is_none() || !segment_words.contains(&"damage") {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = words(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

pub(crate) fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        _ => None,
    }
}

pub(crate) fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

pub(crate) fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::Exile { target, .. } | EffectAst::ExileUntilSourceLeaves { target, .. } =
        effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::ExileAll { filter, .. } = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }

    let player = match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub(crate) fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. } => matches!(*player, PlayerAst::Implicit),
        _ => false,
    }
}

pub(crate) fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => match effect {
            EffectAst::Draw { player, .. }
            | EffectAst::DiscardHand { player }
            | EffectAst::Discard { player, .. }
            | EffectAst::GainLife { player, .. }
            | EffectAst::LoseLife { player, .. }
            | EffectAst::Scry { player, .. }
            | EffectAst::Surveil { player, .. }
            | EffectAst::Mill { player, .. }
            | EffectAst::PoisonCounters { player, .. }
            | EffectAst::EnergyCounters { player, .. }
            | EffectAst::RevealTop { player }
            | EffectAst::RevealHand { player }
            | EffectAst::PutIntoHand { player, .. } => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = carried_player;
                }
            }
            _ => {}
        },
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

pub(crate) fn clause_words_for_carry(tokens: &[Token]) -> Vec<&str> {
    let mut clause_words = words(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }
    clause_words
}

pub(crate) fn should_skip_draw_player_carry(
    effect: &EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[Token],
) -> bool {
    let clause_words = clause_words_for_carry(clause_tokens);
    match carried_context {
        CarryContext::Player(_) => {
            let EffectAst::Draw { player, .. } = effect else {
                return false;
            };
            if !matches!(*player, PlayerAst::Implicit) {
                return false;
            }
            matches!(clause_words.first().copied(), Some("draw"))
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::Draw {
                    player: PlayerAst::Implicit,
                    ..
                } | EffectAst::Scry {
                    player: PlayerAst::Implicit,
                    ..
                } | EffectAst::Surveil {
                    player: PlayerAst::Implicit,
                    ..
                }
            );
            if !is_implicit_vision_effect {
                return false;
            }
            matches!(
                clause_words.first().copied(),
                Some("draw" | "scry" | "surveil")
            )
        }
    }
}

pub(crate) fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[Token],
) {
    if should_skip_draw_player_carry(effect, carried_context, clause_tokens) {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

pub(crate) fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::Draw {
            player: effect_player,
            ..
        }
        | EffectAst::DiscardHand {
            player: effect_player,
        }
        | EffectAst::Discard {
            player: effect_player,
            ..
        }
        | EffectAst::GainLife {
            player: effect_player,
            ..
        }
        | EffectAst::LoseLife {
            player: effect_player,
            ..
        }
        | EffectAst::Sacrifice {
            player: effect_player,
            ..
        }
        | EffectAst::Scry {
            player: effect_player,
            ..
        }
        | EffectAst::Surveil {
            player: effect_player,
            ..
        }
        | EffectAst::Mill {
            player: effect_player,
            ..
        }
        | EffectAst::PoisonCounters {
            player: effect_player,
            ..
        }
        | EffectAst::EnergyCounters {
            player: effect_player,
            ..
        }
        | EffectAst::RevealTop {
            player: effect_player,
        }
        | EffectAst::RevealHand {
            player: effect_player,
        }
        | EffectAst::PutIntoHand {
            player: effect_player,
            ..
        }
        | EffectAst::PayMana {
            player: effect_player,
            ..
        }
        | EffectAst::PayEnergy {
            player: effect_player,
            ..
        }
        | EffectAst::AddMana {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaScaled {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyOneColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaChosenColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaFromLandCouldProduce {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaCommanderIdentity {
            player: effect_player,
            ..
        }
        | EffectAst::SearchLibrary {
            player: effect_player,
            ..
        }
        | EffectAst::ShuffleGraveyardIntoLibrary {
            player: effect_player,
        }
        | EffectAst::ShuffleLibrary {
            player: effect_player,
        }
        | EffectAst::CreateToken {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenCopy {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenWithMods {
            player: effect_player,
            ..
        }
        | EffectAst::CopySpell {
            player: effect_player,
            ..
        }
        | EffectAst::RetargetStackObject {
            chooser: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::UnlessPays { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            for nested in effects {
                bind_implicit_player_context(nested, player);
            }
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            for nested in effects {
                bind_implicit_player_context(nested, player);
            }
            for nested in alternative {
                bind_implicit_player_context(nested, player);
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            for nested in if_true {
                bind_implicit_player_context(nested, player);
            }
            for nested in if_false {
                bind_implicit_player_context(nested, player);
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_leading_player_may(tokens: &[Token]) -> Option<PlayerAst> {
    let mut words = words(tokens);
    while words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        words.remove(0);
    }
    if words.len() < 2 {
        return None;
    }

    if words.starts_with(&["you", "may"]) {
        return Some(PlayerAst::You);
    }
    if words.starts_with(&["target", "opponent", "may"])
        || words.starts_with(&["target", "opponents", "may"])
    {
        return Some(PlayerAst::TargetOpponent);
    }
    if words.starts_with(&["target", "player", "may"])
        || words.starts_with(&["target", "players", "may"])
    {
        return Some(PlayerAst::Target);
    }
    if words.starts_with(&["that", "player", "may"])
        || words.starts_with(&["that", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "controller"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsController);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "owner"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsOwner);
    }
    if words.starts_with(&["the", "player", "may"]) || words.starts_with(&["the", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if words.starts_with(&["defending", "player", "may"]) {
        return Some(PlayerAst::Defending);
    }
    if words.starts_with(&["attacking", "player", "may"])
        || words.starts_with(&["the", "attacking", "player", "may"])
    {
        return Some(PlayerAst::Attacking);
    }
    if words.starts_with(&["its", "controller", "may"])
        || words.starts_with(&["their", "controller", "may"])
    {
        return Some(PlayerAst::ItsController);
    }
    if words.starts_with(&["its", "owner", "may"]) || words.starts_with(&["their", "owner", "may"])
    {
        return Some(PlayerAst::ItsOwner);
    }
    if words.starts_with(&["opponent", "may"])
        || words.starts_with(&["opponents", "may"])
        || words.starts_with(&["an", "opponent", "may"])
    {
        return Some(PlayerAst::Opponent);
    }

    None
}

pub(crate) fn remove_first_word(tokens: &[Token], word: &str) -> Vec<Token> {
    let mut removed = false;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        if !removed && token.is_word(word) {
            removed = true;
            continue;
        }
        out.push(token.clone());
    }
    out
}

pub(crate) fn remove_through_first_word(tokens: &[Token], word: &str) -> Vec<Token> {
    let mut seen = false;
    let mut out = Vec::new();
    for token in tokens {
        if !seen {
            if token.is_word(word) {
                seen = true;
            }
            continue;
        }
        out.push(token.clone());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Flip,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Shuffle,
    Reorder,
    Pay,
    Goad,
}

pub(crate) type ClausePrimitiveParser = fn(&[Token]) -> Result<Option<EffectAst>, CardTextError>;

pub(crate) struct ClausePrimitive {
    pub(crate) parser: ClausePrimitiveParser,
}

pub(crate) fn parse_retarget_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_new_targets_clause(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_change_target_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}

pub(crate) fn parse_choose_new_targets_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let is_choose = clause_words.starts_with(&["choose", "new", "targets", "for"])
        || clause_words.starts_with(&["chooses", "new", "targets", "for"]);
    if !is_choose {
        return Ok(None);
    }

    let mut tail_tokens = &tokens[4..];
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    if let Some(if_idx) = tail_tokens.iter().position(|token| token.is_word("if")) {
        tail_tokens = &tail_tokens[..if_idx];
    }

    let tail_words = words(tail_tokens);
    if tail_words.starts_with(&["it"])
        || tail_words.starts_with(&["them"])
        || tail_words.starts_with(&["the", "copy"])
        || tail_words.starts_with(&["that", "copy"])
        || tail_words.starts_with(&["the", "spell"])
        || tail_words.starts_with(&["that", "spell"])
    {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tail_tokens));
        return Ok(Some(EffectAst::RetargetStackObject {
            target,
            mode: RetargetModeAst::All,
            chooser: PlayerAst::Implicit,
            require_change: false,
            new_target_restriction: None,
        }));
    }

    let (count, base_tokens, explicit_target) = if tail_words.starts_with(&["any", "number", "of"])
    {
        (Some(ChoiceCount::any_number()), &tail_tokens[3..], false)
    } else if tail_words.starts_with(&["target"]) {
        (None, &tail_tokens[1..], true)
    } else {
        (None, tail_tokens, false)
    };

    let mut filter = parse_stack_retarget_filter(base_tokens)?;
    if base_tokens.iter().any(|token| token.is_word("other")) {
        filter.other = true;
    }

    let mut target = TargetAst::Object(
        filter,
        if explicit_target {
            span_from_tokens(tail_tokens)
        } else {
            None
        },
        None,
    );
    if let Some(count) = count {
        target = TargetAst::WithCount(Box::new(target), count);
    }

    Ok(Some(EffectAst::RetargetStackObject {
        target,
        mode: RetargetModeAst::All,
        chooser: PlayerAst::Implicit,
        require_change: false,
        new_target_restriction: None,
    }))
}

pub(crate) fn parse_change_target_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() || clause_words[0] != "change" {
        return Ok(None);
    }

    if let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        let unless_tokens = trim_commas(&tokens[unless_idx + 1..]);
        let Some(inner) = parse_change_target_clause_inner(&main_tokens)? else {
            return Ok(None);
        };
        let (player, mana) = parse_unless_pays_clause(&unless_tokens)?;
        return Ok(Some(EffectAst::UnlessPays {
            effects: vec![inner],
            player,
            mana,
        }));
    }

    parse_change_target_clause_inner(tokens)
}

pub(crate) fn parse_change_target_clause_inner(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let (mode, after_of_idx) = if clause_words.starts_with(&["change", "the", "target", "of"]) {
        (RetargetModeAst::All, 4)
    } else if clause_words.starts_with(&["change", "the", "targets", "of"]) {
        (RetargetModeAst::All, 4)
    } else if clause_words.starts_with(&["change", "a", "target", "of"]) {
        (RetargetModeAst::All, 4)
    } else {
        return Ok(None);
    };

    if tokens.len() <= after_of_idx {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let mut tail_tokens = trim_commas(&tokens[after_of_idx..]).to_vec();
    let mut fixed_target: Option<TargetAst> = None;
    if let Some(to_idx) = tail_tokens.iter().position(|token| token.is_word("to")) {
        let to_tail = &tail_tokens[to_idx + 1..];
        let to_words = words(to_tail);
        if to_words.starts_with(&["this"]) {
            fixed_target = Some(TargetAst::Source(span_from_tokens(to_tail)));
            tail_tokens.truncate(to_idx);
        }
    }

    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;
    let tail_words = words(&tail_tokens);

    if tail_words
        .windows(4)
        .any(|w| w == ["with", "a", "single", "target"])
    {
        filter = filter.target_count_exact(1);
    }
    if tail_words
        .windows(5)
        .any(|w| w == ["targets", "only", "a", "single", "creature"])
    {
        filter = filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1);
    }
    if tail_words
        .windows(4)
        .any(|w| w == ["targets", "only", "this", "creature"])
        || tail_words
            .windows(4)
            .any(|w| w == ["targets", "only", "this", "permanent"])
    {
        filter = filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1);
    }
    if tail_words
        .windows(3)
        .any(|w| w == ["targets", "only", "you"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }
    if tail_words
        .windows(4)
        .any(|w| w == ["targets", "only", "a", "player"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1);
    }
    if tail_words
        .windows(5)
        .any(|w| w == ["if", "that", "target", "is", "you"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if let Some(fixed) = fixed_target {
        RetargetModeAst::OneToFixed { target: fixed }
    } else {
        mode
    };

    Ok(Some(EffectAst::RetargetStackObject {
        target,
        mode,
        chooser: PlayerAst::Implicit,
        require_change: true,
        new_target_restriction: None,
    }))
}

pub(crate) fn parse_unless_pays_clause(
    tokens: &[Token],
) -> Result<(PlayerAst, Vec<ManaSymbol>), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing unless clause".to_string(),
        ));
    }
    let pays_idx = tokens
        .iter()
        .position(|token| token.is_word("pays"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing pays keyword (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let player_tokens = trim_commas(&tokens[..pays_idx]);
    let player = match parse_subject(&player_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let mut mana = Vec::new();
    for token in tokens[pays_idx + 1..].iter() {
        let Some(word) = token.as_word() else {
            continue;
        };
        match parse_mana_symbol(word) {
            Ok(symbol) => mana.push(symbol),
            Err(_) => break,
        }
    }

    if mana.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana cost (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok((player, mana))
}

pub(crate) fn parse_stack_retarget_filter(tokens: &[Token]) -> Result<ObjectFilter, CardTextError> {
    let words = words(tokens);
    let has_ability = words
        .iter()
        .any(|word| *word == "ability" || *word == "abilities");
    let has_spell = words
        .iter()
        .any(|word| *word == "spell" || *word == "spells");
    let has_activated = words.iter().any(|word| *word == "activated");
    let has_instant = words.iter().any(|word| *word == "instant");
    let has_sorcery = words.iter().any(|word| *word == "sorcery");

    let mut filter = if has_activated && has_ability {
        ObjectFilter::activated_ability()
    } else if has_ability && has_spell {
        ObjectFilter::spell_or_ability()
    } else if has_ability {
        ObjectFilter::ability()
    } else if (has_instant || has_sorcery) && has_spell {
        ObjectFilter::instant_or_sorcery()
    } else if has_spell {
        ObjectFilter::spell()
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported retarget target clause (clause: '{}')",
            words.join(" ")
        )));
    };

    if words.iter().any(|word| *word == "other") {
        filter.other = true;
    }

    Ok(filter)
}

pub(crate) fn run_clause_primitives(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    const PRIMITIVES: &[ClausePrimitive] = &[
        ClausePrimitive {
            parser: parse_retarget_clause,
        },
        ClausePrimitive {
            parser: parse_copy_spell_clause,
        },
        ClausePrimitive {
            parser: parse_win_the_game_clause,
        },
        ClausePrimitive {
            parser: parse_deal_damage_equal_to_power_clause,
        },
        ClausePrimitive {
            parser: parse_fight_clause,
        },
        ClausePrimitive {
            parser: parse_clash_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_target_players_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_opponent_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_player_clause,
        },
        ClausePrimitive {
            parser: parse_double_counters_clause,
        },
        ClausePrimitive {
            parser: parse_distribute_counters_clause,
        },
        ClausePrimitive {
            parser: parse_until_end_of_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_until_your_next_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_cast_or_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_next_damage_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_all_damage_clause,
        },
        ClausePrimitive {
            parser: parse_can_attack_as_though_no_defender_clause,
        },
        ClausePrimitive {
            parser: parse_can_block_additional_creature_this_turn_clause,
        },
        ClausePrimitive {
            parser: parse_attack_this_turn_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_be_blocked_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_block_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_until_duration_triggered_clause,
        },
        ClausePrimitive {
            parser: parse_keyword_mechanic_clause,
        },
        ClausePrimitive {
            parser: parse_connive_clause,
        },
        ClausePrimitive {
            parser: parse_choose_target_and_verb_clause,
        },
        ClausePrimitive {
            parser: parse_verb_first_clause,
        },
    ];

    for primitive in PRIMITIVES {
        if let Some(effect) = (primitive.parser)(tokens)? {
            return Ok(Some(effect));
        }
    }
    Ok(None)
}

pub(crate) fn parse_attack_this_turn_if_able_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause_words = words(tokens);
    let Some(attack_idx) = tokens
        .iter()
        .position(|token| token.is_word("attack") || token.is_word("attacks"))
    else {
        return Ok(None);
    };
    if attack_idx == 0 {
        return Ok(None);
    }

    let tail_words = words(&tokens[attack_idx..]);
    if tail_words != ["attack", "this", "turn", "if", "able"]
        && tail_words != ["attacks", "this", "turn", "if", "able"]
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&subject_tokens)?;
    let ability = StaticAbility::must_attack();

    if starts_with_target_indicator(&subject_tokens) {
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities: vec![ability],
            duration: Until::EndOfTurn,
        }));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in attacks-if-able clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities: vec![ability],
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_must_be_blocked_if_able_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause_words = words(tokens);
    let Some(must_idx) = tokens.iter().position(|token| token.is_word("must")) else {
        return Ok(None);
    };
    if must_idx == 0 {
        return Ok(None);
    }

    let tail_words = words(&tokens[must_idx..]);
    let has_supported_tail = tail_words == ["must", "be", "blocked", "if", "able"]
        || tail_words == ["must", "be", "blocked", "this", "turn", "if", "able"];
    if !has_supported_tail {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..must_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if starts_with_target_indicator(&subject_tokens) {
        // We only support source/tagged subjects here; explicit "target ..." needs
        // a target+restriction sequence that this single-clause parser cannot encode.
        return Ok(None);
    }

    let attacker_target = parse_target_phrase(&subject_tokens)?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in must-be-blocked clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Cant {
        restriction: crate::effect::Restriction::must_block_specific_attacker(
            ObjectFilter::creature(),
            attacker_filter,
        ),
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_must_block_if_able_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause_words = words(tokens);

    // "All creatures able to block target creature this turn do so."
    if clause_words.starts_with(&["all", "creatures", "able", "to", "block"]) {
        let mut tail_tokens = trim_commas(&tokens[5..]);
        let tail_words = words(&tail_tokens);
        if !tail_words.ends_with(&["do", "so"]) {
            return Ok(None);
        }
        tail_tokens = trim_commas(&tail_tokens[..tail_tokens.len().saturating_sub(2)]);

        let (duration, attacker_tokens) =
            if let Some((duration, remainder)) = parse_restriction_duration(&tail_tokens)? {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, tail_tokens.to_vec())
            };
        let attacker_tokens = trim_commas(&attacker_tokens);
        if attacker_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attacker in must-block clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let attacker_target = parse_target_phrase(&attacker_tokens)?;
        let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported attacker target in must-block clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        return Ok(Some(EffectAst::Cant {
            restriction: crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                attacker_filter,
            ),
            duration,
        }));
    }

    // "<subject> blocks <attacker> this turn if able."
    let Some(block_idx) = tokens
        .iter()
        .position(|token| token.is_word("block") || token.is_word("blocks"))
    else {
        return Ok(None);
    };
    if block_idx == 0 || block_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..block_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let blockers_filter = parse_subject_object_filter(&subject_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported blocker subject in must-block clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let mut tail_tokens = trim_commas(&tokens[block_idx + 1..]);
    let tail_words = words(&tail_tokens);
    if !tail_words.ends_with(&["if", "able"]) {
        return Ok(None);
    }
    tail_tokens = trim_commas(&tail_tokens[..tail_tokens.len().saturating_sub(2)]);

    let (duration, attacker_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, tail_tokens.to_vec())
        };
    let attacker_tokens = trim_commas(&attacker_tokens);
    if attacker_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attacker in must-block clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let attacker_target = parse_target_phrase(&attacker_tokens)?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker target in must-block clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Cant {
        restriction: crate::effect::Restriction::must_block_specific_attacker(
            blockers_filter,
            attacker_filter,
        ),
        duration,
    }))
}

pub(crate) fn parse_until_duration_triggered_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let has_leading_duration = clause_words.starts_with(&["until", "end", "of", "turn"])
        || clause_words.starts_with(&["until", "your", "next", "turn"])
        || clause_words.starts_with(&["until", "your", "next", "upkeep"])
        || clause_words.starts_with(&["until", "your", "next", "untap", "step"])
        || clause_words.starts_with(&["during", "your", "next", "untap", "step"]);
    if !has_leading_duration {
        return Ok(None);
    }

    let Some((duration, trigger_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing trigger after duration clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trigger_words = words(&trigger_tokens);
    let looks_like_trigger = trigger_words
        .first()
        .is_some_and(|word| *word == "when" || *word == "whenever")
        || trigger_words.starts_with(&["at", "the"]);
    if !looks_like_trigger {
        return Ok(None);
    }

    let (trigger, effects, max_triggers_per_turn) = match parse_triggered_line(&trigger_tokens)? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => (trigger, effects, max_triggers_per_turn),
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported duration-triggered clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    let (compiled_effects, choices) = compile_trigger_effects(Some(&trigger), &effects)?;
    let trigger_text = trigger_words.join(" ");
    let ability = Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: compile_trigger_spec(trigger),
            effects: compiled_effects,
            choices,
            intervening_if: max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(trigger_text.clone()),
    };
    let granted = StaticAbility::grant_object_ability_for_filter(
        ObjectFilter::source(),
        ability,
        trigger_text,
    );

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target: TargetAst::Source(span_from_tokens(tokens)),
        abilities: vec![granted],
        duration,
    }))
}

pub(crate) fn parse_power_reference_word_count(words: &[&str]) -> Option<usize> {
    if words.starts_with(&["its", "power"]) || words.starts_with(&["that", "power"]) {
        return Some(2);
    }
    if words.starts_with(&["this", "source", "power"])
        || words.starts_with(&["this", "creature", "power"])
        || words.starts_with(&["that", "creature", "power"])
        || words.starts_with(&["that", "objects", "power"])
    {
        return Some(3);
    }
    None
}

pub(crate) fn is_damage_source_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _)
    )
}

pub(crate) fn parse_deal_damage_equal_to_power_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(deal_idx) = tokens
        .iter()
        .position(|token| token.is_word("deal") || token.is_word("deals"))
    else {
        return Ok(None);
    };
    if deal_idx == 0 {
        return Ok(None);
    }

    let source_tokens = trim_commas(&tokens[..deal_idx]);

    let rest = trim_commas(&tokens[deal_idx + 1..]);
    if rest.is_empty() || !rest[0].is_word("damage") {
        return Ok(None);
    }

    let Some(equal_idx) = rest
        .windows(2)
        .position(|window| window[0].is_word("equal") && window[1].is_word("to"))
    else {
        return Ok(None);
    };

    let source = parse_target_phrase(&source_tokens)?;
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let power_ref_words = words(&rest[equal_idx + 2..]);
    let Some(power_ref_len) = parse_power_reference_word_count(&power_ref_words) else {
        return Ok(None);
    };

    let tail_after_power = trim_commas(&rest[equal_idx + 2 + power_ref_len..]);
    let pre_equal_words = words(&rest[..equal_idx]);

    let target = if pre_equal_words == ["damage"] {
        let mut target_tokens = tail_after_power.as_slice();
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage target after power reference (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut normalized_target_tokens = target_tokens;
        let target_words = words(target_tokens);
        if target_words.starts_with(&["each", "of"]) {
            let each_of_tokens = &target_tokens[2..];
            let each_of_words = words(each_of_tokens);
            if each_of_words.iter().any(|word| *word == "target") {
                normalized_target_tokens = each_of_tokens;
            }
        }
        let normalized_target_words = words(normalized_target_tokens);
        if normalized_target_words.as_slice() == ["each", "player"]
            || normalized_target_words.as_slice() == ["each", "players"]
        {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if normalized_target_words.as_slice() == ["each", "opponent"]
            || normalized_target_words.as_slice() == ["each", "opponents"]
            || normalized_target_words.as_slice() == ["each", "other", "player"]
            || normalized_target_words.as_slice() == ["each", "other", "players"]
        {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        parse_target_phrase(normalized_target_tokens)?
    } else if pre_equal_words.starts_with(&["damage", "to"]) {
        let target_tokens = trim_commas(&rest[2..equal_idx]);
        let target_words = words(&target_tokens);
        if target_words.as_slice() == ["each", "player"]
            || target_words.as_slice() == ["each", "players"]
        {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if target_words.as_slice() == ["each", "opponent"]
            || target_words.as_slice() == ["each", "opponents"]
            || target_words.as_slice() == ["each", "other", "player"]
            || target_words.as_slice() == ["each", "other", "players"]
        {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if target_words == ["itself"] || target_words == ["it"] {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after self-damage power clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            source.clone()
        } else {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after explicit power-damage target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            parse_target_phrase(&target_tokens)?
        }
    } else {
        return Ok(None);
    };

    Ok(Some(EffectAst::DealDamageEqualToPower { source, target }))
}

pub(crate) fn parse_fight_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(fight_idx) = tokens
        .iter()
        .position(|token| token.is_word("fight") || token.is_word("fights"))
    else {
        return Ok(None);
    };

    if fight_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let right_tokens = trim_commas(&tokens[fight_idx + 1..]);
    if right_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let creature1 = if fight_idx == 0 {
        TargetAst::Source(None)
    } else {
        let left_tokens = trim_commas(&tokens[..fight_idx]);
        if left_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "fight clause requires two creatures (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(filter) = parse_for_each_object_subject(&left_tokens)? {
            let creature2 = parse_target_phrase(&right_tokens)?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::FightIterated { creature2 }],
            }));
        }
        parse_target_phrase(&left_tokens)?
    };
    let right_words = words(&right_tokens);
    let creature2 = if right_words == ["each", "other"] || right_words == ["one", "another"] {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&right_tokens))
    } else {
        parse_target_phrase(&right_tokens)?
    };

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::Fight {
        creature1,
        creature2,
    }))
}

pub(crate) fn parse_clash_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(first) = clause_words.first().copied() else {
        return Ok(None);
    };
    if first != "clash" && first != "clashes" {
        return Ok(None);
    }

    let mut tail = trim_commas(&tokens[1..]);
    if tail.first().is_some_and(|token| token.is_word("with")) {
        tail = trim_commas(&tail[1..]);
    }
    let tail_end = tail
        .iter()
        .position(|token| token.is_word("then") || matches!(token, Token::Comma(_)))
        .unwrap_or(tail.len());
    let tail = trim_commas(&tail[..tail_end]);
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing opponent in clash clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail_words: Vec<&str> = words(&tail)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let opponent = match tail_words.as_slice() {
        ["opponent"] => ClashOpponentAst::Opponent,
        ["target", "opponent"] => ClashOpponentAst::TargetOpponent,
        ["defending", "player"] => ClashOpponentAst::DefendingPlayer,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported clash target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    Ok(Some(EffectAst::Clash { opponent }))
}
