//! Semantic-loss fix wave (2026-10-21), package T3: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn parse(name: &str, types: Vec<CardType>, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(types)
        .parse_text(text)
        .unwrap()
}

fn creature(name: &str, colors: crate::color::ColorSet, subtypes: Vec<crate::types::Subtype>)
    -> crate::cards::CardDefinition
{
    let mut builder = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .color_indicator(colors);
    if !subtypes.is_empty() {
        builder = builder.subtypes(subtypes);
    }
    builder.build()
}

fn first_activation(definition: &crate::cards::CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.mana_output.is_none() => Some(activated),
            _ => None,
        })
        .expect("non-mana activated ability")
}

fn first_trigger(definition: &crate::cards::CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("triggered ability")
}

/// Run every default effect of `program` and return the objects that newly
/// entered the battlefield.
fn run_program(
    game: &mut crate::game_state::GameState,
    program: &crate::resolution::ResolutionProgram,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    targets: Vec<crate::ids::ObjectId>,
) -> Vec<crate::ids::ObjectId> {
    let before: Vec<_> = game.battlefield.iter().copied().collect();
    let mut ctx = crate::effects::EffectContext::new_default(source, controller).with_targets(
        targets
            .into_iter()
            .map(crate::effects::ResolvedTarget::Object)
            .collect(),
    );
    ctx.snapshot_targets(game);
    for segment in &program.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    }
    game.battlefield
        .iter()
        .copied()
        .filter(|id| !before.contains(id))
        .collect()
}


/// Depth-first search through the common wrapper effects.
fn find_effect<T: 'static>(effect: &Effect) -> Option<&T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return find_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return find_effect(&with_id.effect);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(find_effect);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return may.effects.iter().find_map(find_effect);
    }
    if let Some(each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return each.effects.iter().find_map(find_effect);
    }
    if let Some(each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return each.effects.iter().find_map(find_effect);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return conditional.then.iter().find_map(find_effect);
    }
    None
}

fn find_in_program<T: 'static>(program: &crate::resolution::ResolutionProgram) -> Option<&T> {
    program.all_effects().into_iter().find_map(find_effect)
}

fn new_game() -> (crate::game_state::GameState, crate::ids::PlayerId, crate::ids::PlayerId) {
    let game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    (game, alice, bob)
}

// --- Finding: snow artifact tokens kept their supertype (Svella, Replicating Ring).

