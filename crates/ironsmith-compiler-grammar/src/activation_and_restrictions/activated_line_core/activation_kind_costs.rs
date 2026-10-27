//! Complete activation-kind cost clauses. Scope belongs to the selected
//! activation, not an ability marker on its source or presentation text.
use super::*;
use ironsmith_core::{ActivatedAbilityCostCondition as Gate, ActivatedAbilityKeyword as Keyword};

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(tokens)
}
fn error(tokens: &[OwnedLexToken], part: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "unsupported activation-kind cost {part} (clause: '{}')", render_token_slice(tokens)
    ))
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = crate::grammar::structure::split_lexed_sentences(tokens);
    let Some(first) = sentences.first() else { return Ok(None); };
    let first = crate::util::trim_edge_punctuation_tokens(first);
    let Some(cost) = first.iter().position(|token| token.is_any_word(&["cost", "costs"])) else {
        return Ok(None);
    };
    let subject = &first[..cost];
    let subject_words = words(subject);
    // "The first activated ability of an artifact you activate each turn
    // costs {2} less to activate." (Tezzeret, Betrayer of Flesh): the
    // activator's first activation this turn of an ability whose source
    // matches the filter.
    if let ["the", "first", "activated", "ability", "of", .., "you", "activate", "each", "turn"] =
        subject_words.as_slice()
        && sentences.len() == 1
        && subject.len() > 9
    {
        let filter_tokens = &subject[5..subject.len() - 4];
        let mut filter =
            crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
                filter_tokens,
                false,
            )
            .map_err(|_| error(tokens, "first-activation source filter"))?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        let amount_tokens = &first[cost + 1..];
        let Some((amount, used)) = parse_cost_modifier_amount(amount_tokens) else {
            return Err(error(tokens, "generic amount"));
        };
        let Value::Fixed(amount) = amount.unhinted() else {
            return Err(error(tokens, "fixed amount"));
        };
        let amount = u32::try_from(*amount).map_err(|_| error(tokens, "negative amount"))?;
        if words(&amount_tokens[used..]) != ["less", "to", "activate"] {
            return Err(error(tokens, "first-activation direction"));
        }
        let display = render_token_slice(tokens).trim().trim_end_matches('.').to_string();
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter.clone(),
                amount,
                None,
                display,
            )
            .with_activated_ability_cost_condition(Gate::Activator(PlayerFilter::You))
            .with_activated_ability_cost_condition(
                Gate::FirstActivationOfMatchingSourceThisTurn { filter },
            ),
        ));
    }
    // "The first activated ability you activate during your turn that targets
    // a creature you control costs {2} less to activate." (Professor Hojo)
    if let [
        "the", "first", "activated", "ability", "you", "activate", "during", "your", "turn",
        "that", "targets", ..,
    ] = subject_words.as_slice()
        && sentences.len() == 1
        && let Some(targets_at) = subject.iter().position(|token| token.is_word("targets"))
        && targets_at + 1 < subject.len()
    {
        let filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
            &subject[targets_at + 1..],
            false,
        )
        .map_err(|_| error(tokens, "first-targeting-activation target filter"))?;
        let amount_tokens = &first[cost + 1..];
        let Some((amount, used)) = parse_cost_modifier_amount(amount_tokens) else {
            return Err(error(tokens, "generic amount"));
        };
        let Value::Fixed(amount) = amount.unhinted() else {
            return Err(error(tokens, "fixed amount"));
        };
        let amount = u32::try_from(*amount).map_err(|_| error(tokens, "negative amount"))?;
        if words(&amount_tokens[used..]) != ["less", "to", "activate"] {
            return Err(error(tokens, "first-targeting-activation direction"));
        }
        let display = render_token_slice(tokens).trim().trim_end_matches('.').to_string();
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                ObjectFilter::default(),
                amount,
                None,
                display,
            )
            .with_activated_ability_cost_condition(Gate::Activator(PlayerFilter::You))
            .with_activated_ability_cost_condition(
                Gate::FirstTargetingActivationDuringYourTurn { filter },
            ),
        ));
    }
    let Some(abilities) = subject.iter().position(|token| token.is_word("abilities")) else {
        return Ok(None);
    };
    let kind = match words(&subject[..abilities]).as_slice() {
        ["cycling"] => Some(Gate::Keyword(Keyword::Cycling)),
        ["ninjutsu"] => Some(Gate::Keyword(Keyword::Ninjutsu)),
        ["boast"] => Some(Gate::Keyword(Keyword::Boast)),
        ["exhaust"] => Some(Gate::Keyword(Keyword::Exhaust)),
        ["power-up"] | ["powerup"] | ["power", "up"] => Some(Gate::Keyword(Keyword::PowerUp)),
        ["loyalty"] => Some(Gate::LoyaltyAbility),
        // "Planeswalkers' loyalty abilities you activate ..." (Carth the
        // Lion): loyalty abilities, scoped to planeswalker sources below.
        ["planeswalkers'" | "planeswalkers" | "planeswalker's", "loyalty"] => {
            Some(Gate::LoyaltyAbility)
        }
        ["mana"] => Some(Gate::ManaAbility),
        ["activated"] | [] => None,
        _ => return Ok(None),
    };
    // Ordinary 'activated abilities of ...' keeps its established owner.
    // Only the complete all-abilities exception surface belongs here.
    if kind.is_none()
        && !matches!(subject_words.as_slice(), ["activated", "abilities"])
        && !matches!(subject_words.as_slice(), ["abilities", "you", "activate", "that", "aren't" | "arent", "mana", "abilities"])
    {
        return Ok(None);
    }
    let planeswalker_scope = matches!(
        words(&subject[..abilities]).as_slice(),
        ["planeswalkers'" | "planeswalkers" | "planeswalker's", "loyalty"]
    );
    let loyalty_scope = matches!(kind, Some(Gate::LoyaltyAbility));
    let mut filter = if planeswalker_scope {
        ObjectFilter::planeswalker()
    } else {
        ObjectFilter::default()
    };
    let mut gates = Vec::new();
    if let Some(kind) = kind { gates.push(kind); }
    let scope = &subject[abilities + 1..];
    match words(scope).as_slice() {
        [] => {},
        ["you", "activate"] => gates.push(Gate::Activator(PlayerFilter::You)),
        ["your", "opponents", "activate"] => gates.push(Gate::Activator(PlayerFilter::Opponent)),
        ["you", "activate", "that", "aren't" | "arent", "mana", "abilities"] => {
            gates.push(Gate::Activator(PlayerFilter::You));
            gates.push(Gate::NonManaAbility);
        }
        // "Mana abilities of this land ..." (Thran Portal): the source's own
        // abilities.
        ["of", rest @ ..] if crate::util::is_source_reference_words(rest) => {
            filter = ObjectFilter::source();
        }
        ["of", ..] if scope.len() > 1 => {
            filter = crate::keyword_static::parse_complete_cost_count_filter(&scope[1..])?
                .ok_or_else(|| error(tokens, "complete source scope"))?;
            // A source qualifier is an object scope, distinct from the payer.
            if filter.zone.is_none() { filter.zone = Some(Zone::Battlefield); }
        }
        _ => return Err(error(tokens, "source or activator scope")),
    }
    let amount_tokens = &first[cost + 1..];
    // "cost an additional [+1] to activate" (Carth the Lion): a loyalty
    // surcharge; the engine folds it into the printed loyalty cost so a
    // [-2] ability costs [-1].
    if loyalty_scope
        && sentences.len() == 1
        && let ["an", "additional", .., "to", "activate"] = words(amount_tokens).as_slice()
        && let Some(additional) = amount_tokens.iter().position(|token| token.is_word("additional"))
        && let Some(to) = amount_tokens.iter().rposition(|token| token.is_word("to"))
        && to > additional + 1
    {
        let increase = parse_compiler_activation_cost(&amount_tokens[additional + 1..to])?;
        if increase.is_free() {
            return Err(error(tokens, "loyalty surcharge"));
        }
        let display = render_token_slice(tokens).trim().trim_end_matches('.').to_string();
        let mut ability = StaticAbility::increase_activated_ability_costs(filter, increase);
        if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { display: slot, .. } = &mut ability.payload {
            *slot = Some(display);
        }
        for gate in gates { ability = ability.with_activated_ability_cost_condition(gate); }
        return Ok(Some(ability));
    }
    // "cost an additional 1 life to activate" (Thran Portal): a life
    // surcharge on the selected activation.
    if let ["an", "additional", amount, "life", "to", "activate"] = words(amount_tokens).as_slice()
        && let Ok(life) = amount.parse::<u32>()
        && sentences.len() == 1
    {
        let pay = crate::lexer::synthetic_word_tokens(["pay", &life.to_string(), "life"]);
        let increase = parse_compiler_activation_cost(&pay)?;
        let display = render_token_slice(tokens).trim().trim_end_matches('.').to_string();
        let mut ability = StaticAbility::increase_activated_ability_costs(filter, increase);
        if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { display: slot, .. } = &mut ability.payload {
            *slot = Some(display);
        }
        for gate in gates { ability = ability.with_activated_ability_cost_condition(gate); }
        return Ok(Some(ability));
    }
    let optional = words(amount_tokens).starts_with(&["up", "to"]);
    let amount_tokens = if optional { &amount_tokens[2..] } else { amount_tokens };
    let Some((amount, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Err(error(tokens, "generic amount"));
    };
    let Value::Fixed(amount) = amount.unhinted() else { return Err(error(tokens, "fixed amount")); };
    let amount = u32::try_from(*amount).map_err(|_| error(tokens, "negative amount"))?;
    let remainder = &amount_tokens[used..];
    let Some(direction) = remainder.first().and_then(OwnedLexToken::as_word) else {
        return Err(error(tokens, "direction"));
    };
    if !matches!(direction, "less" | "more") || remainder.len() < 3
        || words(&remainder[1..3]) != ["to", "activate"]
    { return Err(error(tokens, "direction or action")); }
    let tail = &remainder[3..];
    let mut per_objects = None;
    match words(tail).as_slice() {
        [] => {},
        ["unless", "they're" | "theyre", "mana", "abilities"] => gates.push(Gate::NonManaAbility),
        ["for", "each", ..] if direction == "less" && tail.len() > 2 => {
            let mut counted = crate::keyword_static::parse_complete_cost_count_filter(&tail[2..])?
                .ok_or_else(|| error(tokens, "counted objects"))?;
            if counted.zone.is_none() { counted.zone = Some(Zone::Battlefield); }
            per_objects = Some(counted);
        }
        _ => return Err(error(tokens, "trailing qualifier")),
    }
    let minimum = match sentences.as_slice() {
        [_] => None,
        [_, minimum] if direction == "less" && matches!(words(minimum).as_slice(),
            ["this", "effect", "can't" | "cant", "reduce", "the", "mana", "in", "that", "cost", "to", "less", "than", "one", "mana"]
        ) => Some(1),
        _ => return Err(error(tokens, "trailing sentence")),
    };
    if optional && (direction != "less" || per_objects.is_some() || minimum.is_some()
        || !gates.iter().any(|gate| matches!(gate, Gate::Keyword(Keyword::Cycling)))) {
        return Err(error(tokens, "optional reduction scope"));
    }
    // Runtime rendering owns the typed count filter (including text changes).
    // Keep only the cost head in its presentation to avoid printing it twice.
    let display_tokens = if per_objects.is_some() {
        &first[..cost + 1 + used + 3]
    } else {
        tokens
    };
    let display = render_token_slice(display_tokens).trim().trim_end_matches('.').to_string();
    let mut ability = if direction == "less" {
        let mut ability = StaticAbility::reduce_activated_ability_costs_with_display(filter, amount, minimum, display);
        if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction { per_matching_objects, optional: optional_slot, .. } = &mut ability.payload {
            *per_matching_objects = per_objects;
            *optional_slot = optional;
        }
        ability
    } else {
        // The cost representation limits one generic pip to u8. Reject a
        // larger unsupported source amount instead of clamping its surcharge.
        let amount = u8::try_from(amount).map_err(|_| error(tokens, "surcharge range"))?;
        let increase = ironsmith_core::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(amount)]));
        let mut ability = StaticAbility::increase_activated_ability_costs(filter, increase);
        if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { display: slot, .. } = &mut ability.payload {
            *slot = Some(display);
        }
        ability
    };
    for gate in gates { ability = ability.with_activated_ability_cost_condition(gate); }
    Ok(Some(ability))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(text: &str) -> Result<Option<StaticAbility>, CardTextError> {
        parse(&crate::lexer::lex_line(text, 0).unwrap())
    }
    #[test]
    fn complete_scopes_and_nonmana_floor_are_retained() {
        for (text, expected) in [
            ("Cycling abilities you activate cost {2} less to activate.", "Cycling"),
            ("Ninjutsu abilities you activate cost {1} less to activate.", "Ninjutsu"),
            ("Boast abilities you activate cost {1} less to activate for each Dragon you control.", "Boast"),
            ("Exhaust abilities of other permanents you control cost {2} less to activate.", "Exhaust"),
            ("Power-up abilities of other creatures you control cost {3} less to activate.", "PowerUp"),
            ("Loyalty abilities of planeswalkers your opponents control cost {1} more to activate.", "LoyaltyAbility"),
            ("Activated abilities cost {2} more to activate unless they're mana abilities.", "NonManaAbility"),
            ("Abilities you activate that aren't mana abilities cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.", "NonManaAbility"),
        ] {
            let ability = read(text).unwrap().unwrap();
            assert!(format!("{ability:?}").contains(expected), "{text}: {ability:?}");
        }
    }
    #[test]
    fn unrelated_keyword_and_cost_tails_are_never_discarded() {
        for text in [
            "Cycling abilities you activate cost {2} less to activate except on Tuesdays.",
            "Ninjutsu abilities you activate cost {1} less to activate. Draw a card.",
            "Boast abilities you activate cost {1} less to activate for each Dragon you control with invented quality.",
            "Exhaust abilities of other permanents you control with invented quality cost {2} less to activate.",
            "Loyalty abilities of planeswalkers your opponents control cost {256} more to activate.",
        ] { assert!(read(text).is_err(), "{text}"); }
        assert!(read("Activated abilities of creatures you control cost {2} less to activate.").unwrap().is_none());
        assert!(read("Cycling {2}").unwrap().is_none());
    }
}
