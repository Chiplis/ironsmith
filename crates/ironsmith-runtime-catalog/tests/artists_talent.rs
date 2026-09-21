use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardId, CardType, CounterType, GameState, PlayerId, Zone};

#[test]
fn artists_talent_static_abilities_require_their_class_levels() {
    let asset: serde_json::Value = serde_json::from_str(include_str!(
        "../../../web/ui/public/cards/artist-s-talent.json"
    ))
    .unwrap();
    let artifact = serde_json::from_value(asset["artifacts"][0].clone()).unwrap();
    let talent =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&talent, alice, Zone::Battlefield);
    let base_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Noncreature cost probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(base_cost.clone())
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

    for (counters, expected_cost, expected_damage) in [(0, 3, 1), (1, 2, 1), (2, 2, 3)] {
        if counters > 0 {
            game.add_counters(source, CounterType::Level, 1);
        }
        let cost = ironsmith::decision::calculate_effective_mana_cost(
            &game,
            alice,
            game.object(spell_id).unwrap(),
            &base_cost,
        );
        assert_eq!(
            cost.mana_value(),
            expected_cost,
            "Class level {} cost",
            counters + 1
        );
        let life_before = game.player(bob).unwrap().life;
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::effect::Effect::deal_damage(
                1,
                ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(
                    bob,
                )),
            ),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            life_before - game.player(bob).unwrap().life,
            expected_damage,
            "Class level {} damage",
            counters + 1
        );
    }
}