#[test]
fn snow_artifact_token_enters_with_the_snow_supertype() {
    let definition = parse(
        "Svella, Ice Shaper",
        vec![CardType::Creature],
        "{3}, {T}: Create a colorless snow artifact token named Icy Manalith with \"{T}: Add one mana of any color.\"",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let created = run_program(&mut game, &first_activation(&definition).effects, source, alice, vec![]);
    assert_eq!(created.len(), 1);
    assert!(game.current_has_supertype(created[0], crate::types::Supertype::Snow));
    assert!(game.current_colors(created[0]).unwrap().is_empty());
}

// --- Finding: "that's all colors" after a comma keyword list (Mechtitan Core).

#[test]
fn appositive_token_after_comma_keyword_list_is_all_colors() {
    let definition = parse(
        "Mechtitan Core",
        vec![CardType::Artifact],
        "{5}: Create Mechtitan, a legendary 10/10 Construct artifact creature token with flying, vigilance, trample, lifelink, and haste that's all colors.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let created = run_program(&mut game, &first_activation(&definition).effects, source, alice, vec![]);
    assert_eq!(created.len(), 1);
    assert_eq!(game.current_colors(created[0]).unwrap().count(), 5);
}

// --- Finding: token keyword vocabulary lacked flanking and training.

#[test]
fn created_knight_token_has_flanking() {
    let definition = parse(
        "Flanking Maker",
        vec![CardType::Sorcery],
        "Create a 2/2 black Knight creature token with flanking, protection from white, and haste.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let created = run_program(&mut game, definition.spell_effect.as_ref().unwrap(), source, alice, vec![]);
    assert_eq!(created.len(), 1);
    assert!(game.current_has_static_ability_id(
        created[0],
        crate::static_abilities::StaticAbilityId::Flanking
    ));
}

#[test]
fn created_soldier_token_has_the_training_trigger() {
    let definition = parse(
        "Training Maker",
        vec![CardType::Sorcery],
        "Create a 1/1 green and white Human Soldier creature token with training.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let created = run_program(&mut game, definition.spell_effect.as_ref().unwrap(), source, alice, vec![]);
    assert_eq!(created.len(), 1);
    let abilities = game.current_abilities(created[0]).unwrap();
    assert!(
        abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::ThisAttacksWithGreaterPowerTrigger>()
                    .is_some()
        )),
        "{abilities:#?}"
    );
}

// --- Finding: "Create two ... tokens for each ..." kept only one per item.

#[test]
fn two_tokens_for_each_item_scale_the_dynamic_count() {
    let text = "Choose a card type. Target opponent reveals their hand. Create two 1/1 green Saproling creature tokens for each card of the chosen type revealed this way.";
    let definition = parse("Fertile Imagination", vec![CardType::Sorcery], text);
    let create = find_in_program::<crate::effects::CreateTokenEffect>(
        definition.spell_effect.as_ref().unwrap(),
    )
    .expect("create token effect");
    assert!(
        matches!(create.count.unhinted(), Value::Scaled(_, 2)),
        "{:?}",
        create.count
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        "You choose a card type. Target opponent reveals their hand. Create two 1/1 green Saproling creature tokens for each card of the chosen type revealed this way."
    );
}

// --- Finding: "in addition to its other colors and types" (added_colors).

#[test]
fn colors_and_types_addition_is_an_added_color_not_a_set_color() {
    let definition = parse(
        "Ratadrabik of Urborg",
        vec![CardType::Creature],
        "Whenever another legendary creature you control dies, create a token that's a copy of that creature, except it's not legendary and it's a 2/2 black Zombie in addition to its other colors and types.",
    );
    let copy = find_in_program::<crate::effects::CreateTokenCopyEffect>(
        &first_trigger(&definition).effects,
    )
    .expect("token copy");
    assert_eq!(copy.added_colors, crate::color::ColorSet::BLACK);
    assert_eq!(copy.set_colors, None, "copied colors must not be replaced");
    assert!(copy.added_subtypes.contains(&crate::types::Subtype::Zombie));
}

#[test]
fn token_copy_with_added_colors_keeps_the_copied_colors() {
    let (mut game, alice, _) = new_game();
    let original = creature(
        "Blue Wizard",
        crate::color::ColorSet::BLUE,
        vec![crate::types::Subtype::Wizard],
    );
    let original = game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let effect = crate::effects::CreateTokenCopyEffect::new(
        ChooseSpec::SpecificObject(original),
        1,
        PlayerFilter::You,
    )
    .added_colors(crate::color::ColorSet::BLACK)
    .added_subtype(crate::types::Subtype::Zombie);
    let mut ctx = crate::effects::EffectContext::new_default(original, alice);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let created = outcome.output_objects();
    assert_eq!(created.len(), 1);
    let colors = game.current_colors(created[0]).unwrap();
    assert!(colors.contains(crate::color::Color::Blue), "copied color kept");
    assert!(colors.contains(crate::color::Color::Black), "added color present");
    assert!(game.current_has_subtype(created[0], crate::types::Subtype::Wizard));
    assert!(game.current_has_subtype(created[0], crate::types::Subtype::Zombie));
}

#[test]
fn jolly_balloon_man_copy_keeps_original_colors_at_runtime() {
    let definition = parse(
        "The Jolly Balloon Man",
        vec![CardType::Creature],
        "{1}, {T}: Create a token that's a copy of another target creature you control, except it's a 1/1 red Balloon creature in addition to its other colors and types and it has flying and haste. Sacrifice it at the beginning of the next end step. Activate only as a sorcery.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let original = creature(
        "Green Elf",
        crate::color::ColorSet::GREEN,
        vec![crate::types::Subtype::Elf],
    );
    let original = game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let created =
        run_program(&mut game, &first_activation(&definition).effects, source, alice, vec![original]);
    assert_eq!(created.len(), 1);
    let colors = game.current_colors(created[0]).unwrap();
    assert!(colors.contains(crate::color::Color::Green));
    assert!(colors.contains(crate::color::Color::Red));
    assert!(game.current_has_subtype(created[0], crate::types::Subtype::Elf));
}

// --- Finding: copy token that's blocking that creature (enters_blocking).

#[test]
fn mirror_match_copy_carries_its_blocking_entry() {
    let definition = parse(
        "Mirror Match",
        vec![CardType::Instant],
        "For each creature attacking you or a planeswalker you control, create a token that's a copy of that creature and that's blocking that creature. Exile those tokens at end of combat.",
    );
    let copy = find_in_program::<crate::effects::CreateTokenCopyEffect>(
        definition.spell_effect.as_ref().unwrap(),
    )
    .expect("token copy");
    assert_eq!(copy.enters_blocking, Some(ChooseSpec::Iterated));
    assert!(
        crate::compiled_text::compiled_text_lines(&definition)
            .join("\n")
            .contains("a copy of that creature and that's blocking that creature")
    );
}

#[test]
fn token_copy_enters_blocking_the_named_attacker() {
    let (mut game, alice, bob) = new_game();
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.active_player = bob;
    let attacker = creature("Raider", crate::color::ColorSet::RED, vec![]);
    let attacker = game.create_object_from_definition(&attacker, bob, Zone::Battlefield);
    game.combat = Some(crate::combat_state::CombatState::default());
    game.combat
        .as_mut()
        .unwrap()
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Player(alice),
        });
    let mut effect = crate::effects::CreateTokenCopyEffect::new(
        ChooseSpec::SpecificObject(attacker),
        1,
        PlayerFilter::You,
    );
    effect.enters_blocking = Some(ChooseSpec::SpecificObject(attacker));
    let mut ctx = crate::effects::EffectContext::new_default(attacker, alice);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let created = outcome.output_objects();
    assert_eq!(created.len(), 1);
    let combat = game.combat.as_ref().unwrap();
    assert_eq!(
        crate::combat_state::get_blockers(combat, attacker),
        &[created[0]]
    );
}

