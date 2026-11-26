use super::{A, Menu, card, game, put, resolve_spell};
use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers, declare_blockers};
use ironsmith::{
    CardDefinition, CastingMethod, Color, GameState, LegalAction, ManaSymbol, ObjectId, Phase,
    PlayerId, Step, Target, Zone,
};
const B: PlayerId = PlayerId(1);

fn cast_action(g: &GameState, card: ObjectId, method: &CastingMethod) -> Option<LegalAction> {
    ironsmith::compute_legal_actions(g, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card && casting_method == method))
}

fn spend(g: &mut GameState, card: ObjectId, method: CastingMethod) -> u32 {
    let stable = g.object(card).unwrap().stable_id;
    let before = g.player(A).unwrap().mana_pool.total();
    let action = cast_action(g, card, &method).expect("offered cast");
    let mut state = ironsmith::PriorityLoopState::new(g.players_in_game());
    let mut queue = ironsmith::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &ironsmith::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    loop {
        let paid = state.pending_cast.is_none() && state.pending_method_selection.is_none();
        if paid
            && g.find_object_by_stable_id(stable).is_some_and(|id| {
                g.stack
                    .iter()
                    .any(|entry| entry.object_id == id && !entry.is_ability)
            })
        {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("cast stopped before payment");
        };
        progress = if let Some(choice) = &state.pending_method_selection {
            let index = choice
                .available_methods
                .iter()
                .position(|c| c.method == method)
                .unwrap();
            ironsmith::game_loop::apply_priority_response_with_dm(
                g,
                &mut queue,
                &mut state,
                &ironsmith::PriorityResponse::CastingMethodChoice(index),
                &mut dm,
            )
        } else {
            ironsmith::game_loop::apply_decision_context_with_dm(
                g, &mut queue, &mut state, &ctx, &mut dm,
            )
        }
        .unwrap();
    }
    before - g.player(A).unwrap().mana_pool.total()
}

#[test]
fn spirit_guides_pay_during_casting() {
    for (guide, spell) in [
        ("elvish-spirit-guide", "llanowar-elves"),
        ("simian-spirit-guide", "lightning-bolt"),
    ] {
        let mut g = game();
        let spell = put(&mut g, spell, A, Zone::Hand);
        assert!(cast_action(&g, spell, &CastingMethod::Normal).is_none());
        let source = put(&mut g, guide, A, Zone::Hand);
        let guide_stable = g.object(source).unwrap().stable_id;
        assert_eq!(spend(&mut g, spell, CastingMethod::Normal), 0);
        let paid = g.find_object_by_stable_id(guide_stable).unwrap();
        assert_eq!(g.object(paid).unwrap().zone, Zone::Exile);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn stack_colors_and_types_determine_affordability_and_actual_payment() {
    for (route, change, modifier, method, expected) in [
        (
            "grizzly-bears",
            "painter-s-servant",
            "chill",
            CastingMethod::Normal,
            4,
        ),
        (
            "hill-giant",
            "mycosynth-lattice",
            "chill",
            CastingMethod::Normal,
            4,
        ),
        ("ghostfire", "", "chill", CastingMethod::Normal, 3),
        (
            "sphinx-of-the-guildpact",
            "",
            "chill",
            CastingMethod::Normal,
            9,
        ),
        (
            "grizzly-bears",
            "biotransference",
            "foundry-inspector",
            CastingMethod::Normal,
            1,
        ),
        (
            "willbender",
            "painter-s-servant",
            "chill",
            CastingMethod::FaceDown,
            5,
        ),
    ] {
        let mut g = game();
        put(&mut g, modifier, A, Zone::Battlefield);
        if !change.is_empty() {
            let source = put(&mut g, change, A, Zone::Battlefield);
            if change == "painter-s-servant" {
                g.set_chosen_color(source, Color::Red);
            }
        }
        let id = put(&mut g, route, A, Zone::Hand);
        let symbol = if route == "grizzly-bears" {
            ManaSymbol::Green
        } else {
            ManaSymbol::Red
        };
        g.player_mut(A).unwrap().mana_pool.add(symbol, expected - 1);
        assert!(cast_action(&g, id, &method).is_none(), "{route} too little");
        g.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        assert_eq!(spend(&mut g, id, method), expected, "{route}");
    }
}

fn split(g: &mut GameState) -> (CardDefinition, CardDefinition) {
    use ironsmith::card::LinkedFaceLayout;
    let mut front = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Left",
        "Mana cost: {1}{R}\nType: Sorcery\nDraw a card.",
        false,
    )
    .unwrap();
    let mut back = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Right",
        "Mana cost: {1}{U}\nType: Instant\nDraw a card.",
        false,
    )
    .unwrap();
    front.card.other_face = Some(back.card.id);
    back.card.other_face = Some(front.card.id);
    front.card.other_face_name = Some(back.card.name.clone());
    back.card.other_face_name = Some(front.card.name.clone());
    for face in [&mut front, &mut back] {
        face.card.linked_face_layout = LinkedFaceLayout::Split;
        g.register_linked_face_definition(face);
    }
    (front, back)
}

