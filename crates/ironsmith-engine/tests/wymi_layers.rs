//! Behavioral ports from wymi/src/layers.test.ts at
//! 2a5c5360a97312f3453cbd44604f88b5d20c9136. This is a selected behavioral
//! audit, not a complete port of all 261 active source tests. Wymi's intermediate
//! representation is deliberately not reproduced: assertions use real runtime
//! copy, face-down and merge operations, including copying the result again.
//! The full indexed source port lives in ironsmith-compiler-runtime/tests/wymi_complete.rs;
//! fixtures/wymi/README.md records its scope and representation adaptations.
use ironsmith::EffectContext as ExecutionContext;
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::ColorSet;
use ironsmith::effect::OutcomeValue;
use ironsmith::effects::{CreateTokenCopyEffect, EffectExecutor};
use ironsmith::game_state::GameState;
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::types::{CardType, Subtype, Supertype};
use ironsmith::zone::Zone;

fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn creature(
    name: &str,
    power: i32,
    toughness: i32,
    color: ColorSet,
    abilities: Vec<StaticAbility>,
) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(power, toughness));
    for ability in abilities {
        builder = builder.with_ability(Ability::static_ability(ability));
    }
    builder.build()
}
fn put(game: &mut GameState, definition: &CardDefinition, zone: Zone) -> ObjectId {
    game.create_object_from_definition(definition, PlayerId::from_index(0), zone)
}
fn copy(
    game: &mut GameState,
    donor: ObjectId,
    edit: impl FnOnce(&mut CreateTokenCopyEffect),
) -> ObjectId {
    let mut effect = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(donor));
    edit(&mut effect);
    let mut context = ExecutionContext::new_default(donor, PlayerId::from_index(0));
    let OutcomeValue::Objects(ids) = effect
        .execute(game, &mut context)
        .expect("copy resolves")
        .value
    else {
        panic!("copy must produce objects");
    };
    assert_eq!(ids.len(), 1);
    ids[0]
}
fn has(game: &GameState, id: ObjectId, predicate: impl Fn(&StaticAbility) -> bool) -> bool {
    game.current_characteristics(id)
        .unwrap()
        .abilities
        .iter()
        .any(|a| matches!(&a.kind, AbilityKind::Static(s) if predicate(s)))
}
fn assert_nameless(name: &str) {
    // Ironsmith uses a reserved UI label for the rules-level absence of a name.
    assert!(
        name.is_empty() || name == ironsmith::object::FACE_DOWN_DISPLAY_NAME,
        "unexpected name: {name}"
    );
}
fn assert_pt(game: &GameState, id: ObjectId, p: i32, t: i32) {
    assert_eq!(game.calculated_power(id), Some(p));
    assert_eq!(game.calculated_toughness(id), Some(t));
}

// Wymi lines 264–282: mana-derived color, including both halves of hybrid pips.
#[test]
fn wymi_mana_colors() {
    for (name, pips, expected) in [
        (
            "Akoum Warrior",
            vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Red]],
            ColorSet::RED,
        ),
        (
            "Yargle and Multani",
            vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Green],
                vec![ManaSymbol::Green],
            ],
            ColorSet::BLACK.union(ColorSet::GREEN),
        ),
        (
            "Old Ghastbark",
            vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green, ManaSymbol::White],
                vec![ManaSymbol::Green, ManaSymbol::White],
            ],
            ColorSet::GREEN.union(ColorSet::WHITE),
        ),
    ] {
        let mut g = game();
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(pips))
            .build();
        let id = put(&mut g, &def, Zone::Battlefield);
        assert_eq!(g.current_colors(id), Some(expected), "{name}");
    }
}

