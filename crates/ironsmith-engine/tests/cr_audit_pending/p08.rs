//! CR audit (pending-entry pass, package p08): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::ColorSet;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::effects::{ApplyContinuousEffect, EffectExecutor};
use ironsmith::{CardId, CardType, ChooseSpec, ObjectId, PowerToughness, StaticAbility, Until, Zone};

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}
fn new_game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture: {name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture: {name} must be fully implemented"
    );
    def
}
fn creature_def(name: &str, p: i32, t: i32, colors: ColorSet) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}
fn put_creature(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    p: i32,
    t: i32,
    colors: ColorSet,
) -> ObjectId {
    game.create_object_from_definition(&creature_def(name, p, t, colors), owner, Zone::Battlefield)
}
fn apply(game: &mut GameState, source: ObjectId, target: ObjectId, modification: Modification) {
    let controller = game.current_controller(source).unwrap_or(alice());
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::EndOfTurn)
        .execute(game, &mut EffectContext::new_default(source, controller))
        .unwrap();
}
fn become_copy(game: &mut GameState, host: ObjectId, donor: ObjectId) {
    use ironsmith::effects::continuous::RuntimeModification;
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(host),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(game, &mut EffectContext::new_default(host, alice()))
    .unwrap();
}
fn pt(game: &GameState, id: ObjectId) -> (Option<i32>, Option<i32>) {
    (game.current_power(id), game.current_toughness(id))
}

// ---------------------------------------------------------------------------
// 611 / 613: continuous effects and the layer system
// ---------------------------------------------------------------------------

#[test]
fn cr_611_1_611_2_resolution_effects_modify_characteristics_control_and_players() {
    use ironsmith::effects::DealDamageEffect;
    let mut game = new_game();
    let source = put_creature(&mut game, alice(), "Effect source", 1, 1, ColorSet::RED);
    let subject = put_creature(&mut game, alice(), "Subject", 2, 2, ColorSet::GREEN);
    apply(&mut game, source, subject, Modification::SetColors(ColorSet::WHITE));
    apply(&mut game, source, subject, Modification::ModifyPowerToughness { power: 1, toughness: 1 });
    apply(&mut game, source, subject, Modification::ChangeController(bob()));
    assert_eq!(game.current_colors(subject), Some(ColorSet::WHITE), "CR 611.1: characteristics are modified");
    assert_eq!(pt(&game, subject), (Some(3), Some(3)), "CR 611.1: P/T modified");
    assert_eq!(game.current_controller(subject), Some(bob()), "CR 611.1: control modified");
    // A resolving ability that affects players / the rules of the game: players can't gain life this turn.
    game.stack.push(ironsmith::StackEntry::ability(
        source,
        alice(),
        vec![ironsmith::Effect::cant_until(
            ironsmith::effect::Restriction::GainLife(ironsmith::PlayerFilter::Any),
            Until::EndOfTurn,
        )],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.update_cant_effects();
    game.stack.push(ironsmith::StackEntry::ability(source, alice(), vec![ironsmith::Effect::gain_life(5)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice()).unwrap().life, 20, "CR 611.1/611.2: a resolved effect can affect players and the rules");
    // Effects last for the indefinite/fixed period: the end-of-turn effects end in cleanup.
    let _ = DealDamageEffect::new(0, ChooseSpec::SpecificObject(subject));
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(game.current_colors(subject), Some(ColorSet::GREEN), "CR 611.1: fixed-period effect ends");
    assert_eq!(game.current_controller(subject), Some(alice()));
}

#[test]
fn cr_613_1a_1b_1e_2_2c_copy_then_control_then_color_regardless_of_timestamp() {
    use ironsmith::snapshot::ObjectSnapshot;
    let mut game = new_game();
    let donor = put_creature(&mut game, bob(), "Red Donor", 4, 4, ColorSet::RED);
    let host = put_creature(&mut game, alice(), "Blue Host", 1, 1, ColorSet::GREEN);
    // Older color and control effects, then a newer copy effect.
    apply(&mut game, host, host, Modification::SetColors(ColorSet::BLUE));
    apply(&mut game, host, host, Modification::ChangeController(bob()));
    become_copy(&mut game, host, donor);
    assert_eq!(game.current_name(host).as_deref(), Some("Red Donor"), "CR 613.1a: copy applies in layer 1");
    assert_eq!(pt(&game, host), (Some(4), Some(4)));
    assert_eq!(
        game.current_colors(host),
        Some(ColorSet::BLUE),
        "CR 613.1e/613.2: the older color-changing effect still applies after the newer copy effect"
    );
    assert_eq!(
        game.current_controller(host),
        Some(bob()),
        "CR 613.1b: control-changing effect applies after layer 1"
    );
    let values = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(host).unwrap(), &game)
        .copiable_values;
    assert_eq!(values.colors, ColorSet::RED, "CR 613.2c: copiable values come only from layer 1");
    assert_eq!(values.name.as_ref() as &str, "Red Donor");
}

#[test]
fn cr_613_5_honor_of_the_pure_and_gray_ogre_examples() {
    let mut game = new_game();
    let honor = compile("Honor of the Pure", "Mana cost: {1}{W}\nType: Enchantment\nWhite creatures you control get +1/+1.");
    game.create_object_from_definition(&honor, alice(), Zone::Battlefield);
    let creature = put_creature(&mut game, alice(), "Black bear", 2, 2, ColorSet::BLACK);
    assert_eq!(pt(&game, creature), (Some(2), Some(2)));
    apply(&mut game, creature, creature, Modification::SetColors(ColorSet::WHITE));
    assert_eq!(pt(&game, creature), (Some(3), Some(3)), "CR 613.5/613.9: turning white immediately applies Honor");
    apply(&mut game, creature, creature, Modification::SetColors(ColorSet::RED));
    assert_eq!(pt(&game, creature), (Some(2), Some(2)), "CR 613.5: Honor stops applying immediately");

    let mut game = new_game();
    let ogre = put_creature(&mut game, alice(), "Gray Ogre", 2, 2, ColorSet::RED);
    game.add_counters(ogre, ironsmith::CounterType::PlusOnePlusOne, 1);
    assert_eq!(pt(&game, ogre), (Some(3), Some(3)));
    apply(&mut game, ogre, ogre, Modification::ModifyPowerToughness { power: 4, toughness: 4 });
    assert_eq!(pt(&game, ogre), (Some(7), Some(7)));
    let anthem = compile("Toughness anthem", "Mana cost: {0}\nType: Enchantment\nCreatures you control get +0/+2.");
    game.create_object_from_definition(&anthem, alice(), Zone::Battlefield);
    assert_eq!(pt(&game, ogre), (Some(7), Some(9)));
    apply(
        &mut game,
        ogre,
        ogre,
        Modification::SetPowerToughness {
            power: ironsmith::effect::Value::Fixed(0),
            toughness: ironsmith::effect::Value::Fixed(1),
            sublayer: ironsmith::continuous::PtSublayer::Setting,
        },
    );
    assert_eq!(pt(&game, ogre), (Some(5), Some(8)), "CR 613.5: 0/1 setting (7b) under 7c modifiers and counter");
}

#[test]
fn cr_613_6_march_of_the_machines_keeps_applying_pt_to_the_type_changed_set() {
    let mut game = new_game();
    let march = compile(
        "March of the Machines",
        "Mana cost: {3}{U}\nType: Enchantment\nEach noncreature artifact is an artifact creature with power and toughness each equal to its mana value.",
    );
    game.create_object_from_definition(&march, alice(), Zone::Battlefield);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Three-drop rock")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ironsmith::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(3)]))
        .build();
    let rock = game.create_object_from_definition(&artifact, bob(), Zone::Battlefield);
    assert!(game.current_is_creature(rock), "CR 613.6: layer 4 part applies");
    assert_eq!(
        pt(&game, rock),
        (Some(3), Some(3)),
        "CR 613.6: the layer 7b part applies to the same objects even though they are no longer noncreature artifacts"
    );
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(game.battlefield.contains(&rock));
}

#[test]
fn cr_613_7c_counters_of_a_kind_share_the_newest_timestamp() {
    use ironsmith::CounterType;
    let mut game = new_game();
    let c = put_creature(&mut game, alice(), "Counter host", 1, 1, ColorSet::GREEN);
    game.add_counters(c, CounterType::PlusOnePlusOne, 1);
    let first = game.effect_store.continuous_effects.get_counter_timestamp(c, CounterType::PlusOnePlusOne).unwrap();
    game.add_counters(c, CounterType::MinusOneMinusOne, 1);
    let other = game.effect_store.continuous_effects.get_counter_timestamp(c, CounterType::MinusOneMinusOne).unwrap();
    assert!(other > first);
    game.add_counters(c, CounterType::PlusOnePlusOne, 1);
    let renewed = game.effect_store.continuous_effects.get_counter_timestamp(c, CounterType::PlusOnePlusOne).unwrap();
    assert!(renewed > other, "CR 613.7c: every counter of that kind gets the new counter's timestamp");
}

#[test]
fn cr_613_7d_objects_receive_a_timestamp_when_entering_a_zone() {
    let mut game = new_game();
    let c = put_creature(&mut game, alice(), "Zone mover", 1, 1, ColorSet::GREEN);
    let first = game.effect_store.continuous_effects.get_entry_timestamp(c).expect("battlefield entry timestamp");
    game.effect_store.continuous_effects.advance_timestamp();
    let in_yard = game.move_object_by_effect(c, Zone::Graveyard).unwrap();
    let yard_ts = game.effect_store.continuous_effects.get_entry_timestamp(in_yard);
    let back = game.move_object_by_effect(in_yard, Zone::Battlefield).unwrap();
    let second = game.effect_store.continuous_effects.get_entry_timestamp(back).expect("re-entry timestamp");
    assert!(second > first, "CR 613.7d: re-entering the battlefield gives a new, later timestamp");
    assert!(
        yard_ts.is_some_and(|ts| ts > first && ts < second),
        "CR 613.7d: entering the graveyard also gives a timestamp (got {yard_ts:?})"
    );
}

#[test]
fn cr_613_7g_transform_and_convert_give_a_new_timestamp() {
    use ironsmith::effects::{ConvertEffect, TransformEffect};
    for convert in [false, true] {
        let mut game = new_game();
        let mut front = creature_def("Front face", 2, 2, ColorSet::RED);
        let mut back = creature_def("Back face", 4, 4, ColorSet::RED);
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.to_string());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.to_string());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let dfc = game.create_object_from_definition(&front, alice(), Zone::Battlefield);
        let before = game.effect_store.continuous_effects.get_object_timestamp(dfc).unwrap();
        // An "all creatures are blue" style effect created after entry but before transforming.
        apply(&mut game, dfc, dfc, Modification::SetColors(ColorSet::BLUE));
        let mut ctx = EffectContext::new_default(dfc, alice());
        if convert {
            ConvertEffect::source().execute(&mut game, &mut ctx).unwrap();
        } else {
            TransformEffect::source().execute(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(game.current_name(dfc).as_deref(), Some("Back face"));
        let after = game.effect_store.continuous_effects.get_object_timestamp(dfc).unwrap();
        assert!(
            after > before && after > game.effect_store.continuous_effects.current_timestamp() - 1,
            "CR 613.7g: convert={convert}: transformed permanent gets a new timestamp ({before} -> {after})"
        );
    }
}

#[test]
fn cr_613_8_8a_8b_8c_blood_moon_removes_urborg_before_urborg_applies() {
    use ironsmith::{Subtype, Supertype};
    let moon = compile("Blood Moon", "Mana cost: {2}{R}\nType: Enchantment\nNonbasic lands are Mountains.");
    let urborg = compile("Urborg, Tomb of Yawgmoth", "Type: Legendary Land\nEach land is a Swamp in addition to its other land types.");
    let forest = CardDefinitionBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    for moon_first in [false, true] {
        let mut game = new_game();
        let f = game.create_object_from_definition(&forest, alice(), Zone::Battlefield);
        let (m, u) = if moon_first {
            let m = game.create_object_from_definition(&moon, bob(), Zone::Battlefield);
            (m, game.create_object_from_definition(&urborg, alice(), Zone::Battlefield))
        } else {
            let u = game.create_object_from_definition(&urborg, alice(), Zone::Battlefield);
            (game.create_object_from_definition(&moon, bob(), Zone::Battlefield), u)
        };
        let _ = m;
        assert!(game.current_has_subtype(u, Subtype::Mountain), "Blood Moon applies to Urborg");
        assert!(
            !game.current_has_subtype(f, Subtype::Swamp) && !game.current_has_subtype(u, Subtype::Swamp),
            "CR 613.8a-c: Urborg's effect depends on Blood Moon (which removes it), so it never applies; moon_first={moon_first}"
        );
        assert!(game.current_has_subtype(f, Subtype::Forest));
    }
}

#[test]
fn cr_613_8a_8b_dependency_loop_applies_in_timestamp_order() {
    let lord = compile(
        "Ability eater",
        "Mana cost: {0}\nType: Creature — Horror\nPower/Toughness: 2/2\nFlying\nAll other creatures lose all abilities.",
    );
    for second_owner in [alice(), bob()] {
        let mut game = new_game();
        let older = game.create_object_from_definition(&lord, alice(), Zone::Battlefield);
        let newer = game.create_object_from_definition(&lord, second_owner, Zone::Battlefield);
        let flying = StaticAbility::flying();
        assert_eq!(
            (game.object_has_ability(older, &flying), game.object_has_ability(newer, &flying)),
            (true, false),
            "CR 613.8b: each effect would remove the other's existence (a loop), so they apply in timestamp order: the older one removes the newer one's ability"
        );
    }
}

#[test]
fn cr_613_8a_cda_effects_apply_before_non_cda_type_effects() {
    use ironsmith::Subtype;
    let shapeshifter = compile(
        "Changeling subject",
        "Mana cost: {0}\nType: Creature — Shapeshifter\nPower/Toughness: 1/1\nChangeling",
    );
    let mut game = new_game();
    let older = put_creature(&mut game, alice(), "Older source", 1, 1, ColorSet::RED);
    let c = game.create_object_from_definition(&shapeshifter, alice(), Zone::Battlefield);
    let _ = older;
    apply(&mut game, c, c, Modification::SetSubtypes(vec![Subtype::Goblin]));
    assert_eq!(
        game.current_subtypes(c),
        Some(vec![Subtype::Goblin]),
        "CR 613.8a(c)/613.3: a CDA and a non-CDA are independent; the CDA applies first, then the subtype-setting effect"
    );
}

#[test]
fn cr_613_9_flying_grant_and_removal_apply_in_timestamp_order() {
    let flying = StaticAbility::flying();
    for grant_first in [false, true] {
        let mut game = new_game();
        let c = put_creature(&mut game, alice(), "Bird", 1, 1, ColorSet::BLUE);
        if grant_first {
            apply(&mut game, c, c, Modification::AddAbility(flying.clone()));
            apply(&mut game, c, c, Modification::RemoveAbility(flying.clone()));
        } else {
            apply(&mut game, c, c, Modification::RemoveAbility(flying.clone()));
            apply(&mut game, c, c, Modification::AddAbility(flying.clone()));
        }
        assert_eq!(
            game.object_has_ability(c, &flying),
            !grant_first,
            "CR 613.9: the later of two independent effects wins (grant_first={grant_first})"
        );
    }
}

#[test]
fn cr_613_2b_face_down_permanent_is_a_nameless_colorless_2_2() {
    let mut game = new_game();
    let def = compile(
        "Flying Lord",
        "Mana cost: {3}{U}{U}\nType: Creature — Bird\nPower/Toughness: 4/5\nFlying",
    );
    let c = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    assert!(game.set_face_down(c));
    assert_eq!(pt(&game, c), (Some(2), Some(2)), "CR 613.2b/708.2: face-down is 2/2");
    assert_eq!(game.current_colors(c), Some(ColorSet::COLORLESS));
    assert!(game.current_subtypes(c).unwrap().is_empty());
    assert!(!game.object_has_ability(c, &StaticAbility::flying()));
    let name = game.current_name(c).unwrap_or_default();
    assert!(name.is_empty() || name == ironsmith::object::FACE_DOWN_DISPLAY_NAME, "nameless: {name}");
}

fn white_static_text(kind: &str) -> String {
    format!("Mana cost: {{0}}\nType: {kind}\nAll creatures are white.")
}

#[test]
fn cr_613_7h_plane_receives_a_timestamp_when_turned_face_up() {
    use ironsmith::PlanarCardKind;
    let plane = compile("Whitewash Plane", &white_static_text("Plane — Whitewash"));
    let filler = |i: usize| {
        CardDefinitionBuilder::new(CardId::new(), &format!("Filler Plane {i}"))
            .card_types(vec![CardType::Plane])
            .build()
    };
    let mut deck: Vec<_> = (0..9).map(|i| (filler(i), PlanarCardKind::Plane)).collect();
    deck.push((plane, PlanarCardKind::Plane)); // last = top of the planar deck
    let bob_deck: Vec<_> = (10..20).map(|i| (filler(i), PlanarCardKind::Plane)).collect();
    let mut game = new_game();
    game.enable_planechase(vec![(alice(), deck), (bob(), bob_deck)]).unwrap();
    let c = put_creature(&mut game, alice(), "Bear", 2, 2, ColorSet::GREEN);
    // Resolution effect created before the plane turns face up.
    apply(&mut game, c, c, Modification::SetColors(ColorSet::BLUE));
    assert_eq!(game.current_colors(c), Some(ColorSet::BLUE));
    {
        let target = game
            .planar_deck(alice())
            .unwrap()
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().name.as_ref() as &str == "Whitewash Plane")
            .unwrap();
        let state = game.planechase.as_mut().unwrap();
        let deck = state.decks.get_mut(&alice()).unwrap();
        deck.retain(|id| *id != target);
        deck.push(target);
    }
    game.turn.active_player = alice();
    let revealed = game.reveal_starting_plane().unwrap();
    assert_eq!(game.object(revealed).unwrap().name.as_ref() as &str, "Whitewash Plane", "fixture: top plane revealed");
    let control = put_creature(&mut game, alice(), "Control bear", 2, 2, ColorSet::GREEN);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_colors(control), Some(ColorSet::WHITE), "fixture: the face-up plane's static ability functions");
    assert_eq!(
        game.current_colors(c),
        Some(ColorSet::WHITE),
        "CR 613.7h: the plane's timestamp is when it turned face up, after the older blue effect"
    );
}

