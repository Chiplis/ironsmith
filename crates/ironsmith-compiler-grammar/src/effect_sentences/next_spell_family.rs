use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::grammar::effects::{
    NextSpellGrantAbilitySurface, NextSpellKeywordActionShape, parse_next_spell_grant_tokens,
    parse_next_spell_keyword_action_tokens,
};
use super::super::lexer::OwnedLexToken;
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst};

fn parse_next_spell_grant_ability(
    surface: NextSpellGrantAbilitySurface<'_>,
) -> Option<Vec<GrantedAbilityAst>> {
    if surface == NextSpellGrantAbilitySurface::CantBeCountered {
        return Some(vec![GrantedAbilityAst::KeywordAction(Box::new(
            crate::payload::KeywordAction::CantBeCountered,
        ))]);
    }
    let NextSpellGrantAbilitySurface::Keyword(tokens) = surface else {
        return None;
    };
    let mut end = tokens.len();
    while end > 0 && matches!(tokens[end-1].kind, crate::lexer::TokenKind::Period | crate::lexer::TokenKind::Comma | crate::lexer::TokenKind::Semicolon) { end -= 1; }
    let mut tokens = &tokens[..end];
    for suffix in [&["as","you","cast","it"][..], &["as","you","cast","them"][..], &["as","you","cast","that","spell"][..]] {
        if tokens.len()>=suffix.len() && crate::lexer::token_word_refs(&tokens[tokens.len()-suffix.len()..])==suffix {
            crate::word_claims::claim_tokens(&tokens[tokens.len()-suffix.len()..]);
            tokens=&tokens[..tokens.len()-suffix.len()];break;
        }
    }
    if tokens.iter().any(|token|token.kind==crate::lexer::TokenKind::Quote) {
        let (abilities,choice)=super::parse_granted_abilities_for_gain_clause(tokens,&crate::lexer::token_word_refs(tokens),false).ok()?;
        return (!choice&&!abilities.is_empty()).then_some(abilities);
    }
    if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(tokens),
        &["affinity", "for", "artifacts"],
    ) {
        return Some(vec![GrantedAbilityAst::KeywordAction(Box::new(
            crate::payload::KeywordAction::AffinityForArtifacts,
        ))]);
    }
    let action = match parse_next_spell_keyword_action_tokens(tokens)? {
        NextSpellKeywordActionShape::Known(action) => action,
        NextSpellKeywordActionShape::SingleWord(word) => parse_single_word_keyword_action(word)?,
    };
    action.lowers_to_static_ability().then(|| vec![action.into()])
}

/// "Rather than pay the mana cost of the next spell you cast this turn, you
/// may pay life equal to that spell's mana value." (Marshland Bloodcaster):
/// a one-shot alternative cost for the next matching spell. The spell
/// description is the one the free-cast grant reads.
fn parse_next_spell_life_alternative_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    const HEAD: &[&str] = &["rather", "than", "pay", "the", "mana", "cost", "of"];
    let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let head_words = crate::lexer::token_word_refs(&tokens[..comma]);
    if !head_words.starts_with(HEAD) {
        return Ok(None);
    }
    let tail_words = crate::lexer::token_word_refs(&tokens[comma + 1..]);
    let pays_life = matches!(
        tail_words.as_slice(),
        ["you", "may", "pay", "life", "equal", "to", "that", possessive, "mana", "value"]
            if matches!(*possessive, "spell's" | "spells" | "spell’s")
    );
    if !pays_life {
        return Ok(None);
    }
    // The described spell, read as "<spell> can be cast without paying its
    // mana cost" by the next-spell grant grammar.
    let Some(spell_start) = crate::lexer::TokenWordView::new(&tokens[..comma])
        .token_index_after_words(HEAD.len())
    else {
        return Ok(None);
    };
    let mut described = tokens[spell_start..comma].to_vec();
    described.extend(crate::lexer::synthetic_word_tokens([
        "can", "be", "cast", "without", "paying", "its", "mana", "cost",
    ]));
    let Some(shape) = parse_next_spell_grant_tokens(&described)? else {
        return Ok(None);
    };
    if shape.ability
        != super::super::grammar::effects::NextSpellGrantAbilitySurface::WithoutPayingManaCost
    {
        return Ok(None);
    }
    let Ok([filter]) = <[_; 1]>::try_from(shape.filters) else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::subject_verb_pay_life_for_next_spell_this_turn(
        shape.player,
        filter,
    )]))
}

