//! Virtuous Role predefined token (CR 111.10) for Ellivere. Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

use ironsmith::object::AttachmentTarget;
use ironsmith::{GameState, Phase, PlayerId, Subtype, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId::from_index(0);

#[test]
fn ellivere_creates_an_attached_virtuous_role() {
    for definition in support::definitions("Ellivere of the Wild Court") {
        assert_eq!(support::triggered_count(&definition), 2);
        let effects = support::effects(&definition);
        let create = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>())
            .expect("typed Role creation");
        assert_eq!(create.token.card.name, "Virtuous Role");
        assert!(create.token.card.subtypes.contains(&Subtype::Aura));
        assert!(create.token.card.subtypes.contains(&Subtype::Role));
    }
}

/// CR 613.4c: the Role's bonus counts enchantments you control continuously.
#[test]
fn virtuous_role_bonus_tracks_enchantments_you_control() {
    let [definition, _] = support::definitions("Ellivere of the Wild Court");
    let role = support::effects(&definition)
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<ironsmith::effects::CreateTokenEffect>()
                .map(|create| create.token.clone())
        })
        .unwrap();
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    let bear = compile_to_runtime_definition(
        "Bear",
        "Type: Creature — Bear\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    let bear = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let role = game.create_object_from_definition(&role, A, Zone::Battlefield);
    assert!(game.attach_object_to_target(role, AttachmentTarget::Object(bear)));
    // The Role itself is an enchantment you control.
    assert_eq!(game.calculated_power(bear), Some(3));
    assert_eq!(game.calculated_toughness(bear), Some(3));
    let enchantment = compile_to_runtime_definition("Glimmer", "Type: Enchantment", false).unwrap();
    game.create_object_from_definition(&enchantment, A, Zone::Battlefield);
    assert_eq!(game.calculated_power(bear), Some(4));
    assert_eq!(game.calculated_toughness(bear), Some(4));
}
