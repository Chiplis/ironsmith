//! "If the top card of target player's graveyard is a creature card, put that
//! card on top of that player's library." (Guiding Spirit): the target player
//! is declared, the card on top of their graveyard (CR 404.2 keeps graveyard
//! order) is looked up as the ability resolves, and only when that card has
//! the stated quality does it move to the top of its owner's library. The
//! quality filters the top card; it never selects a deeper matching card.
use crate::cards::builders::{
    ConditionalEffectAst, EffectAst, ObjectChoiceEffectAst, OwnedLexToken, PlayerAst,
    PredicateAst, ReturnControllerAst, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::lexer::token_word_refs;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

fn strip_target_players<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    words
        .strip_prefix(&["target", "player's"][..])
        .or_else(|| words.strip_prefix(&["target", "players"][..]))
        .or_else(|| words.strip_prefix(&["target", "player", "s"][..]))
}

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let comma = tokens.iter().position(OwnedLexToken::is_comma)?;
    let (condition, body) = (&tokens[..comma], &tokens[comma + 1..]);
    let condition_words = token_word_refs(condition);
    let rest = condition_words.strip_prefix(&["if", "the", "top", "card", "of"][..])?;
    let rest = strip_target_players(rest)?;
    let quality = rest.strip_prefix(&["graveyard", "is"][..])?;
    if !matches!(quality.first(), Some(&("a" | "an"))) {
        return None;
    }
    // The quality's tokens: everything in the condition after "is".
    let quality_start = condition
        .iter()
        .rposition(|token| token.is_word("is"))?
        + 1;
    let mut quality_filter =
        crate::object_filters::parse_object_filter(&condition[quality_start..], false).ok()?;
    // The quality describes the graveyard card itself; a battlefield default
    // from the bare "a creature card" noun would never match it.
    if quality_filter.zone == Some(Zone::Battlefield) {
        quality_filter.zone = None;
    }

    let body_words = token_word_refs(crate::util::trim_edge_punctuation_tokens(body));
    if !matches!(
        body_words.as_slice(),
        ["put", "that", "card", "on", "top", "of", "that", "player's" | "players", "library"]
            | ["put", "that", "card", "on", "top", "of", "that", "player", "s", "library"]
            | ["put", "it", "on", "top", "of", "that", "player's" | "players", "library"]
            | ["put", "that", "card", "on", "top", "of", "its", "owner's" | "owners", "library"]
    ) {
        return None;
    }

    let player_tokens_start = condition
        .iter()
        .position(|token| token.is_word("target"))?;
    let player_tokens_end = condition
        .iter()
        .position(|token| token.is_word("graveyard"))?;
    let target_player = TargetAst::Player(
        PlayerFilter::Any,
        crate::util::span_from_tokens(&condition[player_tokens_start..player_tokens_end]),
    );
    let mut graveyard_card = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_card.owner = Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    let top = crate::util::helper_tag_for_tokens(tokens, "graveyard_top");
    Some(vec![
        EffectAst::subject_verb_target_only(target_player),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone {
            filter: graveyard_card,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: top.clone(),
        }),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(top.clone(), quality_filter),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(top, None),
                Zone::Library,
                true,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
            if_false: Vec::new(),
        }),
    ])
}
