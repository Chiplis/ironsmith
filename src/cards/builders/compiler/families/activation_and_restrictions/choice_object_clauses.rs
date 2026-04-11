use super::*;

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let (chooser, choose_start_idx) =
        if clause_words.first().copied() == Some("target") && clause_words.len() >= 4 {
            let chooser = match clause_words.get(1).copied() {
                Some("player") => PlayerAst::Target,
                Some("opponent") | Some("opponents") => PlayerAst::TargetOpponent,
                _ => return Ok(None),
            };
            if !matches!(
                clause_words.get(2).copied(),
                Some("choose") | Some("chooses")
            ) {
                return Ok(None);
            }
            (chooser, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("that")
            && matches!(clause_words.get(1).copied(), Some("player" | "players"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("the")
            && matches!(clause_words.get(1).copied(), Some("voter"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else {
            return Ok(None);
        };

    let mut choose_object_tokens = trim_commas(&tokens[choose_start_idx..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut count = ChoiceCount::exactly(1);
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("up"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((value, used)) = parse_number(&choose_object_tokens[2..])
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        };
        choose_object_tokens = trim_commas(&choose_object_tokens[2 + used..]);
    } else if let Some((value, used)) = parse_number(&choose_object_tokens) {
        count = ChoiceCount::exactly(value as usize);
        choose_object_tokens = trim_commas(&choose_object_tokens[used..]);
    }
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter after count in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("target"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("player") || token.is_word("opponent"))
    {
        return Ok(None);
    }
    if find_verb(&choose_object_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if clause_words.first().copied() == Some("you") {
        1usize
    } else {
        0usize
    };
    if !matches!(
        clause_words.get(choose_word_idx).copied(),
        Some("choose" | "chooses")
    ) {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let mut choose_object_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]).to_vec();
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut references_it = false;
    loop {
        let len = choose_object_tokens.len();
        let trailing_it = len >= 2
            && choose_object_tokens[len - 2]
                .as_word()
                .is_some_and(|word| matches!(word, "from" | "in"))
            && choose_object_tokens[len - 1]
                .as_word()
                .is_some_and(|word| matches!(word, "it" | "them"));
        let trailing_there = len >= 3
            && choose_object_tokens[len - 3]
                .as_word()
                .is_some_and(|word| matches!(word, "from" | "in"))
            && choose_object_tokens[len - 2].is_word("there")
            && choose_object_tokens[len - 1].is_word("in");
        if trailing_it {
            references_it = true;
            choose_object_tokens.truncate(len - 2);
            continue;
        }
        if trailing_there {
            references_it = true;
            choose_object_tokens.truncate(len - 3);
            continue;
        }
        break;
    }
    let mut choose_words = crate::cards::builders::compiler::token_word_refs(&choose_object_tokens);
    loop {
        if matches!(
            choose_words.as_slice(),
            [.., "from", "it"] | [.., "from", "them"] | [.., "in", "it"] | [.., "in", "them"]
        ) {
            references_it = true;
            choose_words.truncate(choose_words.len().saturating_sub(2));
            continue;
        }
        if matches!(choose_words.as_slice(), [.., "from", "there", "in"]) {
            references_it = true;
            choose_words.truncate(choose_words.len().saturating_sub(3));
            continue;
        }
        break;
    }
    let mut count = ChoiceCount::exactly(1);
    if slice_starts_with(&choose_words, &["up", "to"])
        && let Some((value, used)) = parse_number(
            &choose_words[2..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>(),
        )
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        };
        choose_words = choose_words[2 + used..].to_vec();
    } else if let Some((value, used)) = parse_number(
        &choose_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>(),
    ) {
        count = ChoiceCount::exactly(value as usize);
        choose_words = choose_words[used..].to_vec();
    } else if choose_words.first().is_some_and(|word| is_article(word)) {
        choose_words = choose_words[1..].to_vec();
    }

    if choose_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter in choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let choose_filter_tokens = choose_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if find_verb(&choose_filter_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter =
        if references_it && matches!(choose_words.as_slice(), ["card"] | ["cards"]) {
            ObjectFilter::default()
        } else {
            parse_object_filter(&choose_filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported chosen object filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
    if references_it {
        if choose_filter.zone.is_none() {
            choose_filter.zone = Some(Zone::Hand);
        }
        if !choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if references_it {
        choose_filter.controller = None;
        choose_filter.owner = None;
    } else if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((PlayerAst::You, choose_filter, count)))
}

pub(crate) fn parse_you_choose_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, PlayerFilter, bool, usize)>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if clause_words.first().copied() == Some("you") {
        1usize
    } else {
        0usize
    };
    if !matches!(
        clause_words.get(choose_word_idx).copied(),
        Some("choose" | "chooses")
    ) {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose-player clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let player_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]);
    let mut player_words = crate::cards::builders::compiler::token_word_refs(&player_tokens);
    if player_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen player in choose-player clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut exclude_previous_choices = 0usize;
    while let Some(word) = player_words.first().copied() {
        match word {
            "a" | "an" => {
                player_words = player_words[1..].to_vec();
            }
            "another" => {
                exclude_previous_choices = exclude_previous_choices.max(1);
                player_words = player_words[1..].to_vec();
            }
            "second" => {
                exclude_previous_choices = exclude_previous_choices.max(1);
                player_words = player_words[1..].to_vec();
            }
            "third" => {
                exclude_previous_choices = exclude_previous_choices.max(2);
                player_words = player_words[1..].to_vec();
            }
            _ => break,
        }
    }

    let mut filter = match player_words.first().copied() {
        Some("player") => {
            player_words = player_words[1..].to_vec();
            None
        }
        Some("opponent" | "opponents") => {
            player_words = player_words[1..].to_vec();
            Some(PlayerFilter::Opponent)
        }
        _ => return Ok(None),
    };

    let mut random = false;
    if slice_starts_with(&player_words, &["at", "random"]) {
        random = true;
        player_words = player_words[2..].to_vec();
    }

    let filter = if let Some(filter) = filter.take() {
        if player_words.is_empty() {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        match player_words.as_slice() {
            [] => PlayerFilter::Any,
            [
                "with",
                "the",
                "most",
                "life",
                "or",
                "tied",
                "for",
                "most",
                "life",
            ] => PlayerFilter::MostLifeTied,
            [
                "who",
                "cast",
                "one",
                "or",
                "more",
                "sorcery",
                "spells",
                "this",
                "turn",
            ] => PlayerFilter::CastCardTypeThisTurn(CardType::Sorcery),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen player filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    Ok(Some((
        PlayerAst::You,
        filter,
        random,
        exclude_previous_choices,
    )))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = crate::cards::builders::compiler::token_word_refs(second);
    let Some((neg_start, neg_end)) = find_negation_span(second) else {
        return Ok(None);
    };
    let tail_words_storage = normalize_cant_words(&second[neg_end..]);
    let tail_words = tail_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !matches!(tail_words.as_slice(), ["block", "this", "turn"] | ["block"]) {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&second[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut exclude_tagged_choice = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        exclude_tagged_choice = true;
        subject_tokens = trim_commas(&subject_tokens[1..]);
    }
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut restriction_filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-block subject filter (clause: '{}')",
            second_words.join(" ")
        ))
    })?;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::block(restriction_filter),
            duration: Until::EndOfTurn,
            condition: None,
        },
    ]))
}

#[cfg(test)]
mod tests {
    use super::super::super::util::tokenize_line;
    use super::*;
    use crate::effect::Restriction;
    use crate::zone::Zone;

    #[test]
    fn parse_negated_object_restriction_clause_supports_attack_or_block_alone() {
        let tokens = tokenize_line("This creature can't attack or block alone.", 0);

        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("parse attack-or-block-alone restriction")
            .expect("expected restriction");

        assert!(matches!(
            parsed.restriction,
            Restriction::AttackOrBlockAlone(_)
        ));
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_bare_card_from_it() {
        let tokens = tokenize_line("You choose a card from it.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-a-card-from-it clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
        );
        assert!(
            filter.controller.is_none(),
            "expected no controller pin, got {filter:?}"
        );
        assert!(
            filter.owner.is_none(),
            "expected no owner pin, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_player_clause_supports_choose_an_opponent() {
        let tokens = tokenize_line("Choose an opponent.", 0);

        let (chooser, filter, random, exclude_previous_choices) =
            parse_you_choose_player_clause(&tokens)
                .expect("parse choose-an-opponent clause")
                .expect("expected choose-player clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(filter, PlayerFilter::Opponent);
        assert!(!random);
        assert_eq!(exclude_previous_choices, 0);
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_limited_type_lists() {
        let parsed =
            parse_choose_card_type_phrase_words(&["choose", "artifact", "creature", "or", "land"])
                .expect("limited choose-card-type phrase should parse")
                .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                5,
                vec![CardType::Artifact, CardType::Creature, CardType::Land]
            )
        );
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_permanent_types() {
        let parsed = parse_choose_card_type_phrase_words(&["choose", "a", "permanent", "type"])
            .expect("permanent-type choice phrase should parse")
            .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                4,
                vec![
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                    CardType::Land,
                    CardType::Planeswalker,
                    CardType::Battle,
                ]
            )
        );
    }

    #[test]
    fn parse_cant_restriction_clause_supports_that_player_cant_cast_spells() {
        let tokens = tokenize_line("That player can't cast spells.", 0);

        let parsed = parse_cant_restriction_clause(&tokens)
            .expect("parse that-player cant-cast clause")
            .expect("expected cant restriction");

        assert_eq!(
            parsed.restriction,
            Restriction::cast_spells(PlayerFilter::IteratedPlayer)
        );
    }
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::cards::builders::compiler::token_word_refs(first);
    let Some(mut idx) = find_index(&first_words, |word| matches!(*word, "choose" | "chooses"))
    else {
        return Ok(None);
    };
    idx += 1;
    if first_words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    if first_words.get(idx) != Some(&"card") || first_words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let reveal_words = &first_words[idx..];
    if !slice_starts_with(&reveal_words, &["then", "reveal", "the", "top"]) {
        return Ok(None);
    }
    let reveal_tokens = reveal_words[4..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&reveal_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing reveal count in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        ))
    })?;
    if reveal_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        )));
    }
    let reveal_tail = crate::cards::builders::compiler::token_word_refs(&reveal_tokens[used + 1..]);
    if !slice_ends_with(&reveal_tail, &["of", "your", "library"]) {
        return Ok(None);
    }

    let second_words = crate::cards::builders::compiler::token_word_refs(second);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let has_chosen_type = contains_word_sequence(&second_words, &["chosen", "type"]);
    let has_revealed_this_way = contains_word_sequence(&second_words, &["revealed", "this", "way"]);
    let has_into_your_hand = contains_word_sequence(&second_words, &["into", "your", "hand"]);
    let has_bottom_of_library =
        contains_word_sequence(&second_words, &["bottom", "of", "your", "library"]);
    if !has_chosen_type || !has_revealed_this_way || !has_into_your_hand || !has_bottom_of_library {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom {
            player: PlayerAst::You,
            count,
        },
    ]))
}

