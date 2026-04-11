pub(crate) fn format_negated_restriction_display(tokens: &[OwnedLexToken]) -> String {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        match (words[idx], words.get(idx + 1).copied()) {
            ("cant", _) => {
                out.push("can't".to_string());
                idx += 1;
            }
            ("can", Some("not")) => {
                out.push("can't".to_string());
                idx += 2;
            }
            ("does", Some("not")) => {
                out.push("doesn't".to_string());
                idx += 2;
            }
            ("do", Some("not")) => {
                out.push("don't".to_string());
                idx += 2;
            }
            ("non", Some("phyrexian")) => {
                out.push("non-phyrexian".to_string());
                idx += 2;
            }
            _ => {
                out.push(words[idx].to_string());
                idx += 1;
            }
        }
    }
    out.join(" ")
}

pub(crate) fn parse_cant_restrictions(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(segment).join(" ")
                )));
            };
            restrictions.push(restriction);
        }

        if restrictions.is_empty() {
            return Ok(None);
        }
        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

pub(crate) fn parse_cant_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
    {
        return parse_cant_restriction_clause(&remainder);
    }

    if let Some(parsed) = parse_player_negated_restriction_clause(tokens)? {
        return Ok(Some(parsed));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let restriction = if let Some(parsed) = parse_cant_cast_restriction_words(&normalized) {
        parsed
    } else {
        if let [
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
            parity,
            "mana",
            "values",
        ] = normalized.as_slice()
        {
            let parity = match *parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return parse_negated_object_restriction_clause(tokens),
            };
            return Ok(Some(ParsedCantRestriction {
                restriction: Restriction::block(
                    ObjectFilter::creature()
                        .opponent_controls()
                        .with_mana_value_parity(parity),
                ),
                target: None,
            }));
        }
        match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Any),
            ["players", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::Any)
            }
            ["players", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::Any),
            [
                "players",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Any),
            ["damage", "cant", "be", "prevented"] => Restriction::prevent_damage(),
            ["you", "cant", "lose", "the", "game"] => Restriction::lose_game(PlayerFilter::You),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                Restriction::win_game(PlayerFilter::Opponent)
            }
            ["your", "life", "total", "cant", "change"] => {
                Restriction::change_life_total(PlayerFilter::You)
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
            [
                "each",
                "opponent",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
            ["you", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::You),
            ["you", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::You)
            }
            ["you", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::You),
            ["you", "cant", "become", "the", "monarch"]
            | ["you", "cant", "become", "monarch"]
            | ["you", "cant", "become", "the", "monarch", "this", "turn"]
            | ["you", "cant", "become", "monarch", "this", "turn"] => {
                Restriction::become_monarch(PlayerFilter::You)
            }
            ["they", "cant", "gain", "life"] | ["that", "player", "cant", "gain", "life"] => {
                Restriction::gain_life(PlayerFilter::IteratedPlayer)
            }
            ["opponents", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Opponent),
            _ => return parse_negated_object_restriction_clause(tokens),
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn parse_cant_cast_restriction_words(words: &[&str]) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some((player, used)) = parse_cant_cast_subject(words) {
        let tail = &words[used..];

        if let Some(spell_filter) = parse_cast_additional_limit_filter(tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }

        if tail.first() != Some(&"cant") {
            return None;
        }
        let cant_tail = &tail[1..];

        if cant_tail == ["cast", "spells"] || cant_tail == ["cast", "spells", "this", "turn"] {
            return Some(Restriction::cast_spells(player));
        }
        if cant_tail.len() >= 6
            && cant_tail[0] == "cast"
            && cant_tail[1] == "spells"
            && cant_tail[2] == "with"
            && cant_tail[4] == "mana"
            && cant_tail[5] == "values"
        {
            let parity = cant_tail[3];
            let parity = match parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return None,
            };
            return Some(Restriction::cast_spells_matching(
                player,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ));
        }
        if cant_tail == ["cast", "creature", "spells"]
            || cant_tail == ["cast", "creature", "spells", "this", "turn"]
        {
            return Some(Restriction::cast_creature_spells(player));
        }
        if cant_tail.first() == Some(&"cast") {
            let mut idx = 1usize;
            if let Some((spell_filter, used)) = parse_cast_limit_qualifier(&cant_tail[idx..]) {
                idx += used;
                if cant_tail.get(idx) == Some(&"spell") || cant_tail.get(idx) == Some(&"spells") {
                    idx += 1;
                    if cant_tail.get(idx) == Some(&"this")
                        && cant_tail.get(idx + 1) == Some(&"turn")
                    {
                        idx += 2;
                    }
                    if idx == cant_tail.len() {
                        return Some(Restriction::cast_spells_matching(player, spell_filter));
                    }
                }
            }
        }
        if let Some(spell_filter) = parse_cast_more_than_one_limit_filter(cant_tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }
        return None;
    }

    if let Some(spell_filter) = parse_cast_additional_limit_filter(words) {
        return Some(restriction_from_cast_limit_filter(
            PlayerFilter::Any,
            spell_filter,
        ));
    }

    None
}

