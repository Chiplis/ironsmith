//! Claude package 24 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

const WORD_OF_COMMAND: &str = "Mana cost: {B}{B}\nType: Instant\nLook at target opponent's hand and choose a card from it. You control that player until Word of Command finishes resolving. The player plays that card if able. While doing so, the player can activate mana abilities only if they're from lands that player controls and only if mana they produce is spent to activate other mana abilities of lands the player controls and/or to play that card. If the chosen card is cast as a spell, you control the player while that spell is resolving.";

const DURESS: &str = "Mana cost: {B}\nType: Sorcery\nTarget opponent reveals their hand. You choose a noncreature, nonland card from it. That player discards that card.";

const SOL_RING: &str = "Mana cost: {1}\nType: Artifact\n{T}: Add {C}{C}.";

fn p24_land(mana: ironsmith::mana::ManaSymbol) -> ironsmith::cards::CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Land mana witness")
        .card_types(vec![ironsmith::types::CardType::Land])
        .with_ability(ironsmith::ability::Ability::mana(ironsmith::TotalCost::free(), vec![mana]))
        .build()
}

fn p24_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() {
            walk(effect, &mut out);
        }
    }
    out
}

/// Records who actually answers each decision (the controlling player of the
/// player the decision is for, CR 722) and answers with the first legal
/// choices.
struct P24Recorder {
    objects: Vec<(PlayerId, PlayerId, String)>,
    targets: Vec<(PlayerId, PlayerId)>,
}

impl ironsmith::decision::DecisionMaker for P24Recorder {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.objects.push((
            ctx.player,
            game.controlling_player_for(ctx.player),
            ctx.description.clone(),
        ));
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min.max(1).min(ctx.candidates.len()))
            .collect()
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.targets
            .push((ctx.player, game.controlling_player_for(ctx.player)));
        ctx.requirements
            .iter()
            .flat_map(|requirement| {
                requirement
                    .legal_targets
                    .iter()
                    .take(requirement.min_targets.max(1))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

fn p24_resolve_word_of_command(
    game: &mut GameState,
    alice: PlayerId,
    bob: PlayerId,
    dm: &mut P24Recorder,
) {
    let word = compile_to_runtime_definition("Word of Command", WORD_OF_COMMAND, false)
        .expect("Word of Command should compile");
    let source = game.create_object_from_definition(&word, alice, Zone::Stack);
    game.push_to_stack(
        ironsmith::game_state::StackEntry::new(source, alice)
            .with_targets(vec![ironsmith::game_state::Target::Player(bob)]),
    );
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).expect("Word of Command resolves");
}

/// Word of Command lowers to a resolution-bounded control of the target
/// opponent followed by that player's mandatory play of the chosen card,
/// carrying the land-only mana lock and the resolving-spell control.
#[test]
fn word_of_command_lowers_controlled_play_with_mana_lock() {
    let definition = compile_to_runtime_definition("Word of Command", WORD_OF_COMMAND, false)
        .expect("Word of Command should compile");
    let effects = p24_effects(&definition);
    let control = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ControlPlayerEffect>())
        .expect("control effect");
    assert_eq!(
        control.duration,
        ironsmith::game_state::PlayerControlDuration::UntilSourceFinishesResolving
    );
    assert_eq!(control.start, ironsmith::game_state::PlayerControlStart::Immediate);
    assert!(
        !matches!(control.player, ironsmith::target::PlayerFilter::IteratedPlayer),
        "that player is the target opponent: {:?}",
        control.player
    );
    let cast = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>())
        .expect("controlled play");
    assert!(cast.allow_land, "the player plays the card (lands included)");
    assert!(!cast.without_paying_mana_cost, "the player pays for the spell");
    assert!(cast.caster_land_mana_abilities_only);
    assert!(cast.control_caster_while_spell_resolves);
    assert!(
        !matches!(cast.player, ironsmith::target::PlayerFilter::You),
        "the target opponent plays the card: {:?}",
        cast.player
    );
}

