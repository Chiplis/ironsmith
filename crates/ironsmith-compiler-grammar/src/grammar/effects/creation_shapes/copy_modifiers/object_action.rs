use crate::word_primitives::WordWindows as _;
use super::*;

/// Splits "its name is <name>" or "named <name>" (ending at "and" or the end
/// of the exception) out of a copy exception.
fn split_copy_name_exception<'a>(words: &[&'a str]) -> (Option<Vec<String>>, Vec<&'a str>) {
    let marker = words
        .word_windows(3)
        .position(|window| window == ["its", "name", "is"])
        .map(|idx| (idx, idx + 3))
        .or_else(|| {
            words
                .iter()
                .position(|word| *word == "named")
                .map(|idx| (idx, idx + 1))
        });
    let Some((marker_start, name_start)) = marker else {
        return (None, words.to_vec());
    };
    let name_end = words[name_start..]
        .iter()
        .position(|word| *word == "and")
        .map_or(words.len(), |offset| name_start + offset);
    if name_end == name_start {
        return (None, words.to_vec());
    }
    let name = words[name_start..name_end]
        .iter()
        .map(|word| (*word).to_string())
        .collect();
    let mut kept = words[..marker_start].to_vec();
    let mut rest = &words[name_end..];
    if kept.is_empty() && rest.first() == Some(&"and") {
        rest = &rest[1..];
    }
    kept.extend_from_slice(rest);
    (Some(name), kept)
}

/// Quoted custom identities are bounded operands; names may contain spaces,
/// hyphens and possessives independently of following copy clauses.
pub fn parse_copy_modifier_tokens(tokens: &[crate::lexer::OwnedLexToken]) -> Result<CopyModifierSpec,CardTextError> {
    use crate::lexer::TokenKind;
    let mut kept=tokens.to_vec();let mut removed=Vec::new();let mut index=0;
    while index+4<kept.len() {
        if !kept[index].is_any_word(&["doesn't","doesnt"]) || !kept[index+1].is_word("have")
            || !kept[index+2].is_word("custom") || !kept[index+3].is_word("keyword")
            || kept[index+4].kind!=TokenKind::Quote {index+=1;continue;}
        let start_name=index+5;
        let close=(start_name..kept.len()).find(|at|kept[*at].kind==TokenKind::Quote)
            .ok_or_else(||CardTextError::ParseError("custom keyword copy exception requires a complete quoted identity".into()))?;
        let words=&kept[start_name..close];
        if words.is_empty() || words.iter().any(|token|token.kind!=TokenKind::Word) {
            return Err(CardTextError::ParseError("custom keyword copy exception requires one opaque name".into()));
        }
        let name=crate::lexer::render_token_slice(words).trim().to_ascii_lowercase();
        if name.len()>64 || !name.chars().all(|character|character.is_ascii_alphabetic()||matches!(character,' '|'-'|'\'')) {
            return Err(CardTextError::ParseError("invalid custom keyword designation in copy exception".into()));
        }
        let start=if index>1 && kept[index-2].is_word("each") && kept[index-1].is_any_word(&["token","copy"]) {index-2}
            else if index>0 && kept[index-1].is_any_word(&["it","they"]) {index-1} else {index};
        crate::word_claims::claim_tokens(&kept[start..=close]);
        if !removed.contains(&name){removed.push(name);}
        kept.drain(start..=close);index=start;
    }
    let mut parsed=parse_copy_modifier_words(&crate::lexer::token_word_refs(&kept))?;
    for name in removed {if !parsed.removed_ability_designations.contains(&name){parsed.removed_ability_designations.push(name);}}
    Ok(parsed)
}

