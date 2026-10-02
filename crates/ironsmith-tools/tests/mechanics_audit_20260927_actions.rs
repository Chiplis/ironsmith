//! A1–A9: Comprehensive Rules effective 2026-09-25.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder as B};
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::*;
use ironsmith::game_loop::drain_pending_trigger_events;
use ironsmith::object::CounterType;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::{Subtype, Supertype};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
#[derive(Default)]
struct Dm {
    choose_last: bool,
    selections: Vec<Vec<ObjectId>>,
}
impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.selections
            .push(c.candidates.iter().map(|o| o.id).collect());
        if self.choose_last {
            c.candidates
                .iter()
                .rev()
                .take(c.min.max(1))
                .map(|o| o.id)
                .collect()
        } else {
            c.candidates
                .iter()
                .take(c.min.max(1))
                .map(|o| o.id)
                .collect()
        }
    }
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20)
}
fn creature(name: &str, p: i32, t: i32) -> CardDefinition {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}
fn modify(g: &mut GameState, id: ObjectId, m: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(id), m, Until::Forever)
        .execute(g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
}
fn triggered_program(def: &CardDefinition) -> Vec<Effect> {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.iter().cloned().collect()),
            _ => None,
        })
        .expect("fixture has a triggered ability")
}

#[test]
fn a1_ring_bearer_retains_designation_and_legendary_after_type_loss() {
    let mut g = game();
    let def = B::new(CardId::new(), "Animated land")
        .card_types(vec![CardType::Land, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
    RingTemptsYouEffect::you()
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    modify(
        &mut g,
        id,
        Modification::RemoveCardTypes(vec![CardType::Creature]),
    );
    g.reconcile_ring_bearers();
    assert_eq!(g.current_ring_bearer(A), Some(id));
    assert!(
        g.current_supertypes(id)
            .unwrap()
            .contains(&Supertype::Legendary)
    );
    modify(
        &mut g,
        id,
        Modification::AddCardTypes(vec![CardType::Creature]),
    );
    assert_eq!(g.current_ring_bearer(A), Some(id));
    g.set_current_controller(id, BOB).expect("finite controller fixture must refresh successfully");
    g.set_current_controller(id, A).expect("finite controller fixture must refresh successfully");
    assert_eq!(
        g.current_ring_bearer(A),
        None,
        "control change permanently ends designation"
    );
}

#[test]
fn a2_equal_life_toughness_exchange_still_sets_base_toughness() {
    let mut g = game();
    g.player_mut(A).unwrap().life = 14;
    let id = g.create_object_from_definition(
        &creature("Tree of Redemption", 0, 13),
        A,
        Zone::Battlefield,
    );
    g.add_counters(id, CounterType::PlusOnePlusOne, 1);
    ExchangeValuesEffect::new(
        ExchangeValueOperand::LifeTotal(PlayerFilter::You),
        ExchangeValueOperand::Toughness(ChooseSpec::Source),
        Until::Forever,
    )
    .execute(&mut g, &mut ExecutionContext::new_default(id, A))
    .unwrap();
    assert_eq!(g.player(A).unwrap().life, 14);
    assert_eq!(g.calculated_toughness(id), Some(15));
}

fn encode(g: &mut GameState, bearer: ObjectId) -> ObjectId {
    let spell = B::new(CardId::new(), "Cipher card")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell = g.create_object_from_definition(&spell, A, Zone::Stack);
    let mut dm = Dm::default();
    let out = CipherEffect::new()
        .execute(g, &mut ExecutionContext::new(spell, A, &mut dm))
        .unwrap();
    assert_eq!(out.objects().unwrap()[1], bearer);
    out.objects().unwrap()[0]
}
fn encoded_trigger_count(g: &GameState, bearer: ObjectId) -> usize {
    g.current_abilities(bearer)
        .unwrap()
        .iter()
        .filter(|ability| {
            matches!(&ability.kind, AbilityKind::Triggered(triggered) if triggered.effects.iter()
            .any(|effect| effect.downcast_ref::<CastEncodedCardCopyEffect>().is_some()))
        })
        .count()
}

#[test]
fn a3_cipher_ends_on_exile_departure_and_does_not_revive_after_reexile() {
    let mut g = game();
    let bearer = g.create_object_from_definition(&creature("Bearer", 2, 2), A, Zone::Battlefield);
    let exiled = encode(&mut g, bearer);
    assert_eq!(encoded_trigger_count(&g, bearer), 1);
    let grave = g.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
    let _new_exile = g.move_object_by_effect(grave, Zone::Exile).unwrap();
    g.refresh_continuous_state();
    assert_eq!(encoded_trigger_count(&g, bearer), 0);
    let mut dm = Dm::default();
    CastEncodedCardCopyEffect::new(exiled)
        .execute(&mut g, &mut ExecutionContext::new(bearer, A, &mut dm))
        .unwrap();
    assert!(
        g.stack.is_empty(),
        "a queued old trigger cannot copy an unrelated new exile object"
    );
}

#[test]
fn a3_cipher_survives_bearer_control_type_and_phasing_changes_but_is_not_copiable() {
    let mut g = game();
    let bearer = g.create_object_from_definition(&creature("Bearer", 2, 2), A, Zone::Battlefield);
    let exiled = encode(&mut g, bearer);
    modify(
        &mut g,
        bearer,
        Modification::RemoveCardTypes(vec![CardType::Creature]),
    );
    g.set_current_controller(bearer, BOB).expect("finite controller fixture must refresh successfully");
    assert_eq!(encoded_trigger_count(&g, bearer), 1);
    g.phase_out(bearer);
    g.refresh_continuous_state();
    g.phase_in(bearer);
    g.refresh_continuous_state();
    assert_eq!(encoded_trigger_count(&g, bearer), 1);
    let copy = CreateTokenCopyEffect::new(ChooseSpec::SpecificObject(bearer), 1, PlayerFilter::You)
        .execute(&mut g, &mut ExecutionContext::new_default(bearer, BOB))
        .unwrap()
        .objects()
        .unwrap()[0];
    assert_eq!(encoded_trigger_count(&g, copy), 0);
    let old = g.move_object_by_effect(bearer, Zone::Hand).unwrap();
    let new = g.move_object_by_effect(old, Zone::Battlefield).unwrap();
    assert_eq!(encoded_trigger_count(&g, new), 0);
    assert_eq!(g.object(exiled).unwrap().zone, Zone::Exile);
}

#[test]
fn a4_next_adapt_permission_does_not_follow_blinked_creature() {
    let mut g = game();
    let id =
        g.create_object_from_definition(&creature("Adapt creature", 2, 2), A, Zone::Battlefield);
    NextAdaptIgnoresCountersEffect::new(ChooseSpec::SpecificObject(id))
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    let hand = g.move_object_by_effect(id, Zone::Hand).unwrap();
    let returned = g.move_object_by_effect(hand, Zone::Battlefield).unwrap();
    g.add_counters(returned, CounterType::PlusOnePlusOne, 1);
    AdaptEffect::new(2)
        .execute(&mut g, &mut ExecutionContext::new_default(returned, A))
        .unwrap();
    assert_eq!(g.counter_count(returned, CounterType::PlusOnePlusOne), 1);
}

#[test]
fn a5_all_existing_next_adapt_permissions_expire_on_the_same_adaptation() {
    let mut g = game();
    let id =
        g.create_object_from_definition(&creature("Adapt creature", 2, 2), A, Zone::Battlefield);
    g.add_counters(id, CounterType::PlusOnePlusOne, 1);
    for _ in 0..2 {
        NextAdaptIgnoresCountersEffect::new(ChooseSpec::SpecificObject(id))
            .execute(&mut g, &mut ExecutionContext::new_default(id, A))
            .unwrap();
    }
    for _ in 0..2 {
        AdaptEffect::new(2)
            .execute(&mut g, &mut ExecutionContext::new_default(id, A))
            .unwrap();
    }
    assert_eq!(g.counter_count(id, CounterType::PlusOnePlusOne), 3);
    NextAdaptIgnoresCountersEffect::new(ChooseSpec::SpecificObject(id))
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    g.turn.turn_number += 1;
    AdaptEffect::new(2)
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    assert_eq!(
        g.counter_count(id, CounterType::PlusOnePlusOne),
        3,
        "permission expires at turn boundary"
    );
}

#[test]
fn a6_amass_places_counters_before_adding_the_subtype() {
    let mut g = game();
    let army = B::new(CardId::new(), "Zombie Army")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Army])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let army = g.create_object_from_definition(&army, A, Zone::Battlefield);
    // Mauhur after Artificial Evolution changes Army to Elf.
    let mut filter = ObjectFilter::creature().you_control();
    filter.subtypes = vec![Subtype::Elf, Subtype::Goblin, Subtype::Orc];
    let replacement = B::new(CardId::new(), "Altered counter replacement")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::add_counters_placement_replacement(
                filter,
                Some(CounterType::PlusOnePlusOne),
                1,
                "Altered Mauhur".into(),
            ),
        ))
        .build();
    g.create_object_from_definition(&replacement, A, Zone::Battlefield);
    AmassEffect::new(Some(Subtype::Orc), 1)
        .execute(&mut g, &mut ExecutionContext::new_default(army, A))
        .unwrap();
    assert_eq!(g.counter_count(army, CounterType::PlusOnePlusOne), 1);
    assert!(g.current_has_subtype(army, Subtype::Orc));
}

