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
    /// "... from your graveyard with mana value less than or equal to the
    /// number of quest counters on this creature" (Arcade Gannon).
    mana_value: Option<&'a [OwnedLexToken]>,
}
pub(super) fn subject_before_from<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::kw("from").void()))
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
        return Ok(PermissionShape { play, subject: None, zone: Zone::Library, top_only: true, instant_timing, usage, mana_value: None });
    }
    let subject = subject_before_from(input)?;
    primitives::kw("from").parse_next(input)?;
    let (zone, top_only) = alt((
        primitives::phrase(&["the", "top", "of", "your", "library"]).value((Zone::Library, true)),
        // "cast instant and sorcery spells from the top of your graveyard"
        // (Bösium Strip): only the topmost graveyard card qualifies.
        primitives::phrase(&["the", "top", "of", "your", "graveyard"]).value((Zone::Graveyard, true)),
        primitives::phrase(&["your", "graveyard"]).value((Zone::Graveyard, false)),
    )).parse_next(input)?;
    let mana_value = opt((
        primitives::phrase(&["with", "mana", "value"]),
        repeat_till(1.., any.void(), peek(primitives::sentence_end().void()))
            .map(|((), ())| ())
            .take(),
    ))
    .parse_next(input)?
    .map(|(_, tokens)| tokens);
    let instant_timing = permission_end(input)?;
    Ok(PermissionShape { play, subject: Some(subject), zone, top_only, instant_timing, usage, mana_value })
}
fn action_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("and"), primitives::kw("or"))).parse_next(input)?;
    primitives::kw("cast").void().parse_next(input)
}
fn names_land_domain(tokens: &[OwnedLexToken]) -> bool {
    crate::lexer::parser_token_word_refs(tokens).iter().any(|word|
        matches!(*word, "land" | "lands" | "forest" | "forests" | "island" | "islands" | "plains" | "swamp" | "swamps" | "mountain" | "mountains"))
}
pub(super) fn card_filter(tokens: &[OwnedLexToken]) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(mut filter) = permission_subject_facts::parse_permission_subject_filter_tokens(tokens)? else { return Ok(None); };
    // These nouns describe the proposed card face, not an object already on
    // the battlefield/stack. Keep nested comparison domains intact.
    if matches!(filter.zone, Some(Zone::Battlefield | Zone::Stack)) { filter.zone = None; }
    filter.stack_kind = None;
    Ok(Some(filter))
}

/// Both origins are one static permission. Expanding into two abilities would
/// incorrectly give each zone its own once-turn budget.
pub(super) fn parse_shared_hand_top_free_cast(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(()) = primitives::probe_all(tokens, (
        primitives::phrase(&["once", "during", "each", "of", "your", "turns"]), primitives::comma(),
        primitives::phrase(&["you", "may", "cast", "a", "spell", "from", "your", "hand", "or", "the", "top", "of", "your", "library", "without", "paying", "its", "mana", "cost"]),
        primitives::sentence_end(),
    ).void(), "shared-hand-top-free-cast") else { return Ok(None); };
    let mut filter = ObjectFilter::nonland(); filter.owner = Some(PlayerFilter::You);
    let method = ironsmith_core::AlternativeCastingMethod::<EffectAst, crate::model::CompilerCost, ironsmith_core::ThisSpellCostCondition>::cast_from_zone_with_total_cost(
        "Cast without paying mana cost", Zone::Hand, ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(), None, false);
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::AlternativeCast(method), filter, Zone::Hand);
    spec.additional_zones = vec![Zone::Library]; spec.top_card_only = true;
    spec.usage_limit = Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns);
    spec.filtered_zone_surface = Some("Once during each of your turns, you may cast a spell from your hand or the top of your library without paying its mana cost".into());
    Ok(Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static}))
}