#[test]
fn cr_613_7i_vanguard_timestamp_is_the_beginning_of_the_game() {
    let mut vanguard = compile("Pale Avatar", &white_static_text("Vanguard"));
    vanguard.card.card_types = vec![CardType::Vanguard];
    vanguard.card.hand_modifier = 0;
    vanguard.card.life_modifier = 0;
    let other = CardDefinitionBuilder::new(CardId::new(), "Plain Avatar")
        .card_types(vec![CardType::Vanguard])
        .build();
    let mut game = new_game();
    game.enable_vanguard(vec![(alice(), vanguard), (bob(), other)]).unwrap();
    let c = put_creature(&mut game, alice(), "Bear", 2, 2, ColorSet::GREEN);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_colors(c), Some(ColorSet::WHITE), "vanguard static functions from command");
    apply(&mut game, c, c, Modification::SetColors(ColorSet::BLUE));
    assert_eq!(
        game.current_colors(c),
        Some(ColorSet::BLUE),
        "CR 613.7i: the vanguard's effect has the beginning-of-game timestamp, older than any later effect"
    );
}

#[test]
fn cr_613_7j_conspiracy_timestamps_at_start_and_when_turned_face_up() {
    use ironsmith::ConspiracySetupCard;
    for hidden in [false, true] {
        let text = if hidden {
            "Mana cost: {0}\nType: Conspiracy\nHidden agenda\nAll creatures are white."
        } else {
            "Mana cost: {0}\nType: Conspiracy\nAll creatures are white."
        };
        let mut conspiracy = compile("Pale Conspiracy", text);
        conspiracy.card.card_types = vec![CardType::Conspiracy];
        let mut game = new_game();
        game.enable_conspiracy(vec![(
            alice(),
            vec![ConspiracySetupCard {
                definition: conspiracy,
                agenda_names: if hidden { vec!["Grizzly Bears".into()] } else { Vec::new() },
            }],
        )])
        .unwrap();
        let id = game.conspiracy_cards()[0];
        let c = put_creature(&mut game, alice(), "Bear", 2, 2, ColorSet::GREEN);
        apply(&mut game, c, c, Modification::SetColors(ColorSet::BLUE));
        if hidden {
            game.turn.priority_player = Some(alice());
            struct Dm;
            impl ironsmith::DecisionMaker for Dm {}
            ironsmith::special_actions::perform(
                ironsmith::special_actions::SpecialAction::TurnConspiracyFaceUp { conspiracy_id: id },
                &mut game,
                alice(),
                &mut Dm,
            )
            .unwrap();
            assert!(!game.is_face_down_conspiracy(id));
        }
        let control = put_creature(&mut game, alice(), "Control bear", 2, 2, ColorSet::GREEN);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_colors(control), Some(ColorSet::WHITE), "fixture: the face-up conspiracy's static ability functions");
        assert_eq!(
            game.current_colors(c),
            Some(if hidden { ColorSet::WHITE } else { ColorSet::BLUE }),
            "CR 613.7j: hidden={hidden}: a face-up conspiracy is timestamped at game start; a face-down one gets a new timestamp when turned face up"
        );
    }
}

