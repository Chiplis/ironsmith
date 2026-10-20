//! Claude package 52 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::CardType;
use ironsmith::{GameState, PlayerId, Zone};

fn p52_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

/// Emergent Ultimatum: "the other cards" is the exiled collection minus the
/// opponent's pick (which was shuffled away), bound by the shared "the rest"
/// complement; any number of them are then cast without paying mana costs.
#[test]
fn p52_emergent_ultimatum_casts_the_other_exiled_cards_for_free() {
    let definition = p52_compile(
        "Emergent Ultimatum",
        "Mana cost: {B}{B}{G}{G}{G}{U}{U}\nType: Sorcery\nSearch your library for up to three monocolored cards with different names and exile them. An opponent chooses one of those cards. Shuffle that card into your library. You may cast the other cards without paying their mana costs. Exile Emergent Ultimatum.",
    );
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("TagMatchingObjectsEffect"), "the remainder is tagged: {debug}");
    assert!(debug.contains("TagKey(\"rest\")"), "the remainder uses the complement tag: {debug}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    // The remainder producer reads the exiled collection still in exile.
    let rest_at = debug.find("TagMatchingObjectsEffect").unwrap();
    assert!(debug[rest_at..].contains("Exile"), "{debug}");
    let cast_segment = &definition.spell_effect.as_ref().unwrap().segments[3];
    let producer = cast_segment.default_effects[0]
        .downcast_ref::<ironsmith::effects::TagMatchingObjectsEffect>()
        .expect("capture the unchosen exile objects");
    let choose = cast_segment.default_effects[1]
        .downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()
        .expect("choose spells from that exact captured collection");
    assert_eq!(producer.zone, Some(Zone::Exile));
    let pick = definition.spell_effect.as_ref().unwrap().segments[1].default_effects[1]
        .downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()
        .expect("opponent's selected card");
    assert_eq!(producer.excluded_stable_card_tags.as_slice(), &[pick.tag.clone()],
        "the opponent's pick is excluded even if its move is redirected");
    assert_eq!(choose.zone, Some(Zone::Exile));
    assert_eq!(choose.filter.zone, Some(Zone::Exile));
    assert!(choose.additional_zones.is_empty());
    assert!(choose.filter.tagged_constraints.iter().any(|constraint|
        constraint.relation == ironsmith::filter::TaggedOpbjectRelation::SameObjectId),
        "the consumer must exclude a card that leaves and returns as a new object");
}

/// Enthralling Hold: the untapped-creature restriction filters target
/// announcement for the spell only; the attach target itself (rechecked on
/// resolution) still allows untapped creatures.
#[test]
fn p52_enthralling_hold_restricts_only_cast_time_target_choice() {
    let definition = p52_compile(
        "Enthralling Hold",
        "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant creature\nYou can't choose an untapped creature as this spell's target as you cast it.\nYou control enchanted creature.",
    );
    let statics = format!("{:?}", definition.abilities);
    assert!(statics.contains("CastTargetRestriction"), "{statics}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Restriction witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let tapped = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let untapped = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    game.tap(tapped);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let program = definition.spell_effect.as_ref().expect("Aura spell attach program");
    let announced = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game, program, alice, Some(spell), None,
    );
    assert_eq!(announced.len(), 1);
    assert!(announced[0].legal_targets.contains(&ironsmith::Target::Object(tapped)));
    assert!(!announced[0].legal_targets.contains(&ironsmith::Target::Object(untapped)),
        "an untapped creature can't be chosen as you cast it");
    // Without the casting spell as the source, the target specification
    // itself accepts the untapped creature: the recheck stays legal.
    let unrestricted = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game, program, alice, None, None,
    );
    assert!(unrestricted[0].legal_targets.contains(&ironsmith::Target::Object(untapped)));
}

