use crate::activation_and_restrictions::parse_payment_clause_as_total_cost;
use crate::cards::builders::CardTextError;
use crate::grammar::{leaf, permission_shapes};
use crate::keyword_static::parse_this_spell_cost_condition;
use crate::lexer::{OwnedLexToken, TokenWordView, lex_line, render_token_slice};
use crate::mana::ManaCost;
use crate::model::CompilerAlternativeCastingMethod as AlternativeCastingMethod;
use crate::static_abilities::ThisSpellCostCondition;
use crate::util::trim_edge_punctuation;

/// Explanatory prose may restate a paid cost, but must never add a different
/// payment or silently change the eligible objects.
fn validates_cost_explanation(
    tokens: &[OwnedLexToken],
    cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let ["to", "behold", "a" | "an", named, "choose", "a" | "an", controlled,
        "you", "control", "or", "reveal", "a" | "an", revealed, "card", "from", "your", "hand"] = words.as_slice() else {
        return false;
    };
    let Some(subtype) = crate::util::parse_subtype_flexible(named) else { return false; };
    if crate::util::parse_subtype_flexible(controlled) != Some(subtype)
        || crate::util::parse_subtype_flexible(revealed) != Some(subtype) {
        return false;
    }
    fn contains_payment(cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>, subtype: ironsmith_core::types::Subtype) -> bool {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(costs) => costs.iter().any(|cost| matches!(
                cost, crate::model::CompilerCost::Behold { subtype: paid, .. } if *paid == subtype
            )),
            ironsmith_core::TotalCostKind::OneOf(branches) => !branches.is_empty()
                && branches.iter().all(|branch| contains_payment(branch, subtype)),
        }
    }
    contains_payment(cost, subtype)
}