#[test]
fn cr_613_7k_sticker_gets_new_timestamp_after_its_objects_new_timestamp() {
    use ironsmith::effects::{PutStickerEffect, TransformEffect};
    use ironsmith::events::KeywordActionKind;
    let mut game = new_game();
    let mut front = creature_def("Sticker front", 2, 2, ColorSet::RED);
    let mut back = creature_def("Sticker back", 2, 2, ColorSet::RED);
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let dfc = game.create_object_from_definition(&front, alice(), Zone::Battlefield);
    game.add_accessible_pt_sticker(alice(), 0, 6, 7);
    PutStickerEffect::new(ChooseSpec::SpecificObject(dfc), KeywordActionKind::PowerToughnessSticker)
        .execute(&mut game, &mut EffectContext::new_default(dfc, alice()))
        .unwrap();
    assert_eq!(pt(&game, dfc), (Some(6), Some(7)), "fixture: sticker sets P/T");
    apply(
        &mut game,
        dfc,
        dfc,
        Modification::SetPowerToughness {
            power: ironsmith::effect::Value::Fixed(1),
            toughness: ironsmith::effect::Value::Fixed(1),
            sublayer: ironsmith::continuous::PtSublayer::Setting,
        },
    );
    assert_eq!(pt(&game, dfc), (Some(1), Some(1)), "CR 613.7k: the later setting effect beats the older sticker");
    TransformEffect::source()
        .execute(&mut game, &mut EffectContext::new_default(dfc, alice()))
        .unwrap();
    assert_eq!(game.current_name(dfc).as_deref(), Some("Sticker back"));
    assert_eq!(
        pt(&game, dfc),
        (Some(6), Some(7)),
        "CR 613.7k: when the object gets a new timestamp the sticker gets one immediately after, so it now beats the setting effect"
    );
}

struct OrderBobFirst {
    prompts: Vec<PlayerId>,
}
impl ironsmith::DecisionMaker for OrderBobFirst {
    fn decide_order(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::OrderContext) -> Vec<ObjectId> {
        self.prompts.push(ctx.player);
        let mut items: Vec<ObjectId> = ctx.items.iter().map(|(id, _)| *id).collect();
        items.sort_by_key(|id| if game.object(*id).map(|o| o.owner) == Some(bob()) { 0 } else { 1 });
        items
    }
}

#[test]
fn cr_613_7m_simultaneous_entry_timestamps_follow_apnap() {
    let lord = compile(
        "Anthem bear",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2\nOther creatures you control get +1/+1.",
    );
    let mass = compile(
        "Mass return",
        "Mana cost: {0}\nType: Sorcery\nReturn all creature cards from all graveyards to the battlefield under their owners' control.",
    );
    let mut game = new_game();
    game.turn.active_player = alice();
    let a = game.create_object_from_definition(&lord, alice(), Zone::Graveyard);
    let b = game.create_object_from_definition(&lord, bob(), Zone::Graveyard);
    let (sa, sb) = (game.object(a).unwrap().stable_id, game.object(b).unwrap().stable_id);
    let spell = game.create_object_from_definition(&mass, alice(), Zone::Stack);
    game.stack.push(ironsmith::StackEntry::new(spell, alice()));
    let mut dm = OrderBobFirst { prompts: Vec::new() };
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let a = game.find_object_by_stable_id(sa).unwrap();
    let b = game.find_object_by_stable_id(sb).unwrap();
    assert!(game.battlefield.contains(&a) && game.battlefield.contains(&b), "fixture: both returned");
    let ts = |id| game.effect_store.continuous_effects.get_entry_timestamp(id).unwrap();
    assert!(
        ts(a) < ts(b),
        "CR 613.7m: the active player's object gets the earlier timestamp regardless of a requested order putting the opponent's first (prompts: {:?}; a={}, b={})",
        dm.prompts,
        ts(a),
        ts(b)
    );
}

#[test]
fn cr_613_7n_own_static_effect_is_older_than_simultaneous_resolution_effect() {
    let painter = compile(
        "White painter",
        "Mana cost: {1}{W}\nType: Creature — Human\nPower/Toughness: 2/2\nAll creatures are white.",
    );
    let rise = compile(
        "Rise from the Grave",
        "Mana cost: {4}{B}\nType: Sorcery\nPut target creature card from a graveyard onto the battlefield under your control. That creature is a black Zombie in addition to its other colors and types.",
    );
    let mut game = new_game();
    let card = game.create_object_from_definition(&painter, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    let spell = game.create_object_from_definition(&rise, alice(), Zone::Stack);
    game.stack.push(ironsmith::StackEntry::new(spell, alice()).with_targets(vec![ironsmith::Target::Object(card)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let c = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.battlefield.contains(&c), "fixture: creature returned");
    assert_eq!(
        game.current_colors(c),
        Some(ColorSet::WHITE.union(ColorSet::BLACK)),
        "CR 613.7n: the object's own 'all creatures are white' effect is older, so the resolving 'black in addition' still adds black"
    );
}
// ---------------------------------------------------------------------------
// 614 / 615 / 616: replacement and prevention effects
// ---------------------------------------------------------------------------

fn cast_with(
    game: &mut GameState,
    def: &CardDefinition,
    controller: PlayerId,
    targets: Vec<ironsmith::Target>,
    dm: &mut dyn ironsmith::DecisionMaker,
) {
    let spell = game.create_object_from_definition(def, controller, Zone::Stack);
    game.stack.push(ironsmith::StackEntry::new(spell, controller).with_targets(targets));
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}
fn cast(game: &mut GameState, def: &CardDefinition, controller: PlayerId, targets: Vec<ironsmith::Target>) {
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    cast_with(game, def, controller, targets, &mut dm);
}
fn deal(game: &mut GameState, source: ObjectId, to: ChooseSpec, amount: u32) {
    let controller = game.current_controller(source).unwrap_or(alice());
    ironsmith::effects::DealDamageEffect::new(amount, to)
        .execute(game, &mut EffectContext::new_default(source, controller))
        .unwrap();
}
fn to_player(p: PlayerId) -> ChooseSpec {
    ChooseSpec::Player(ironsmith::PlayerFilter::Specific(p))
}
fn obj(id: ObjectId) -> ironsmith::Target {
    ironsmith::Target::Object(id)
}
fn library_cards(game: &mut GameState, owner: PlayerId, n: usize) {
    for i in 0..n {
        let d = CardDefinitionBuilder::new(CardId::new(), &format!("Library card {i}"))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_definition(&d, owner, Zone::Library);
    }
}

#[test]
fn cr_614_1a_instead_replaces_the_graveyard_move_with_exile() {
    let rip = compile(
        "Rest in Peace",
        "Mana cost: {1}{W}\nType: Enchantment\nWhen Rest in Peace enters, exile all graveyards.\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
    );
    let destroy = compile("Destroy", "Mana cost: {0}\nType: Instant\nDestroy target creature.");
    let mut game = new_game();
    game.create_object_from_definition(&rip, alice(), Zone::Battlefield);
    let c = put_creature(&mut game, bob(), "Victim", 2, 2, ColorSet::GREEN);
    let stable = game.object(c).unwrap().stable_id;
    cast(&mut game, &destroy, alice(), vec![obj(c)]);
    let moved = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(moved).unwrap().zone, Zone::Exile, "CR 614.1a: 'instead' replaces the graveyard move");
}

#[test]
fn cr_614_1d_enters_tapped_is_a_replacement_effect() {
    let tapland = compile("Tapland", "Type: Land\nThis land enters tapped.\n{T}: Add {G}.");
    let reanimate = compile("Reanimate land", "Mana cost: {0}\nType: Sorcery\nReturn target land card from your graveyard to the battlefield.");
    let mut game = new_game();
    let land = game.create_object_from_definition(&tapland, alice(), Zone::Graveyard);
    let stable = game.object(land).unwrap().stable_id;
    cast(&mut game, &reanimate, alice(), vec![obj(land)]);
    let land = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.battlefield.contains(&land));
    assert!(game.is_tapped(land), "CR 614.1d: '[this permanent] enters tapped' modifies the entry event");
}

#[test]
fn cr_614_1e_as_turned_face_up_replacement_applies() {
    let hydra = compile(
        "Hooded Hydra",
        "Mana cost: {3}{G}{G}\nType: Creature — Snake Hydra\nPower/Toughness: 0/0\nHooded Hydra enters with X +1/+1 counters on it.\nWhen Hooded Hydra dies, create a 1/1 green Snake creature token for each +1/+1 counter on it.\nMorph {3}{G}{G}\nAs Hooded Hydra is turned face up, put five +1/+1 counters on it.",
    );
    let mut game = new_game();
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    let h = game.create_object_from_definition(&hydra, alice(), Zone::Battlefield);
    assert!(game.set_face_down(h));
    game.player_mut(alice()).unwrap().mana_pool.green = 5;
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::TurnFaceUp {
            permanent_id: h,
            method: ironsmith::special_actions::TurnFaceUpMethod::TurnFaceUpAbility,
        },
        &mut game,
        alice(),
        &mut dm,
    )
    .unwrap();
    assert!(!game.is_face_down(h), "fixture: turned face up");
    assert_eq!(
        game.counter_count(h, ironsmith::CounterType::PlusOnePlusOne),
        5,
        "CR 614.1e: 'As this is turned face up' puts the counters on as part of turning face up"
    );
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(game.battlefield.contains(&h), "CR 614.1e: it is never a 0/0 face-up creature");
}

#[test]
fn cr_614_2_615_2_source_based_prevention_applies_only_to_that_sources_damage() {
    let fog = compile("Source Fog", "Mana cost: {0}\nType: Instant\nPrevent all damage target creature would deal this turn.");
    let mut game = new_game();
    let a = put_creature(&mut game, bob(), "Attacker A", 3, 3, ColorSet::RED);
    let b = put_creature(&mut game, bob(), "Attacker B", 3, 3, ColorSet::RED);
    cast(&mut game, &fog, alice(), vec![obj(a)]);
    deal(&mut game, a, to_player(alice()), 3);
    assert_eq!(game.player(alice()).unwrap().life, 20, "CR 615.2/609.7: damage from the chosen source is prevented");
    deal(&mut game, b, to_player(alice()), 3);
    assert_eq!(game.player(alice()).unwrap().life, 17, "CR 609.7: other sources are unaffected");
}

