#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;

struct ChooseNamedObjects(Vec<String>);

impl ChooseNamedObjects {
    fn new(names: &[&str]) -> Self {
        Self(names.iter().map(|name| (*name).to_string()).collect())
    }
}

impl DecisionMaker for ChooseNamedObjects {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.0
            .iter()
            .filter_map(|name| {
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.name == *name)
                    .map(|candidate| candidate.id)
            })
            .take(ctx.max.unwrap_or(usize::MAX))
            .collect()
    }
}

fn creature_card(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn activated_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Night Soil should have an activated ability")
}

fn zone_by_stable_id(game: &crate::GameState, stable_id: StableId) -> Zone {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
        .expect("card should remain tracked after its zone change")
}

#[test]
fn night_soil_keeps_the_typed_single_graveyard_cost_and_exact_surface() {
    let definition = parse_oracle_card_definition("Night Soil");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "{1}, Exile two creature cards from a single graveyard: Create a 1/1 green Saproling creature token."
    );

    let activated = activated_ability(&definition);
    let costs = activated.mana_cost.costs();
    let (choose_index, choose) = costs
        .iter()
        .enumerate()
        .find_map(|(index, cost)| {
            cost.effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
                .map(|choose| (index, choose))
        })
        .expect("the exile cost should start with an object choice");
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(2));
    assert_eq!(choose.filter.zone, Some(Zone::Graveyard));
    assert_eq!(choose.filter.card_types.as_slice(), [CardType::Creature]);
    assert!(choose.filter.single_graveyard);
    assert_eq!(choose.filter.owner, None);

    let exile = costs
        .get(choose_index + 1)
        .and_then(|cost| cost.effect_ref())
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .expect("the chosen graveyard cards should be consumed by an exile cost");
    assert!(
        matches!(&exile.spec, ChooseSpec::Tagged(tag) if tag == &choose.tag),
        "the exile cost must consume exactly the cards chosen from one graveyard"
    );
}

#[test]
fn night_soil_payment_cannot_mix_graveyards_and_can_use_an_opponents_graveyard() {
    let definition = parse_oracle_card_definition("Night Soil");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 1);

    let alice_card = game.create_object_from_definition(
        &creature_card("Alice Grave Creature"),
        alice,
        Zone::Graveyard,
    );
    let bob_card_a = game.create_object_from_definition(
        &creature_card("Bob Grave Creature A"),
        bob,
        Zone::Graveyard,
    );
    assert!(
        crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).is_err(),
        "one creature card in each graveyard must not satisfy the two-from-one-graveyard cost"
    );

    let bob_card_b = game.create_object_from_definition(
        &creature_card("Bob Grave Creature B"),
        bob,
        Zone::Graveyard,
    );
    let alice_stable = game.object(alice_card).expect("Alice card").stable_id;
    let bob_a_stable = game.object(bob_card_a).expect("Bob card A").stable_id;
    let bob_b_stable = game.object(bob_card_b).expect("Bob card B").stable_id;
    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost)
        .expect("two creature cards in Bob's graveyard should make the cost payable");

    let mut decisions = ChooseNamedObjects::new(&["Alice Grave Creature", "Bob Grave Creature A"]);
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut decisions,
    )
    .expect("Night Soil's activation cost should be paid");

    assert_eq!(zone_by_stable_id(&game, alice_stable), Zone::Graveyard);
    assert_eq!(zone_by_stable_id(&game, bob_a_stable), Zone::Exile);
    assert_eq!(zone_by_stable_id(&game, bob_b_stable), Zone::Exile);
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0
    );

    let mut resolution_decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context =
        crate::effects::ExecutionContext::new(source, alice, &mut resolution_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Night Soil's token effect should resolve");

    let saprolings = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|object| {
            object.owner == alice
                && object.kind == crate::object::ObjectKind::Token
                && object.subtypes.contains(&Subtype::Saproling)
        })
        .collect::<Vec<_>>();
    assert_eq!(saprolings.len(), 1);
    assert_eq!(saprolings[0].power(), Some(1));
    assert_eq!(saprolings[0].toughness(), Some(1));
}