/// "Once each turn, you may cast a spell with mana value less than or equal
/// to the number of Elves and Faeries you control from among cards exiled
/// with Maralen this turn without paying its mana cost." (Maralen, Fae
/// Ascendant): one free cast per turn from the source's linked exile pool,
/// optionally limited to the cards you exiled with it this turn. The use
/// budget belongs to this permission (CR 118.9).
pub(super) fn parse_once_per_turn_source_exiled_free_cast(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(((usage, _, _, subject, _), rest)) = primitives::parse_prefix(tokens, (
        alt((
            primitives::phrase(&["once", "each", "turn"]).value(crate::grant::GrantUsageLimit::OnceEachTurn),
            primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).value(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
        )),
        primitives::comma(),
        primitives::phrase(&["you", "may", "cast"]),
        subject_before_from,
        primitives::phrase(&["from", "among"]),
    )) else { return Ok(None); };
    let Some((_, tail)) =
        crate::grammar::permission_facts::source_exiled::parse_cards_from_source_exiled_tokens(rest)
    else { return Ok(None); };
    let (this_turn, tail) = match primitives::parse_prefix(tail, primitives::phrase(&["this", "turn"])) {
        Some((_, after)) => (true, after),
        None => (false, tail),
    };
    let Some(()) = primitives::probe_all(tail, (
        primitives::phrase(&["without", "paying", "its", "mana", "cost"]),
        primitives::sentence_end(),
    ).void(), "once-per-turn-source-exiled-free-cast") else { return Ok(None); };
    let subject_words = crate::lexer::parser_token_word_refs(subject);
    if !subject_words.iter().any(|word| matches!(*word, "spell" | "spells")) {
        return Ok(None);
    }
    // "a spell with mana value <comparison>": the comparison may be dynamic.
    let with_mana_value = subject_words
        .windows(3)
        .position(|window| window == ["with", "mana", "value"]);
    let (noun_tokens, comparison_tokens) = match with_mana_value {
        Some(word_index) => {
            let view = crate::lexer::TokenWordView::new(subject);
            let Some(split) = view.token_index_after_words(word_index) else { return Ok(None); };
            let Some(after) = view.token_index_after_words(word_index + 3) else { return Ok(None); };
            (&subject[..split], Some(&subject[after..]))
        }
        None => (subject, None),
    };
    let Some(mut filter) = card_filter(noun_tokens)? else { return Ok(None); };
    exclude_lands_from_spell_filter(&mut filter);
    if let Some(comparison_tokens) = comparison_tokens {
        let Some((operator, rhs_tokens)) = crate::grammar::values::parse_value_comparison_tokens(comparison_tokens) else { return Ok(None); };
        let Some((rhs_value, used)) = crate::grammar::values::parse_value_prefix_lexed(rhs_tokens) else { return Ok(None); };
        if used != rhs_tokens.len() { return Ok(None); }
        filter.mana_value = Some(super::mana_value_filter_comparison(comparison_tokens, operator, rhs_value));
    }
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    if this_turn {
        filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: ironsmith_core::tag::TagKey::from(ironsmith_core::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG).into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    let method = ironsmith_core::AlternativeCastingMethod::<EffectAst, crate::model::CompilerCost, ironsmith_core::ThisSpellCostCondition>::cast_from_zone_with_total_cost(
        "Cast without paying mana cost", Zone::Exile, ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(), None, false);
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::AlternativeCast(method), filter, Zone::Exile);
    spec.usage_limit = Some(usage);
    let surface = crate::lexer::render_token_slice(tokens);
    let surface = surface.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static}))
}