fn meld_setup() -> (GameState, ObjectId, ObjectId, ObjectId) {
    let mut g = game();
    let rats = creature("Graf Rats", 2, 1);
    let scav = creature("Midnight Scavengers", 3, 3);
    let host = creature("Chittering Host", 5, 6);
    for def in [&rats, &scav, &host] {
        g.register_linked_face_definition(def);
    }
    let source = g.create_object_from_definition(&rats, A, Zone::Battlefield);
    let first = g.create_object_from_definition(&scav, A, Zone::Battlefield);
    let second = g.create_object_from_definition(&scav, A, Zone::Battlefield);
    (g, source, first, second)
}
#[test]
fn a7_meld_ignores_phased_counterpart() {
    let (mut g, source, phased, present) = meld_setup();
    g.phase_out(phased);
    let out = MeldEffect::new("Chittering Host")
        .execute(&mut g, &mut ExecutionContext::new_default(source, A))
        .unwrap();
    assert!(out.objects().is_some());
    assert!(g.object(phased).is_some());
    assert!(g.object(present).is_none());
}
#[test]
fn a7_meld_controller_chooses_between_multiple_present_counterparts() {
    let (mut g, source, first, second) = meld_setup();
    let mut dm = Dm {
        choose_last: true,
        ..Default::default()
    };
    MeldEffect::new("Chittering Host")
        .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
        .unwrap();
    assert_eq!(dm.selections, vec![vec![first, second]]);
    assert!(g.object(first).is_some());
    assert!(g.object(second).is_none());
}