fn incarnation_static_ability(ability:&GrantedAbilityAst)->bool {
    match ability {
        GrantedAbilityAst::StaticAbility(_)=>true,
        GrantedAbilityAst::KeywordAction(action)=>crate::static_ability_helpers::static_ability_for_keyword_action((**action).clone()).is_some(),
        GrantedAbilityAst::ParsedObjectAbility{ability,..}=>matches!(&ability.ability.kind,crate::model::CompilerAbilityKindCore::Static(_)),
        _=>false,
    }
}

// A following as-you-cast clause refers to the same next-spell set and
// turn window. Both sentences must be completely read before either grant
// is registered; quoted entry programs retain their own sentence boundaries.
fn parse_bounded_next_spell_followup(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    use crate::lexer::TokenKind;
    let mut quoted=false;
    let Some(split)=tokens.iter().position(|token|{
        if token.kind==TokenKind::Quote {quoted=!quoted;}
        token.kind==TokenKind::Period&&!quoted
    })else{return Ok(None)};
    let Some(shape)=parse_next_spell_grant_tokens(&tokens[..=split])?else{return Ok(None)};
    if shape.player!=crate::cards::builders::PlayerAst::You
        || shape.ability!=NextSpellGrantAbilitySurface::CastTiming{return Ok(None);}
    let tail=&tokens[split+1..];
    let view=crate::lexer::TokenWordView::new(tail);
    let words=view.word_refs();
    if words.get(..7)!=Some(&["as","you","cast","that","spell","it","gains"][..]){return Ok(None);}
    let Some(gains)=view.token_start_indices().get(6).copied()else{return Ok(None)};
    let Some(comma)=tail.iter().position(|token|token.kind==TokenKind::Comma)else{return Ok(None)};
    if comma>=gains||crate::lexer::parser_token_word_refs(&tail[..comma])!=["as","you","cast","that","spell"]{return Ok(None);}
    let mut end=tail.len();
    if end>gains+1&&tail[end-1].kind==TokenKind::Period{end-=1;}
    let payload=&tail[gains+1..end];
    let mut quoted=false;
    for token in payload{
        if token.kind==TokenKind::Quote{quoted=!quoted;}
        else if token.kind==TokenKind::Period&&!quoted{return Ok(None);}
    }
    if quoted||payload.is_empty(){return Ok(None);}
    let Some(abilities)=parse_next_spell_grant_ability(NextSpellGrantAbilitySurface::Keyword(payload))else{return Ok(None)};
    if abilities.iter().any(|ability|!incarnation_static_ability(ability)){return Ok(None);}
    let filters=shape.filters.clone();let player=shape.player;
    let Some(mut effects)=lower_next_spell_grant(shape)else{return Ok(None)};
    for filter in filters {for ability in &abilities{
        effects.push(EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,player,
            crate::cards::builders::SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantNextSpellAbilityThisTurn{
                filter:filter.clone(),ability:Box::new(ability.clone()),mode:ironsmith_core::NextSpellGrantMode::IncarnationAbility,repeat_for_turn:false,
            }),
        ));
    }}
    crate::word_claims::claim_tokens(&tail[..gains+1]);
    if end<tail.len(){crate::word_claims::claim_tokens(&tail[end..]);}
    Ok(Some(vec![EffectAst::Coordinated{effects,leading_duration:false,result_conjunction:false}]))
}


