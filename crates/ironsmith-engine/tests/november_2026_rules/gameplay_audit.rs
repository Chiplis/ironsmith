use super::{A, game};
use ironsmith::continuous::{ContinuousEffect, Modification};
use ironsmith::effect::Until;
use ironsmith::{GameState, ObjectId, Zone};
use ironsmith_core::{Color, Subtype, TextChange};

fn put(game: &mut GameState, name: &str, text: &str, zone: Zone) -> ObjectId {
    let definition =
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, A, zone)
}

fn change(game: &mut GameState, id: ObjectId, change: TextChange) {
    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::from_resolution(id, A, vec![id], Modification::RewriteText(change))
            .until(Until::Forever),
    );
    game.refresh_continuous_state().unwrap();
}

#[test]
fn zookeeper_counts_each_affected_creatures_own_types_after_a_text_change() {
    let mut game = game();
    let keeper = put(
        &mut game,
        "Diligent Zookeeper",
        "Type: Creature — Human Ranger\nPower/Toughness: 2/3\nEach non-Human creature you control gets +1/+1 for each of its creature types, to a maximum of 10.",
        Zone::Battlefield,
    );
    let scout = put(
        &mut game,
        "Scout",
        "Type: Creature — Elf Scout\nPower/Toughness: 1/1",
        Zone::Battlefield,
    );
    let ranger = put(
        &mut game,
        "Ranger",
        "Type: Creature — Human Ranger\nPower/Toughness: 1/1",
        Zone::Battlefield,
    );
    let wolf = put(
        &mut game,
        "Wolf",
        "Type: Creature — Wolf\nPower/Toughness: 2/2",
        Zone::Battlefield,
    );
    assert_eq!(
        (game.current_power(scout), game.current_power(ranger)),
        (Some(3), Some(1))
    );
    change(
        &mut game,
        keeper,
        TextChange::creature_type(Subtype::Human, Subtype::Elf).unwrap(),
    );
    assert_eq!(
        game.current_subtypes(keeper).unwrap(),
        [Subtype::Elf, Subtype::Ranger]
    );
    assert_eq!(
        (game.current_power(keeper), game.current_toughness(keeper)),
        (Some(2), Some(3))
    );
    assert_eq!(
        (
            game.current_power(scout),
            game.current_power(ranger),
            game.current_power(wolf)
        ),
        (Some(1), Some(3), Some(3))
    );
}

#[test]
fn high_tide_registers_a_delayed_trigger_for_the_rewritten_land_type() {
    let mut game = game();
    let spell = put(
        &mut game,
        "High Tide",
        "Mana cost: {U}\nType: Instant\nUntil end of turn, whenever a player taps an Island for mana, that player adds an additional {U}.",
        Zone::Stack,
    );
    game.try_push_to_stack(ironsmith::StackEntry::new(spell, A))
        .unwrap();
    change(
        &mut game,
        spell,
        TextChange::basic_land_type(Subtype::Island, Subtype::Forest).unwrap(),
    );
    ironsmith::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    let delayed = game.effect_store.delayed_triggers.last().unwrap();
    let matcher = delayed
        .trigger
        .downcast_ref::<ironsmith::triggers::TapForManaTrigger>()
        .unwrap();
    assert_eq!(matcher.filter.subtypes, [Subtype::Forest]);
}

#[test]
fn chatterfang_creates_rewritten_tokens_alongside_the_original_tokens() {
    let mut game = game();
    let provider = put(
        &mut game,
        "Chatterfang",
        "Type: Creature — Squirrel Warrior\nPower/Toughness: 3/3\nIf one or more tokens would be created under your control, those tokens plus that many 1/1 green Squirrel creature tokens are created instead.",
        Zone::Battlefield,
    );
    change(
        &mut game,
        provider,
        TextChange::creature_type(Subtype::Squirrel, Subtype::Elf).unwrap(),
    );
    change(
        &mut game,
        provider,
        TextChange::color(Color::Green, Color::Blue).unwrap(),
    );
    let spell = put(
        &mut game,
        "Raise the Alarm",
        "Mana cost: {1}{W}\nType: Instant\nCreate two 1/1 white Soldier creature tokens.",
        Zone::Stack,
    );
    game.try_push_to_stack(ironsmith::StackEntry::new(spell, A))
        .unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    let mut elves = 0;
    let mut soldiers = 0;
    for &id in &game.battlefield {
        if id == provider {
            continue;
        }
        let chars = game.calculated_characteristics(id).unwrap();
        if chars.subtypes.contains(&Subtype::Elf) {
            assert_eq!(chars.name.as_ref(), "Elf Token");
            assert_eq!(chars.colors, ironsmith_core::ColorSet::BLUE);
            elves += 1;
        } else {
            assert!(chars.subtypes.contains(&Subtype::Soldier));
            soldiers += 1;
        }
    }
    assert_eq!((elves, soldiers), (2, 2));
}

#[test]
fn false_dawn_rewrites_the_mana_produced_and_the_permission_to_spend_it() {
    use ironsmith::mana::ManaSymbol;
    use ironsmith::replacement::ReplacementAction;
    use ironsmith_core::ManaRewriteOutput;

    let mut game = game();
    put(&mut game, "Forest", "Type: Basic Land — Forest", Zone::Library);
    let spell = put(
        &mut game,
        "False Dawn",
        "Mana cost: {1}{W}\nType: Sorcery\nUntil end of turn, spells and abilities you control that would add colored mana instead add that much white mana. Until end of turn, you may spend white mana as though it were mana of any color.\nDraw a card.",
        Zone::Stack,
    );
    game.try_push_to_stack(ironsmith::StackEntry::new(spell, A))
        .unwrap();
    change(
        &mut game,
        spell,
        TextChange::color(Color::White, Color::Green).unwrap(),
    );
    ironsmith::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|effect| matches!(
                effect.replacement,
                ReplacementAction::RewriteMana {
                    output: ManaRewriteOutput::Symbol(ManaSymbol::Green),
                    ..
                }
            ))
    );
    assert_eq!(game.player(A).unwrap().hand.len(), 1);
    let permissions = &game.effect_store.mana_spend_effects.permissions;
    assert_eq!(permissions.len(), 1);
    assert_eq!(
        permissions[0].permission.any_color_mana_symbol,
        Some(ManaSymbol::Green)
    );
}
