//! Claude package 25 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p25_definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn p25_triggered_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => {
                Some(trigger.effects.all_effects_owned())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

const ULDAROS_TEXT: &str = "Mana cost: {3}{U}{B}{B}\nType: Legendary Creature — Elder Sphinx\nFlying\nWhen Uldaros Theorix enters, if you cast him, exile up to one target nonland card of each card type from your graveyard. Copy those cards. You may cast any number of spells with total mana value 6 or less from among the copies without paying their mana costs. (Permanent spells cast this way become tokens.)";

#[test]
fn uldaros_targets_one_graveyard_card_per_card_type_slot() {
    use ironsmith::card::CardBuilder;
    use ironsmith::game_state::{GameState, Target};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = p25_definition("Uldaros Theorix", ULDAROS_TEXT);
    let debug = format!("{definition:#?}");
    let spec = p25_triggered_effects(&definition)
        .iter()
        .find_map(|effect| {
            effect
                .0
                .get_target_spec()
                .filter(|spec| format!("{spec:?}").contains("one_per_card_type: true"))
                .cloned()
        })
        .unwrap_or_else(|| panic!("one-per-card-type graveyard target: {debug}"));
    assert_eq!(spec.count().min, 0, "{spec:?}");
    assert_eq!(spec.count().max, None, "one slot per card type, not one target: {spec:?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("SourceWasCast"), "{debug}");
    assert!(compact.contains("ExileEffect") || compact.contains("Exile"), "{debug}");

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut card = |name: &str, types: Vec<CardType>| {
        let card = CardBuilder::new(CardId::new(), name).card_types(types).build();
        Target::Object(game.create_object_from_card(&card, alice, Zone::Graveyard))
    };
    let artifact_creature = card("Artifact creature", vec![CardType::Artifact, CardType::Creature]);
    let creature = card("Creature", vec![CardType::Creature]);
    let artifact = card("Artifact", vec![CardType::Artifact]);
    let instant = card("Instant", vec![CardType::Instant]);
    let legal = vec![artifact_creature, creature, artifact, instant];
    let constraint = ironsmith::targeting::resolved_target_aggregate_constraint(
        &game,
        &spec,
        alice,
        Some(source),
        &legal,
    )
    .expect("one-per-card-type slots are a set constraint");
    assert!(constraint.allows(&[artifact_creature, creature]), "artifact creature fills the artifact slot");
    assert!(constraint.allows(&[artifact_creature, artifact, instant]));
    assert!(!constraint.allows(&[artifact_creature, creature, artifact]), "two cards would share a slot");
    assert!(!constraint.allows(&[creature, artifact_creature, artifact, instant]));
    assert!(constraint.supports_minimum(3) && !constraint.supports_minimum(4));
}

const THREE_DOG_TEXT: &str = "Mana cost: {1}{R}{W}\nType: Legendary Creature — Human Bard\nPower/Toughness: 1/5\nWhenever you attack, you may pay {2} and sacrifice an Aura attached to Three Dog. When you sacrifice an Aura this way, for each other attacking creature you control, create a token that's a copy of that Aura attached to that creature.";

#[test]
fn three_dog_copies_the_sacrificed_aura_onto_each_other_attacker() {
    let definition = p25_definition("Three Dog, Galaxy News DJ", THREE_DOG_TEXT);
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("ForEachObject") || compact.contains("ForEachObjectEffect"), "{debug}");
    let copy = compact
        .split_once("CreateTokenCopyEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("token copy: {debug}"));
    let copy_target = copy.split_once("count:").map_or(copy, |(head, _)| head);
    assert!(copy_target.contains("sacrific"), "copy source is the sacrificed Aura: {debug}");
    assert!(!copy_target.contains("\"__it__\""), "not the iterated creature: {debug}");
    assert!(!copy_target.contains("result_antecedent"), "alias must be resolved: {debug}");
    assert!(copy.contains("suppress_aura_attachment_choice:true"), "{debug}");
    let attach = compact
        .split_once("AttachObjectsEffect")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("copies are attached: {debug}"));
    assert!(attach[..attach.len().min(600)].contains("created"), "attaches the created copies: {debug}");
    assert!(compact.contains("attacking:true") || compact.contains("Attacking"), "{debug}");
}

const STANGG_TEXT: &str = "Mana cost: {2}{R}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/4\nWhenever Stangg attacks, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. It enters tapped and attacking. For each Aura and Equipment attached to Stangg, create a token that's a copy of it attached to Stangg Twin. Sacrifice all tokens created this way at the beginning of the next end step.";

