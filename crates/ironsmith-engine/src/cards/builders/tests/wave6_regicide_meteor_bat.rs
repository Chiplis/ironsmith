#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const REGICIDE_ORACLE: &str = "Reveal this card as you draft it. The player to your right chooses a color, you choose another color, then the player to your left chooses a third color.\nDestroy target creature that's one or more of the colors chosen as you drafted cards named Regicide.";
const METEOR_ORACLE: &str =
    "{T}: Choose a color of a permanent you control. Add one mana of that color.";
const BAT_ORACLE: &str = "When this enchantment enters, create a 1/1 black Bat creature token with flying for each mana from a Cave spent to cast it.\nWhenever a Cave you control enters, put a +1/+1 counter on target creature you control.";

fn colored_creature(name: &str, color: crate::color::Color) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(crate::color::ColorSet::from(color))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn find_destroy_target(definition: &CardDefinition) -> ChooseSpec {
    fn find(effect: &crate::Effect) -> Option<ChooseSpec> {
        if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
            return Some(destroy.spec.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find(child);
            }
        });
        found
    }

    definition
        .spell_effect
        .as_ref()
        .expect("Regicide spell effect")
        .flattened_default_effects()
        .iter()
        .find_map(find)
        .expect("Regicide should retain a destroy target")
}

fn find_create_token(effect: &crate::Effect) -> Option<crate::effects::CreateTokenEffect> {
    if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        return Some(create.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_create_token(child);
        }
    });
    found
}

#[test]
fn regicide_uses_the_named_players_drafted_colors_for_target_legality() {
    let definition = parse_oracle_card_definition("Regicide");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        REGICIDE_ORACLE
    );

    let spec = find_destroy_target(&definition);
    if !spec.is_target() {
        panic!("Regicide should target a typed drafted-color filter: {spec:#?}");
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        panic!("Regicide target should be an object filter: {spec:#?}");
    };
    assert!(
        filter
            .colors_chosen_while_drafting_named
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case("Regicide")),
        "{filter:#?}"
    );
    assert!(filter.name.is_none(), "{filter:#?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_draft_chosen_colors(
        alice,
        "  REGICIDE ",
        crate::color::ColorSet::from(crate::color::Color::Red)
            .union(crate::color::ColorSet::from(crate::color::Color::Green)),
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let red = game.create_object_from_definition(
        &colored_creature("Red Creature", crate::color::Color::Red),
        bob,
        Zone::Battlefield,
    );
    let blue = game.create_object_from_definition(
        &colored_creature("Blue Creature", crate::color::Color::Blue),
        bob,
        Zone::Battlefield,
    );

    let legal = crate::game_loop::compute_legal_targets(&game, &spec, alice, Some(source));
    assert!(
        legal.contains(&crate::game_state::Target::Object(red)),
        "{legal:#?}"
    );
    assert!(
        !legal.contains(&crate::game_state::Target::Object(blue)),
        "{legal:#?}"
    );
}

#[test]
fn meteor_crater_chooses_a_color_from_controlled_permanents() {
    let definition = parse_oracle_card_definition("Meteor Crater");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        METEOR_ORACLE
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Meteor Crater activated ability");
    let [effect] = activated.effects.flattened_default_effects() else {
        panic!("Meteor should lower to one atomic restricted color choice: {activated:#?}");
    };
    let add = effect
        .downcast_ref::<crate::effects::AddOneManaOfAnyColorAmongEffect>()
        .expect("Meteor should use the executable colors-among effect");
    assert!(add.choose_color_of_object_surface);
    assert_eq!(add.filter.controller, Some(PlayerFilter::You));
    assert_eq!(add.filter.card_types, ObjectFilter::permanent().card_types);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(
        &colored_creature("Alice Red", crate::color::Color::Red),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &colored_creature("Bob Blue", crate::color::Color::Blue),
        bob,
        Zone::Battlefield,
    );
    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Meteor mana ability should resolve");
    let pool = &game.player(alice).expect("Alice").mana_pool;
    assert_eq!(pool.red, 1);
    assert_eq!(pool.white + pool.blue + pool.black + pool.green, 0);
}

#[test]
fn bat_colony_counts_cave_mana_sources_not_battlefield_caves() {
    let definition = parse_oracle_card_definition("Bat Colony");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), BAT_ORACLE);

    let enters = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| find_create_token(effect).is_some()) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Bat Colony ETB trigger");
    let create = enters
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(find_create_token)
        .expect("Bat Colony token creation");
    let Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        reference,
        ..
    } = create.count.unhinted()
    else {
        panic!("Bat Colony should retain Cave mana provenance: {create:#?}");
    };
    assert_eq!(source_filter.subtypes, [Subtype::Cave]);
    assert_eq!(
        *reference,
        ironsmith_core::ManaSpentCastReferenceSurface::It
    );

    let alice = PlayerId::from_index(0);
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let cave = CardDefinitionBuilder::new(CardId::new(), "Test Cave")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Cave])
        .build();
    let cave_id = game.create_object_from_definition(&cave, alice, Zone::Graveyard);
    let cave_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(cave_id).expect("Cave exists"),
        &game,
    );
    game.object_mut(source)
        .expect("Bat Colony exists")
        .cast_tagged_objects
        .insert(
            crate::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG),
            vec![cave_snapshot.clone(), cave_snapshot],
        );

    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &enters.effects,
        None,
        &[],
    )
    .expect("Bat Colony ETB should create tokens");
    let bats = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| {
            object.subtypes.contains(&Subtype::Bat)
                && object.kind == crate::object::ObjectKind::Token
        })
        .count();
    assert_eq!(bats, 2);
}
