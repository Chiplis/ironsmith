//! Claude package 23 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

const P23_TAWNOS_COFFIN: &str = "Mana cost: {4}\nType: Artifact\nYou may choose not to untap this artifact during your untap step.\n{3}, {T}: Exile target creature and all Auras attached to it. Note the number and kind of counters that were on that creature. When this artifact leaves the battlefield or becomes untapped, return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that permanent.";

fn p23_resolve_triggers(game: &mut ironsmith::GameState, entries: Vec<ironsmith::triggers::TriggeredAbilityEntry>) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(game).unwrap(); }
}

#[test]
fn tawnos_coffin_returns_the_host_tapped_with_its_noted_counters_and_reattaches_its_auras() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::object::{AttachmentTarget, CounterType};
    let definition = compile_to_runtime_definition("Tawnos's Coffin", P23_TAWNOS_COFFIN, false)
        .expect("noted counters and the attached-Aura return should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("coffin activation");
    let debug = format!("{:#?}", activated.effects);
    assert!(debug.contains("NoteCountersEffect"), "the note records counters: {debug}");
    assert!(debug.contains("enters_with_noted_counters: Some("), "the return reads them back: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let coffin = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bear = CardBuilder::new(CardId::new(), "Coffin bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let charm = compile_to_runtime_definition("Coffin charm",
        "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
    let creature = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&charm, bob, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(creature)));
    game.add_counters(creature, CounterType::PlusOnePlusOne, 2);
    let creature_stable = game.object(creature).unwrap().stable_id;
    let aura_stable = game.object(aura).unwrap().stable_id;

    let mut ctx = EffectContext::new_default(coffin, alice)
        .with_targets(vec![ResolvedTarget::Object(creature)]);
    for effect in activated.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let exiled = game.find_object_by_stable_id(creature_stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    let exiled_aura = game.find_object_by_stable_id(aura_stable).unwrap();
    assert_eq!(game.object(exiled_aura).unwrap().zone, Zone::Exile, "the Aura is exiled with its host");
    assert_eq!(game.counter_count(exiled, CounterType::PlusOnePlusOne), 0);

    let untap = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::PermanentUntappedEvent::new(coffin), Default::default());
    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &untap);
    assert_eq!(entries.len(), 1, "the coffin untapping returns the noted card");
    p23_resolve_triggers(&mut game, entries);

    let returned = game.find_object_by_stable_id(creature_stable).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield, "the host, not its Aura, returns");
    assert!(game.is_tapped(returned), "it returns tapped");
    assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 2,
        "it returns with the noted number and kind of counters");
    let returned_aura = game.find_object_by_stable_id(aura_stable).unwrap();
    assert_eq!(game.object(returned_aura).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(returned_aura).unwrap().attached_to, Some(AttachmentTarget::Object(returned)),
        "the other exiled card returns attached to that permanent");
}

const P23_SOUL_ECHO: &str = "Mana cost: {X}{W}{W}\nType: Enchantment\nThis enchantment enters with X echo counters on it.\nYou don't lose the game for having 0 or less life.\nAt the beginning of your upkeep, sacrifice this enchantment if there are no echo counters on it. Otherwise, target opponent may choose that for each 1 damage that would be dealt to you until your next upkeep, you remove an echo counter from this enchantment instead.";

#[test]
fn soul_echo_opponent_elects_damage_to_you_removing_echo_counters_instead() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::object::CounterType;
    let definition = compile_to_runtime_definition("Soul Echo", P23_SOUL_ECHO, false)
        .expect("the opponent-elected damage replacement should compile");
    let upkeep = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger)
            if format!("{:?}", trigger.effects).contains("RegisterDamageCounterReplacementEffect") => Some(trigger),
        _ => None,
    }).expect("upkeep trigger registers the replacement");
    let debug = format!("{:?}", upkeep.effects);
    assert!(debug.contains("YourNextUpkeep"), "{debug}");
    assert!(debug.contains("Echo"), "{debug}");

    struct Accept;
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let echo = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(echo, CounterType::Echo, 3);
    let mut dm = Accept;
    let mut ctx = EffectContext::new(echo, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in upkeep.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert!(game.battlefield.contains(&echo), "echo counters remain, so it isn't sacrificed");

    let goblin = CardBuilder::new(CardId::new(), "Damage source").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let source = game.create_object_from_card(&goblin, bob, Zone::Battlefield);
    let damage = Effect::deal_damage(2, ironsmith::target::ChooseSpec::Player(
        ironsmith::target::PlayerFilter::Specific(alice)));
    let mut ctx = EffectContext::new_default(source, bob);
    execute_effect(&mut game, &damage, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 20, "the damage is replaced");
    assert_eq!(game.counter_count(echo, CounterType::Echo), 1, "one echo counter per 1 damage");
}

const P23_STRUGGLE_FOR_SANITY: &str = "Mana cost: {2}{B}{B}\nType: Sorcery\nTarget opponent reveals their hand. That player exiles a card from it, then you exile a card from it. Repeat this process until all cards in that hand have been exiled. That player returns the cards they exiled this way to their hand and puts the rest into their graveyard.";