/// An intrinsic permission to cast this exact card from its owner's
/// graveyard for a complete alternative price. It does not grant a price to
/// other cards or make any other casting zone available.
pub fn parse_self_zone_alternative_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let tokens = crate::util::trim_statement_separators(tokens).to_vec();
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    let first = sentences.first().copied().unwrap_or(&tokens);
    let first = trim_edge_punctuation(first);
    let first_view = TokenWordView::new(&first);
    let first_words = first_view.word_refs();
    let prose = render_token_slice(&tokens).to_ascii_lowercase();
    if let Some(rest) = prose.strip_prefix("you may cast this spell for ")
        && let Some((price, rest)) = rest.split_once(" rather than pay its mana cost. if you do, put ")
        && let Some(amount) = rest.trim_end_matches('.').strip_suffix(" +1/+1 counters on target land you control. that land becomes a 0/0 elemental creature with haste. it's still a land")
        && let Ok(amount) = amount.parse::<u32>() {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?;
        if matches!(total.kind(), ironsmith_core::TotalCostKind::OneOf(_)) { return Ok(None); }
        return Ok(Some(AlternativeCastingMethod::Awaken {
            amount, cost: total.mana_cost().cloned().unwrap_or_default(), effects: Vec::new(), descriptive: true,
            additional_cost: ironsmith_core::TotalCost::from_costs(total.non_mana_costs().cloned().collect()),
        }));
    }
    // Suspend's full rule (CR 702.62a), or a described timed exile with its
    // own counter kind and no haste clause. Any other wording declines.
    if let Some(rest) = prose.strip_prefix("if you could begin to cast this card from your hand, you may pay ")
        && let Some((price, rest)) = rest.split_once(" and exile it with ")
        && let Some((time, rest)) = rest.split_once(' ')
        && let Ok(time) = time.parse::<u32>()
        && let Some((kind, rest)) = rest.split_once(" counters on it. this action doesn't use the stack. at the beginning of your upkeep, if this card is exiled with a ")
        && let Some(rest) = rest.strip_prefix(&format!("{kind} counter on it, remove a {kind} counter from it. when the last {kind} counter is removed from this card, if it's exiled, you may play it without paying its mana cost"))
        && let Some(haste) = match rest.trim_end_matches('.') {
            "" => Some(false),
            ". if you cast a creature spell this way, it gains haste until you lose control of the spell or the permanent it becomes" => Some(true),
            _ => None,
        }
        && let Some(counter) = crate::grammar::filters::parse_counter_type_words(
            &kind.split(' ').chain(["counters"]).collect::<Vec<_>>()) {
        let price = crate::grammar::values::parse_mana_cost_tokens(&lex_line(price, 0)?)?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            ironsmith_core::definition_model::GENERIC_TIMED_EXILE_NAME, crate::zone::Zone::Exile, ironsmith_core::TotalCost::free(), None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            timed_exile: Some((ironsmith_core::SuspendTime::Fixed(time), price)),
            timed_exile_counter: (counter != ironsmith_core::CounterType::Time).then_some(counter),
            timed_exile_without_haste: !haste,
            ..Default::default()
        })));
    }
    for (tail, aura) in [
        (" rather than pay its mana cost as an aura enchantment spell with enchant creature. if its target is illegal as it begins resolving, it instead resolves using its normal types. if the resulting aura becomes unattached or phases in unattached, it returns to its normal types", true),
        (" rather than pay its mana cost targeting a non-human creature with the same owner as this spell. as it resolves, if that target is legal, merge this spell above or below that creature without entering the battlefield. the resulting permanent has the top object's characteristics and all abilities of the merged objects. otherwise, this spell resolves normally as a creature", false),
    ] {
        if let Some(price) = prose.strip_prefix("you may cast this spell for ").and_then(|rest| rest.trim_end_matches('.').strip_suffix(tail)) {
            let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?;
            return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                if aura { "Parsed Aura casting choice" } else { "Parsed merging casting choice" }, crate::zone::Zone::Hand, total, None, false,
            ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
                aura_cast_with_unattached_fallback: aura, merge_with_same_owner_nonhuman: !aura, ..Default::default()
            })));
        }
    }
    if let Some(rest) = prose.strip_prefix("during your turn, you may pay ")
        && let Some((prepare, rest)) = rest.split_once(" and exile this card from your hand face down. cast it on a later turn for ") {
        let price = crate::grammar::values::parse_mana_cost_tokens(&lex_line(prepare, 0)?)?;
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(rest.trim_end_matches('.'), 0)?)?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed face-down preparation", crate::zone::Zone::Exile, total, None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            exile_preparation: Some(ironsmith_core::alternative_cast_model::ExilePreparation::FaceDown { price }), ..Default::default()
        })));
    }
    if let Some(prepare) = prose.strip_prefix("you may pay ")
        .and_then(|rest| rest.trim_end_matches('.').strip_suffix(" and exile this card from your hand. cast it as a sorcery on a later turn without paying its mana cost. activate only as a sorcery")) {
        let price = crate::grammar::values::parse_mana_cost_tokens(&lex_line(prepare, 0)?)?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed face-up preparation", crate::zone::Zone::Exile, ironsmith_core::TotalCost::free(), None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            exile_preparation: Some(ironsmith_core::alternative_cast_model::ExilePreparation::FaceUpSorcery { price }), ..Default::default()
        })));
    }
    if let Some(rest) = prose.strip_prefix("you may cast this spell with mana cost ")
        && let Some((price, rest)) = rest.split_once(", color determined by that mana cost, and base power and toughness ")
        && let Some(pt) = rest.trim_end_matches('.').strip_suffix(". it keeps its abilities and types")
        && let Some((power, toughness)) = pt.split_once('/')
        && let (Ok(power), Ok(toughness)) = (power.trim().parse::<i32>(), toughness.trim().parse::<i32>()) {
        let cost = crate::grammar::values::parse_mana_cost_tokens(&lex_line(price, 0)?)?;
        return Ok(Some(AlternativeCastingMethod::Composed {
            name: "Parsed casting characteristics".into(),
            total_cost: ironsmith_core::TotalCost::from_costs(vec![crate::model::CompilerCost::Mana(cost)]),
            condition: None, prototype_power_toughness: Some(ironsmith_core::PowerToughness::fixed(power, toughness)),
        }));
    }
    if let Some(price) = prose.strip_prefix("you may reveal this card from your hand as you draw it if it is the first card you drew this turn. when you reveal it this way, you may cast it for ")
        .and_then(|rest| rest.trim_end_matches('.').strip_suffix(" rather than pay its mana cost")) {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed first-draw reveal price", crate::zone::Zone::Hand, total, None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            first_draw_reveal_cast: true, ..Default::default()
        })));
    }
    if let Some(rest) = prose.strip_prefix("you may cast this spell from your hand for ")
        && let Some((price, rest)) = rest.split_once(" and sacrifice ")
        && let Some(quality) = rest.trim_end_matches('.').strip_suffix(" rather than pay its mana cost. reduce that price by an amount of generic mana equal to the sacrificed permanent's mana value") {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(&format!("{price}, Sacrifice {quality}"), 0)?)?;
        if matches!(total.kind(), ironsmith_core::TotalCostKind::OneOf(_)) { return Ok(None); }
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed sacrifice-reduced price", crate::zone::Zone::Hand, total, None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            sacrifice_reduces_generic_by_mana_value: true, ..Default::default()
        })));
    }

    if let Some(rest) = prose.strip_prefix("during your declare blockers step, any time you could cast an instant, you may cast this spell from your hand for ")
        && let Some(price) = rest.trim_end_matches('.').strip_suffix(" and return an unblocked attacker you control to its owner's hand rather than pay this spell's mana cost. it enters tapped and attacking the same player, planeswalker, or battle as the returned creature") {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?;
        if matches!(total.kind(), ironsmith_core::TotalCostKind::OneOf(_)) { return Ok(None); }
        let mut costs = total.costs().to_vec();
        costs.push(crate::model::CompilerCost::Sneak);
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed attack replacement price", crate::zone::Zone::Hand,
            ironsmith_core::TotalCost::from_costs(costs), None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            returns_attacker_enters_attacking: true, ..Default::default()
        })));
    }

    if let Some(rest) = prose.strip_prefix("you may cast this card from your hand for ")
        && let Some(price) = rest.trim_end_matches('.').strip_suffix(" rather than its mana cost. if you do, exile the permanent this spell becomes at the beginning of the next end step. its owner may cast that card after this turn ends for as long as it remains exiled") {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?;
        if matches!(total.kind(), ironsmith_core::TotalCostKind::OneOf(_)) { return Ok(None); }
        return Ok(Some(AlternativeCastingMethod::Warp {
            cost: total.mana_cost().cloned().unwrap_or_default(),
            additional_cost: ironsmith_core::TotalCost::from_costs(total.non_mana_costs().cloned().collect()),
            descriptive: true,
        }));
    }
    if let Some((cleave, price)) = described_spell_text_substitution(&tokens) {
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&price)?;
        let cost = total.mana_cost().cloned().unwrap_or_default();
        let additional_cost = ironsmith_core::TotalCost::from_costs(total.non_mana_costs().cloned().collect());
        return Ok(Some(if cleave {
            AlternativeCastingMethod::Cleave { cost, additional_cost, effects: Vec::new(), descriptive: true }
        } else {
            AlternativeCastingMethod::Overload { cost, additional_cost, effects: Vec::new(), descriptive: true }
        }));
    }
    if sentences.len() == 2
        && first_words == ["if", "you", "discard", "this", "card", "discard", "it", "into", "exile"] {
        let second = trim_edge_punctuation(sentences[1]);
        let view = TokenWordView::new(&second);
        let words = view.word_refs();
        if words.starts_with(&["when", "you", "do", "cast", "it", "for"])
            && words.ends_with(&["or", "put", "it", "into", "your", "graveyard"]) {
            let start = view.token_start_indices()[5] + 1;
            let end = view.token_start_indices()[words.len() - 6];
            let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&second[start..end])?;
            return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                "Parsed discard exile price", crate::zone::Zone::Exile, total_cost, None, false,
            ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
                cast_when_discarded_into_exile: true, ..Default::default()
            })));
        }
    }
    if sentences.len() == 3
        && first_words.starts_with(&["you", "may", "cast", "this", "card", "from", "your", "graveyard", "for"])
        && TokenWordView::new(sentences[1]).word_refs() == ["you", "may", "tap", "a", "creature", "you", "control", "to", "reduce", "that", "cost", "by", "an", "amount", "of", "generic", "mana", "equal", "to", "its", "power"]
        && matches!(TokenWordView::new(sentences[2]).word_refs().as_slice(), ["then", "exile", "this", "spell"] | ["then", "exile", "it"]) {
        let start = first_view.token_start_indices()[8] + 1;
        let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[start..])?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed creature-assisted graveyard price", crate::zone::Zone::Graveyard, total_cost, None, false,
        ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
            tap_creature_reduces_generic: true, exile_on_stack_exit: true, ..Default::default()
        })));
    }
    // A condition on this exact graveyard card is independent of keyword
    // identity. The timing reminder restates the ordinary casting window.
    if (sentences.len() == 1 || (sentences.len() == 2
        && TokenWordView::new(sentences[1]).word_refs() == ["timing", "rules", "still", "apply"]))
        && matches!(first_words.get(..9), Some(["you", "may", "cast", "this", "card" | "spell", "from", "your", "graveyard", "for"]))
        && first_words.ends_with(&["if", "you", "discarded", "it", "this", "turn"])
    {
        let start = first_view.token_start_indices()[8] + 1;
        let end = first_view.token_start_indices()[first_words.len() - 6];
        let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[start..end])?;
        let condition = ThisSpellCostCondition::ConditionExpr {
            condition: crate::effect::Condition::SourceMatches(
                crate::target::ObjectFilter::default()
                    .discarded_or_cycled_this_turn_by(crate::target::PlayerFilter::You),
            ),
            display: "you discarded this card this turn".into(),
        };
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed conditional graveyard price", crate::zone::Zone::Graveyard,
            total_cost, Some(condition), false,
        )));
    }
    let exile_on_exit = (2..=3).contains(&sentences.len()) && matches!(
        TokenWordView::new(sentences[1]).word_refs().as_slice(),
        ["then", "exile", "it"] | ["then", "exile", "this", "card" | "spell"]
    );
    if sentences.len() == 1 || exile_on_exit {
        if sentences.len() <= 2 && matches!(first_words.get(..7), Some(["you", "may", "cast", "this", "card" | "spell", "converted" | "transformed", "for"])) {
            let start = first_view.token_start_indices()[6] + 1;
            let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[start..])?;
            return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                "Parsed zone alternative cost", crate::zone::Zone::Hand, total_cost, None, false,
            ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
                transformed: true, exile_on_stack_exit: exile_on_exit, ..Default::default()
            })));
        }
        // A bare permission whose payment the ordinary graveyard-permission
        // reader understands stays with it; this reader owns the rest
        // (Retrace's filtered discard) and complete Jump-start definitions.
        if sentences.len() <= 2 && matches!(first_words.get(..9), Some(["you", "may", "cast", "this", "card" | "spell", "from", "your", "graveyard", "by"]))
            && first_words.ends_with(&["in", "addition", "to", "paying", "its", "other", "costs"])
            && (exile_on_exit || crate::grammar::permission_facts::graveyard_source::parse_graveyard_additional_cost_tokens(
                &first[first_view.token_start_indices()[8] + 1..first_view.token_start_indices()[first_words.len() - 7]]).is_none())
        {
            let start = first_view.token_start_indices()[8] + 1;
            let end = first_view.token_start_indices()[first_words.len() - 7];
            let mut payment = first[start..end].to_vec();
            if let Some(head) = payment.first_mut() {
                for (gerund, verb) in [("discarding", "discard"), ("sacrificing", "sacrifice"), ("exiling", "exile"), ("returning", "return"), ("paying", "pay"), ("tapping", "tap")] {
                    if head.is_word(gerund) { head.replace_word(verb); }
                }
            }
            // An unreadable payment belongs to another owner, not an error here.
            let Ok(total_cost) = crate::activation_and_restrictions::parse_compiler_activation_cost(&payment) else { return Ok(None); };
            return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                "Parsed graveyard alternative cost", crate::zone::Zone::Graveyard, total_cost, None, false,
            ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
                add_printed_mana_cost: true, exile_on_stack_exit: exile_on_exit, ..Default::default()
            })));
        }
        if matches!(first_words.get(..8), Some(["you", "may", "cast", "this", "card" | "spell", "from", "your", "graveyard"])) {
            let transformed = first_words.get(8) == Some(&"transformed");
            let for_index = if transformed { 9 } else { 8 };
            if first_words.get(for_index) == Some(&"for") {
                let start = first_view.token_start_indices()[for_index] + 1;
                // "and any additional costs" restates CR 601.2f: additional
                // costs always apply to an alternative price.
                let end = if first_words.ends_with(&["and", "any", "additional", "costs"]) {
                    first_view.token_start_indices()[first_words.len() - 4]
                } else { first.len() };
                let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[start..end])?;
                if sentences.len() == 3 && !validates_cost_explanation(sentences[2], &total_cost) {
                    return Err(CardTextError::ParseError("alternative-cost explanation does not describe the bound payment".into()));
                }
                return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                    "Parsed graveyard alternative cost", crate::zone::Zone::Graveyard, total_cost, None, false,
                ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
                    transformed, exile_on_stack_exit: exile_on_exit, ..Default::default()
                })));
            }
        }
    }
    let view = TokenWordView::new(&tokens);
    let words = view.word_refs();
    // A fully bound reminder price uses the same total-cost grammar as an
    // activated ability, preserving mana and every non-mana payment. The
    // zone permission carries no Escape/Flashback keyword identity.
    if matches!(words.get(..9), Some(["you", "may", "cast", "this", "card" | "spell" | "creature", "from", "your", "graveyard", "for"])) {
        let cost_start = view.token_start_indices()[8] + 1;
        let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&tokens[cost_start..])?;
        return Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Parsed graveyard alternative cost", crate::zone::Zone::Graveyard,
            total_cost, None, false,
        )));
    }
    let Some(subject_start) = permission_shapes::find_words(&words, &["you", "may", "cast", "this"]) else {
        return Ok(None);
    };
    let head = &words[subject_start..];
    if !matches!(head.get(4), Some(&("card" | "creature" | "spell")))
        || !head.get(5..9).is_some_and(|tail| tail == ["from", "your", "graveyard", "by"])
    {
        return Ok(None);
    }
    let condition = if subject_start == 0 {
        None
    } else if words.starts_with(&["as", "long", "as"]) {
        let start = view.token_start_indices()[3];
        let end = view.token_start_indices()[subject_start];
        let condition_tokens = trim_edge_punctuation(&tokens[start..end]);
        Some(parse_this_spell_cost_condition(&condition_tokens).ok_or_else(||
            CardTextError::ParseError("unsupported intrinsic graveyard-cast condition".into()))?)
    } else {
        return Ok(None);
    };
    let Some(relative_rather) = permission_shapes::find_words(head, &["rather", "than"]) else {
        return Ok(None);
    };
    let rather = subject_start + relative_rather;
    let tail = &words[rather + 2..];
    let cost_tail_len = if matches!(tail, ["pay" | "paying", "its", "mana", "cost", ..]) {
        4
    } else if matches!(tail, ["pay" | "paying", "this", "cards" | "spells", "mana", "cost", ..]) {
        5
    } else {
        return Ok(None);
    };
    let after_cost_word = rather + 2 + cost_tail_len;
    let exile_rider = &words[after_cost_word..];
    let mut entry_counters = Vec::new();
    let exiles_after_resolution = if exile_rider.is_empty() {
        false
    } else if exile_rider == ["if", "you", "cast", "this", "card", "this", "way", "and", "it", "would", "be", "put", "into", "your", "graveyard", "exile", "it", "instead"] {
        true
    } else if exile_rider.starts_with(&["if", "you", "do", "it", "enters", "with"]) {
        let entry_start = view.token_start_indices()[after_cost_word + 4];
        let mut entry_tokens = crate::lexer::synthetic_word_tokens(["this", "creature"]);
        entry_tokens.extend_from_slice(&tokens[entry_start..]);
        let entries = crate::keyword_static::parse_enters_with_counters_line(&entry_tokens)?.ok_or_else(||
            CardTextError::ParseError("unsupported alternative-cost entry counter rider".into()))?;
        for ability in entries {
            let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } = ability.payload else {
                return Err(CardTextError::ParseError("alternative-cost entry rider must only place counters".into()));
            };
            let crate::effect::Value::Fixed(amount) = count.unhinted() else {
                return Err(CardTextError::ParseError("dynamic alternative-cost entry counter amount is unsupported".into()));
            };
            let amount = u32::try_from(*amount).map_err(|_| CardTextError::ParseError("negative entry counter count".into()))?;
            entry_counters.push((counter, amount));
        }
        false
    } else {
        return Err(CardTextError::ParseError("unsupported intrinsic graveyard-cast follow-up".into()));
    };
    let cost_start = view.token_start_indices()[subject_start + 8] + 1;
    let cost_end = view.token_start_indices()[rather];
    let mut cost_tokens = tokens[cost_start..cost_end].to_vec();
    // Preserve the typed payment parser and original source spans. Only
    // grammatical cost heads change tense, never arbitrary object words.
    for index in 0..cost_tokens.len() {
        if index == 0 || cost_tokens[index - 1].is_word("and") {
            for (gerund, verb) in [("paying", "pay"), ("sacrificing", "sacrifice"),
                ("exiling", "exile"), ("discarding", "discard"), ("returning", "return")] {
                if cost_tokens[index].is_word(gerund) { cost_tokens[index].replace_word(verb); }
            }
        }
    }
    let total_cost = parse_payment_clause_as_total_cost(&cost_tokens)?.ok_or_else(||
        CardTextError::ParseError("unsupported intrinsic graveyard alternative price".into()))?;
    Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
        "Parsed graveyard alternative cost", crate::zone::Zone::Graveyard,
        total_cost, condition, exiles_after_resolution,
    ).with_entry_counters(entry_counters)))
}