pub fn parse_copy_modifier_words(tail_words: &[&str]) -> Result<CopyModifierSpec, CardTextError> {
    let exception = last_class_location(tail_words, CreationWordClass::Except);
    if exception.is_some_and(|index| index + 1 == tail_words.len()) {
        return Err(CardTextError::ParseError("copy exception requires a modifier".into()));
    }
    let raw_modifier_words = exception
        .and_then(|idx| {
            // The characteristic/grant reader owns the exception delimiter
            // even when a later quoted ability consumes the remaining words.
            crate::word_claims::claim_word(tail_words[idx]);
            tail_words.get(idx + 1..)
        })
        .unwrap_or_default();
    let mut spec = CopyModifierSpec::default();
    if raw_modifier_words.is_empty() {
        return Ok(spec);
    }
    // CR 707.9b: the name exception is removed before the characteristic
    // words are read, so name words never become subtypes or colors.
    let (name_words, kept_words) = split_copy_name_exception(raw_modifier_words);
    spec.name_words = name_words;
    let mut kept_words = kept_words;
    if let Some(start) = kept_words.windows(8).position(|words| matches!(words,
        ["it" | "they", "loses" | "lose", "all", "abilities", "until", "end", "of", "turn"])) {
        crate::word_claims::claim_words(&kept_words[start..start+8]);
        spec.ability_loss_until=Some(crate::effect::Until::EndOfTurn);
        kept_words.drain(start..start+8);
    }
    // This leaf names an explicitly designated keyword group. It never
    // guesses whether an unrecognized ordinary keyword has that identity.
    while let Some(index) = kept_words.word_windows(5).position(|words| matches!(words,
        ["doesn't" | "doesnt", "have", "custom", "keyword", _])) {
        let name = kept_words[index + 4];
        if !name.chars().all(|character| character.is_ascii_alphabetic() || character == '-') {
            return Err(CardTextError::ParseError("custom keyword identity must be a plain name".into()));
        }
        if !spec.removed_ability_designations.iter().any(|prior| prior == name) {
            spec.removed_ability_designations.push(name.to_string());
        }
        let start = if index > 0 && matches!(kept_words[index - 1], "it" | "they") { index - 1 } else { index };
        crate::word_claims::claim_words(&kept_words[start..index + 5]);
        kept_words.drain(start..index + 5);
    }
    if let Some(index) = kept_words.word_windows(5).position(|words| matches!(words, ["it" | "they", "has" | "have", "no", "mana", "cost"])) {
        spec.without_mana_cost = true;
        kept_words.drain(index..index + 5);
    }
    // This bounded suffix specifies the replacement subtype family; it is
    // independent of the identity descriptor before it.
    if let Some(index) = kept_words.word_windows(6).position(|words| words == ["instead", "of", "its", "other", "creature", "types"]) {
        let end=index+6;
        crate::word_claims::claim_words(&kept_words[index..end]);
        kept_words.drain(index..end);
        spec.preserve_noncreature_subtypes = true;
    } else if let Some(index)=kept_words.word_windows(5).position(|words|words==["instead","of","its","other","types"]) {
        crate::word_claims::claim_words(&kept_words[index..index+5]);
        kept_words.drain(index..index+5);
    }
    let modifier_words: &[&str] = &kept_words;
    let surface = CreationWords::new(modifier_words);
    if modifier_words.iter().any(|word| *word == "legendary")
        && !surface.has_phrase(CreationPhrase::NotLegendary)
    {
        push_unique(&mut spec.added_supertypes, Supertype::Legendary);
    }

    if surface.has(CreationWordClass::LoseVerb) && surface.has(CreationWordClass::Soulbond) {
        // "loses soulbond" (Mirage Phalanx): the copy is created without the
        // soulbond pairing ability. Only the adjacent "lose(s) soulbond" pair
        // has that meaning; anything else stays unsupported.
        if crate::slice_primitives::find_window_by(modifier_words, 2, |pair| {
            matches!(pair[0], "lose" | "loses") && pair[1] == "soulbond"
        })
        .is_some()
        {
            spec.loses_soulbond = true;
        } else {
            return Err(CardTextError::ParseError(
                "removing soulbond requires non-marker semantics".to_string(),
            ));
        }
    }
    if let Some(location) = surface.phrase_location(CreationPhrase::NotLegendary) {
        spec.removed_supertypes.push(Supertype::Legendary);
        // "except it isn't legendary": the exception's subject and the
        // negated supertype are the removed-supertype modifier.
        if let Some(phrase) = CreationPhrase::NotLegendary
            .alternatives()
            .iter()
            .copied()
            .filter(|phrase| modifier_words[location..].starts_with(phrase))
            .max_by_key(|phrase| phrase.len())
        {
            let mut start = if location > 0
                && matches!(
                    modifier_words[location - 1],
                    "it" | "they" | "tokens" | "token" | "copy" | "copies"
                )
            {
                location - 1
            } else {
                location
            };
            // "except the token isn't legendary": the article of the subject.
            if start + 1 == location
                && start > 0
                && matches!(modifier_words[start], "token" | "tokens" | "copy" | "copies")
                && matches!(modifier_words[start - 1], "the" | "that" | "those" | "this" | "these")
            {
                start -= 1;
            }
            crate::word_claims::claim_words(&modifier_words[start..location + phrase.len()]);
        }
    }
    spec.set_base_power_toughness = modifier_words
        .iter()
        .filter_map(|word| parse_unsigned_pt_word(word))
        .next();
    let power_is_source_total = contains_words(
        modifier_words,
        &[
            "its",
            "power",
            "is",
            "equal",
            "to",
            "the",
            "total",
            "power",
            "of",
            "those",
            "creatures",
        ],
    );
    let toughness_is_source_total = contains_words(
        modifier_words,
        &[
            "its",
            "toughness",
            "is",
            "equal",
            "to",
            "the",
            "total",
            "toughness",
            "of",
            "those",
            "creatures",
        ],
    );
    if power_is_source_total != toughness_is_source_total {
        return Err(CardTextError::ParseError(
            "copy aggregate power/toughness exception must preserve both values".to_string(),
        ));
    }
    spec.set_base_power_toughness_to_source_totals = power_is_source_total;

    spec.starting_loyalty = crate::slice_primitives::find_window_by(modifier_words, 4, |words| {
        crate::word_primitives::parse_sequence_complete(&words[..3], &["starting", "loyalty", "is"])
    })
    .and_then(|start| modifier_words.get(start..start + 4))
    .and_then(|words| crate::util::parse_number_word_u32(words[3]));

    let grants_keyword = |phrase, keyword: &str| {
        surface.has_phrase(phrase)
            || (surface.has(CreationWordClass::GrantVerb)
                && CreationWords::new(modifier_words).has_literal(keyword))
    };
    if grants_keyword(CreationPhrase::WithFlying, "flying") {
        push_unique(&mut spec.granted_abilities, StaticAbility::flying());
    }
    if grants_keyword(CreationPhrase::WithTrample, "trample") {
        push_unique(&mut spec.granted_abilities, StaticAbility::trample());
    }
    // "... and they have vigilance and menace" (Rebuild the City): the
    // keyword list after the last grant verb (CR 707.9a). Reading stops at
    // the first word that isn't a keyword, leaving quoted or self-referential
    // grants ("has this ability") to their own owners.
    // "except it's a 3/3 black Wraith with menace" (Sauron, the
    // Necromancer): an identity clause's `with` list grants the same way.
    let grant_positions = modifier_words
        .iter()
        .enumerate()
        .filter(|(_, word)| matches!(**word, "has" | "have" | "gain" | "gains" | "with"))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    for grant_idx in grant_positions {
        let list = &modifier_words[grant_idx + 1..];
        let mut idx = 0;
        // A list connective is read only when a keyword follows it.
        let mut pending_and = None;
        while idx < list.len() {
            let (ability, used) = match &list[idx..] {
                ["and", ..] => {
                    pending_and = Some(idx);
                    idx += 1;
                    continue;
                }
                ["first", "strike", ..] => (StaticAbility::first_strike(), 2),
                ["double", "strike", ..] => (StaticAbility::double_strike(), 2),
                ["vigilance", ..] => (StaticAbility::vigilance(), 1),
                ["menace", ..] => (StaticAbility::menace(), 1),
                ["haste", ..] => (StaticAbility::haste(), 1),
                ["reach", ..] => (StaticAbility::reach(), 1),
                ["lifelink", ..] => (StaticAbility::lifelink(), 1),
                ["deathtouch", ..] => (StaticAbility::deathtouch(), 1),
                ["hexproof", ..] => (StaticAbility::hexproof(), 1),
                ["defender", ..] => (StaticAbility::defender(), 1),
                ["flying", ..] => (StaticAbility::flying(), 1),
                ["trample", ..] => (StaticAbility::trample(), 1),
                _ => break,
            };
            push_unique(&mut spec.granted_abilities, ability);
            // The grant verb (or `with`), the keyword and its connective.
            crate::word_claims::claim_words(&modifier_words[grant_idx..=grant_idx]);
            if let Some(and_idx) = pending_and.take() {
                crate::word_claims::claim_words(&list[and_idx..=and_idx]);
            }
            crate::word_claims::claim_words(&list[idx..idx + used]);
            idx += used;
        }
    }
    if let Some(amount) =
        crate::slice_primitives::find_window_by(modifier_words, 2, |pair| pair[0] == "toxic")
            .and_then(|start| modifier_words.get(start..start + 2))
            .and_then(|pair| crate::util::decimal_count(pair[1]))
    {
        push_unique(
            &mut spec.granted_abilities,
            StaticAbility::keyword_marker(format!("toxic {amount}")),
        );
    }

    if let Some(idx) = surface.phrase_location(CreationPhrase::GetsForEach) {
        let mut tail = modifier_words.get(idx + 6..).unwrap_or_default();
        while CreationWords::new(tail).first_is(CreationWordClass::ArticleOrThe) {
            tail = &tail[1..];
        }
        if let Some(subtype) = tail
            .first()
            .and_then(|word| crate::util::parse_subtype_flexible(word))
            && CreationWords::new(tail).has_phrase(CreationPhrase::YouControl)
        {
            let mut filter = ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.controller = Some(PlayerFilter::You);
            filter.subtypes = vec![subtype];
            let count = AnthemCountExpression::MatchingFilter(filter);
            let anthem = crate::model::CompilerAnthem::for_source(0, 0).with_values(
                AnthemValue::scaled(1, count.clone()),
                AnthemValue::scaled(1, count),
            );
            spec.granted_abilities.push(StaticAbility::new(anthem));
        }
    }

    if let Some(addition) = surface.phrase_location(CreationPhrase::AdditionToOtherTypes) {
        // "except it's a Spirit in addition to its other types": the
        // identity copula, its article and the addition phrase read as the
        // added-types modifier.
        let phrase_len = CreationPhrase::AdditionToOtherTypes
            .alternatives()
            .iter()
            .filter(|alternative| modifier_words[addition..].starts_with(alternative))
            .map(|alternative| alternative.len())
            .max()
            .unwrap_or(0);
        crate::word_claims::claim_words(&modifier_words[addition..addition + phrase_len]);
        let mut colors = ColorSet::new();
        for word in &modifier_words[..addition] {
            if CreationWords::new(&[*word]).first_is(CreationWordClass::Article)
                || matches!(*word, "its" | "it" | "is" | "s" | "it's" | "it’s" | "they" | "are" | "re" | "theyre" | "they're" | "they’re")
            {
                crate::word_claims::claim_word(word);
            }
            if *word == "colorless" {
                // Colorless is an explicit empty color set, not a sixth color.
                // It is read by this descriptor even though parse_color only
                // recognizes the five individual colors.
                crate::word_claims::claim_word(word);
            }
            if let Some(color) = crate::util::parse_color(word) {
                colors = colors.union(color);
            }
            if let Some(card_type) = crate::util::parse_card_type(word) {
                push_unique(&mut spec.added_card_types, card_type);
            }
            if let Some(subtype) = crate::util::parse_subtype_flexible(word) {
                push_unique(&mut spec.added_subtypes, subtype);
            }
        }
        let explicit_colorless =
            crate::word_primitives::contains_word(&modifier_words[..addition], "colorless");
        // "in addition to its other colors and types" keeps the copied
        // colors; "in addition to its other types" replaces them.
        let keeps_colors = modifier_words[addition..]
            .iter()
            .take(6)
            .any(|word| matches!(*word, "colors" | "colours"));
        if keeps_colors && !explicit_colorless {
            spec.added_colors = colors;
        } else {
            spec.set_colors = (explicit_colorless || !colors.is_empty()).then_some(colors);
        }
    } else if surface.starts(CreationPhrase::IdentityClause) {
        let descriptor_end = surface
            .location(CreationWordClass::DescriptorEnd)
            .unwrap_or(modifier_words.len());
        let mut colors = ColorSet::new();
        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &modifier_words[..descriptor_end] {
            if CreationWords::new(&[*word]).first_is(CreationWordClass::Article)
                || matches!(
                    *word,
                    "its"
                        | "it"
                        | "is"
                        | "s"
                        | "it's"
                        | "it’s"
                        | "they"
                        | "are"
                        | "re"
                        | "theyre"
                        | "they're"
                        | "they’re"
                        // "except that the copy is red" (Fork): the
                        // complementizer and the copy subject.
                        | "that"
                        | "copy"
                        | "copies"
                )
            {
                // "except it's a 4/4 black Zombie": the identity copula and
                // its article introduce the replacement characteristics.
                crate::word_claims::claim_word(word);
                continue;
            }
            if parse_pt_word(word).is_some() {
                continue;
            }
            if *word == "colorless" {
                // Colorless is an explicit empty color set, not a sixth color.
                // It is read by this descriptor even though parse_color only
                // recognizes the five individual colors.
                crate::word_claims::claim_word(word);
            }
            if let Some(color) = crate::util::parse_color(word) {
                colors = colors.union(color);
            }
            if let Some(card_type) = crate::util::parse_card_type(word) {
                push_unique(&mut card_types, card_type);
            }
            if let Some(subtype) = crate::util::parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        let explicit_colorless =
            crate::word_primitives::contains_word(&modifier_words[..descriptor_end], "colorless");
        spec.set_colors = (explicit_colorless || !colors.is_empty()).then_some(colors);
        spec.set_card_types = (!card_types.is_empty()).then_some(card_types);
        spec.set_subtypes = (!subtypes.is_empty()).then_some(subtypes);
    }
    Ok(spec)
}

#[cfg(test)]
mod quoted_identity_copy_tests {
    use super::*;
    #[test]
    fn dangling_copy_exception_is_not_an_empty_modifier() {
        assert!(parse_copy_modifier_words(&["except"]).is_err());
        assert!(parse_copy_modifier_words(&["it", "except"]).is_err());
        assert!(parse_copy_modifier_words(&[]).is_ok());
        assert!(parse_copy_modifier_words(&["except", "it", "has", "haste"]).is_ok());
    }
    #[test]
    fn quoted_copy_exclusions_keep_full_names_and_independent_following_clauses() {
        for name in ["steady craft","traveler's gift","craft-and-care"] {
            let text=format!("except it doesn't have custom keyword \"{name}\" and it has haste");
            let tokens=crate::lexer::lex_line(&text,0).unwrap();
            let spec=parse_copy_modifier_tokens(&tokens).unwrap();
            assert_eq!(spec.removed_ability_designations,vec![name]);
            assert!(spec.granted_abilities.iter().any(|ability|ability.id==Some(crate::static_abilities::StaticAbilityId::Haste)));
        }
        for text in ["except it doesn't have custom keyword \"steady craft", "except it doesn't have custom keyword \"craft 2\""] {
            assert!(parse_copy_modifier_tokens(&crate::lexer::lex_line(text,0).unwrap()).is_err());
        }
    }
}

#[cfg(test)]
mod colorless_descriptor_claim_tests {
    use super::*;

    #[test]
    fn colorless_copy_descriptors_claim_the_empty_color_set() {
        for text in [
            "except it is colorless",
            "except it is colorless, it has base power and toughness 1/1, it has no mana cost, and it is a Spirit artifact in addition to its other types",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (parsed, claims) = crate::word_claims::scoped(|| {
                crate::word_claims::register_tokens(&tokens);
                parse_copy_modifier_tokens(&tokens)
            });
            let spec = parsed.unwrap();
            assert_eq!(spec.set_colors, Some(ColorSet::new()));
            assert!(!crate::word_claims::unclaimed_words(&tokens, &claims).contains(&"colorless"));
            if text.contains("in addition") {
                assert_eq!(spec.added_card_types, vec![CardType::Artifact]);
                assert_eq!(spec.added_subtypes, vec![Subtype::Spirit]);
                assert_eq!(spec.set_base_power_toughness, Some((1, 1)));
                assert!(spec.without_mana_cost);
                assert!(spec.set_card_types.is_none());
                assert!(spec.set_subtypes.is_none());
            }
        }
    }
}