pub(crate) fn parse_choose_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<Subtype>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) != Some(&"creature") || words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let mut excluded_subtypes = Vec::new();
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let subtype_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| str_strip_suffix(subtype_word, "s").and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        excluded_subtypes.push(subtype);
        idx += 3;
    }

    Ok(Some((idx, excluded_subtypes)))
}

pub(crate) fn parse_choose_phrase_prefix_words(words: &[&str]) -> Option<usize> {
    if words.is_empty() || !matches!(words[0], "choose" | "chooses") {
        return None;
    }

    let mut idx = 1usize;
    if words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    Some(idx)
}

pub(crate) fn parse_choose_color_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Option<ColorSet>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) != Some(&"color") {
        return Ok(None);
    }
    idx += 1;

    let mut excluded = None;
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let color_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        excluded = Some(parse_color(color_word).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?);
        idx += 3;
    }

    Ok(Some((idx, excluded)))
}

pub(crate) fn parse_choose_card_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<CardType>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) == Some(&"card") && words.get(idx + 1) == Some(&"type") {
        return Ok(Some((idx + 2, Vec::new())));
    }
    if words.get(idx) == Some(&"permanent")
        && matches!(words.get(idx + 1).copied(), Some("type" | "types"))
    {
        return Ok(Some((
            idx + 2,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
        )));
    }

    let mut options = Vec::new();
    let mut consumed_any = false;
    while let Some(word) = words.get(idx).copied() {
        if matches!(word, "or" | "and") {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type(word) else {
            break;
        };
        if !options.contains(&card_type) {
            options.push(card_type);
        }
        consumed_any = true;
        idx += 1;
    }

    if !consumed_any {
        return Ok(None);
    }

    Ok(Some((idx, options)))
}

