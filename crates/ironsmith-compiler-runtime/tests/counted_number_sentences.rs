//! "Count the number of cards in your library. Your life total becomes that
//! number." (Invincible Hymn): the counted quantity is the antecedent of
//! "that number". Source-authored, deliberately unrun.
use ironsmith::{GameState, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const INVINCIBLE_HYMN: &str = "Mana cost: {6}{W}{W}\nType: Sorcery\nCount the number of cards in your library. Your life total becomes that number.";

#[test]
fn invincible_hymn_sets_life_to_library_size() {
    for definition in compile::compile_both("Invincible Hymn", INVINCIBLE_HYMN) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("SetLifeTotal") || debug.contains("LifeTotal"), "{debug}");
        assert!(debug.contains("Library"), "{debug}");
    }
}

#[test]
fn invincible_hymn_resolution_uses_current_library_count() {
    let alice = PlayerId::from_index(0);
    for definition in compile::compile_both("Invincible Hymn", INVINCIBLE_HYMN) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for _ in 0..37 {
            let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Filler")
                .card_types(vec![ironsmith::CardType::Land])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }
        let effects = definition.spell_effect.as_ref().unwrap().all_effects();
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in effects {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 37);
    }
}