/// "Once during each of your turns, you may cast <spells> from your hand
/// without paying its mana cost." (Zaffai and the Tempests, Vision): one
/// free cast of a matching hand card per turn (CR 118.9). The usage budget
/// belongs to the permission, so the grant carries it.
pub(super) fn parse_once_per_turn_hand_free_cast(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some((usage, subject)) = primitives::probe_all(tokens, (
        alt((
            primitives::phrase(&["once", "each", "turn"]).value(crate::grant::GrantUsageLimit::OnceEachTurn),
            primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).value(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
        )),
        primitives::comma(),
        primitives::phrase(&["you", "may", "cast"]),
        subject_before_from,
        primitives::phrase(&["from", "your", "hand", "without", "paying", "its", "mana", "cost"]),
        primitives::sentence_end(),
    ).map(|(usage, _, _, subject, _, _)| (usage, subject)), "once-per-turn-hand-free-cast") else { return Ok(None); };
    // "the first ..." / "this card" subjects have their own grammars.
    if !crate::lexer::parser_token_word_refs(subject).iter().any(|word| matches!(*word, "spell" | "spells")) {
        return Ok(None);
    }
    let Some(mut filter) = card_filter(subject)? else { return Ok(None); };
    exclude_lands_from_spell_filter(&mut filter);
    filter.owner = Some(PlayerFilter::You);
    let method = ironsmith_core::AlternativeCastingMethod::<EffectAst, crate::model::CompilerCost, ironsmith_core::ThisSpellCostCondition>::cast_from_zone_with_total_cost(
        "Cast without paying mana cost", Zone::Hand, ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(), None, false);
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::AlternativeCast(method), filter, Zone::Hand);
    spec.usage_limit = Some(usage);
    let surface = crate::lexer::render_token_slice(tokens);
    let surface = surface.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static}))
}

/// Bounded, target-free reflexive follow-up to using one static permission.
/// The trigger is retained on that permission; it is never an immediate effect
/// or a trigger for every otherwise matching play.
pub(super) fn parse_permission_with_token_follow_up(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some((separator, _, follow_up)) = primitives::find_prefix(tokens, || (
        primitives::period(), primitives::phrase(&["when", "you", "do"]), primitives::comma(),
    )) else { return Ok(None); };
    if primitives::probe_all(follow_up, (
        primitives::phrase(&["create", "a"]),
        alt((primitives::kw("food"), primitives::kw("treasure"), primitives::kw("clue"))),
        primitives::kw("token"), primitives::sentence_end(),
    ).void(), "permission-reflexive-named-token").is_none() { return Ok(None); }
    let Some(PermissionClauseSpec::GrantBySpec {player, mut spec, lifetime: PermissionLifetime::Static}) = parse_filtered_zone_permission(&tokens[..separator])?
        else { return Ok(None); };
    spec.on_use_effects = crate::clause_support::parse_effect_sentences_lexed(follow_up)?;
    // The permission keeps its own surface; the reflexive sentence restores
    // its sentence case and the capitalized token name ("create a Food token").
    let token_name = crate::lexer::parser_token_word_refs(follow_up)
        .into_iter()
        .nth(2)
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str())).unwrap_or_default()
        })
        .unwrap_or_default();
    spec.filtered_zone_surface = spec
        .filtered_zone_surface
        .take()
        .map(|permission| format!("{permission}. When you do, create a {token_name} token"));
    Ok(Some(PermissionClauseSpec::GrantBySpec {player, spec, lifetime: PermissionLifetime::Static}))
}

pub(super) fn parse_filtered_zone_permission(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some(shape) = primitives::probe_all(tokens, shape, "filtered-zone-play-cast-permission") else { return Ok(None); };
    permission_from_shape(tokens, shape)
}

