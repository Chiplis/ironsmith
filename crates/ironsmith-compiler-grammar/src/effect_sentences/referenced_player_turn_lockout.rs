//! "Until end of turn, that player can't cast spells, you may look at the top
//! card of their library any time, you may play the top card of their
//! library, and you may spend mana as though it were mana of any color to
//! cast spells this way." (Xanathar, Guild Kingpin): one turn-long bundle on
//! the player the instruction is talking about. The player loses the right to
//! cast spells (CR 101.2), and the controller gains a resolving play
//! permission over that player's current library top with a continuous
//! private view of each top (CR 401.2 / CR 609.4b mana rider belonging to
//! this exact permission). The owner is the discourse player ("that player"
//! / "their"); the engine locks it to the concrete player when the grant is
//! created (CR 608.2h).
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;
use crate::zone::Zone;

const LOOK: &[&str] = &[
    "you", "may", "look", "at", "the", "top", "card", "of", "their", "library", "any", "time",
];
const PLAY: &[&str] = &["you", "may", "play", "the", "top", "card", "of", "their", "library"];
const ANY_COLOR: &[&str] = &[
    "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any", "color",
    "to", "cast", "spells", "this", "way",
];

fn strip_list_item<'a>(
    words: &'a [&'a str],
    item: &'static [&'static str],
) -> Option<&'a [&'a str]> {
    let words = words.strip_prefix(&["and"][..]).unwrap_or(words);
    words.strip_prefix(item)
}

/// "This turn, that player can't cast spells or activate abilities and plays
/// with their hand revealed." (Sen Triplets): the turn-long lockout of the
/// discourse player plus a public reveal of their hand for the same turn.
fn parse_lockout_with_hand_reveal(words: &[&str]) -> Option<Vec<EffectAst>> {
    let body = words
        .strip_prefix(&["until", "end", "of", "turn"][..])
        .or_else(|| words.strip_prefix(&["this", "turn"][..]))?;
    let rest = body.strip_prefix(&["that", "player", "cant"][..])?;
    let (restrictions, rest) = if let Some(rest) =
        rest.strip_prefix(&["cast", "spells", "or", "activate", "abilities"][..])
    {
        (
            vec![
                crate::effect::Restriction::cast_spells(PlayerFilter::IteratedPlayer),
                crate::effect::Restriction::activate_abilities(PlayerFilter::IteratedPlayer),
            ],
            rest,
        )
    } else if let Some(rest) = rest.strip_prefix(&["cast", "spells"][..]) {
        (
            vec![crate::effect::Restriction::cast_spells(PlayerFilter::IteratedPlayer)],
            rest,
        )
    } else {
        return None;
    };
    if !matches!(
        rest,
        ["and", "plays", "with", "their", "hand", "revealed"]
            | ["and", "play", "with", "their", "hand", "revealed"]
    ) {
        return None;
    }
    let mut effects: Vec<EffectAst> = restrictions
        .into_iter()
        .map(|restriction| {
            EffectAst::subject_verb_cant(restriction, crate::effect::Until::EndOfTurn, None)
        })
        .collect();
    effects.push(EffectAst::PlayWithHandRevealed(
        ironsmith_core::PlayWithHandRevealedEffect::this_turn(PlayerFilter::IteratedPlayer),
    ));
    Some(effects)
}

