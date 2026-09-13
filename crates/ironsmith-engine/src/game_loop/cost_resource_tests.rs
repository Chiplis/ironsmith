use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::{CardBuilder, PowerToughness};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};

fn fixture(harmonize: bool) -> (GameState, PriorityLoopState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardBuilder::new(CardId::new(), "Chosen creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::new().add_generic(2))
        .power_toughness(PowerToughness::fixed(3, 3)).build();
    let creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let card = CardBuilder::new(CardId::new(), "Resource spell")
        .card_types(vec![CardType::Instant]).mana_cost(ManaCost::new().add_generic(3)).build();
    let spell = game.create_object_from_card(&card, alice, Zone::Stack);
    let method = if harmonize { AlternativeCastingMethod::Harmonize { total_cost: crate::cost::TotalCost::mana(ManaCost::new().add_generic(3)) } }
        else { AlternativeCastingMethod::alternative_cost("Emerge", Some(ManaCost::new().add_generic(3)), vec![crate::costs::Cost::sacrifice(ObjectFilter::creature().you_control())]) };
    game.object_mut(spell).unwrap().alternative_casts = vec![method].into();
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let pending = PendingCast::new(spell, if harmonize { Zone::Graveyard } else { Zone::Hand }, alice,
        ProvNodeId::default(), CastStage::ChoosingCostResource, None, vec![], CastingMethod::Alternative(0),
        OptionalCostsPaid::default(), None, spell);
    let mut state = PriorityLoopState::new(2);
    state.save_checkpoint(&game);
    state.pending_cast = Some(pending);
    (game, state, creature, spell)
}

#[test]
fn harmonize_announces_without_tapping_and_pays_chosen_creature() {
    let (mut game, mut state, creature, _) = fixture(true);
    let mut queue = TriggerQueue::new();
    let pending = state.pending_cast.take().unwrap();
    let prompt = check_x_or_continue(&mut game, &mut queue, &mut state, pending, &mut SelectFirstDecisionMaker).unwrap();
    assert!(matches!(prompt, GameProgress::NeedsDecisionCtx(_)));
    assert!(!game.is_tapped(creature), "announcing a resource must not pay it yet");
    let result = apply_cost_resource_response(&mut game, &mut queue, &mut state, 1, &mut SelectFirstDecisionMaker);
    assert!(result.is_ok(), "{result:?}");
    // All generic mana was covered by the chosen creature. The remaining
    // executable nonmana cost is paid through the ordinary cost pipeline.
    assert!(game.is_tapped(creature));
    assert_eq!(game.player(PlayerId::from_index(0)).unwrap().mana_pool.total(), 3);
}

#[test]
fn harmonize_can_be_declined_and_emerge_locks_the_selected_sacrifice() {
    for harmonize in [true, false] {
        let (mut game, mut state, creature, _) = fixture(harmonize);
        let mut queue = TriggerQueue::new();
        let result = apply_cost_resource_response(&mut game, &mut queue, &mut state, 0, &mut SelectFirstDecisionMaker);
        assert!(result.is_ok(), "{result:?}");
        assert!(!game.is_tapped(creature));
        let pending = state.pending_cast.as_ref().expect("mana payment remains");
        assert_eq!(pending.cost_resource, if harmonize { None } else { Some(creature) });
        assert_eq!(pending.mana_cost_to_pay.as_ref().unwrap().generic_mana_total(), if harmonize { 3 } else { 1 });
    }
}

#[test]
fn offering_locks_typed_reduction_and_sacrifice() {
    let (mut game, mut state, creature, spell) = fixture(false);
    let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Green], vec![ManaSymbol::Blue]]);
    game.object_mut(creature).unwrap().mana_cost = Some(mana.into());
    let optional = crate::cost::OptionalCost::custom("Offering", crate::cost::TotalCost::from_cost(
        crate::costs::Cost::sacrifice(ObjectFilter::creature().you_control())));
    let paid = optional.cost_ref();
    game.object_mut(spell).unwrap().optional_costs = vec![optional].into();
    game.object_mut(spell).unwrap().mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Green]]).into());
    let pending = state.pending_cast.as_mut().unwrap();
    pending.casting_method = CastingMethod::Normal;
    pending.optional_costs_paid.mark_label_paid(paid);
    let mut queue = TriggerQueue::new();
    apply_cost_resource_response(&mut game, &mut queue, &mut state, 0, &mut SelectFirstDecisionMaker).unwrap();
    let pending = state.pending_cast.as_ref().expect("remaining generic payment");
    assert_eq!(pending.cost_resource, Some(creature));
    assert_eq!(pending.mana_cost_to_pay.as_ref().unwrap(), &ManaCost::new().add_generic(2));
}

#[test]
fn offering_excess_typed_mana_reduces_only_generic() {
    let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Green], vec![ManaSymbol::Colorless]]);
    let offered = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Green], vec![ManaSymbol::Blue]]);
    assert_eq!(crate::decision::reduce_offering_mana_cost(&cost, &offered),
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Colorless]]));
}

#[test]
fn graveyard_payment_replacements_apply_to_counter_and_bounce() {
    for bounce in [false, true] {
        for method in [
            AlternativeCastingMethod::Flashback { total_cost: crate::cost::TotalCost::mana(ManaCost::new()) },
            AlternativeCastingMethod::Harmonize { total_cost: crate::cost::TotalCost::mana(ManaCost::new()) },
            AlternativeCastingMethod::JumpStart { additional_cost: crate::cost::TotalCost::from_costs(vec![]) },
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let card = CardBuilder::new(CardId::new(), "Graveyard payment probe")
                .card_types(vec![CardType::Instant]).mana_cost(ManaCost::new()).build();
            let method_name = method.name().to_string();
            let source = game.create_object_from_card(&card, alice, Zone::Graveyard);
            game.object_mut(source).unwrap().alternative_casts = vec![method].into();
            let mut dm = SelectFirstDecisionMaker;
            let stack = cast_spell_from_resolving_effect(&mut game, source, Zone::Graveyard, alice,
                &CastingMethod::Alternative(0), false, None, ProvNodeId::default(), &mut dm).unwrap().unwrap();
            let effect = if bounce { crate::effect::Effect::move_to_zone(ChooseSpec::SpecificObject(stack), Zone::Hand, true) }
                else { crate::effect::Effect::counter(ChooseSpec::SpecificObject(stack)) };
            let mut ctx = crate::effects::ExecutionContext::new(stack, alice, &mut dm);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Graveyard payment probe"),
                "{method_name}: paid graveyard keyword must exile on counter or bounce={bounce}; zones={:?}", game.object_ids_in_deterministic_order().iter().map(|id| (*id, game.object(*id).unwrap().zone)).collect::<Vec<_>>());
        }
    }
}
