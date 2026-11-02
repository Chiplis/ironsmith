//! Producer filters apply to all permanents, including custom creature/artifact subtypes.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{AddManaOfLandProducedTypesEffect, EffectContext, EffectExecutor};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{
    ManaPaymentRequest, execute_mana_payment_plan, plan_first_mana_payment,
};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
use ironsmith_core::SubtypeFamily;
use ironsmith_core::custom_subtypes::CustomSubtypeDefinition;

#[path = "cf8_p08/support.rs"]
mod support;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn compile(name: &str, body: &str) -> CardDefinition {
    compile_to_runtime_definition(name, body, false).unwrap()
}

fn add(game: &mut GameState, owner: PlayerId, name: &str, body: &str) -> ObjectId {
    game.create_object_from_definition(&compile(name, body), owner, Zone::Battlefield)
}

fn resolve(game: &mut GameState, source: ObjectId, definition: &CardDefinition) {
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("expected activated mana ability")
    };
    let mut context = EffectContext::new_default(source, A);
    for effect in ability.effects.all_effects() {
        effect.0.execute(game, &mut context).unwrap();
    }
}

#[test]
fn custom_subtypes_work_on_creatures_artifacts_and_lands_with_exact_filters() {
    for (family, card_type) in [
        (SubtypeFamily::Creature, "Creature"),
        (SubtypeFamily::Artifact, "Artifact"),
        (SubtypeFamily::Land, "Land"),
    ] {
        ironsmith_compiler::with_custom_subtypes(&[CustomSubtypeDefinition {
            singular: "Factory".into(),
            plural: "Factories".into(),
            family,
        }], || {
            for definition in support::definitions("Factory mirror", "Type: Land\n{T}: Add one mana of any color that a Factory you control could produce.") {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let producer = add(&mut game, A, "Blue Factory", &format!("Type: {card_type} — Factory\n{{T}}: Add {{U}}."));
                game.tap(producer);
                add(&mut game, B, "Opponent Factory", &format!("Type: {card_type} — Factory\n{{T}}: Add {{R}}."));
                add(&mut game, A, "Non-Factory", "Type: Land\n{T}: Add {G}.");
                add(&mut game, A, "Colorless Factory", &format!("Type: {card_type} — Factory\n{{T}}: Add {{C}}."));
                resolve(&mut game, source, &definition);
                assert_eq!(game.player(A).unwrap().mana_pool.blue, 1, "{card_type}");
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
                assert!(game.is_tapped(producer));
                game.remove_object(producer);
                game.player_mut(A).unwrap().mana_pool = Default::default();
                resolve(&mut game, source, &definition);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "exclude other owners, other subtypes and colorless for {card_type}");
            }
        }).unwrap();
    }
}

#[test]
fn elves_feed_real_activation_and_automatic_payment_even_when_tapped_and_summoning_sick() {
    for definition in support::definitions(
        "Elf mirror",
        "Type: Land\n{T}: Add one mana of any color that an Elf you control could produce.",
    ) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let producer = add(
            &mut game,
            A,
            "Blue Elf",
            "Type: Creature — Elf\nPower/Toughness: 1/1\n{T}: Add {U}.",
        );
        game.tap(producer);
        let request = ManaPaymentRequest::new(
            A,
            source,
            ironsmith::costs::PaymentReason::Other,
            ManaCost::from_symbols(vec![ManaSymbol::Blue]),
        );
        let plan = plan_first_mana_payment(&game, &request)
            .expect("mirror can pay using the tapped Elf's potential output");
        assert!(
            plan.mana_ability_steps
                .iter()
                .any(|step| step.source == source)
        );
        let mut payment_game = game.clone();
        execute_mana_payment_plan(
            &mut payment_game,
            &request,
            &plan,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(payment_game.is_tapped(source));
        assert_eq!(payment_game.player(A).unwrap().mana_pool.total(), 0);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            A,
            source,
            0,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 1);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn recursive_creature_producers_need_a_real_color_seed() {
    let definition = compile(
        "Elf mirror",
        "Type: Land\n{T}: Add one mana of any color that an Elf you control could produce.",
    );
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    for name in ["First mirror Elf", "Second mirror Elf"] {
        add(
            &mut game,
            A,
            name,
            "Type: Creature — Elf\nPower/Toughness: 1/1\n{T}: Add one mana of any color that an Elf you control could produce.",
        );
    }
    resolve(&mut game, source, &definition);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    add(
        &mut game,
        A,
        "Green Elf",
        "Type: Creature — Elf\nPower/Toughness: 1/1\n{T}: Add {G}.",
    );
    resolve(&mut game, source, &definition);
    assert_eq!(game.player(A).unwrap().mana_pool.green, 1);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
}

#[test]
fn explicit_land_filters_still_exclude_creatures() {
    // Reflecting Pool's existing wording retains its land restriction.
    for definition in support::definitions(
        "Reflecting Pool",
        "Type: Land\n{T}: Add one mana of any type that a land you control could produce.",
    ) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add(
            &mut game,
            A,
            "Blue Elf",
            "Type: Creature — Elf\nPower/Toughness: 1/1\n{T}: Add {U}.",
        );
        resolve(&mut game, source, &definition);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        add(&mut game, A, "Colorless land", "Type: Land\n{T}: Add {C}.");
        resolve(&mut game, source, &definition);
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
    }
}

#[test]
fn last_known_producer_can_be_a_nonland() {
    use ironsmith::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = add(&mut game, A, "Source", "Type: Land");
    let producer = add(
        &mut game,
        A,
        "Blue Elf",
        "Type: Creature — Elf\nPower/Toughness: 1/1\n{T}: Add {U}.",
    );
    let snapshot =
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(producer).unwrap(), &game);
    game.remove_object(producer);
    let mut context = EffectContext::new_default(source, A);
    context.tag_object("producer", snapshot);
    let effect = AddManaOfLandProducedTypesEffect::new(
        1,
        PlayerFilter::You,
        ObjectFilter::creature().match_tagged("producer", TaggedOpbjectRelation::IsTaggedObject),
        false,
        false,
    );
    effect.execute(&mut game, &mut context).unwrap();
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 1);
}