/// A literal alternative price optionally gated by an independently parsed
/// condition. The cost, condition, and identity remain separate data.
pub fn parse_self_bound_alternative_price(tokens: &[OwnedLexToken]) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if let Some(method) = parse_price_with_battlefield_instructions(tokens)? { return Ok(Some(method)); }
    let tokens = trim_edge_punctuation(tokens);
    let view = TokenWordView::new(&tokens);
    let words = view.word_refs();
    if !matches!(words.get(..6), Some(["you", "may", "cast", "this", "spell" | "card" | "creature", "for"])) {
        return Ok(None);
    }
    let start = view.token_start_indices()[5] + 1;
    let if_index = tokens[start..].iter().position(|token| token.is_word("if")).map(|index| index + start);
    let mut additional_payment = None;
    let condition = if let Some(index) = if_index {
        let tail = &tokens[index + 1..];
        if tail.first().is_some_and(|token| token.is_word("you"))
            && tail.get(1).is_some_and(|token| token.is_word("also"))
        {
            additional_payment = Some(crate::activation_and_restrictions::parse_compiler_activation_cost(&tail[2..])?);
            None
        } else {
            let Some(condition) = parse_this_spell_cost_condition(tail) else {
                return Ok(None);
            };
            Some(condition)
        }
    } else { None };
    let mut cost_tokens = &tokens[start..if_index.unwrap_or(tokens.len())];
    let cost_view = TokenWordView::new(cost_tokens);
    let cost_words = cost_view.word_refs();
    if cost_words.ends_with(&["rather", "than", "its", "mana", "cost"]) {
        cost_tokens = &cost_tokens[..cost_view.token_start_indices()[cost_words.len() - 5]];
    }
    let mut total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(cost_tokens)?;
    if let Some(additional) = additional_payment {
        total_cost = conjoin_total_costs(&total_cost, &additional);
    }
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost".into(), total_cost, condition, prototype_power_toughness: None,
    }))
}

