#[test]
fn p2_probe() {
    use crate::effect_sentences::parse_create;
    for text in [
        "Create Tamiyo's Notebook, a legendary colorless Book artifact token with \"Spells you cast cost {2} less to cast\" and \"{T}: Draw a card.\"",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let ast = parse_create(&tokens, None).unwrap();
        if let crate::cards::builders::EffectAst::SubjectVerb(sv) = &ast
            && let crate::cards::builders::SubjectVerbActionAst::Tokens(crate::cards::builders::TokenActionAst::CreateTokenWithMods { definition, granted_abilities, .. }) = &sv.action
        {
            eprintln!("P2PROBE createdef {text}\n{definition:#?}\nGRANTED {granted_abilities:#?}");
        }
    }
    let text = "Type: Creature — Bird\nFlying\nWhen this creature enters, look at the top two cards of your library, then put one onto the battlefield face down as a 2/2 creature with ward {2} and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.\nPower/Toughness: 1/1";
    let (actual, trace) = crate::parse_trace::capture(|| crate::parse_card_text(crate::CardDefinitionBuilder::new(ironsmith_core::ids::CardId(1), "Reminder Test"), text));
    eprintln!("P2PROBE reminder {:#?}\n{}", actual.map(|d| d.abilities.iter().map(|a| format!("{:?}", a.kind).chars().take(300).collect::<String>()).collect::<Vec<_>>()), trace.render());
}

#[test]
fn p2_probe2() {
    for text in ["a land", "legendary permanent card", "a land and/or legendary permanent card"] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        eprintln!("P2PROBE filter {text} => {:?}", crate::object_filters::parse_object_filter(&tokens, false).map(|f| f.description()));
    }
    for text in [
        "Each player mills three cards, then each opponent discards a card and you draw a card.",
        "You may cast any number of spells from among cards exiled with this artifact with total mana value X or less without paying their mana costs.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let (result, trace) = crate::parse_trace::capture(|| crate::clause_support::parse_effect_sentences_lexed(&tokens).map(|e| format!("{e:?}").chars().take(1500).collect::<String>()));
        eprintln!("P2PROBE seq {text} => {result:?}\n{}", trace.render());
    }
    let tokens = crate::lexer::lex_line("You may cast any number of spells from among cards exiled with this artifact with total mana value X or less without paying their mana costs.", 0).unwrap();
    let (result, trace) = crate::parse_trace::capture(|| crate::permission_helpers::collection_casts::parse_collection_cast_clause(&tokens).map(|e| format!("{e:?}").chars().take(500).collect::<String>()));
    eprintln!("P2PROBE collcast => {result:?}\n{}", trace.render());
}

#[test]
fn p2_probe3() {
    for text in [
        "You and this creature each have flying.",
        "Each instant and sorcery card in your graveyard has flashback. The flashback cost is equal to that card's mana cost.",
        "Each nonland card in your hand without foretell has foretell. Its foretell cost is equal to its mana cost reduced by {2}.",
        "The top card of your library has plot. The plot cost is equal to its mana cost.",
        "Enchanted creature has protection from each color among permanents you control,. This effect doesn't remove this Aura.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let (result, trace) = crate::parse_trace::capture(|| crate::keyword_static::parse_granted_keyword_static_line(&tokens).map(|e| format!("{e:?}").chars().take(600).collect::<String>()));
        eprintln!("P2PROBE kw {text} => {result:?}\n{}", trace.render());
    }
}
