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
    // "by waterbending {X}" (CR 702.185): a generic or X price with the
    // waterbend obligation (artifacts and creatures may be tapped for it).
    if matches!(*head, "waterbending" | "waterbend") {
        let mut payment = crate::lexer::synthetic_word_tokens(["pay"]);
        payment.extend_from_slice(&tokens[1..]);
        let Some(cost) = crate::activation_and_restrictions::keyword_action_costs::parse_payment_clause_as_total_cost(&payment)? else {
            return Ok(None);
        };
        let Some([crate::model::CompilerCost::Mana(mana)]) = cost.as_all() else { return Ok(None); };
        if mana.pips().is_empty() || mana.pips().iter().any(|pip| !matches!(pip.as_slice(),
            [crate::mana::ManaSymbol::Generic(_) | crate::mana::ManaSymbol::X]))
        { return Ok(None); }
        return Ok(Some(ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Mana(
            mana.clone().with_waterbend()))));
    }
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

/// "For as long as you control Kotose, you may play one of the exiled cards,
/// and you may spend mana as though it were mana of any color to cast it."
/// The leading duration bounds a lasting play permission (the same grant as
/// the trailing "for as long as you control this creature" lifetime), not a
/// one-shot play now. Reads only an unpriced, unconditional tagged permission.
pub(crate) fn parse_control_source_leading_permission(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    const HEAD: [&str; 6] = ["for", "as", "long", "as", "you", "control"];
    if tokens.len() <= HEAD.len()
        || !tokens.iter().zip(HEAD).all(|(token, word)| token.is_word(word))
    {
        return Ok(None);
    }
    let rest = &tokens[HEAD.len()..];
    let Some(comma) = rest.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let source = trim_lexed_commas(&rest[..comma]);
    if source.is_empty()
        || !(crate::util::is_source_reference_words(&token_word_refs(source))
            || crate::lexer::is_authored_proper_name_phrase(source))
    {
        return Ok(None);
    }
    let body = trim_lexed_commas(&rest[comma + 1..]);
    let words = token_word_refs(body);
    if !(words.starts_with(&["you", "may", "play"]) || words.starts_with(&["you", "may", "cast"]))
        || words
            .iter()
            .any(|word| matches!(*word, "rather" | "by" | "during" | "if" | "without"))
    {
        return Ok(None);
    }
    let duration = crate::lexer::synthetic_word_tokens([
        "for", "as", "long", "as", "you", "control", "this", "creature",
    ]);
    let mut rewritten = Vec::with_capacity(body.len() + duration.len() + 1);
    if let Some(rider) = super::strip_allow_any_color_for_cast_suffix_tokens(body) {
        let split = rider.body_tokens.len();
        rewritten.extend_from_slice(trim_lexed_commas(&body[..split]));
        rewritten.extend(duration);
        rewritten.push(OwnedLexToken::comma(crate::cards::TextSpan::synthetic()));
        rewritten.extend_from_slice(&body[split..]);
    } else {
        rewritten.extend_from_slice(body);
        rewritten.extend(duration);
    }
    let Some(effect) = super::parse_cast_or_play_tagged_clause(&rewritten)? else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(subject) = &effect else {
        return Ok(None);
    };
    if !matches!(
        subject.action,
        SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. })
    ) {
        return Ok(None);
    }
    Ok(Some(effect))
}

/// "For as long as you control <this source>, you may cast <the exiled card>
/// [during your turn] by <price> rather than paying its mana cost[, where X
/// is its mana value]." (Hama, the Bloodbender): a lasting priced permission
/// bounded by continued control of the source, optionally usable only during
/// the permission player's turns, whose X is fixed by the granted card.
pub(crate) fn parse_control_source_priced_permission(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    const HEAD: [&str; 6] = ["for", "as", "long", "as", "you", "control"];
    if tokens.len() <= HEAD.len()
        || !tokens.iter().zip(HEAD).all(|(token, word)| token.is_word(word))
    {
        return Ok(None);
    }
    let rest = &tokens[HEAD.len()..];
    let Some(may) = rest.windows(3).position(|window| {
        window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("cast")
    }) else {
        return Ok(None);
    };
    let source = trim_lexed_commas(&rest[..may]);
    if source.is_empty()
        || !(crate::util::is_source_reference_words(&token_word_refs(source))
            || crate::lexer::is_authored_proper_name_phrase(source))
    {
        return Ok(None);
    }
    let mut body = trim_lexed_commas(&rest[may + 3..]);
    let mut x_is_mana_value = false;
    if let Some(where_index) = body.iter().rposition(|token| token.is_word("where"))
        && token_word_refs(&body[where_index..]) == ["where", "x", "is", "its", "mana", "value"]
    {
        body = trim_lexed_commas(&body[..where_index]);
        x_is_mana_value = true;
    }
    let Some((granted, price)) = replacement_price_suffix(body)? else {
        return Ok(None);
    };
    if x_is_mana_value
        && !price.as_all().is_some_and(|costs| costs.iter().any(|cost| matches!(cost,
            crate::model::CompilerCost::Mana(mana) if mana.has_x())))
    {
        return Ok(None);
    }
    let mut granted = trim_lexed_commas(granted);
    let mut during_your_turns = false;
    if granted.len() > 3
        && token_word_refs(&granted[granted.len() - 3..]) == ["during", "your", "turn"]
    {
        granted = trim_lexed_commas(&granted[..granted.len() - 3]);
        during_your_turns = true;
    }
    let Some((target, tail)) = super::parse_tagged_cast_or_play_target_tokens(granted) else {
        return Ok(None);
    };
    if !trim_lexed_commas(tail).is_empty() || target.as_copy || target.max_plays.is_some() {
        return Ok(None);
    }
    let mut surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_control_source(ironsmith_core::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string(),
        ));
    if let Some(object) = target
        .surface
        .clone()
        .or_else(|| super::tagged_permission_target_surface(granted))
    {
        surface = surface.with_object(object);
    }
    let mut effect = EffectAst::subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
        crate::tag::TagRef::of(target.tag),
        PlayerAst::Implicit,
        false,
        ironsmith_core::value_model::ManaSpendMode::Normal,
        Some(surface),
    );
    if let EffectAst::SubjectVerb(subject) = &mut effect
        && let SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
            during_your_turns: during,
            alternative_cost,
            alternative_cost_x_is_mana_value,
            ..
        }) = &mut subject.action
    {
        *during = during_your_turns;
        *alternative_cost = Some(price);
        *alternative_cost_x_is_mana_value = x_is_mana_value;
    }
    Ok(Some(effect))
}