fn conjoin_total_costs<C: Clone>(left: &ironsmith_core::TotalCost<C>, right: &ironsmith_core::TotalCost<C>) -> ironsmith_core::TotalCost<C> {
    use ironsmith_core::{TotalCost, TotalCostKind};
    match (left.kind(), right.kind()) {
        (TotalCostKind::OneOf(branches), _) => TotalCost::one_of(branches.iter().map(|branch| conjoin_total_costs(branch, right)).collect()),
        (_, TotalCostKind::OneOf(branches)) => TotalCost::one_of(branches.iter().map(|branch| conjoin_total_costs(left, branch)).collect()),
        (TotalCostKind::All(a), TotalCostKind::All(b)) => TotalCost::from_costs(a.iter().chain(b).cloned().collect()),
    }
}

pub fn parse_self_free_cast(tokens: &[OwnedLexToken]) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens).word_refs();
    if !exact_one_of(
        &words,
        &[
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
            ],
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ],
        ],
    ) {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub fn parse_flash_with_additional_cost(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens);
    if !permission_shapes::prefix_words(
        &words.word_refs(),
        &[
            "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash", "if",
            "you", "pay",
        ],
    ) {
        return None;
    }
    let cost_start = words.token_start_indices().get(13).copied()?;
    let parsed = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[cost_start..])?;
    let suffix = TokenWordView::new(&tokens[cost_start + parsed.consumed..]).word_refs();
    if !permission_shapes::exact_words(&suffix, &["more", "to", "cast", "it"]) {
        return None;
    }
    Some(AlternativeCastingMethod::flash_with_additional_cost(
        parsed.cost,
        ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(),
    ))
}

