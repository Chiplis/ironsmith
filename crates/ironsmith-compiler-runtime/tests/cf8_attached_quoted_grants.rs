//! Complete frozen cards: attached stat bonuses compose with quoted grants.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn complete_attached_grants_compile_loss_free_on_both_routes() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/cf8_attached_quoted_grants.json.fixture"
    )).unwrap();
    for row in rows {
        support::definitions_for_text(row["name"].as_str().unwrap(), row["text"].as_str().unwrap());
    }
}

#[test]
fn quoted_trigger_and_keyword_apply_to_the_enchanted_host_together() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Zone};
    let player = PlayerId(0);
    for aura in support::definitions_for_text("Staggering Insight", "Mana cost: {W}{U}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1 and has lifelink and \"Whenever this creature deals combat damage to a player, draw a card.\"") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = ironsmith_compiler_runtime::compile_to_runtime_definition("Bear", "Mana cost: {2}\nType: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
        let host = game.create_object_from_definition(&bear, player, Zone::Battlefield);
        let other = game.create_object_from_definition(&bear, player, Zone::Battlefield);
        let aura = game.create_object_from_definition(&aura, player, Zone::Battlefield);
        let check = |game: &GameState, id, attached| {
            assert_eq!(game.current_power(id), Some(if attached { 3 } else { 2 }));
            assert_eq!(game.calculated_characteristics(id).unwrap().static_abilities.iter().any(|a| a.id() == StaticAbilityId::Lifelink), attached);
            assert_eq!(game.current_abilities(id).unwrap().iter().any(|a| matches!(&a.kind, AbilityKind::Triggered(_))), attached);
        };
        check(&game, host, false);
        assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
        check(&game, host, true);
        check(&game, other, false);
        game.object_mut(aura).unwrap().attached_to = None;
        game.refresh_continuous_state().unwrap();
        check(&game, host, false);
    }
}

#[test]
fn filtered_entry_reader_does_not_steal_sibling_quoted_grants() {
    for definition in support::definitions_for_text("Master Chef", "Mana cost: {2}{G}\nType: Legendary Enchantment — Background\nCommander creatures you own have \"This creature enters with an additional +1/+1 counter on it\" and \"Other creatures you control enter with an additional +1/+1 counter on them.\"") {
        let text = support::rendered(&definition);
        assert!(text.to_lowercase().contains("commander creatures you own have"), "{text}");
        assert!(text.to_lowercase().contains("other creatures you control"), "second grant lost: {text}");
    }
}

#[test]
fn source_linked_static_permission_stays_separate_from_activation() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/cf8_attached_quoted_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Rona, Disciple of Gix").unwrap();
    for definition in support::definitions_for_text(row["name"].as_str().unwrap(), row["text"].as_str().unwrap()) {
        let text = support::rendered(&definition);
        assert!(text.to_lowercase().contains("you may cast spells from among cards exiled with"), "{text}");
        assert!(!text.to_lowercase().contains("cast that card"), "permission must not become an activation followup: {text}");
    }
}

#[test]
fn stat_count_preserves_both_attached_types_and_the_attachment_relation() {
    use ironsmith::{GameState, PlayerId, Zone};
    let player = PlayerId(0);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/cf8_attached_quoted_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Thran Power Suit").unwrap();
    for suit in support::definitions_for_text(row["name"].as_str().unwrap(), row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = ironsmith_compiler_runtime::compile_to_runtime_definition("Bear", "Mana cost: {2}\nType: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
        let aura = ironsmith_compiler_runtime::compile_to_runtime_definition("Test Aura", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature", false).unwrap();
        let host = game.create_object_from_definition(&bear, player, Zone::Battlefield);
        let suit = game.create_object_from_definition(&suit, player, Zone::Battlefield);
        let aura = game.create_object_from_definition(&aura, player, Zone::Battlefield);
        let unrelated = ironsmith_compiler_runtime::compile_to_runtime_definition("Other Equipment", "Mana cost: {1}\nType: Artifact — Equipment\nEquip {1}", false).unwrap();
        game.create_object_from_definition(&unrelated, player, Zone::Battlefield);
        assert_eq!(game.current_power(host), Some(2));
        assert!(game.attach_object_to_target(suit, ironsmith::object::AttachmentTarget::Object(host)));
        assert_eq!(game.current_power(host), Some(3), "the Equipment itself contributes one");
        assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
        assert_eq!(game.current_power(host), Some(4), "Aura and Equipment both count; unattached objects do not");
    }
}

#[test]
fn shared_x_definition_scales_only_the_declared_skeleton_subject() {
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Zone};
    let alice = PlayerId(0);
    let bob = PlayerId(1);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/cf8_attached_quoted_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Skeletal Swarming").unwrap();
    for enchantment in support::definitions_for_text(row["name"].as_str().unwrap(), row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&enchantment, alice, Zone::Battlefield);
        let skeleton = ironsmith_compiler_runtime::compile_to_runtime_definition("Skeleton", "Mana cost: {B}\nType: Creature — Skeleton\nPower/Toughness: 1/1", false).unwrap();
        let first = game.create_object_from_definition(&skeleton, alice, Zone::Battlefield);
        assert_eq!(game.current_power(first), Some(2));
        let enemy = game.create_object_from_definition(&skeleton, bob, Zone::Battlefield);
        assert_eq!(game.current_power(first), Some(2), "opponent's Skeleton is not part of X");
        assert_eq!(game.current_power(enemy), Some(1));
        let second = game.create_object_from_definition(&skeleton, alice, Zone::Battlefield);
        assert_eq!(game.current_power(first), Some(3));
        assert_eq!(game.current_power(second), Some(3));
        assert!(game.calculated_characteristics(first).unwrap().static_abilities.iter().any(|a| a.id() == StaticAbilityId::Trample));
        assert!(!game.calculated_characteristics(enemy).unwrap().static_abilities.iter().any(|a| a.id() == StaticAbilityId::Trample));
    }
}

#[test]
fn source_linked_permission_allows_only_its_exiled_cards_while_source_remains() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::resolve_stack_entry_with;
    use ironsmith::game_state::StackEntry;
    use ironsmith::{GameState, PlayerId, Zone};
    let alice = PlayerId(0);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/cf8_attached_quoted_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Rona, Disciple of Gix").unwrap();
    for definition in support::definitions_for_text(row["name"].as_str().unwrap(), row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = ironsmith_compiler_runtime::compile_to_runtime_definition("Free Sorcery", "Mana cost: {0}\nType: Sorcery\nDraw a card.", false).unwrap();
        let library = game.create_object_from_definition(&spell, alice, Zone::Library);
        let stable = game.object(library).unwrap().stable_id;
        let unrelated = game.create_object_from_definition(&spell, alice, Zone::Exile);
        let activation = definition.abilities.iter().find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a), _ => None,
        }).unwrap();
        game.push_to_stack(StackEntry::ability(source, alice, activation.effects.clone()));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let linked = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(linked).unwrap().zone, Zone::Exile);
        assert!(game.stack.is_empty(), "exiling the card must not immediately cast it");
        let can_cast = |game: &GameState, id| compute_legal_actions(game, alice).unwrap().iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == id));
        assert!(can_cast(&game, linked));
        assert!(!can_cast(&game, unrelated));
        game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        assert!(!can_cast(&game, linked));
    }
}