/// "Until that player's next turn, that player plays with that card revealed
/// in their hand and can't play it." (Firestorm Phoenix, after "return it to
/// its owner's hand instead"): the returned source card stays revealed in its
/// owner's hand, and unplayable, until that player's next turn.
fn parse_source_card_revealed_until_owners_next_turn(words: &[&str]) -> Option<Vec<EffectAst>> {
    let words: Vec<&str> = words.iter().copied().filter(|word| *word != ",").collect();
    let rest = words
        .strip_prefix(&["until", "that", "player's", "next", "turn"][..])
        .or_else(|| words.strip_prefix(&["until", "that", "players", "next", "turn"][..]))?;
    let rest = rest.strip_prefix(&["that", "player", "plays", "with", "that", "card", "revealed", "in"][..])?;
    let rest = match rest {
        ["their" | "his" | "her", "hand", rest @ ..] => rest,
        ["that", "player's" | "players", "hand", rest @ ..] => rest,
        ["his", "or", "her", "hand", rest @ ..] => rest,
        _ => return None,
    };
    let cant_play = match rest {
        [] => false,
        ["and", "cant" | "can't", "play", "it"] => true,
        _ => return None,
    };
    Some(vec![EffectAst::RevealSourceInHandUntilOwnersNextTurn(
        ironsmith_core::RevealSourceInHandUntilOwnersNextTurnEffect::new(cant_play),
    )])
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::parser_token_word_refs(tokens);
    if let Some(effects) = parse_lockout_with_hand_reveal(&words) {
        return Ok(Some(effects));
    }
    if let Some(effects) = parse_source_card_revealed_until_owners_next_turn(&words) {
        return Ok(Some(effects));
    }
    // The whole bundle shares one authored turn bound.
    let Some(body) = words
        .strip_prefix(&["until", "end", "of", "turn"][..])
        .or_else(|| words.strip_prefix(&["this", "turn"][..]))
    else {
        return Ok(None);
    };
    let Some(rest) = body.strip_prefix(&["that", "player", "cant", "cast", "spells"][..]) else {
        return Ok(None);
    };
    let Some(rest) = strip_list_item(rest, LOOK) else {
        return Ok(None);
    };
    let Some(rest) = strip_list_item(rest, PLAY) else {
        return Ok(None);
    };
    let mana_spend_mode = if rest.is_empty() {
        ironsmith_core::value_model::ManaSpendMode::Normal
    } else if strip_list_item(rest, ANY_COLOR).is_some_and(|tail| tail.is_empty()) {
        ironsmith_core::value_model::ManaSpendMode::AnyColor
    } else {
        return Ok(None);
    };

    let restriction = EffectAst::subject_verb_cant(
        crate::effect::Restriction::cast_spells(PlayerFilter::IteratedPlayer),
        crate::effect::Until::EndOfTurn,
        None,
    );

    let filter = crate::target::ObjectFilter {
        owner: Some(PlayerFilter::IteratedPlayer),
        ..Default::default()
    };
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Library,
    );
    spec.top_card_only = true;
    spec.may_look_at_top = true;
    spec.cast_mana_spend_mode = mana_spend_mode;
    // The permission surface starts at its own first member ("you may look").
    if let Some(start) = tokens.iter().position(|token| token.is_word("you")) {
        let rendered = crate::lexer::render_token_slice(&tokens[start..]);
        let surface = rendered.trim().trim_end_matches('.');
        let mut chars = surface.chars();
        spec.filtered_zone_surface = chars
            .next()
            .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    }
    let grant = EffectAst::subject_verb_grant_by_spec(
        spec,
        PlayerAst::You,
        crate::grant::GrantDuration::UntilEndOfTurn,
    );
    Ok(Some(vec![restriction, grant]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xanathar_bundle_reads_restriction_and_top_permission() {
        let tokens = crate::lexer::lex_line(
            "Until end of turn, that player can't cast spells, you may look at the top card of their library any time, you may play the top card of their library, and you may spend mana as though it were mana of any color to cast spells this way.",
            0,
        )
        .unwrap();
        let effects = parse(&tokens).unwrap().expect("bundle");
        assert_eq!(effects.len(), 2);
        let debug = format!("{effects:?}");
        assert!(debug.contains("AnyColor"), "{debug}");
        assert!(debug.contains("IteratedPlayer"), "{debug}");
    }

    #[test]
    fn xanathar_bundle_rejects_unknown_tail() {
        let tokens = crate::lexer::lex_line(
            "Until end of turn, that player can't cast spells, you may look at the top card of their library any time, you may play the top card of their library, and you draw a card.",
            0,
        )
        .unwrap();
        assert!(parse(&tokens).unwrap().is_none());
    }
}
