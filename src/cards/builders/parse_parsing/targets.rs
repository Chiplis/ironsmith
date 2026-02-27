use super::*;

pub(crate) fn parse_target_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    while tokens.first().is_some_and(|token| token.is_word("then")) {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);
    let mut target_count: Option<ChoiceCount> = None;
    let mut explicit_target = false;

    let all_words = words(tokens);
    if all_words.as_slice() == ["that", "permanent"] || all_words.as_slice() == ["that", "creature"]
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if remaining_words.len() >= 2
        && remaining_words[0] == "chosen"
        && is_demonstrative_object_head(remaining_words[1])
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["equipped", "creature"]
        || remaining_words.as_slice() == ["equipped", "creatures"]
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["enchanted", "creature"]
        || remaining_words.as_slice() == ["enchanted", "creatures"]
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if matches!(
        remaining_words.as_slice(),
        [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spells",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "additional",
            "costs"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spells",
            "additional",
            "costs"
        ]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("tap_cost_0"), span),
            target_count,
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        target_count = Some(ChoiceCount::any_number());
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((count, used)) = parse_number(&tokens[idx..]) {
            target_count = Some(ChoiceCount::up_to(count as usize));
            idx += used;
        } else {
            let next_word = tokens.get(idx).and_then(Token::as_word).unwrap_or("?");
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic or missing target count after 'up to' (found '{next_word}' in clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    } else if let Some((count, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    } else if let Some((count, used)) = parse_number(&tokens[idx..]) {
        let next_is_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"));
        let next_is_other_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("other"))
            && tokens
                .get(idx + used + 1)
                .is_some_and(|token| token.is_word("target"));
        let mut object_selector_idx = idx + used;
        while tokens
            .get(object_selector_idx)
            .and_then(Token::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "tapped"
                        | "untapped"
                        | "attacking"
                        | "nonattacking"
                        | "blocked"
                        | "unblocked"
                        | "blocking"
                        | "nonblocking"
                        | "non"
                        | "other"
                        | "another"
                        | "nonartifact"
                        | "noncreature"
                        | "nonland"
                        | "nontoken"
                        | "legendary"
                        | "basic"
                )
            })
        {
            object_selector_idx += 1;
        }
        let next_is_object_selector = tokens
            .get(object_selector_idx)
            .and_then(Token::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "card"
                        | "cards"
                        | "permanent"
                        | "permanents"
                        | "creature"
                        | "creatures"
                        | "spell"
                        | "spells"
                        | "source"
                        | "sources"
                        | "token"
                        | "tokens"
                ) || parse_card_type(word).is_some()
                    || parse_non_type(word).is_some()
                    || parse_subtype_word(word).is_some()
                    || word
                        .strip_suffix('s')
                        .and_then(parse_subtype_word)
                        .is_some()
            });
        if next_is_target || next_is_other_target {
            target_count = Some(ChoiceCount::exactly(count as usize));
            idx += used;
        } else if next_is_object_selector {
            target_count = Some(ChoiceCount::exactly(count as usize));
            idx += used;
        }
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("x")) {
        let next_is_target = tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"));
        let mut object_selector_idx = idx + 1;
        while tokens
            .get(object_selector_idx)
            .and_then(Token::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "tapped"
                        | "untapped"
                        | "attacking"
                        | "nonattacking"
                        | "blocked"
                        | "unblocked"
                        | "blocking"
                        | "nonblocking"
                        | "non"
                        | "other"
                        | "another"
                        | "nonartifact"
                        | "noncreature"
                        | "nonland"
                        | "nontoken"
                        | "legendary"
                        | "basic"
                )
            })
        {
            object_selector_idx += 1;
        }
        let next_is_object_selector = tokens
            .get(object_selector_idx)
            .and_then(Token::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "card"
                        | "cards"
                        | "permanent"
                        | "permanents"
                        | "creature"
                        | "creatures"
                        | "spell"
                        | "spells"
                        | "source"
                        | "sources"
                        | "token"
                        | "tokens"
                ) || parse_card_type(word).is_some()
                    || parse_non_type(word).is_some()
                    || parse_subtype_word(word).is_some()
                    || word
                        .strip_suffix('s')
                        .and_then(parse_subtype_word)
                        .is_some()
            });
        if next_is_target || next_is_object_selector {
            // Preserve explicit/implicit X-count wording while reusing open-ended
            // target selection internals.
            target_count = Some(ChoiceCount::dynamic_x());
            idx += 1;
        }
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    while tokens
        .get(idx)
        .and_then(Token::as_word)
        .is_some_and(is_article)
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("other"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        other = true;
        explicit_target = true;
        idx += 2;
    } else {
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            idx += 1;
        }

        if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
            explicit_target = true;
            idx += 1;
        }
    }

    if let Some(ordinal_word) = tokens.get(idx).and_then(Token::as_word)
        && matches!(
            ordinal_word,
            "first"
                | "second"
                | "third"
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
        )
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        if ordinal_word != "first" {
            other = true;
        }
        explicit_target = true;
        idx += 2;
    }

    let words_all = words(&tokens[idx..]);
    if words_all.as_slice() == ["any", "target"] {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = words(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.as_slice() == ["player", "on", "your", "team"]
        || remaining_words.as_slice() == ["players", "on", "your", "team"]
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "player"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "opponent"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words.get(1).is_some_and(|word| {
        matches!(
            *word,
            "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "spell"
                | "spells"
                | "source"
                | "sources"
                | "card"
                | "cards"
        ) || parse_card_type(word).is_some()
            || word
                .strip_suffix('s')
                .is_some_and(|singular| parse_card_type(singular).is_some())
    });
    if remaining_words.len() >= 3
        && remaining_words[0] == "that"
        && second_word_is_object_head
        && matches!(
            remaining_words[2],
            "controller" | "controllers" | "owner" | "owners"
        )
    {
        let player = if remaining_words[2].starts_with("owner") {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        } else {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        };
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "controller"])
        || remaining_words.starts_with(&["its", "controllers"])
        || remaining_words.starts_with(&["their", "controller"])
        || remaining_words.starts_with(&["their", "controllers"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "owner"])
        || remaining_words.starts_with(&["its", "owners"])
        || remaining_words.starts_with(&["their", "owner"])
        || remaining_words.starts_with(&["their", "owners"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["you"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["opponent"] || remaining_words.as_slice() == ["opponents"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["spell"] || remaining_words.as_slice() == ["spells"] {
        return Ok(wrap_target_count(
            TargetAst::Spell(target_span),
            target_count,
        ));
    }

    if is_source_reference_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Source(target_span),
            target_count,
        ));
    }
    if is_source_from_your_graveyard_words(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Graveyard);
        source_filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["thiss", "power", "and", "toughness"]) {
        return Ok(wrap_target_count(
            TargetAst::Source(target_span),
            target_count,
        ));
    }

    if remaining_words.first().is_some_and(|word| *word == "it")
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| *word == "instead" || *word == "this" || *word == "way")
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["itself"] {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches!(
        remaining_words.as_slice(),
        ["them"] | ["him"] | ["her"] | ["that", "player"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let attacking_you_or_your_planeswalker = matches!(
        remaining_words.as_slice(),
        [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ]
    );
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

    let opponent_or_planeswalker = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["opponent", "or", "planeswalker"]
                | ["opponents", "or", "planeswalkers"]
                | ["planeswalker", "or", "opponent"]
                | ["planeswalkers", "or", "opponents"]
        )
    });
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let player_or_planeswalker_its_attacking = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["player", "or", "planeswalker"]
                | ["players", "or", "planeswalkers"]
                | ["planeswalker", "or", "player"]
                | ["planeswalkers", "or", "players"]
        )
    }) && remaining_words.contains(&"attacking")
        && (remaining_words.contains(&"its")
            || remaining_words.contains(&"it")
            || remaining_words.contains(&"thats")
            || remaining_words.contains(&"that"));
    if player_or_planeswalker_its_attacking {
        return Ok(wrap_target_count(
            TargetAst::AttackedPlayerOrPlaneswalker(target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["player", "or", "planeswalker"]
                | ["players", "or", "planeswalkers"]
                | ["planeswalker", "or", "player"]
                | ["planeswalkers", "or", "players"]
        )
    });
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        remaining_words.as_slice(),
        ["permanent", "or", "player"]
            | ["permanents", "or", "players"]
            | ["player", "or", "permanent"]
            | ["players", "or", "permanents"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let creature_or_player = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["creature", "or", "player"]
                | ["creatures", "or", "players"]
                | ["player", "or", "creature"]
                | ["players", "or", "creatures"]
                | ["creature", "and", "player"]
                | ["creatures", "and", "players"]
                | ["player", "and", "creature"]
                | ["players", "and", "creatures"]
                | ["creature", "and/or", "player"]
                | ["creatures", "and/or", "players"]
                | ["player", "and/or", "creature"]
                | ["players", "and/or", "creatures"]
        )
    }) || remaining_words.windows(4).any(|window| {
        matches!(
            window,
            ["creature", "and", "or", "player"]
                | ["creatures", "and", "or", "players"]
                | ["player", "and", "or", "creature"]
                | ["players", "and", "or", "creatures"]
        )
    });
    if creature_or_player {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let filter = parse_object_filter(remaining, other)?;
    let it_span = if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        tokens
            .iter()
            .rev()
            .find(|token| token.is_word("it"))
            .map(Token::span)
    } else {
        None
    };
    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, it_span),
        target_count,
    ))
}

