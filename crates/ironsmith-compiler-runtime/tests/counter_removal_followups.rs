//! cf8/p07: follow-ups of a player's counter removal. "Target player loses
//! all poison counters. Leeches deals that much damage to that player." —
//! "that much" is the number of counters removed and "that player" the
//! target (CR 608.2c); "Sacrifice this artifact." after a target player's
//! instruction is still performed by the ability's controller (CR 608.2c,
//! 701.21a).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, ResolvedTarget, SacrificeTargetEffect, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::{GameState, PlayerId, Zone};

const FIXTURE: &str = include_str!("../../../fixtures/counter_removal_followups.json.fixture");
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

#[test]
fn leeches_deals_damage_equal_to_the_poison_counters_removed() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Leeches");
    assert_eq!(row["oracle_id"], "0a265cb5-47c7-4e06-9030-602e17bedae5");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", support::spell_effects(&definition));
        assert!(debug.contains("Poison"), "{debug}");
        assert!(debug.contains("EffectValue"), "the damage reads the removal: {debug}");

        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        game.player_mut(B).unwrap().poison_counters = 3;
        let spell = game.create_object_from_definition(&definition, A, Zone::Exile);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx =
            EffectContext::new(spell, A, &mut dm).with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(B).unwrap().poison_counters, 0);
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn survivors_med_kit_radaway_sacrifices_the_kit_for_its_controller() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Survivor's Med Kit");
    assert_eq!(row["oracle_id"], "fa08400f-b7ca-4c5c-b7ca-a6583b783878");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        assert_eq!(activated.len(), 1);
        let effects = support::activated_effects(activated[0]);
        let debug = format!("{effects:?}");
        assert!(debug.contains("Rad"), "{debug}");
        let sacrifices = support::find::<SacrificeTargetEffect>(&effects);
        assert_eq!(sacrifices.len(), 1, "{debug}");
        assert!(format!("{:?}", sacrifices[0]).contains("Source"), "{debug}");
    }
}
