use super::{A, Menu, can_cast, game, put, resolve_spell};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::object::AttachmentTarget;
use ironsmith::{
    AbilityKind, Color, CounterType, DecisionMaker, GameState, ManaSymbol, ObjectId, PlayerId,
    StackEntry, Target, Zone,
};

const B: PlayerId = PlayerId(1);

fn resolve_last_activated(game: &mut GameState, source: ObjectId) {
    let program = game
        .current_abilities(source)
        .unwrap()
        .into_iter()
        .rev()
        .find_map(|ability| match ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects),
            _ => None,
        })
        .unwrap();
    game.push_to_stack(StackEntry::ability(source, A, program));
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut Menu::default()).unwrap();
}

#[test]
fn imprint_reads_exiled_cards_as_the_board_changes() {
    for (route, bonus) in [("duplicant", 0), ("phyrexian-ingester", 3)] {
        let mut g = game();
        let source = put(&mut g, route, A, Zone::Battlefield);
        let donor = put(&mut g, "battle-squadron", B, Zone::Exile);
        g.add_exiled_with_source_link(source, donor);
        put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        assert_eq!(g.current_power(source), Some(bonus + 1));
        put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        assert_eq!(g.current_toughness(source), Some(bonus + 2));
        if route == "duplicant" {
            let nexus = put(&mut g, "maskwood-nexus", B, Zone::Battlefield);
            assert!(g.current_has_subtype(source, ironsmith::Subtype::Dragon));
            g.phase_out(nexus);
            assert!(!g.current_has_subtype(source, ironsmith::Subtype::Dragon));
            assert!(g.current_has_subtype(source, ironsmith::Subtype::Shapeshifter));
        }
    }
}

#[test]
fn linked_exile_mana_reads_current_colors_and_ignores_absent_color_effects() {
    for route in ["chrome-mox", "pit-of-offerings"] {
        let mut g = game();
        let source = put(&mut g, route, A, Zone::Battlefield);
        let link = put(&mut g, "divination", A, Zone::Exile);
        if route == "chrome-mox" {
            g.imprint_card(source, link);
        } else {
            g.add_exiled_with_source_link(source, link);
        }
        let ritual = put(&mut g, "dark-ritual", A, Zone::Hand);
        let opt = put(&mut g, "opt", A, Zone::Hand);
        assert!(can_cast(&g, opt));
        assert!(!can_cast(&g, ritual));
        let lattice = put(&mut g, "mycosynth-lattice", B, Zone::Battlefield);
        // Pit can still produce colorless, which Lattice allows its controller to spend as blue.
        assert_eq!(can_cast(&g, opt), route == "pit-of-offerings");
        resolve_last_activated(&mut g, source);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0, "{route}");
        g.phase_out(lattice);
        resolve_last_activated(&mut g, source);
        assert_eq!(g.player(A).unwrap().mana_pool.blue, 1, "{route}");
        let painter = put(&mut g, "painter-s-servant", B, Zone::Battlefield);
        g.set_chosen_color(painter, Color::Black);
        assert!(can_cast(&g, ritual), "{route}");
    }
}

#[test]
fn colors_among_permanents_observe_layer_five_in_static_and_activated_abilities() {
    let mut g = game();
    let elder = put(&mut g, "faeburrow-elder", A, Zone::Battlefield);
    g.add_counters(elder, CounterType::PlusOnePlusOne, 1);
    let orrery = put(&mut g, "chromatic-orrery", A, Zone::Battlefield);
    for _ in 0..3 {
        put(&mut g, "forest", A, Zone::Library);
    }
    let lattice = put(&mut g, "mycosynth-lattice", B, Zone::Battlefield);
    assert_eq!(g.current_power(elder), Some(1));
    resolve_last_activated(&mut g, orrery);
    assert!(g.player(A).unwrap().hand.is_empty());
    g.phase_out(lattice);
    let painter = put(&mut g, "painter-s-servant", B, Zone::Battlefield);
    g.set_chosen_color(painter, Color::Blue);
    assert_eq!(g.current_power(elder), Some(4));
    resolve_last_activated(&mut g, orrery);
    assert_eq!(g.player(A).unwrap().hand.len(), 3);
}

#[test]
fn graveyard_type_changing_effects_apply_to_characteristic_defining_abilities_and_delirium_costs() {
    let mut g = game();
    let goyf = put(&mut g, "tarmogoyf", A, Zone::Battlefield);
    let scavenger = put(&mut g, "nighthawk-scavenger", B, Zone::Battlefield);
    for route in ["grizzly-bears", "forest", "divination"] {
        put(&mut g, route, A, Zone::Graveyard);
    }
    let feaster = put(&mut g, "dusk-feaster", A, Zone::Hand);
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 5);
    assert!(!can_cast(&g, feaster));
    assert_eq!(g.current_power(goyf), Some(3));
    let transfer = put(&mut g, "biotransference", A, Zone::Battlefield);
    assert_eq!(g.current_power(goyf), Some(4));
    assert_eq!(g.current_power(scavenger), Some(5));
    assert!(can_cast(&g, feaster));
    g.phase_out(transfer);
    assert_eq!(g.current_power(goyf), Some(3));
    assert!(!can_cast(&g, feaster));
}