#[test]
fn a8_library_manifest_and_cloak_generate_separate_entry_triggers() {
    for cloak in [false, true] {
        for origin in [Zone::Library, Zone::Hand] {
            let mut g = game();
            let witness = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Entry witness",
            )
            .card_types(vec![CardType::Enchantment])
            .parse_text("Whenever one or more creatures you control enter, you gain 1 life.")
            .unwrap();
            let witness = g.create_object_from_definition(&witness, A, Zone::Battlefield);
            let ids = ["First card", "Second card"]
                .map(|name| g.create_object_from_definition(&creature(name, 1, 1), A, origin));
            let memories = ids
                .iter()
                .map(|id| ObjectSnapshot::from_object(g.object(*id).unwrap(), &g))
                .collect();
            g.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(witness, A);
            ctx.set_tagged_objects("chosen", memories);
            let mut effect =
                ManifestObjectsEffect::new(ChooseSpec::Tagged("chosen".into()), PlayerFilter::You);
            effect.cloak = cloak;
            let out = effect.execute(&mut g, &mut ctx).unwrap();
            assert_eq!(out.objects().unwrap().len(), 2);
            for id in out.objects().unwrap() {
                assert!(g.is_face_down(*id));
                if cloak {
                    assert!(g.current_has_static_ability_id(*id, StaticAbilityId::Ward));
                }
            }
            let mut q = TriggerQueue::new();
            drain_pending_trigger_events(&mut g, &mut q);
            assert_eq!(
                q.entries.iter().filter(|t| t.source == witness).count(),
                if origin == Zone::Library { 2 } else { 1 },
                "cloak={cloak}, origin={origin:?}: library actions are sequential; hand collection is simultaneous"
            );
        }
    }
}