#[test]
fn iona_checks_global_color_changes_on_the_chosen_face() {
    for change in ["painter-s-servant", "mycosynth-lattice"] {
        let mut g = game();
        let iona = put(&mut g, "iona-shield-of-emeria", B, Zone::Battlefield);
        g.set_chosen_color(iona, Color::Blue);
        let modifier = put(&mut g, change, B, Zone::Battlefield);
        if change == "painter-s-servant" {
            g.set_chosen_color(modifier, Color::Blue);
        }
        for symbol in [ManaSymbol::Red, ManaSymbol::Blue] {
            g.player_mut(A).unwrap().mana_pool.add(symbol, 6);
        }
        let (front, _) = split(&mut g);
        let id = g.create_object_from_definition(&front, A, Zone::Hand);
        let morph = put(&mut g, "willbender", A, Zone::Hand);
        for (object, method) in [
            (id, CastingMethod::Normal),
            (id, CastingMethod::SplitOtherHalf),
            (morph, CastingMethod::FaceDown),
        ] {
            assert_eq!(
                cast_action(&g, object, &method).is_some(),
                change == "mycosynth-lattice",
                "{change} {method:?}"
            );
        }
    }
}

#[test]
fn companion_validators_agree_on_combined_faces_and_pregame_types() {
    for (companion, candidate, filler, allowed) in [
        ("keruga-the-macrosage", "split", None, true),
        ("jegantha-the-wellspring", "split", None, false),
        ("umori-the-collector", "split", Some("opt"), true),
        (
            "kaheera-the-orphanguard",
            "grist-the-hunger-tide",
            None,
            false,
        ),
        (
            "umori-the-collector",
            "grist-the-hunger-tide",
            Some("grizzly-bears"),
            true,
        ),
        ("kaheera-the-orphanguard", "mistform-ultimus", None, true),
    ] {
        let mut g = game();
        let def = if candidate == "split" {
            split(&mut g).0
        } else {
            card(candidate)
        };
        let mut deck = vec![def];
        if let Some(filler) = filler {
            deck.push(card(filler));
        }
        let ids: Vec<_> = deck
            .iter()
            .map(|d| g.create_object_from_definition(d, A, Zone::Library))
            .collect();
        let companion_card = card(companion);
        let id = g.create_object_from_definition(&companion_card, A, Zone::OutsideGame);
        assert_eq!(
            g.validate_companion_definition(&companion_card, &deck, 1)
                .is_ok(),
            allowed,
            "{candidate} {companion} definition"
        );
        assert_eq!(
            g.designate_companion(A, id, &ids, 1).is_ok(),
            allowed,
            "{candidate} {companion} designation"
        );
    }
}

#[test]
fn blocker_declarations_use_the_attacked_player_and_ignore_phased_out_permanents() {
    for (attacker, permanent, is_team, on_teammate, phased, allowed) in [
        ("river-boa", "island", true, false, false, false),
        ("river-boa", "island", true, true, false, true),
        ("bouncing-beebles", "sol-ring", true, false, false, false),
        ("bouncing-beebles", "sol-ring", false, false, true, true),
        (
            "arctic-foxes",
            "snow-covered-forest",
            false,
            false,
            false,
            false,
        ),
        ("hazy-homunculus", "forest", false, false, false, false),
    ] {
        let mut g = if is_team {
            let mut g = GameState::new(
                ["Alice", "Ally", "Bob", "Bea"].map(String::from).to_vec(),
                30,
            );
            g.enable_two_headed_giant(vec![vec![A, B], vec![PlayerId(2), PlayerId(3)]])
                .unwrap();
            g
        } else {
            game()
        };
        let (defender, blocker_owner) = if is_team {
            (PlayerId(2), PlayerId(3))
        } else {
            (B, B)
        };
        let a = put(&mut g, attacker, A, Zone::Battlefield);
        let b = put(&mut g, "wind-drake", blocker_owner, Zone::Battlefield);
        let land = put(
            &mut g,
            permanent,
            if on_teammate { blocker_owner } else { defender },
            Zone::Battlefield,
        );
        if phased {
            g.phase_out(land);
        }
        g.remove_summoning_sickness(a);
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        let mut combat = CombatState::default();
        declare_attackers(
            &mut g,
            &mut combat,
            vec![(a, AttackTarget::Player(defender))],
        )
        .unwrap();
        g.turn.step = Some(Step::DeclareBlockers);
        let offered = ironsmith::compute_legal_blockers(&g, &combat, defender)
            .iter()
            .any(|entry| entry.attacker == a && entry.valid_blockers.contains(&b));
        assert_eq!(offered, allowed, "{attacker} offered");
        assert_eq!(
            declare_blockers(&g, &mut combat, vec![(b, a)]).is_ok(),
            allowed,
            "{attacker} accepted"
        );
    }
}