/// "You may cast this spell as though it had flash by tapping three untapped
/// creatures you control with flying in addition to paying its other costs."
/// "You may cast this spell as though it had flash if you behold a Dragon as
/// an additional cost to cast it."
/// The non-mana payment is an optional additional cost that grants flash
/// timing when announced (CR 601.2b, 601.2f), exactly like the mana-only
/// "if you pay {N} more" form.
pub fn parse_flash_timing_optional_payment(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::model::CompilerOptionalCost>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let view = TokenWordView::new(&tokens);
    let words = view.word_refs();
    let head = [
        "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash",
    ];
    if !permission_shapes::prefix_words(&words, &head) {
        return Ok(None);
    }
    let rest = &words[head.len()..];
    let (cost_word_start, suffix_len, gerund) = if rest.first() == Some(&"by")
        && rest.ends_with(&["in", "addition", "to", "paying", "its", "other", "costs"])
    {
        (head.len() + 1, 7, true)
    } else if rest.starts_with(&["if", "you"])
        && rest.ends_with(&["as", "an", "additional", "cost", "to", "cast", "it"])
        // The mana-only "if you pay {N} more" form keeps its own reader.
        && !rest.starts_with(&["if", "you", "pay"])
    {
        (head.len() + 2, 7, false)
    } else {
        return Ok(None);
    };
    let cost_word_end = words.len() - suffix_len;
    if cost_word_end <= cost_word_start {
        return Ok(None);
    }
    let starts = view.token_start_indices();
    let cost_start = starts[cost_word_start];
    let cost_end = starts[cost_word_end];
    let mut cost_tokens = tokens[cost_start..cost_end].to_vec();
    if gerund {
        for index in 0..cost_tokens.len() {
            if index == 0 || cost_tokens[index - 1].is_word("and") {
                for (gerund, verb) in [("tapping", "tap"), ("paying", "pay"), ("sacrificing", "sacrifice"),
                    ("exiling", "exile"), ("discarding", "discard"), ("returning", "return"),
                    ("beholding", "behold"), ("revealing", "reveal")] {
                    if cost_tokens[index].is_word(gerund) { cost_tokens[index].replace_word(verb); }
                }
            }
        }
    }
    let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(&cost_tokens)?;
    if cost.is_free() {
        return Ok(None);
    }
    Ok(Some(crate::model::CompilerOptionalCost::flash_timing(cost)))
}

