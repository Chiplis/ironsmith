use super::*;
use ironsmith::game_state::Phase;
use ironsmith_registry_test::compile_to_runtime_definition;

fn card(name: &str, text: &str) -> ironsmith::CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

/// Port of Karfell's {3}{U}{B}{B} ability beside Lotus Field and a second
/// black source: payable, and the existence check must not need thousands of
/// search units to say so.
#[test]
fn port_of_karfell_payment_is_found_quickly_beside_lotus_field() {
    let _guard = crate::test_id_counter_guard();
    for extra in ["none", "deeps"] {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        let port = card("Port of Karfell", "Type: Land\nThis land enters tapped.\n{T}: Add {U}.\n{3}{U}{B}{B}, {T}, Sacrifice this land: Mill four cards, then return a creature card from your graveyard to the battlefield tapped.");
        let lotus = card("Lotus Field", "Type: Land\nHexproof\nThis land enters tapped.\nWhen this land enters, sacrifice two lands.\n{T}: Add three mana of any one color.");
        let mortuary = card("Underground Mortuary", "Type: Land — Swamp Forest\n({T}: Add {B} or {G}.)\nThis land enters tapped.\nWhen this land enters, surveil 1.");
        let island = card("Island", "Type: Basic Land — Island\n({T}: Add {U}.)");
        let deeps = card("Echoing Deeps", "Type: Land — Cave\nYou may have this land enter tapped as a copy of any land card in a graveyard, except it's a Cave in addition to its other types.\n{T}: Add {C}.");
        let source = wasm.game.create_object_from_definition(&port, alice, Zone::Battlefield);
        wasm.game.create_object_from_definition(&lotus, alice, Zone::Battlefield);
        wasm.game.create_object_from_definition(&mortuary, alice, Zone::Battlefield);
        for _ in 0..3 {
            wasm.game.create_object_from_definition(&island, alice, Zone::Battlefield);
        }
        if extra == "deeps" {
            wasm.game.create_object_from_definition(&deeps, alice, Zone::Battlefield);
        }
        wasm.game.refresh_continuous_state().unwrap();
        let cost = ironsmith::mana::ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]);
        let mut request = ironsmith::mana_payment::ManaPaymentRequest::new(
            alice, source, ironsmith::costs::PaymentReason::ActivateAbility, cost,
        )
        .with_spend_policy(wasm.game.mana_spend_policy(alice, Some(source)));
        request.reserved_tap_sources.push(source);
        let mut analysis = ironsmith::mana_payment::ManaPaymentAnalysis::check(&wasm.game, request);
        let mut units = 0;
        let mut outcome = None;
        for _ in 0..200 {
            outcome = analysis.step(64);
            units += analysis.last_slice_units();
            if outcome.is_some() {
                break;
            }
        }
        assert!(matches!(outcome, Some(Ok(_))), "{extra}: no plan after {units} units");
        assert!(units <= 64, "{extra}: {units} units");
    }
}
