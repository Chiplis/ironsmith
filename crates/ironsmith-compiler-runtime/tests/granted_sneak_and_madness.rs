//! Granted sneak (CR 702.190a) as an alternative casting method, with a
//! separate permission to use sneak abilities from the graveyard (Ninja
//! Teen), and granted madness (CR 702.35a) as a derived madness casting
//! method used by the existing madness discard replacement and its trigger
//! (Falkenrath Gorger). Source-authored, deliberately unrun.
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, EffectExecutor, MayCastForMadnessCostEffect};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const NINJA_TEEN: &str = "Mana cost: {2}{B}\nType: Enchantment — Class\n(Gain the next level as a sorcery to add its ability.)\nWhenever a creature you control leaves the battlefield, each opponent loses 1 life.\n{1}{B}: Level 2\nCreatures you control get +1/+0 and have menace.\n{B}: Level 3\nCreature cards in your graveyard have sneak {3}{B}.\nYou may cast creature spells from your graveyard using their sneak abilities.";
const GORGER: &str = "Mana cost: {R}\nType: Creature — Vampire Berserker\nPower/Toughness: 2/1\nEach Vampire creature card you own that isn't on the battlefield has madness. The madness cost is equal to its mana cost. (If you discard a card with madness, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)";

#[test]
fn ninja_teen_grants_graveyard_sneak_and_permits_casting_with_it() {
    for definition in compile::compile_both("Ninja Teen", NINJA_TEEN) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Sneak"), "{debug}");
        assert!(debug.contains("AlternativeCastFromZoneForFilter"), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
    }
}

fn declare_blockers_game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    game
}

#[test]
fn ninja_teen_level_three_lets_graveyard_creatures_be_cast_with_granted_sneak() {
    for teen in compile::compile_both("Ninja Teen", NINJA_TEEN) {
        for level in [1, 3] {
            let mut game = declare_blockers_game();
            let class = game.create_object_from_definition(&teen, A, Zone::Battlefield);
            assert!(game.set_class_level(class, level));
            let attacker_def = compile_to_runtime_definition(
                "Unblocked attacker",
                "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1",
                false,
            )
            .unwrap();
            let attacker = game.create_object_from_definition(&attacker_def, A, Zone::Battlefield);
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(B),
            });
            game.combat = Some(combat);
            let card_def = compile_to_runtime_definition(
                "Graveyard ninja",
                "Mana cost: {5}{B}{B}\nType: Creature — Turtle Ninja\nPower/Toughness: 5/5",
                false,
            )
            .unwrap();
            let card = game.create_object_from_definition(&card_def, A, Zone::Graveyard);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 1);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
            game.refresh_continuous_state();
            let castable = compute_legal_actions(&game, A).unwrap().into_iter().any(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, .. }
                        if spell_id == card
                )
            });
            // Level 3 grants sneak {3}{B} and the graveyard permission
            // together; at level 1 the card has neither (CR 716.2).
            assert_eq!(castable, level == 3);
        }
    }
}

#[test]
fn falkenrath_gorger_grants_madness_from_the_cards_mana_cost() {
    for definition in compile::compile_both("Falkenrath Gorger", GORGER) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("MadnessFromCardManaCost"), "{debug}");
        assert!(debug.contains("Hand") && debug.contains("Exile"), "{debug}");
        assert!(debug.contains("Vampire"), "{debug}");
    }
}

#[test]
fn falkenrath_gorger_discarded_vampires_use_the_madness_replacement_and_trigger() {
    for gorger in compile::compile_both("Falkenrath Gorger", GORGER) {
        for vampire in [true, false] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.phase = Phase::FirstMain;
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.create_object_from_definition(&gorger, A, Zone::Battlefield);
            let text = if vampire {
                "Mana cost: {1}{B}\nType: Creature — Vampire\nPower/Toughness: 2/2"
            } else {
                "Mana cost: {1}{B}\nType: Creature — Zombie\nPower/Toughness: 2/2"
            };
            let card_def = compile_to_runtime_definition("Discarded card", text, false).unwrap();
            let card = game.create_object_from_definition(&card_def, A, Zone::Hand);
            game.refresh_continuous_state();
            let mut dm = SelectFirstDecisionMaker;
            let out = ironsmith::events::processing::execute_discard(
                &mut game,
                card,
                A,
                ironsmith::events::cause::EventCause::from_effect(card, A),
                false,
                Default::default(),
                &mut dm,
            )
            .unwrap()
            .unwrap();
            assert_eq!(out.final_zone, if vampire { Zone::Exile } else { Zone::Graveyard });
            if !vampire {
                continue;
            }
            let exiled = out.new_id.unwrap();
            assert!(game.is_madness_exiled(exiled));
            assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
            // The madness trigger casts it for its mana cost {1}{B}.
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 1);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
            MayCastForMadnessCostEffect::new()
                .execute(&mut game, &mut EffectContext::new_default(exiled, A))
                .unwrap();
            assert_eq!(game.stack.len(), 1, "cast from exile for its madness cost");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn granted_madness_is_not_an_ordinary_cast_from_exile() {
    for gorger in compile::compile_both("Falkenrath Gorger", GORGER) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.create_object_from_definition(&gorger, A, Zone::Battlefield);
        let card_def = compile_to_runtime_definition(
            "Exiled vampire",
            "Mana cost: {1}{B}\nType: Creature — Vampire\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let card = game.create_object_from_definition(&card_def, A, Zone::Exile);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 2);
        game.refresh_continuous_state();
        assert!(!compute_legal_actions(&game, A).unwrap().into_iter().any(|action| {
            matches!(action, LegalAction::CastSpell { spell_id, .. } if spell_id == card)
        }));
    }
}
