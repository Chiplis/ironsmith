//! "Put N counters on a <type> you control": a bare singular object noun after
//! the counter count is a non-targeted, resolution-time choice of exactly one
//! matching permanent. Frozen complete bodies; source-authored and UNRUN.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker};
use ironsmith::effect::Value;
use ironsmith::effects::PutCountersEffect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, CounterType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[path = "cf8_p08/support.rs"]
mod support;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const SETTLE_THE_SCORE: &str = "Mana cost: {2}{B}{B}\nType: Sorcery\nExile target creature. Put two loyalty counters on a planeswalker you control.";
const LILIANAS_SCROUNGER: &str = "Mana cost: {2}{B}\nType: Creature — Human Wizard\nPower/Toughness: 3/2\nAt the beginning of each end step, if a creature died this turn, you may put a loyalty counter on a Liliana planeswalker you control.";
const COSMIUM_CONFLUENCE: &str = "Mana cost: {4}{G}\nType: Sorcery\nChoose three. You may choose the same mode more than once.\n• Search your library for a Cave card, put it onto the battlefield tapped, then shuffle.\n• Put three +1/+1 counters on a Cave you control. It becomes a 0/0 Elemental creature with haste. It's still a land.\n• Destroy target enchantment.";
const WICK: &str = "Mana cost: {3}{B}\nType: Legendary Creature — Rat Warlock\nPower/Toughness: 2/4\nWhenever Wick or another Rat you control enters, create a 1/1 black Snail creature token if you don't control a Snail. Otherwise, put a +1/+1 counter on a Snail you control.\n{U}{B}{R}, Sacrifice a Snail: Wick deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the sacrificed creature's power.";
const ASTARIONS_THIRST: &str = "Mana cost: {3}{B}\nType: Instant\nExile target creature. Put X +1/+1 counters on a commander creature you control, where X is the power of the creature exiled this way.";

/// The one counter placement whose recipient is a non-targeted single choice
/// among permanents you control.
fn chosen_recipient(definition: &CardDefinition, counter: CounterType) -> (PutCountersEffect, ironsmith::target::ObjectFilter) {
    let puts: Vec<PutCountersEffect> = support::find_all::<PutCountersEffect>(definition)
        .into_iter()
        .filter(|put| put.counter_type == counter)
        .collect();
    assert_eq!(puts.len(), 1, "{}: {puts:?}", definition.card.name);
    let put = puts[0].clone();
    assert!(!put.target.is_target(), "a bare noun never declares a target");
    let ChooseSpec::WithCount(inner, count) = &put.target else {
        panic!("{}: exactly one chosen recipient, got {:?}", definition.card.name, put.target);
    };
    assert_eq!((count.min, count.max), (1, Some(1)));
    let ChooseSpec::Object(filter) = inner.base() else {
        panic!("{}: object recipient, got {inner:?}", definition.card.name);
    };
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    let filter = filter.clone();
    (put, filter)
}

#[test]
fn every_bare_type_noun_recipient_is_one_chosen_permanent_you_control() {
    for definition in support::definitions("Settle the Score", SETTLE_THE_SCORE) {
        let (put, filter) = chosen_recipient(&definition, CounterType::Loyalty);
        assert_eq!(put.amount.unhinted(), &Value::Fixed(2));
        assert_eq!(filter.card_types, vec![CardType::Planeswalker]);
    }
    for definition in support::definitions("Liliana's Scrounger", LILIANAS_SCROUNGER) {
        let (put, filter) = chosen_recipient(&definition, CounterType::Loyalty);
        assert_eq!(put.amount.unhinted(), &Value::Fixed(1));
        assert_eq!(filter.card_types, vec![CardType::Planeswalker]);
        assert_eq!(filter.subtypes, vec![Subtype::Liliana]);
        assert!(support::all_effects(&definition)
            .iter()
            .any(|effect| effect.downcast_ref::<ironsmith::effects::MayEffect>().is_some()));
    }
    for definition in support::definitions("Cosmium Confluence", COSMIUM_CONFLUENCE) {
        let (put, filter) = chosen_recipient(&definition, CounterType::PlusOnePlusOne);
        assert_eq!(put.amount.unhinted(), &Value::Fixed(3));
        assert_eq!(filter.subtypes, vec![Subtype::Cave]);
    }
    for definition in support::definitions("Wick, the Whorled Mind", WICK) {
        let (put, filter) = chosen_recipient(&definition, CounterType::PlusOnePlusOne);
        assert_eq!(put.amount.unhinted(), &Value::Fixed(1));
        assert_eq!(filter.subtypes, vec![Subtype::Snail]);
    }
    for definition in support::definitions("Astarion's Thirst", ASTARIONS_THIRST) {
        let (put, filter) = chosen_recipient(&definition, CounterType::PlusOnePlusOne);
        assert_ne!(put.amount, Value::Fixed(1), "X is the exiled creature's power");
        assert!(filter.is_commander);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }
}

#[test]
fn malformed_bare_nouns_still_fail_closed() {
    for text in [
        "Put two loyalty counters on a blorp you control.",
        "Put two loyalty counters on a you control.",
    ] {
        let body = format!("Mana cost: {{2}}{{B}}\nType: Sorcery\n{text}");
        assert!(compile_to_runtime_definition("Malformed recipient", &body, false).is_err(), "{text}");
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 10);
    game
}

fn permanent(game: &mut GameState, owner: PlayerId, body: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Scenario permanent", body, false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn cast(game: &mut GameState, definition: &CardDefinition) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        }),
        &mut dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
}

#[test]
fn settle_the_score_exiles_the_target_and_loads_the_planeswalker_you_control() {
    for definition in support::definitions("Settle the Score", SETTLE_THE_SCORE) {
        let mut game = game();
        let victim = permanent(&mut game, B, "Type: Creature — Bear\nPower/Toughness: 2/2");
        let walker = permanent(&mut game, A, "Type: Planeswalker — Jace\nLoyalty: 3\n+1: You gain 1 life.");
        let foreign_walker = permanent(&mut game, B, "Type: Planeswalker — Jace\nLoyalty: 3\n+1: You gain 1 life.");
        let before = game.counter_count(walker, CounterType::Loyalty);
        let foreign_before = game.counter_count(foreign_walker, CounterType::Loyalty);
        cast(&mut game, &definition);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.object(victim).is_none_or(|object| object.zone != Zone::Battlefield));
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), before + 2);
        assert_eq!(game.counter_count(foreign_walker, CounterType::Loyalty), foreign_before);
    }
}
