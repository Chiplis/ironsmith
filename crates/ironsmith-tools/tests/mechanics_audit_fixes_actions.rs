//! September 2026 CR audit: action, exchange, multiplayer and combat regressions.
use ironsmith::ability::Ability;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder as B};
use ironsmith::color::ColorSet;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::*;
use ironsmith::decisions::specs::ProliferateResponse;
use ironsmith::effect::Until;
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::*;
use ironsmith::events::{DamageEvent, KeywordActionEvent, KeywordActionKind};
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);

#[derive(Default)]
struct Dm {
    options: Vec<Vec<String>>,
    offered_permanents: Vec<ObjectId>,
    choose_last_option: bool,
    decline: bool,
}
impl DecisionMaker for Dm {
    fn decide_proliferate(&mut self, _: &GameState, c: &ProliferateContext) -> ProliferateResponse {
        self.offered_permanents = c.eligible_permanents.iter().map(|(id, _)| *id).collect();
        ProliferateResponse {
            permanents: self.offered_permanents.clone(),
            players: c.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }
    fn decide_partition(&mut self, _: &GameState, c: &PartitionContext) -> Vec<ObjectId> {
        c.cards.iter().map(|(id, _)| *id).collect()
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.options
            .push(c.options.iter().map(|o| o.description.clone()).collect());
        if self.choose_last_option {
            c.options
                .iter()
                .rev()
                .find(|o| o.legal)
                .map(|o| vec![o.index])
                .unwrap_or_default()
        } else {
            c.options
                .iter()
                .filter(|o| o.legal)
                .take(c.min)
                .map(|o| o.index)
                .collect()
        }
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        c.candidates
            .iter()
            .take(c.min.max(1))
            .map(|o| o.id)
            .collect()
    }
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn teams(shared: bool) -> GameState {
    let mut g = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()],
        20,
    );
    let teams = vec![vec![A, BOB], vec![C, D]];
    if shared {
        g.enable_two_headed_giant(teams).unwrap();
    } else {
        g.set_teams(teams).unwrap();
    }
    g
}
fn creature(name: &str, power: i32) -> CardDefinition {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 8))
        .build()
}
fn execute<E: EffectExecutor>(
    g: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    dm: &mut Dm,
    effect: E,
) -> ironsmith::effect::EffectOutcome {
    effect
        .execute(g, &mut ExecutionContext::new(source, controller, dm))
        .unwrap()
}
fn keywords(outcome: &ironsmith::effect::EffectOutcome, kind: KeywordActionKind) -> usize {
    outcome
        .events
        .iter()
        .filter_map(|e| e.downcast::<KeywordActionEvent>())
        .filter(|e| e.action == kind)
        .count()
}
#[test]
fn f4_clash_excludes_teammates_in_choices_and_explicit_defender() {
    let mut g = teams(false);
    let src = g.new_object_id();
    let mut dm = Dm::default();
    let out = execute(&mut g, src, A, &mut dm, ClashEffect::against_any_opponent());
    assert_eq!(dm.options[0], vec!["Carol", "Dan"]);
    let players = out
        .events
        .iter()
        .filter_map(|e| e.downcast::<KeywordActionEvent>())
        .map(|e| e.player)
        .collect::<Vec<_>>();
    assert_eq!(players, vec![A, C]);
    let out = ClashEffect::against_defending_player()
        .execute(
            &mut g,
            &mut ExecutionContext::new(src, A, &mut dm).with_defending_player(BOB),
        )
        .unwrap();
    assert_eq!(keywords(&out, KeywordActionKind::Clash), 0);
}
#[test]
fn f6_fateseal_excludes_teammates() {
    let mut g = teams(true);
    let src = g.new_object_id();
    let mut dm = Dm::default();
    execute(&mut g, src, A, &mut dm, FatesealEffect::you(1));
    assert_eq!(dm.options[0], vec!["Carol", "Dan"]);
}
fn aura(name: &str, color: ColorSet) -> CardDefinition {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .color_indicator(color)
        .enchants(ObjectFilter::creature())
        .build()
}
#[test]
fn f7_aura_swap_protection_prevents_both_halves_of_exchange() {
    let mut g = game();
    let mut dm = Dm::default();
    let target = B::new(CardId::new(), "Protected")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .protection_from(ColorSet::RED)
        .build();
    let target = g.create_object_from_definition(&target, A, Zone::Battlefield);
    let old = g.create_object_from_definition(&aura("Blue", ColorSet::BLUE), A, Zone::Battlefield);
    let incoming = g.create_object_from_definition(&aura("Red", ColorSet::RED), A, Zone::Hand);
    g.attach_object_to_target(old, AttachmentTarget::Object(target));
    execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    assert_eq!(g.object(old).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        g.object(old).unwrap().attached_to,
        Some(AttachmentTarget::Object(target))
    );
    assert_eq!(g.object(incoming).unwrap().zone, Zone::Hand);
}
#[test]
fn f7_legal_aura_swap_exchanges_and_attaches_to_same_object() {
    let mut g = game();
    let mut dm = Dm::default();
    let target = g.create_object_from_definition(&creature("Target", 2), A, Zone::Battlefield);
    let old = g.create_object_from_definition(&aura("Old", ColorSet::BLUE), A, Zone::Battlefield);
    let mut incoming_def = aura("New", ColorSet::RED);
    incoming_def.abilities.push(Ability::static_ability(
        StaticAbility::enters_tapped_ability(),
    ));
    let incoming = g.create_object_from_definition(&incoming_def, A, Zone::Hand);
    g.attach_object_to_target(old, AttachmentTarget::Object(target));
    let out = execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    let ids = out.objects().unwrap();
    assert_eq!(g.object(ids[0]).unwrap().zone, Zone::Hand);
    assert_eq!(
        g.object(ids[1]).unwrap().attached_to,
        Some(AttachmentTarget::Object(target))
    );
    assert!(
        g.is_tapped(ids[1]),
        "Aura swap must run as-enters replacements"
    );
    assert!(g.object(old).is_none() && g.object(incoming).is_none());
}
#[test]
fn f7_entry_prohibition_leaves_both_aura_objects_unchanged() {
    let mut g = game();
    let mut dm = Dm::default();
    let target = g.create_object_from_definition(&creature("Target", 2), A, Zone::Battlefield);
    let old = g.create_object_from_definition(&aura("Old", ColorSet::BLUE), A, Zone::Battlefield);
    let incoming = g.create_object_from_definition(&aura("New", ColorSet::RED), A, Zone::Hand);
    g.attach_object_to_target(old, AttachmentTarget::Object(target));
    let blocker = B::new(CardId::new(), "No hand entry")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            ironsmith::effect::Restriction::EnterBattlefield(
                ObjectFilter::default().in_zone(Zone::Hand),
            ),
            "No hand entry".into(),
        )))
        .build();
    g.create_object_from_definition(&blocker, A, Zone::Battlefield);
    g.refresh_continuous_state();
    execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    assert_eq!(g.object(old).unwrap().zone, Zone::Battlefield);
    assert_eq!(g.object(incoming).unwrap().zone, Zone::Hand);
}

