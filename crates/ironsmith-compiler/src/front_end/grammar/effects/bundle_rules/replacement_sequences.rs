use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::effect::{ChoiceCount, SearchSelectionMode};
use crate::grammar::{filters, leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas};
use crate::mana::ManaCost;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KickedMultiZoneSearchDestinationShape {
    pub(crate) filter: ObjectFilter,
    pub(crate) count: ChoiceCount,
    pub(crate) search_mode: SearchSelectionMode,
    pub(crate) zones: Vec<Zone>,
    pub(crate) default_destination: Zone,
    pub(crate) kicked_destination: Zone,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PersistentExilePlayTaxShape {
    pub(crate) target_filter: ObjectFilter,
    pub(crate) permission_player: PlayerAst,
    pub(crate) taxed_caster: PlayerFilter,
    pub(crate) additional_cost: ManaCost,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpellCastThisWayTaxShape {
    pub(crate) taxed_caster: Option<PlayerFilter>,
    pub(crate) additional_cost: ManaCost,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EachPlayerHandExilePlayConstraintsShape {
    pub(crate) players: PlayerFilter,
    pub(crate) additional_cost: ManaCost,
    pub(crate) lands_enter_tapped: bool,
}

fn commas<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)
}

fn destination_zone<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    alt((
        primitives::kw("hand").value(Zone::Hand),
        primitives::kw("battlefield").value(Zone::Battlefield),
        primitives::kw("graveyard").value(Zone::Graveyard),
        primitives::kw("exile").value(Zone::Exile),
        primitives::kw("library").value(Zone::Library),
    ))
    .parse_next(input)
}

fn multi_zone_search_head<'a>(input: &mut LexStream<'a>) -> WResult<Vec<Zone>> {
    primitives::phrase(&["search", "your", "library"]).parse_next(input)?;
    alt((
        (primitives::kw("and/or"), primitives::kw("graveyard")).void(),
        primitives::phrase(&["and", "or", "graveyard"]),
        primitives::phrase(&["and", "graveyard"]),
        primitives::phrase(&["or", "graveyard"]),
    ))
    .parse_next(input)?;
    primitives::kw("for").parse_next(input)?;
    Ok(vec![Zone::Library, Zone::Graveyard])
}

fn reveal_and_put_destination<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    primitives::kw("them").parse_next(input)?;
    commas(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    primitives::phrase(&["put", "them"]).parse_next(input)?;
    alt((primitives::kw("into"), primitives::kw("onto"))).parse_next(input)?;
    opt(alt((primitives::kw("your"), primitives::kw("the")))).parse_next(input)?;
    destination_zone.parse_next(input)
}

fn conditional_search_shuffle<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "search", "your", "library", "this", "way"])
        .parse_next(input)?;
    commas(input)?;
    primitives::kw("shuffle").void().parse_next(input)
}

fn kicked_destination_replacement<'a>(input: &mut LexStream<'a>) -> WResult<(Zone, Zone)> {
    primitives::phrase(&["if", "this", "spell", "was", "kicked"]).parse_next(input)?;
    commas(input)?;
    primitives::phrase(&["put", "those", "cards"]).parse_next(input)?;
    alt((primitives::kw("onto"), primitives::kw("into"))).parse_next(input)?;
    opt(alt((primitives::kw("the"), primitives::kw("your")))).parse_next(input)?;
    let kicked = destination_zone.parse_next(input)?;
    primitives::phrase(&["instead", "of", "putting", "them"]).parse_next(input)?;
    alt((primitives::kw("into"), primitives::kw("onto"))).parse_next(input)?;
    opt(alt((primitives::kw("the"), primitives::kw("your")))).parse_next(input)?;
    let default = destination_zone.parse_next(input)?;
    Ok((kicked, default))
}