fn permission_from_shape(tokens: &[OwnedLexToken], shape: PermissionShape<'_>) -> Result<Option<PermissionClauseSpec>, CardTextError> {
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
                let general_cards = crate::lexer::parser_token_word_refs(subject) == ["cards"];
                if !names_land_domain(subject) && !general_cards { return Ok(None); }
                if !general_cards { lands.card_types = vec![CardType::Land]; }
                lands
            }
        } else {
            let Some(mut spells) = card_filter(subject)? else { return Ok(None); };
            exclude_lands_from_spell_filter(&mut spells); spells
        }
    } else { ObjectFilter::default() };
    filter.owner = Some(PlayerFilter::You);
    if let Some(comparison_tokens) = shape.mana_value {
        let Some((operator, rhs_tokens)) = crate::grammar::values::parse_value_comparison_tokens(comparison_tokens) else { return Ok(None); };
        let Some((rhs_value, used)) = crate::grammar::values::parse_value_prefix_lexed(rhs_tokens) else { return Ok(None); };
        if used != rhs_tokens.len() { return Ok(None); }
        filter.mana_value = Some(super::mana_value_filter_comparison(comparison_tokens, operator, rhs_value));
    }
    let mut spec = crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::play_from(), filter, shape.zone);
    spec.usage_limit = shape.usage; spec.top_card_only = shape.top_only;
    spec.instant_timing = shape.instant_timing;
    let surface = crate::lexer::render_token_slice(tokens);
    let surface = surface.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec { player: PlayerAst::You, spec, lifetime: PermissionLifetime::Static }))
}

/// A complete resolving compound permission. Its private view follows each
/// changing library top for the duration; it is not a one-time look instruction.
pub(super) fn parse_timed_top_look_and_permission(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    let Some((_, body)) = primitives::parse_prefix(tokens, (
        primitives::phrase(&["until", "end", "of", "turn"]), primitives::comma(),
    )) else { return Ok(None); };
    let Some((_, permission)) = primitives::parse_prefix(body, (
        primitives::phrase(&["you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time"]),
        opt(primitives::comma()), primitives::kw("and"),
    )) else { return Ok(None); };
    let Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, mut spec, lifetime: PermissionLifetime::Static}) = parse_filtered_zone_permission(permission)?
        else { return Ok(None); };
    if spec.zone != Zone::Library || !spec.top_card_only || spec.usage_limit.is_some() { return Ok(None); }
    spec.may_look_at_top = true;
    let surface = crate::lexer::render_token_slice(body);
    let surface = surface.trim().trim_end_matches('.'); let mut chars = surface.chars();
    spec.filtered_zone_surface = chars.next().map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(PermissionClauseSpec::GrantBySpec {player: PlayerAst::You, spec, lifetime: PermissionLifetime::UntilEndOfTurn}))
}