#[test]
fn mana_cost_and_distinct_mana_values_follow_layer_one_copies() {
    let mut g = game();
    let lions = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let giant = put(&mut g, "hill-giant", B, Zone::Battlefield);
    let aura = put(&mut g, "elemental-resonance", B, Zone::Battlefield);
    assert!(g.attach_object_to_target(aura, AttachmentTarget::Object(lions)));
    for _ in 0..3 {
        put(&mut g, "forest", A, Zone::Library);
    }
    let mut dm = Menu::default();
    resolve_spell(&mut g, "mirrorweave", vec![Target::Object(giant)], &mut dm);
    resolve_spell(&mut g, "lunar-insight", vec![], &mut dm);
    assert_eq!(g.player(A).unwrap().hand.len(), 1);

    let trigger = g
        .current_abilities(aura)
        .unwrap()
        .into_iter()
        .find_map(|ability| match ability.kind {
            AbilityKind::Triggered(ability) => Some(ability.effects),
            _ => None,
        })
        .unwrap();
    g.push_to_stack(StackEntry::ability(aura, B, trigger));
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    let mana = &g.player(B).unwrap().mana_pool;
    assert_eq!((mana.red, mana.colorless, mana.total()), (1, 3, 4));
}

#[test]
fn reverse_the_sands_can_only_redistribute_life_totals_within_range() {
    #[derive(Default)]
    struct Swap(Vec<String>);
    impl DecisionMaker for Swap {
        fn decide_boolean(&mut self, _: &GameState, choice: &BooleanContext) -> bool {
            self.0.push(choice.description.clone());
            choice.description.contains("Alice") || choice.description.contains("Cara")
        }
        fn decide_options(&mut self, _: &GameState, choice: &SelectOptionsContext) -> Vec<usize> {
            vec![
                choice
                    .options
                    .iter()
                    .find(|option| option.description == "7 life")
                    .unwrap()
                    .index,
            ]
        }
    }
    for limited in [false, true] {
        let seats = ["Alice", "Bob", "Cara", "Dan", "Eve"].map(String::from);
        let mut g = GameState::new(seats.into(), 20);
        if limited {
            g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
                .unwrap();
        }
        let cara = PlayerId(2);
        g.player_mut(cara).unwrap().life = 7;
        let mut choices = Swap::default();
        resolve_spell(&mut g, "reverse-the-sands", vec![], &mut choices);
        assert_eq!(choices.0.len(), if limited { 3 } else { 5 });
        let totals = (g.player(A).unwrap().life, g.player(cara).unwrap().life);
        assert_eq!(totals, if limited { (20, 7) } else { (7, 20) });
    }
}

#[test]
fn color_counts_agree_in_filter_comparisons_and_combined_spell_history() {
    use ironsmith::effects::helpers::resolve_value;
    use ironsmith::filter::Comparison;
    use ironsmith::{EffectContext, ObjectFilter, Value};
    let mut g = game();
    let lions = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let ctx = EffectContext::new_default(lions, A);
    let colors = Value::ColorsAmong(ObjectFilter::permanent().you_control());
    let matching =
        Value::Count(ObjectFilter::creature().with_power(Comparison::EqualExpr(Box::new(colors))));
    let combined = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(
            ironsmith::PlayerFilter::You,
        ),
    );
    assert_eq!(resolve_value(&g, &matching, &ctx).unwrap(), 2);
    assert_eq!(resolve_value(&g, &combined, &ctx).unwrap(), 2);
    let spell = put(&mut g, "divination", A, Zone::Stack);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        g.object(spell).unwrap(),
        &g,
    );
    let cast = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            A,
            Zone::Hand,
            snapshot.clone(),
        ),
        ironsmith::provenance::ProvNodeId::default(),
    );
    g.turn_store
        .turn_history
        .record_event(&cast, Some(snapshot), None);
    assert_eq!(resolve_value(&g, &combined, &ctx).unwrap(), 3);
    put(&mut g, "mycosynth-lattice", B, Zone::Battlefield);
    assert_eq!(resolve_value(&g, &matching, &ctx).unwrap(), 0);
    assert_eq!(resolve_value(&g, &combined, &ctx).unwrap(), 1);
}

#[test]
fn friend_or_foe_designations_use_the_effect_controllers_range() {
    use ironsmith::effects::ChooseFriendsOrFoesEffect;
    use ironsmith::{EffectContext, EffectExecutor};
    struct Foes;
    impl DecisionMaker for Foes {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            false
        }
    }
    let seats = ["Alice", "Bob", "Cara", "Dan", "Eve"].map(String::from);
    let mut g = GameState::new(seats.into(), 20);
    g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
        .unwrap();
    let source = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    let mut choices = Foes;
    let mut ctx = EffectContext::new(source, A, &mut choices);
    ChooseFriendsOrFoesEffect::new("friends", "foes")
        .execute(&mut g, &mut ctx)
        .unwrap();
    assert_eq!(
        ctx.get_tagged_players("foes").unwrap(),
        &[A, B, PlayerId(4)]
    );
}
