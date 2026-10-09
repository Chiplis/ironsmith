//! Optional mana-cost replacements, without granting a casting origin.
use super::*;
use crate::grammar::permission_shapes::find_words;
use crate::lexer::{TokenWordView, render_token_slice};

pub fn parse_independent_alternative_price_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The commander tax life substitution (Liesa) has its own owner.
    if super::commander_tax_life::parse_commander_tax_life_line(tokens).is_some() {
        return Ok(None);
    }
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(OwnedLexToken::is_quote) {
        return Ok(None);
    }
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let starts = view.token_start_indices();
    if words
        == [
            "rather",
            "than",
            "pay",
            "the",
            "mana",
            "cost",
            "for",
            "a",
            "spell",
            "its",
            "controller",
            "may",
            "discard",
            "a",
            "card",
            "that",
            "shares",
            "a",
            "color",
            "with",
            "that",
            "spell",
        ]
    {
        let mut discard = ObjectFilter::default();
        discard.zone = Some(Zone::Hand);
        discard.other = true;
        discard.characteristic_relations.push(
            ironsmith_core::ObjectCharacteristicRelation::shares(
                vec![ironsmith_core::ObjectCharacteristic::Color],
                ObjectFilter::source(),
            ),
        );
        let cost = crate::model::CompilerCost::Discard {
            count: 1,
            card_types: Vec::new(),
            supertypes: Vec::new(),
            filter: Some(discard),
            random: false,
            name: None,
            other: true,
            binding: None,
        };
        let mut spec = crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::AlternativePrice {
                costs: vec![cost],
                origin: None,
            },
            ObjectFilter::nonland(),
            Zone::Hand,
        )
        .with_beneficiary(PlayerFilter::Any);
        spec.filtered_zone_surface = Some("Rather than pay the mana cost for a spell, its controller may discard a card that shares a color with that spell".into());
        return Ok(Some(StaticAbility::grants(spec)));
    }
    let once = words.starts_with(&["once", "each", "turn"]);
    let head = if once { 3 } else { 0 };
    if words.get(head..head + 2) != Some(&["you", "may"][..]) {
        return Ok(None);
    }
    let Some(rather) = find_words(&words, &["rather", "than"]) else {
        return Ok(None);
    };
    let cast_head = words.get(head + 2) == Some(&"cast");
    // Existing generic fixed-price surfaces retain their prior owner. This
    // production adds bounded once-turn, keyword/energy and cast-by surfaces.
    if !once
        && !cast_head
        && words.get(head + 2) == Some(&"pay")
        && words.get(head + 3) == Some(&"rather")
    {
        return Ok(None);
    }
    // "You may pay {W}{U}{B}{R}{G} rather than pay the mana cost for spells you
    // cast": a pure mana price is owned by the fixed alternative-mana-cost
    // grant. Mana groups contribute parser word pieces ("w", "0"), so the word
    // check above never sees "rather" directly after "pay" for them; without
    // this deferral both readings match with different ASTs and the registry
    // rejects the line as ambiguous.
    if !once
        && !cast_head
        && words.get(head + 2) == Some(&"pay")
        && let Some(pay_token) = starts.get(head + 2).copied()
        && let Some(rather_token) = starts.get(rather).copied()
        && rather_token > pay_token + 1
        && tokens[pay_token + 1..rather_token]
            .iter()
            .all(|token| token.kind == TokenKind::ManaGroup)
    {
        return Ok(None);
    }
    let error = |what: &str| {
        CardTextError::ParseError(format!("unsupported independent alternative price: {what}"))
    };
    let (subject_start, subject_end, cost_start, cost_end, trailing_start) = if cast_head {
        let Some(by) = find_words(&words[head + 3..rather], &["by"]).map(|i| i + head + 3) else {
            return Ok(None);
        };
        if words.get(rather + 2..rather + 6) != Some(&["paying", "their", "mana", "costs"][..]) {
            return Ok(None);
        }
        (head + 3, by, starts[by] + 1, starts[rather], rather + 6)
    } else {
        if words.get(rather + 2..rather + 7) != Some(&["pay", "the", "mana", "cost", "for"][..]) {
            return Ok(None);
        }
        let Some(cast) = find_words(&words[rather + 7..], &["you", "cast"]).map(|i| i + rather + 7)
        else {
            return Ok(None);
        };
        (
            rather + 7,
            cast,
            starts[head + 1] + 1,
            starts[rather],
            cast + 2,
        )
    };
    if subject_start >= subject_end {
        return Err(error("missing spell subject"));
    }
    let mut tail_end = words.len();
    let flash = [
        "if", "you", "cast", "a", "spell", "this", "way", "you", "may", "cast", "it", "as",
        "though", "it", "had", "flash",
    ];
    let instant_timing = words.ends_with(&flash);
    if instant_timing {
        tail_end -= flash.len();
    }
    let tail = &words[trailing_start..tail_end];
    let mut origin = None;
    let mut source_counter_bound = None;
    if !tail.is_empty() {
        if tail == ["from", "your", "hand"] {
            origin = Some(Zone::Hand);
        } else if tail == ["from", "exile"] {
            origin = Some(Zone::Exile);
        } else if tail.starts_with(&[
            "with", "mana", "value", "x", "or", "less", "where", "x", "is", "the", "number", "of",
        ]) {
            let counter_start = trailing_start + 12;
            let Some(counter_end) = (counter_start..tail_end)
                .find(|i| words[*i] == "counters" || words[*i] == "counter")
            else {
                return Err(error("missing source counter type"));
            };
            if words.get(counter_end + 1) != Some(&"on")
                || !crate::util::is_source_reference_words(&words[counter_end + 2..tail_end])
            {
                return Err(error("counter bound must name this source"));
            }
            source_counter_bound = Some(
                crate::grammar::filters::parse_counter_type_from_tokens(
                    &tokens[starts[counter_start]..starts[counter_end]],
                )
                .ok_or_else(|| error("unknown source counter"))?,
            );
        } else {
            return Err(error("unrepresented origin, bound or follow-up"));
        }
    }
    let mut subject = tokens[starts[subject_start]..starts[subject_end]].to_vec();
    // Object filter grammar consumes the full characteristic description. A
    // casting subject's 'spell' means its chosen spell face, not Stack zone.
    if !subject
        .iter()
        .any(|token| token.is_any_word(&["spell", "spells"]))
    {
        return Err(error("subject is not spells"));
    }
    for token in &mut subject {
        if token.is_word("spell") {
            token.replace_word("card");
        } else if token.is_word("spells") {
            token.replace_word("cards");
        }
    }
    let mut filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
        &subject, false,
    )?;
    filter.zone = None;
    if origin == Some(Zone::Hand) {
        filter.owner = Some(PlayerFilter::You);
    }
    if !filter.excluded_card_types.contains(&CardType::Land) {
        filter.excluded_card_types.push(CardType::Land);
    }
    if let Some(counter) = source_counter_bound {
        if filter.mana_value.is_some() {
            return Err(error("two mana-value bounds"));
        }
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
            Value::CountersOnSource(counter),
        )));
    }
    let mut cost_tokens = tokens[cost_start..cost_end].to_vec();
    if let Some(first) = cost_tokens.first_mut()
        && first.is_word("paying")
    {
        first.replace_word("pay");
    }
    let total_cost = parse_payment_clause_as_total_cost(&cost_tokens)?
        .ok_or_else(|| error("unrepresented payment"))?;
    if total_cost.as_all().is_none() {
        return Err(error(
            "price branch choice requires an announcement binding",
        ));
    }
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativePrice {
            costs: total_cost.costs().to_vec(),
            origin,
        },
        filter,
        Zone::Hand,
    );
    spec.usage_limit = once.then_some(crate::grant::GrantUsageLimit::OnceEachTurn);
    spec.instant_timing = instant_timing;
    // Presentation only. Every cost/filter/origin/timing word has been consumed
    // above; execution never reads this complete, strictly recognized surface.
    let surface = render_token_slice(tokens);
    let surface = surface.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(StaticAbility::grants(spec)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(text: &str) -> Result<Option<StaticAbility>, CardTextError> {
        let mut tokens = crate::lexer::lex_line(text, 0).unwrap();
        for token in &mut tokens {
            token.lowercase_word();
        }
        parse_independent_alternative_price_line(&tokens)
    }
    #[test]
    fn price_origins_conditions_costs_and_flash_are_typed() {
        let ability=read("Once each turn, you may pay {0} rather than pay the mana cost for a spell you cast with mana value X or less, where X is the number of time counters on this enchantment.").unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::Grants(spec) = ability.payload else {
            panic!("grant")
        };
        assert_eq!(
            spec.usage_limit,
            Some(crate::grant::GrantUsageLimit::OnceEachTurn)
        );
        assert!(matches!(
            spec.grantable,
            ironsmith_core::Grantable::AlternativePrice { origin: None, .. }
        ));
        assert_eq!(
            spec.filter.mana_value,
            Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
                Value::CountersOnSource(crate::object::CounterType::Time)
            )))
        );
        let ability=read("Once each turn, you may pay {0} rather than pay the mana cost for a colorless spell you cast from your hand.").unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::Grants(spec) = ability.payload else {
            panic!("grant")
        };
        assert_eq!(spec.filter.owner, Some(PlayerFilter::You));
        assert!(matches!(
            spec.grantable,
            ironsmith_core::Grantable::AlternativePrice {
                origin: Some(Zone::Hand),
                ..
            }
        ));
        let ability=read("You may cast creature spells with mana value 3 or less by paying {E} rather than paying their mana costs. If you cast a spell this way, you may cast it as though it had flash.").unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::Grants(spec) = ability.payload else {
            panic!("grant")
        };
        assert!(spec.instant_timing);
    }
    #[test]
    fn unrelated_qualifiers_and_unknown_followups_are_not_dropped() {
        for text in [
            "Once each turn, you may pay {0} rather than pay the mana cost for a creature spell you cast from your opponent's hand.",
            "You may collect evidence 10 rather than pay the mana cost for spells you cast. Draw a card.",
            "You may cast creature spells with mana value 3 or less by paying {E} rather than paying their mana costs. It can't be countered.",
        ] {
            assert!(!matches!(read(text), Ok(Some(_))), "{text}");
        }
    }
}