#[test]
fn f7_zone_replacements_modify_either_half_of_a_legal_exchange() {
    use ironsmith::events::zones::matchers::{
        WouldChangeZoneMatcher, WouldEnterBattlefieldMatcher,
    };
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for redirect_incoming in [false, true] {
        let mut g = game();
        let mut dm = Dm::default();
        let target = g.create_object_from_definition(&creature("Target", 2), A, Zone::Battlefield);
        let old =
            g.create_object_from_definition(&aura("Old", ColorSet::BLUE), A, Zone::Battlefield);
        let incoming = g.create_object_from_definition(&aura("New", ColorSet::RED), A, Zone::Hand);
        g.attach_object_to_target(old, AttachmentTarget::Object(target));
        let replacement = if redirect_incoming {
            ReplacementEffect::with_matcher(
                target,
                A,
                WouldEnterBattlefieldMatcher::new(ObjectFilter::default()),
                ReplacementAction::ChangeDestination(Zone::Exile),
            )
        } else {
            ReplacementEffect::with_matcher(
                target,
                A,
                WouldChangeZoneMatcher::new(
                    ObjectFilter::default(),
                    Some(Zone::Battlefield),
                    Some(Zone::Hand),
                ),
                ReplacementAction::ChangeDestination(Zone::Exile),
            )
        };
        g.effect_store.replacement_effects.add_effect(replacement);
        let out = execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
        let ids = out.objects().unwrap();
        assert_eq!(
            g.object(ids[0]).unwrap().zone,
            if redirect_incoming {
                Zone::Hand
            } else {
                Zone::Exile
            }
        );
        assert_eq!(
            g.object(ids[1]).unwrap().zone,
            if redirect_incoming {
                Zone::Exile
            } else {
                Zone::Battlefield
            }
        );
        assert_eq!(
            g.object(ids[1]).unwrap().attached_to,
            (!redirect_incoming).then_some(AttachmentTarget::Object(target))
        );
        assert!(g.object(old).is_none() && g.object(incoming).is_none());
    }
}
#[test]
fn f8_proliferate_excludes_phased_permanents_and_preserves_other_counters() {
    let mut g = game();
    let src = g.new_object_id();
    let mut dm = Dm::default();
    let phased = g.create_object_from_definition(&creature("Phased", 1), A, Zone::Battlefield);
    let present = g.create_object_from_definition(&creature("Present", 2), A, Zone::Battlefield);
    for id in [phased, present] {
        g.object_mut(id)
            .unwrap()
            .counters
            .insert(CounterType::Charge, 1);
    }
    g.phase_out(phased);
    execute(&mut g, src, A, &mut dm, ProliferateEffect::new(1));
    assert_eq!(dm.offered_permanents, vec![present]);
    assert_eq!(g.counter_count(phased, CounterType::Charge), 1);
    assert_eq!(g.counter_count(present, CounterType::Charge), 2);
}
#[test]
fn f9_two_headed_giant_proliferate_adds_poison_once_and_individual_energy_twice() {
    let mut g = teams(true);
    let src = g.new_object_id();
    let mut dm = Dm {
        choose_last_option: true,
        ..Default::default()
    };
    for p in [A, BOB] {
        let p = g.player_mut(p).unwrap();
        p.poison_counters = 1;
        p.energy_counters = 2;
    }
    let out = execute(&mut g, src, C, &mut dm, ProliferateEffect::new(2));
    let recipients = out
        .events
        .iter()
        .filter_map(|e| e.downcast::<ironsmith::events::MarkersChangedEvent>())
        .filter(|e| e.marker.as_counter() == Some(CounterType::Poison))
        .map(|e| e.player())
        .collect::<Vec<_>>();
    assert_eq!(recipients, vec![Some(BOB), Some(BOB)]);
    for p in [A, BOB] {
        assert_eq!(g.player(p).unwrap().poison_counters, 3);
        assert_eq!(g.player(p).unwrap().energy_counters, 4);
    }
    assert_eq!(dm.options, vec![vec!["Alice", "Bob"], vec!["Alice", "Bob"]]);
}
#[test]
fn f10_connive_zero_neither_draws_nor_emits_a_keyword_event() {
    let mut g = game();
    let mut dm = Dm::default();
    let id = g.create_object_from_definition(&creature("Conniver", 2), A, Zone::Battlefield);
    g.create_object_from_definition(&creature("Library card", 1), A, Zone::Library);
    let out = execute(
        &mut g,
        id,
        A,
        &mut dm,
        ConniveEffect::new_with_count(ChooseSpec::Source, 0),
    );
    assert_eq!(keywords(&out, KeywordActionKind::Connive), 0);
    assert_eq!(g.player(A).unwrap().library.len(), 1);
    assert!(g.player(A).unwrap().graveyard.is_empty());
}
#[test]
fn f11_source_connives_after_losing_creature_type() {
    let mut g = game();
    let mut dm = Dm::default();
    let id = g.create_object_from_definition(&creature("Conniver", 2), A, Zone::Battlefield);
    execute(
        &mut g,
        id,
        A,
        &mut dm,
        ApplyContinuousEffect::new(
            EffectTarget::Specific(id),
            Modification::SetCardTypes(vec![CardType::Artifact]),
            Until::Forever,
        ),
    );
    g.create_object_from_definition(&creature("Discard", 1), A, Zone::Library);
    assert!(!g.current_is_creature(id));
    let out = execute(
        &mut g,
        id,
        A,
        &mut dm,
        ConniveEffect::new(ChooseSpec::Source),
    );
    assert_eq!(keywords(&out, KeywordActionKind::Connive), 1);
    assert_eq!(g.player(A).unwrap().library.len(), 0);
    assert_eq!(g.player(A).unwrap().graveyard.len(), 1);
    assert_eq!(g.counter_count(id, CounterType::PlusOnePlusOne), 1);
}
#[test]
fn f12_blocked_manifest_dread_moves_all_looked_cards_with_replacements() {
    for replace in [false, true] {
        for n in [1, 2] {
            let mut g = game();
            let src = g.new_object_id();
            let mut dm = Dm::default();
            let blocker = B::new(CardId::new(), "No creature entry")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::restriction(
                    ironsmith::effect::Restriction::EnterBattlefield(
                        ObjectFilter::creature().in_zone(Zone::Library),
                    ),
                    "No creature entry".into(),
                )))
                .build();
            g.create_object_from_definition(&blocker, A, Zone::Battlefield);
            g.refresh_continuous_state();
            for _ in 0..n {
                g.create_object_from_definition(&creature("Looked", 1), A, Zone::Library);
            }
            if replace {
                execute(
                    &mut g,
                    src,
                    A,
                    &mut dm,
                    ExileInsteadOfGraveyardEffect::you(),
                );
            }
            let out = execute(&mut g, src, A, &mut dm, ManifestDreadEffect::new());
            assert!(g.player(A).unwrap().library.is_empty());
            assert_eq!(
                g.player(A).unwrap().graveyard.len(),
                if replace { 0 } else { n }
            );
            assert_eq!(
                g.objects_in_zone(Zone::Exile).len(),
                if replace { n } else { 0 }
            );
            assert_eq!(keywords(&out, KeywordActionKind::ManifestDread), 1);
        }
    }
}
#[test]
fn f14_surveil_respects_graveyard_replacement_and_still_emits_surveil() {
    let mut g = game();
    let src = g.new_object_id();
    let mut dm = Dm::default();
    for _ in 0..2 {
        g.create_object_from_definition(&creature("Looked", 1), A, Zone::Library);
    }
    execute(
        &mut g,
        src,
        A,
        &mut dm,
        ExileInsteadOfGraveyardEffect::you(),
    );
    let out = execute(&mut g, src, A, &mut dm, SurveilEffect::you(2));
    assert!(g.player(A).unwrap().library.is_empty());
    assert!(g.player(A).unwrap().graveyard.is_empty());
    assert_eq!(g.objects_in_zone(Zone::Exile).len(), 2);
    assert_eq!(keywords(&out, KeywordActionKind::Surveil), 1);
}
#[test]
fn f15_self_fight_deals_one_combined_damage_event_and_one_fight_event() {
    for (power, prevent_one, expected) in [
        (2, false, 4),
        (2, true, 3),
        (i32::MAX, false, i32::MAX as u32),
    ] {
        let mut g = game();
        let src = g.new_object_id();
        let mut dm = Dm::default();
        let id = g.create_object_from_definition(&creature("Fighter", power), A, Zone::Battlefield);
        if prevent_one {
            g.effect_store.replacement_effects.add_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    src,
                    A,
                    ironsmith::events::damage::matchers::DamageToObjectMatcher::to_creature(),
                    ironsmith::replacement::ReplacementAction::PreventDamageAmount(1),
                ),
            );
        }
        let out = execute(
            &mut g,
            src,
            A,
            &mut dm,
            FightEffect::new(
                ChooseSpec::SpecificObject(id),
                ChooseSpec::SpecificObject(id),
            ),
        );
        let damage = out
            .events
            .iter()
            .filter_map(|e| e.downcast::<DamageEvent>())
            .map(|e| e.amount)
            .collect::<Vec<_>>();
        assert_eq!(damage, vec![expected]);
        assert_eq!(keywords(&out, KeywordActionKind::Fight), 1);
    }
}
#[test]
fn f21_declined_or_unpayable_madness_obeys_graveyard_replacement() {
    for replace in [false, true] {
        for decline in [false, true] {
            let mut g = game();
            let src = g.new_object_id();
            let mut dm = Dm {
                decline,
                ..Default::default()
            };
            let def = B::new(CardId::new(), "Madness")
                .card_types(vec![CardType::Instant])
                .madness(ironsmith::mana::ManaCost::from_symbols(vec![
                    ironsmith::mana::ManaSymbol::Generic(1),
                ]))
                .build();
            let id = g.create_object_from_definition(&def, A, Zone::Exile);
            g.set_madness_exiled(id);
            if replace {
                execute(
                    &mut g,
                    src,
                    A,
                    &mut dm,
                    ExileInsteadOfGraveyardEffect::you(),
                );
            }
            execute(&mut g, id, A, &mut dm, MayCastForMadnessCostEffect::new());
            assert_eq!(g.player(A).unwrap().graveyard.len(), usize::from(!replace));
            assert_eq!(g.objects_in_zone(Zone::Exile).len(), usize::from(replace));
            assert!(!g.is_madness_exiled(id));
        }
    }
}
#[test]
fn f23_life_exchange_allows_ordinary_teammates_but_not_shared_life_teammates() {
    for shared in [false, true] {
        let mut g = teams(shared);
        let src = g.new_object_id();
        let mut dm = Dm::default();
        if !shared {
            g.player_mut(BOB).unwrap().life = 10;
        }
        let before = (g.player(A).unwrap().life, g.player(BOB).unwrap().life);
        execute(
            &mut g,
            src,
            A,
            &mut dm,
            ExchangeLifeTotalsEffect::new(PlayerFilter::Specific(A), PlayerFilter::Specific(BOB)),
        );
        assert_eq!(
            (g.player(A).unwrap().life, g.player(BOB).unwrap().life),
            if shared { before } else { (10, 20) }
        );
    }
}
#[test]
fn f24_exchange_uses_current_not_printed_shared_types() {
    for gain in [true, false] {
        let mut g = game();
        let src = g.new_object_id();
        let mut dm = Dm::default();
        let first = B::new(CardId::new(), "First")
            .card_types(if gain {
                vec![CardType::Land]
            } else {
                vec![CardType::Artifact]
            })
            .build();
        let second = B::new(CardId::new(), "Second")
            .card_types(vec![CardType::Artifact])
            .build();
        let a = g.create_object_from_definition(&first, A, Zone::Battlefield);
        let b = g.create_object_from_definition(&second, BOB, Zone::Battlefield);
        let modification = if gain {
            Modification::AddCardTypes(vec![CardType::Artifact])
        } else {
            Modification::SetCardTypes(vec![CardType::Land])
        };
        execute(
            &mut g,
            src,
            A,
            &mut dm,
            ApplyContinuousEffect::new(EffectTarget::Specific(a), modification, Until::Forever),
        );
        execute(
            &mut g,
            src,
            A,
            &mut dm,
            ExchangeControlEffect::new(
                ChooseSpec::SpecificObject(a),
                ChooseSpec::SpecificObject(b),
            )
            .with_shared_type(SharedTypeConstraint::PermanentType),
        );
        assert_eq!(
            (g.current_controller(a), g.current_controller(b)),
            if gain {
                (Some(BOB), Some(A))
            } else {
                (Some(A), Some(BOB))
            }
        );
    }
}
#[test]
fn f30_bands_with_other_qualifying_pair_survives_an_unrelated_combatant() {
    use ironsmith::combat_state::{
        AttackTarget, AttackerInfo, CombatState, combat_damage_assignment_player,
    };
    for reverse in [false, true] {
        for qualifying_pair in [false, true] {
            let mut g = game();
            let group_controller = if reverse { A } else { BOB };
            let lone_controller = if reverse { BOB } else { A };
            let lone = g.create_object_from_definition(
                &creature("Lone", 5),
                lone_controller,
                Zone::Battlefield,
            );
            let bander = B::new(CardId::new(), "Bander")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Elf])
                .power_toughness(PowerToughness::fixed(2, 2))
                .with_ability(Ability::static_ability(StaticAbility::bands_with_other(
                    ObjectFilter::creature().with_subtype(Subtype::Elf),
                    "bands with other Elves",
                )))
                .build();
            let elf = B::new(CardId::new(), "Other")
                .card_types(vec![CardType::Creature])
                .subtypes(if qualifying_pair {
                    vec![Subtype::Elf]
                } else {
                    vec![]
                })
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            let first =
                g.create_object_from_definition(&bander, group_controller, Zone::Battlefield);
            let second = g.create_object_from_definition(&elf, group_controller, Zone::Battlefield);
            let third = g.create_object_from_definition(
                &creature("Non-Elf", 2),
                group_controller,
                Zone::Battlefield,
            );
            let group = [first, second, third];
            let combat = if reverse {
                CombatState {
                    attackers: group
                        .iter()
                        .map(|id| AttackerInfo {
                            creature: *id,
                            target: AttackTarget::Player(BOB),
                        })
                        .collect(),
                    blockers: group.iter().map(|id| (*id, vec![lone])).collect(),
                    ..Default::default()
                }
            } else {
                CombatState {
                    attackers: vec![AttackerInfo {
                        creature: lone,
                        target: AttackTarget::Player(BOB),
                    }],
                    blockers: std::collections::BTreeMap::from([(lone, group.to_vec())]),
                    ..Default::default()
                }
            };
            assert_eq!(
                combat_damage_assignment_player(&g, &combat, lone),
                Some(if qualifying_pair {
                    group_controller
                } else {
                    lone_controller
                })
            );
        }
    }
}

