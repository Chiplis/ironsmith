use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
pub(super) fn nonbattlefield_untyped_filters_render_as_cards() {
    let night_dealings = compiled_card("Night Dealings");
    assert!(
        night_dealings.contains("Search your library for a nonland card with mana value"),
        "Night Dealings should keep its library filter in card context:\n{night_dealings}"
    );
    assert!(
        !night_dealings.contains("nonland permanent card"),
        "Night Dealings must not acquire a permanent restriction:\n{night_dealings}"
    );

    let quiet_speculation = compiled_card("Quiet Speculation");
    assert!(
        quiet_speculation.contains("up to three cards with flashback"),
        "Quiet Speculation should pluralize the card noun before its qualifier:\n{quiet_speculation}"
    );
    assert!(
        !quiet_speculation.contains("permanent with flashback"),
        "Quiet Speculation must not acquire a permanent restriction:\n{quiet_speculation}"
    );

    let sibylline_soothsayer = compiled_card("Sibylline Soothsayer");
    assert!(
        sibylline_soothsayer
            .contains("until you reveal a nonland card with mana value 3 or greater"),
        "Sibylline Soothsayer should render its implicit-library match as a card:\n{sibylline_soothsayer}"
    );
    assert!(
        !sibylline_soothsayer.contains("nonland permanent card"),
        "Sibylline Soothsayer must not acquire a permanent restriction:\n{sibylline_soothsayer}"
    );

    let coercion = compiled_card("Coercion");
    assert!(
        coercion.contains("choose a card from it"),
        "an unconstrained hand choice should also remain a card:\n{coercion}"
    );
    assert!(
        !coercion.contains("choose a permanent card"),
        "an unconstrained hand choice must not acquire a permanent restriction:\n{coercion}"
    );
}

#[test]
pub(super) fn explicit_permanent_card_filters_remain_restricted() {
    let explicit_search = CardDefinitionBuilder::new(CardId::new(), "Permanent Card Search")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::search_library_to_hand(
            ObjectFilter::permanent_card()
                .in_zone(Zone::Library)
                .owned_by(PlayerFilter::You),
            false,
        )])
        .build();
    let explicit_search = canonical_compiled_lines(&explicit_search).join("\n");
    assert!(
        explicit_search.contains("Search your library for a permanent card"),
        "a direct search must retain an explicit permanent-card restriction:\n{explicit_search}"
    );

    let divine_gambit = compiled_card("Divine Gambit");
    assert!(
        divine_gambit.contains("may put a permanent card from")
            && divine_gambit.contains("hand onto the battlefield"),
        "Divine Gambit's explicit permanent-card restriction must remain intact:\n{divine_gambit}"
    );

    let amrou_scout = compiled_card("Amrou Scout");
    let search_clause = amrou_scout
        .split("Search your library for ")
        .nth(1)
        .expect("Amrou Scout should render a library-search clause");
    assert!(
        search_clause.contains("permanent") && search_clause.contains("card"),
        "Amrou Scout's explicit Rebel permanent-card restriction must remain intact:\n{amrou_scout}"
    );
}

#[test]
pub(super) fn nonbattlefield_land_subtypes_keep_the_card_noun_after_the_subtype() {
    let filter = ObjectFilter {
        zone: Some(Zone::Library),
        owner: Some(PlayerFilter::You),
        card_types: vec![CardType::Land],
        subtypes: vec![Subtype::Forest],
        supertypes: vec![Supertype::Basic],
        ..ObjectFilter::default()
    };

    assert_eq!(filter.description(), "basic Forest card in your library");
}