fn hideaway(g: &mut GameState) -> (ObjectId, ObjectId) {
    let bridge = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Hideaway land",
    )
    .card_types(vec![CardType::Land])
    .parse_text("Hideaway 4")
    .unwrap();
    let program = triggered_program(&bridge);
    let source = g.create_object_from_definition(&bridge, A, Zone::Battlefield);
    g.create_object_from_definition(&creature("Hidden card", 3, 3), A, Zone::Library);
    let mut ctx = ExecutionContext::new_default(source, A);
    for effect in program {
        execute_effect(g, &effect, &mut ctx).unwrap();
    }
    (source, g.exile[0])
}
#[test]
fn a9_parsed_hideaway_gives_current_controller_and_retains_previous_entitlements() {
    let mut g = game();
    let (source, exiled) = hideaway(&mut g);
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, A));
    assert!(!g.can_player_look_at_face_down_exiled_card(exiled, BOB));
    g.set_current_controller(source, BOB).expect("finite controller fixture must refresh successfully");
    // Bob need not look before losing control: entitlement alone persists.
    g.set_current_controller(source, A).expect("finite controller fixture must refresh successfully");
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, BOB));
    let hand = g.move_object_by_effect(source, Zone::Hand).unwrap();
    let returned = g.move_object_by_effect(hand, Zone::Battlefield).unwrap();
    g.set_current_controller(returned, C).expect("finite controller fixture must refresh successfully");
    assert!(
        !g.can_player_look_at_face_down_exiled_card(exiled, C),
        "new source incarnation has no old link"
    );
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, A));
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, BOB));
    let grave = g.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
    let new_exile = g.move_object_by_effect(grave, Zone::Exile).unwrap();
    g.set_face_down(new_exile);
    for player in [A, BOB, C] {
        assert!(!g.can_player_look_at_face_down_exiled_card(new_exile, player));
    }
}

#[test]
fn a9_face_down_exile_without_linked_permission_stays_private() {
    let mut g = game();
    let source = g.create_object_from_definition(&creature("Source", 1, 1), A, Zone::Battlefield);
    let card = g.create_object_from_definition(&creature("Card", 1, 1), A, Zone::Library);
    let mut ctx = ExecutionContext::new_default(source, A);
    LookAtTopCardsEffect::new(PlayerFilter::You, 1, "looked")
        .execute(&mut g, &mut ctx)
        .unwrap();
    ExileEffect::with_spec(ChooseSpec::SpecificObject(card))
        .with_face_down(true)
        .execute(&mut g, &mut ctx)
        .unwrap();
    let exiled = g.exile[0];
    g.set_current_controller(source, BOB).expect("finite controller fixture must refresh successfully");
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, A));
    assert!(!g.can_player_look_at_face_down_exiled_card(exiled, BOB));
}

#[test]
fn a9_hideaway_continuous_control_grants_permission_but_copying_the_source_does_not() {
    let mut g = game();
    let (source, exiled) = hideaway(&mut g);
    let copy = CreateTokenCopyEffect::new(ChooseSpec::SpecificObject(source), 1, PlayerFilter::You)
        .execute(&mut g, &mut ExecutionContext::new_default(source, C))
        .unwrap()
        .objects()
        .unwrap()[0];
    assert_eq!(g.current_controller(copy), Some(C));
    assert!(!g.can_player_look_at_face_down_exiled_card(exiled, C));
    GainControlEffect::permanent(ChooseSpec::SpecificObject(source))
        .execute(&mut g, &mut ExecutionContext::new_default(copy, BOB))
        .unwrap();
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, BOB));
    g.phase_out(source);
    assert!(
        g.can_player_look_at_face_down_exiled_card(exiled, BOB),
        "earned permission survives phasing"
    );
    g.phase_in(source);
    GainControlEffect::permanent(ChooseSpec::SpecificObject(source))
        .execute(&mut g, &mut ExecutionContext::new_default(copy, A))
        .unwrap();
    assert!(g.can_player_look_at_face_down_exiled_card(exiled, BOB));
    assert!(!g.can_player_look_at_face_down_exiled_card(exiled, C));
}
