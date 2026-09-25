//! _____ Goblin (queued as "________ Goblin"): "When this creature enters,
//! you may put a name sticker on it. Add {R} for each unique vowel on that
//! sticker."
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::mana::ManaSymbol;
use ironsmith::{AbilityKind, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "_____ Goblin",
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

/// Resolves the enters trigger with one name sticker named `sticker`
/// available (or none); returns the red mana added.
fn enters_with(sticker: Option<&str>) -> u32 {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    let definition = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let goblin = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    if let Some(name) = sticker {
        game.add_accessible_name_sticker(alice, name);
    }
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("enters trigger");
    };
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(goblin, alice, &mut dm);
    for effect in triggered.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.player(alice).unwrap().mana_pool.amount(ManaSymbol::Red)
}

#[test]
fn adds_red_for_each_unique_vowel_on_the_sticker() {
    // "Ancestral Mob": a, e, o = 3 unique vowels.
    assert_eq!(enters_with(Some("Ancestral Mob")), 3);
    // "Queen": u, e = 2 (repeated vowels count once).
    assert_eq!(enters_with(Some("Queen")), 2);
}

#[test]
fn no_sticker_no_mana() {
    assert_eq!(enters_with(None), 0);
}