pub(crate) fn parse_choose_player_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"player") {
        return None;
    }
    idx += 1;
    Some(idx)
}

pub(crate) fn parse_choose_basic_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"basic")
        || words.get(idx + 1) != Some(&"land")
        || words.get(idx + 2) != Some(&"type")
    {
        return None;
    }
    idx += 3;
    Some(idx)
}

pub(crate) fn parse_choose_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"land") || words.get(idx + 1) != Some(&"type") {
        return None;
    }
    idx += 2;
    Some(idx)
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::cards::builders::compiler::token_word_refs(&first_tokens);
    let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&first_words)?
    else {
        return Ok(None);
    };
    if consumed != first_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-type choice clause (clause: '{}')",
            first_words.join(" ")
        )));
    }

    let second_words = crate::cards::builders::compiler::token_word_refs(second);
    let Some(become_idx) = find_index(second, |token| {
        token.is_word("become") || token.is_word("becomes")
    }) else {
        return Ok(None);
    };
    if become_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&second[..become_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in creature-type become clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let become_tail_tokens = trim_commas(&second[become_idx + 1..]);
    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&become_tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, become_tail_tokens.to_vec())
        };
    let become_words = crate::cards::builders::compiler::token_word_refs(&become_tokens);
    if become_words.as_slice() != ["that", "type"] {
        return Ok(None);
    }

    let subject_words = crate::cards::builders::compiler::token_word_refs(&subject_tokens);
    let target = if slice_starts_with(&subject_words, &["each"])
        || slice_starts_with(&subject_words, &["all"])
    {
        let filter_tokens = trim_commas(&subject_tokens[1..]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            ))
        })?;
        TargetAst::Object(filter, span_from_tokens(&subject_tokens), None)
    } else {
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(vec![EffectAst::BecomeCreatureTypeChoice {
        target,
        duration,
        excluded_subtypes,
    }]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(and_idx) = find_index(tokens, |token| token.is_word("and")) else {
        return Ok(None);
    };
    let first_clause = trim_commas(&tokens[..and_idx]);
    let second_clause = trim_commas(&tokens[and_idx + 1..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::cards::builders::compiler::token_word_refs(&second_clause);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(on_idx) = find_index(&second_clause, |token: &OwnedLexToken| token.is_word("on"))
    else {
        return Ok(None);
    };
    if !second_clause
        .get(on_idx + 1)
        .is_some_and(|token| token.is_word("top"))
        || !second_clause
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }
    let destination_words =
        crate::cards::builders::compiler::token_word_refs(&second_clause[on_idx + 3..]);
    if !slice_contains(&destination_words, &"library") {
        return Ok(None);
    }

    let moved_tokens = trim_commas(&second_clause[1..on_idx]);
    let moved_words = crate::cards::builders::compiler::token_word_refs(&moved_tokens);
    let target = if moved_tokens.is_empty()
        || moved_words.as_slice() == ["it"]
        || moved_words.as_slice() == ["them"]
        || moved_words.as_slice() == ["those"]
        || moved_words.as_slice() == ["those", "cards"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause))
    } else {
        parse_target_phrase(&moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
    ]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = find_window_by(tokens, 2, |window| {
        window[0].is_comma() && window[1].is_word("then")
    })
    .map(|idx| (idx, idx + 2))
    .or_else(|| {
        find_index(tokens, |token| token.is_word("then"))
            .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
    });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let first_clause = trim_commas(&tokens[..head_end]);
    let second_clause = trim_commas(&tokens[tail_start..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::cards::builders::compiler::token_word_refs(&second_clause);
    if second_words.len() < 4
        || second_words[0] != "you"
        || !matches!(second_words[1], "put" | "puts")
    {
        return Ok(None);
    }

    let Some(onto_idx) = find_index(&second_clause, |token: &OwnedLexToken| {
        token.is_word("onto")
    }) else {
        return Ok(None);
    };
    if onto_idx < 2 {
        return Ok(None);
    }

    let moved_words =
        crate::cards::builders::compiler::token_word_refs(&second_clause[2..onto_idx]);
    let moved_is_tagged_choice = moved_words == ["it"]
        || moved_words == ["that", "card"]
        || moved_words == ["that", "permanent"];
    if !moved_is_tagged_choice {
        return Ok(None);
    }

    let destination_words: Vec<&str> =
        crate::cards::builders::compiler::token_word_refs(&second_clause[onto_idx + 1..])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    if destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }
    let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
    let battlefield_tapped = slice_contains(&destination_tail, &"tapped");
    destination_tail.retain(|word| *word != "tapped");
    let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail.is_empty() {
        ReturnControllerAst::Preserve
    } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
        || destination_tail.as_slice() == ["under", "their", "owners", "control"]
        || destination_tail.as_slice() == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause)),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        },
    ]))
}
