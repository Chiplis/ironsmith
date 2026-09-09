use super::*;
const TEXT: &str = "{T}: Add {R}{R}.\nAt the beginning of each player's upkeep, that player may put a doom counter on this artifact or remove a doom counter from it. Then if it has three or more doom counters on it, sacrifice this artifact. When you do, it deals 6 damage to each creature.";
struct CounterDecision {
    player: crate::ids::PlayerId,
    accept: bool,
    mode: usize,
    options: usize,
}
impl crate::decision::DecisionMaker for CounterDecision {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(
            ctx.player, self.player,
            "the upkeep player chooses whether to act"
        );
        self.accept
    }
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(
            ctx.player, self.player,
            "the upkeep player chooses which counter action"
        );
        assert!(ctx.options.iter().any(|o| o.index == self.mode && o.legal));
        self.options += 1;
        vec![self.mode]
    }
}
#[test]
fn lavabrink_counter_sacrifice_upkeep_choice_and_reflexive_damage() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Lavabrink Floodgates")
            .card_types(vec![CardType::Artifact])
            .parse_text(TEXT)
            .unwrap();
    for (initial, accept, mode, explodes) in [
        (2, true, 0, true),
        (3, true, 1, false),
        (3, false, 0, true),
        (2, false, 0, false),
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.add_counters(source, CounterType::Named("doom".into()), initial);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Damage Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 10))
            .build();
        let first = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::BeginningOfUpkeepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let mut dm = CounterDecision {
            player: bob,
            accept,
            mode,
            options: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.options, usize::from(accept));
        assert_eq!(
            game.battlefield.contains(&source),
            !explodes,
            "initial={initial}, accept={accept}, mode={mode}"
        );
        assert_eq!(
            game.stack.len(),
            usize::from(explodes),
            "only a successful sacrifice creates the reflexive trigger"
        );
        if explodes {
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        assert_eq!(game.damage_on(first), if explodes { 6 } else { 0 });
        assert_eq!(game.damage_on(second), if explodes { 6 } else { 0 });
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn lavabrink_counter_sacrifice_renders_the_actual_source() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Lavabrink Floodgates")
        .card_types(vec![CardType::Artifact]).parse_text(TEXT).unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(!rendered.contains("hand"), "{rendered}");
    assert!(rendered.contains("that player may put a doom counter on"), "{rendered}");
    assert!(rendered.contains("When you do, it deals 6 damage to each creature.")
        || rendered.contains("When you do, this artifact deals 6 damage to each creature."), "{rendered}");
}
