//! Independent timing, price, use-limit and effect readers for a static action
//! permission. No activated ability is synthesized.
use super::*;
use ironsmith_core::static_ability_model::ImmediateActionTiming;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let mut sentences = crate::lexer::split_lexed_sentences(tokens);
    let Some(first) = sentences.first().copied() else { return Ok(None); };
    let first = crate::util::trim_edge_punctuation_tokens(first);
    let mut label = "Paid special action".to_owned();
    let first = if first.first().is_some_and(|token| token.is_word("special"))
        && first.get(1).is_some_and(|token| token.is_word("action"))
        && first.get(2).is_some_and(|token| token.is_quote())
    {
        let Some(close) = first[3..].iter().position(|token| token.is_quote()).map(|index| index + 3)
        else { return Ok(None); };
        if close == 3 { return Ok(None); }
        label = crate::lexer::render_token_slice(&first[3..close]);
        let start = first[close + 1..].iter().position(|token| token.as_word().is_some())
            .map(|index| index + close + 1).unwrap_or(first.len());
        crate::word_claims::claim_tokens(&first[..start]);
        &first[start..]
    } else { first };
    let view = crate::lexer::TokenWordView::new(first);
    let words = view.word_refs();
    let Some(may) = words.windows(2).position(|part| part == ["may", "pay"])
    else { return Ok(None); };
    let pay = may + 1;
    let (actor_start, actor) = if may > 0 && words[may - 1] == "you" { (may - 1, PlayerFilter::You) }
        else if may >= 2 && matches!(&words[may - 2..may], ["any" | "a", "player"]) { (may - 2, PlayerFilter::Any) }
        else { return Ok(None); };
    let timing = match &words[..actor_start] {
        ["any", "time", "you", "have", "priority"] => ImmediateActionTiming::Priority,
        ["any", "time", "you", "could", "cast", "a", "sorcery"] => ImmediateActionTiming::Sorcery,
        ["any", "time", "you", "have", "priority", "and", "this", "permanent", "is", "attacking"] => ImmediateActionTiming::Attacking,
        _ => return Ok(None),
    };
    let action_start = words.windows(5).rposition(|tail| tail == ["to", "take", "a", "special", "action"]);
    // Cost-template binding may render the same instruction as two sentences:
    // "you may pay PRICE. If you do, take a special action." Recognize that
    // complete structural pair; an unrelated optional payment is not an action.
    let separated_action = action_start.is_none() && sentences.get(1).is_some_and(|sentence| {
        crate::lexer::token_word_refs(sentence) == ["if", "you", "do", "take", "a", "special", "action"]
    });
    let cost_end = if let Some(action_start) = action_start {
        if action_start <= pay || action_start + 5 != words.len() { return Ok(None); }
        view.token_start_indices()[action_start]
    } else if separated_action { first.len() } else { return Ok(None); };
    let cost_start = view.token_start_indices()[pay] + 1;
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&first[cost_start..cost_end])?;
    sentences.remove(0);
    if separated_action {
        crate::word_claims::claim_tokens(sentences[0]);
        sentences.remove(0);
    }
    let mut once = false;
    let mut condition = None;
    let mut shared_turn_group = None;
    let mut source_zone = Zone::Battlefield;
    loop {
        let Some(last) = sentences.last().copied() else { return Ok(None); };
        let last = crate::util::trim_edge_punctuation_tokens(last);
        let view = crate::lexer::TokenWordView::new(last);
        let words = view.word_refs();
        if words == ["take", "this", "action", "only", "once"] {
            if once { return Ok(None); }
            once = true;
        } else if words.starts_with(&["take", "this", "action", "only", "if"]) {
            if condition.is_some() || words.len() <= 5 { return Ok(None); }
            condition = Some(parse_static_condition_clause(&last[view.token_start_indices()[5]..])?);
        } else if words.starts_with(&["take", "this", "action", "only", "while", "this", "card", "is", "in", "your"]) && words.len() == 11 {
            let Some(zone) = crate::util::parse_zone_word(words[10]) else { return Ok(None); };
            source_zone = zone;
        } else {
            let prose = crate::lexer::render_token_slice(last);
            let group = prose.strip_prefix("actions of this permanent in group ")
                .and_then(|tail| tail.strip_suffix(" share one use each turn"))
                .and_then(|label| label.strip_prefix('"').and_then(|label| label.strip_suffix('"')));
            let Some(group) = group else { break; };
            if shared_turn_group.is_some() || group.is_empty() { return Ok(None); }
            shared_turn_group = Some(group.to_owned());
        }
        crate::word_claims::claim_tokens(last);
        sentences.pop();
    }
    let Some(body_head) = sentences.first().copied() else { return Ok(None); };
    let body_view = crate::lexer::TokenWordView::new(body_head);
    let body_words = body_view.word_refs();
    if !matches!(body_words.get(..3), Some(["if", "you" | "they", "do"])) || body_words.len() <= 3 { return Ok(None); }
    let body_start = body_view.token_start_indices()[3];
    let mut body = body_head[body_start..].to_vec();
    body.push(OwnedLexToken::period(crate::cards::TextSpan::synthetic()));
    for sentence in &sentences[1..] {
        body.extend_from_slice(sentence);
        body.push(OwnedLexToken::period(crate::cards::TextSpan::synthetic()));
    }
    let effects = crate::effect_sentences::parse_effect_sentences_lexed(&body)?;
    crate::word_claims::claim_tokens(first);
    crate::word_claims::claim_tokens(&body_head[..body_start]);
    let ability = crate::model::CompilerStaticAbilityCore {
        id: Some(ironsmith_core::StaticAbilityId::IntrinsicImmediateAction),
        label,
        payload: ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction {
            cost, timing, source_zone, effects, once,
            player: actor, condition, shared_turn_group,
        },
    };
    Ok(Some(vec![StaticAbilityAst::Static(ability)]))
}

/// Recipient filtering and quoted price recognition remain independent. Only
/// a hand-zone alternative price is granted; this never permits casting a
/// battlefield permanent merely because it has a price ability.
pub(super) fn parse_casting_grant(tokens: &[OwnedLexToken]) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(open) = tokens.iter().position(|token| token.is_quote()) else { return Ok(None); };
    let Some(close) = tokens[open + 1..].iter().position(|token| token.is_quote()).map(|index| open + 1 + index)
    else { return Ok(None); };
    if tokens[close + 1..].iter().any(|token| token.as_word().is_some() || token.is_quote()) { return Ok(None); }
    let head = crate::lexer::TokenWordView::new(&tokens[..open]);
    let words = head.word_refs();
    if words.len() <= 5 || !matches!(&words[words.len()-5..], ["has" | "have", "the", "alternative", "casting", "cost"]) { return Ok(None); }
    let subject_end = head.token_start_indices()[words.len() - 5];
    let mut filter = parse_object_filter(&tokens[..subject_end], false)?;
    if filter.zone != Some(Zone::Hand) { return Ok(None); }
    filter.zone = None;
    let Some(method) = crate::grammar::shared_util::alternative_cost_lines::parse_self_zone_alternative_cost(&tokens[open + 1..close])?
    else { return Ok(None); };
    if method.cast_from_zone() != Zone::Hand { return Ok(None); }
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativeCast(method), filter, Zone::Hand,
    );
    crate::word_claims::claim_tokens(&tokens[subject_end..open + 1]);
    crate::word_claims::claim_tokens(&tokens[close..]);
    Ok(Some(vec![StaticAbilityAst::Static(StaticAbility::grants(spec))]))
}