fn rorix(g: &mut GameState) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rorix Bladewing")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .supertypes(vec![Supertype::Legendary])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .power_toughness(PowerToughness::fixed(6, 5))
        .with_ability(Ability::static_ability(StaticAbility::flying()))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build();
    put(g, &def, Zone::Battlefield)
}
// Wymi 679–891: the copy-exception families, checked a second generation too.
#[test]
fn wymi_plain_copy_preserves_characteristics_and_abilities() {
    let mut g = game();
    let donor = rorix(&mut g);
    let first = copy(&mut g, donor, |_| {});
    let second = copy(&mut g, first, |_| {});
    for id in [first, second] {
        assert_eq!(g.object(id).unwrap().name, "Rorix Bladewing");
        assert_eq!(
            g.object(id).unwrap().mana_cost,
            g.object(donor).unwrap().mana_cost
        );
        assert_eq!(g.current_colors(id), Some(ColorSet::RED));
        assert_pt(&g, id, 6, 5);
        assert!(has(&g, id, |s| s.has_flying()));
        assert!(has(&g, id, |s| s.has_haste()));
    }
}
#[test]
fn wymi_auton_soldier_removes_legendary_and_adds_artifact() {
    let mut g = game();
    let donor = rorix(&mut g);
    let token = copy(&mut g, donor, |e| {
        e.removed_supertypes = vec![Supertype::Legendary];
        e.added_card_types = vec![CardType::Artifact];
    });
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        let obj = g.object(id).unwrap();
        assert!(!obj.supertypes.contains(&Supertype::Legendary));
        assert!(obj.card_types.contains(&CardType::Artifact));
        assert!(obj.card_types.contains(&CardType::Creature));
        assert_pt(&g, id, 6, 5);
    }
    assert!(
        g.object(donor)
            .unwrap()
            .supertypes
            .contains(&Supertype::Legendary)
    );
}
#[test]
fn wymi_vizier_embalm_copy_clears_cost_sets_white_and_adds_zombie() {
    let mut g = game();
    let donor = rorix(&mut g);
    let token = copy(&mut g, donor, |e| {
        e.clear_mana_cost = true;
        e.set_colors = Some(ColorSet::WHITE);
        e.added_subtypes = vec![Subtype::Zombie];
    });
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        assert_eq!(g.object(id).unwrap().mana_cost, None);
        assert_eq!(g.current_colors(id), Some(ColorSet::WHITE));
        assert!(g.object(id).unwrap().subtypes.contains(&Subtype::Dragon));
        assert!(g.object(id).unwrap().subtypes.contains(&Subtype::Zombie));
        assert_pt(&g, id, 6, 5);
    }
}
#[test]
fn wymi_imposter_mech_replaces_types() {
    let mut g = game();
    let donor = rorix(&mut g);
    let token = copy(&mut g, donor, |e| {
        e.set_card_types = Some(vec![CardType::Artifact]);
        e.set_subtypes = Some(vec![Subtype::Vehicle]);
    });
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        assert_eq!(
            g.object(id).unwrap().card_types.as_slice(),
            &[CardType::Artifact]
        );
        assert_eq!(
            g.object(id).unwrap().subtypes.as_slice(),
            &[Subtype::Vehicle]
        );
        assert!(has(&g, id, |s| s.has_flying()));
    }
}
#[test]
fn wymi_sakashima_copy_name_and_legendary_are_copiable() {
    let mut g = game();
    let donor = put(
        &mut g,
        &creature(
            "Banehound",
            1,
            1,
            ColorSet::BLACK,
            vec![StaticAbility::lifelink(), StaticAbility::haste()],
        ),
        Zone::Battlefield,
    );
    let token = copy(&mut g, donor, |e| {
        e.set_name = Some("Sakashima the Impostor".into());
        e.added_supertypes = vec![Supertype::Legendary];
    });
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        assert_eq!(g.object(id).unwrap().name, "Sakashima the Impostor");
        assert!(
            g.object(id)
                .unwrap()
                .supertypes
                .contains(&Supertype::Legendary)
        );
        assert!(has(&g, id, |s| s.has_lifelink()));
        assert_pt(&g, id, 1, 1);
    }
}

fn color_cda_case(colors: ColorSet, replacement: Option<ColorSet>) {
    let mut g = game();
    let donor = put(
        &mut g,
        &creature(
            "Color CDA donor",
            5,
            5,
            ColorSet::COLORLESS,
            vec![
                StaticAbility::set_colors(ObjectFilter::source(), colors),
                StaticAbility::flying(),
            ],
        ),
        Zone::Battlefield,
    );
    let token = copy(&mut g, donor, |e| e.set_colors = replacement);
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        assert_eq!(g.current_colors(id), Some(replacement.unwrap_or(colors)));
        assert_eq!(
            has(&g, id, |s| s.characteristic_defining_colors().is_some()),
            replacement.is_none(),
            "copy must exclude the overridden color CDA"
        );
        assert!(has(&g, id, |s| s.has_flying()));
    }
    assert_eq!(g.current_colors(donor), Some(colors));
}
#[test]
fn wymi_sphinx_color_cda_plain_copy() {
    color_cda_case(
        ColorSet::WHITE
            .union(ColorSet::BLUE)
            .union(ColorSet::BLACK)
            .union(ColorSet::RED)
            .union(ColorSet::GREEN),
        None,
    );
}
#[test]
fn wymi_sphinx_color_cda_white_copy_exception() {
    color_cda_case(
        ColorSet::WHITE
            .union(ColorSet::BLUE)
            .union(ColorSet::BLACK)
            .union(ColorSet::RED)
            .union(ColorSet::GREEN),
        Some(ColorSet::WHITE),
    );
}
#[test]
fn wymi_devoid_plain_copy() {
    color_cda_case(ColorSet::COLORLESS, None);
}
#[test]
fn wymi_devoid_white_copy_exception() {
    color_cda_case(ColorSet::COLORLESS, Some(ColorSet::WHITE));
}
#[test]
fn wymi_vesuvan_keeps_blue_instead_of_color_cda() {
    color_cda_case(ColorSet::COLORLESS, Some(ColorSet::BLUE));
}

