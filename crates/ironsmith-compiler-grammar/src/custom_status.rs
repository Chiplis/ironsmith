//! Explicit named-status syntax used by custom mechanic expansion. Names are data.
use crate::cards::builders::{
    EffectAst, KeywordActionAst, PlayerAst, PredicateAst, SourcePredicateAst, SubjectVerbActionAst,
    SubjectVerbRoleAst,
};
use crate::lexer::{OwnedLexToken, TokenKind, token_word_refs};
use crate::target::ObjectFilter;

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    token_word_refs(tokens)
}
fn name(tokens: &[OwnedLexToken]) -> Option<String> {
    if tokens.is_empty() { return None; }
    let tokens = if tokens.first().is_some_and(|token| token.kind == TokenKind::Quote)
        && tokens.last().is_some_and(|token| token.kind == TokenKind::Quote) && tokens.len() > 2 {
        let inner = &tokens[1..tokens.len()-1];
        if inner.iter().any(|token| token.kind == TokenKind::Quote) { return None; }
        inner
    } else {
        if tokens.iter().any(|token| token.kind != TokenKind::Word) { return None; }
        tokens
    };
    // This independently bounded operand is a literal designation name.
    // Preserve authored spelling if source-reference preprocessing annotated
    // one of its words; it is never an executable self-reference.
    let name = crate::lexer::render_literal_token_slice(tokens).to_ascii_lowercase();
    (!name.is_empty()).then_some(name)
}
fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let end = tokens
        .iter()
        .rposition(|t| !matches!(t.kind, TokenKind::Period | TokenKind::Comma | TokenKind::Semicolon))
        .map_or(0, |i| i + 1);
    &tokens[..end]
}

/// Peel precisely `with[out] the <name> status`, retaining all other filter words.
pub(crate) fn filter_clause(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String, bool)> {
    let mut quoted = false;
    for start in 0..tokens.len() {
        if tokens[start].kind == TokenKind::Quote { quoted = !quoted; continue; }
        if quoted { continue; }
        let prefix = words(&tokens[start..tokens.len().min(start + 2)]);
        if !matches!(prefix.as_slice(), ["with" | "without", "the"]) {
            continue;
        }
        let mut quoted_name = false;
        let end = (start + 2..tokens.len()).find(|i| {
            if tokens[*i].kind == TokenKind::Quote { quoted_name = !quoted_name; return false; }
            !quoted_name && tokens[*i].is_word("status")
        })?;
        let name = name(&tokens[start + 2..end])?;
        let mut base = tokens[..start].to_vec();
        base.extend_from_slice(&tokens[end + 1..]);
        crate::word_claims::claim_tokens(&tokens[start..end + 1]);
        return Some((base, name, prefix[0] == "without"));
    }
    None
}

pub(crate) fn source_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let tokens = trimmed(tokens);
    let end = tokens.len().checked_sub(1)?;
    if words(&tokens[end..]) != ["status"] {
        return None;
    }
    let at = (0..end).find(|i| words(&tokens[*i..*i + 1]) == ["the"])?;
    if tokens[..at]
        .iter()
        .any(|token| token.kind != TokenKind::Word)
    {
        return None;
    }
    let prefix = words(&tokens[..at]);
    let prefix = prefix.strip_prefix(&["if"]).unwrap_or(&prefix);
    let (subject, negative) = if let Some(subject) = prefix.strip_suffix(&["has"]) {
        (subject, false)
    } else if let Some(subject) = prefix.strip_suffix(&["does", "not", "have"]) {
        (subject, true)
    } else if let Some(subject) = prefix.strip_suffix(&["doesn't", "have"]).or_else(|| prefix.strip_suffix(&["doesnt", "have"])) {
        (subject, true)
    } else {
        return None;
    };
    if !matches!(
        subject,
        ["this"] | ["it"]
            | [
                "this",
                "creature"
                    | "permanent"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "planeswalker"
                    | "battle"
                    | "card"
            ]
    ) {
        return None;
    }
    let name = name(&tokens[at + 1..end])?;
    let mut filter = ObjectFilter::default();
    filter.custom_statuses.push(name);
    crate::word_claims::claim_tokens(tokens);
    let condition = if subject == ["it"] {
        PredicateAst::ItMatches(filter)
    } else {
        PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
    };
    Some(if negative {
        PredicateAst::Not(Box::new(condition))
    } else {
        condition
    })
}