fn parse_cant_cast_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if slice_starts_with(&words, &["that", "player"]) {
        return Some((PlayerFilter::IteratedPlayer, 2));
    }
    if slice_starts_with(&words, &["your", "opponents", "who", "have"]) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if slice_starts_with(&words, &["each", "player", "who", "has"]) {
        return Some((PlayerFilter::Any, 4));
    }
    if slice_starts_with(&words, &["each", "opponent", "who", "has"]) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if slice_starts_with(&words, &["your", "opponents"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    if slice_starts_with(&words, &["each", "player"]) {
        return Some((PlayerFilter::Any, 2));
    }
    if slice_starts_with(&words, &["each", "opponent"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    match words.first().copied() {
        Some("players") => Some((PlayerFilter::Any, 1)),
        Some("opponents") => Some((PlayerFilter::Opponent, 1)),
        Some("they") => Some((PlayerFilter::IteratedPlayer, 1)),
        Some("you") => Some((PlayerFilter::You, 1)),
        _ => None,
    }
}

fn parse_cast_more_than_one_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    if !matches!(words, ["cast", "more", "than", "one", ..]) {
        return None;
    }
    let mut idx = 4usize;
    let (spell_filter, consumed) = if words.get(idx) == Some(&"spell") {
        (ObjectFilter::default(), 0usize)
    } else {
        parse_cast_limit_qualifier(&words[idx..])?
    };
    idx += consumed;

    if words.get(idx) != Some(&"spell")
        || words.get(idx + 1) != Some(&"each")
        || words.get(idx + 2) != Some(&"turn")
        || idx + 3 != words.len()
    {
        return None;
    }

    Some(spell_filter)
}

fn parse_cast_additional_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    let mut idx = 0usize;
    if matches!(words, ["who", "has", ..]) {
        idx += 2;
    }

    if words.get(idx) != Some(&"cast") {
        return None;
    }
    idx += 1;
    if words
        .get(idx)
        .is_some_and(|word| *word == "a" || *word == "an")
    {
        idx += 1;
    }

    let (first_filter, first_used) = parse_cast_limit_qualifier(&words[idx..])?;
    idx += first_used;

    if words.get(idx) != Some(&"spell") {
        return None;
    }
    idx += 1;

    if words.get(idx) == Some(&"this") && words.get(idx + 1) == Some(&"turn") {
        idx += 2;
    }

    if words.get(idx) != Some(&"cant")
        || words.get(idx + 1) != Some(&"cast")
        || words.get(idx + 2) != Some(&"additional")
    {
        return None;
    }
    idx += 3;

    let (second_filter, second_used) = parse_cast_limit_qualifier(&words[idx..])?;
    if second_filter != first_filter {
        return None;
    }
    idx += second_used;

    if words.get(idx) != Some(&"spells") || idx + 1 != words.len() {
        return None;
    }

    Some(first_filter)
}

fn parse_cast_limit_qualifier(words: &[&str]) -> Option<(ObjectFilter, usize)> {
    let parse_non_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().without_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().without_subtype(subtype));
        }
        None
    };
    let parse_positive_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().with_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().with_subtype(subtype));
        }
        None
    };

    if let Some(first) = words.first().copied() {
        if let Some(term) =
            str_strip_prefix(first, "non-").or_else(|| str_strip_prefix(first, "non"))
            && !term.is_empty()
            && let Some(filter) = parse_non_term(term)
        {
            return Some((filter, 1));
        }
    }

    if words.len() >= 2
        && words[0] == "non"
        && let Some(filter) = parse_non_term(words[1])
    {
        return Some((filter, 2));
    }

    if let Some(first) = words.first().copied()
        && let Some(filter) = parse_positive_term(first)
    {
        let mut filters = vec![filter];
        let mut used = 1usize;
        while words
            .get(used)
            .is_some_and(|word| *word == "or" || *word == "and")
        {
            let Some(next_word) = words.get(used + 1).copied() else {
                break;
            };
            let Some(next_filter) = parse_positive_term(next_word) else {
                break;
            };
            filters.push(next_filter);
            used += 2;
        }
        if filters.len() == 1 {
            return Some((filters.pop().expect("single filter"), used));
        }
        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = filters;
        return Some((disjunction, used));
    }

    None
}