fn type_cda_case(replace: bool) {
    let mut g = game();
    let donor = put(
        &mut g,
        &creature(
            "Venomous Changeling",
            1,
            3,
            ColorSet::BLACK,
            vec![StaticAbility::changeling(), StaticAbility::deathtouch()],
        ),
        Zone::Battlefield,
    );
    let token = copy(&mut g, donor, |e| {
        if replace {
            e.set_card_types = Some(vec![CardType::Artifact]);
            e.set_subtypes = Some(vec![Subtype::Vehicle]);
        } else {
            e.added_card_types = vec![CardType::Artifact];
            e.added_subtypes = vec![Subtype::Zombie];
        }
    });
    let second = copy(&mut g, token, |_| {});
    for id in [token, second] {
        assert_eq!(has(&g, id, |s| s.is_changeling()), !replace);
        assert!(has(&g, id, |s| s.has_deathtouch()));
    }
}
#[test]
fn wymi_changeling_survives_additive_type_exception() {
    type_cda_case(false);
}
#[test]
fn wymi_changeling_excluded_by_replacement_type_exception() {
    type_cda_case(true);
}

fn pt_cda_case(exception: Option<(i32, i32)>) {
    let mut g = game();
    let donor = put(
        &mut g,
        &creature(
            "Burrowguard Mentor",
            0,
            0,
            ColorSet::GREEN.union(ColorSet::WHITE),
            vec![
                StaticAbility::trample(),
                StaticAbility::characteristic_defining_pt(
                    ironsmith::effect::Value::Count(ObjectFilter::creature().you_control()),
                    ironsmith::effect::Value::Count(ObjectFilter::creature().you_control()),
                ),
            ],
        ),
        Zone::Battlefield,
    );
    let token = copy(&mut g, donor, |e| e.set_base_power_toughness = exception);
    let second = copy(&mut g, token, |_| {});
    let (p, t) = exception.unwrap_or((3, 3));
    for id in [token, second] {
        assert_pt(&g, id, p, t);
        assert!(has(&g, id, |s| s.has_trample()));
    }
    assert_pt(&g, donor, 3, 3);
}
#[test]
fn wymi_mentor_cda_plain_copy() {
    pt_cda_case(None);
}
#[test]
fn wymi_quicksilver_gargantuan_excludes_pt_cda() {
    pt_cda_case(Some((7, 7)));
}
#[test]
fn wymi_hulking_metamorph_keeps_pt_instead_of_cda() {
    pt_cda_case(Some((7, 7)));
}

// Wymi face-down and mutate cases (510–678, 3183–3422).
#[test]
fn wymi_face_down_hides_printed_characteristics_and_copy_sees_two_two() {
    let mut g = game();
    let donor = rorix(&mut g);
    assert!(g.set_face_down(donor));
    let token = copy(&mut g, donor, |_| {});
    for id in [donor, token] {
        assert_pt(&g, id, 2, 2);
        let snapshot =
            ObjectSnapshot::from_object_with_calculated_characteristics(g.object(id).unwrap(), &g);
        assert_nameless(&snapshot.copiable_values.name);
        assert_eq!(snapshot.copiable_values.mana_cost, None);
        assert!(snapshot.copiable_values.abilities.is_empty());
        assert!(snapshot.copiable_values.subtypes.is_empty());
        assert_eq!(snapshot.copiable_values.colors, ColorSet::COLORLESS);
    }
    assert!(
        !g.is_face_down(token),
        "copy of a face-down permanent is face up"
    );
}
fn mutate_case(top: bool, hidden: bool, extra: bool) {
    let mut g = game();
    let host = put(
        &mut g,
        &creature(
            "Air Elemental",
            4,
            4,
            ColorSet::BLUE,
            vec![StaticAbility::flying()],
        ),
        Zone::Battlefield,
    );
    if hidden {
        assert!(
            g.object_mut(host)
                .unwrap()
                .apply_face_down_cast_overlay_with_disguise_ward(true)
        );
        assert!(g.set_face_down(host));
        g.set_cloaked(host);
    }
    let spell = put(
        &mut g,
        &creature(
            "Cubwarden",
            3,
            5,
            ColorSet::WHITE,
            vec![StaticAbility::lifelink()],
        ),
        Zone::Stack,
    );
    assert_eq!(
        g.merge_mutating_creature_spell(spell, host, top),
        Some(host)
    );
    if extra {
        let spell = put(
            &mut g,
            &creature(
                "Cloudpiercer",
                5,
                4,
                ColorSet::RED,
                vec![StaticAbility::reach()],
            ),
            Zone::Stack,
        );
        assert_eq!(
            g.merge_mutating_creature_spell(spell, host, false),
            Some(host)
        );
    }
    let token = copy(&mut g, host, |_| {});
    for id in [host, token] {
        let (p, t) = if top {
            (3, 5)
        } else if hidden {
            (2, 2)
        } else {
            (4, 4)
        };
        assert_pt(&g, id, p, t);
        assert_eq!(has(&g, id, |s| s.has_flying()), !hidden);
        assert_eq!(has(&g, id, |s| s.has_lifelink()), !hidden || top);
        assert_eq!(has(&g, id, |s| s.ward_cost().is_some()), hidden && !top);
        assert_eq!(has(&g, id, |s| s.has_reach()), extra);
    }
}
#[test]
fn wymi_mutate_under() {
    mutate_case(false, false, false);
}
#[test]
fn wymi_mutate_over() {
    mutate_case(true, false, false);
}
#[test]
fn wymi_mutate_over_then_under() {
    mutate_case(true, false, true);
}
#[test]
fn wymi_mutate_under_face_down() {
    mutate_case(false, true, false);
}
#[test]
fn wymi_mutate_over_face_down() {
    mutate_case(true, true, false);
}
#[test]
fn wymi_mutate_over_face_down_then_under() {
    mutate_case(true, true, true);
}