pub(crate) fn parse_kicked_multi_zone_search_destination_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KickedMultiZoneSearchDestinationShape> {
    let sentences = split_lexed_sentences(tokens);
    let [search, shuffle, replacement] = sentences.as_slice() else {
        return None;
    };

    let (zones, search_tail) = primitives::parse_prefix(search, multi_zone_search_head)?;
    let (selection_tokens, reveal_tail) =
        primitives::split_lexed_once_on_separator(search_tail, || primitives::kw("reveal").void())?;
    let ((up_to, count), filter_tokens) = primitives::parse_prefix(
        trim_lexed_commas(selection_tokens),
        (
            opt(primitives::phrase(&["up", "to"])).map(|prefix| prefix.is_some()),
            leaf::parse_leaf_number_prefix_lexed,
        ),
    )?;
    let count = usize::try_from(count).ok()?;
    let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(filter_tokens),
        false,
    )
    .ok()?;
    filter.owner = Some(PlayerFilter::You);
    filter.zone = None;

    let default_destination = primitives::parse_all_or_none(
        trim_lexed_commas(reveal_tail),
        reveal_and_put_destination,
        "multi-zone search destination",
    )
    .ok()
    .flatten()?;
    primitives::parse_all_or_none(
        trim_lexed_commas(shuffle),
        conditional_search_shuffle,
        "conditional search shuffle",
    )
    .ok()
    .flatten()?;
    let (kicked_destination, replacement_default) = primitives::parse_all_or_none(
        trim_lexed_commas(replacement),
        kicked_destination_replacement,
        "kicked search destination replacement",
    )
    .ok()
    .flatten()?;
    if replacement_default != default_destination {
        return None;
    }

    Some(KickedMultiZoneSearchDestinationShape {
        filter,
        count: if up_to {
            ChoiceCount::up_to(count)
        } else {
            ChoiceCount::exactly(count)
        },
        search_mode: if up_to {
            SearchSelectionMode::Optional
        } else {
            SearchSelectionMode::Exact
        },
        zones,
        default_destination,
        kicked_destination,
    })
}

fn persistent_permission_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    alt((
        primitives::phrase(&["its", "owner"]).value(PlayerAst::ItsOwner),
        primitives::phrase(&["their", "owner"]).value(PlayerAst::ItsOwner),
        primitives::kw("you").value(PlayerAst::You),
    ))
    .parse_next(input)
}

fn persistent_play_permission<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    primitives::phrase(&[
        "for", "as", "long", "as", "that", "card", "remains", "exiled",
    ])
    .parse_next(input)?;
    commas(input)?;
    let player = persistent_permission_player.parse_next(input)?;
    primitives::phrase(&["may", "play", "it"]).parse_next(input)?;
    Ok(player)
}

fn taxed_player(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let reference = leaf::parse_leaf_player_reference_tokens(
        tokens,
        leaf::LeafPlayerReferenceMode::ControlSubject {
            allow_that_player: true,
            allow_opponent_players: true,
            allow_defending_player: false,
        },
    )?;
    match reference {
        leaf::LeafPlayerReference::You => Some(PlayerFilter::You),
        leaf::LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        leaf::LeafPlayerReference::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(crate) fn parse_spell_cast_this_way_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastThisWayTaxShape> {
    let sentences = split_lexed_sentences(tokens);
    let tokens = match sentences.as_slice() {
        [sentence] => *sentence,
        _ => tokens,
    };
    let (_, tax_tail) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        alt((
            primitives::phrase(&["a", "spell", "cast"]).void(),
            primitives::phrase(&["each", "spell", "cast"]).void(),
        ))
        .parse_next(input)
    })?;
    let (caster_tokens, cost_tokens) = primitives::split_lexed_once_on_separator(tax_tail, || {
        primitives::phrase(&["this", "way", "costs"]).void()
    })?;
    let caster_tokens = trim_lexed_commas(caster_tokens);
    let taxed_caster = if caster_tokens.is_empty() {
        None
    } else {
        let (_, player_tokens) =
            primitives::parse_prefix(caster_tokens, primitives::kw("by").void())?;
        Some(taxed_player(trim_lexed_commas(player_tokens))?)
    };
    let (cost_prefix, rest) = primitives::parse_prefix(
        trim_lexed_commas(cost_tokens),
        leaf::parse_leaf_fixed_mana_cost_prefix_lexed,
    )?;
    primitives::parse_all_or_none(
        trim_lexed_commas(rest),
        primitives::phrase(&["more", "to", "cast"]).void(),
        "spell tax suffix",
    )
    .ok()
    .flatten()?;

    Some(SpellCastThisWayTaxShape {
        taxed_caster,
        additional_cost: cost_prefix.cost,
    })
}

fn each_player_hand_exile_play_permission<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    let players = alt((
        primitives::phrase(&["each", "opponent"]).value(PlayerFilter::Opponent),
        primitives::phrase(&["each", "player"]).value(PlayerFilter::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&[
        "exiles", "a", "card", "from", "their", "hand", "and", "may", "play", "that", "card",
        "for", "as", "long", "as", "it", "remains", "exiled",
    ])
    .parse_next(input)?;
    Ok(players)
}

fn each_land_played_this_way_enters_tapped<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["each", "land", "played", "this", "way", "enters"]).parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    primitives::kw("tapped").void().parse_next(input)
}

