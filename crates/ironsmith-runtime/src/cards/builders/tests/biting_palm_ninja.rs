#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::effects::ExecutionContext;

const ORACLE_TEXT: &str = "Ninjutsu {2}{B}\nThis creature enters with a menace counter on it.\nWhenever this creature deals combat damage to a player, you may remove a menace counter from it. When you do, that player reveals their hand and you choose a nonland card from it. Exile that card.";

#[derive(Default)]
struct BitingPalmDecisions {
    chosen: Option<ObjectId>,
    legal_hand_choices: Vec<ObjectId>,
    revealed_hands: Vec<(PlayerId, Vec<ObjectId>)>,
}

impl DecisionMaker for BitingPalmDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.legal_hand_choices = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        self.chosen
            .filter(|chosen| self.legal_hand_choices.contains(chosen))
            .into_iter()
            .collect()
    }

    fn view_cards(
        &mut self,
        _game: &crate::GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        if ctx.public {
            self.revealed_hands.push((viewer, cards.to_vec()));
        }
    }
}

fn triggered_ability(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Biting-Palm Ninja should have its combat-damage trigger")
}

fn card(name: &str, card_type: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder.power_toughness(PowerToughness::fixed(1, 1)).build()
    } else {
        builder.build()
    }
}

#[test]
fn biting_palm_ninja_keeps_the_typed_result_boundary_and_exact_surface() {
    let definition = parse_oracle_card_definition("Biting-Palm Ninja");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE_TEXT
    );

    let effects = triggered_ability(&definition)
        .effects
        .flattened_default_effects();
    let reflexive = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>())
        .expect("the successful optional removal should create a reflexive trigger");
    let [result_effect, exile_effect] = reflexive.effects.as_slice() else {
        panic!("the reflexive branch should keep its two authored sentences: {reflexive:#?}");
    };
    let result = result_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the reveal and card choice should remain one typed result conjunction");
    assert_eq!(
        result.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false,
        }
    );
    let [look_effect, choose_effect] = result.effects.as_slice() else {
        panic!("the result conjunction should contain reveal then choice: {result:#?}");
    };
    let look = look_effect
        .downcast_ref::<crate::effects::LookAtHandEffect>()
        .expect("the damaged player's hand should be revealed");
    assert!(look.reveal);
    assert_eq!(look.target, ChooseSpec::Player(PlayerFilter::DamagedPlayer));
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the controller should choose from the revealed hand");
    assert_eq!(choose.tag.as_str(), "__it__");
    assert_eq!(choose.zone, Some(Zone::Hand));
    assert_eq!(choose.filter.owner, Some(PlayerFilter::DamagedPlayer));
    assert!(choose.filter.excluded_card_types.contains(&CardType::Land));
    let exile = exile_effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("the second sentence should exile the tagged choice");
    assert_eq!(exile.zone, Zone::Exile);
    assert_eq!(exile.target, ChooseSpec::Tagged("__it__".into()));

    let debug = format!("{:#?}", reflexive.effects);
    assert!(
        debug.contains("__revealed_this_way__"),
        "the hand choice must be restricted to the exact revealed set: {debug}"
    );
}

#[test]
fn biting_palm_ninja_removes_its_counter_then_exiles_only_the_chosen_nonland() {
    let definition = parse_oracle_card_definition("Biting-Palm Ninja");
    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ninja = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(ninja, crate::object::CounterType::Menace, 1)
        .expect("the test Ninja should receive its menace counter");

    let nonland = game.create_object_from_definition(
        &card("Biting-Palm Chosen Nonland", CardType::Sorcery),
        bob,
        Zone::Hand,
    );
    let land = game.create_object_from_definition(
        &card("Biting-Palm Land Near Miss", CardType::Land),
        bob,
        Zone::Hand,
    );
    let nonland_stable = game
        .object(nonland)
        .expect("chosen nonland should exist")
        .stable_id;
    let land_stable = game
        .object(land)
        .expect("near-miss land should exist")
        .stable_id;

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            ninja,
            crate::events::DamageTarget::Player(bob),
            3,
            true,
            crate::events::cause::EventCause::combat_damage(ninja),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut decisions = BitingPalmDecisions {
        chosen: Some(nonland),
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new_default(ninja, alice)
        .with_decision_maker(&mut decisions)
        .with_triggering_event(damage_event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        ninja,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Biting-Palm Ninja's combat-damage trigger should resolve");
    drop(ctx);

    assert_eq!(
        game.counter_count(ninja, crate::object::CounterType::Menace),
        0,
        "accepting the may instruction should remove exactly one menace counter"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the successful removal should create its reflexive ability"
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Biting-Palm Ninja's reflexive ability should resolve");

    assert_eq!(
        decisions.legal_hand_choices,
        vec![nonland],
        "the land is the executable near miss and must not be selectable"
    );
    assert!(
        decisions
            .revealed_hands
            .iter()
            .any(|(viewer, cards)| *viewer == alice
                && cards.contains(&nonland)
                && cards.contains(&land)),
        "the damaged player's whole hand should be revealed to the trigger controller"
    );
    assert_eq!(
        game.find_object_by_stable_id(nonland_stable)
            .and_then(|id| game.object(id))
            .expect("chosen card should remain tracked")
            .zone,
        Zone::Exile
    );
    assert_eq!(
        game.find_object_by_stable_id(land_stable)
            .and_then(|id| game.object(id))
            .expect("unchosen land should remain tracked")
            .zone,
        Zone::Hand,
        "the nonland filter and chosen-card tag must prevent the near-miss land from being exiled"
    );
}