/// Erithizon: the defending player announces the counter's target through
/// the delegated TargetOnly chooser; the counters follow that target's tag.
#[test]
fn p52_erithizon_defending_player_chooses_the_counter_target() {
    let definition = p52_compile(
        "Erithizon",
        "Mana cost: {2}{G}{G}\nType: Creature — Beast\nPower/Toughness: 4/4\nWhenever this creature attacks, put a +1/+1 counter on target creature of defending player's choice.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("chooser: Some(Defending)"), "defending player chooses: {debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("chooser_declared_target"), "counters go on the declared target: {debug}");
}

/// Esix, Fractal Bloom: the first-time window is limited to your turns, and
/// the chosen-creature copy template registers one replacement alternative per
/// eligible creature (never Esix itself), all sharing one application identity.
#[test]
fn p52_esix_offers_one_copy_alternative_per_other_creature_on_your_turns() {
    let definition = p52_compile(
        "Esix, Fractal Bloom",
        "Mana cost: {4}{G}{U}\nType: Legendary Creature — Fractal\nPower/Toughness: 4/4\nFlying\nThe first time you would create one or more tokens during each of your turns, you may instead choose a creature other than Esix and create that many tokens that are copies of that creature.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TokenCreationTemplates"), "{debug}");
    assert!(debug.contains("YourTurn"), "only on your turns: {debug}");
    assert!(debug.contains("TokensCreated"), "only the first time: {debug}");
    assert!(debug.contains("CreateTokenCopyEffect"), "{debug}");
    assert!(debug.contains("optional: true"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let esix = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Copy candidate")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let own = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let theirs = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.update_replacement_effects().unwrap();
    let chosen: Vec<_> = game
        .effect_store
        .replacement_effects
        .effects()
        .iter()
        .filter(|effect| effect.source == esix)
        .filter_map(|effect| match &effect.replacement {
            ironsmith::replacement::ReplacementAction::TokenCreationTemplates { templates, .. } => {
                templates.iter().find_map(|template| {
                    let copy = template.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>()?;
                    match copy.target.base() {
                        ironsmith::target::ChooseSpec::SpecificObject(id) => Some(*id),
                        _ => None,
                    }
                })
            }
            _ => None,
        })
        .collect();
    assert_eq!(chosen.len(), 2, "one alternative per other creature: {chosen:?}");
    assert!(chosen.contains(&own) && chosen.contains(&theirs));
    assert!(!chosen.contains(&esix), "Esix is never a candidate");
    let keys: std::collections::HashSet<_> = game
        .effect_store
        .replacement_effects
        .effects()
        .iter()
        .filter(|effect| effect.source == esix)
        .map(|effect| format!("{:?}", effect.application_key()))
        .collect();
    assert_eq!(keys.len(), 1, "alternatives and decline share one CR 614.5 opportunity");
}

/// Ertai's Meddling: the target spell's choices are kept with its card, it is
/// exiled with X delay counters, and a recurring upkeep trigger of that
/// spell's controller (live while the card stays exiled) removes a counter
/// and then puts the card back as a copy of the original spell.
#[test]
fn p52_ertais_meddling_retains_the_spell_and_schedules_its_return() {
    let definition = p52_compile(
        "Ertai's Meddling",
        "Mana cost: {X}{U}\nType: Instant\nX can't be 0.\nTarget spell's controller exiles it with X delay counters on it.\nAt the beginning of each of that player's upkeeps, if that card is exiled, remove a delay counter from it. If the card has no delay counters on it, the player puts it onto the stack as a copy of the original spell.",
    );
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("RetainSpellChoicesEffect"), "{debug}");
    assert!(debug.contains("ExileEffect") || debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("one_shot: false"), "recurring each upkeep: {debug}");
    assert!(debug.contains("TaggedPlayer"), "that player's upkeeps: {debug}");
    assert!(debug.contains("RemoveCountersEffect"), "{debug}");
    assert!(debug.contains("PutRetainedSpellCopyOntoStackEffect"), "{debug}");
}

/// The retained choices come back with the card: same targets and X, under
/// the original spell's controller, and the card is on the stack again.
#[test]
fn p52_retained_spell_returns_to_the_stack_with_the_original_choices() {
    use ironsmith::effects::{PutRetainedSpellCopyOntoStackEffect, RetainSpellChoicesEffect};
    use ironsmith::target::ChooseSpec;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let instant = CardBuilder::new(CardId::new(), "Retained spell witness")
        .card_types(vec![CardType::Instant])
        .build();
    let creature = CardBuilder::new(CardId::new(), "Retained target witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let spell = game.create_object_from_card(&instant, bob, Zone::Stack);
    let mut entry = ironsmith::game_state::StackEntry::new(spell, bob);
    entry.targets = vec![ironsmith::Target::Object(target)];
    entry.x_value = Some(3);
    game.stack.push(entry);
    let source = game.create_object_from_card(&instant, alice, Zone::Stack);

    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(
        &mut game,
        &Effect::new(RetainSpellChoicesEffect::new(ChooseSpec::SpecificObject(spell), "that_player")),
        &mut ctx,
    )
    .unwrap();
    drop(ctx);
    game.stack.retain(|entry| entry.object_id != spell);
    let exiled = game.move_object_by_effect(spell, Zone::Exile).expect("exiled");

    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(
        &mut game,
        &Effect::new(PutRetainedSpellCopyOntoStackEffect::new(ChooseSpec::SpecificObject(exiled))),
        &mut ctx,
    )
    .unwrap();
    drop(ctx);
    let top = game.stack.last().expect("the copy is on the stack");
    assert_eq!(top.controller, bob, "the original spell's controller puts it there");
    assert_eq!(top.targets, vec![ironsmith::Target::Object(target)]);
    assert_eq!(top.x_value, Some(3));
    assert!(!top.is_ability);
    assert_eq!(game.object(top.object_id).map(|object| object.zone), Some(Zone::Stack));
    let stable = game.object(top.object_id).unwrap().stable_id;
    assert!(game.retained_spell_choices(stable).is_none(), "the retained choices are consumed");
}