fn strip_static_restriction_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::ConditionExpr, Vec<OwnedLexToken>)>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if slice_starts_with(&normalized, &["during", "your", "turn"]) {
        let remainder = find_index(tokens, |token| token.is_comma())
            .map(|idx| trim_commas(&tokens[idx + 1..]).to_vec())
            .unwrap_or_else(|| trim_commas(&tokens[3..]).to_vec());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringYourTurn),
            remainder,
        )));
    }

    if slice_starts_with(&normalized, &["during", "combat"]) {
        let remainder = find_index(tokens, |token| token.is_comma())
            .map(|idx| trim_commas(&tokens[idx + 1..]).to_vec())
            .unwrap_or_else(|| trim_commas(&tokens[2..]).to_vec());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringCombat),
            remainder,
        )));
    }

    if slice_ends_with(&normalized, &["during", "your", "turn"]) {
        let cut = rfind_index(tokens, |token| token.is_word("during")).unwrap_or(tokens.len());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringYourTurn),
            trim_commas(&tokens[..cut]).to_vec(),
        )));
    }

    if slice_ends_with(&normalized, &["during", "combat"]) {
        let cut = rfind_index(tokens, |token| token.is_word("during")).unwrap_or(tokens.len());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringCombat),
            trim_commas(&tokens[..cut]).to_vec(),
        )));
    }

    if slice_starts_with(&normalized, &["as", "long", "as"]) {
        let Some(comma_idx) = find_index(tokens, |token| token.is_comma()) else {
            return Ok(None);
        };
        let condition_tokens = trim_commas(&tokens[3..comma_idx]);
        let condition = parse_static_condition_clause(&condition_tokens).or_else(|_| {
            let condition_words = normalize_cant_words(&condition_tokens);
            let normalized_condition = condition_words
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            match normalized_condition.as_slice() {
                ["this", "equipment", "is", "attached", "to", "a", "creature"]
                | ["this", "equipment", "is", "attached", "to", "creature"]
                | ["this", "permanent", "is", "attached", "to", "a", "creature"]
                | ["this", "permanent", "is", "attached", "to", "creature"] => {
                    Ok(crate::ConditionExpr::SourceIsEquipped)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported static condition clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))),
            }
        })?;
        return Ok(Some((
            condition,
            trim_commas(&tokens[comma_idx + 1..]).to_vec(),
        )));
    }

    Ok(None)
}

fn parse_player_negated_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let Some((player, target)) = parse_player_restriction_subject(&subject_tokens)? else {
        return Ok(None);
    };
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Ok(None);
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if let Some(spell_filter) = parse_cast_restriction_tail_filter(&remainder_words) {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells_matching(player, spell_filter),
            target,
        }));
    }
    if remainder_words.as_slice() == ["cast", "spells"] {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells(player),
            target,
        }));
    }
    if remainder_words.as_slice()
        == [
            "activate",
            "abilities",
            "that",
            "arent",
            "mana",
            "abilities",
        ]
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::activate_non_mana_abilities(player),
            target,
        }));
    }
    if slice_starts_with(&remainder_words, &["activate", "abilities", "of"]) {
        let Some(mut filter) =
            parse_card_type_list_filter(&remainder_words[3..], Some(Zone::Battlefield))
        else {
            return Ok(None);
        };
        filter.controller = Some(player);
        let restriction =
            if slice_ends_with(&remainder_words, &["unless", "theyre", "mana", "abilities"]) {
                Restriction::activate_non_mana_abilities_of(filter)
            } else {
                Restriction::activate_abilities_of(filter)
            };
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }

    Ok(None)
}

