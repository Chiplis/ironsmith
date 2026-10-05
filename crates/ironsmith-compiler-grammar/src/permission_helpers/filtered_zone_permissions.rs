//! Complete source-wide filtered play/cast permission clauses.
use super::*;
use crate::grammar::primitives;
use crate::lexer::LexStream;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone)]
struct PermissionShape<'a> {
    play: bool,
    subject: Option<&'a [OwnedLexToken]>,
    zone: Zone,
    top_only: bool,
    instant_timing: bool,
    usage: Option<crate::grant::GrantUsageLimit>,
}
fn subject_before_from<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::kw("from")))
        .map(|((), ())| ()).take().parse_next(input)
}
fn permission_end<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        (
            primitives::period(),
            primitives::phrase(&["if", "you", "cast", "a", "spell", "this", "way"]),
            primitives::comma(),
            primitives::phrase(&["you", "may", "cast", "it", "as", "though", "it", "had", "flash"]),
            primitives::sentence_end(),
        ).value(true),
        primitives::sentence_end().value(false),
    )).parse_next(input)
}
fn shape<'a>(input: &mut LexStream<'a>) -> WResult<PermissionShape<'a>> {
    let usage = opt(alt((
        primitives::phrase(&["once", "each", "turn"]).value(crate::grant::GrantUsageLimit::OnceEachTurn),
        primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).value(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
    ))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may"]).parse_next(input)?;
    let play = alt((primitives::kw("play").value(true), primitives::kw("cast").value(false))).parse_next(input)?;
    let mut bare = input.clone();
    if play && primitives::phrase(&["the", "top", "card", "of", "your", "library"]).parse_next(&mut bare).is_ok()
        && let Ok(instant_timing) = permission_end(&mut bare)
    {
        *input = bare;
        return Ok(PermissionShape { play, subject: None, zone: Zone::Library, top_only: true, instant_timing, usage });
    }
    let subject = subject_before_from(input)?;
    primitives::kw("from").parse_next(input)?;
    let (zone, top_only) = alt((
        primitives::phrase(&["the", "top", "of", "your", "library"]).value((Zone::Library, true)),
        primitives::phrase(&["your", "graveyard"]).value((Zone::Graveyard, false)),
    )).parse_next(input)?;
    let instant_timing = permission_end(input)?;
    Ok(PermissionShape { play, subject: Some(subject), zone, top_only, instant_timing, usage })
}
fn action_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("and"), primitives::kw("or"))).parse_next(input)?;
    primitives::kw("cast").parse_next(input)
}
fn names_land_domain(tokens: &[OwnedLexToken]) -> bool {
    crate::lexer::parser_token_word_refs(tokens).iter().any(|word|
        matches!(*word, "land" | "lands" | "forest" | "forests" | "island" | "islands" | "plains" | "swamp" | "swamps" | "mountain" | "mountains"))
}
fn card_filter(tokens: &[OwnedLexToken]) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(mut filter) = permission_subject_facts::parse_permission_subject_filter_tokens(tokens)? else { return Ok(None); };
    // These nouns describe the proposed card face, not an object already on
    // the battlefield/stack. Keep nested comparison domains intact.
    if matches!(filter.zone, Some(Zone::Battlefield | Zone::Stack)) { filter.zone = None; }
    filter.stack_kind = None;
    Ok(Some(filter))
}