// The next creature's external entry followup belongs to the same one-shot
// caster/filter/turn window. It does not become that creature's own ability.
fn parse_next_creature_entry_followup(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    use crate::lexer::TokenKind;
    let mut quoted=false;
    let Some(split)=tokens.iter().position(|token|{if token.kind==TokenKind::Quote{quoted=!quoted;}token.kind==TokenKind::Period&&!quoted})else{return Ok(None)};
    let Some(shape)=parse_next_spell_grant_tokens(&tokens[..=split])?else{return Ok(None)};
    if shape.filters.len()!=1 || shape.filters.iter().any(|filter|!filter.card_types.contains(&crate::types::CardType::Creature)){return Ok(None)}
    let NextSpellGrantAbilitySurface::Keyword(payload)=shape.ability else{return Ok(None)};
    let Some(abilities)=parse_next_spell_grant_ability(NextSpellGrantAbilitySurface::Keyword(payload))else{return Ok(None)};
    if abilities.iter().any(|ability|!incarnation_static_ability(ability)){return Ok(None)}
    let tail=&tokens[split+1..];let words=crate::lexer::parser_token_word_refs(tail);
    if words.get(..4)!=Some(&["when","that","creature","enters"][..]){return Ok(None)}
    let mut quoted=false;let mut end=tail.len();
    if end>0&&tail[end-1].kind==TokenKind::Period{end-=1;}
    for token in &tail[..end]{if token.kind==TokenKind::Quote{quoted=!quoted;}else if token.kind==TokenKind::Period&&!quoted{return Ok(None)}}
    if quoted{return Ok(None)}
    // A generic event subject makes ordinary `it` bind the entrant, while
    // `this permanent` in the body still names the issuing ability's source.
    let Some(that)=tail.iter().position(|token|token.is_word("that"))else{return Ok(None)};
    let mut event=tail.to_vec();event[that]=OwnedLexToken::word("a",event[that].span);
    let span=event.first().map(|token|token.span).unwrap();
    event.insert(0,OwnedLexToken::new(TokenKind::Quote,"\"",span));
    event.push(OwnedLexToken::new(TokenKind::Quote,"\"",span));
    let Some(followup)=parse_next_spell_grant_ability(NextSpellGrantAbilitySurface::Keyword(&event))else{return Ok(None)};
    if followup.len()!=1{return Ok(None)}
    if !matches!(&followup[0],GrantedAbilityAst::ParsedObjectAbility{ability,..} if matches!(&ability.ability.kind,crate::model::CompilerAbilityKindCore::Triggered(_))){return Ok(None)}
    let mut effects=Vec::new();
    for filter in shape.filters {
        for ability in &abilities {effects.push(EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,shape.player,
            crate::cards::builders::SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantNextSpellAbilityThisTurn{
                filter:filter.clone(),ability:Box::new(ability.clone()),mode:ironsmith_core::NextSpellGrantMode::IncarnationAbility,repeat_for_turn:false,
            }),
        ));}
        effects.push(EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,shape.player,
            crate::cards::builders::SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantNextSpellAbilityThisTurn{
                filter,ability:Box::new(followup[0].clone()),mode:ironsmith_core::NextSpellGrantMode::EntryFollowup,repeat_for_turn:false,
            }),
        ));
    }
    crate::word_claims::claim_tokens(tail);
    Ok(Some(vec![EffectAst::Coordinated{effects,leading_duration:false,result_conjunction:false}]))
}

pub fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effects) = parse_next_creature_entry_followup(tokens)? { return Ok(Some(effects)); }
    if let Some(effects) = parse_bounded_next_spell_followup(tokens)? { return Ok(Some(effects)); }
    if let Some(effects)=parse_recurring_spell_grant(tokens)? { return Ok(Some(effects)); }
    // The cast-time qualifier belongs to the next-spell envelope, never to
    // a quoted ability's dynamic count/filter. Remove it before any nested
    // static program parser can see it. Quotes must be balanced before it.
    let mut end = tokens.len();
    if end > 0 && tokens[end - 1].kind == crate::lexer::TokenKind::Period { end -= 1; }
    let mut qualifier = None;
    if tokens.first().is_some_and(|t| t.is_word("the"))
        && tokens.get(1).is_some_and(|t| t.is_word("next"))
    {
        for suffix in [&["as", "you", "cast", "it"][..], &["as", "you", "cast", "that", "spell"][..]] {
            if end >= suffix.len() {
                let start = end - suffix.len();
                if tokens[start..end].iter().zip(suffix).all(|(token, word)| token.is_word(word))
                    && tokens[..start].iter().filter(|t| t.kind == crate::lexer::TokenKind::Quote).count() % 2 == 0
                {
                    qualifier = Some(&tokens[start..end]);
                    break;
                }
            }
        }
    }
    let tokens = if let Some(qualifier) = qualifier {
        &tokens[..end - qualifier.len()]
    } else { tokens };
    if let Some(effects) = parse_next_spell_life_alternative_sentence(tokens)? {
        return Ok(Some(effects));
    }
    if let Some(shape) = parse_next_spell_grant_tokens(tokens)?
        && let Some(effects) = lower_next_spell_grant(shape)
    {
        if let Some(qualifier) = qualifier { crate::word_claims::claim_tokens(qualifier); }
        return Ok(Some(effects));
    }

    // A next-spell grant can be coordinated with an independent optional
    // action, as in “The next spell ... has cascade and you may planeswalk.”
    // Parse the one-shot grant on the left before the broad static-grant
    // fallback can discard both `next` and `this turn`, then route the right
    // clause through the ordinary effect grammar.
    let Some(split) = crate::slice_primitives::find_window_by(tokens, 3, |window| {
        window[0].is_word("and") && window[1].is_word("you") && window[2].is_word("may")
    }) else {
        return Ok(None);
    };
    let Some(shape) = parse_next_spell_grant_tokens(&tokens[..split])? else {
        return Ok(None);
    };
    let Some(mut effects) = lower_next_spell_grant(shape) else {
        return Ok(None);
    };
    effects.extend(super::parse_effect_sentence_lexed(&tokens[split + 1..])?);
    Ok(Some(effects))
}

