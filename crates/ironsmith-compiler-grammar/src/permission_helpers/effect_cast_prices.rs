//! A price belongs to the cast it authorizes, rather than a later payment.
use super::*;

pub(crate) fn mana_value_life_price() -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Life(
        Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source))))
}

/// Exact payment tail, excluding the outer "by" and replacement wording.
fn price(tokens: &[OwnedLexToken]) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if matches!(words.as_slice(),
        ["paying" | "pay", "life", "equal", "to", "its", "mana", "value"]
        | ["paying" | "pay", "life", "equal", "to", "the", "spells", "mana", "value"]
        | ["paying" | "pay", "life", "equal", "to", "that", "spells", "mana", "value"])
    { return Ok(Some(mana_value_life_price())); }
    let Some(head) = words.first() else { return Ok(None); };
    let normalized = match *head {
        "paying" => "pay",
        "discarding" => "discard",
        "sacrificing" => "sacrifice",
        "exiling" => "exile",
        "pay" | "discard" | "sacrifice" | "exile" => *head,
        _ => return Ok(None),
    };
    let mut payment = crate::lexer::synthetic_word_tokens([normalized]);
    payment.extend_from_slice(&tokens[1..]);
    crate::activation_and_restrictions::keyword_action_costs::parse_payment_clause_as_total_cost(&payment)
}

pub(crate) fn replacement_price_suffix(tokens: &[OwnedLexToken]) -> Result<Option<(&[OwnedLexToken], ironsmith_core::TotalCost<crate::model::CompilerCost>)>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(by) = tokens.iter().position(|token| token.is_word("by")) else { return Ok(None); };
    let tail = &tokens[by + 1..];
    let Some(replacement) = tail.windows(2).position(|pair|
        (pair[0].is_word("rather") && pair[1].is_word("than"))
        || (pair[0].is_word("instead") && pair[1].is_word("of"))) else { return Ok(None); };
    let ending = crate::lexer::parser_token_word_refs(&tail[replacement + 2..]);
    if !matches!(ending.as_slice(), ["paying" | "pay", "its", "mana", "cost"] | ["its", "mana", "cost"]) {
        return Ok(None);
    }
    Ok(price(&tail[..replacement])?.map(|price| (&tokens[..by], price)))
}

pub(crate) fn attach_price(effect: &mut EffectAst, price: &ironsmith_core::TotalCost<crate::model::CompilerCost>) -> usize {
    if let EffectAst::SubjectVerb(subject) = effect {
        if let SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { spec, player, duration }) = &subject.action
            && matches!(spec.grantable, crate::model::CompilerGrantableCore::PlayFrom)
            && spec.max_plays.is_none() && spec.usage_limit.is_none()
            && spec.additional_zones.is_empty() && spec.on_use_effects.is_empty() && price.as_all().is_some()
        {
            let mut land = (**spec).clone();
            land.filtered_zone_surface = None;
            land.filter.all_card_types.push(CardType::Land);
            let mut spell = (**spec).clone();
            spell.filtered_zone_surface = None;
            spell.filter.excluded_card_types.push(CardType::Land);
            spell.grantable = crate::model::CompilerGrantableCore::AlternativeCast(
                ironsmith_core::AlternativeCastingMethod::<EffectAst, crate::model::CompilerCost, ironsmith_core::ThisSpellCostCondition>::cast_from_zone_with_total_cost(
                    "Effect casting price", spec.zone, price.clone(), None, false));
            let make = |spec| EffectAst::subject_verb(crate::cards::builders::SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit, SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec {
                    spec: Box::new(spec), player: *player, duration: *duration,
                }));
            *effect = EffectAst::Sequence { effects: vec![make(land), make(spell)] };
            return 1;
        }
        let slot = match &mut subject.action {
            SubjectVerbActionAst::Stack(crate::cards::builders::StackActionAst::CastTagged {
                alternative_cost, alternative_payment: None, without_paying_mana_cost: false, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                alternative_cost, without_paying_mana_cost: false, .. }) => Some(alternative_cost),
            _ => None,
        };
        if let Some(slot) = slot {
            if slot.is_some() { return 0; }
            *slot = Some(price.clone());
            return 1;
        }
    }
    let mut count = 0;
    crate::model::visit::for_each_nested_effects_mut(effect, false, |effects| {
        for child in effects { count += attach_price(child, price); }
    });
    count
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some((body, cost)) = replacement_price_suffix(tokens)? else { return Ok(None); };
    let Some(mut effect) = super::parse_cast_or_play_tagged_clause(body)? else { return Ok(None); };
    if attach_price(&mut effect, &cost) != 1 {
        return Err(CardTextError::ParseError("an alternative casting price needs one exact cast instruction".into()));
    }
    Ok(Some(effect))
}
