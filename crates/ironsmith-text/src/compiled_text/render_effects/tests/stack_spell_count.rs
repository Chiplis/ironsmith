//! "the number of spells (currently) on the stack": spells and spell copies
//! count, abilities don't (CR 113.1c), and a resolving instant or sorcery is
//! still on the stack until its last step (CR 608.2n), so it counts itself.
use super::*;

const COLLIDER: &str = "Radian Collider deals X damage to any target, where X is 1 plus the \
                        number of spells currently on the stack.";

fn instant(name: &str, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .parse_text(text)
        .unwrap()
}

/// Alice resolves `text` at Bob with `other_spells` of her spells below it,
/// optionally a copy of one of them and an ability. Returns the damage dealt.
fn damage_with_stack(text: &str, other_spells: usize, copy: bool, ability: bool) -> i32 {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let filler = instant("Filler", "Draw a card.");
    let mut spells = Vec::new();
    for _ in 0..other_spells {
        let spell = game.create_object_from_definition(&filler, alice, Zone::Stack);
        game.stack.push(crate::game_state::StackEntry::new(spell, alice));
        spells.push(spell);
    }
    if copy {
        let copier = instant("Copier", "Copy target instant or sorcery spell.");
        let source = game.create_object_from_definition(&copier, alice, Zone::Graveyard);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(spells[0])]);
        for effect in copier.spell_effect.as_ref().unwrap() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    if ability {
        let gain = instant("Gain", "You gain 1 life.");
        let source = game.create_object_from_definition(&gain, alice, Zone::Graveyard);
        game.stack.push(crate::game_state::StackEntry::ability(
            source,
            alice,
            gain.spell_effect.clone().unwrap(),
        ));
    }
    let resolving = instant("Radian Collider", text);
    let spell = game.create_object_from_definition(&resolving, alice, Zone::Stack);
    game.stack.push(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(game.stack.iter().all(|entry| entry.object_id != spell));
    20 - game.player(bob).unwrap().life
}

#[test]
fn spells_on_the_stack_wording_compiles_to_a_stack_spell_count() {
    for text in [
        COLLIDER,
        "Radian Collider deals X damage to any target, where X is the number of spells on the stack.",
        "Radian Collider deals X damage to any target, where X is 1 plus the number of spells.",
    ] {
        let definition = instant("Radian Collider", text);
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("the number of spells"), "{text} -> {rendered}");
    }
}

#[test]
fn resolving_spell_counts_itself_among_spells_on_the_stack() {
    assert_eq!(damage_with_stack(COLLIDER, 0, false, false), 2, "itself");
    assert_eq!(damage_with_stack(COLLIDER, 2, false, false), 4);
    assert_eq!(damage_with_stack(COLLIDER, 1, true, false), 4, "a copy is a spell");
    assert_eq!(damage_with_stack(COLLIDER, 1, false, true), 3, "an ability isn't");
    let bare = "Radian Collider deals X damage to any target, where X is the number of spells.";
    assert_eq!(damage_with_stack(bare, 0, false, false), 1);
}

#[test]
fn resolving_copy_counts_itself_and_its_original() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let collider = instant("Radian Collider", COLLIDER);
    let spell = game.create_object_from_definition(&collider, alice, Zone::Stack);
    game.stack.push(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)]),
    );
    let copier = instant("Copier", "Copy target instant or sorcery spell.");
    let source = game.create_object_from_definition(&copier, alice, Zone::Graveyard);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(spell)]);
    for effect in copier.spell_effect.as_ref().unwrap() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.stack.len(), 2);
    // The copy resolves first: it and the original are both spells.
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(20 - game.player(bob).unwrap().life, 3);
}