/// CR 722: during Word of Command the target opponent casts the chosen Duress
/// with their decisions made by Word of Command's controller, and that control
/// is restored while Duress itself resolves, then ends.
#[test]
fn word_of_command_controls_the_player_while_the_chosen_spell_resolves() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    // A typed witness isolates control of a caster's target and resolution
    // choices without compiling another card's Oracle text.
    let duress = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(), "Controlled choice witness")
        .card_types(vec![ironsmith::types::CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Black]]))
        .with_spell_effect(vec![
            Effect::new(ironsmith::effects::TargetOnlyEffect::new(ironsmith::target::ChooseSpec::target_opponent())),
            Effect::choose_objects(
                ironsmith::target::ObjectFilter::default().in_zone(Zone::Hand)
                    .owned_by(ironsmith::target::PlayerFilter::target_opponent()),
                1, ironsmith::target::PlayerFilter::You, "controlled_choice"),
            Effect::move_to_zone(ironsmith::target::ChooseSpec::tagged("controlled_choice"), Zone::Graveyard, false),
        ]).build();
    let duress_id = game.create_object_from_definition(&duress, bob, Zone::Hand);
    game.create_object_from_definition(&p24_land(ironsmith::mana::ManaSymbol::Black), bob, Zone::Battlefield);
    let instant = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Alice's Instant")
        .card_types(vec![ironsmith::types::CardType::Instant])
        .build();
    let alice_instant = game.create_object_from_card(&instant, alice, Zone::Hand);
    // Force a genuine resolution choice rather than a one-option auto-pick.
    game.create_object_from_card(&instant, alice, Zone::Hand);

    let mut dm = P24Recorder { objects: Vec::new(), targets: Vec::new() };
    p24_resolve_word_of_command(&mut game, alice, bob, &mut dm);
    assert_eq!(
        game.controlling_player_for(bob),
        bob,
        "control ends when Word of Command finishes resolving"
    );
    assert_eq!(game.stack.len(), 1, "Bob cast Duress: {:?}", game.stack);
    let spell = game.stack.last().unwrap().object_id;
    assert_eq!(game.object(spell).unwrap().name.as_ref(), "Controlled choice witness");
    assert_ne!(game.object(duress_id).map(|object| object.zone), Some(Zone::Hand));
    assert!(
        dm.targets.iter().any(|(player, controller)| *player == bob && *controller == alice),
        "Bob's Duress target is chosen by Alice: {:?}",
        dm.targets
    );

    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).expect("Duress resolves");
    assert!(
        dm.objects.iter().any(|(player, controller, _)| *player == bob && *controller == alice),
        "Bob's Duress choice is made by Alice while Duress resolves: {:?}",
        dm.objects
    );
    let _ = alice_instant;
    assert!(
        game.player(alice).unwrap().graveyard.iter().any(|id| game
            .object(*id)
            .is_some_and(|object| object.name.as_ref() == "Alice's Instant")),
        "Duress discards Alice's instant"
    );
    assert_eq!(game.controlling_player_for(bob), bob, "control ends with Duress's resolution");
}

/// While playing the chosen card, the player can activate only mana abilities
/// of lands they control: a nonland mana source cannot fund the spell.
#[test]
fn word_of_command_play_uses_only_the_players_land_mana() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sol_ring = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(), "Nonland mana witness")
        .card_types(vec![ironsmith::types::CardType::Artifact])
        .with_ability(ironsmith::ability::Ability::mana(
            ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Colorless; 2]))
        .build();
    for forests in [1usize, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::new(), "Creature spell witness")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![
                vec![ironsmith::mana::ManaSymbol::Generic(1)], vec![ironsmith::mana::ManaSymbol::Green]]))
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        let bears = game.create_object_from_definition(&creature, bob, Zone::Hand);
        let ring = game.create_object_from_definition(&sol_ring, bob, Zone::Battlefield);
        let lands: Vec<ObjectId> = (0..forests)
            .map(|_| {
                game.create_object_from_definition(
                    &p24_land(ironsmith::mana::ManaSymbol::Green),
                    bob,
                    Zone::Battlefield,
                )
            })
            .collect();
        let mut dm = P24Recorder { objects: Vec::new(), targets: Vec::new() };
        p24_resolve_word_of_command(&mut game, alice, bob, &mut dm);
        assert!(!game.is_tapped(ring), "Sol Ring is not a land: {forests} forest(s)");
        if forests == 1 {
            assert!(game.stack.is_empty(), "one Forest cannot pay {{1}}{{G}}: {:?}", game.stack);
            assert_eq!(game.object(bears).map(|object| object.zone), Some(Zone::Hand));
        } else {
            assert_eq!(game.stack.len(), 1, "two Forests pay for the Bears");
            assert!(lands.iter().all(|land| game.is_tapped(*land)));
            let spell = game.stack.last().unwrap().object_id;
            assert_eq!(game.controller_of(game.object(spell).unwrap()), bob);
        }
        assert_eq!(game.controlling_player_for(bob), bob);
    }
}
