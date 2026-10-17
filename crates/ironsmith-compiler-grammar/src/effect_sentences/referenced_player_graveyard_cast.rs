//! "you may cast a creature spell from that player's graveyard this turn[, and
//! you may spend mana as though it were mana of any color to cast that
//! spell]" (Whispersteel Dagger): a resolving one-spell permission over the
//! graveyard of the player the instruction is talking about. The singular
//! subject is one shared use (CR 601.2), the turn bound is the grant lifetime,
//! and the mana rider belongs to this exact permission (CR 609.4b). The owner
//! is the discourse player ("that player"); the engine locks it to the
//! concrete player when the grant is created (CR 608.2h).
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, trim_lexed_commas};
use crate::target::PlayerFilter;
use crate::zone::Zone;
use winnow::Parser as _;
use winnow::combinator::alt;

/// "You may play lands and cast spells from other players' graveyards this
/// turn as though those cards were in your graveyard." (Shaman's Trance): a
/// turn-long play permission over every graveyard but yours, without a use
/// budget.
fn parse_other_players_graveyards(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let rest = words.strip_prefix(&[
        "you", "may", "play", "lands", "and", "cast", "spells", "from", "other", "players",
        "graveyards", "this", "turn",
    ][..])?;
    if !(rest.is_empty()
        || rest == ["as", "though", "those", "cards", "were", "in", "your", "graveyard"])
    {
        return None;
    }
    let lands = crate::target::ObjectFilter::default().with_type(crate::types::CardType::Land);
    let mut spells = crate::target::ObjectFilter::default();
    spells.excluded_card_types.push(crate::types::CardType::Land);
    let filter = crate::target::ObjectFilter {
        any_of: vec![lands, spells],
        owner: Some(PlayerFilter::NotYou),
        ..Default::default()
    };
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Graveyard,
    );
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Some(EffectAst::subject_verb_grant_by_spec(
        spec,
        PlayerAst::You,
        crate::grant::GrantDuration::UntilEndOfTurn,
    ))
}

/// "You may play lands and cast spells from that player's hand this turn."
/// (Sen Triplets): a turn-long play permission over the discourse player's
/// hand, which the same instruction reveals; the engine locks the owner to
/// the concrete player when the grant is created (CR 608.2h).
fn parse_referenced_player_hand(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let rest = words.strip_prefix(&[
        "you", "may", "play", "lands", "and", "cast", "spells", "from", "that",
    ][..])?;
    let rest = rest
        .strip_prefix(&["players"][..])
        .or_else(|| rest.strip_prefix(&["player's"][..]))
        .or_else(|| rest.strip_prefix(&["player", "s"][..]))?;
    if rest != ["hand", "this", "turn"] {
        return None;
    }
    let lands = crate::target::ObjectFilter::default().with_type(crate::types::CardType::Land);
    let mut spells = crate::target::ObjectFilter::default();
    spells.excluded_card_types.push(crate::types::CardType::Land);
    let filter = crate::target::ObjectFilter {
        any_of: vec![lands, spells],
        owner: Some(PlayerFilter::IteratedPlayer),
        ..Default::default()
    };
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Hand,
    );
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Some(EffectAst::subject_verb_grant_by_spec(
        spec,
        PlayerAst::You,
        crate::grant::GrantDuration::UntilEndOfTurn,
    ))
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if let Some(effect) = parse_other_players_graveyards(tokens) {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_referenced_player_hand(tokens) {
        return Ok(Some(effect));
    }
    let mut body = tokens;
    let mut mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::Normal;
    if let Some(fact) =
        crate::grammar::permission_facts::tagged_surface::parse_allow_any_color_for_cast_suffix_tokens(tokens)
    {
        use crate::grammar::permission_facts::tagged_surface::ManaSpendCastReference;
        if !matches!(fact.reference, ManaSpendCastReference::ThatSpell | ManaSpendCastReference::It) {
            return Ok(None);
        }
        mana_spend_mode = fact.mana_spend_mode;
        body = trim_lexed_commas(fact.body_tokens);
    }
    let Some(((), rest)) = primitives::parse_prefix(body, primitives::phrase(&["you", "may", "cast"]))
    else {
        return Ok(None);
    };
    // The origin is the last "from <player's> graveyard this turn".
    let Some(from) = rest.iter().rposition(|token| token.is_word("from")) else {
        return Ok(None);
    };
    let subject_tokens = trim_lexed_commas(&rest[..from]);
    let Some(owner) = primitives::probe_all(
        &rest[from..],
        (
            primitives::kw("from").void(),
            alt((
                primitives::phrase(&["that", "players"]).value(PlayerFilter::IteratedPlayer),
                primitives::phrase(&["that", "player"]).value(PlayerFilter::IteratedPlayer),
            )),
            primitives::phrase(&["graveyard", "this", "turn"]),
            primitives::sentence_end(),
        )
            .map(|((), owner, (), ())| owner),
        "referenced-player graveyard turn cast",
    ) else {
        return Ok(None);
    };
    // Only a singular spell subject ("a creature spell") is one shared use.
    let facts = crate::grammar::permission_facts::subject_filters::parse_spell_subject_facts(subject_tokens);
    if !facts.contains_singular_spell || facts.contains_plural_spells {
        return Ok(None);
    }
    if primitives::parse_prefix(subject_tokens, alt((primitives::kw("a"), primitives::kw("an")))).is_none() {
        return Ok(None);
    }
    let Some(mut filter) =
        crate::grammar::permission_facts::subject_filters::parse_cast_permission_filter_tokens(subject_tokens)?
    else {
        return Ok(None);
    };
    if !filter.excluded_card_types.contains(&crate::types::CardType::Land) {
        filter.excluded_card_types.push(crate::types::CardType::Land);
    }
    filter.zone = None;
    filter.owner = Some(owner);
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Graveyard,
    )
    .with_max_plays(1);
    spec.cast_mana_spend_mode = mana_spend_mode;
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(EffectAst::subject_verb_grant_by_spec(
        spec,
        PlayerAst::You,
        crate::grant::GrantDuration::UntilEndOfTurn,
    )))
}