#[test]
fn cr_614_3_614_4_615_1_615_3_615_4_615_7_shields_are_used_up_expire_and_cannot_go_back_in_time() {
    let shield = compile("Shield Spell", "Mana cost: {W}\nType: Instant\nPrevent the next 3 damage that would be dealt to any target this turn.");
    let mut game = new_game();
    let src = put_creature(&mut game, bob(), "Pinger", 1, 1, ColorSet::RED);
    let c = put_creature(&mut game, alice(), "Wall", 0, 10, ColorSet::WHITE);
    // 614.4/615.4: damage already dealt is not undone by a later shield.
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 2);
    cast(&mut game, &shield, alice(), vec![obj(c)]);
    assert_eq!(game.damage_on(c), 2, "CR 615.4: a shield can't go back in time");
    // 615.7: shield counts damage, not events.
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 2);
    assert_eq!(game.damage_on(c), 2, "CR 615.7: first 2 damage prevented");
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 2);
    assert_eq!(game.damage_on(c), 3, "CR 615.7: shield has 1 left, so 1 of 2 is dealt");
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 1);
    assert_eq!(game.damage_on(c), 4, "CR 614.3/615.3: a used-up shield is gone");
    // Duration: a fresh shield ends at cleanup.
    cast(&mut game, &shield, alice(), vec![obj(c)]);
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(game.damage_on(c), 0);
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 2);
    assert_eq!(game.damage_on(c), 2, "CR 614.3/615.3: 'this turn' shield expired");
}

#[test]
fn cr_614_8_regeneration_taps_removes_damage_and_removes_from_combat() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    let regen = compile("Regen", "Mana cost: {0}\nType: Instant\nRegenerate target creature.");
    let destroy = compile("Destroy", "Mana cost: {0}\nType: Instant\nDestroy target creature.");
    let mut game = new_game();
    let src = put_creature(&mut game, bob(), "Pinger", 1, 1, ColorSet::RED);
    let c = put_creature(&mut game, alice(), "Regenerator", 2, 4, ColorSet::GREEN);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: c, target: AttackTarget::Player(bob()) });
    game.combat = Some(combat);
    deal(&mut game, src, ChooseSpec::SpecificObject(c), 1);
    cast(&mut game, &regen, alice(), vec![obj(c)]);
    cast(&mut game, &destroy, bob(), vec![obj(c)]);
    assert!(game.battlefield.contains(&c), "CR 614.8: destruction replaced");
    assert!(game.is_tapped(c), "CR 614.8: tapped");
    assert_eq!(game.damage_on(c), 0, "CR 614.8: all damage removed");
    assert!(
        !ironsmith::combat_state::is_attacking(game.combat.as_ref().unwrap(), c),
        "CR 614.8: removed from combat"
    );
    // A second destruction is not replaced: the shield applies to the next destruction only.
    cast(&mut game, &destroy, bob(), vec![obj(c)]);
    assert!(!game.battlefield.contains(&c));
}

#[test]
fn cr_614_9_redirection_does_nothing_if_the_new_recipient_left_the_battlefield() {
    let redirect = compile(
        "Redirector",
        "Mana cost: {0}\nType: Instant\nThe next 2 damage that would be dealt to target creature this turn is dealt to another target creature instead.",
    );
    for leaves in [false, true] {
        let mut game = new_game();
        let src = put_creature(&mut game, bob(), "Pinger", 1, 1, ColorSet::RED);
        let protected = put_creature(&mut game, alice(), "Protected", 0, 10, ColorSet::WHITE);
        let decoy = put_creature(&mut game, bob(), "Decoy", 0, 10, ColorSet::BLACK);
        native_cast(&mut game, &redirect, alice(), vec![obj(protected), obj(decoy)]);
        if leaves {
            game.move_object_by_effect(decoy, Zone::Exile).unwrap();
        }
        deal(&mut game, src, ChooseSpec::SpecificObject(protected), 2);
        assert_eq!(
            game.damage_on(protected),
            if leaves { 2 } else { 0 },
            "CR 614.9: leaves={leaves}: redirection to a departed permanent does nothing"
        );
        if !leaves {
            assert_eq!(game.damage_on(decoy), 2, "CR 614.9: redirected damage");
        }
    }
}

#[test]
fn cr_614_11b_616_2_replaced_draw_is_not_revealed_and_gain_becomes_draw_becomes_return() {
    let draw_to_return = compile(
        "Draw to Return",
        "Mana cost: {0}\nType: Enchantment\nIf you would draw a card, return a card from your graveyard to your hand instead.",
    );
    let gain_to_draw = compile(
        "Gain to Draw",
        "Mana cost: {0}\nType: Enchantment\nIf you would gain life, draw that many cards instead.",
    );
    let reveal = compile(
        "Reveal Draw",
        "Mana cost: {0}\nType: Sorcery\nDraw a card and reveal it. If it's a land card, you gain 3 life.",
    );
    let land = CardDefinitionBuilder::new(CardId::new(), "Yard Land").card_types(vec![CardType::Land]).build();
    // 614.11b: the returned card is not the drawn card; no "if it's a land" bonus.
    let mut game = new_game();
    library_cards(&mut game, alice(), 3);
    game.create_object_from_definition(&draw_to_return, alice(), Zone::Battlefield);
    let yard = game.create_object_from_definition(&land, alice(), Zone::Graveyard);
    let stable = game.object(yard).unwrap().stable_id;
    cast(&mut game, &reveal, alice(), vec![]);
    let moved = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(moved).unwrap().zone, Zone::Hand, "fixture: draw replaced by return");
    assert_eq!(game.player(alice()).unwrap().life, 20, "CR 614.11b: the additional action is not performed on the returned card");
    // 616.2: gain 1 life -> draw instead -> return instead.
    let mut game = new_game();
    library_cards(&mut game, alice(), 3);
    game.create_object_from_definition(&draw_to_return, alice(), Zone::Battlefield);
    game.create_object_from_definition(&gain_to_draw, alice(), Zone::Battlefield);
    let yard = game.create_object_from_definition(&land, alice(), Zone::Graveyard);
    let stable = game.object(yard).unwrap().stable_id;
    let src = put_creature(&mut game, alice(), "Healer", 1, 1, ColorSet::WHITE);
    game.stack.push(ironsmith::StackEntry::ability(src, alice(), vec![ironsmith::Effect::gain_life(1)]));
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let moved = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.player(alice()).unwrap().life, 20, "CR 616.2: no life gained");
    assert_eq!(game.player(alice()).unwrap().library.len(), 3, "CR 616.2: no card drawn");
    assert_eq!(game.object(moved).unwrap().zone, Zone::Hand, "CR 616.2: the graveyard card was returned");
}

#[test]
fn cr_614_15_616_1a_self_replacement_applies_before_other_replacements() {
    let fiery = compile(
        "Fiery Self",
        "Mana cost: {0}\nType: Sorcery\nThis spell deals 2 damage to target creature. If you control an artifact, it deals 4 damage instead.",
    );
    let torbran = compile(
        "Torbran",
        "Mana cost: {1}{R}{R}{R}\nType: Legendary Creature — Dwarf Noble\nPower/Toughness: 2/4\nIf a red source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus 2 instead.",
    );
    let mut fiery = fiery;
    fiery.card.color_indicator = Some(ColorSet::RED);
    for artifact in [false, true] {
        let mut game = new_game();
        game.create_object_from_definition(&torbran, alice(), Zone::Battlefield);
        if artifact {
            let a = CardDefinitionBuilder::new(CardId::new(), "Trinket").card_types(vec![CardType::Artifact]).build();
            game.create_object_from_definition(&a, alice(), Zone::Battlefield);
        }
        let target = put_creature(&mut game, bob(), "Big target", 0, 20, ColorSet::GREEN);
        cast(&mut game, &fiery, alice(), vec![obj(target)]);
        assert_eq!(
            game.damage_on(target),
            if artifact { 6 } else { 4 },
            "CR 614.15/616.1a: artifact={artifact}: the self-replacement (4 instead of 2) applies first, then +2"
        );
    }
}