#[test]
fn directly_phased_attachments_stop_granting_names_and_host_bonuses() {
    use ironsmith::object::AttachmentTarget;
    for phased in [false, true] {
        let mut g = game();
        let bear = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        let giant = put(&mut g, "hill-giant", B, Zone::Battlefield);
        let kit = put(&mut g, "spy-kit", B, Zone::Battlefield);
        assert!(g.attach_object_to_target(kit, AttachmentTarget::Object(giant)));
        if phased {
            g.phase_out(kit);
        }
        resolve_spell(
            &mut g,
            "maelstrom-pulse",
            vec![Target::Object(bear)],
            &mut Menu::default(),
        );
        assert_eq!(g.battlefield.contains(&giant), phased);
        for (host, attachment) in [
            ("thran-golem", "pacifism"),
            ("armory-veteran", "paradise-mantle"),
        ] {
            let id = put(&mut g, host, A, Zone::Battlefield);
            let attached = put(&mut g, attachment, A, Zone::Battlefield);
            assert!(g.attach_object_to_target(attached, AttachmentTarget::Object(id)));
            if phased {
                g.phase_out(attached);
            }
            if host == "thran-golem" {
                assert_eq!(g.current_power(id), Some(if phased { 3 } else { 5 }));
            } else {
                assert_eq!(
                    g.current_has_static_ability_id(
                        id,
                        ironsmith::static_abilities::StaticAbilityId::Menace
                    ),
                    !phased
                );
            }
        }
    }
}

#[test]
fn evoke_exiles_a_card_with_the_color_granted_by_painters_servant() {
    let mut g = game();
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Endurance", "Mana cost: {1}{G}{G}\nType: Creature — Elemental Incarnation\nPower/Toughness: 3/4\nEvoke—Exile a green card from your hand.", false,
    ).unwrap();
    let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
    let wastes = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Wastes",
        "Type: Basic Land\n{T}: Add {C}.",
        false,
    )
    .unwrap();
    let fuel = g.create_object_from_definition(&wastes, A, Zone::Hand);
    assert!(!g.current_colors(fuel).unwrap().contains(Color::Green));
    let painter = put(&mut g, "painter-s-servant", B, Zone::Battlefield);
    g.set_chosen_color(painter, Color::Green);
    assert!(g.current_colors(fuel).unwrap().contains(Color::Green));
    let stable = g.object(fuel).unwrap().stable_id;
    let method = ironsmith::compute_legal_actions(&g, A)
        .unwrap()
        .into_iter()
        .find_map(|action| match action {
            LegalAction::CastSpell {
                spell_id,
                casting_method: method @ CastingMethod::Alternative(_),
                ..
            } if spell_id == spell => Some(method),
            _ => None,
        })
        .expect("evoke offered");
    assert_eq!(spend(&mut g, spell, method), 0);
    assert_eq!(
        g.object(g.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone,
        Zone::Exile
    );
}

#[test]
fn graveyard_aggregate_costs_use_both_halves_of_a_split_card() {
    use ironsmith_core::effect::ChoiceAggregateMetric::{ManaSymbolsOfColor, ManaValue};
    let mut g = game();
    let (front, _) = split(&mut g);
    let id = g.create_object_from_definition(&front, A, Zone::Graveyard);
    for (metric, expected) in [
        (ManaValue, 4),
        (ManaSymbolsOfColor(Color::Blue), 1),
        (ManaSymbolsOfColor(Color::Red), 1),
    ] {
        assert_eq!(
            ironsmith::targeting::aggregate_object_value(&g, id, metric),
            expected
        );
    }
}

#[test]
fn trinisphere_checks_the_payers_range_when_mana_is_deducted() {
    let sphere = ironsmith_compiler_runtime::compile_to_runtime_definition("Trinisphere",
        "Mana cost: {3}\nType: Artifact\nAs long as this artifact is untapped, each spell that would cost less than three mana to cast costs three mana to cast.", false).unwrap();
    for (seat, expected) in [(B, 3), (PlayerId(3), 2)] {
        let mut g = GameState::new(["A", "B", "C", "D", "E"].map(String::from).to_vec(), 20);
        g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
            .unwrap();
        g.turn.active_player = A;
        g.turn.priority_player = Some(A);
        g.turn.phase = Phase::FirstMain;
        g.turn.step = None;
        g.create_object_from_definition(&sphere, seat, Zone::Battlefield);
        let bears = put(&mut g, "grizzly-bears", A, Zone::Hand);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 3);
        assert_eq!(spend(&mut g, bears, CastingMethod::Normal), expected);
    }
}
