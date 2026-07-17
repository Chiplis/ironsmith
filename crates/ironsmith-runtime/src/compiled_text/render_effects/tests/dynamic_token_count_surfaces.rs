use super::*;
use ironsmith_core::TurnHistoryCount;

fn creature_token(
    name: &str,
    subtype: Subtype,
    color: crate::color::ColorSet,
) -> crate::cards::CardDefinition {
    crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .color_indicator(color)
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .build()
}

fn render_create(token: crate::cards::CardDefinition, count: Value) -> String {
    describe_effect(&Effect::new(crate::effects::CreateTokenEffect::new(
        token,
        count,
        PlayerFilter::You,
    )))
}

#[test]
fn ferrafor_and_hare_equal_to_counts_are_not_rewritten_as_for_each() {
    let ferrafor_count = Value::CountersOn(
        Box::new(ChooseSpec::All(
            ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
        )),
        None,
    )
    .with_surface_hint(ValueSurfaceHint::CountersAmong)
    .with_surface_hint(ValueSurfaceHint::EqualTo);
    assert_eq!(
        render_create(
            creature_token(
                "Saproling",
                Subtype::Saproling,
                crate::color::ColorSet::GREEN,
            ),
            ferrafor_count,
        ),
        "Create a number of 1/1 green Saproling creature tokens equal to the number of counters among creatures target player controls"
    );

    let hare_count = Value::Count(
        ObjectFilter::creature()
            .you_control()
            .other()
            .named("Hare Apparent"),
    )
    .with_surface_hint(ValueSurfaceHint::EqualTo);
    assert_eq!(
        render_create(
            creature_token("Rabbit", Subtype::Rabbit, crate::color::ColorSet::WHITE,),
            hare_count,
        ),
        "Create a number of 1/1 white Rabbit creature tokens equal to the number of other creatures you control named Hare Apparent"
    );
}

#[test]
fn heidegger_and_hornbeetle_keep_dynamic_token_count_surfaces() {
    let heidegger_count = Value::PlayersWhoControlMoreThanYou {
        players: PlayerFilter::Opponent,
        filter: ObjectFilter::creature(),
    }
    .with_surface_hint(ValueSurfaceHint::EqualTo);
    assert_eq!(
        render_create(
            creature_token("Soldier", Subtype::Soldier, crate::color::ColorSet::WHITE,),
            heidegger_count,
        ),
        "Create a number of 1/1 white Soldier creature tokens equal to the number of opponents who control more creatures than you"
    );

    let hornbeetle_count = Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
        counter_type: Some(crate::object::CounterType::PlusOnePlusOne),
        filter: ObjectFilter::creature().you_control(),
    })
    .with_surface_hint(ValueSurfaceHint::ForEach);
    assert_eq!(
        render_create(
            creature_token("Insect", Subtype::Insect, crate::color::ColorSet::GREEN,),
            hornbeetle_count,
        ),
        "Create a 1/1 green Insect creature token for each +1/+1 counter you've put on creatures under your control this turn"
    );
}

#[test]
fn chroma_aggregate_values_keep_plural_cost_scopes() {
    let white_symbols = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::All(ObjectFilter::permanent().you_control())),
        color: crate::color::Color::White,
    };
    let goat = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goat")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goat])
        .color_indicator(crate::color::ColorSet::WHITE)
        .power_toughness(crate::card::PowerToughness::fixed(0, 1))
        .build();
    assert_eq!(
        render_create(goat, white_symbols),
        "Create a 0/1 white Goat creature token for each white mana symbol in the mana costs of permanents you control"
    );

    let mut graveyard_cards = ObjectFilter::default()
        .in_zone(crate::zone::Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    graveyard_cards.set_explicit_card_noun(true);
    let black_symbols = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::All(graveyard_cards)),
        color: crate::color::Color::Black,
    };
    assert_eq!(
        describe_value(&black_symbols),
        "the number of black mana symbols in the mana costs of cards in your graveyard"
    );
}