#[test]
fn cr_614_16_counter_doubling_applies_to_enters_with_counters_replacement() {
    let doubling = compile(
        "Doubling Season",
        "Mana cost: {4}{G}\nType: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.\nIf an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.",
    );
    let bear = compile("Twobear", "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 0/0\nThis creature enters with two +1/+1 counters on it.");
    let reanimate = compile("Reanimate", "Mana cost: {0}\nType: Sorcery\nReturn target creature card from your graveyard to the battlefield.");
    let mut game = new_game();
    game.create_object_from_definition(&doubling, alice(), Zone::Battlefield);
    let card = game.create_object_from_definition(&bear, alice(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    cast(&mut game, &reanimate, alice(), vec![obj(card)]);
    let c = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.counter_count(c, ironsmith::CounterType::PlusOnePlusOne),
        4,
        "CR 614.16: the counter replacement applies to counters placed by another replacement effect"
    );
}

#[test]
fn cr_614_17a_17b_cant_effects_block_costs_and_do_not_go_back_in_time() {
    let fanatic = compile("Mogg Fanatic", "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1\nSacrifice this creature: It deals 1 damage to any target.");
    let ban = compile("Sac Ban", "Mana cost: {0}\nType: Enchantment\nCreatures can't be sacrificed.");
    for banned in [false, true] {
        let mut game = new_game();
        game.turn.active_player = alice();
        game.turn.priority_player = Some(alice());
        game.turn.phase = ironsmith::Phase::FirstMain;
        let f = game.create_object_from_definition(&fanatic, alice(), Zone::Battlefield);
        if banned {
            game.create_object_from_definition(&ban, bob(), Zone::Battlefield);
        }
        game.refresh_continuous_state().unwrap();
        game.update_cant_effects();
        let can = ironsmith::compute_legal_actions(&game, alice())
            .unwrap()
            .iter()
            .any(|a| matches!(a, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == f));
        assert_eq!(can, !banned, "CR 614.17b: banned={banned}: a cost including an impossible event can't be paid");
    }
    // 614.17a: a later "can't gain life" does not undo an earlier gain.
    let mut game = new_game();
    let src = put_creature(&mut game, alice(), "Healer", 1, 1, ColorSet::WHITE);
    game.stack.push(ironsmith::StackEntry::ability(src, alice(), vec![ironsmith::Effect::gain_life(3)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.stack.push(ironsmith::StackEntry::ability(
        src,
        alice(),
        vec![ironsmith::Effect::cant_until(
            ironsmith::effect::Restriction::GainLife(ironsmith::PlayerFilter::Any),
            Until::EndOfTurn,
        )],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.update_cant_effects();
    assert_eq!(game.player(alice()).unwrap().life, 23, "CR 614.17a");
    game.stack.push(ironsmith::StackEntry::ability(src, alice(), vec![ironsmith::Effect::gain_life(3)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice()).unwrap().life, 23, "CR 614.17: the can't effect stops later gains");
}

struct ChooseSource(ObjectId);
impl ironsmith::DecisionMaker for ChooseSource {
    fn decide_objects(&mut self, _g: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.candidates.iter().any(|c| c.id == self.0 && c.legal) {
            vec![self.0]
        } else {
            ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(ctx.min).collect()
        }
    }
}

#[test]
fn cr_615_5_615_6_prevention_with_additional_effect_and_prevented_damage_never_happens() {
    let reverse = compile(
        "Reverse Damage",
        "Mana cost: {1}{W}{W}\nType: Instant\nThe next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.",
    );
    let fog = compile("Fog Self", "Mana cost: {0}\nType: Instant\nPrevent all damage that would be dealt to you this turn.");
    let linker = compile("Lifelinker", "Mana cost: {0}\nType: Creature — Cat\nPower/Toughness: 3/3\nLifelink");
    let mut game = new_game();
    let src = game.create_object_from_definition(&linker, bob(), Zone::Battlefield);
    let mut dm = ChooseSource(src);
    cast_with(&mut game, &reverse, alice(), vec![], &mut dm);
    deal(&mut game, src, to_player(alice()), 3);
    assert_eq!(game.player(alice()).unwrap().life, 23, "CR 615.5: prevented, then gain equal to prevented amount");
    assert_eq!(game.player(bob()).unwrap().life, 20, "CR 615.6: prevented damage never happens, so no lifelink");
    // 615.6 with a plain prevention.
    cast(&mut game, &fog, alice(), vec![]);
    let outcome = ironsmith::effects::DealDamageEffect::new(3, to_player(alice()))
        .execute(&mut game, &mut EffectContext::new_default(src, bob()))
        .unwrap();
    assert!(
        !outcome.events.iter().any(|e| e.downcast::<ironsmith::events::DamageEvent>().is_some()),
        "CR 615.6: no damage event"
    );
    assert_eq!(game.player(bob()).unwrap().life, 20);
    assert_eq!(game.player(alice()).unwrap().life, 23);
}

#[test]
fn cr_615_8_615_9_next_instance_from_a_red_source_rechecks_color() {
    let cop = compile(
        "CoP instant",
        "Mana cost: {0}\nType: Instant\nThe next time a red source of your choice would deal damage to you this turn, prevent that damage.",
    );
    let mut game = new_game();
    let src = put_creature(&mut game, bob(), "Red source", 3, 3, ColorSet::RED);
    let mut dm = ChooseSource(src);
    cast_with(&mut game, &cop, alice(), vec![], &mut dm);
    // 615.9: the source is no longer red: not prevented, shield not used.
    apply(&mut game, src, src, Modification::SetColors(ColorSet::BLUE));
    deal(&mut game, src, to_player(alice()), 2);
    assert_eq!(game.player(alice()).unwrap().life, 18, "CR 615.9: properties rechecked; not prevented");
    apply(&mut game, src, src, Modification::SetColors(ColorSet::RED));
    deal(&mut game, src, to_player(alice()), 5);
    assert_eq!(game.player(alice()).unwrap().life, 18, "CR 615.9/615.8: shield unused before, now prevents the whole instance");
    deal(&mut game, src, to_player(alice()), 2);
    assert_eq!(game.player(alice()).unwrap().life, 16, "CR 615.8: later instances are dealt normally");
}

#[test]
fn cr_615_10_static_prevent_one_applies_to_each_damage_event() {
    let defender = compile(
        "Daunting Defender",
        "Mana cost: {4}{W}\nType: Creature — Human Cleric\nPower/Toughness: 3/3\nIf a source would deal damage to a Cleric creature you control, prevent 1 of that damage.",
    );
    let pyro = compile("Pyroclasm", "Mana cost: {1}{R}\nType: Sorcery\nPyroclasm deals 2 damage to each creature.");
    let mut game = new_game();
    let d = game.create_object_from_definition(&defender, alice(), Zone::Battlefield);
    let cleric = CardDefinitionBuilder::new(CardId::new(), "Cleric")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![ironsmith::Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 5))
        .build();
    let c2 = game.create_object_from_definition(&cleric, alice(), Zone::Battlefield);
    let other = put_creature(&mut game, alice(), "Non-cleric", 1, 5, ColorSet::GREEN);
    cast(&mut game, &pyro, bob(), vec![]);
    assert_eq!(
        (game.damage_on(d), game.damage_on(c2), game.damage_on(other)),
        (1, 1, 2),
        "CR 615.10: 1 is prevented from each simultaneous applicable event"
    );
}

#[test]
fn cr_615_11_untargeted_shields_are_fixed_at_resolution() {
    let wojek = compile(
        "Wojek shield",
        "Mana cost: {0}\nType: Instant\nPrevent the next 1 damage that would be dealt to target creature and each other creature that shares a color with it this turn.",
    );
    let mut game = new_game();
    let src = put_creature(&mut game, bob(), "Pinger", 1, 1, ColorSet::BLACK);
    let target = put_creature(&mut game, alice(), "Target", 0, 9, ColorSet::WHITE);
    let sharer = put_creature(&mut game, alice(), "Sharer", 0, 9, ColorSet::WHITE);
    let recolored = put_creature(&mut game, alice(), "Later white", 0, 9, ColorSet::GREEN);
    cast(&mut game, &wojek, alice(), vec![obj(target)]);
    apply(&mut game, src, recolored, Modification::SetColors(ColorSet::WHITE));
    apply(&mut game, src, sharer, Modification::SetColors(ColorSet::GREEN));
    let late = put_creature(&mut game, alice(), "Late white", 0, 9, ColorSet::WHITE);
    for id in [target, sharer, recolored, late] {
        deal(&mut game, src, ChooseSpec::SpecificObject(id), 2);
    }
    assert_eq!(
        (game.damage_on(target), game.damage_on(sharer), game.damage_on(recolored), game.damage_on(late)),
        (1, 1, 2, 2),
        "CR 615.11: shields go to the target and creatures sharing a color at resolution only"
    );
}

#[test]
fn cr_615_13_prevention_trigger_fires_once_per_application() {
    let squire = compile(
        "Selfless Squire",
        "Mana cost: {3}{W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nFlash\nWhen this creature enters, prevent all damage that would be dealt to you this turn.\nWhenever damage that would be dealt to you is prevented, put that many +1/+1 counters on this creature.",
    );
    let fog = compile("Fog Self", "Mana cost: {0}\nType: Instant\nPrevent all damage that would be dealt to you this turn.");
    let mut game = new_game();
    let s = game.create_object_from_definition(&squire, alice(), Zone::Battlefield);
    let src = put_creature(&mut game, bob(), "Hitter", 3, 3, ColorSet::RED);
    cast(&mut game, &fog, alice(), vec![]);
    let _ = game.take_pending_trigger_events();
    let outcome = ironsmith::effects::DealDamageEffect::new(3, to_player(alice()))
        .execute(&mut game, &mut EffectContext::new_default(src, bob()))
        .unwrap();
    assert_eq!(game.player(alice()).unwrap().life, 20);
    let _ = outcome;
    let mut q = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut q).unwrap();
    let triggers = q.entries.iter().filter(|e| e.source == s).count();
    assert_eq!(triggers, 1, "CR 615.13: one trigger for one application of a prevention effect");
}

struct PickReplacement(bool);
impl ironsmith::DecisionMaker for PickReplacement {
    fn decide_options(&mut self, _g: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
        let legal: Vec<usize> = ctx.options.iter().filter(|o| o.legal).map(|o| o.index).collect();
        if ctx.min == 0 && ctx.max == 0 {
            return vec![];
        }
        vec![if self.0 { legal[0] } else { *legal.last().unwrap() }]
    }
}

#[test]
fn cr_616_1e_affected_player_chooses_any_applicable_replacement() {
    use ironsmith::events::life::matchers::WouldGainLifeMatcher;
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    let mut results = Vec::new();
    for first in [true, false] {
        let mut game = new_game();
        let src = put_creature(&mut game, alice(), "Healer", 1, 1, ColorSet::WHITE);
        for m in [EventModification::Add(1), EventModification::Multiply(2)] {
            game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                src,
                alice(),
                WouldGainLifeMatcher::you(),
                ReplacementAction::Modify(m),
            ));
        }
        game.stack.push(ironsmith::StackEntry::ability(src, alice(), vec![ironsmith::Effect::gain_life(1)]));
        let mut dm = PickReplacement(first);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        results.push(game.player(alice()).unwrap().life - 20);
    }
    results.sort();
    assert_eq!(results, vec![3, 4], "CR 616.1e: either order may be chosen ((1+1)*2=4 or 1*2+1=3)");
}

#[test]
fn cr_616_1b_control_changing_entry_replacement_applies_first() {
    let gather = compile(
        "Gather Specimens",
        "Mana cost: {3}{U}\nType: Instant\nIf a creature would enter under an opponent's control this turn, it enters under your control instead.",
    );
    let authority = compile(
        "Authority of the Consuls",
        "Mana cost: {W}\nType: Enchantment\nCreatures your opponents control enter tapped.\nWhenever a creature an opponent controls enters, you gain 1 life.",
    );
    let reanimate = compile("Reanimate", "Mana cost: {0}\nType: Sorcery\nReturn target creature card from your graveyard to the battlefield.");
    let mut game = new_game();
    game.create_object_from_definition(&authority, bob(), Zone::Battlefield);
    cast(&mut game, &gather, alice(), vec![]);
    let card = game.create_object_from_definition(&creature_def("Bob bear", 2, 2, ColorSet::GREEN), bob(), Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    cast(&mut game, &reanimate, bob(), vec![obj(card)]);
    let c = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.current_controller(c), Some(alice()), "fixture: control replaced");
    assert!(
        game.is_tapped(c),
        "CR 616.1b: the control-changing replacement is applied first, so Bob's 'opponents' creatures enter tapped' then applies"
    );
}

struct FixedTargets(Vec<ironsmith::Target>);
impl ironsmith::DecisionMaker for FixedTargets {
    fn decide_targets(&mut self, _g: &GameState, _ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
        self.0.clone()
    }
}

/// Cast a {0} card from hand through native priority actions, then resolve it.
fn native_cast(game: &mut GameState, def: &CardDefinition, player: PlayerId, targets: Vec<ironsmith::Target>) {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let card = game.create_object_from_definition(def, player, Zone::Hand);
    let action = ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, ironsmith::LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .expect("fixture: castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = FixedTargets(targets);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm,
    );
    for _ in 0..16 {
        if game.stack.iter().any(|e| e.object_id != card) || !game.stack.is_empty() && state.pending_cast.is_none() {
            break;
        }
        let ctx = match &result {
            Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(c)) => c.clone(),
            other => panic!("fixture: cast stalled: {other:?}"),
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert!(!game.stack.is_empty(), "fixture: spell on stack");
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
}
// ---------------------------------------------------------------------------
// 614.1b / 614.10 skips, 700, 703 turn-based actions
// ---------------------------------------------------------------------------

struct TurnLog {
    life_at_first_main: Option<i32>,
    pool_green_at_first_main: Option<u32>,
}

/// Drive the active player's whole turn through the native turn runner.
fn drive_turn(
    game: &mut GameState,
    attackers: Vec<ironsmith::AttackerDeclaration>,
    blockers: Vec<ironsmith::BlockerDeclaration>,
    accept_optional_skip: bool,
) -> TurnLog {
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginTurn);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut attackers = Some(attackers);
    let mut blockers = Some(blockers);
    let mut log = TurnLog { life_at_first_main: None, pool_green_at_first_main: None };
    let active = game.turn.active_player;
    for _ in 0..2000 {
        match runner.advance(game, &mut queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if game.turn.phase == ironsmith::Phase::FirstMain && log.life_at_first_main.is_none() {
                    log.life_at_first_main = Some(game.player(active).unwrap().life);
                    log.pool_green_at_first_main = Some(game.player(active).unwrap().mana_pool.green);
                }
                ironsmith::run_priority_loop_with(game, &mut queue, &mut dm).unwrap();
                runner.priority_done();
            }
            TurnAction::Decision(ctx) => match ctx {
                DecisionContext::Boolean(_) => runner.respond_boolean(accept_optional_skip),
                DecisionContext::Attackers(_) => runner.respond_attackers(attackers.take().unwrap_or_default()),
                DecisionContext::Blockers(b) => {
                    let mine: Vec<_> = blockers.take().unwrap_or_default();
                    runner.respond_blockers(mine, b.player)
                }
                DecisionContext::SelectObjects(o) => {
                    runner.respond_discard(o.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(o.min).collect())
                }
                DecisionContext::Order(o) => runner.respond_order(o.items.iter().map(|(id, _)| *id).collect()),
                DecisionContext::SelectOptions(o) => {
                    runner.respond_options(o.options.iter().filter(|x| x.legal).map(|x| x.index).take(o.min.max(1)).collect())
                }
                DecisionContext::Distribute(d) => {
                    let n = d.targets.len() as u32;
                    let mut out = Vec::new();
                    for (i, t) in d.targets.iter().enumerate() {
                        let amount = if i == 0 { d.total - (n - 1).min(d.total) } else if (i as u32) <= d.total { 1 } else { 0 };
                        out.push((t.target.clone(), amount));
                    }
                    runner.respond_distribute(out)
                }
                other => panic!("fixture: unexpected turn decision {other:?}"),
            },
            TurnAction::TurnComplete | TurnAction::GameOver(_) => return log,
        }
    }
    panic!("fixture: turn did not complete");
}

/// Make `player` the active player of a fresh turn.
fn start_turn_of(game: &mut GameState, player: PlayerId) {
    if game.turn.active_player == player {
        game.next_turn();
    }
    while game.turn.active_player != player {
        game.next_turn();
    }
}

#[test]
fn cr_703_1_703_2_703_3_703_4c_4d_4n_4p_turn_based_actions_run_first_and_without_the_stack() {
    let draw_trig = compile("Draw trig", "Mana cost: {0}\nType: Enchantment\nAt the beginning of your draw step, you gain 1 life for each card in your hand.");
    let upkeep_trig = compile("Upkeep trig", "Mana cost: {0}\nType: Enchantment\nAt the beginning of your upkeep, you gain 1 life.");
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    library_cards(&mut game, alice(), 5);
    let hand_card = CardDefinitionBuilder::new(CardId::new(), "Hand card").card_types(vec![CardType::Sorcery]).build();
    for _ in 0..8 {
        game.create_object_from_definition(&hand_card, alice(), Zone::Hand);
    }
    game.create_object_from_definition(&draw_trig, alice(), Zone::Battlefield);
    game.create_object_from_definition(&upkeep_trig, alice(), Zone::Battlefield);
    let rock = CardDefinitionBuilder::new(CardId::new(), "Tapped rock").card_types(vec![CardType::Artifact]).build();
    let rock = game.create_object_from_definition(&rock, alice(), Zone::Battlefield);
    game.tap(rock);
    let bobs = game.create_object_from_definition(&creature_def("Bob tapped", 1, 1, ColorSet::RED), bob(), Zone::Battlefield);
    game.tap(bobs);
    let c = put_creature(&mut game, alice(), "Cleanup subject", 2, 4, ColorSet::GREEN);
    apply(&mut game, c, c, Modification::ModifyPowerToughness { power: 2, toughness: 2 });
    deal(&mut game, bobs, ChooseSpec::SpecificObject(c), 1);
    drive_turn(&mut game, vec![], vec![], false);
    assert!(!game.is_tapped(rock), "CR 703.4c: active player's permanents untap");
    assert!(game.is_tapped(bobs), "CR 703.4c: only the active player's permanents untap");
    assert_eq!(
        game.player(alice()).unwrap().life,
        20 + 1 + 9,
        "CR 703.3: the turn-based draw happens before the draw-step trigger is put on the stack (9 cards in hand when it resolves)"
    );
    assert_eq!(game.player(alice()).unwrap().hand.len(), 7, "CR 703.4n: discard to maximum hand size");
    assert_eq!(game.player(alice()).unwrap().library.len(), 4, "CR 703.4d: exactly one card drawn");
    assert_eq!(game.damage_on(c), 0, "CR 703.4p: damage removed");
    assert_eq!(pt(&game, c), (Some(2), Some(4)), "CR 703.4p: until-end-of-turn effects end");
}

#[test]
fn cr_703_4a_4b_phasing_then_day_night_check_in_untap_step() {
    let mut game = new_game();
    start_turn_of(&mut game, bob());
    game.set_daytime(true);
    let phaser = CardDefinitionBuilder::new(CardId::new(), "Phaser")
        .card_types(vec![CardType::Artifact])
        .with_ability(ironsmith::Ability::static_ability(StaticAbility::phasing()))
        .build();
    let phaser = game.create_object_from_definition(&phaser, alice(), Zone::Battlefield);
    library_cards(&mut game, alice(), 3);
    game.refresh_continuous_state().unwrap();
    // Bob cast no spells in his turn.
    game.next_turn();
    assert_eq!(game.turn.active_player, alice());
    drive_turn(&mut game, vec![], vec![], false);
    assert!(game.is_phased_out(phaser), "CR 703.4a: phasing permanents of the active player phase out");
    assert!(!game.is_daytime(), "CR 703.4b: day becomes night when the previous active player cast no spells");
}

#[test]
fn cr_703_4i_4j_4k_4m_700_1_combat_turn_based_actions() {
    let blockee = compile(
        "Block trig",
        "Mana cost: {0}\nType: Creature — Bear\nPower/Toughness: 5/5\nWhenever this creature becomes blocked, you gain 1 life.\nWhenever this creature becomes blocked by a creature, you gain 10 life.",
    );
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    library_cards(&mut game, alice(), 3);
    library_cards(&mut game, bob(), 3);
    let big = game.create_object_from_definition(&blockee, alice(), Zone::Battlefield);
    let duelist = put_creature(&mut game, alice(), "Duelist", 2, 2, ColorSet::RED);
    let free = put_creature(&mut game, alice(), "Unblocked", 3, 3, ColorSet::RED);
    let b1 = put_creature(&mut game, bob(), "Chump 1", 1, 1, ColorSet::GREEN);
    let b2 = put_creature(&mut game, bob(), "Chump 2", 1, 1, ColorSet::GREEN);
    let b3 = put_creature(&mut game, bob(), "Trader", 2, 2, ColorSet::GREEN);
    for id in [big, duelist, free, b1, b2, b3] {
        game.remove_summoning_sickness(id);
    }
    use ironsmith::combat_state::AttackTarget;
    let atk = |id| ironsmith::AttackerDeclaration { creature: id, target: AttackTarget::Player(bob()) };
    let blk = |b, a| ironsmith::BlockerDeclaration { blocker: b, blocking: a };
    drive_turn(
        &mut game,
        vec![atk(big), atk(duelist), atk(free)],
        vec![blk(b1, big), blk(b2, big), blk(b3, duelist)],
        false,
    );
    assert_eq!(game.player(bob()).unwrap().life, 17, "CR 703.4i/4k/4m: unblocked attacker's damage dealt");
    assert!(
        !game.battlefield.contains(&duelist) && !game.battlefield.contains(&b3),
        "CR 703.4m: combat damage is dealt simultaneously, so both 2/2s die"
    );
    assert!(!game.battlefield.contains(&b1) && !game.battlefield.contains(&b2), "CR 703.4j: blocks were declared");
    assert_eq!(
        game.player(alice()).unwrap().life,
        20 + 1 + 20,
        "CR 700.1: one blocking happening is one event for 'becomes blocked' and two for 'becomes blocked by a creature'"
    );
}

#[test]
fn cr_614_1b_614_10_skip_effects_skip_the_next_occurrence_only() {
    let skipper = compile("Skipper", "Mana cost: {0}\nType: Enchantment\nSkip your draw step.");
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    library_cards(&mut game, alice(), 5);
    game.create_object_from_definition(&skipper, alice(), Zone::Battlefield);
    drive_turn(&mut game, vec![], vec![], false);
    assert_eq!(game.player(alice()).unwrap().hand.len(), 0, "CR 614.1b: 'skip' replaces the draw step with nothing");

    // 614.10: "skips their next draw step" resolving during the current draw step waits for the next one.
    let fatigue = compile("Fatigue", "Mana cost: {1}{U}\nType: Sorcery\nTarget player skips their next draw step.");
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    library_cards(&mut game, alice(), 5);
    library_cards(&mut game, bob(), 5);
    game.turn.phase = ironsmith::Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Draw);
    cast(&mut game, &fatigue, alice(), vec![ironsmith::Target::Player(alice())]);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.next_turn();
    drive_turn(&mut game, vec![], vec![], false); // Bob's turn
    game.next_turn();
    let before = game.player(alice()).unwrap().hand.len();
    drive_turn(&mut game, vec![], vec![], false); // Alice's next turn
    assert_eq!(
        game.player(alice()).unwrap().hand.len(),
        before,
        "CR 614.10: the skip applies to the next draw step that actually begins"
    );
    game.next_turn();
    drive_turn(&mut game, vec![], vec![], false);
    game.next_turn();
    let before = game.player(alice()).unwrap().hand.len();
    drive_turn(&mut game, vec![], vec![], false);
    assert_eq!(game.player(alice()).unwrap().hand.len(), before + 1, "CR 614.10: only one draw step is skipped");
}

#[test]
fn cr_614_10b_action_after_skipping_happens_in_the_next_step() {
    let fasting = compile(
        "Mana Fasting",
        "Mana cost: {W}\nType: Enchantment\nIf you would begin your draw step, you may skip that step instead. If you do, add {G}{G}.",
    );
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    library_cards(&mut game, alice(), 5);
    game.create_object_from_definition(&fasting, alice(), Zone::Battlefield);
    let log = drive_turn(&mut game, vec![], vec![], true);
    assert_eq!(game.player(alice()).unwrap().library.len(), 5, "fixture: the draw step was skipped");
    assert_eq!(
        log.pool_green_at_first_main,
        Some(2),
        "CR 614.10b: the 'if you do' action is the first thing in the next step (main phase), so the mana is still available there"
    );
}

#[test]
fn cr_700_2_modal_spell_resolves_only_the_chosen_mode() {
    let modal = compile("Modal", "Mana cost: {0}\nType: Sorcery\nChoose one —\n• You gain 3 life.\n• You lose 1 life.");
    let mut game = new_game();
    native_cast(&mut game, &modal, alice(), vec![]);
    let life = game.player(alice()).unwrap().life;
    assert!(life == 23 || life == 19, "CR 700.2: exactly one chosen mode resolves (life {life})");
}

// ---------------------------------------------------------------------------
// 704: state-based actions
// ---------------------------------------------------------------------------

fn priority_game() -> GameState {
    let mut game = new_game();
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

#[test]
fn cr_704_1_704_2_704_3_sbas_repeat_before_priority_without_the_stack() {
    let lord = compile(
        "Lord",
        "Mana cost: {0}\nType: Creature — Elf\nPower/Toughness: 1/1\nOther creatures you control get +1/+1.",
    );
    let mut game = priority_game();
    let l = game.create_object_from_definition(&lord, alice(), Zone::Battlefield);
    let zero = put_creature(&mut game, alice(), "Zero", 0, 0, ColorSet::GREEN);
    let src = put_creature(&mut game, bob(), "Pinger", 1, 1, ColorSet::RED);
    assert_eq!(pt(&game, zero), (Some(1), Some(1)));
    deal(&mut game, src, ChooseSpec::SpecificObject(l), 1);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::AutoPassDecisionMaker;
    let progress = ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert!(!game.battlefield.contains(&l), "CR 704.3: lethal damage SBA performed before priority");
    assert!(
        !game.battlefield.contains(&zero),
        "CR 704.3: the check repeats, so the now-0/0 creature also dies before priority"
    );
    assert!(game.stack.is_empty(), "CR 704.1: SBAs don't use the stack");
    assert!(matches!(progress, ironsmith::decision::GameProgress::NeedsDecisionCtx(_)), "a player then gets priority: {progress:?}");
}

#[test]
fn cr_704_1a_state_triggers_use_the_stack() {
    let fish = compile(
        "State trig",
        "Mana cost: {0}\nType: Creature — Fish\nPower/Toughness: 1/1\nWhen you control no Islands, sacrifice this creature.",
    );
    let mut game = priority_game();
    let f = game.create_object_from_definition(&fish, alice(), Zone::Battlefield);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::AutoPassDecisionMaker;
    let _ = ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert!(game.battlefield.contains(&f), "CR 704.1a: not a state-based action");
    assert_eq!(game.stack.len(), 1, "CR 704.1a: the state trigger is put on the stack");
}

#[test]
fn cr_704_5i_zero_loyalty_planeswalker_goes_to_graveyard() {
    let walker = compile("Walker", "Mana cost: {0}\nType: Legendary Planeswalker — Test\nLoyalty: 3\n+1: You gain 1 life.");
    let mut game = new_game();
    let w = game.create_object_from_definition(&walker, alice(), Zone::Battlefield);
    let src = put_creature(&mut game, bob(), "Hitter", 3, 3, ColorSet::RED);
    assert_eq!(game.counter_count(w, ironsmith::CounterType::Loyalty), 3, "fixture: starting loyalty");
    deal(&mut game, src, ChooseSpec::SpecificObject(w), 2);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(game.battlefield.contains(&w));
    deal(&mut game, src, ChooseSpec::SpecificObject(w), 1);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(!game.battlefield.contains(&w), "CR 704.5i");
    assert_eq!(game.player(alice()).unwrap().graveyard.len(), 1);
}

#[test]
fn cr_704_5t_completed_dungeon_is_removed_after_its_last_room_ability() {
    let mut dungeon = CardDefinitionBuilder::new(CardId::new(), "P08 Audit Dungeon")
        .card_types(vec![CardType::Dungeon])
        .build();
    for (room, leads_to) in [("Entry", vec!["Middle"]), ("Middle", vec!["Vault"]), ("Vault", vec![])] {
        dungeon.abilities.push(ironsmith::Ability::triggered(
            ironsmith::triggers::Trigger::dungeon_room(room, leads_to.into_iter().map(String::from).collect()),
            vec![ironsmith::Effect::gain_life(1)],
        ));
    }
    ironsmith::dungeon::register_dungeon_definition(&dungeon).expect("fixture: dungeon registers");
    let mut game = priority_game();
    library_cards(&mut game, alice(), 20);
    library_cards(&mut game, bob(), 20);
    let src = put_creature(&mut game, alice(), "Venturer", 1, 1, ColorSet::WHITE);
    // Other audit modules register dungeons in the same process, so pick this one by name.
    struct PickP08Dungeon;
    impl ironsmith::DecisionMaker for PickP08Dungeon {
        fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
            let legal = || ctx.options.iter().filter(|option| option.legal);
            legal()
                .find(|option| option.description.eq_ignore_ascii_case("P08 Audit Dungeon"))
                .or_else(|| legal().next())
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }
    let mut dm = PickP08Dungeon;
    let mut active_when_last_room_triggered = false;
    let mut completed = false;
    for _ in 0..8 {
        game.stack.push(ironsmith::StackEntry::ability(
            src,
            alice(),
            vec![ironsmith::Effect::venture_into_dungeon_player(ironsmith::PlayerFilter::You)],
        ));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let active_with_room_pending = game.active_dungeon(alice()).is_some() && !queue.entries.is_empty();
        for _ in 0..20 {
            if queue.entries.is_empty() && game.stack.is_empty() {
                break;
            }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        }
        if game.active_dungeon(alice()).is_none() {
            active_when_last_room_triggered = active_with_room_pending;
            completed = true;
            break;
        }
    }
    assert_eq!(game.player(alice()).unwrap().life, 23, "fixture: all three room abilities resolved");
    assert!(completed, "CR 704.5t: the dungeon is removed once the bottommost room's ability has left the stack");
    assert!(active_when_last_room_triggered, "CR 704.5t: the dungeon stays while its last room ability is waiting");
}

#[test]
fn cr_704_5u_sector_choices_without_sculptor_first() {
    struct RecordSectors(Vec<PlayerId>);
    impl ironsmith::DecisionMaker for RecordSectors {
        fn decide_options(&mut self, _g: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
            self.0.push(ctx.player);
            vec![ctx.options.iter().find(|o| o.legal).unwrap().index]
        }
    }
    let mut game = priority_game();
    let sculptor = CardDefinitionBuilder::new(CardId::new(), "Sculptor")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(ironsmith::Ability::static_ability(StaticAbility::space_sculptor()))
        .build();
    let s = game.create_object_from_definition(&sculptor, alice(), Zone::Battlefield);
    let mine = put_creature(&mut game, alice(), "Alice creature", 1, 1, ColorSet::RED);
    let theirs = put_creature(&mut game, bob(), "Bob creature", 1, 1, ColorSet::RED);
    let mut dm = RecordSectors(Vec::new());
    ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut dm).unwrap();
    for id in [s, mine, theirs] {
        assert!(game.sector_designation(id).is_some(), "CR 704.5u: every undesignated creature gets a sector");
    }
    assert_eq!(dm.0.first(), Some(&bob()), "CR 704.5u: players without space sculptor choose first: {:?}", dm.0);
    assert_eq!(dm.0.last(), Some(&alice()));
}

#[test]
fn cr_704_5z_only_newest_role_per_controller_remains() {
    let cursed = compile("Role maker2", "Mana cost: {0}\nType: Sorcery\nCreate a Cursed Role token attached to target creature.");
    let mut game = new_game();
    let c = put_creature(&mut game, bob(), "Role host", 4, 4, ColorSet::GREEN);
    cast(&mut game, &cursed, alice(), vec![obj(c)]);
    cast(&mut game, &bob_cast_guard(&cursed), bob(), vec![obj(c)]);
    cast(&mut game, &cursed, alice(), vec![obj(c)]);
    let roles = |game: &GameState| -> Vec<ObjectId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|id| game.current_has_subtype(*id, ironsmith::Subtype::Role))
            .collect()
    };
    assert_eq!(roles(&game).len(), 3, "fixture: three Roles attached");
    let newest_alice = *roles(&game).iter().max_by_key(|id| game.effect_store.continuous_effects.get_object_timestamp(**id)).unwrap();
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    let left = roles(&game);
    assert_eq!(left.len(), 2, "CR 704.5z: Alice's older Role goes to the graveyard; Bob's Role stays");
    assert!(left.contains(&newest_alice), "CR 704.5z: the most recent Role remains");
}
fn bob_cast_guard(def: &CardDefinition) -> CardDefinition {
    def.clone()
}