// The predefined token tests in Wymi assert names; also exercise creation and
// assert the associated native type and executable ability are present.
macro_rules! token_test {
    ($test:ident, $definition:ident, $name:literal, $kind:ident, $subtype:ident) => {
        #[test]
        fn $test() {
            let mut g = game();
            let def = ironsmith::cards::tokens::$definition();
            let mut context =
                ExecutionContext::new_default(g.new_object_id(), PlayerId::from_index(0));
            let OutcomeValue::Objects(ids) = ironsmith::effects::CreateTokenEffect::one(def)
                .execute(&mut g, &mut context)
                .unwrap()
                .value
            else {
                panic!("token creation");
            };
            assert_eq!(ids.len(), 1);
            let token = g.object(ids[0]).unwrap();
            assert_eq!(token.name, $name);
            assert_eq!(token.kind, ironsmith::object::ObjectKind::Token);
            assert!(token.card_types.contains(&CardType::$kind));
            assert!(token.subtypes.contains(&Subtype::$subtype));
            assert!(!token.abilities.is_empty());
        }
    };
}
token_test!(
    wymi_treasure_token,
    treasure_token_definition,
    "Treasure Token",
    Artifact,
    Treasure
);
token_test!(
    wymi_food_token,
    food_token_definition,
    "Food Token",
    Artifact,
    Food
);
token_test!(
    wymi_gold_token,
    gold_token_definition,
    "Gold Token",
    Artifact,
    Gold
);
token_test!(
    wymi_clue_token,
    clue_token_definition,
    "Clue Token",
    Artifact,
    Clue
);
token_test!(
    wymi_blood_token,
    blood_token_definition,
    "Blood Token",
    Artifact,
    Blood
);
token_test!(
    wymi_powerstone_token,
    powerstone_token_definition,
    "Powerstone Token",
    Artifact,
    Powerstone
);
token_test!(
    wymi_incubator_token,
    incubator_token_definition,
    "Incubator Token",
    Artifact,
    Incubator
);
token_test!(
    wymi_map_token,
    map_token_definition,
    "Map Token",
    Artifact,
    Map
);
token_test!(
    wymi_junk_token,
    junk_token_definition,
    "Junk Token",
    Artifact,
    Junk
);
token_test!(
    wymi_lander_token,
    lander_token_definition,
    "Lander Token",
    Artifact,
    Lander
);
token_test!(
    wymi_mutagen_token,
    mutagen_token_definition,
    "Mutagen Token",
    Artifact,
    Mutagen
);
token_test!(
    wymi_shard_token,
    shard_token_definition,
    "Shard Token",
    Enchantment,
    Shard
);

