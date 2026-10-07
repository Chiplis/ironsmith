use super::*;
use crate::cards::builders::LifeResourceActionAst;

/// One hand-exile instruction owns its actual-arrival draw count and a
/// persistent permission over precisely the objects that instruction exiled.
/// Also accept the independently rendered sentence boundaries/inline rider.
pub(super) fn parse_exile_hand_draw_play_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::permission_facts::tagged_surface::parse_cast_this_way_mana_rider_tokens;
    let Some(last) = sentences.last() else { return Ok(None); };
    let rider = parse_cast_this_way_mana_rider_tokens(last);
    let body = if rider.is_some() { &sentences[..sentences.len() - 1] } else { sentences };
    let Some((permission_tokens, producer_tokens)) = body.split_last() else { return Ok(None); };
    if !(1..=2).contains(&producer_tokens.len()) { return Ok(None); }
    let Some(mut permission) = parse_cast_or_play_tagged_clause(permission_tokens)? else { return Ok(None); };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Grants(
        GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
            tag, player: PlayerAst::Implicit | PlayerAst::You, allow_land: true,
            without_paying_mana_cost: false, allow_any_color_for_cast: mode,
            permission_bound_mana, filter: None, during_turns_counter_put_on_source: None,
            spell_cost_increase: None, lands_enter_tapped: false, ..
        }), .. }) = &mut permission else { return Ok(None); };
    if let Some(rider) = rider {
        if !mode.is_normal() { return Ok(None); }
        *mode = rider;
    }
    if mode.is_normal() { return Ok(None); }
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() { return Ok(None); }
    fn flatten(effect: EffectAst, into: &mut Vec<EffectAst>) {
        match effect {
            EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
                for effect in effects { flatten(effect, into); }
            }
            effect => into.push(effect),
        }
    }
    let mut producer = Vec::new();
    for sentence in producer_tokens {
        let Ok(effects) = effect_sentences::parse_effect_sentence_lexed(sentence) else { return Ok(None); };
        for effect in effects { flatten(effect, &mut producer); }
    }
    let [exile, draw] = producer.as_mut_slice() else { return Ok(None); };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::ZoneMoves(
        ZoneMoveActionAst::ExileAll { filter, face_down: false }), .. }) = exile else { return Ok(None); };
    if filter.zone != Some(Zone::Hand) || filter.owner.is_none() { return Ok(None); }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, action: SubjectVerbActionAst::LifeResources(
        LifeResourceActionAst::Draw { count }), .. }) = draw else { return Ok(None); };
    if subject.player != PlayerAst::That
        || (!count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ThatManyCards)
            && !matches!(count.unhinted(), Value::PendingPriorEffectMetric(query)
                if query.action == Some(ironsmith_core::PriorEffectAction::Exiled)))
    { return Ok(None); }
    let mut query = ironsmith_core::PriorEffectMetricQuery::new(
        ironsmith_core::EffectMetricSource::AffectedObjects, ironsmith_core::EffectMetric::Count)
        .with_action(ironsmith_core::PriorEffectAction::Exiled);
    query.original_destination = Some(Zone::Exile);
    *count = Value::PendingPriorEffectMetric(query).with_surface_hint(ironsmith_core::ValueSurfaceHint::ThatManyCards);
    let exact = helper_tag_for_tokens(producer_tokens[0], "exiled");
    *tag = crate::tag::TagRef::of(exact.clone());
    *permission_bound_mana = true;
    let exile = EffectAst::TagReferenced { effect: Box::new(exile.clone()), tag: crate::tag::TagRef::of(exact) };
    Ok(Some(vec![exile, draw.clone(), permission]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> Option<Vec<EffectAst>> {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let sentences = crate::lexer::split_lexed_sentences(&tokens);
        parse_exile_hand_draw_play_bundle(&sentences).unwrap()
    }
    #[test]
    fn complete_body_and_canonical_clause_boundaries_keep_one_exact_permission() {
        for text in [
            "Exile all cards from that player's hand, then they draw that many cards. You may play lands and cast spells from among the exiled cards for as long as they remain exiled. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it.",
            "Exile all cards from that player's hand. That player draws that many cards. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast those spells.",
        ] {
            let effects = parse(text).expect("complete hand-exile/draw/play composition");
            assert_eq!(effects.len(), 3);
            assert!(matches!(&effects[2], EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Grants(
                GrantActionAst::GrantPlayTaggedForAsLongAsExiled { permission_bound_mana: true, .. }), .. })));
        }
    }
    #[test]
    fn extra_price_different_drawer_and_fixed_draw_count_do_not_get_the_marker() {
        for prefix in [
            "Exile all cards from that player's hand, then you draw that many cards.",
            "Exile all cards from that player's hand, then they draw three cards.",
            "Exile all cards from that player's graveyard, then they draw that many cards.",
        ] {
            let text = format!("{prefix} You may play lands and cast spells from among the exiled cards for as long as they remain exiled. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it.");
            assert!(parse(&text).is_none());
        }
        assert!(parse("Exile all cards from that player's hand, then they draw that many cards. You may play lands and cast spells from among the exiled cards for as long as they remain exiled without paying their mana costs. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it.").is_none());
    }
}