pub(crate) fn source_effect(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    if let Some(effect)=source_collection_effect(tokens){return Some(effect);}
    let tokens = trimmed(tokens);
    let inspection_suffix=["for","as","long","as","it","remains","exiled"];
    if words(tokens).starts_with(&["allow","the","controllers","of"])
        && words(tokens).ends_with(&inspection_suffix) {
        let split=(4..tokens.len()).find(|start|words(&tokens[*start..tokens.len().min(*start+5)])==["just","created","to","look","at"])?;
        // "the controllers of <tokens> just created": the tokens this
        // program just created, not an earlier antecedent ("those ...").
        let created=words(&tokens[4..split]).iter().any(|word|matches!(*word,"token"|"tokens"));
        let named=created && tokens[4..split].iter().any(|token|token.is_word("named"));
        let sources=if named {
            // "tokens named Memory just created": lowering binds the name to
            // the token this instruction list created.
            let named_at=(4..split).find(|index|tokens[*index].is_word("named"))?;
            let name=tokens.get(named_at+1..split)?.iter().map(|token|token.parser_text()).collect::<Vec<_>>().join(" ");
            if name.is_empty(){return None;}
            crate::cards::builders::TargetAst::Object(ObjectFilter::default().token().named(name),None,None)
        } else if created {
            crate::util::parse_target_phrase(&crate::lexer::synthetic_word_tokens(["the","tokens","created","this","way"])).ok()?
        } else {
            let mut source_phrase=crate::lexer::synthetic_word_tokens(["those"]);
            source_phrase.extend_from_slice(&tokens[4..split]);
            crate::util::parse_target_phrase(&source_phrase).ok()?
        };
        if created { crate::word_claims::claim_tokens(&tokens[4..split]); }
        let end=tokens.len().checked_sub(inspection_suffix.len())?;
        let target=crate::util::parse_target_phrase(tokens.get(split+5..end)?).ok()?;
        crate::word_claims::claim_tokens(&tokens[..4]);crate::word_claims::claim_tokens(&tokens[split..split+5]);crate::word_claims::claim_tokens(&tokens[end..]);
        return Some(EffectAst::subject_verb(SubjectVerbRoleAst::Actor,PlayerAst::You,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::GrantExileControllerInspection{target,sources})));
    }
    if words(tokens).as_slice() == ["remember", "the", "original", "cards", "exiled", "this", "way"] {
        crate::word_claims::claim_tokens(tokens);
        return Some(EffectAst::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::You,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::RememberOriginalMovementArrivals {
                from: crate::effect::EffectId::PENDING_PRIOR_ACTION, zone: crate::zone::Zone::Exile,
                tag: crate::tag::TagRef::of(crate::tag::TagKey::from("__remembered_original_arrivals__")),
            })));
    }
    if words(tokens).starts_with(&["remove", "the"]) {
        let end = tokens.iter().position(|token| token.is_word("status"))?;
        if !tokens.get(end + 1).is_some_and(|token| token.is_word("from")) { return None; }
        let in_at = (end + 2..tokens.len()).find(|i| tokens[*i].is_word("in"))?;
        let zone = match words(&tokens[in_at..]).as_slice() {
            ["in", "the", "command", "zone"] => crate::zone::Zone::Command,
            ["in", "exile"] => crate::zone::Zone::Exile,
            _ => return None,
        };
        let status_name = name(&tokens[2..end])?;
        let target = crate::util::parse_target_phrase(&tokens[end + 2..in_at]).ok()?;
        crate::word_claims::claim_tokens(tokens);
        return Some(EffectAst::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::You,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::SetPermanentDesignation {
                target, designation: ironsmith_core::PermanentDesignation::CustomStatusInZone {
                    name: status_name, zone, present: false, carry_to_stack: false,
                },
            })));
    }
    // Do not treat an enclosing activation cost as part of this instruction.
    if !tokens.first().is_some_and(|token| token.is_word("give")) {
        return None;
    }
    // The status operand's article is the last unquoted "the" before the
    // status word: the recipient phrase may contain its own ("Give the most
    // recently exiled card the cached status").
    let first_status = tokens.iter().position(|t| words(std::slice::from_ref(t)) == ["status"]);
    let at = match first_status {
        Some(status) if !tokens[..status].iter().any(|token| token.kind == TokenKind::Quote) =>
            tokens[..status].iter().rposition(|t| words(std::slice::from_ref(t)) == ["the"])?,
        _ => tokens.iter().position(|t| words(std::slice::from_ref(t)) == ["the"])?,
    };
    let end = if tokens.get(at + 1).is_some_and(|token| token.kind == TokenKind::Quote) {
        let close = (at + 2..tokens.len()).find(|index| tokens[*index].kind == TokenKind::Quote)?;
        let end = close + 1;
        if words(tokens.get(end..end+1)?) != ["status"] { return None; }
        end
    } else {
        tokens.iter().position(|t| words(std::slice::from_ref(t)) == ["status"])?
    };
    // The ordinary target reader owns target/non-target/reference distinctions.
    // Never infer Source from an arbitrary recipient phrase.
    let target = crate::util::parse_target_phrase(&tokens[1..at]).ok()?;
    // "until it has no +1/+1 counters": a counter-absence lifetime. The kind
    // is read by the ordinary counter-type reader.
    let tail_tokens = trimmed(&tokens[end + 1..]);
    let tail_words = words(tail_tokens);
    if let ["until", "it", "has", "no", ..] = tail_words.as_slice()
        && tail_tokens.len() > 4
        && tokens[..at].iter().all(|token| token.kind == TokenKind::Word)
        && let Some(counter_type) = crate::grammar::filters::parse_counter_type_from_tokens(&tail_tokens[4..])
    {
        let name = name(&tokens[at + 1..end])?;
        crate::word_claims::claim_tokens(tokens);
        return Some(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::SetPermanentDesignation {
                target,
                designation: ironsmith_core::PermanentDesignation::CustomStatusUntilEvent {
                    name, event: ironsmith_core::CustomStatusExpiryEvent::NoCounters(counter_type),
                },
            }),
        ));
    }
    if tokens[..at]
        .iter()
        .chain(tokens[end + 1..].iter())
        .any(|token| token.kind != TokenKind::Word)
    {
        return None;
    }
    let tail = words(&tokens[end + 1..]);
    let name = name(&tokens[at + 1..end])?;
    let designation = match tail.as_slice() {
        ["in", "exile", "and", "on", "the", "spell", "it", "becomes"] => ironsmith_core::PermanentDesignation::CustomStatusInZone { name, zone: crate::zone::Zone::Exile, present: true, carry_to_stack: true },
        ["in", "the", "command", "zone"] => ironsmith_core::PermanentDesignation::CustomStatusInZone { name, zone: crate::zone::Zone::Command, present: true, carry_to_stack: false },
        ["in", "exile"] => ironsmith_core::PermanentDesignation::CustomStatusInZone { name, zone: crate::zone::Zone::Exile, present: true, carry_to_stack: false },
        [] => ironsmith_core::PermanentDesignation::CustomStatus { name, until_end_of_turn: false },
        ["while", "in", "exile"] => ironsmith_core::PermanentDesignation::CustomStatusInZone { name, zone: ironsmith_core::Zone::Exile, present: true, carry_to_stack: false },
        ["until", "end", "of", "turn"] => ironsmith_core::PermanentDesignation::CustomStatus { name, until_end_of_turn: true },
        ["until", "it", "turns", "face", "up"] => ironsmith_core::PermanentDesignation::CustomStatusUntilEvent { name, event: ironsmith_core::CustomStatusExpiryEvent::TurnsFaceUp },
        ["until", "it", "phases", "in"] => ironsmith_core::PermanentDesignation::CustomStatusUntilEvent { name, event: ironsmith_core::CustomStatusExpiryEvent::PhasesIn },
        _ => return None,
    };
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::You,
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::SetPermanentDesignation {
            target,
            designation,
        }),
    ))
}

