//! One static permission with several origins and one shared use identity.
use super::*;
use crate::grammar::primitives;
use crate::lexer::LexStream;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult;
use winnow::prelude::*;

fn origin(input: &mut LexStream<'_>) -> ModalResult<Zone> {
    alt((
        primitives::phrase(&["outside", "the", "game"]).value(Zone::OutsideGame),
        primitives::phrase(&["your", "graveyard"]).value(Zone::Graveyard),
        primitives::phrase(&["your", "hand"]).value(Zone::Hand),
        primitives::kw("exile").value(Zone::Exile),
    )).parse_next(input)
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(((usage, play), rest)) = primitives::parse_prefix(tokens, (
        alt((
            primitives::phrase(&["once", "during", "each", "of", "your", "turns"])
                .value(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
            primitives::phrase(&["once", "each", "turn"])
                .value(crate::grant::GrantUsageLimit::OnceEachTurn),
        )),
        opt(primitives::comma()),
        primitives::phrase(&["you", "may"]),
        alt((primitives::kw("play").value(true), primitives::kw("cast").value(false))),
    ).map(|(usage, _, _, play)| (usage, play))) else { return Ok(None); };
    let Some((separator, _, origins)) = primitives::find_prefix(rest, || primitives::kw("from")) else {
        return Ok(None);
    };
    let Some((first, second)) = primitives::probe_all(origins, (
        origin, primitives::kw("or"), opt(primitives::kw("from")), origin,
        primitives::sentence_end(),
    ).map(|(first, _, _, second, _)| (first, second)), "shared-zone-permission") else { return Ok(None); };
    let Some(mut filter) = super::filtered_zone_permissions::card_filter(&rest[..separator])? else {
        return Ok(None);
    };
    if filter.owner.as_ref().is_some_and(|owner| *owner != PlayerFilter::You) {
        return Ok(None);
    }
    filter.owner = Some(PlayerFilter::You);
    if !play { exclude_lands_from_spell_filter(&mut filter); }
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(), filter, first);
    spec.additional_zones = vec![second];
    spec.usage_limit = Some(usage);
    let surface = crate::lexer::render_token_slice(tokens);
    let mut chars = surface.trim().trim_end_matches('.').chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec {
        player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_origins_keep_one_permission_and_own_turn_limit() {
        let tokens = crate::lexer::lex_line("Once during each of your turns, you may play a permanent card you own from outside the game or from your graveyard.", 0).unwrap();
        let Some(PermissionClauseSpec::GrantBySpec { spec, .. }) = parse(&tokens).unwrap() else {
            panic!("shared origin permission");
        };
        assert_eq!(spec.zone, Zone::OutsideGame);
        assert_eq!(spec.additional_zones, vec![Zone::Graveyard]);
        assert_eq!(spec.usage_limit, Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns));
        assert_eq!(spec.filter.owner, Some(PlayerFilter::You));
    }

    #[test]
    fn shared_origins_require_complete_supported_clause() {
        let tokens = crate::lexer::lex_line("Once each turn, you may cast a creature card from your graveyard or from exile without paying its mana cost.", 0).unwrap();
        assert!(parse(&tokens).unwrap().is_none(), "a cost rider cannot be discarded");
    }
}