pub fn parse_you_may_rather_than_spell_cost(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if permission_shapes::prefix_words(&words, &["you", "may", "cast", "this"])
        && permission_shapes::find_words(&words, &["from", "your", "graveyard", "by"]).is_some()
    {
        return Ok(None);
    }
    if !permission_shapes::prefix_words(&words, &["you", "may"]) {
        return Ok(None);
    }
    let Some(rather_word) = permission_shapes::find_words(&words, &["rather"]) else {
        return Ok(None);
    };
    let Some(rather_token) = word_view.token_start_indices().get(rather_word).copied() else {
        return Ok(None);
    };
    let rather_tail =
        TokenWordView::new(tokens.get(rather_token + 1..).unwrap_or_default()).word_refs();
    if !is_rather_than_spell_cost_tail(&rather_tail) {
        return Ok(None);
    }
    let cost_clause_end = last_cost_word_token(tokens.get(rather_token + 1..).unwrap_or_default())
        .map(|relative| rather_token + 1 + relative)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    let trailing_tokens = trim_edge_punctuation(&tokens[cost_clause_end + 1..]);
    let spending_rule = crate::consumer_mana::source_spending_rule(&trailing_tokens, true);
    // "You may pay {B} rather than pay this spell's mana cost if there are
    // thirteen or more creatures on the battlefield." (Blasphemous Edict)
    let trailing_condition = if trailing_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        let condition_tokens = trim_edge_punctuation(&trailing_tokens[1..]);
        Some(
            parse_this_spell_cost_condition(&condition_tokens)
                .or_else(|| parse_special_cost_condition(&condition_tokens))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported this-spell cost condition (clause: '{}')",
                        TokenWordView::new(&condition_tokens).word_refs().join(" ")
                    ))
                })?,
        )
    } else if spending_rule.is_some() {
        None
    } else if !TokenWordView::new(&trailing_tokens).word_refs().is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            TokenWordView::new(&trailing_tokens).word_refs().join(" ")
        )));
    } else {
        None
    };
    let cost_tokens = tokens.get(2..rather_token).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let total_cost = parse_payment_clause_as_total_cost(cost_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported alternative cost clause (line: '{}', cost: '{}')",
            line,
            render_token_slice(cost_tokens).trim()
        ))
    })?;
    let total_cost = if let Some(rule) = spending_rule {
        total_cost.try_map(|cost| -> Result<_, CardTextError> {
            Ok(match cost {
                crate::model::CompilerCost::Mana(mana) =>
                    crate::model::CompilerCost::Mana(mana.with_spending_restriction(rule.clone())),
                _ => return Err(CardTextError::ParseError("source-restricted alternative requires a mana cost".into())),
            })
        })?
    } else { total_cost };
    let method = AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost".into(),
        total_cost,
        condition: None,
        prototype_power_toughness: None,
    };
    Ok(Some(match trailing_condition {
        Some(condition) => normalize_trap_method(method.with_cast_condition(condition)),
        None => method,
    }))
}