fn parse_player_restriction_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerFilter, Option<TargetAst>)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if starts_with_target_indicator(subject_tokens) {
        let target = parse_target_phrase(subject_tokens)?;
        if let TargetAst::Player(player, span) = &target {
            return Ok(Some((
                target_ast_player_filter(player.clone(), *span),
                Some(target),
            )));
        }
        return Ok(None);
    }

    let normalized_storage = normalize_cant_words(subject_tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    match normalized.as_slice() {
        ["you"] => return Ok(Some((PlayerFilter::You, None))),
        ["that", "player"] | ["they"] => {
            return Ok(Some((PlayerFilter::IteratedPlayer, None)));
        }
        ["your", "opponents"] | ["each", "opponent"] | ["opponents"] => {
            return Ok(Some((PlayerFilter::Opponent, None)));
        }
        ["players"] | ["each", "player"] => return Ok(Some((PlayerFilter::Any, None))),
        ["defending", "player"] => return Ok(Some((PlayerFilter::Defending, None))),
        ["attacking", "player"] => return Ok(Some((PlayerFilter::Attacking, None))),
        ["its", "controller"] | ["their", "controller"] => {
            return Ok(Some((
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
                None,
            )));
        }
        ["its", "owner"] | ["their", "owner"] => {
            return Ok(Some((
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
                None,
            )));
        }
        _ => {}
    }

    let player = match parse_subject(subject_tokens) {
        crate::cards::builders::SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit) => {
            PlayerFilter::You
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Opponent) => PlayerFilter::Opponent,
        crate::cards::builders::SubjectAst::Player(PlayerAst::That) => PlayerFilter::IteratedPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Defending) => PlayerFilter::Defending,
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsController) => {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsOwner) => {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Chosen) => PlayerFilter::ChosenPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Attacking) => PlayerFilter::Attacking,
        crate::cards::builders::SubjectAst::Player(PlayerAst::MostLifeTied) => {
            PlayerFilter::MostLifeTied
        }
        _ => return Ok(None),
    };
    Ok(Some((player, None)))
}

fn target_ast_player_filter(player: PlayerFilter, span: Option<TextSpan>) -> PlayerFilter {
    if span.is_some() {
        match player {
            PlayerFilter::Any => PlayerFilter::target_player(),
            PlayerFilter::Opponent => PlayerFilter::target_opponent(),
            other => other,
        }
    } else {
        player
    }
}

fn parse_cast_restriction_tail_filter(words: &[&str]) -> Option<ObjectFilter> {
    if words == ["cast", "spells"] {
        return Some(ObjectFilter::default());
    }
    if words.first() != Some(&"cast") || words.last() != Some(&"spells") || words.len() < 3 {
        return None;
    }
    let tail = &words[1..words.len() - 1];
    let (filter, used) = parse_cast_limit_qualifier(tail)?;
    (used == tail.len()).then_some(filter)
}

fn parse_card_type_list_filter(words: &[&str], zone: Option<Zone>) -> Option<ObjectFilter> {
    let cleaned = words
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "the" | "or" | "and" | ","))
        .filter(|word| !matches!(*word, "unless" | "theyre" | "mana" | "abilities"))
        .collect::<Vec<_>>();
    if cleaned.is_empty() {
        return None;
    }

    let mut filters = Vec::new();
    for word in cleaned {
        let normalized = word.trim_end_matches('s');
        let card_type = parse_card_type(normalized)?;
        let mut filter = ObjectFilter::default();
        filter.zone = zone;
        filter.card_types.push(card_type);
        filters.push(filter);
    }
    if filters.len() == 1 {
        return filters.pop();
    }
    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Some(disjunction)
}

fn restriction_from_cast_limit_filter(
    player: PlayerFilter,
    spell_filter: ObjectFilter,
) -> crate::effect::Restriction {
    crate::effect::Restriction::cast_more_than_one_spell_each_turn_matching(player, spell_filter)
}