pub(super) fn parse_filtered_zone_permission(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(shape) = primitives::probe_all(tokens, shape, "filtered-zone-play-cast-permission") else { return Ok(None); };
    let mut filter = if let Some(subject) = shape.subject {
        if shape.play {
            if let Some((separator, _, spell_tokens)) = primitives::find_prefix(subject, || action_separator) {
                let land_tokens = &subject[..separator];
                if !names_land_domain(land_tokens) { return Ok(None); }
                let Some(mut lands) = card_filter(land_tokens)? else { return Ok(None); };
                lands.card_types = vec![CardType::Land];
                let Some(mut spells) = card_filter(spell_tokens)? else { return Ok(None); };
                exclude_lands_from_spell_filter(&mut spells);
                ObjectFilter { any_of: vec![lands, spells], ..Default::default() }
            } else {
                let Some(mut lands) = card_filter(subject)? else { return Ok(None); };
                // The play-only spelling here must name land cards. General
                // tagged-card permissions remain owned by their own grammar.
                if !names_land_domain(subject) { return Ok(None); }
                lands.card_types = vec![CardType::Land]; lands
            }
        } else {
            let Some(mut spells) = card_filter(subject)? else { return Ok(None); };
            exclude_lands_from_spell_filter(&mut spells); spells
        }
    } else { ObjectFilter::default() };
    filter.owner = Some(PlayerFilter::You);
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::play_from(), filter, shape.zone);
    spec.usage_limit = shape.usage; spec.top_card_only = shape.top_only;
    spec.instant_timing = shape.instant_timing;
    let surface = crate::lexer::render_token_slice(tokens);
    let surface = surface.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec { player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static }))
}

pub(crate) fn parse_top_look_and_permission(tokens: &[OwnedLexToken]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some((_, permission)) = primitives::parse_prefix(tokens, (
        primitives::phrase(&["you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time"]),
        primitives::comma(), primitives::kw("and"),
    )) else { return Ok(None); };
    let Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static}) = parse_filtered_zone_permission(permission)?
        else { return Ok(None); };
    if spec.zone != Zone::Library || !spec.top_card_only { return Ok(None); }
    Ok(Some(vec![StaticAbility::look_at_top_card_of_library(), StaticAbility::grants(spec)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> crate::model::CompilerGrantSpecCore {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let Some(PermissionClauseSpec::GrantBySpec {spec, ..}) = parse_filtered_zone_permission(&tokens).unwrap() else { panic!("{text}"); };
        spec
    }
    #[test]
    fn full_filters_and_origins_are_retained_in_one_permission() {
        let shared = parse("You may play historic lands and cast historic spells from the top of your library.");
        assert!(shared.top_card_only); assert_eq!(shared.zone, Zone::Library); assert_eq!(shared.filter.any_of.len(), 2);
        assert_eq!(shared.filter.owner, Some(PlayerFilter::You));
        let insects = parse("You may play lands and cast Insect spells from your graveyard.");
        assert!(!insects.top_card_only); assert_eq!(insects.zone, Zone::Graveyard);
        assert!(insects.filter.any_of[1].subtypes.contains(&crate::types::Subtype::Insect));
        let forests = parse("You may play Forests from your graveyard.");
        assert!(forests.filter.subtypes.contains(&crate::types::Subtype::Forest));
    }
    #[test]
    fn each_turn_usage_and_power_filter_are_not_erased() {
        let spec = parse("Once each turn, you may cast a creature spell with power 2 or less from the top of your library.");
        assert_eq!(spec.usage_limit, Some(crate::grant::GrantUsageLimit::OnceEachTurn));
        assert!(spec.filter.power.is_some()); assert!(spec.top_card_only);
        let bare = parse("You may play the top card of your library.");
        assert!(bare.top_card_only); assert!(bare.filter.card_types.is_empty());
    }
    #[test]
    fn flash_applies_to_this_permission_and_compound_look_keeps_both_members() {
        let spec = parse("You may cast noncreature spells from the top of your library. If you cast a spell this way, you may cast it as though it had flash.");
        assert!(spec.instant_timing && spec.top_card_only);
        let look = crate::lexer::lex_line("You may look at the top card of your library any time, and you may play lands and cast creature and enchantment spells from the top of your library.", 0).unwrap();
        let members = parse_top_look_and_permission(&look).unwrap().unwrap();
        assert_eq!(members.len(), 2);
    }
    #[test]
    fn unknown_riders_and_other_owners_are_not_partially_claimed() {
        for text in ["You may play lands from your graveyard unless you pay 2 life.",
            "You may cast spells from an opponent's graveyard.",
            "Once each turn, you may play a historic land or cast a historic spell from the top of your library. When you do, create a Food token."] {
            assert!(parse_filtered_zone_permission(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().is_none(), "{text}");
        }
    }
}