#[test]
fn cr_704_5aa_start_your_engines_gives_speed_one() {
    let mut game = new_game();
    let pilot = CardDefinitionBuilder::new(CardId::new(), "Pilot")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(ironsmith::Ability::static_ability(StaticAbility::start_your_engines()))
        .build();
    assert_eq!(game.player_speed(alice()), None);
    game.create_object_from_definition(&pilot, alice(), Zone::Battlefield);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(game.player_speed(alice()), Some(1), "CR 704.5aa");
    assert_eq!(game.player_speed(bob()), None);
}

#[test]
fn cr_704_6a_two_headed_giant_team_at_zero_life_loses() {
    let mut game = GameState::new((0..4).map(|i| format!("P{i}")).collect(), 20);
    let p: Vec<PlayerId> = (0..4).map(PlayerId::from_index).collect();
    game.set_random_seed(810);
    game.enable_two_headed_giant(vec![p[..2].to_vec(), p[2..].to_vec()]).unwrap();
    let src = game.create_object_from_definition(&creature_def("Burner", 1, 1, ColorSet::RED), p[2], Zone::Battlefield);
    let life = game.player(p[0]).unwrap().life;
    deal(&mut game, src, to_player(p[0]), (life - 1) as u32);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(game.player(p[1]).unwrap().is_in_game(), "fixture: team still alive at 1");
    deal(&mut game, src, to_player(p[1]), 1);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(
        !game.player(p[0]).unwrap().is_in_game() && !game.player(p[1]).unwrap().is_in_game(),
        "CR 704.6a: the team with 0 life loses"
    );
    assert!(game.player(p[2]).unwrap().is_in_game());
}

