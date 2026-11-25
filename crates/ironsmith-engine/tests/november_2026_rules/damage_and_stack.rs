use super::{A, game, put, resolve_spell};
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::{
    DecisionMaker, GameState, LegalAction, ManaSymbol, PlayerId, Subtype, Target, Zone,
};

const B: PlayerId = PlayerId(1);

#[test]
fn alternative_cycling_cost_pays_activation_taxes_and_discards_the_card() {
    for modifier in ["gavi-nest-warden", "new-perspectives"] {
        for route in ["ash-barrens", "street-wraith"] {
            let mut g = game();
            put(&mut g, modifier, A, Zone::Battlefield);
            put(&mut g, "suppression-field", B, Zone::Battlefield);
            let id = put(&mut g, route, A, Zone::Hand);
            for _ in 0..6 {
                put(&mut g, "island", A, Zone::Hand);
            }
            let action = |g: &GameState| {
                ironsmith::compute_legal_actions(g, A)
                    .unwrap()
                    .into_iter()
                    .find(
                        |a| matches!(a, LegalAction::ActivateAbility {source, ..} if *source == id),
                    )
            };
            assert!(action(&g).is_none(), "{modifier} / {route}");
            g.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 2);
            let selected = action(&g).unwrap();
            let stable = g.object(id).unwrap().stable_id;
            let name = g.object(id).unwrap().name.to_string();
            let mut queue = ironsmith::TriggerQueue::new();
            let mut state = ironsmith::PriorityLoopState::new(g.players_in_game());
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut queue,
                &mut state,
                &ironsmith::PriorityResponse::PriorityAction(selected),
                &mut dm,
            )
            .unwrap();
            while !g.stack.iter().any(|e| e.is_ability && e.object_id == id) {
                let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("activation stopped during payment");
                };
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut g, &mut queue, &mut state, &ctx, &mut dm,
                )
                .unwrap();
            }
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(g.player(A).unwrap().life, 20);
            assert_eq!(
                g.object(g.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Graveyard
            );
            assert_eq!(
                g.player(A).unwrap().cycled_card_names_this_game.get(&name),
                Some(&1)
            );
        }
    }
}

#[test]
fn traveling_chocobo_doubles_both_branches_of_its_entry_filter() {
    for (entrant, observer, expected) in [
        ("forest", "tireless-tracker", 2),
        ("sage-owl", "impact-tremors", 2),
        ("street-wraith", "impact-tremors", 1),
    ] {
        let mut g = game();
        put(&mut g, "traveling-chocobo", A, Zone::Battlefield);
        let watcher = put(&mut g, observer, A, Zone::Battlefield);
        put(&mut g, entrant, A, Zone::Hand);
        let _ = g.take_pending_trigger_events();
        resolve_spell(
            &mut g,
            "show-and-tell",
            vec![],
            &mut SelectFirstDecisionMaker,
        );
        let mut queue = ironsmith::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
        assert_eq!(
            queue.entries.iter().filter(|e| e.source == watcher).count(),
            expected,
            "{entrant}"
        );
    }
}

#[test]
fn the_wanderer_prevents_noncombat_damage_to_its_controller_and_other_permanents() {
    let mut g = game();
    put(&mut g, "the-wanderer", B, Zone::Battlefield);
    let owl = put(&mut g, "sage-owl", B, Zone::Battlefield);
    for target in [Target::Player(A), Target::Player(B), Target::Object(owl)] {
        resolve_spell(
            &mut g,
            "lightning-bolt",
            vec![target],
            &mut SelectFirstDecisionMaker,
        );
    }
    assert_eq!(
        (g.player(A).unwrap().life, g.player(B).unwrap().life),
        (17, 20)
    );
    assert_eq!(g.damage_on(owl), 0);
}

#[test]
fn conspiracy_applies_after_changeling_in_every_zone() {
    let mut g = game();
    let enchantment = put(&mut g, "conspiracy", A, Zone::Battlefield);
    g.set_chosen_creature_type(enchantment, Subtype::Elf);
    for zone in [
        Zone::Library,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Stack,
        Zone::Exile,
        Zone::Battlefield,
    ] {
        let id = put(&mut g, "amoeboid-changeling", A, zone);
        assert_eq!(g.current_subtypes(id).unwrap(), [Subtype::Elf], "{zone:?}");
    }
}

#[test]
fn removing_graveyard_abilities_preserves_the_earlier_changeling_type_effect() {
    let mut g = game();
    let id = put(&mut g, "nameless-inversion", A, Zone::Graveyard);
    put(&mut g, "yixlid-jailer", B, Zone::Battlefield);
    assert!(g.current_abilities(id).unwrap().is_empty());
    assert!(g.current_subtypes(id).unwrap().contains(&Subtype::Zombie));
    assert!(g.current_subtypes(id).unwrap().contains(&Subtype::Knight));
}

#[test]
fn characteristic_power_and_toughness_counts_read_graveyard_changelings() {
    for (source, owner, expected) in [
        ("soulless-one", B, 2),
        ("abomination-of-llanowar", A, 2),
        ("abomination-of-llanowar", B, 1),
    ] {
        let mut g = game();
        let id = put(&mut g, source, A, Zone::Battlefield);
        put(&mut g, "amoeboid-changeling", owner, Zone::Graveyard);
        assert_eq!(
            (g.current_power(id), g.current_toughness(id)),
            (Some(expected), Some(expected)),
            "{source}"
        );
    }
}

#[test]
fn life_total_changes_preserve_the_source_of_the_gain_for_rain_of_gore() {
    for (spell, recipients, expected) in [
        ("beacon-of-immortality", vec![A], (0, 16)),
        ("blessed-wind", vec![A], (4, 16)),
        ("psychic-transfer", vec![B], (8, 12)),
        ("beacon-of-immortality", vec![B], (12, 32)),
    ] {
        let mut g = game();
        g.player_mut(A).unwrap().life = 12;
        g.player_mut(B).unwrap().life = 16;
        put(&mut g, "rain-of-gore", B, Zone::Battlefield);
        resolve_spell(
            &mut g,
            spell,
            recipients.into_iter().map(Target::Player).collect(),
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(
            (g.player(A).unwrap().life, g.player(B).unwrap().life),
            expected,
            "{spell}"
        );
    }
}

#[test]
fn replacing_life_gain_with_nothing_allows_exchange_and_replacement_order() {
    let mut g = game();
    put(&mut g, "sulfuric-vortex", B, Zone::Battlefield);
    g.player_mut(A).unwrap().life = 12;
    g.player_mut(B).unwrap().life = 16;
    resolve_spell(
        &mut g,
        "psychic-transfer",
        vec![Target::Player(B)],
        &mut SelectFirstDecisionMaker,
    );
    assert_eq!(
        (g.player(A).unwrap().life, g.player(B).unwrap().life),
        (12, 12)
    );

    struct ChooseLast(bool);
    impl DecisionMaker for ChooseLast {
        fn decide_options(&mut self, g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if self.0 && ctx.min == 1 && ctx.max == 1 {
                ctx.options
                    .iter()
                    .rev()
                    .find(|o| o.legal)
                    .map(|o| o.index)
                    .into_iter()
                    .collect()
            } else {
                SelectFirstDecisionMaker.decide_options(g, ctx)
            }
        }
    }
    put(&mut g, "tainted-remedy", B, Zone::Battlefield);
    let mut totals = [false, true].map(|last| {
        let mut g = g.clone();
        resolve_spell(&mut g, "sacred-nectar", vec![], &mut ChooseLast(last));
        g.player(A).unwrap().life
    });
    totals.sort();
    assert_eq!(totals, [8, 12]);
}