pub(crate) fn parse_negated_object_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);

    let (filter, target, ability_scope) =
        if let Some(parsed) = parse_activated_ability_subject(&subject_tokens)? {
            (parsed.filter, parsed.target, Some(parsed.scope))
        } else if starts_with_target_indicator(&subject_tokens) {
            let target = parse_target_phrase(&subject_tokens)?;
            let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported target restriction subject (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))
            })?;
            ensure_it_tagged_constraint(&mut filter);
            (filter, Some(target), None)
        } else if subject_tokens.is_empty() {
            // Supports carried clauses like "... and can't be blocked this turn."
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
            (
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                Some(target),
                None,
            )
        } else {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported subject in negated restriction clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            };
            (filter, None, None)
        };

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let subject_words_storage = normalize_cant_words(&subject_tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if matches!(
        subject_words.as_slice(),
        ["damage"] | ["the", "damage"] | ["that", "damage"]
    ) && remainder_words.as_slice() == ["be", "prevented"]
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::prevent_damage(),
            target: None,
        }));
    }

    let restriction = match remainder_words.as_slice() {
        ["attack"] => Restriction::attack(filter),
        ["attack", "this", "turn"] => Restriction::attack(filter),
        ["attack", "alone"] => Restriction::attack_alone(filter),
        ["attack", "alone", "this", "turn"] => Restriction::attack_alone(filter),
        ["attack", "or", "block"] => Restriction::attack_or_block(filter),
        ["attack", "or", "block", "this", "turn"] => Restriction::attack_or_block(filter),
        ["attack", "or", "block", "alone"] => Restriction::attack_or_block_alone(filter),
        ["attack", "or", "block", "alone", "this", "turn"] => {
            Restriction::attack_or_block_alone(filter)
        }
        ["block"] => Restriction::block(filter),
        ["block", "this", "turn"] => Restriction::block(filter),
        ["block", "alone"] => Restriction::block_alone(filter),
        ["block", "alone", "this", "turn"] => Restriction::block_alone(filter),
        ["be", "blocked"] => Restriction::be_blocked(filter),
        ["be", "blocked", "this", "turn"] => Restriction::be_blocked(filter),
        _ if slice_starts_with(&remainder_words, &["be", "blocked", "by"])
            && remainder_words.len() > 3 =>
        {
            let blocker_tokens = trim_commas(&remainder_tokens[3..]);
            let blocker_filter = parse_subject_object_filter(&blocker_tokens)?
                .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(blocker_filter, filter)
        }
        ["be", "destroyed"] => Restriction::be_destroyed(filter),
        ["be", "regenerated"] => Restriction::be_regenerated(filter),
        ["be", "regenerated", "this", "turn"] => Restriction::be_regenerated(filter),
        ["be", "sacrificed"] => Restriction::be_sacrificed(filter),
        ["be", "countered"] => Restriction::be_countered(filter),
        ["be", "activated"] | ["be", "activated", "this", "turn"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) => {
                Restriction::activate_tap_abilities_of(filter)
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
        },
        ["be", "activated", "unless", "theyre", "mana", "abilities"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_non_mana_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) | None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
        },
        ["transform"] => Restriction::transform(filter),
        ["be", "targeted"] => Restriction::be_targeted(filter),
        _ if remainder_words.first() == Some(&"block") && remainder_words.len() > 1 => {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(filter, attacker_filter)
        }
        _ if is_supported_untap_restriction_tail(&remainder_words) => Restriction::untap(filter),
        _ => {
            if matches!(
                remainder_words.first().copied(),
                Some(
                    "put"
                        | "draw"
                        | "reveal"
                        | "look"
                        | "search"
                        | "create"
                        | "return"
                        | "exile"
                        | "sacrifice"
                        | "discard"
                        | "gain"
                        | "lose"
                )
            ) {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported negated restriction tail (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivatedAbilityScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone)]
struct ParsedActivatedAbilitySubject {
    filter: ObjectFilter,
    target: Option<TargetAst>,
    scope: ActivatedAbilityScope,
}

fn strip_trailing_possessive_token(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    if let Some(last) = normalized.last_mut()
        && let Some(word) = last.as_word().map(str::to_string)
    {
        if let Some(stripped) = str_strip_suffix(&word, "'s")
            .or_else(|| str_strip_suffix(&word, "’s"))
            .or_else(|| str_strip_suffix(&word, "s'"))
            .or_else(|| str_strip_suffix(&word, "s’"))
        {
            last.replace_word(stripped);
        }
    }
    normalized
}

fn parse_activated_ability_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedActivatedAbilitySubject>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let subject_words_storage = normalize_cant_words(tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (owner_word_len, scope) = if slice_ends_with(&subject_words, &["activated", "abilities"]) {
        (
            subject_words.len().saturating_sub(2),
            ActivatedAbilityScope::All,
        )
    } else if slice_ends_with(
        &subject_words,
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ],
    ) {
        (
            subject_words.len().saturating_sub(7),
            ActivatedAbilityScope::TapCostOnly,
        )
    } else {
        return Ok(None);
    };

    if owner_word_len == 0 {
        return Ok(None);
    }
    let owner_end = ActivationRestrictionCompatWords::new(tokens)
        .token_index_after_words(owner_word_len)
        .unwrap_or(tokens.len());
    let owner_tokens = trim_commas(&tokens[..owner_end]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let normalized_owner_tokens = strip_trailing_possessive_token(&owner_tokens);

    let owner_word_view = ActivationRestrictionCompatWords::new(&normalized_owner_tokens);
    let owner_words = owner_word_view.to_word_refs();
    if owner_words.len() == 1 && matches!(owner_words[0], "it" | "its" | "them" | "their") {
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            target: Some(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            )),
            scope,
        }));
    }

    if starts_with_target_indicator(&normalized_owner_tokens) {
        let target = parse_target_phrase(&normalized_owner_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            ))
        })?;
        ensure_it_tagged_constraint(&mut filter);
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter,
            target: Some(target),
            scope,
        }));
    }

    let Some(filter) = parse_subject_object_filter(&normalized_owner_tokens)?
        .or_else(|| parse_object_filter(&normalized_owner_tokens, false).ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedActivatedAbilitySubject {
        filter,
        target: None,
        scope,
    }))
}