pub(crate) fn parse_target_count_range_prefix(tokens: &[Token]) -> Option<(ChoiceCount, usize)> {
    let (first, first_used) = parse_number(tokens)?;
    let or_idx = first_used;
    if !tokens.get(or_idx).is_some_and(|token| token.is_word("or")) {
        return None;
    }
    let (second, second_used) = parse_number(&tokens[or_idx + 1..])?;
    if second < first {
        return None;
    }
    Some((
        ChoiceCount {
            min: first as usize,
            max: Some(second as usize),
            dynamic_x: false,
        },
        first_used + 1 + second_used,
    ))
}

pub(crate) fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

pub(crate) fn is_source_from_your_graveyard_words(words: &[&str]) -> bool {
    if words.len() < 4 {
        return false;
    }

    let starts_with_this = words[0] == "this" || words[0] == "thiss";
    let references_source_noun =
        words.contains(&"card") || words.contains(&"creature") || words.contains(&"permanent");

    starts_with_this
        && references_source_noun
        && words.contains(&"from")
        && words.contains(&"your")
        && words.contains(&"graveyard")
}

pub(crate) fn is_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if words[0] != "this" && words[0] != "thiss" {
        return false;
    }

    if words.len() == 1 {
        return true;
    }

    if words.len() != 2 {
        return false;
    }

    match words[1] {
        "source" | "spell" | "permanent" | "card" | "creature" => true,
        other => parse_card_type(other).is_some() || parse_subtype_word(other).is_some(),
    }
}