pub(crate) fn parse_each_player_hand_exile_play_constraints_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerHandExilePlayConstraintsShape> {
    let sentences = split_lexed_sentences(tokens);
    let [exile_and_permission, tax, land_entry] = sentences.as_slice() else {
        return None;
    };
    let players = primitives::parse_all_or_none(
        trim_lexed_commas(exile_and_permission),
        each_player_hand_exile_play_permission,
        "each-player hand exile play permission",
    )
    .ok()
    .flatten()?;
    let tax = parse_spell_cast_this_way_tax_tokens(tax)?;
    if tax.taxed_caster.is_some() {
        return None;
    }
    primitives::parse_all_or_none(
        trim_lexed_commas(land_entry),
        each_land_played_this_way_enters_tapped,
        "each land played this way enters tapped",
    )
    .ok()
    .flatten()?;

    Some(EachPlayerHandExilePlayConstraintsShape {
        players,
        additional_cost: tax.additional_cost,
        lands_enter_tapped: true,
    })
}

pub(crate) fn parse_persistent_exile_play_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PersistentExilePlayTaxShape> {
    let sentences = split_lexed_sentences(tokens);
    let [exile, permission, tax] = sentences.as_slice() else {
        return None;
    };
    let (_, target_tokens) = primitives::parse_prefix(exile, |input: &mut LexStream<'_>| {
        primitives::phrase(&["exile", "target"])
            .void()
            .parse_next(input)
    })?;
    let mut target_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(target_tokens),
        false,
    )
    .ok()?;
    target_filter.zone = Some(Zone::Battlefield);
    let permission_player = primitives::parse_all_or_none(
        trim_lexed_commas(permission),
        persistent_play_permission,
        "persistent exile play permission",
    )
    .ok()
    .flatten()?;

    let tax = parse_spell_cast_this_way_tax_tokens(tax)?;
    let taxed_caster = tax.taxed_caster?;

    Some(PersistentExilePlayTaxShape {
        target_filter,
        permission_player,
        taxed_caster,
        additional_cost: tax.additional_cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::types::Subtype;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_typed_kicked_multi_zone_search_destination() {
        let tokens = lex(
            "Search your library and/or graveyard for up to three Wizard cards, reveal them, and put them into your hand. If you search your library this way, shuffle. If this spell was kicked, put those cards onto the battlefield instead of putting them into your hand.",
        );
        let shape = parse_kicked_multi_zone_search_destination_tokens(&tokens).unwrap();
        assert_eq!(shape.zones, vec![Zone::Library, Zone::Graveyard]);
        assert_eq!(shape.default_destination, Zone::Hand);
        assert_eq!(shape.kicked_destination, Zone::Battlefield);
        assert_eq!(shape.count, ChoiceCount::up_to(3));
        assert!(shape.filter.subtypes.contains(&Subtype::Wizard));
    }

    #[test]
    fn parses_typed_persistent_exile_permission_tax() {
        let shape = parse_persistent_exile_play_tax_tokens(&lex(
            "Exile target artifact. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {3} more to cast.",
        ))
        .unwrap();
        assert!(
            shape
                .target_filter
                .card_types
                .contains(&crate::types::CardType::Artifact)
        );
        assert_eq!(shape.permission_player, PlayerAst::ItsOwner);
        assert_eq!(shape.taxed_caster, PlayerFilter::Opponent);
    }

    #[test]
    fn parses_spell_cast_this_way_tax_without_explicit_caster() {
        let shape = parse_spell_cast_this_way_tax_tokens(&lex(
            "A spell cast this way costs {2} more to cast.",
        ))
        .unwrap();
        assert_eq!(shape.taxed_caster, None);
        assert_eq!(shape.additional_cost.to_oracle(), "{2}");
    }

    #[test]
    fn parses_each_opponent_hand_exile_with_linked_play_constraints() {
        let shape = parse_each_player_hand_exile_play_constraints_tokens(&lex(
            "Each opponent exiles a card from their hand and may play that card for as long as it remains exiled. Each spell cast this way costs {1} more to cast. Each land played this way enters tapped.",
        ))
        .expect("typed hand-exile permission bundle");
        assert_eq!(shape.players, PlayerFilter::Opponent);
        assert_eq!(shape.additional_cost.to_oracle(), "{1}");
        assert!(shape.lands_enter_tapped);
    }
}