fn become_copy(g: &mut GameState, host: ObjectId, donor: ObjectId) {
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::continuous::{ApplyContinuousEffect, RuntimeModification};
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
        ironsmith::effect::Until::EndOfTurn,
    )
    .execute(
        g,
        &mut ExecutionContext::new_default(host, PlayerId::from_index(0)),
    )
    .unwrap();
}
#[test]
fn wymi_face_down_overlay_applies_after_copy_effect() {
    let mut g = game();
    let donor = rorix(&mut g);
    let host = put(
        &mut g,
        &creature(
            "Air Elemental",
            4,
            4,
            ColorSet::BLUE,
            vec![StaticAbility::flying()],
        ),
        Zone::Battlefield,
    );
    assert!(g.set_face_down(host));
    become_copy(&mut g, host, donor);
    let token = copy(&mut g, host, |_| {});
    for id in [host, token] {
        assert_pt(&g, id, 2, 2);
        let values =
            ObjectSnapshot::from_object_with_calculated_characteristics(g.object(id).unwrap(), &g)
                .copiable_values;
        assert!(values.abilities.is_empty());
        assert_nameless(&values.name);
        assert!(values.mana_cost.is_none());
    }
}
#[test]
fn wymi_runtime_copy_preserves_original_donor_values() {
    let mut g = game();
    let donor = rorix(&mut g);
    let host = put(
        &mut g,
        &creature("Sakashima's Protege", 3, 1, ColorSet::BLUE, vec![]),
        Zone::Battlefield,
    );
    become_copy(&mut g, host, donor);
    assert_pt(&g, host, 6, 5);
    let token = copy(&mut g, host, |_| {});
    assert_eq!(g.object(token).unwrap().name, "Rorix Bladewing");
    assert!(has(&g, token, |s| s.has_flying()));
    assert!(has(&g, token, |s| s.has_haste()));
}
#[test]
fn wymi_copy_ignores_later_layer_bonuses_and_counters() {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    let mut g = game();
    let donor = rorix(&mut g);
    g.object_mut(donor)
        .unwrap()
        .add_counters(ironsmith::object::CounterType::PlusOnePlusOne, 2);
    g.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            donor,
            PlayerId::from_index(0),
            EffectTarget::Specific(donor),
            Modification::ModifyPowerToughness {
                power: 3,
                toughness: 4,
            },
        ));
    let host = put(
        &mut g,
        &creature("Sakashima's Protege", 3, 1, ColorSet::BLUE, vec![]),
        Zone::Battlefield,
    );
    become_copy(&mut g, host, donor);
    let token = copy(&mut g, host, |_| {});
    assert_pt(&g, donor, 11, 11);
    assert_pt(&g, host, 6, 5);
    assert_pt(&g, token, 6, 5);
    assert!(g.object(token).unwrap().counters.is_empty());
}
#[test]
fn wymi_incubator_has_named_back_face() {
    let (front, back) = ironsmith::cards::tokens::incubator_token_definitions();
    assert_eq!(front.card.name, "Incubator Token");
    assert_eq!(back.card.name, "Phyrexian Token");
    let mut g = game();
    let id = put(&mut g, &back, Zone::Battlefield);
    assert_pt(&g, id, 0, 0);
    assert!(
        g.object(id)
            .unwrap()
            .card_types
            .contains(&CardType::Creature)
    );
    assert!(g.object(id).unwrap().subtypes.contains(&Subtype::Phyrexian));
}
#[test]
fn wymi_walker_token() {
    let def = ironsmith::cards::tokens::walker_token_definition();
    let mut g = game();
    let id = put(&mut g, &def, Zone::Battlefield);
    assert_eq!(g.object(id).unwrap().name, "Walker");
    assert_pt(&g, id, 2, 2);
    assert_eq!(g.current_colors(id), Some(ColorSet::BLACK));
    assert!(g.object(id).unwrap().subtypes.contains(&Subtype::Zombie));
}
macro_rules! role_test {
    ($test:ident,$definition:ident,$name:literal) => {
        #[test]
        fn $test() {
            let def = ironsmith::cards::tokens::$definition();
            assert_eq!(def.card.name, $name);
            assert_eq!(def.card.card_types, vec![CardType::Enchantment]);
            assert!(def.card.subtypes.contains(&Subtype::Aura));
            assert!(def.card.subtypes.contains(&Subtype::Role));
            assert!(!def.abilities.is_empty());
        }
    };
}
role_test!(wymi_cursed_role, cursed_role_token_definition, "Cursed");
role_test!(wymi_monster_role, monster_role_token_definition, "Monster");
role_test!(wymi_royal_role, royal_role_token_definition, "Royal");
role_test!(
    wymi_sorcerer_role,
    sorcerer_role_token_definition,
    "Sorcerer"
);
role_test!(wymi_wicked_role, wicked_role_token_definition, "Wicked");
role_test!(
    wymi_young_hero_role,
    young_hero_role_token_definition,
    "Young Hero"
);

#[test]
fn wymi_prototype_cost_color_and_pt_are_copiable() {
    let mut g = game();
    let def = CardDefinitionBuilder::new(CardId::new(), "Hulking Metamorph")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(9)]]))
        .power_toughness(PowerToughness::fixed(7, 7))
        .build();
    let donor = put(&mut g, &def, Zone::Battlefield);
    let ordinary = copy(&mut g, donor, |_| {});
    assert_pt(&g, ordinary, 7, 7);
    assert_eq!(g.current_colors(ordinary), Some(ColorSet::COLORLESS));
    let cost = ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(2)],
        vec![ManaSymbol::Blue],
        vec![ManaSymbol::Blue],
    ]);
    assert!(
        g.object_mut(donor)
            .unwrap()
            .apply_prototype_cast_overlay(cost.clone(), PowerToughness::fixed(3, 3))
    );
    let token = copy(&mut g, donor, |_| {});
    assert_pt(&g, token, 3, 3);
    assert_eq!(g.current_colors(token), Some(ColorSet::BLUE));
    assert_eq!(g.object(token).unwrap().mana_cost.as_deref(), Some(&cost));
    assert!(g.object_mut(donor).unwrap().end_prototype_cast_overlay());
    assert_pt(&g, token, 3, 3);
    assert_pt(&g, donor, 7, 7);
}