#[test]
fn f7_exchange_legality_uses_the_target_with_old_aura_still_present() {
    let mut g = game();
    let mut dm = Dm::default();
    let land = B::new(CardId::new(), "Land")
        .card_types(vec![CardType::Land])
        .build();
    let target = g.create_object_from_definition(&land, A, Zone::Battlefield);
    let old = g.create_object_from_definition(
        &aura("Animating Aura", ColorSet::BLUE),
        A,
        Zone::Battlefield,
    );
    let incoming =
        g.create_object_from_definition(&aura("Creature Aura", ColorSet::RED), A, Zone::Hand);
    execute(
        &mut g,
        old,
        A,
        &mut dm,
        ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            Modification::AddCardTypes(vec![CardType::Creature]),
            Until::ThisLeavesTheBattlefield,
        ),
    );
    g.attach_object_to_target(old, AttachmentTarget::Object(target));
    assert!(g.current_is_creature(target));
    let out = execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    let ids = out.objects().expect("legal exchange must complete");
    assert_eq!(g.object(ids[0]).unwrap().zone, Zone::Hand);
    assert_eq!(
        g.object(ids[1]).unwrap().attached_to,
        Some(AttachmentTarget::Object(target))
    );
    assert!(!g.current_is_creature(target));
    assert!(g.object(incoming).is_none());
}