pub(crate) fn parse_top_look_and_permission(tokens: &[OwnedLexToken]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some((_, permission)) = primitives::parse_prefix(tokens, (
        primitives::phrase(&["you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time"]),
        opt(primitives::comma()), primitives::kw("and"),
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
    fn temporary_compound_top_view_and_play_are_one_complete_duration_permission() {
        let tokens = crate::lexer::lex_line("Until end of turn, you may look at the top card of your library any time, and you may play lands and cast spells from the top of your library.", 0).unwrap();
        let Some(PermissionClauseSpec::GrantBySpec {spec, lifetime: PermissionLifetime::UntilEndOfTurn, ..}) = parse_timed_top_look_and_permission(&tokens).unwrap() else { panic!("missing compound permission"); };
        assert!(spec.top_card_only && spec.may_look_at_top); assert!(!spec.instant_timing);
        let bad = crate::lexer::lex_line("Until end of turn, you may look at the top card of your library any time, and you may play lands and cast spells from the top of your library unless you pay 2 life.", 0).unwrap();
        assert!(parse_timed_top_look_and_permission(&bad).unwrap().is_none());
    }
    #[test]
    fn two_free_cast_origins_remain_one_permission_with_concrete_origin_scopes() {
        let tokens = crate::lexer::lex_line("Once during each of your turns, you may cast a spell from your hand or the top of your library without paying its mana cost.", 0).unwrap();
        let Some(PermissionClauseSpec::GrantBySpec {spec, ..}) = parse_shared_hand_top_free_cast(&tokens).unwrap() else { panic!("missing shared permission"); };
        assert_eq!(spec.usage_limit, Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns));
        let scopes = spec.zone_specs(); assert_eq!(scopes.len(), 2);
        for scope in scopes {
            assert_eq!(scope.filter.zone, Some(scope.zone));
            assert_eq!(scope.top_card_only, scope.zone == Zone::Library);
            let crate::model::CompilerGrantableCore::AlternativeCast(method) = scope.grantable else { panic!("missing free cost"); };
            assert_eq!(method.cast_from_zone(), scope.zone);
        }
    }
    #[test]
    fn permission_reflexive_token_tail_is_retained_and_targeted_tails_are_not_accepted() {
        let tokens = crate::lexer::lex_line("Once each turn, you may play a historic land or cast a historic spell from the top of your library. When you do, create a Food token.", 0).unwrap();
        let Some(PermissionClauseSpec::GrantBySpec {spec, ..}) = parse_permission_with_token_follow_up(&tokens).unwrap() else { panic!("missing permission follow-up"); };
        assert_eq!(spec.usage_limit, Some(crate::grant::GrantUsageLimit::OnceEachTurn));
        assert_eq!(spec.on_use_effects.len(), 1); assert_eq!(spec.filter.any_of.len(), 2);
        let targeted = crate::lexer::lex_line("You may cast spells from the top of your library. When you do, destroy target creature.", 0).unwrap();
        assert!(parse_permission_with_token_follow_up(&targeted).unwrap().is_none());
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


/// A current-turn origin qualifier belongs to the proposed card incarnation,
/// not to a source-wide condition or a generic count of cards milled.
pub(super) fn parse_recent_graveyard_permission(tokens: &[OwnedLexToken]) -> Result<Option<PermissionClauseSpec>, CardTextError> {
    #[derive(Clone, Copy)]
    enum Recent {
        Milled,
        FromLibrary,
        NotFromBattlefield,
    }
    fn recent<'a>(input: &mut LexStream<'a>) -> WResult<(PermissionShape<'a>, Recent)> {
        primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).parse_next(input)?;
        primitives::comma().parse_next(input)?;
        primitives::phrase(&["you", "may"]).parse_next(input)?;
        let play = alt((primitives::kw("play").value(true), primitives::kw("cast").value(false))).parse_next(input)?;
        let subject = subject_before_from(input)?;
        primitives::phrase(&["from", "among", "cards", "in", "your", "graveyard", "that", "were"]).parse_next(input)?;
        let mill = alt((
            primitives::phrase(&["milled", "this", "turn"]).value(Recent::Milled),
            primitives::phrase(&["put", "there", "from", "your", "library", "this", "turn"]).value(Recent::FromLibrary),
            // "... that were put there from anywhere other than the
            // battlefield this turn" (Banon, the Returners' Leader)
            primitives::phrase(&[
                "put", "there", "from", "anywhere", "other", "than", "the", "battlefield", "this", "turn",
            ]).value(Recent::NotFromBattlefield),
        )).parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        Ok((PermissionShape {play, subject: Some(subject), zone: Zone::Graveyard, top_only: false,
            instant_timing: false, usage: Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
            mana_value: None}, mill))
    }
    let Some((shape, mill)) = primitives::probe_all(tokens, recent, "recent-graveyard-play-permission") else { return Ok(None); };
    let Some(PermissionClauseSpec::GrantBySpec {player, mut spec, lifetime}) = permission_from_shape(tokens, shape)? else { return Ok(None); };
    match mill {
        Recent::Milled => spec.filter.milled_into_graveyard_this_turn = true,
        Recent::FromLibrary => spec.filter.entered_graveyard_from_library_this_turn = true,
        Recent::NotFromBattlefield => {
            spec.filter.entered_graveyard_not_from_battlefield_this_turn = true;
        }
    }
    Ok(Some(PermissionClauseSpec::GrantBySpec {player, spec, lifetime}))
}
