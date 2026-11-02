//! Independent preparation, zone permission and copiable-characteristic clauses.
use super::*;

fn preparation(sentence: &str) -> Result<Option<ironsmith_core::alternative_cast_model::ExilePreparation>, CardTextError> {
    let Some(price) = sentence.strip_prefix("as a sorcery, you may pay ")
        .and_then(|tail| tail.strip_suffix(" and exile this card from your hand face down")) else { return Ok(None); };
    let price = crate::grammar::values::parse_mana_cost_tokens(&lex_line(price, 0)?)?;
    Ok(Some(ironsmith_core::alternative_cast_model::ExilePreparation::FaceDownSorcery { price }))
}

fn permission(sentence: &str) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let Some(price) = sentence.strip_prefix("cast it on a later turn for ") else { return Ok(None); };
    Ok(Some(crate::activation_and_restrictions::parse_compiler_activation_cost(&lex_line(price, 0)?)?))
}

fn characteristics(sentence: &str) -> Result<Option<(ManaCost, Option<ironsmith_core::PowerToughness>)>, CardTextError> {
    let Some(rest) = sentence.strip_prefix("when cast this way, this spell and the permanent it becomes have mana cost ") else { return Ok(None); };
    let Some((price, tail)) = rest.split_once(", colors determined by that mana cost") else { return Ok(None); };
    let pt = if tail.is_empty() { None } else {
        let Some(pt) = tail.strip_prefix(", and base power and toughness ") else { return Ok(None); };
        let Some((power,toughness)) = pt.split_once('/') else { return Ok(None); };
        let (Ok(power),Ok(toughness)) = (power.parse(),toughness.parse()) else { return Ok(None); };
        Some(ironsmith_core::PowerToughness::fixed(power,toughness))
    };
    Ok(Some((crate::grammar::values::parse_mana_cost_tokens(&lex_line(price, 0)?)?,pt)))
}

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let mut prepare = None; let mut cost = None; let mut characteristic = None;
    let mut retained = false; let mut copiable = false; let mut resolved_creature = None;
    for sentence in sentences {
        let text = render_token_slice(&trim_edge_punctuation(sentence)).to_ascii_lowercase();
        if let Some(value) = preparation(&text)? {
            if prepare.replace(value).is_some() { return Ok(None); }
        } else if let Some(value) = permission(&text)? {
            if cost.replace(value).is_some() { return Ok(None); }
        } else if let Some(value) = characteristics(&text)? {
            if characteristic.replace(value).is_some() { return Ok(None); }
        } else if let Some(types) = text.strip_prefix("when cast this way, after resolving this spell enters the battlefield as a ")
            .and_then(|tail| tail.strip_suffix(" creature")) {
            let Some(subtypes) = types.split_whitespace().map(crate::util::parse_subtype_word).collect::<Option<Vec<_>>>() else { return Ok(None); };
            if subtypes.is_empty() || subtypes.iter().any(|subtype| !subtype.is_creature_type()) || resolved_creature.replace(subtypes).is_some() { return Ok(None); }
        } else if text == "it keeps its other characteristics" && !retained { retained = true;
        } else if text == "these are copiable characteristics" && !copiable { copiable = true;
        } else { return Ok(None); }
    }
    let (Some(prepare),Some(cost),Some((mana,pt))) = (prepare,cost,characteristic) else { return Ok(None); };
    if !retained || !copiable { return Ok(None); }
    crate::word_claims::claim_tokens(tokens);
    Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
        "Parsed prepared casting characteristics", crate::zone::Zone::Exile, cost, None, false,
    ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
        exile_preparation: Some(prepare), casting_mana_cost: Some(mana), casting_power_toughness: pt, resolves_as_creature: resolved_creature,
        ..Default::default()
    })))
}