pub(crate) fn contains_source_from_your_graveyard_phrase(words: &[&str]) -> bool {
    words.windows(5).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && matches!(window[1], "card" | "creature" | "permanent")
            && window[2] == "from"
            && window[3] == "your"
            && window[4] == "graveyard"
    })
}

pub(crate) fn contains_source_from_your_hand_phrase(words: &[&str]) -> bool {
    // Match "this card/creature/permanent from your hand" (5 words)
    words.windows(5).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && matches!(window[1], "card" | "creature" | "permanent")
            && window[2] == "from"
            && window[3] == "your"
            && window[4] == "hand"
    })
    // Match "this from your hand" (4 words) — when card name was normalized to bare "this"
    || words.windows(4).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && window[1] == "from"
            && window[2] == "your"
            && window[3] == "hand"
    })
}

pub(crate) fn contains_discard_source_phrase(words: &[&str]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["discard", "this", "card"])
}

pub(crate) fn is_demonstrative_object_head(word: &str) -> bool {
    if matches!(
        word,
        "creature"
            | "creatures"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "spell"
            | "spells"
            | "source"
            | "sources"
            | "token"
            | "tokens"
    ) {
        return true;
    }
    if parse_card_type(word).is_some() {
        return true;
    }
    if let Some(singular) = word.strip_suffix('s') {
        return parse_card_type(singular).is_some();
    }
    false
}