// --- Finding: copy exception keyword grants after "with" (Sauron).

#[test]
fn copy_exception_with_keyword_grants_the_keyword() {
    let definition = parse(
        "Wraith Maker",
        vec![CardType::Sorcery],
        "Create a token that's a copy of target creature you control, except it's a 3/3 black Wraith with menace.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let original = creature("Plain Bear", crate::color::ColorSet::GREEN, vec![]);
    let original = game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let created = run_program(
        &mut game,
        definition.spell_effect.as_ref().unwrap(),
        source,
        alice,
        vec![original],
    );
    assert_eq!(created.len(), 1);
    assert!(game.current_has_static_ability_id(
        created[0],
        crate::static_abilities::StaticAbilityId::Menace
    ));
}

// --- Finding: copy exception entry counter (Littjara Mirrorlake).

#[test]
fn copy_exception_additional_counter_is_an_entry_counter() {
    let text = "{2}{G}{G}{U}, {T}, Sacrifice this land: Create a token that's a copy of target creature you control, except it enters with an additional +1/+1 counter on it. Activate only as a sorcery.";
    let definition = parse("Littjara Mirrorlake", vec![CardType::Land], text);
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let original = creature("Plain Bear", crate::color::ColorSet::GREEN, vec![]);
    let original = game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let created =
        run_program(&mut game, &first_activation(&definition).effects, source, alice, vec![original]);
    assert_eq!(created.len(), 1);
    assert_eq!(
        game.object(created[0])
            .unwrap()
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied(),
        Some(1)
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

// --- Finding: "except those creatures aren't legendary" was dropped.

#[test]
fn mass_become_copy_plural_nonlegendary_exception_is_kept() {
    let definition = parse(
        "Echoing Equation",
        vec![CardType::Sorcery],
        "Choose target creature you control. Each other creature you control becomes a copy of it until end of turn, except those creatures aren't legendary.",
    );
    let (mut game, alice, _) = new_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let legend = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Legend")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .build();
    let legend = game.create_object_from_definition(&legend, alice, Zone::Battlefield);
    let other = creature("Plain Bear", crate::color::ColorSet::GREEN, vec![]);
    let other = game.create_object_from_definition(&other, alice, Zone::Battlefield);
    run_program(
        &mut game,
        definition.spell_effect.as_ref().unwrap(),
        source,
        alice,
        vec![legend],
    );
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.object(other).map(|_| ()), Some(()));
    assert_eq!(game.current_power(other), Some(3), "became a copy");
    assert!(!game.current_has_supertype(other, crate::types::Supertype::Legendary));
}

// --- Finding: "put its +1/+1 counters on ..." parsed a "control" counter.

#[test]
fn referential_typed_counters_keep_their_counter_kind() {
    let definition = parse(
        "Selfless Police Captain",
        vec![CardType::Creature],
        "When this creature leaves the battlefield, put its +1/+1 counters on target creature you control.",
    );
    let triggered = first_trigger(&definition);
    let put = find_in_program::<crate::effects::PutCountersEffect>(&triggered.effects)
        .expect("put counters");
    assert_eq!(put.counter_type, crate::object::CounterType::PlusOnePlusOne);
    assert!(matches!(
        put.amount.unhinted(),
        Value::CountersOn(_, Some(crate::object::CounterType::PlusOnePlusOne))
    ));
}

// --- Finding: countered-spell trigger for any player claimed "cast".

#[test]
fn any_player_spell_countered_trigger_does_not_require_a_cast_spell() {
    let trigger = crate::triggers::Trigger::spell_countered(None, PlayerFilter::Any);
    assert_eq!(trigger.display(), "Whenever a spell is countered");
}