#[test]
fn f7_swap_uses_ability_controller_hand_after_source_changes_controller() {
    let mut g = game();
    let mut dm = Dm::default();
    let target = g.create_object_from_definition(&creature("Target", 2), A, Zone::Battlefield);
    let mut old_def = aura("Old", ColorSet::BLUE);
    old_def.abilities.push(Ability::static_ability(
        StaticAbility::permanents_enter_tapped(),
    ));
    let old = g.create_object_from_definition(&old_def, A, Zone::Battlefield);
    g.create_object_from_definition(&aura("New", ColorSet::RED), A, Zone::Hand);
    g.attach_object_to_target(old, AttachmentTarget::Object(target));
    execute(
        &mut g,
        old,
        A,
        &mut dm,
        ApplyContinuousEffect::new(
            EffectTarget::Specific(old),
            Modification::ChangeController(BOB),
            Until::Forever,
        ),
    );
    assert_eq!(g.current_controller(old), Some(BOB));
    let out = execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    let ids = out.objects().expect("ownership still allows the exchange");
    assert_eq!(g.object(ids[0]).unwrap().zone, Zone::Hand);
    assert_eq!(g.current_controller(ids[1]), Some(A));
    assert!(
        g.is_tapped(ids[1]),
        "departing Aura's replacement must apply to simultaneous entrant"
    );
    assert_eq!(
        g.object(ids[1]).unwrap().attached_to,
        Some(AttachmentTarget::Object(target))
    );
}