pub fn parse_if_conditional_alternative_cost(
    tokens: &[OwnedLexToken],
    line_tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let line = render_token_slice(line_tokens);
    let line = line.as_str();
    let clause_words = TokenWordView::new(tokens).word_refs();
    // "As long as you've discarded a card this turn, you may pay {B/R} to
    // cast this spell." (Asmoranomardicadaistinaculdacar): an alternative
    // cost whose condition is checked as the spell is cast, the same as an
    // "if" condition (CR 118.9).
    let as_long_as = permission_shapes::prefix_words(&clause_words, &["as", "long", "as"]);
    if !as_long_as && !permission_shapes::prefix_words(&clause_words, &["if"]) {
        return Ok(None);
    }
    let condition_source: Vec<OwnedLexToken> = if as_long_as {
        let Some(start) = TokenWordView::new(tokens).token_start_indices().get(2).copied() else {
            return Ok(None);
        };
        tokens[start..].to_vec()
    } else {
        tokens.to_vec()
    };
    let Some((condition_tokens, tail_tokens)) = split_condition_and_cost_tail(&condition_source)
    else {
        return Ok(None);
    };
    // "If you control a Forest, rather than pay this spell's mana cost, you
    // may have an opponent gain 3 life." (Invigorate): the same alternative
    // cost with the "rather than" clause leading.
    let reordered_tail = leading_rather_than_tail(tail_tokens);
    let tail_tokens = reordered_tail.as_deref().unwrap_or(tail_tokens);
    // "you may pay {B/R} to cast this spell": the payment replaces the
    // spell's mana cost.
    let to_cast_tail = {
        let view = TokenWordView::new(tail_tokens);
        let words = view.word_refs();
        let suffix = ["to", "cast", "this", "spell"];
        if permission_shapes::prefix_words(&words, &["you", "may", "pay"])
            && words.len() > suffix.len() + 3
            && words.ends_with(&suffix)
            && let Some(to_token) = view.token_start_indices().get(words.len() - suffix.len()).copied()
        {
            let mut rewritten = tail_tokens[..to_token].to_vec();
            rewritten.extend(crate::lexer::synthetic_word_tokens([
                "rather", "than", "pay", "this", "spell", "mana", "cost",
            ]));
            Some(rewritten)
        } else {
            None
        }
    };
    let tail_tokens = to_cast_tail.as_deref().unwrap_or(tail_tokens);
    // "If this spell is ..., you may cast it without paying its mana cost":
    // `it` repeats the condition's own subject, this spell.
    let condition_names_this_spell = permission_shapes::prefix_words(
        &TokenWordView::new(condition_tokens).word_refs(),
        &["this", "spell"],
    );
    let self_free_cast = parse_self_free_cast(tail_tokens).is_some()
        || (condition_names_this_spell
            && TokenWordView::new(tail_tokens).word_refs()
                == [
                    "you", "may", "cast", "it", "without", "paying", "its", "mana", "cost",
                ]);
    // "If ..., you may cast this creature without paying its mana cost and as
    // though it had flash" (Qasali Ambusher): the free method carries its own
    // instant-timing permission.
    let free_cast_with_flash = matches!(
        TokenWordView::new(tail_tokens).word_refs().as_slice(),
        [
            "you", "may", "cast", "this", "spell" | "creature" | "card" | "permanent",
            "without", "paying", "its", "mana", "cost", "and", "as", "though", "it", "had",
            "flash",
        ]
    );
    if !self_free_cast
        && !free_cast_with_flash
        && parse_you_may_rather_than_spell_cost(tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }

    let condition = if let Some(condition) = parse_this_spell_cost_condition(condition_tokens) {
        condition
    } else {
        parse_special_cost_condition(condition_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    if free_cast_with_flash {
        return Ok(Some(AlternativeCastingMethod::alternative_cost_with_condition(
            ironsmith_core::FREE_CAST_AS_THOUGH_FLASH_METHOD,
            None,
            Vec::new(),
            condition,
        )));
    }
    if self_free_cast {
        let method = AlternativeCastingMethod::alternative_cost_with_condition(
            "Parsed alternative cost",
            None,
            Vec::new(),
            condition,
        );
        return Ok(Some(normalize_trap_method(method)));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost(tail_tokens, line)? else {
        return Ok(None);
    };
    // "Freerunning—Return a blue creature you control to its owner's hand"
    // (Escape Detection): the freerunning cost may be entirely non-mana.
    if permission_shapes::prefix_tokens(line_tokens, &["freerunning"]) {
        let cost = method.mana_cost().cloned();
        return Ok(Some(
            AlternativeCastingMethod::alternative_cost_with_condition(
                "Freerunning",
                cost,
                method.non_mana_costs(),
                condition,
            ),
        ));
    }
    Ok(Some(normalize_trap_method(
        method.with_cast_condition(condition),
    )))
}

/// Reorder "rather than pay this spell's mana cost, you may <cost>" into
/// "you may <cost> rather than pay this spell's mana cost".
fn leading_rather_than_tail(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    if !tokens.first().is_some_and(|token| token.is_word("rather")) {
        return None;
    }
    let comma = first_comma(tokens)?;
    let rather_clause = trim_commas(&tokens[..comma]);
    if !is_rather_than_spell_cost_tail(
        &TokenWordView::new(tokens.get(1..comma).unwrap_or_default()).word_refs(),
    ) {
        return None;
    }
    let permission = trim_edge_punctuation(trim_commas(tokens.get(comma + 1..)?));
    if !permission_shapes::prefix_words(&TokenWordView::new(&permission).word_refs(), &["you", "may"])
    {
        return None;
    }
    let mut reordered = permission.to_vec();
    reordered.extend(rather_clause.iter().cloned());
    Some(reordered)
}

fn split_condition_and_cost_tail(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    if let Some(comma) = first_comma(tokens) {
        return Some((
            trim_commas(&tokens[1..comma]),
            trim_commas(tokens.get(comma + 1..).unwrap_or_default()),
        ));
    }
    let view = TokenWordView::new(tokens);
    let may_word = permission_shapes::find_words(&view.word_refs(), &["you", "may", "pay"])?;
    let may_token = view.token_start_indices().get(may_word).copied()?;
    Some((
        trim_commas(&tokens[1..may_token]),
        trim_commas(&tokens[may_token..]),
    ))
}

fn parse_special_cost_condition(tokens: &[OwnedLexToken]) -> Option<ThisSpellCostCondition> {
    let words = TokenWordView::new(tokens).word_refs();
    if exact_one_of(
        &words,
        &[
            &[
                "this", "spell", "is", "the", "first", "spell", "youve", "cast", "this", "game",
            ],
            &[
                "this", "spell", "is", "the", "first", "spell", "you've", "cast", "this", "game",
            ],
            &[
                "this", "spell", "is", "the", "first", "spell", "you", "have", "cast", "this",
                "game",
            ],
            &[
                "this", "spell", "is", "first", "spell", "youve", "cast", "this", "game",
            ],
            &[
                "this", "spell", "is", "first", "spell", "you've", "cast", "this", "game",
            ],
        ],
    ) {
        return Some(ThisSpellCostCondition::FirstSpellYouCastThisGame);
    }
    if permission_shapes::prefix_words(
        &words,
        &[
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ],
    ) {
        return Some(
            ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                crate::types::Subtype::Assassin,
            ),
        );
    }
    let count_start =
        if permission_shapes::prefix_words(&words, &["youve", "been", "dealt", "damage", "by"]) {
            5
        } else if permission_shapes::prefix_words(
            &words,
            &["you", "have", "been", "dealt", "damage", "by"],
        ) {
            6
        } else {
            return None;
        };
    if !permission_shapes::suffix_words(&words, &["creatures", "this", "turn"]) {
        return None;
    }
    let count_token = TokenWordView::new(tokens)
        .token_start_indices()
        .get(count_start)
        .copied()?;
    let (count, _) = leaf::parse_leaf_number_prefix_tokens(&tokens[count_token..])?.into_fixed()?;
    Some(ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(count))
}

fn normalize_trap_method(method: AlternativeCastingMethod) -> AlternativeCastingMethod {
    let trap = method
        .cast_condition()
        .and_then(trap_condition_from_this_spell_cost_condition)
        .zip(simple_trap_cost_from_alternative_method(&method));
    if let Some((condition, cost)) = trap {
        AlternativeCastingMethod::trap("Trap", cost, condition)
    } else {
        method
    }
}

fn trap_condition_from_this_spell_cost_condition(
    condition: &ThisSpellCostCondition,
) -> Option<crate::TrapCondition> {
    match condition {
        ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(count) => {
            Some(crate::TrapCondition::OpponentCastSpells { count: *count })
        }
        // The runtime trap condition carries no count; only the printed
        // "two or more creatures" shape (Inferno Trap) renders back exactly.
        ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(2) => {
            Some(crate::TrapCondition::CreatureDealtDamageToYou)
        }
        _ => None,
    }
}

fn simple_trap_cost_from_alternative_method(method: &AlternativeCastingMethod) -> Option<ManaCost> {
    let AlternativeCastingMethod::Composed { total_cost, .. } = method else {
        return None;
    };
    if total_cost.non_mana_costs().next().is_some() {
        return None;
    }
    Some(
        total_cost
            .mana_cost()
            .cloned()
            .unwrap_or_else(ManaCost::new),
    )
}

fn is_rather_than_spell_cost_tail(words: &[&str]) -> bool {
    permission_shapes::prefix_words(words, &["than", "pay", "this"])
        && permission_shapes::find_words(words, &["mana", "cost"]).is_some()
        && ["spell", "spells"]
            .iter()
            .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

fn last_cost_word_token(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if tokens[idx].as_word().is_some_and(|word| {
            permission_shapes::exact_words(&[word], &["cost"])
                || permission_shapes::exact_words(&[word], &["costs"])
        }) {
            return Some(idx);
        }
    }
    None
}

fn first_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = 0;
    while idx < tokens.len() {
        if tokens[idx].is_comma() {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn trim_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_free_cast_surfaces() {
        let tokens = lex_line("you may cast this spell without paying its mana cost", 0)
            .expect("lex fixture");
        assert!(parse_self_free_cast(&tokens).is_some());
    }
}

#[cfg(test)]
mod intrinsic_zone_alternative_tests {
    use super::*;
    fn parse(text: &str) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        let mut tokens = lex_line(text, 0).unwrap();
        for token in &mut tokens { token.lowercase_word(); }
        parse_self_zone_alternative_cost(&tokens)
    }
    #[test]
    fn intrinsic_zone_reader_preserves_price_condition_and_exile_rider() {
        for text in [
            "You may cast this card from your graveyard by paying {2}{W} rather than paying its mana cost.",
            "You may cast this creature from your graveyard by paying {B}{B} and sacrificing two creatures rather than paying its mana cost.",
            "You may cast this card from your graveyard by paying {3}{R} and exiling four other cards from your graveyard rather than paying its mana cost.",
        ] {
            let method = parse(text).unwrap().unwrap();
            assert_eq!(method.cast_from_zone(), crate::zone::Zone::Graveyard);
            assert!(!method.exiles_after_resolution());
            assert!(method.total_cost().is_some());
        }
        let method = parse("As long as you control a Giant, you may cast this card from your graveyard by paying {U} rather than paying its mana cost. If you cast this card this way and it would be put into your graveyard, exile it instead.").unwrap().unwrap();
        assert!(method.cast_condition().is_some());
        assert!(method.exiles_after_resolution());
    }
    #[test]
    fn intrinsic_zone_reader_rejects_unknown_riders_and_does_not_claim_static_price_grants() {
        let entry = parse("You may cast this card from your graveyard by paying {3}{R} rather than paying its mana cost. If you do, it enters with two +1/+1 counters on it.").unwrap().unwrap();
        assert_eq!(entry.entry_counters(), &[(ironsmith_core::CounterType::PlusOnePlusOne, 2)]);
        assert!(parse("You may cast this card from your graveyard by paying {3}{R} rather than paying its mana cost. If you do, it enters with two +1/+1 counters on it and you draw a card.").is_err());
        for text in [
            "You may pay {0} rather than pay the mana cost for spells you cast.",
            "You may cast spells from your graveyard by paying {2} rather than paying their mana costs.",
            "You may cast this card from your graveyard by paying 2 life in addition to paying its other costs.",
        ] { assert!(parse(text).unwrap().is_none(), "{text}"); }
    }
}

/// The price owns the complete conditional battlefield instructions. Keeping
/// them on the chosen casting method also preserves spell-copy behavior.
fn parse_price_with_battlefield_instructions(tokens: &[OwnedLexToken]) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [first, second] = sentences.as_slice() else { return Ok(None); };
    let first = trim_edge_punctuation(first);
    let view = TokenWordView::new(&first);
    let words = view.word_refs();
    let tail = TokenWordView::new(second).word_refs();
    let mut instructions = ironsmith_core::ZoneCastInstructions::default();
    let cost_end;
    if words.get(..6) == Some(&["you", "may", "cast", "this", "spell", "for"])
        && matches!(tail.as_slice(), ["if", "you", "do", "it", "gains", "haste", "and", "it's" | "its", "returned", "from", "the", "battlefield", "to", "its", "owners" | "owner's", "hand", "at", "the", "beginning", "of", "the", "next", "end", "step"]) {
        instructions.haste_on_battlefield = true;
        instructions.return_at_next_end_step = true;
        cost_end = first.len();
    } else if words.get(..6) == Some(&["you", "may", "cast", "this", "spell", "for"])
        && matches!(tail.as_slice(), ["if", "you", "do", "it's" | "its", "sacrificed", "when", "it", "enters"]) {
        instructions.sacrifice_on_entry = true;
        cost_end = first.len();
    } else if words.get(..6) == Some(&["if", "you", "cast", "this", "spell", "for"])
        && tail == ["sacrifice", "it", "at", "the", "beginning", "of", "the", "next", "end", "step"] {
        let Some(grant_start) = words.windows(2).position(|part| part == ["it", "gains"]) else { return Ok(None); };
        let grant_start = view.token_start_indices()[grant_start];
        let Some(comma) = first[..grant_start].iter().rposition(|token| token.kind == crate::lexer::TokenKind::Comma) else { return Ok(None); };
        let grant = TokenWordView::new(&first[comma + 1..]).word_refs();
        if grant != ["it", "gains", "haste", "and", "when", "this", "creature", "dies", "draw", "a", "card"] { return Ok(None); }
        instructions.haste_on_battlefield = true;
        instructions.draw_on_death = true;
        instructions.sacrifice_at_next_end_step = true;
        cost_end = comma;
    } else { return Ok(None); }
    let start = view.token_start_indices()[5] + 1;
    let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[start..cost_end])?;
    Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
        "Parsed conditional battlefield price", crate::zone::Zone::Hand, total_cost, None, false,
    ).with_zone_cast_instructions(instructions)))
}

/// Complete prose describing a casting-time text substitution, before targets
/// or modes are chosen. A generic price uses the same document rewrite owner.
pub(crate) fn described_spell_text_substitution(tokens: &[OwnedLexToken]) -> Option<(bool, Vec<OwnedLexToken>)> {
    let text = crate::lexer::render_token_slice(tokens).to_ascii_lowercase().replace(". \"", ".\"");

    let rest = text.trim_end_matches('.').strip_prefix("you may cast this spell for ")?;
    let (price, consequence) = rest.split_once(". if you do, ")?;
    let cleave = match consequence {
        "remove the words in square brackets" => true,
        "change its text by replacing all instances of \"target\" with \"each.\"" | "change \"target\" in its text to \"each.\"" => false,
        _ => return None,
    };
    Some((cleave, crate::lexer::lex_line(price, 0).ok()?))
}