fn ensure_it_tagged_constraint(filter: &mut ObjectFilter) {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

fn starts_with_possessive_activated_ability_subject(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["its", "activated", "abilities", ..] | ["their", "activated", "abilities", ..]
    )
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedCantRestriction {
    pub(crate) restriction: crate::effect::Restriction,
    pub(crate) target: Option<TargetAst>,
}

pub(crate) fn starts_with_target_indicator(tokens: &[OwnedLexToken]) -> bool {
    let mut idx = 0usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((_, used)) = parse_number(&tokens[idx..]) {
            idx += used;
        }
    } else if let Some((_, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        idx += used;
    } else if let Some((_, used)) = parse_number(&tokens[idx..])
        && tokens
            .get(idx + used)
            .is_some_and(|token: &OwnedLexToken| token.is_word("target"))
    {
        idx += used;
    } else if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        idx += 1;
    }

    tokens.get(idx).is_some_and(|token| token.is_word("target"))
}

pub(crate) fn find_negation_span(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    for word_idx in 0..word_view.len() {
        let Some(word) = word_view.get(word_idx) else {
            continue;
        };
        if matches!(word, "cant" | "cannot") {
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 1)?;
            return Some((start, end));
        }
        if matches!(word, "doesnt" | "dont") {
            let next_word = word_view.get(word_idx + 1);
            if matches!(next_word, Some("control" | "controls" | "own" | "owns")) {
                continue;
            }
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 1)?;
            return Some((start, end));
        }
        if matches!(word, "does" | "do" | "can") && word_view.get(word_idx + 1) == Some("not") {
            if matches!(word, "does" | "do")
                && matches!(
                    word_view.get(word_idx + 2),
                    Some("control" | "controls" | "own" | "owns")
                )
            {
                continue;
            }
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 2)?;
            return Some((start, end));
        }
    }
    None
}

pub(crate) fn parse_subject_object_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if matches!(
        normalized_words.as_slice(),
        ["damage"] | ["the", "damage"] | ["that", "damage"]
    ) {
        return Ok(Some(ObjectFilter::default()));
    }
    if matches!(
        normalized_words.as_slice(),
        ["it"] | ["they"] | ["them"] | ["itself"] | ["themselves"]
    ) {
        return Ok(Some(ObjectFilter::tagged(TagKey::from(IT_TAG))));
    }

    let words_all = crate::cards::builders::compiler::token_word_refs(tokens);
    if find_window_by(&words_all, 3, |window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    })
    .is_some()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject object filter (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Ok(filter) = parse_object_filter(tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(filter));
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