#[test]
fn f7_prevalidated_exchange_does_not_recheck_dynamic_entry_filter_after_departure() {
    let mut g = game();
    let mut dm = Dm::default();
    let target = g.create_object_from_definition(&creature("Bearer", 2), A, Zone::Battlefield);
    let old =
        g.create_object_from_definition(&aura("Old Aura", ColorSet::BLUE), A, Zone::Battlefield);
    let incoming =
        g.create_object_from_definition(&aura("Incoming Aura", ColorSet::RED), A, Zone::Hand);
    g.attach_object_to_target(old, AttachmentTarget::Object(target));

    // The incoming Aura has mana value 0. The restriction's count is 1 while
    // the old Aura exists, then 0 after it leaves. This changes the filter's
    // answer without needing to rebuild the cached list of prohibitions.
    let filter = ObjectFilter::default()
        .with_type(CardType::Enchantment)
        .in_zone(Zone::Hand)
        .with_mana_value(ironsmith::filter::Comparison::EqualExpr(Box::new(
            ironsmith::effect::Value::Count(ObjectFilter::specific(old).in_zone(Zone::Battlefield)),
        )));
    let restriction = B::new(CardId::new(), "Dynamic entry restriction")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            ironsmith::effect::Restriction::EnterBattlefield(filter),
            "Matching enchantments cannot enter the battlefield".into(),
        )))
        .build();
    g.create_object_from_definition(&restriction, A, Zone::Battlefield);
    g.refresh_continuous_state();

    let mut sequential = g.clone();
    sequential.move_object_by_effect(old, Zone::Hand).unwrap();
    assert!(
        sequential
            .move_object_by_effect(incoming, Zone::Battlefield)
            .is_none(),
        "ordinary sequential entry must still enforce the now-matching prohibition"
    );

    let outcome = execute(&mut g, old, A, &mut dm, AuraSwapEffect::new());
    let ids = outcome
        .objects()
        .expect("legality checked before either exchange move must be preserved");
    assert_eq!(g.object(ids[0]).unwrap().zone, Zone::Hand);
    assert_eq!(g.object(ids[1]).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        g.object(ids[1]).unwrap().attached_to,
        Some(AttachmentTarget::Object(target))
    );
}