pub(crate) fn gained_trigger(tokens: &[OwnedLexToken]) -> Option<(Vec<OwnedLexToken>, String)> {
    let tokens = trimmed(tokens);
    let end = tokens.len().checked_sub(1)?;
    if words(&tokens[end..]) != ["status"] {
        return None;
    }
    let at = (0..end).find(|i| {
        matches!(
            words(&tokens[*i..tokens.len().min(*i + 2)]).as_slice(),
            ["gains" | "gain", "the"]
        )
    })?;
    let name = name(&tokens[at + 2..end])?;
    crate::word_claims::claim_tokens(&tokens[at..]);
    Some((tokens[..at].to_vec(), name))
}

/// Explicit source-owned membership. The relation name is data; the existing
/// target reader retains choose/target/reference distinctions for the member.
fn source_collection_effect(tokens:&[OwnedLexToken])->Option<EffectAst>{
    let tokens=trimmed(tokens);
    if !tokens.first().is_some_and(|token|token.is_word("record")){return None;}
    let start=(1..tokens.len()).find(|index|words(&tokens[*index..(*index+2).min(tokens.len())])==["in","the"])?;
    let collection=(start+2..tokens.len()).find(|index|tokens[*index].is_word("collection"))?;
    if !tokens.get(collection+1).is_some_and(|token|token.is_word("of")){return None;}
    let while_at=(collection+2..tokens.len()).find(|index|tokens[*index].is_word("while"))?;
    let label=name(&tokens[start+2..collection])?;
    let source_tag=match words(&tokens[collection+2..while_at]).as_slice(){
        ["this","card"|"permanent"|"object"]=>None,
        ["the","created","token"]|["the","created","tokens"]=>Some(crate::tag::TagRef::of(crate::tag::TagKey::from("__named_action_created_objects"))),
        ["the","exiled","card"]=>Some(crate::tag::CompilerReferenceTag::PriorExiledCard.bind()),
        ["the","iterated","card"|"object"]=>Some(crate::tag::CompilerReferenceTag::CurrentIteratedObject.bind()),
        _=>return None,
    };
    let tail=words(&tokens[while_at+1..]);
    fn zone(words:&[&str])->Option<crate::zone::Zone>{Some(match words{
        ["on","the","battlefield"]=>crate::zone::Zone::Battlefield,
        ["on","the","stack"]=>crate::zone::Zone::Stack,
        ["in","exile"]=>crate::zone::Zone::Exile,
        ["in","the","graveyard"]=>crate::zone::Zone::Graveyard,
        ["in","the","hand"]=>crate::zone::Zone::Hand,
        ["in","the","library"]=>crate::zone::Zone::Library,
        ["in","the","command","zone"]=>crate::zone::Zone::Command,
        _=>return None,
    })}
    let (source_zone, members) = if let Some(source) = tail.strip_prefix(&["that","source","remains"]) {
        let and = source.iter().position(|word|*word=="and")?;
        (Some(zone(&source[..and])?), &source[and+1..])
    } else { (None, tail.as_slice()) };
    let member_zone=zone(members.strip_prefix(&["those","members","remain"])?)?;
    let target=if words(&tokens[1..start]).as_slice()==["the","remembered","cards"] {
        crate::cards::builders::TargetAst::Tagged(crate::tag::TagRef::of(crate::tag::TagKey::from("__remembered_original_arrivals__")),None)
    } else {crate::util::parse_target_phrase(&tokens[1..start]).ok()?};
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::subject_verb(SubjectVerbRoleAst::Actor,PlayerAst::You,
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::RecordSourceCollection{target,source_tag,name:label,source_zone,member_zone})))
}

