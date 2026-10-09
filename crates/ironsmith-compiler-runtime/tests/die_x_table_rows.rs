//! Die-result rows that only fix X ("1—9 | X is one."), source-authored and
//! UNRUN. The selected row runs the sentences between the roll and the table
//! with its X (CR 706.2). Wand of Wonder's own body is still blocked on its
//! traversal/cast grammar, so its full-card test is ignored until then.
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{BindXValueEffect, GainLifeEffect};
use ironsmith::target::PlayerFilter;
use ironsmith::Zone;

#[path = "cf8_p08/support.rs"]
mod support;
#[path = "cf8_p08/play.rs"]
mod play;

const WAND_OF_WONDER: &str = "Mana cost: {3}{R}\nType: Artifact\n{4}, {T}: Roll a d20. Each opponent exiles cards from the top of their library until they exile an instant or sorcery card, then shuffles the rest into their library. You may cast up to X instant and/or sorcery spells from among cards exiled this way without paying their mana costs.\n1—9 | X is one.\n10—19 | X is two.\n20 | X is three.";

#[test]
fn a_bound_x_governs_only_its_program() {
    let mut game = play::game();
    let source = game.create_object_from_definition(
        &play::vanilla("Source", "{R}", "Goblin", 1, 1),
        play::A,
        Zone::Battlefield,
    );
    let gain_x = || Effect::new(GainLifeEffect::with_filter(Value::X, PlayerFilter::You));
    play::apply(
        &mut game,
        source,
        Effect::new(BindXValueEffect::new(Value::Fixed(3), vec![gain_x()])),
    );
    assert_eq!(play::life(&game, play::A), 23);
}

#[test]
#[ignore = "Wand of Wonder's traversal and counted cast are not yet supported"]
fn wand_of_wonder_rows_bind_x_for_the_cast() {
    for definition in support::definitions("Wand of Wonder", WAND_OF_WONDER) {
        let binds = support::find_all::<BindXValueEffect>(&definition);
        let values = binds.iter().map(|bind| bind.value.clone()).collect::<Vec<_>>();
        assert_eq!(values, vec![Value::Fixed(1), Value::Fixed(2), Value::Fixed(3)]);
    }
}
