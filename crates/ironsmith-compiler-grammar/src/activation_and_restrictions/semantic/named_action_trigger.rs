//! Ordinary labelled-action trigger syntax; label is data, never a card name.
use super::*;
pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    if let Some(at) = words.windows(3).position(|part| matches!(part, ["participates" | "participate", "in", "the"])) {
        let Some(end)=words[at+3..].iter().position(|word|*word=="action").map(|i|at+3+i)else{return Ok(None);};
        if at==0||end==at+3{return Ok(None);}
        let one_or_more=words[..at].starts_with(&["one","or","more"]);
        if one_or_more && at <= 3 { return Ok(None); }
        let label_start=view.token_start_indices()[at+3];
        let label_end=view.token_start_indices()[end];
        if tokens[label_start..label_end].iter().any(|token|token.kind!=crate::lexer::TokenKind::Word){return Ok(None);}
        let start=if one_or_more{view.token_start_indices()[3]}else{0};
        let source_end=view.token_start_indices()[at];
        let participant_filter=parse_object_filter_lexed(&tokens[start..source_end],false)?;
        let mut player=PlayerFilter::Any;
        let performer_filter=if end+1==words.len(){None}else if words.get(end+1..end+3)==Some(&["performed","by"][..]) && end+3<words.len(){
            let actor_tokens=&tokens[view.token_start_indices()[end+3]..];
            match parse_target_phrase(actor_tokens) {
                Ok(TargetAst::Player(actor, _)) => { player=actor; None },
                _ => Some(parse_object_filter_lexed(actor_tokens,false)?),
            }
        }else{return Ok(None);};
        crate::word_claims::claim_tokens(tokens);
        return Ok(Some(TriggerSpec::NamedActionParticipant{label:words[at+3..end].join(" "),player,participant_filter,performer_filter,one_or_more}));
    }
    // A named failed-payment event has the same retained actor and affected
    // spell packet as an ordinary named action. The label is authored data.
    if let Some(fails)=words.iter().position(|word|*word=="fails")
        && words.get(fails+1)==Some(&"a")
        && let Some(of)=words[fails+2..].iter().position(|word|*word=="of").map(|index|index+fails+2)
        && of>fails+2 && of+1<words.len()
    {
        let player=match &words[..fails] {
            ["an","opponent"]=>Some(PlayerFilter::Opponent),["a","player"]=>Some(PlayerFilter::Any),["you"]=>Some(PlayerFilter::You),_=>None,
        };
        if let Some(player)=player {
            let start=view.token_index_after_words(of+1).unwrap_or(of+1);
            let filter=parse_object_filter_lexed(&tokens[start..],false)?;
            crate::word_claims::claim_tokens(tokens);
            return Ok(Some(TriggerSpec::NamedAction {label:format!("fail {}",words[fails+2..of].join(" ")),player,source_filter:ObjectFilter::default(),object_filter:Some(filter),minimum_amount:0}));
        }
    }
    // "Whenever an opponent fails a trial": the failed named payment by
    // that player, whatever spell it was attached to.
    if let Some(fails)=words.iter().position(|word|matches!(*word,"fails"|"fail"))
        && words.get(fails+1)==Some(&"a")
        && fails+2<words.len()
        && !words[fails+2..].contains(&"of")
        && words[fails+2..].iter().all(|word|word.chars().all(|ch|ch.is_ascii_alphabetic()))
        && words.len()-(fails+2)<=2
    {
        let player=match &words[..fails] {
            ["an","opponent"]=>Some(PlayerFilter::Opponent),["a","player"]=>Some(PlayerFilter::Any),["you"]=>Some(PlayerFilter::You),_=>None,
        };
        if let Some(player)=player {
            crate::word_claims::claim_tokens(tokens);
            return Ok(Some(TriggerSpec::NamedAction {label:format!("fail {}",words[fails+2..].join(" ")),player,source_filter:ObjectFilter::default(),object_filter:None,minimum_amount:0}));
        }
    }
    let Some(at) = words.windows(2).position(|words| matches!(words, ["performs" | "perform", "the"])) else { return Ok(None); };
    let Some(end) = words[at + 2..].iter().position(|word| *word == "action").map(|offset| at + 2 + offset) else { return Ok(None); };
    if at == 0 || end == at + 2 { return Ok(None); }
    // "Whenever you draw a card or perform the cache action with a card":
    // the named action shares the left event's subject.
    if words[at - 1] == "or" && at >= 3 && words[0] == "you" && words[at] == "perform" {
        let or_index = view.token_start_indices()[at - 1];
        let action_start = view.token_start_indices()[at];
        let probe = crate::word_claims::probe_only(|| crate::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(&tokens[..or_index]).is_ok());
        if probe {
            let left = crate::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(&tokens[..or_index])?;
            let mut right_tokens = vec![tokens[0].clone()];
            right_tokens.extend_from_slice(&tokens[action_start..]);
            let Some(right) = read(&right_tokens)? else { return Ok(None); };
            crate::word_claims::claim_tokens(&tokens[or_index..or_index + 1]);
            return Ok(Some(TriggerSpec::Either(Box::new(left), Box::new(right))));
        }
    }
    let source_end = view.token_index_after_words(at).unwrap_or(at);
    let label_start = view.token_index_after_words(at + 2).unwrap_or(at + 2);
    let label_end = view.token_index_after_words(end).unwrap_or(end);
    if tokens[label_start..label_end].iter().any(|token| token.kind != crate::lexer::TokenKind::Word) { return Ok(None); }
    let (player, source_filter) = match &words[..at] {
        ["you"] => (PlayerFilter::You, ObjectFilter::default()),
        ["a", "player"] => (PlayerFilter::Any, ObjectFilter::default()),
        ["an", "opponent"] => (PlayerFilter::Opponent, ObjectFilter::default()),
        _ => (PlayerFilter::Any, parse_object_filter_lexed(&tokens[..source_end], false)?),
    };
    let mut after = end + 1;
    let minimum_amount = if words.get(after..after+4) == Some(&["with", "an", "amount", "of"][..]) {
        let Some(amount) = words.get(after+4).and_then(|word| crate::util::parse_number_word_u32(word)) else { return Ok(None); };
        if words.get(after+5..after+7) != Some(&["or", "more"][..]) { return Ok(None); }
        after += 7;
        amount
    } else { 0 };
    // A completed action has an actor and an independent source object.
    // Keep both facts; source control is not a substitute for event.player.
    if words.get(after) == Some(&"from") && after + 1 < words.len() {
        if !matches!(&words[..at], ["you"] | ["a", "player"] | ["an", "opponent"]) {
            return Ok(None);
        }
        let start = view.token_index_after_words(after + 1).unwrap_or(after + 1);
        let source_filter = parse_object_filter_lexed(&tokens[start..], false)?;
        crate::word_claims::claim_tokens(tokens);
        return Ok(Some(TriggerSpec::NamedAction { label: words[at + 2..end].join(" "), player, source_filter, object_filter: None, minimum_amount }));
    }
    let object_filter = if after == words.len() { None }
        else if words.get(after) == Some(&"with") {
            let start = view.token_index_after_words(after + 1).unwrap_or(after + 1);
            if start >= tokens.len() { return Ok(None); }
            Some(parse_object_filter_lexed(&tokens[start..], false)?)
        } else { return Ok(None); };
    crate::word_claims::claim_tokens(tokens);
    Ok(Some(TriggerSpec::NamedAction { label: words[at + 2..end].join(" "), player, source_filter, object_filter, minimum_amount }))
}