fn lower_next_spell_grant(
    shape: super::super::grammar::effects::NextSpellGrantShape<'_>,
) -> Option<Vec<EffectAst>> {
    if shape.ability
        == super::super::grammar::effects::NextSpellGrantAbilitySurface::WithoutPayingManaCost
    {
        let [filter] = <[_; 1]>::try_from(shape.filters).ok()?;
        return Some(vec![
            EffectAst::subject_verb_free_cast_next_spell_this_turn(shape.player, filter),
        ]);
    }
    if matches!(shape.ability, NextSpellGrantAbilitySurface::CastTiming | NextSpellGrantAbilitySurface::PlayTiming) {
        return Some(shape.filters.into_iter().map(|filter| {
            EffectAst::subject_verb_next_play_timing_this_turn(
                shape.player, filter, shape.ability == NextSpellGrantAbilitySurface::PlayTiming,
            )
        }).collect());
    }
    let abilities = parse_next_spell_grant_ability(shape.ability)?;
    let effects = shape
        .filters
        .into_iter()
        .flat_map(|filter| abilities.iter().map(move |ability| {
            EffectAst::subject_verb_grant_next_spell_ability_this_turn(shape.player,filter.clone(),ability.clone())
        }))
        .collect::<Vec<_>>();
    Some(if effects.len() > 1 {
        vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]
    } else {
        effects
    })
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    #[test]
    fn next_spell_qualifier_stays_outside_complete_quoted_static_programs() {
        for suffix in ["", " as you cast it", " as you cast that spell"] {
            let text = format!("The next creature spell you cast this turn gains custom keyword \"practiced\", and \"This spell costs {{1}} less to cast for each creature you attacked with this turn.\"{suffix}.");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let effects = parse_next_spell_grant_sentence_lexed(&tokens).unwrap().expect("complete next-spell envelope");
            let debug = format!("{effects:?}");
            assert!(debug.contains("CreaturesAttackedWith"), "{debug}");
            assert!(debug.contains("practiced"), "{debug}");
        }
    }
    #[test]
    fn next_spell_qualifier_does_not_consume_an_independent_cast_clause() {
        let tokens = crate::lexer::lex_line("The next creature spell you cast this turn gains haste. As you cast this spell, draw a card.",0).unwrap();
        assert!(parse_next_spell_grant_sentence_lexed(&tokens).unwrap().is_none());
    }
}