fn mutate_cda_case(top: bool, cda: &str) {
    let mut g = game();
    let all_colors = ColorSet::WHITE
        .union(ColorSet::BLUE)
        .union(ColorSet::BLACK)
        .union(ColorSet::RED)
        .union(ColorSet::GREEN);
    let ability = match cda {
        "pt" => StaticAbility::characteristic_defining_pt(
            ironsmith::effect::Value::Count(ObjectFilter::creature().you_control()),
            ironsmith::effect::Value::Count(ObjectFilter::creature().you_control()),
        ),
        "color" => StaticAbility::set_colors(ObjectFilter::source(), all_colors),
        "types" => StaticAbility::changeling(),
        _ => unreachable!(),
    };
    let host = put(
        &mut g,
        &creature("CDA host", 1, 3, ColorSet::BLACK, vec![ability]),
        Zone::Battlefield,
    );
    // Changeling includes Human, so mutating it is not a legal cast target.
    // Wymi directly merges it to test layer composition. Mirror that setup
    // through a non-Human type-setting effect, then test CDA retention itself.
    if cda == "types" {
        use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
        g.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                host,
                PlayerId::from_index(0),
                EffectTarget::Specific(host),
                Modification::SetSubtypes(vec![Subtype::Shapeshifter]),
            ));
    }
    let spell = put(
        &mut g,
        &creature(
            "Cubwarden",
            3,
            5,
            ColorSet::WHITE,
            vec![StaticAbility::lifelink()],
        ),
        Zone::Stack,
    );
    assert_eq!(
        g.merge_mutating_creature_spell(spell, host, top),
        Some(host)
    );
    let token = copy(&mut g, host, |_| {});
    for id in [host, token] {
        assert!(has(&g, id, |s| s.has_lifelink()));
        match cda {
            "pt" => assert_pt(&g, id, 2, 2),
            "color" => assert_eq!(g.current_colors(id), Some(all_colors)),
            "types" => assert!(has(&g, id, |s| s.is_changeling())),
            _ => unreachable!(),
        }
    }
}
#[test]
fn wymi_mutate_over_preserves_pt_cda() {
    mutate_cda_case(true, "pt");
}
#[test]
fn wymi_mutate_under_preserves_pt_cda() {
    mutate_cda_case(false, "pt");
}
#[test]
fn wymi_mutate_over_preserves_color_cda() {
    mutate_cda_case(true, "color");
}
#[test]
fn wymi_mutate_under_preserves_color_cda() {
    mutate_cda_case(false, "color");
}
#[test]
fn wymi_mutate_over_preserves_changeling() {
    mutate_cda_case(true, "types");
}
#[test]
fn wymi_mutate_under_preserves_changeling() {
    mutate_cda_case(false, "types");
}

fn linked_faces(g: &mut GameState, flip: bool) -> ObjectId {
    use ironsmith::card::LinkedFaceLayout;
    let front_id = CardId::new();
    let back_id = CardId::new();
    let (front_name, back_name, color, layout) = if flip {
        (
            "Hired Muscle",
            "Scarmaker",
            ColorSet::BLACK,
            LinkedFaceLayout::Flip,
        )
    } else {
        (
            "Huntmaster of the Fells",
            "Ravager of the Fells",
            ColorSet::RED.union(ColorSet::GREEN),
            LinkedFaceLayout::TransformLike,
        )
    };
    let front = CardDefinitionBuilder::new(front_id, front_name)
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(back_id)
        .other_face_name(back_name)
        .linked_face_layout(layout)
        .build();
    let back = CardDefinitionBuilder::new(back_id, back_name)
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(4, 4))
        .other_face(front_id)
        .other_face_name(front_name)
        .linked_face_layout(layout)
        .build();
    g.register_linked_face_definition(&front);
    g.register_linked_face_definition(&back);
    put(g, &front, Zone::Battlefield)
}
fn linked_copy_case(flip: bool, back: bool, modifier: bool) {
    let mut g = game();
    let donor = linked_faces(&mut g, flip);
    if back {
        if flip {
            assert!(g.flip_permanent(donor));
        } else {
            assert!(g.transform_permanent(donor).unwrap());
        }
    }
    let token = copy(&mut g, donor, |e| {
        if modifier {
            e.set_base_power_toughness = Some((7, 7));
        }
    });
    let expected_name = match (flip, back) {
        (true, _) => "Hired Muscle",
        (false, false) => "Huntmaster of the Fells",
        (false, true) => "Ravager of the Fells",
    };
    assert_eq!(g.object(token).unwrap().name, expected_name);
    let p = if modifier {
        7
    } else if !flip && back {
        4
    } else {
        2
    };
    assert_pt(&g, token, p, p);
    assert!(!g.is_flipped(token));
    if back {
        assert_pt(&g, donor, 4, 4);
    }
}
#[test]
fn wymi_copy_unflipped_card() {
    linked_copy_case(true, false, false);
}
#[test]
fn wymi_copy_flipped_card_copies_unflipped_values() {
    linked_copy_case(true, true, false);
}
#[test]
fn wymi_copy_unflipped_card_with_pt_exception() {
    linked_copy_case(true, false, true);
}
#[test]
fn wymi_copy_flipped_card_with_pt_exception() {
    linked_copy_case(true, true, true);
}
#[test]
fn wymi_copy_transform_front() {
    linked_copy_case(false, false, false);
}
#[test]
fn wymi_copy_transform_back() {
    linked_copy_case(false, true, false);
}
#[test]
fn wymi_copy_transform_front_with_pt_exception() {
    linked_copy_case(false, false, true);
}
#[test]
fn wymi_copy_transform_back_with_pt_exception() {
    linked_copy_case(false, true, true);
}

