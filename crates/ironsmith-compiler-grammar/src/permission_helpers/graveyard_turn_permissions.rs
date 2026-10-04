//! Exact persistent recipient riders on a completed source-zone permission.
use super::*;
use crate::grammar::primitives;
use crate::lexer::LexStream;
use winnow::combinator::{peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

fn quote<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &OwnedLexToken| token.is_quote()).void().parse_next(input)
}
fn quoted_rider<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    quote(input)?;
    let body = repeat_till(1.., any.void(), peek(quote)).map(|((), ())| ()).take().parse_next(input)?;
    quote(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(body)
}
fn repeated_origin<'a>(input: &mut LexStream<'a>) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["you", "may", "play"]).parse_next(input)?;
    let land = super::filtered_zone_permissions::subject_before_from(input)?;
    primitives::phrase(&["from", "your", "graveyard", "or", "cast"]).parse_next(input)?;
    let spell = super::filtered_zone_permissions::subject_before_from(input)?;
    primitives::phrase(&["from", "your", "graveyard"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((land, spell))
}
fn permission(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    if let Some(spec) = super::filtered_zone_permissions::parse_filtered_zone_permission(tokens)? { return Ok(Some(spec)); }
    let Some((land, spell)) = primitives::probe_all(tokens, repeated_origin, "shared-repeated-graveyard-origin") else { return Ok(None); };
    let Some(mut land) = super::filtered_zone_permissions::card_filter(land)? else { return Ok(None); };
    if !land.card_types.contains(&CardType::Land) { return Ok(None); }
    land.card_types = vec![CardType::Land];
    let Some(mut spell) = super::filtered_zone_permissions::card_filter(spell)? else { return Ok(None); };
    exclude_lands_from_spell_filter(&mut spell);
    let filter = ObjectFilter { any_of: vec![land, spell], owner: Some(PlayerFilter::You), ..Default::default() };
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::play_from(), filter, Zone::Graveyard);
    spec.usage_limit = Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns);
    Ok(Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static}))
}

pub(super) fn parse_permanent_permission_rider(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some((separator, _, tail)) = primitives::find_prefix(tokens, || (
        primitives::period(), primitives::phrase(&["if", "you", "do"]), primitives::comma(),
        primitives::phrase(&["it", "gains"]),
    )) else { return Ok(None); };
    let Some(body) = primitives::probe_all(tail, quoted_rider, "quoted-permission-recipient-rider") else { return Ok(None); };
    let Some(PermissionClauseSpec::GrantBySpec {player, mut spec, lifetime: PermissionLifetime::Static}) = permission(&tokens[..separator])?
        else { return Ok(None); };
    if spec.zone != Zone::Graveyard || spec.usage_limit != Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns) { return Ok(None); }
    let leave = primitives::probe_all(body, (
        primitives::phrase(&["if", "this", "permanent", "would", "leave", "the", "battlefield"]), primitives::comma(),
        primitives::phrase(&["exile", "it", "instead", "of", "putting", "it", "anywhere", "else"]), primitives::sentence_end(),
    ).void(), "permission-exile-on-leaving").is_some();
    let rider = if leave {
        StaticAbility::redirect_zone_change(ObjectFilter::source(), Some(Zone::Battlefield), None, Zone::Exile)
    } else {
        let death = primitives::probe_all(body, (
            primitives::phrase(&["when", "this", "permanent", "is", "put", "into", "a", "graveyard", "from", "the", "battlefield"]), primitives::comma(),
            primitives::phrase(&["exile", "it", "and", "you", "gain"]),
            crate::grammar::leaf::parse_leaf_number_prefix_lexed,
            primitives::kw("life"), primitives::sentence_end(),
        ).void(), "permission-exile-and-life-death-trigger").is_some();
        if !death { return Ok(None); }
        let words = crate::lexer::parser_token_word_refs(body);
        let Some(parsed) = crate::effect_sentences::parse_granted_activated_or_triggered_ability_for_gain(body, &words)? else { return Ok(None); };
        let ability = crate::static_ability_helpers::compiler_granted_ability_ast_to_object_ability(&parsed)?;
        crate::model::CompilerStaticAbilityCore::grant_object_ability_for_filter(ObjectFilter::source(), ability, crate::lexer::render_token_slice(body))
    };
    spec.permanent_this_way_grants.push(rider);
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.'); let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec {player, spec, lifetime: PermissionLifetime::Static}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(text: &str) -> crate::model::CompilerGrantSpecCore {
        let Some(PermissionClauseSpec::GrantBySpec {spec, ..}) = parse_permanent_permission_rider(&crate::lexer::lex_line(text, 0).unwrap()).unwrap() else { panic!("{text}"); };
        spec
    }
    #[test]
    fn repeated_origin_and_common_origin_keep_one_persistent_rider_and_budget() {
        let serra = spec("Once during each of your turns, you may play a land from your graveyard or cast a permanent spell with mana value 3 or less from your graveyard. If you do, it gains \"When this permanent is put into a graveyard from the battlefield, exile it and you gain 2 life.\"");
        assert_eq!(serra.filter.any_of.len(), 2);
        assert_eq!(serra.permanent_this_way_grants.len(), 1); assert!(serra.cast_this_way_grants.is_empty());
        assert_eq!(serra.usage_limit, Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns));
        let doctor = spec("Once during each of your turns, you may play a historic land or cast a historic permanent spell from your graveyard. If you do, it gains \"If this permanent would leave the battlefield, exile it instead of putting it anywhere else.\"");
        assert_eq!(doctor.filter.any_of.len(), 2); assert_eq!(doctor.permanent_this_way_grants.len(), 1);
        assert!(matches!(doctor.permanent_this_way_grants[0].payload,
            ironsmith_core::StaticAbilityPayload::RedirectZoneChange {from_zone: Some(Zone::Battlefield), to_zone: None, destination: Zone::Exile, ..}));
    }
    #[test]
    fn history_qualifiers_are_distinct_and_unknown_rider_tails_do_not_disappear() {
        for (tail, mill) in [("milled this turn", true), ("put there from your library this turn", false)] {
            let text = format!("Once during each of your turns, you may cast a spell from among cards in your graveyard that were {tail}.");
            let Some(PermissionClauseSpec::GrantBySpec {spec, ..}) = super::super::filtered_zone_permissions::parse_recent_graveyard_permission(&crate::lexer::lex_line(&text, 0).unwrap()).unwrap() else { panic!(); };
            assert_eq!(spec.filter.milled_into_graveyard_this_turn, mill);
            assert_eq!(spec.filter.entered_graveyard_from_library_this_turn, !mill);
        }
        for text in [
            "Once during each of your turns, you may cast a spell from among cards in your graveyard that were milled last turn.",
            "Once during each of your turns, you may cast a spell from among cards in your graveyard that were milled this turn and draw a card.",
        ] { assert!(super::super::filtered_zone_permissions::parse_recent_graveyard_permission(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().is_none()); }
        assert!(parse_permanent_permission_rider(&crate::lexer::lex_line("Once during each of your turns, you may play a land or cast a permanent spell from your graveyard. If you do, it gains \"If this permanent would leave the battlefield, exile it instead of putting it anywhere else and draw a card.\"", 0).unwrap()).unwrap().is_none());
    }
}
