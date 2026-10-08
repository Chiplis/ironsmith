//! General attack taxes (CR 508.1g-h): planeswalker-only defenders (Onakke
//! Oathkeeper), every attack regardless of defender (War Tax), life payments
//! (Sivitri), and effect-installed, duration-scoped taxes that outlive their
//! source (CR 611.2a). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, SelectFirstDecisionMaker};
use ironsmith::effect::{Effect, Restriction, Until, Value};
use ironsmith::effects::{CantEffect, EffectContext, execute_effect};
use ironsmith::game_loop::apply_attacker_declarations;
use ironsmith::target::ObjectFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
use ironsmith_core::value_model::{AttackTaxDefenders, AttackTaxRule};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

const WAR_TAX: &str = "Mana cost: {2}{U}\nType: Enchantment\n{X}{U}: This turn, creatures can't attack unless their controller pays {X} for each attacking creature they control.";
const SIVITRI: &str = "Mana cost: {4}{B}\nType: Legendary Planeswalker — Sivitri\nLoyalty: 5\n+1: Until your next turn, creatures can't attack you or planeswalkers you control unless their controller pays 2 life for each of those creatures.\n−3: Search your library for a Dragon card, reveal it, put it into your hand, then shuffle.\n−7: Destroy all non-Dragon creatures.\nSivitri, Dragon Master can be your commander.";
const ONAKKE: &str = "Mana cost: {2}{W}\nType: Creature — Ogre Warrior\nPower/Toughness: 3/3\nCreatures can't attack planeswalkers you control unless their controller pays {1} for each creature they control that's attacking a planeswalker you control.\n{4}{W}{W}, Exile this card from your graveyard: Return target planeswalker card from your graveyard to the battlefield.";

fn cant_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<CantEffect> {
    support::find_all::<CantEffect>(definition)
}

#[test]
fn war_tax_installs_an_any_defender_tax_whose_x_is_the_activation_x() {
    for definition in support::definitions("War Tax", WAR_TAX) {
        let cants = cant_effects(&definition);
        assert_eq!(cants.len(), 1, "{cants:#?}");
        assert_eq!(cants[0].duration, Until::EndOfTurn);
        assert_eq!(
            cants[0].restriction,
            Restriction::AttackTax(AttackTaxRule {
                attackers: ObjectFilter::creature(),
                defenders: AttackTaxDefenders::Anyone,
                mana_per_attacker: Value::X,
                life_per_attacker: 0,
            })
        );
    }
}

#[test]
fn sivitri_installs_a_life_tax_on_attacks_on_its_controller_until_their_next_turn() {
    for definition in support::definitions("Sivitri, Dragon Master", SIVITRI) {
        let cants = cant_effects(&definition);
        let tax = cants
            .iter()
            .find(|cant| matches!(cant.restriction, Restriction::AttackTax(_)))
            .unwrap_or_else(|| panic!("{cants:#?}"));
        assert_eq!(tax.duration, Until::YourNextTurn);
        assert_eq!(
            tax.restriction,
            Restriction::AttackTax(AttackTaxRule {
                attackers: ObjectFilter::creature(),
                defenders: AttackTaxDefenders::ControllerOrPlaneswalkers,
                mana_per_attacker: Value::Fixed(0),
                life_per_attacker: 2,
            })
        );
    }
}

#[test]
fn onakke_taxes_only_attacks_on_its_controllers_planeswalkers() {
    for definition in support::definitions("Onakke Oathkeeper", ONAKKE) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("AttackCost"), "{debug}");
        assert!(debug.contains("planeswalkers_only: true"), "{debug}");
        assert!(debug.contains("covers_planeswalkers: true"), "{debug}");
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn creature(game: &mut GameState, controller: PlayerId) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Taxed attacker",
        "Type: Creature — Human\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    let id = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn install(game: &mut GameState, controller: PlayerId, rule: AttackTaxRule, duration: Until) {
    let source = creature(game, controller);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    execute_effect(
        game,
        &Effect::new(CantEffect::new(Restriction::attack_tax(rule), duration)),
        &mut ctx,
    )
    .unwrap();
    // CR 611.2a: the tax outlives the object that created it.
    game.move_object(source, Zone::Graveyard, ironsmith::events::cause::EventCause::effect());
}

fn attack(game: &mut GameState, attacker: ObjectId, target: AttackTarget) -> bool {
    game.turn.active_player = game.controller_of_id(attacker).unwrap();
    game.refresh_continuous_state().unwrap();
    let mut combat = CombatState::default();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[AttackerDeclaration {
            creature: attacker,
            target,
        }],
    )
    .is_ok()
        && combat.attackers.len() == 1
}

#[test]
fn an_any_defender_mana_tax_charges_every_attack_and_blocks_an_unpaid_one() {
    let rule = AttackTaxRule {
        attackers: ObjectFilter::creature(),
        defenders: AttackTaxDefenders::Anyone,
        mana_per_attacker: Value::Fixed(2),
        life_per_attacker: 0,
    };
    let mut game = game();
    install(&mut game, A, rule.clone(), Until::EndOfTurn);
    let attacker = creature(&mut game, B);
    // B attacks C, not the tax's controller: still taxed.
    assert!(!attack(&mut game, attacker, AttackTarget::Player(C)), "no mana, no attack");

    let mut game = self::game();
    install(&mut game, A, rule, Until::EndOfTurn);
    let attacker = creature(&mut game, B);
    game.player_mut(B)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 2);
    assert!(attack(&mut game, attacker, AttackTarget::Player(C)));
    assert_eq!(game.player(B).unwrap().mana_pool.total(), 0, "{{2}} was paid");
}

#[test]
fn a_life_tax_on_the_controller_charges_life_and_leaves_other_defenders_free() {
    let rule = AttackTaxRule {
        attackers: ObjectFilter::creature(),
        defenders: AttackTaxDefenders::ControllerOrPlaneswalkers,
        mana_per_attacker: Value::Fixed(0),
        life_per_attacker: 2,
    };
    let mut game = game();
    install(&mut game, A, rule.clone(), Until::YourNextTurn);
    let attacker = creature(&mut game, B);
    assert!(attack(&mut game, attacker, AttackTarget::Player(A)));
    assert_eq!(game.player(B).unwrap().life, 18, "2 life paid to attack A");

    let mut game = self::game();
    install(&mut game, A, rule, Until::YourNextTurn);
    let attacker = creature(&mut game, B);
    assert!(attack(&mut game, attacker, AttackTarget::Player(C)));
    assert_eq!(game.player(B).unwrap().life, 20, "attacking C is free");
}