// Cardinality and recipient filter are independent of the quoted program.
fn parse_recurring_spell_grant(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
 let mut end=tokens.len();
 while end>0 && matches!(tokens[end-1].kind,crate::lexer::TokenKind::Period|crate::lexer::TokenKind::Comma|crate::lexer::TokenKind::Semicolon){end-=1;}
 let tokens=&tokens[..end];
 let Some(split)=tokens.windows(5).position(|part|crate::lexer::token_word_refs(part)==["you","cast","this","turn","gain"])else{return Ok(None)};
 if split==0 || !tokens[split-1].is_word("spells"){return Ok(None)}
 // A prior instruction is not part of the recipient noun phrase.
 let subject=&tokens[..split];let mut quoted=false;
 for token in subject{if token.kind==crate::lexer::TokenKind::Quote{quoted=!quoted}else if !quoted&&matches!(token.kind,crate::lexer::TokenKind::Period|crate::lexer::TokenKind::Semicolon|crate::lexer::TokenKind::Colon){return Ok(None)}}
 if let Some(clause)=crate::grammar::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(subject){
  if !clause.subject_tokens.last().is_some_and(|token|token.is_word("spells")){return Ok(None)}
 }
 let ability_tokens=&tokens[split+5..];
 let Some(abilities)=parse_next_spell_grant_ability(NextSpellGrantAbilitySurface::Keyword(ability_tokens))else{return Ok(None)};
 if abilities.iter().any(|ability|!incarnation_static_ability(ability)){return Ok(None)}
 let mut filter=crate::word_claims::audit_filter(subject,std::panic::Location::caller(),||Ok(crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(subject)))?;
 filter.cast_by=Some(crate::target::PlayerFilter::You);
 crate::word_claims::claim_tokens(&tokens[split..split+5]);
 let effects=abilities.into_iter().map(|ability|EffectAst::subject_verb(
  crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,crate::cards::builders::PlayerAst::You,
  crate::cards::builders::SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantNextSpellAbilityThisTurn{
   filter:filter.clone(),ability:Box::new(ability),mode:ironsmith_core::NextSpellGrantMode::IncarnationAbility,repeat_for_turn:true,
  })
 )).collect();
 Ok(Some(effects))
}

#[cfg(test)]
mod bounded_followup_tests {
 use super::*;
 #[test]
 fn shared_next_spell_selector_owns_cast_timing_and_a_complete_entry_program(){
  let text="The next creature spell you cast this turn can be cast as though it had flash. As you cast that spell, it gains custom keyword \"forager\", and \"As this permanent enters, you may sacrifice up to 2 artifacts and/or creatures. This permanent enters with that many +1/+1 counters on it.\".";
  let tokens=crate::lexer::lex_line(text,0).unwrap();
  let effects=parse_next_spell_grant_sentence_lexed(&tokens).unwrap().expect("complete two-sentence envelope");
  let debug=format!("{effects:?}");assert!(debug.contains("forager"));assert!(debug.contains("Coordinated"));assert!(debug.contains("CastTiming"),"{debug}");
 }
 #[test]
 fn unrelated_cast_subjects_and_unbounded_followups_are_not_consumed(){
  for tail in ["As you cast this spell, it gains flying.","As they cast that spell, it gains flying.","As you cast that spell, it gains flying. Draw a card."]{
   let text=format!("The next creature spell you cast this turn can be cast as though it had flash. {tail}");let tokens=crate::lexer::lex_line(&text,0).unwrap();assert!(parse_bounded_next_spell_followup(&tokens).unwrap().is_none(),"{tail}");
  }
 }
}


#[cfg(test)]
mod exact_next_entry_followup_tests {
    use super::*;
    #[test]
    fn paired_next_creature_grant_keeps_an_external_entry_program(){
        let text="The next creature spell you cast this turn has haste. When that creature enters, it becomes an enchantment in addition to its other types.";
        let tokens=crate::lexer::lex_line(text,0).unwrap();
        let program=parse_next_creature_entry_followup(&tokens).unwrap().expect("complete shared next-creature envelope");
        let debug=format!("{program:?}");assert!(debug.contains("EntryFollowup"),"external timing required: {debug}");assert!(debug.contains("IncarnationAbility"),"first grant stays noncopiable: {debug}");assert!(debug.contains("Enchantment"),"complete body survives: {debug}");
    }
    #[test]
    fn entry_followup_does_not_consume_an_independent_instruction_or_other_recipient(){
        for text in [
            "The next instant spell you cast this turn has lifelink. When that creature enters, you gain 2 life.",
            "The next creature spell you cast this turn has haste. When another creature enters, you gain 2 life.",
            "The next creature spell you cast this turn has haste. When that creature enters, you gain 2 life. Draw a card.",
        ] {let tokens=crate::lexer::lex_line(text,0).unwrap();assert!(parse_next_creature_entry_followup(&tokens).unwrap().is_none(),"{text}");}
    }
}
