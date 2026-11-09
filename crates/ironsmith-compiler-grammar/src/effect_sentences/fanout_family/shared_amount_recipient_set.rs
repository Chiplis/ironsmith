use super::*;

/// A complete shared-amount clause, before `and` can split the recipients
/// into separate instructions. This bounded form declares no new targets.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((_, deal_index)) = find_verb(&tokens) else {
        return Ok(None);
    };
    if !tokens[deal_index].is_any_word(&["deal", "deals"]) {
        return Ok(None);
    }
    let source = non_article_token_word_refs(&tokens[..deal_index]);
    if !crate::util::is_source_reference_words(&source) { return Ok(None); }
    let after = &tokens[deal_index + 1..];
    // "each opponent and planeswalker it has dealt damage to this game": the
    // trailing history clause qualifies the whole union; its typed historical
    // recipient reader owns it.
    if crate::effect_sentences::verb_handlers::is_historical_player_object_damage_recipient_clause(after) {
        return Ok(None);
    }
    let (target_tokens, amount) = if after.first().is_some_and(|token| token.is_word("damage"))
        && after.get(1).is_some_and(|token| token.is_word("to"))
    {
        let Some((equal_start,equal_end))=find_token_word_sequence_span(after,&["equal","to"]) else{return Ok(None);};
        let amount_tokens=&after[equal_end..];
        let Some((amount,used))=crate::grammar::shared_util::value_expr::parse_value_expr_tokens(amount_tokens) else{return Ok(None);};
        if used!=amount_tokens.len(){return Ok(None);}
        (&after[2..equal_start],amount)
    } else {
        let Some((damage_start,damage_end))=find_token_word_sequence_span(after,&["damage","to"]) else{return Ok(None);};
        let amount_tokens=&after[..damage_start];
        let Some((amount,used))=crate::grammar::shared_util::value_expr::parse_value_expr_tokens(amount_tokens) else{return Ok(None);};
        if used!=amount_tokens.len(){return Ok(None);}
        (&after[damage_end..],amount)
    };
    if target_tokens.iter().any(|token|token.is_word("target")){return Ok(None);}
    // A per-recipient amount or condition ("3 damage to each creature without
    // flying if {R} was spent ... and 3 damage to each creature with flying
    // if {G} was spent ...") is two gated instructions, not one shared set.
    if target_tokens.iter().any(|token|token.is_any_word(&["if","unless","damage"])){return Ok(None);}
    // One inherited quantifier can govern a comma/conjunction list of
    // independently read recipient operands ("each creature, player, and
    // planeswalker"). Do not execute the operands as separate instructions.
    let pieces=target_tokens.split(|token|token.is_word("and") || token.is_comma())
        .filter(|piece|!piece.is_empty()).collect::<Vec<_>>();
    if pieces.len()<2{return Ok(None);}
    let inherited_each=pieces[0].first().is_some_and(|token|token.is_word("each"));
    let mut recipients=Vec::new();let mut object_groups=Vec::new();let mut player_groups=Vec::new();
    for piece in pieces {
        let explicit_each=piece.first().is_some_and(|token|token.is_word("each"));
        let operand=if explicit_each{&piece[1..]}else{piece};
        let words=crate::lexer::parser_token_word_refs(operand);
        match words.as_slice(){
            ["player"] if explicit_each || inherited_each=>player_groups.push(PlayerFilter::Any),
            ["opponent"] if explicit_each || inherited_each=>player_groups.push(PlayerFilter::Opponent),
            ["that","creature"]|["that","permanent"]|["that","player"]|["you"]=>recipients.push(parse_target_phrase(piece)?),
            // "each player who controls a white creature" (Disorder): a
            // player group qualified by what it controls, not an object group.
            ["player"|"players"|"opponent"|"opponents","who","control"|"controls",..] if explicit_each || inherited_each=>{
                let mut controlled=match parse_object_filter(&operand[3..],false){Ok(filter)=>filter,Err(_)=>return Ok(None)};
                if controlled.zone.is_none(){controlled.zone=Some(Zone::Battlefield);}
                let controls=PlayerFilter::ControlsMatchingObject{filter:Box::new(controlled)};
                player_groups.push(if words[0].starts_with("opponent"){
                    PlayerFilter::Excluding{base:Box::new(controls),excluded:Box::new(PlayerFilter::You)}
                }else{controls});
            }
            _ if explicit_each || inherited_each=>{
                let mut filter=match parse_object_filter(operand,false){Ok(filter)=>filter,Err(_)=>return Ok(None)};
                // "each opponent and each battle they protect": the pronoun
                // names the player group listed before it, not you.
                if let [.., "they", verb] = words.as_slice()
                    && let Some(players) = player_groups.last().cloned()
                {
                    match *verb {
                        "protect" => filter.protected_by = Some(players),
                        "control" => filter.controller = Some(players),
                        "own" => filter.owner = Some(players),
                        _ => {}
                    }
                }
                object_groups.push(filter);
            }
            _=>return Ok(None),
        }
    }
    crate::word_claims::claim_tokens(&tokens);
    Ok(Some(vec![EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {
            amount,
            recipients,
            object_groups,
            player_groups,
        }),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_shared_suffix_quantity_is_one_typed_recipient_set() {
        for (text, references) in [
            (
                "This spell deals damage to that creature and that player equal to the revealed card's mana value.",
                true,
            ),
            (
                "This spell deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way.",
                false,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effects = parse(&tokens).unwrap().unwrap();
            assert_eq!(effects.len(), 1);
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {
                        amount,
                        recipients,
                        object_groups,
                        player_groups,
                    }),
                ..
            }) = &effects[0]
            else {
                panic!("not a shared packet");
            };
            assert!(matches!(
                amount.unhinted(),
                Value::PendingPriorEffectMetric(_)
            ));
            assert_eq!(recipients.len(), if references { 2 } else { 0 });
            assert_eq!(object_groups.len(), usize::from(!references));
            assert_eq!(player_groups.len(), usize::from(!references));
        }
    }
    #[test]
    fn references_cannot_become_fresh_targets_or_change_the_damage_source() {
        for text in [
            "This spell deals damage to target creature and that player equal to 3.",
            "That creature deals damage to each creature and each player equal to 3.",
            "Each creature deals damage to each creature and each player equal to 3.",
            "This spell deals damage to each creature and each player equal to 3 until end of turn.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse(&tokens).unwrap().is_none(), "{text}");
        }
    }
}

#[cfg(test)]
mod inherited_damage_recipient_tests {
    use super::*;
    #[test]
    fn fixed_amount_and_inherited_each_form_one_typed_union() {
        let tokens=crate::lexer::lex_line("This spell deals 1 damage to each creature, player, and planeswalker.",0).unwrap();
        let effects=parse(&tokens).unwrap().unwrap();
        let [EffectAst::SubjectVerb(SubjectVerbEffectAst {action:SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {amount,object_groups,player_groups,..}),..})]=effects.as_slice() else{panic!("{effects:#?}")};
        assert!(matches!(amount.unhinted(),Value::Fixed(1)));assert_eq!(object_groups.len(),2);assert_eq!(player_groups.len(),1);
    }
    #[test]
    fn recipient_union_reader_does_not_reinterpret_target_selection() {
        let tokens=crate::lexer::lex_line("This spell deals 1 damage to target creature or player.",0).unwrap();assert!(parse(&tokens).unwrap().is_none());
    }
}
