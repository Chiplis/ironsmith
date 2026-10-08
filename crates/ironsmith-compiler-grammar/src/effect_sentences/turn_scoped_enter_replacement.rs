//! "Until end of turn, if a [nontoken] creature would enter [and it wasn't
//! cast], exile it instead." (Hallowed Moonlight, Mistcaller): a resolving
//! instruction that creates a replacement effect lasting for the turn
//! (CR 614.1c, 614.12). Never a one-shot exile of an antecedent.
use crate::cards::builders::{EffectAst, FutureZoneReplacementCausePolicyAst, ZoneReplacementDurationAst};
use crate::lexer::OwnedLexToken;
use crate::target::ObjectFilter;
use crate::zone::Zone;

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let rest = words.strip_prefix(&["until", "end", "of", "turn", "if", "a"][..])?;
    let (nontoken, rest) = match rest.strip_prefix(&["nontoken"][..]) {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let rest = rest.strip_prefix(&["creature", "would", "enter"][..])?;
    let rest = rest
        .strip_prefix(&["the", "battlefield"][..])
        .unwrap_or(rest);
    let (not_cast, rest) = match rest {
        ["and", "it", "wasn't" | "wasnt", "cast", rest @ ..] => (true, rest),
        _ => (false, rest),
    };
    if rest != ["exile", "it", "instead"] {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.card_types = vec![crate::types::CardType::Creature];
    filter.nontoken = nontoken;
    if not_cast {
        // A cast creature enters from the stack as its spell resolves; every
        // other entry comes from a non-stack zone.
        filter.any_of = [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile, Zone::Command]
            .into_iter()
            .map(|zone| {
                let mut branch = ObjectFilter::default();
                branch.zone = Some(zone);
                branch
            })
            .collect();
    }
    Some(EffectAst::subject_verb_register_future_zone_replacement(
        filter,
        None,
        Some(Zone::Battlefield),
        Zone::Exile,
        ZoneReplacementDurationAst::UntilEndOfTurn,
        FutureZoneReplacementCausePolicyAst::Any,
        false,
    ))
}