#[test]
fn struggle_for_sanity_alternates_exiles_and_partitions_by_exiler() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    let definition = compile_to_runtime_definition("Struggle for Sanity", P23_STRUGGLE_FOR_SANITY, false)
        .expect("the alternating hand exile should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("RepeatProcessEffect"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let sorcery = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut cards = Vec::new();
    for name in ["Hand one", "Hand two", "Hand three"] {
        let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1)).build();
        let id = game.create_object_from_card(&card, bob, Zone::Hand);
        cards.push(game.object(id).unwrap().stable_id);
    }
    let mut ctx = EffectContext::new_default(sorcery, alice)
        .with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let zones = cards.iter().map(|stable| {
        let id = game.find_object_by_stable_id(*stable).expect("card still exists");
        game.object(id).unwrap().zone
    }).collect::<Vec<_>>();
    assert!(!zones.contains(&Zone::Exile), "nothing stays exiled: {zones:?}");
    assert_eq!(zones.iter().filter(|zone| **zone == Zone::Hand).count(), 2,
        "the opponent exiled two of three cards and gets them back: {zones:?}");
    assert_eq!(zones.iter().filter(|zone| **zone == Zone::Graveyard).count(), 1,
        "the card you exiled goes to the graveyard: {zones:?}");
}

const P23_SPY_KIT: &str = "Mana cost: {2}\nType: Artifact — Equipment\nEquipped creature gets +1/+1 and has all names of nonlegendary creature cards in addition to its name.\nEquip {2}";

#[test]
fn spy_kit_equipped_creature_also_has_every_nonlegendary_creature_card_name() {
    use ironsmith::effects::EffectContext;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::types::Supertype;
    let definition = compile_to_runtime_definition("Spy Kit", P23_SPY_KIT, false)
        .expect("the multi-name equipment grant should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(rule)
            if rule.id() == ironsmith::static_abilities::StaticAbilityId::AttachedHasAllNonlegendaryCreatureCardNames)),
        "{:?}", definition.abilities);

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let kit = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let host = CardBuilder::new(CardId::new(), "Spy Kit host").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let host = game.create_object_from_card(&host, alice, Zone::Battlefield);
    // Physical cards in the game name a nonlegendary creature card and a
    // legendary one, whatever the catalog in this build holds.
    let rats = CardBuilder::new(CardId::new(), "Spy Kit Witness Rats").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    game.create_object_from_card(&rats, alice, Zone::Library);
    let legend = CardBuilder::new(CardId::new(), "Spy Kit Witness Legend").card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary]).power_toughness(PowerToughness::fixed(1, 1)).build();
    game.create_object_from_card(&legend, alice, Zone::Library);
    let named = |name: &str| {
        let mut filter = ironsmith::target::ObjectFilter::creature();
        filter.zone = Some(Zone::Battlefield);
        filter.name = Some(name.to_string());
        ironsmith::Value::Count(filter)
    };
    let count = |game: &ironsmith::GameState, name: &str| {
        let ctx = EffectContext::new_default(kit, alice);
        ironsmith::effects::helpers::resolve_value(game, &named(name), &ctx).unwrap()
    };
    assert_eq!(count(&game, "Spy Kit Witness Rats"), 0, "unequipped, it has only its own name");
    assert!(game.attach_object_to_target(kit, AttachmentTarget::Object(host)));
    assert_eq!(count(&game, "Spy Kit Witness Rats"), 1, "equipped, it has the nonlegendary creature card's name");
    assert_eq!(count(&game, "Spy Kit host"), 1, "in addition to its own name");
    assert_eq!(count(&game, "Spy Kit Witness Legend"), 0, "but no legendary card's name");
}

const P23_SHARE_THE_SPOILS: &str = "Mana cost: {1}{R}\nType: Enchantment\nWhen this enchantment enters and whenever an opponent loses the game, exile the top card of each player's library.\nDuring each player's turn, that player may play a land or cast a spell from among cards exiled with this enchantment, and they may spend mana as though it were mana of any color to cast that spell. When they do, exile the top card of their library.";

#[test]
fn share_the_spoils_lets_the_active_player_play_from_its_exile_with_any_color_and_refills() {
    let definition = compile_to_runtime_definition("Share the Spoils", P23_SHARE_THE_SPOILS, false)
        .expect("the turn player's source-exiled play permission should compile");
    let spec = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(rule) => rule.grant_spec(),
        _ => None,
    }).expect("standing play permission");
    assert!(matches!(spec.grantable, ironsmith::grant::Grantable::PlayFrom), "{spec:?}");
    assert_eq!(spec.zone, Zone::Exile);
    assert_eq!(spec.beneficiary, ironsmith::target::PlayerFilter::Active,
        "during each player's turn, that player may play");
    assert_eq!(spec.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    assert!(spec.filter.tagged_constraints.iter().any(|constraint|
        constraint.tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG), "cards exiled with this: {:?}", spec.filter);
    let on_use = format!("{:?}", spec.on_use_effects);
    assert!(on_use.contains("ExileTopOfLibrary") || on_use.contains("Exile"), "when they do, exile: {on_use}");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(format!("{:?}", trigger.effects)),
        _ => None,
    }).expect("refill trigger");
    assert!(trigger.contains("ExileTopOfLibrary") || trigger.contains("Exile"), "{trigger}");
}
