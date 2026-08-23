#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn life_amounts(definition: &CardDefinition) -> Vec<(bool, Value)> {
    fn collect(effect: &crate::effect::Effect, amounts: &mut Vec<(bool, Value)>) {
        if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
            amounts.push((false, lose.amount.clone()));
        }
        if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
            amounts.push((true, gain.amount.clone()));
        }
        effect.visit_child_effects(&mut |child| collect(child, amounts));
    }

    let mut amounts = Vec::new();
    for ability in &definition.abilities {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            continue;
        };
        for segment in &triggered.effects.segments {
            for effect in &segment.default_effects {
                collect(effect, &mut amounts);
            }
        }
    }
    amounts
}

#[test]
fn dying_wish_binds_both_x_life_amounts_to_the_same_tagged_lki_power() {
    let definition = parse_oracle_card_definition("Dying Wish");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Enchant creature you control",
            "When enchanted creature dies, target player loses X life and you gain X life, where X is its power."
        ]
    );

    let amounts = life_amounts(&definition);
    let [(false, loss), (true, gain)] = amounts.as_slice() else {
        panic!("expected exactly one loss and one gain amount: {amounts:#?}");
    };
    assert_eq!(loss, gain, "both X uses must share one typed value");
    let Value::PowerOf(spec) = loss.unhinted() else {
        panic!("Dying Wish must use the dead creature's LKI power: {loss:#?}");
    };
    assert!(
        matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"),
        "the shared power must retain triggering-object identity: {loss:#?}"
    );
    assert!(loss.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
}

#[derive(Debug)]
struct TargetPlayerDecisionMaker(PlayerId);

impl crate::decision::DecisionMaker for TargetPlayerDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        vec![crate::game_state::Target::Player(self.0)]
    }
}

#[test]
fn dying_wish_executes_both_life_changes_from_the_dead_creatures_lki_power() {
    let definition = parse_oracle_card_definition("Dying Wish");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let wish = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let host_definition = CardDefinitionBuilder::new(CardId::new(), "Wish Host")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 3))
        .build();
    let host = game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(wish, crate::object::AttachmentTarget::Object(host)));

    game.move_object_by_sba(host, Zone::Graveyard)
        .expect("enchanted creature should die");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == wish)
            .count(),
        1,
        "Dying Wish should trigger from its enchanted creature's real death event"
    );

    let mut decisions = TargetPlayerDecisionMaker(bob);
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Dying Wish's target should be chosen");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Dying Wish should resolve");

    assert_eq!(game.player(alice).expect("Alice exists").life, 25);
    assert_eq!(game.player(bob).expect("Bob exists").life, 15);
}