#[test]
fn wymi_disguise_and_cloak_ward_is_copiable() {
    for cloaked in [false, true] {
        let mut g = game();
        let donor = rorix(&mut g);
        assert!(
            g.object_mut(donor)
                .unwrap()
                .apply_face_down_cast_overlay_with_disguise_ward(true)
        );
        assert!(g.set_face_down(donor));
        if cloaked {
            g.set_cloaked(donor);
        }
        let token = copy(&mut g, donor, |_| {});
        for id in [donor, token] {
            assert_pt(&g, id, 2, 2);
            assert!(has(&g, id, |s| s.ward_cost().is_some()));
            assert!(!has(&g, id, |s| s.has_flying()));
            assert_eq!(g.current_colors(id), Some(ColorSet::COLORLESS));
        }
        assert!(!g.is_face_down(token));
        assert!(!g.is_cloaked(token));
    }
}

#[test]
fn wymi_vesuvan_retained_color_survives_a_second_copy() {
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::continuous::{ApplyContinuousEffect, RuntimeModification};
    let mut g = game();
    let donor = rorix(&mut g);
    let host = put(
        &mut g,
        &creature("Vesuvan Doppelganger", 0, 0, ColorSet::BLUE, vec![]),
        Zone::Battlefield,
    );
    let mut effect = ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(host),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        ironsmith::effect::Until::EndOfTurn,
    );
    effect
        .runtime_modifications
        .push(RuntimeModification::RetainSourceColors);
    effect
        .execute(
            &mut g,
            &mut ExecutionContext::new_default(host, PlayerId::from_index(0)),
        )
        .unwrap();
    assert_eq!(g.current_colors(host), Some(ColorSet::BLUE));
    let token = copy(&mut g, host, |_| {});
    assert_eq!(
        g.current_colors(token),
        Some(ColorSet::BLUE),
        "the color retained by a copy exception is itself copiable"
    );
}
#[test]
fn wymi_machine_gods_effigy_excludes_type_cda_without_subtype_override() {
    let mut g = game();
    let donor = put(
        &mut g,
        &creature(
            "Venomous Changeling",
            1,
            3,
            ColorSet::BLACK,
            vec![StaticAbility::changeling(), StaticAbility::deathtouch()],
        ),
        Zone::Battlefield,
    );
    let token = copy(&mut g, donor, |e| {
        e.set_card_types = Some(vec![CardType::Artifact])
    });
    assert!(!has(&g, token, |s| s.is_changeling()));
    assert!(has(&g, token, |s| s.has_deathtouch()));
}

// Follow-up regressions: verify the state transitions around the audit failures.
#[test]
fn wymi_mutated_face_down_creature_reveals_all_component_abilities() {
    let mut g = game();
    let host = put(
        &mut g,
        &creature("Host", 4, 4, ColorSet::BLUE, vec![StaticAbility::flying()]),
        Zone::Battlefield,
    );
    g.object_mut(host)
        .unwrap()
        .apply_face_down_cast_overlay_with_disguise_ward(true);
    assert!(g.set_face_down(host));
    g.set_cloaked(host);
    let spell = put(
        &mut g,
        &creature(
            "Mutator",
            3,
            5,
            ColorSet::WHITE,
            vec![StaticAbility::lifelink()],
        ),
        Zone::Stack,
    );
    assert_eq!(
        g.merge_mutating_creature_spell(spell, host, false),
        Some(host)
    );
    assert!(!has(&g, host, |s| s.has_lifelink()));
    assert!(!g.set_face_down(host));
    assert!(has(&g, host, |s| s.ward_cost().is_some()));
    assert!(g.set_face_up(host).unwrap());
    for id in [host, copy(&mut g, host, |_| {})] {
        assert_pt(&g, id, 4, 4);
        assert!(has(&g, id, |s| s.has_flying()));
        assert!(has(&g, id, |s| s.has_lifelink()));
        assert!(!has(&g, id, |s| s.ward_cost().is_some()));
    }
    assert!(g.set_face_down(host));
    assert_pt(&g, host, 2, 2);
    assert!(!has(&g, host, |s| s.has_flying() || s.has_lifelink()));
    assert!(g.set_face_up(host).unwrap());
    assert!(has(&g, host, |s| s.has_flying()));
    assert!(has(&g, host, |s| s.has_lifelink()));
}

#[test]
fn wymi_copy_of_flipped_card_can_flip_and_be_copied_again() {
    let mut g = game();
    let donor = linked_faces(&mut g, true);
    assert!(g.flip_permanent(donor));
    let token = copy(&mut g, donor, |_| {});
    assert_eq!(g.object(token).unwrap().name, "Hired Muscle");
    assert!(g.flip_permanent(token));
    assert_eq!(g.object(token).unwrap().name, "Scarmaker");
    assert_pt(&g, token, 4, 4);
    let second = copy(&mut g, token, |_| {});
    assert_eq!(g.object(second).unwrap().name, "Hired Muscle");
    assert_pt(&g, second, 2, 2);
}

