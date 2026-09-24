//! Sphere of Law: "If a red source would deal damage to you, prevent 2 of
//! that damage."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Sphere of Law",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

/// Bob's source of `color` deals `amount` damage to `victim`; returns
/// Alice's life afterwards (Alice controls Sphere of Law).
fn hit(color: ManaSymbol, amount: i32, victim: PlayerId) -> (i32, i32) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Source")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![color]]))
        .build();
    let source = game.create_object_from_definition(&source_def, bob, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::deal_damage(amount, ChooseSpec::Player(PlayerFilter::Specific(victim))),
        &mut ctx,
    )
    .unwrap();
    (game.player(alice).unwrap().life, game.player(bob).unwrap().life)
}

#[test]
fn red_damage_to_you_is_reduced_by_two() {
    assert_eq!(hit(ManaSymbol::Red, 3, PlayerId::from_index(0)).0, 19);
    assert_eq!(hit(ManaSymbol::Red, 2, PlayerId::from_index(0)).0, 20);
}

#[test]
fn other_colors_and_other_players_are_unaffected() {
    assert_eq!(hit(ManaSymbol::Green, 3, PlayerId::from_index(0)).0, 17);
    assert_eq!(hit(ManaSymbol::Red, 3, PlayerId::from_index(1)).1, 17);
}
