//! cf8 p05: "Only your opponents may activate this ability [and only as a
//! sorcery]." (CR 602.1, 602.5d): a typed activator-relative timing.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;
use ironsmith::ability::{AbilityKind, ActivationTiming};

#[test]
fn detention_vortex_is_activated_only_by_opponents_at_sorcery_speed() {
    for definition in support::definitions(&support::row("opponent_only_activation", "Detention Vortex")) {
        let activated = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .unwrap();
        assert_opponent_timing(activated, true);
        assert!(activated.allows_any_player_to_activate());
        // Original wording remains available for display; executable
        // restrictions are the typed fields checked above.
        assert!(activated.activation_restrictions.is_empty());
    }
}

#[test]
fn opponent_only_timing_sentences_are_typed() {
    for (text, timing) in [
        ("{1}: Draw a card. Only your opponents may activate this ability.", ActivationTiming::AnyTimeByOpponents),
        (
            "{1}: Draw a card. Only your opponents may activate this ability and only as a sorcery.",
            ActivationTiming::SorcerySpeedByOpponents,
        ),
    ] {
        let source = format!("Mana cost: {{1}}\nType: Artifact\n{text}");
        let definition =
            ironsmith_compiler_runtime::compile_to_runtime_definition("Opponent activation probe", &source, false)
                .unwrap();
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }).unwrap();
        assert_opponent_timing(activated, timing == ActivationTiming::SorcerySpeedByOpponents);
    }
}

fn assert_opponent_timing(activated: &ironsmith::ability::ActivatedAbility, sorcery_only: bool) {
    // Coordinated restrictions may retain the activator in `timing` and the
    // independent sorcery window as a typed condition. Both must survive.
    if sorcery_only && activated.timing == ActivationTiming::SorcerySpeedByOpponents {
        return;
    }
    assert_eq!(activated.timing, ActivationTiming::AnyTimeByOpponents);
    let expected = sorcery_only.then_some(ironsmith::ConditionExpr::ActivationTiming(ActivationTiming::SorcerySpeed));
    assert_eq!(activated.activation_condition, expected);
}

#[test]
fn soul_ransom_captures_auras_controller_before_sacrificing_then_drawing() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{CardType, GameState, Zone};
    let source = "Mana cost: {2}{U}{B}\nType: Enchantment — Aura\nEnchant creature\nYou control enchanted creature.\nDiscard two cards: This Aura's controller sacrifices it, then draws two cards. Only your opponents may activate this ability.";
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Soul Ransom", source, false,
    ).expect("complete Soul Ransom body");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).unwrap();
    assert_opponent_timing(activated, false);
    assert!(activated.choices.is_empty(), "source reference is not a target");
    // The ordinary discard cost is paid by the activating opponent, before
    // the separately scoped controller instructions resolve.
    let ironsmith_core::TotalCostKind::All(costs) = activated.mana_cost.kind() else {
        panic!("expected an ordinary discard cost");
    };
    assert!(costs.iter().any(|cost| cost.discard_details() == Some((2, None))));

    let mut game = GameState::new(vec!["Aura controller".into(), "Activator".into()], 20);
    let owner = game.players[0].id;
    let activator = game.players[1].id;
    let aura = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
    let card = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(), "Library card",
    ).card_types(vec![CardType::Sorcery]).build();
    for _ in 0..3 {
        game.create_object_from_definition(&card, owner, Zone::Library);
        game.create_object_from_definition(&card, activator, Zone::Library);
    }
    // The ability belongs to the opponent on the stack, while its source
    // remains controlled by the other player.
    let mut context = EffectContext::new_default(aura, activator);
    for effect in activated.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut context).expect("resolve controller instructions");
    }
    assert_eq!(game.players[0].graveyard.len(), 1, "Aura was sacrificed");
    assert_eq!(game.players[0].hand.len(), 2, "Aura controller draws");
    assert_eq!(game.players[1].hand.len(), 0, "activator does not draw");
}