#[test]
fn stangg_copies_attachments_onto_the_named_twin_and_sacrifices_every_token() {
    let definition = p25_definition("Stangg, Echo Warrior", STANGG_TEXT);
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    let twin = compact
        .split_once("CreateTokenEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("Stangg Twin creation: {debug}"));
    let twin = twin.split_once("CreateTokenCopyEffect").map_or(twin, |(head, _)| head);
    assert!(twin.to_ascii_lowercase().contains("stanggtwin"), "the token keeps its name: {debug}");
    assert!(twin.contains("enters_tapped:true") && twin.contains("enters_attacking:true"), "{debug}");
    assert!(twin.contains("sacrifice_at_next_end_step:true"), "the Twin is a token created this way: {debug}");
    let copy = compact
        .split_once("CreateTokenCopyEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("attachment copies: {debug}"));
    assert!(copy.contains("sacrifice_at_next_end_step:true"), "copies are sacrificed too: {debug}");
    assert!(copy.contains("suppress_aura_attachment_choice:true"), "{debug}");
    let attach = compact
        .split_once("AttachObjectsEffect")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("copies attach to the Twin: {debug}"));
    let attach = &attach[..attach.len().min(1500)];
    assert!(attach.to_ascii_lowercase().contains("name:some(\"stanggtwin\")"), "{debug}");
}

/// Pygmy Hippo / Drain Power building block: "<player> activates a mana
/// ability of each land they control" activates exactly one mana ability of
/// every land that player controls.
#[test]
fn activate_mana_ability_of_each_land_activates_every_controlled_land_once() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::PlayerFilter;
    use ironsmith::{PlayerId, Zone};
    let land = |name: &str, symbol: ironsmith::mana::ManaSymbol| {
        ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), name)
            .card_types(vec![ironsmith::types::CardType::Land])
            .with_ability(ironsmith::ability::Ability::mana(
                ironsmith::TotalCost::free(),
                vec![symbol],
            ))
            .build()
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&land("Green land", ironsmith::mana::ManaSymbol::Green), bob, Zone::Battlefield);
    game.create_object_from_definition(&land("Blue land", ironsmith::mana::ManaSymbol::Blue), bob, Zone::Battlefield);
    game.create_object_from_definition(&land("Alice land", ironsmith::mana::ManaSymbol::Red), alice, Zone::Battlefield);
    let source = game.create_object_from_definition(&land("Source", ironsmith::mana::ManaSymbol::White), alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    let effect = Effect::activate_mana_ability_of_each_land_player(PlayerFilter::Specific(bob));
    let outcome = execute_effect(&mut game, &effect, &mut ctx).expect("activation resolves");
    assert_eq!(outcome.as_count(), Some(2), "{outcome:?}");
    let bob_pool = &game.player(bob).unwrap().mana_pool;
    assert_eq!((bob_pool.green, bob_pool.blue), (1, 1));
    assert_eq!(game.player(alice).unwrap().mana_pool.red, 0, "only that player's lands");
}

const PYGMY_HIPPO_TEXT: &str = "Mana cost: {G}{U}\nType: Creature — Hippo\nPower/Toughness: 2/2\nWhenever this creature attacks and isn't blocked, you may have defending player activate a mana ability of each land they control and lose all unspent mana. If you do, this creature assigns no combat damage this turn and at the beginning of your next main phase this turn, you add an amount of {C} equal to the amount of mana that player lost this way.";

#[test]
fn pygmy_hippo_freezes_the_mana_lost_into_its_next_main_phase_reward() {
    let definition = p25_definition("Pygmy Hippo", PYGMY_HIPPO_TEXT);
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    let activate = compact
        .find("ActivateManaAbilityOfEachLandEffect")
        .unwrap_or_else(|| panic!("defending player activates each land: {debug}"));
    let empty = compact
        .find("EmptyManaPoolEffect")
        .unwrap_or_else(|| panic!("then loses all unspent mana: {debug}"));
    assert!(activate < empty, "{debug}");
    assert!(compact[activate..activate + 200].contains("Defending"), "{debug}");
    assert!(compact.contains("AssignNoCombatDamage"), "{debug}");
    let bind = compact
        .split_once("BindXValueEffect")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("the lost amount is frozen as X while scheduling: {debug}"));
    let (value, body) = bind
        .split_once("ScheduleDelayedTrigger")
        .unwrap_or_else(|| panic!("X binds around the delayed registration: {debug}"));
    assert!(value.contains("PriorEffectMetric") && value.contains("ManaLost"), "{debug}");
    assert!(!value.contains("Pending"), "the mana-lost producer is bound: {debug}");
    assert!(body.contains("BeginningOfMainPhase") || body.contains("MainPhase"), "{debug}");
    let add = body
        .split_once("AddScaledManaEffect")
        .or_else(|| body.split_once("AddManaEffect"))
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("adds colorless mana: {debug}"));
    assert!(add[..add.len().min(300)].contains("Colorless"), "{debug}");
    assert!(add[..add.len().min(300)].contains("X"), "the reward reads the frozen X: {debug}");
}