#[test]
fn cr_704_6c_twenty_one_combat_damage_from_one_commander_loses() {
    for combat in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
        start_turn_of(&mut game, alice());
        library_cards(&mut game, alice(), 3);
        library_cards(&mut game, bob(), 3);
        let cmd = put_creature(&mut game, alice(), "Big Commander", 21, 21, ColorSet::RED);
        game.set_as_commander(cmd, alice());
        game.remove_summoning_sickness(cmd);
        if combat {
            use ironsmith::combat_state::AttackTarget;
            drive_turn(&mut game, vec![ironsmith::AttackerDeclaration { creature: cmd, target: AttackTarget::Player(bob()) }], vec![], false);
        } else {
            deal(&mut game, cmd, to_player(bob()), 21);
            ironsmith::apply_state_based_actions(&mut game).unwrap();
        }
        assert_eq!(game.player(bob()).unwrap().life, 19, "fixture: 21 damage dealt");
        assert_eq!(
            game.player(bob()).unwrap().is_in_game(),
            !combat,
            "CR 704.6c: combat={combat}: only 21 combat damage from one commander is lethal"
        );
    }
}

#[test]
fn cr_704_6d_commander_may_return_to_command_zone_at_sba() {
    struct Yes;
    impl ironsmith::DecisionMaker for Yes {
        fn decide_boolean(&mut self, _g: &GameState, _c: &ironsmith::decisions::context::BooleanContext) -> bool {
            true
        }
        fn decide_options(&mut self, _g: &GameState, ctx: &ironsmith::SelectOptionsContext) -> Vec<usize> {
            vec![ctx.options.iter().find(|o| o.legal).unwrap().index]
        }
    }
    struct No;
    impl ironsmith::DecisionMaker for No {}
    for accept in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
        let cmd = put_creature(&mut game, alice(), "Returning Commander", 2, 2, ColorSet::GREEN);
        game.set_as_commander(cmd, alice());
        let destroy = compile("Destroy", "Mana cost: {0}\nType: Instant\nDestroy target creature.");
        let stable = game.object(cmd).unwrap().stable_id;
        cast(&mut game, &destroy, bob(), vec![obj(cmd)]);
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard, "fixture: commander died");
        if accept {
            ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut Yes).unwrap();
        } else {
            ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut No).unwrap();
        }
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(id).unwrap().zone,
            if accept { Zone::Command } else { Zone::Graveyard },
            "CR 704.6d: accept={accept}: owner may put it into the command zone"
        );
    }
}