#[cfg(test)]
mod qualified_participant_tests {
 use super::*;
 #[test]
 fn operation_actor_is_distinct_from_participant_owner_and_multiplicity() {
  for (surface,plural,actor) in [
   ("a land card in your hand participates in the gathering action performed by you",false,PlayerFilter::You),
   ("one or more land cards in a hand participate in the gathering action performed by an opponent",true,PlayerFilter::Opponent),
  ] {
   let tokens=crate::lexer::lex_line(surface,0).unwrap();
   let Some(TriggerSpec::NamedActionParticipant {label,player,one_or_more,performer_filter,..})=read(&tokens).unwrap() else {panic!("typed participant trigger required")};
   assert_eq!(label,"gathering"); assert_eq!(player,actor); assert_eq!(one_or_more,plural); assert!(performer_filter.is_none());
  }
 }
}

/// Two independently complete event clauses, with an explicit participant
/// event on the right. Object-filter alternatives and shared-subject verbs
/// remain with their existing readers; neither arm borrows the other's subject.
pub(super) fn read_participant_event_union(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    let participant = tokens.windows(3).position(|words|
        words[0].is_any_word(&["participates", "participate"])
            && words[1].is_word("in") && words[2].is_word("the"))?;
    for separator in (1..participant).rev().filter(|index| tokens[*index].is_word("or")) {
        let Some(right) = crate::grammar::primitives::probe_shape(read(&tokens[separator + 1..])).flatten() else { continue; };
        if !matches!(right, TriggerSpec::NamedActionParticipant { .. }) { continue; }
        let Some(left) = crate::grammar::primitives::probe_shape(parse_trigger_clause_lexed(&tokens[..separator])) else { continue; };
        crate::word_claims::claim_tokens(&tokens[separator..separator + 1]);
        return Some(TriggerSpec::Either(Box::new(left), Box::new(right)));
    }
    None
}

#[cfg(test)]
mod participant_event_union_tests {
    use super::*;
    #[test]
    fn heterogeneous_event_arms_keep_their_independent_complete_subjects() {
        let tokens = crate::lexer::lex_line("another creature you control enters from a graveyard or a creature token participates in the rekindle action performed by you", 0).unwrap();
        let parsed = read_participant_event_union(&tokens).expect("two complete event arms");
        let TriggerSpec::Either(left, right) = parsed else { panic!("expected either"); };
        assert!(!matches!(*left, TriggerSpec::NamedActionParticipant { .. }));
        assert!(matches!(*right, TriggerSpec::NamedActionParticipant { label, .. } if label == "rekindle"));
    }
    #[test]
    fn ordinary_object_or_and_single_participant_are_not_event_unions() {
        for text in ["an artifact or creature participates in the rekindle action performed by you", "one or more creature tokens participate in the rekindle action performed by you", "a creature token participates in the rekindle action performed by you"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(read_participant_event_union(&tokens).is_none(), "{text}");
        }
    }
}

#[cfg(test)]mod named_actor_source_filter_tests{use super::*;
 #[test]fn actor_and_source_controller_are_independent_facts(){let tokens=crate::lexer::lex_line("you perform the bridge action from a creature an opponent controls",0).unwrap();let parsed=read(&tokens).unwrap().unwrap();let TriggerSpec::NamedAction{player,source_filter,object_filter,..}=parsed else{panic!("wrong event")};assert_eq!(player,PlayerFilter::You);assert_eq!(source_filter.controller,Some(PlayerFilter::Opponent));assert!(object_filter.is_none());}
}