#[cfg(test)]
mod collection_record_reader_tests{
    use super::*;
    #[test]
    fn explicit_collection_name_and_endpoint_zones_are_owned(){
        let tokens=crate::lexer::lex_line("Record that creature in the scripted host collection of the exiled card while that source remains in exile and those members remain on the battlefield.",0).unwrap();
        let effect=source_effect(&tokens).unwrap();
        let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst{action:SubjectVerbActionAst::KeywordActions(KeywordActionAst::RecordSourceCollection{name,source_tag:Some(tag),source_zone,member_zone,..}),..})=effect else{panic!("source collection")};
        assert_eq!(name,"scripted host");assert_eq!(tag.key,crate::tag::CompilerReferenceTag::PriorExiledCard.key());assert_eq!(source_zone,Some(crate::zone::Zone::Exile));assert_eq!(member_zone,crate::zone::Zone::Battlefield);
    }
    #[test]
    fn incomplete_or_extra_collection_lifetime_is_not_claimed(){
        for text in ["Record that creature in the scripted host collection of the exiled card.","Record that creature in the scripted host collection of the exiled card while that source remains in exile and those members remain on the battlefield then draw a card."]{
            assert!(source_effect(&crate::lexer::lex_line(text,0).unwrap()).is_none());
        }
    }
}

#[cfg(test)]
mod literal_status_operand_tests {
    use super::*;
    #[test]
    fn status_operand_uses_authored_literal_surface_not_source_alias_spelling() {
        let mut tokens=crate::lexer::lex_line("unlocked Work named canvas",0).unwrap();
        tokens[1].set_literal_surface("WORK");
        assert_eq!(name(&tokens),Some("unlocked work named canvas".into()));
        let mut rewritten=crate::lexer::lex_line("unlocked source named canvas",0).unwrap();
        rewritten[1].set_literal_surface("work");
        assert_eq!(name(&rewritten),Some("unlocked source named canvas".into()),
            "a semantic replacement cannot borrow a different operand's old literal spelling");
    }
}

#[cfg(test)]
mod status_qualifier_quote_boundary_tests {
    use super::*;
    #[test]
    fn quoted_literal_names_and_rule_text_are_not_outer_status_qualifiers() {
        for text in ["a card named \"Portrait with the charted status\"", "a permanent with \"This creature gains the charted status.\""] {
            let tokens=crate::lexer::lex_line(text,0).unwrap();
            assert!(filter_clause(&tokens).is_none(),"{text}");
        }
        let tokens=crate::lexer::lex_line("a card named \"Portrait with the charted status\" in exile without the \"ready status, now\" status",0).unwrap();
        let (base,label,negative)=filter_clause(&tokens).unwrap();
        assert!(negative);assert_eq!(label,"ready status, now");
        assert_eq!(crate::lexer::render_literal_token_slice(&base),"a card named \"Portrait with the charted status\" in exile");
    }
}