fn entry_copy_exception_case(retain_colors: bool) {
    for duration in [None, Some(ironsmith::effect::Until::EndOfTurn)] {
        let mut g = game();
        let alice = PlayerId::from_index(0);
        let donor = put(
            &mut g,
            &creature(
                "CDA donor",
                2,
                2,
                ColorSet::RED,
                vec![
                    StaticAbility::set_colors(ObjectFilter::source(), ColorSet::COLORLESS),
                    StaticAbility::changeling(),
                    StaticAbility::flying(),
                ],
            ),
            Zone::Battlefield,
        );
        let entrant = put(
            &mut g,
            &creature("Copy entrant", 0, 0, ColorSet::BLUE, vec![]),
            Zone::Hand,
        );
        g.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                entrant,
                alice,
                ironsmith::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ironsmith::replacement::ReplacementAction::EnterAsCopy {
                    source: donor,
                    enters_tapped: false,
                    copy_duration: duration,
                    linked_exile_objects: vec![],
                    additional_counters: vec![],
                    name_override: None,
                    added_colors: ColorSet::COLORLESS,
                    added_card_types: if retain_colors {
                        vec![]
                    } else {
                        vec![CardType::Artifact]
                    },
                    removes_other_card_types: !retain_colors,
                    added_supertypes: vec![],
                    removed_supertypes: vec![],
                    added_subtypes: vec![],
                    added_abilities: vec![],
                    set_base_power_toughness: None,
                    copy_followups: if retain_colors {
                        vec![ironsmith_core::EnterAsCopyFollowup::RetainOwnColors]
                    } else {
                        vec![]
                    },
                },
            ),
        );
        let receipt = g
            .move_object_with_etb_processing(entrant, Zone::Battlefield)
            .unwrap();
        assert!(!receipt.pending);
        let ironsmith::events::processing::EventOutcome::Proceed(result) = receipt.original else {
            panic!("entry must complete");
        };
        let recopy = copy(&mut g, result.new_id, |_| {});
        for id in [result.new_id, recopy] {
            assert!(has(&g, id, |s| s.has_flying()));
            assert_eq!(has(&g, id, |s| s.is_changeling()), retain_colors);
            assert_eq!(
                g.current_colors(id),
                Some(if retain_colors {
                    ColorSet::BLUE
                } else {
                    ColorSet::COLORLESS
                })
            );
        }
    }
}

#[test]
fn wymi_entry_copy_retained_color_excludes_cda() {
    entry_copy_exception_case(true);
}

#[test]
fn wymi_entry_copy_type_replacement_excludes_cda() {
    entry_copy_exception_case(false);
}

#[test]
fn wymi_lander_ability_finds_only_basic_lands_and_puts_them_in_tapped() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    struct SearchChooser;
    impl DecisionMaker for SearchChooser {
        fn decide_objects(
            &mut self,
            game: &GameState,
            context: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
    let definition = ironsmith::cards::tokens::lander_token_definition();
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("Lander has an activated ability");
    };
    let costs = ability.mana_cost.costs();
    assert_eq!(costs.len(), 3);
    assert_eq!(
        costs[0].mana_cost_ref(),
        Some(&ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
    );
    assert!(costs[1].requires_tap());
    assert!(costs[2].is_sacrifice_self());
    for with_basic in [true, false] {
        let mut g = game();
        let source = put(&mut g, &definition, Zone::Battlefield);
        let other = CardDefinitionBuilder::new(CardId::new(), "Nonbasic land")
            .card_types(vec![CardType::Land])
            .build();
        let nonbasic = put(&mut g, &other, Zone::Library);
        if with_basic {
            let basic = CardDefinitionBuilder::new(CardId::new(), "Basic land")
                .card_types(vec![CardType::Land])
                .supertypes(vec![Supertype::Basic])
                .build();
            put(&mut g, &basic, Zone::Library);
        }
        let mut dm = SearchChooser;
        let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
        for effect in ability.effects.iter() {
            effect.0.execute(&mut g, &mut ctx).unwrap();
        }
        assert_eq!(g.object(nonbasic).unwrap().zone, Zone::Library);
        let lands: Vec<_> = g
            .battlefield
            .iter()
            .copied()
            .filter(|id| g.object(*id).unwrap().card_types.contains(&CardType::Land))
            .collect();
        assert_eq!(lands.len(), usize::from(with_basic));
        for land in lands {
            assert!(g.is_tapped(land));
            assert!(
                g.object(land)
                    .unwrap()
                    .supertypes
                    .contains(&Supertype::Basic)
            );
        }
    }
}

#[test]
fn wymi_squirrel_token_uses_subtype_derived_name() {
    let def = ironsmith::cards::tokens::squirrel_token_definition();
    assert_eq!(def.card.name, "Squirrel Token");
}
