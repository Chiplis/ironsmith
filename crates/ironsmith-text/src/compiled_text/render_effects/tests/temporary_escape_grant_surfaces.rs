use super::*;

const ESCAPE_GRANT_SURFACE: &str = "Until end of turn, each creature card in your graveyard gains \"Escape—{3}{B}, Exile four other cards from your graveyard.\"";

fn temporary_escape_grant(
    filter: ObjectFilter,
    zone: Zone,
    player: PlayerFilter,
    duration: crate::grant::GrantDuration,
) -> Effect {
    let method = crate::alternative_cast::AlternativeCastingMethod::Escape {
        cost: Some(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(3),
            crate::mana::ManaSymbol::Black,
        ])),
        exile_count: 4,
        additional_cost: crate::cost::TotalCost::from_cost(
            crate::costs::Cost::exile_from_graveyard(4, None),
        ),
    };
    let spec = crate::grant::GrantSpec::new(
        crate::grant::Grantable::AlternativeCast(method),
        filter,
        zone,
    );
    Effect::grant_by_spec(spec, player, duration)
}

fn exact_creature_card_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature().owned_by(PlayerFilter::You);
    filter.zone = None;
    filter.set_explicit_card_noun(true);
    filter.set_explicit_card_type_noun(Some(CardType::Creature));
    filter
}

#[test]
fn temporary_fixed_escape_grant_keeps_the_complete_quoted_cost() {
    let effect = temporary_escape_grant(
        exact_creature_card_filter(),
        Zone::Graveyard,
        PlayerFilter::You,
        crate::grant::GrantDuration::UntilEndOfTurn,
    );

    assert_eq!(describe_effect(&effect), ESCAPE_GRANT_SURFACE);
}

#[test]
fn temporary_escape_surface_requires_matching_recipient_zone_and_duration() {
    let cases = [
        temporary_escape_grant(
            ObjectFilter::artifact().owned_by(PlayerFilter::You),
            Zone::Graveyard,
            PlayerFilter::You,
            crate::grant::GrantDuration::UntilEndOfTurn,
        ),
        temporary_escape_grant(
            exact_creature_card_filter(),
            Zone::Exile,
            PlayerFilter::You,
            crate::grant::GrantDuration::UntilEndOfTurn,
        ),
        temporary_escape_grant(
            exact_creature_card_filter(),
            Zone::Graveyard,
            PlayerFilter::Opponent,
            crate::grant::GrantDuration::UntilEndOfTurn,
        ),
        temporary_escape_grant(
            exact_creature_card_filter(),
            Zone::Graveyard,
            PlayerFilter::You,
            crate::grant::GrantDuration::Forever,
        ),
    ];

    for near_miss in cases {
        assert_ne!(describe_effect(&near_miss), ESCAPE_GRANT_SURFACE);
    }
}
