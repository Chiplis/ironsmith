use super::*;

fn beast_token() -> crate::cards::CardDefinition {
    crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phyrexian")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Beast])
        .color_indicator(crate::color::ColorSet::GREEN)
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .build()
}

fn correlated_fight(
    fight_result_tag: TagKey,
    fight_source_tag: TagKey,
) -> crate::effects::ForEachObjectCorrelatedResultEffect {
    let result_tag = TagKey::from("created_0");
    let source_binding = TagKey::from("created_0_correlated_source");
    let result_binding = TagKey::from("created_0_correlated_result");
    crate::effects::ForEachObjectCorrelatedResultEffect::new(
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        vec![
            Effect::new(crate::effects::CreateTokenEffect::one(beast_token()))
                .tag(result_tag.clone()),
        ],
        result_tag,
        source_binding,
        result_binding,
        vec![Effect::fight(
            ChooseSpec::Tagged(fight_result_tag),
            ChooseSpec::Tagged(fight_source_tag),
        )],
    )
}

#[test]
fn exact_correlated_result_and_source_bindings_render_distinct_fights() {
    let correlated = correlated_fight(
        TagKey::from("created_0_correlated_result"),
        TagKey::from("created_0_correlated_source"),
    );

    assert_eq!(
        describe_correlated_created_token_fight(&correlated),
        Some(
            "For each creature your opponents control, create a 4/4 green Phyrexian Beast creature token. Each of those tokens fights a different one of those creatures"
                .to_string()
        )
    );
}

#[test]
fn correlated_fight_renderer_rejects_wrong_or_self_pairing_bindings() {
    let wrong_source = correlated_fight(
        TagKey::from("created_0_correlated_result"),
        TagKey::from("unrelated_source"),
    );
    assert_eq!(
        describe_correlated_created_token_fight(&wrong_source),
        None,
        "the fighter must consume the exact correlated source binding"
    );

    let self_fight = correlated_fight(
        TagKey::from("created_0_correlated_result"),
        TagKey::from("created_0_correlated_result"),
    );
    assert_eq!(
        describe_correlated_created_token_fight(&self_fight),
        None,
        "the renderer must never canonicalize a produced-object self fight"
    );
}