#[test]
fn cr_704_7_one_replacement_replaces_simultaneous_game_losses() {
    let lich = compile("Lich static", "Mana cost: {0}\nType: Enchantment\nIf you would lose the game, instead you gain 10 life.");
    let mut game = new_game();
    game.create_object_from_definition(&lich, alice(), Zone::Battlefield);
    let src = put_creature(&mut game, bob(), "Burner", 1, 1, ColorSet::RED);
    deal(&mut game, src, to_player(alice()), 20);
    game.stack.push(ironsmith::StackEntry::ability(src, alice(), vec![ironsmith::Effect::draw(1)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut dm).unwrap();
    assert!(game.player(alice()).unwrap().is_in_game(), "CR 704.7: the loss is replaced");
    assert_eq!(game.player(alice()).unwrap().life, 10, "CR 704.7: a single replacement replaces both simultaneous losses (gain 10 once)");
}

// ---------------------------------------------------------------------------
// 705: coin flips
// ---------------------------------------------------------------------------

fn flip_events(game: &mut GameState, effect: ironsmith::effects::FlipCoinEffect, src: ObjectId) -> Vec<ironsmith::events::CoinFlippedEvent> {
    let outcome = effect.execute(game, &mut EffectContext::new_default(src, alice())).unwrap();
    outcome
        .events
        .iter()
        .filter_map(|e| e.downcast::<ironsmith::events::CoinFlippedEvent>().cloned())
        .collect()
}

#[test]
fn cr_705_1_705_2_flips_are_fair_and_only_the_flipper_wins_or_loses() {
    use ironsmith::effects::FlipCoinEffect;
    use ironsmith_core::CoinFace;
    let mut game = new_game();
    let src = put_creature(&mut game, alice(), "Flipper", 1, 1, ColorSet::RED);
    let mut heads = 0;
    let mut wins = 0;
    for _ in 0..400 {
        let events = flip_events(&mut game, FlipCoinEffect::new(ironsmith::PlayerFilter::You), src);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert!(e.call.is_some(), "CR 705.2: the flipper calls heads or tails");
        assert_eq!(e.winner.is_some() as u8 + e.loser.is_some() as u8, 1);
        assert!(e.winner.unwrap_or_else(|| e.loser.unwrap()) == alice(), "CR 705.2: only the flipper wins or loses");
        assert_eq!(e.winner == Some(alice()), Some(e.face) == e.call, "CR 705.2: wins iff the call matches");
        heads += (e.face == CoinFace::Heads) as u32;
        wins += (e.winner == Some(alice())) as u32;
        let face_only = flip_events(&mut game, FlipCoinEffect::face_only(ironsmith::PlayerFilter::You), src);
        assert!(face_only[0].winner.is_none() && face_only[0].loser.is_none(), "CR 705.2: face-only flips have no winner or loser");
    }
    assert!((150..=250).contains(&heads), "CR 705.1: equal likelihood (heads {heads}/400)");
    assert!((150..=250).contains(&wins), "CR 705.1: equal likelihood (wins {wins}/400)");
}

#[test]
fn cr_705_3_stated_results_override_the_actual_flip() {
    use ironsmith::effects::FlipCoinEffect;
    use ironsmith_core::CoinFace;
    let mut game = new_game();
    let src = put_creature(&mut game, alice(), "Flipper", 1, 1, ColorSet::RED);
    for _ in 0..40 {
        let e = flip_events(&mut game, FlipCoinEffect::face_only(ironsmith::PlayerFilter::You).with_forced_face(CoinFace::Tails), src);
        assert_eq!(e[0].face, CoinFace::Tails, "CR 705.3: stated result used");
        let e = flip_events(&mut game, FlipCoinEffect::new(ironsmith::PlayerFilter::You).with_forced_winner(ironsmith::PlayerFilter::You), src);
        assert_eq!(e[0].winner, Some(alice()), "CR 705.3: stated winner used");
    }
}

#[test]
fn cr_614_12c_anchor_word_choice_grants_only_the_chosen_ability() {
    let siege = compile(
        "Outpost Siege",
        "Mana cost: {0}\nType: Enchantment\nAs Outpost Siege enters, choose Khans or Dragons.\n• Khans — At the beginning of your upkeep, exile the top card of your library. Until end of turn, you may play that card.\n• Dragons — Whenever a creature you control leaves the battlefield, Outpost Siege deals 1 damage to any target.",
    );
    let destroy = compile("Destroy", "Mana cost: {0}\nType: Instant\nDestroy target creature.");
    let mut fired = Vec::new();
    for first in [true, false] {
        let mut game = priority_game();
        let spell = game.create_object_from_definition(&siege, alice(), Zone::Stack);
        let stable = game.object(spell).unwrap().stable_id;
        game.stack.push(ironsmith::StackEntry::new(spell, alice()));
        let mut dm = PickReplacement(first);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let s = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.battlefield.contains(&s), "fixture: Siege entered");
        let c = put_creature(&mut game, alice(), "Leaver", 1, 1, ColorSet::RED);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        queue.entries.clear();
        cast(&mut game, &destroy, bob(), vec![obj(c)]);
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let triggered = queue.entries.iter().any(|e| e.source == s);
        let before = game.player(alice()).unwrap().life + game.player(bob()).unwrap().life;
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        let after = game.player(alice()).unwrap().life + game.player(bob()).unwrap().life;
        let creature_damage: u32 = game.battlefield.iter().map(|id| game.damage_on(*id)).sum();
        fired.push((triggered, before - after + creature_damage as i32));
    }
    assert!(
        (fired[0].0 != fired[1].0) && (fired[0].1 + fired[1].1 == 1),
        "CR 614.12c: exactly one of the two anchor choices grants the Dragons leave-the-battlefield ability (fired per choice: {fired:?})"
    );
}

#[test]
fn cr_703_4e_archenemy_sets_scheme_in_motion_before_priority() {
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let scheme = |i: usize| {
        ironsmith::CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), &format!("Scheme {i}"))
                .card_types(vec![CardType::Scheme])
                .build(),
        )
    };
    let mut game = new_game();
    start_turn_of(&mut game, alice());
    game.enable_archenemy(ironsmith::ArchenemyVariant::Default, vec![(alice(), (0..20).map(scheme).collect())])
        .unwrap();
    let top = *game.scheme_deck(alice()).unwrap().last().unwrap();
    assert!(game.face_up_schemes().is_empty());
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMain);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let action = runner.advance(&mut game, &mut queue).unwrap();
    assert!(matches!(action, TurnAction::RunPriority), "fixture: reaches priority");
    assert_eq!(
        game.face_up_schemes(),
        &[top],
        "CR 703.4e: the archenemy sets the top scheme in motion as the main phase begins, before priority"
    );
}

#[test]
fn cr_704_6f_face_up_phenomenon_planeswalks_once_its_trigger_left_the_stack() {
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{PlanarCardKind, StateBasedAction};
    let plane = |name: String, trigger: bool| {
        let mut d = ironsmith::CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), &name).card_types(vec![CardType::Plane]).build(),
        );
        if trigger {
            d.abilities.push(ironsmith::Ability::triggered(
                ironsmith::triggers::Trigger::keyword_action_from_source(KeywordActionKind::EncounterPhenomenon, ironsmith::PlayerFilter::You),
                vec![ironsmith::Effect::gain_life(1)],
            ));
        }
        d
    };
    let deck = |prefix: &str, trig: Option<usize>| -> Vec<_> {
        (0..10).map(|i| (plane(format!("{prefix} {i}"), trig == Some(i)), PlanarCardKind::Plane)).collect()
    };
    let mut game = new_game();
    game.enable_planechase(vec![(alice(), deck("A", Some(8))), (bob(), deck("B", None))]).unwrap();
    let named = |game: &GameState, n: &str| {
        game.planar_deck(alice()).unwrap().iter().copied().find(|id| game.object(*id).unwrap().name.as_ref() as &str == n).unwrap()
    };
    let starting = named(&game, "A 9");
    let phenomenon = named(&game, "A 8");
    let next_plane = named(&game, "A 7");
    game.planechase.as_mut().unwrap().card_kinds.insert(phenomenon, PlanarCardKind::Phenomenon);
    let put_top = |game: &mut GameState, id| {
        let d = game.planechase.as_mut().unwrap().decks.get_mut(&alice()).unwrap();
        d.retain(|x| *x != id);
        d.push(id);
    };
    put_top(&mut game, starting);
    game.reveal_starting_plane().unwrap();
    put_top(&mut game, phenomenon);
    assert_eq!(game.planeswalk(alice(), starting).unwrap(), phenomenon);
    assert!(
        !ironsmith::check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon)),
        "CR 704.6f: not while its encounter trigger is waiting"
    );
    let _ = game.take_pending_trigger_events();
    let mut entry = ironsmith::StackEntry::new(phenomenon, alice());
    entry.is_ability = true;
    game.stack.push(entry);
    assert!(
        !ironsmith::check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon)),
        "CR 704.6f: not while its trigger is on the stack"
    );
    game.stack.clear();
    put_top(&mut game, next_plane);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(game.face_up_planar_objects(), &[next_plane], "CR 704.6f: the planar controller planeswalks");
}
