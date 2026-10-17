//! Claude package 11 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::PlayerId;
use ironsmith::Zone;
use ironsmith::game_state::GameState;

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children.
fn p11_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { walk(effect, &mut out); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { walk(effect, &mut out); }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { walk(effect, &mut out); }
            }
            _ => {}
        }
    }
    out
}

const TIME_VAULT: &str = "Mana cost: {2}\nType: Artifact\nThis artifact enters tapped.\nThis artifact doesn't untap during your untap step.\nIf you would begin your turn while this artifact is tapped, you may skip that turn instead. If you do, untap this artifact.\n{T}: Take an extra turn after this one.";

/// CR 614.10: Time Vault's turn-begin replacement is a static ability the
/// turn runner offers before the next turn is selected.
#[test]
fn time_vault_offers_turn_skip_before_the_next_turn_begins() {
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let definition = compile_to_runtime_definition("Time Vault", TIME_VAULT, false)
        .expect("Time Vault should compile");
    assert!(definition.spell_effect.is_none());
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::MaySkipTurnWhileSourceTappedThenUntap)),
        "{:?}", definition.abilities);
    assert!(p11_effects(&definition).iter().all(|effect|
        effect.downcast_ref::<ironsmith::effects::SkipTurnEffect>().is_none()),
        "the skip is not a resolution-time effect");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for accept in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = bob;
        let vault = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.tap(vault);
        assert_eq!(game.upcoming_turn_player(), Some(alice));
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Complete);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let action = runner.advance(&mut game, &mut queue).unwrap();
        let TurnAction::Decision(DecisionContext::Boolean(prompt)) = action else {
            panic!("expected the turn-begin skip offer, got {action:?}");
        };
        assert_eq!((prompt.player, prompt.source), (alice, Some(vault)));
        runner.respond_boolean(accept);
        let action = runner.advance(&mut game, &mut queue).unwrap();
        assert!(matches!(action, TurnAction::TurnComplete), "{action:?}");
        game.next_turn();
        if accept {
            assert!(!game.is_tapped(vault), "if you do, untap this artifact");
            assert_eq!(game.turn.active_player, bob, "Alice's turn was skipped");
        } else {
            assert!(game.is_tapped(vault));
            assert_eq!(game.turn.active_player, alice);
        }
    }

    // An untapped vault offers nothing.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    let vault = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.untap(vault);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Complete);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::TurnComplete));
}

/// "Until that player's next turn": restrictions and prevention keyed to the
/// target opponent's next turn, not the caster's (`Until::PlayersNextTurn`).
#[test]
fn teferis_reproach_protects_the_target_until_their_next_turn() {
    use ironsmith::effect::{Restriction, Until};
    use ironsmith::effects::{CantEffect, EffectContext, PreventAllDamageToTargetEffect, ResolvedTarget, execute_effect};
    let definition = compile_to_runtime_definition("Teferi's Reproach",
        "Mana cost: {2}{W}\nType: Instant\nChoose target opponent. Until that player's next turn, they gain protection from everything and their life total can't change. All nonland permanents they control phase out. (While they're phased out, they're treated as though they don't exist. They phase in before that player untaps during their next untap step.)\nExile Teferi's Reproach.", false)
        .expect("Teferi's Reproach should compile");
    let effects = p11_effects(&definition);
    let cants = effects.iter().filter_map(|effect| effect.downcast_ref::<CantEffect>()).collect::<Vec<_>>();
    assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::ChangeLifeTotal(_))), "{cants:?}");
    assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::BeTargetedPlayer(_))), "{cants:?}");
    assert!(cants.iter().all(|cant| matches!(cant.duration, Until::PlayersNextTurn { .. })), "{cants:?}");
    assert!(cants.iter().all(|cant| !matches!(cant.restriction,
        Restriction::ChangeLifeTotal(ironsmith::target::PlayerFilter::You)
            | Restriction::BeTargetedPlayer(ironsmith::target::PlayerFilter::You))),
        "the target, not the caster, is protected: {cants:?}");
    let prevent = effects.iter().find_map(|effect| effect.downcast_ref::<PreventAllDamageToTargetEffect>())
        .expect("protection prevents damage to that player");
    assert!(matches!(prevent.duration, Until::PlayersNextTurn { .. }), "{prevent:?}");
    assert!(effects.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>().is_none()),
        "'they' is the player, not the spell");

    // Runtime: Alice (active) casts it at Bob. Bob's life total is locked
    // through Alice's turn and unlocks as Bob's next turn begins.
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in &effects {
        if effect.downcast_ref::<CantEffect>().is_some() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    drop(ctx);
    game.update_cant_effects();
    assert!(!game.can_change_life_total(bob));
    assert!(game.can_change_life_total(alice));
    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
    assert!(game.can_change_life_total(bob), "the duration ends as Bob's next turn begins");
    game.next_turn();
    assert!(game.can_change_life_total(bob), "and never applies again");
}

/// "if that player has another opponent who isn't being attacked": a
/// combat-participant predicate over the triggering attack declaration.
#[test]
fn suppressor_skyguard_checks_for_an_unattacked_other_opponent() {
    let definition = compile_to_runtime_definition("Suppressor Skyguard",
        "Mana cost: {3}{W}\nType: Creature — Human Knight\nPower/Toughness: 3/3\nFlying\nWhenever a player attacks you, if that player has another opponent who isn't being attacked, prevent all combat damage that would be dealt to you this combat.", false)
        .expect("Suppressor Skyguard should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("attack trigger");
    assert_eq!(trigger.intervening_if, Some(ironsmith::effect::Condition::CombatParticipant(
        ironsmith_core::CombatParticipantCondition::AttackingPlayerHasUnattackedOtherOpponent)));
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("PreventAllCombatDamage") && debug.contains("EndOfCombat"), "{debug}");
}

/// "at the beginning of that player's next declare attackers step": a
/// one-shot return at the owner's upcoming declare attackers step (this
/// turn's when still ahead), carrying "It can't be blocked that combat".
#[test]
fn noctis_returns_attacking_at_the_upcoming_declare_attackers_step() {
    let definition = compile_to_runtime_definition("Noctis, Heir Apparent",
        "Mana cost: {1}{W}{U}{B}\nType: Legendary Creature — Human Noble\nPower/Toughness: 3/3\nWhenever a creature you control enters during combat, you may attach target Equipment you control to target creature you control.\nWarp-Strike — {3}: Exile Noctis. Return it to the battlefield under its owner's control tapped and attacking at the beginning of that player's next declare attackers step. It can't be blocked that combat.", false)
        .expect("Noctis should compile");
    let schedule = p11_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>().cloned())
        .expect("delayed return");
    assert!(schedule.one_shot);
    assert!(!schedule.start_next_turn, "the next declare attackers step may be this turn's");
    let declare = schedule.trigger.downcast_ref::<ironsmith::triggers::BeginningOfDeclareAttackersStepTrigger>()
        .unwrap_or_else(|| panic!("{:?}", schedule.trigger));
    assert!(!matches!(declare.player, ironsmith::target::PlayerFilter::You), "the owner's step: {:?}", declare.player);
    let body = format!("{:?}", schedule.effects);
    assert!(body.contains("BeBlocked") && body.contains("EndOfCombat"),
        "'It can't be blocked that combat' starts with the return: {body}");
}

/// "dealt to target creature of an opponent's choice instead": an opponent
/// announces the redirect destination, which the redirect reads by tag.
#[test]
fn nova_pentacle_redirects_to_a_target_an_opponent_chooses() {
    let definition = compile_to_runtime_definition("Nova Pentacle",
        "Mana cost: {4}\nType: Artifact\n{3}, {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.", false)
        .expect("Nova Pentacle should compile");
    let effects = p11_effects(&definition);
    let target_only = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>())
        .expect("the destination is announced");
    assert_eq!(target_only.chooser, Some(ironsmith::target::PlayerFilter::Opponent), "{target_only:?}");
    assert!(target_only.target.is_target());
    let redirect = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RedirectNextTimeDamageToSourceEffect>())
        .expect("next-time redirect");
    let destination = redirect.destination_target.as_ref().expect("redirect destination");
    assert!(!destination.is_target(), "the redirect reuses the announced target: {destination:?}");
}

/// "You control target opponent during their next combat phase": player
/// control that begins with that player's next combat phase and ends with it
/// (CR 723), with the paid-cost "next turn instead" upgrade.
#[test]
fn secret_of_bloodbending_controls_the_opponent_for_one_combat_phase() {
    use ironsmith::game_state::{PlayerControlDuration, PlayerControlStart};
    let definition = compile_to_runtime_definition("Secret of Bloodbending",
        "Mana cost: {U}{U}{U}{U}\nType: Sorcery — Lesson\nAs an additional cost to cast this spell, you may waterbend {10}.\nYou control target opponent during their next combat phase. If this spell's additional cost was paid, you control that player during their next turn instead. (You see all cards that player could see and make all decisions for them.)\nExile Secret of Bloodbending.", false)
        .expect("Secret of Bloodbending should compile");
    let controls = p11_effects(&definition).into_iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::ControlPlayerEffect>().cloned())
        .collect::<Vec<_>>();
    assert!(controls.iter().any(|control| control.start == PlayerControlStart::NextCombatPhase
        && control.duration == PlayerControlDuration::ThroughCombatPhase), "{controls:?}");
    assert!(controls.iter().any(|control| control.start == PlayerControlStart::NextTurn
        && control.duration == PlayerControlDuration::WholeTurn), "paid upgrade: {controls:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.add_player_control(alice, bob, PlayerControlStart::NextCombatPhase,
        PlayerControlDuration::ThroughCombatPhase, None);
    game.mark_combat_phase_started();
    assert_eq!(game.controlling_player_for(bob), bob, "Alice's combat phase is not Bob's");
    game.cleanup_effects_end_of_combat();
    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
    assert_eq!(game.controlling_player_for(bob), bob, "control waits for the combat phase");
    game.mark_combat_phase_started();
    assert_eq!(game.controlling_player_for(bob), alice);
    game.cleanup_effects_end_of_combat();
    assert_eq!(game.controlling_player_for(bob), bob, "control ends with that combat phase");
}

/// Oath of Mages / Oath of Scholars: the upkeep player announces a target
/// opponent with more life / more cards in hand than they have.
#[test]
fn oaths_let_the_upkeep_player_target_an_opponent_with_more() {
    use ironsmith::target::PlayerFilter;
    for (name, text, quantity) in [
        ("Oath of Mages", "Mana cost: {1}{R}\nType: Enchantment\nAt the beginning of each player's upkeep, that player chooses target player who has more life than they do and is their opponent. The first player may have this enchantment deal 1 damage to the second player.", ironsmith_core::PlayerQuantity::Life),
        ("Oath of Scholars", "Mana cost: {3}{U}\nType: Enchantment\nAt the beginning of each player's upkeep, that player chooses target player who has more cards in hand than they do and is their opponent. The first player may discard their hand and draw three cards.", ironsmith_core::PlayerQuantity::CardsInHand),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let effects = p11_effects(&definition);
        let target_only = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>())
            .unwrap_or_else(|| panic!("{name}: target announcement"));
        assert_eq!(target_only.chooser, Some(PlayerFilter::Active), "{name}: the upkeep player chooses");
        let debug = format!("{:?}", target_only.target);
        assert!(debug.contains("QuantityGreaterThanPlayer") && debug.contains(&format!("{quantity:?}")), "{name}: {debug}");
        let reward = format!("{effects:?}");
        assert!(reward.contains("MayByPlayer") || reward.contains("May"), "{name}: {reward}");
    }
}

const P11_THE_MASTER: &str = "Mana cost: {2}{B}{R}\nType: Legendary Creature — Time Lord Rogue\nPower/Toughness: 3/3\nMake Them Pay — Whenever a nontoken artifact creature you control dies, you may exile it. If you do, choose an opponent with the most life among your opponents. That player faces a villainous choice — They lose 4 life, or you create a token that's a copy of that card.";

/// Records the option descriptions offered for a player choice and picks the
/// first one.
struct P11MasterRecorder {
    offered: Vec<String>,
}

impl ironsmith::decision::DecisionMaker for P11MasterRecorder {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.offered = ctx
            .options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.description.clone())
            .collect();
        vec![ctx.options.iter().find(|option| option.legal).map(|o| o.index).unwrap_or(0)]
    }
}

/// "choose an opponent with the most life among your opponents": only the
/// opponents tied for the highest life among opponents are eligible (your own
/// life total is irrelevant), and the villainous choice / "they" bind to the
/// chosen opponent while the copy branch copies the exiled card.
#[test]
fn p11_the_master_chooses_among_highest_life_opponents() {
    use ironsmith::effects::EffectExecutor;
    use ironsmith::target::PlayerFilter;
    let definition = compile_to_runtime_definition("The Master, Gallifrey's End", P11_THE_MASTER, false)
        .expect("The Master should compile");
    let effects = p11_effects(&definition);
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChoosePlayerEffect>())
        .expect("a player choice")
        .clone();
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert_eq!(
        choose.filter,
        PlayerFilter::MostLifeTiedAmong { base: Box::new(PlayerFilter::Opponent) },
        "{choose:?}"
    );
    let villainous = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::VillainousChoiceEffect>())
        .expect("a villainous choice");
    assert_eq!(villainous.modes.len(), 2);
    let debug = format!("{:?}", villainous.modes);
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(debug.contains("CreateTokenCopyEffect"), "{debug}");
    assert!(debug.contains("triggering"), "the copy names the exiled triggering card: {debug}");

    let names = ["Alice", "Bob", "Carol", "Dan"];
    let mut game = GameState::new(names.iter().map(|n| (*n).into()).collect(), 20);
    let alice = PlayerId::from_index(0);
    for (index, life) in [(0, 40), (1, 12), (2, 25), (3, 25)] {
        game.player_mut(PlayerId::from_index(index)).unwrap().life = life;
    }
    let master = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut recorder = P11MasterRecorder { offered: Vec::new() };
    let mut ctx = ironsmith::effects::EffectContext::new_default(master, alice)
        .with_decision_maker(&mut recorder);
    choose.execute(&mut game, &mut ctx).expect("the choice resolves");
    let chosen = ctx.get_tagged_players(choose.tag.as_str()).cloned().unwrap_or_default();
    let it = ctx.get_tagged_players("__it__").cloned().unwrap_or_default();
    drop(ctx);
    let mut offered = recorder.offered.clone();
    offered.sort();
    assert_eq!(offered, vec!["Carol".to_string(), "Dan".to_string()],
        "only the tied highest-life opponents; not you, not Bob");
    assert_eq!(chosen.len(), 1);
    assert!(chosen[0] == PlayerId::from_index(2) || chosen[0] == PlayerId::from_index(3));
    assert_eq!(it, chosen, "\"that player\" / \"they\" read the chosen opponent");
}

const P11_NIV_MIZZET_REBORN: &str = "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Creature — Dragon Avatar\nPower/Toughness: 6/6\nFlying\nWhen Niv-Mizzet enters, reveal the top ten cards of your library. For each color pair, choose a card that's exactly those colors from among them. Put the chosen cards into your hand and the rest on the bottom of your library in a random order.";
const P11_NOXIOUS_VAPORS: &str = "Mana cost: {1}{B}{B}\nType: Sorcery\nEach player reveals their hand, chooses one card of each color from it, then discards all other nonland cards.";

/// Per-color slot choices (CR 105.1, 105.4): Niv-Mizzet Reborn makes one
/// choice per two-color pair, each of a revealed card that is exactly those
/// colors and not already chosen; Noxious Vapors makes one choice per color
/// from each player's hand, and "all other nonland cards" excludes the
/// chosen cards (not only the source).
#[test]
fn p11_color_slot_choices_keep_one_card_per_color_or_pair() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::mana::ManaCost;
    use ironsmith::types::CardType;
    use ironsmith::ManaSymbol;

    struct FirstLegal;
    impl ironsmith::decision::DecisionMaker for FirstLegal {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            ctx.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect()
        }
    }
    let colored = |name: &str, symbols: Vec<ManaSymbol>, types: Vec<CardType>| {
        let builder = CardBuilder::new(CardId::new(), name).card_types(types);
        if symbols.is_empty() { builder.build() } else { builder.mana_cost(ManaCost::from_symbols(symbols)).build() }
    };
    let names_in = |game: &GameState, ids: Vec<ironsmith::ObjectId>| -> Vec<String> {
        let mut names: Vec<String> = ids.iter().filter_map(|id| game.object(*id)).map(|object| object.name.to_string()).collect();
        names.sort();
        names
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Niv-Mizzet Reborn.
    let niv = compile_to_runtime_definition("Niv-Mizzet Reborn", P11_NIV_MIZZET_REBORN, false)
        .expect("Niv-Mizzet Reborn should compile");
    let niv_choices = p11_effects(&niv).iter()
        .filter(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some())
        .count();
    assert_eq!(niv_choices, 10, "one choice per color pair");
    let trigger = niv.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(&trigger.effects), _ => None,
    }).expect("enters trigger");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&niv, alice, Zone::Battlefield);
    let library = [
        ("Azorius one", vec![ManaSymbol::White, ManaSymbol::Blue]),
        ("Azorius two", vec![ManaSymbol::White, ManaSymbol::Blue]),
        ("Dimir", vec![ManaSymbol::Blue, ManaSymbol::Black]),
        ("Gruul", vec![ManaSymbol::Red, ManaSymbol::Green]),
        ("Esper", vec![ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black]),
        ("Mono red", vec![ManaSymbol::Red]),
        ("Colorless", vec![ManaSymbol::Generic(2)]),
        ("Filler one", vec![]),
        ("Filler two", vec![]),
        ("Filler three", vec![]),
    ];
    for (name, symbols) in library {
        let types = if symbols.is_empty() { vec![CardType::Land] } else { vec![CardType::Creature] };
        game.create_object_from_card(&colored(name, symbols, types), alice, Zone::Library);
    }
    let mut dm = FirstLegal;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    for effect in trigger.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    drop(ctx);
    let hand = names_in(&game, game.player(alice).unwrap().hand.to_vec());
    assert_eq!(hand.len(), 3, "{hand:?}");
    assert!(hand.iter().filter(|name| name.starts_with("Azorius")).count() == 1, "{hand:?}");
    assert!(hand.contains(&"Dimir".to_string()) && hand.contains(&"Gruul".to_string()), "{hand:?}");
    assert_eq!(game.player(alice).unwrap().library.len(), 7, "the rest go to the bottom");

    // Noxious Vapors.
    let vapors = compile_to_runtime_definition("Noxious Vapors", P11_NOXIOUS_VAPORS, false)
        .expect("Noxious Vapors should compile");
    let vapors_choices = p11_effects(&vapors).iter()
        .filter(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some())
        .count();
    assert_eq!(vapors_choices, 5, "one choice per color");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&vapors, alice, Zone::Exile);
    for (name, symbols, types) in [
        ("Red one", vec![ManaSymbol::Red], vec![CardType::Instant]),
        ("Red two", vec![ManaSymbol::Red], vec![CardType::Instant]),
        ("Blue", vec![ManaSymbol::Blue], vec![CardType::Creature]),
        ("Colorless", vec![ManaSymbol::Generic(3)], vec![CardType::Artifact]),
        ("Land", vec![], vec![CardType::Land]),
    ] {
        game.create_object_from_card(&colored(name, symbols, types), alice, Zone::Hand);
    }
    for (name, symbols, types) in [
        ("Bob green", vec![ManaSymbol::Green], vec![CardType::Sorcery]),
        ("Bob colorless", vec![ManaSymbol::Generic(1)], vec![CardType::Artifact]),
    ] {
        game.create_object_from_card(&colored(name, symbols, types), bob, Zone::Hand);
    }
    let mut dm = FirstLegal;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    for effect in vapors.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let alice_hand = names_in(&game, game.player(alice).unwrap().hand.to_vec());
    assert_eq!(alice_hand.len(), 3, "one red, the blue card and the land stay: {alice_hand:?}");
    assert!(alice_hand.contains(&"Blue".to_string()) && alice_hand.contains(&"Land".to_string()), "{alice_hand:?}");
    assert_eq!(alice_hand.iter().filter(|name| name.starts_with("Red")).count(), 1, "{alice_hand:?}");
    let alice_graveyard = names_in(&game, game.player(alice).unwrap().graveyard.to_vec());
    assert!(alice_graveyard.contains(&"Colorless".to_string()), "{alice_graveyard:?}");
    assert_eq!(alice_graveyard.len(), 2, "{alice_graveyard:?}");
    let bob_hand = names_in(&game, game.player(bob).unwrap().hand.to_vec());
    assert_eq!(bob_hand, vec!["Bob green".to_string()]);
}

const VOLCANO_HELLION: &str = "Mana cost: {2}{R}{R}\nType: Creature — Hellion\nPower/Toughness: 6/5\nThis creature has echo {X}, where X is your life total. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)\nWhen this creature enters, it deals an amount of damage of your choice to you and target creature. The damage can't be prevented.";

/// CR 702.30a + 107.3: echo {X} with X fixed at payment time by the card's
/// own definition (your life total), never chosen by the payer.
#[test]
fn p11_volcano_hellion_echo_x_is_life_total() {
    use ironsmith::mana::ManaSymbol;
    let definition = compile_to_runtime_definition("Volcano Hellion", VOLCANO_HELLION, false)
        .expect("Volcano Hellion should compile");
    let effects = p11_effects(&definition);
    let pay = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::PayManaEffect>())
        .expect("echo payment");
    assert!(pay.cost.has_x());
    assert!(pay.x_value.is_some(), "X is defined, not chosen: {pay:?}");
    let upkeep = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger)
            if trigger.effects.all_effects().into_iter().any(|effect|
                effect.downcast_ref::<ironsmith::effects::CumulativeUpkeepEffect>().is_some()) => Some(trigger),
        _ => None,
    }).expect("echo upkeep trigger");
    struct Pay;
    impl ironsmith::decision::DecisionMaker for Pay {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
    }
    let alice = PlayerId::from_index(0);
    for (mana, survives) in [(3u32, true), (2u32, false)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.player_mut(alice).unwrap().life = 3;
        let hellion = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, mana);
        let mut dm = Pay;
        let mut ctx = ironsmith::effects::EffectContext::new(hellion, alice, &mut dm);
        for effect in upkeep.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.battlefield.contains(&hellion), survives,
            "life 3 means echo {{3}}; {mana} mana in pool");
    }
}

/// The controller chooses one number; the same amount is dealt to them and
/// to the target, and neither damage can be prevented.
#[test]
fn p11_volcano_hellion_chosen_damage_to_you_and_target() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Volcano Hellion", VOLCANO_HELLION, false)
        .expect("Volcano Hellion should compile");
    let effects = p11_effects(&definition);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::ChooseNumberEffect>().is_some()));
    let damages: Vec<_> = effects.iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::DealDamageEffect>()).collect();
    assert_eq!(damages.len(), 2, "{damages:?}");
    assert!(damages.iter().all(|damage| damage.unpreventable), "{damages:?}");
    assert_eq!(damages[0].amount, damages[1].amount, "one shared chosen amount");
    let etb = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger)
            if trigger.effects.all_effects().into_iter().any(|effect|
                effect.downcast_ref::<ironsmith::effects::ChooseNumberEffect>().is_some()) => Some(trigger),
        _ => None,
    }).expect("ETB trigger");
    struct Three;
    impl ironsmith::decision::DecisionMaker for Three {
        fn decide_number(&mut self, _: &GameState, _: &ironsmith::decisions::context::NumberContext) -> u32 { 3 }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hellion = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let fixture = CardBuilder::new(CardId::new(), "Damage recipient").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 5)).build();
    let victim = game.create_object_from_card(&fixture, bob, Zone::Battlefield);
    game.push_to_stack(ironsmith::game_state::StackEntry::ability(hellion, alice, etb.effects.clone())
        .with_targets(vec![ironsmith::game_state::Target::Object(victim)]));
    let mut dm = Three;
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(game.damage_on(victim), 3);
}

const NORTH_STAR: &str = "Mana cost: {4}\nType: Artifact\n{4}, {T}: For one spell this turn, you may spend mana as though it were mana of any type to pay that spell's mana cost. (Additional costs are still paid normally.)";

/// The permission is an any-type spending rule scoped to one of its
/// controller's spells (used up when that player finishes casting a spell);
/// permanents' activations never get it.
#[test]
fn p11_north_star_one_spell_any_type_permission() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("North Star", NORTH_STAR, false)
        .expect("North Star should compile");
    let register = p11_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RegisterManaSpendPermissionEffect>().cloned())
        .expect("mana spend permission");
    assert_eq!(register.permission.scope, ironsmith::effect::ManaSpendScope::OneSpellCast);
    assert!(register.permission.mode.allows_any_type());
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let star = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell = CardBuilder::new(CardId::new(), "Spell fixture").card_types(vec![CardType::Sorcery]).build();
    let spell = game.create_object_from_card(&spell, alice, Zone::Hand);
    assert!(!game.mana_spend_policy(alice, Some(spell)).mode.allows_any_type());
    let mut ctx = ironsmith::effects::EffectContext::new_default(star, alice);
    for effect in ability.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert!(game.mana_spend_policy(alice, Some(spell)).mode.allows_any_type());
    assert!(!game.mana_spend_policy(alice, Some(star)).mode.allows_any_type(), "not for activations");
    assert!(!game.mana_spend_policy(bob, Some(spell)).mode.allows_any_type(), "only its controller");
}

const P11_WHIPPOORWILL: &str = "Mana cost: {G}\nType: Creature — Bird\nPower/Toughness: 1/1\n{G}{G}, {T}: Target creature can't be regenerated this turn. Damage that would be dealt to that creature this turn can't be prevented or dealt instead to another permanent or player. When the creature dies this turn, exile the creature.";
const P11_SILHOUETTE: &str = "Mana cost: {1}{U}\nType: Instant\nChoose target creature. If a spell or ability that targets that creature would cause a source to deal damage to that creature this turn, prevent that damage.";

fn p11_bear(game: &mut GameState, controller: PlayerId) -> ironsmith::ObjectId {
    let bear = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Bear")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 4)).build();
    game.create_object_from_definition(&bear, controller, Zone::Battlefield)
}

/// Whippoorwill: a recipient-keyed "damage can't be prevented or dealt
/// instead to another permanent or player" rule on the target
/// (`Restriction::PreventDamageTo`), and the delayed "exile the creature"
/// binds the creature that died, not any creature.
#[test]
fn p11_whippoorwill_makes_damage_to_its_target_unpreventable_and_unredirectable() {
    use ironsmith::effect::{Restriction, Until};
    use ironsmith::effects::{CantEffect, EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::GameEventDamageTarget as DamageTarget;
    let definition = compile_to_runtime_definition("Whippoorwill", P11_WHIPPOORWILL, false)
        .expect("Whippoorwill should compile");
    let effects = p11_effects(&definition);
    let cants = effects.iter().filter_map(|effect| effect.downcast_ref::<CantEffect>()).collect::<Vec<_>>();
    assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::BeRegenerated(_))), "{cants:?}");
    assert!(cants.iter().any(|cant| matches!(cant.restriction,
        Restriction::PreventDamageTo { or_redirected: true, .. })), "{cants:?}");
    let schedule = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>())
        .expect("delayed death trigger");
    let body = format!("{:?}", schedule.effects);
    assert!(body.contains("ExileEffect") && body.contains("triggering"),
        "exile the creature that died: {body}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bird = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target = p11_bear(&mut game, bob);
    let other = p11_bear(&mut game, bob);
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("activated ability");
    let mut ctx = EffectContext::new_default(bird, alice).with_targets(vec![ResolvedTarget::Object(target)]);
    for effect in activated.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    game.update_cant_effects();
    assert!(!game.can_prevent_damage_to(DamageTarget::Object(target)));
    assert!(!game.can_redirect_damage_to(DamageTarget::Object(target)));
    assert!(game.can_prevent_damage_to(DamageTarget::Object(other)));
    assert!(game.can_redirect_damage_to(DamageTarget::Object(other)));

    // A prevention shield on each creature: only the untouched one is saved.
    for creature in [target, other] {
        let shield = ironsmith::effect::Effect::new(ironsmith::effects::PreventAllDamageToTargetEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(creature), Until::EndOfTurn));
        let mut ctx = EffectContext::new_default(bird, bob);
        execute_effect(&mut game, &shield, &mut ctx).unwrap();
    }
    for creature in [target, other] {
        let mut ctx = EffectContext::new_default(bird, alice);
        execute_effect(&mut game,
            &ironsmith::effect::Effect::deal_damage(1, ironsmith::target::ChooseSpec::SpecificObject(creature)),
            &mut ctx).unwrap();
    }
    assert_eq!(game.damage_on(target), 1, "prevention can't apply to Whippoorwill's target");
    assert_eq!(game.damage_on(other), 0);
}

/// Silhouette: a shield on the chosen creature that prevents only damage
/// caused by a spell or ability that targets it (`EventCause::targeted_objects`).
#[test]
fn p11_silhouette_prevents_only_damage_from_spells_and_abilities_targeting_it() {
    use ironsmith::effects::{EffectContext, PreventAllDamageToTargetEffect, ResolvedTarget, execute_effect};
    let definition = compile_to_runtime_definition("Silhouette", P11_SILHOUETTE, false)
        .expect("Silhouette should compile");
    let effects = p11_effects(&definition);
    let shield = effects.iter()
        .find_map(|effect| effect.downcast_ref::<PreventAllDamageToTargetEffect>())
        .expect("a prevention shield on that creature");
    assert!(shield.damage_filter.cause_targets_protected, "{shield:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let bear = p11_bear(&mut game, alice);
    let mut ctx = EffectContext::new_default(spell, alice).with_targets(vec![ResolvedTarget::Object(bear)]);
    for effect in definition.spell_effect.as_ref().expect("spell effects").all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);

    let burn = p11_bear(&mut game, bob);
    let damage = ironsmith::effect::Effect::deal_damage(1, ironsmith::target::ChooseSpec::SpecificObject(bear));
    // Damage from an effect that doesn't target the creature is dealt.
    let mut ctx = EffectContext::new_default(burn, bob);
    execute_effect(&mut game, &damage, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.damage_on(bear), 1);
    // Damage caused by an ability that targets it is prevented.
    let mut cause = ironsmith_core::EventCause::from_effect(burn, bob);
    cause.targeted_objects = vec![bear];
    let mut ctx = EffectContext::new_default(burn, bob).with_cause(cause);
    execute_effect(&mut game, &damage, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.damage_on(bear), 1, "targeting damage is prevented");
}

/// "until the end of defending player's next turn, that player gets two rad
/// counters whenever they cast a spell": a delayed spell-cast trigger whose
/// actor and duration are the defending player resolved at registration.
#[test]
fn nuka_nuke_launcher_watches_the_defending_player_until_their_next_turn_ends() {
    let definition = compile_to_runtime_definition("Nuka-Nuke Launcher",
        "Mana cost: {3}\nType: Artifact — Equipment\nEquipped creature gets +3/+0 and has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)\nWhenever equipped creature attacks, until the end of defending player's next turn, that player gets two rad counters whenever they cast a spell.\nEquip {3}", false)
        .expect("Nuka-Nuke Launcher should compile");
    let schedule = p11_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>().cloned())
        .expect("delayed spell-cast trigger");
    assert!(!schedule.one_shot);
    assert_eq!(schedule.duration, ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurnEnd);
    assert_eq!(schedule.duration_player, Some(ironsmith::target::PlayerFilter::Defending));
    let trigger = format!("{:?}", schedule.trigger);
    assert!(trigger.contains("SpellCast") && trigger.contains(ironsmith_core::DELAYED_DURATION_PLAYER_TAG), "{trigger}");
    let body = format!("{:?}", schedule.effects);
    assert!(body.contains("Rad") && body.contains("Fixed(2)"), "{body}");
}

const P11_SANWELL: &str = "Mana cost: {1}{W}\nType: Legendary Creature — Human Pilot\nPower/Toughness: 2/2\nAs long as an artifact creature you control is attacking, prevent all damage that would be dealt to Sanwell.\nWhenever Sanwell becomes tapped, exile the top six cards of your library. You may cast a Vehicle or artifact creature spell from among them. Then put the rest on the bottom of your library in a random order.";

/// "As long as an artifact creature you control is attacking" gates a
/// self-prevention static on an attacking artifact creature you control, and
/// the tapped trigger offers a choice of up to one Vehicle-or-artifact-creature
/// card among the exiled six (not "cast the first tagged card").
#[test]
fn p11_sanwell_conditional_prevention_and_filtered_cast_choice() {
    let definition = compile_to_runtime_definition("Sanwell, Avenger Ace", P11_SANWELL, false)
        .expect("Sanwell should compile");
    let statics = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(statics.contains("PreventAllDamageToSelf"), "{statics}");
    assert!(statics.contains("attacking: true"), "{statics}");
    assert!(statics.contains("Artifact"), "{statics}");

    let effects = p11_effects(&definition);
    let debug = effects.iter().map(|effect| format!("{effect:?}")).collect::<Vec<_>>().join("\n");
    assert!(debug.contains("ChooseObjects"), "the cast is a choice among the pool: {debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("Vehicle"), "{debug}");
    assert!(
        !debug.contains("card_types: [Artifact, Creature], all_card_types: [], card_type_count: None, excluded_card_types: [], subtypes: [Vehicle]"),
        "a Vehicle or an artifact creature, not an artifact-or-creature Vehicle: {debug}"
    );
}

const P11_RIA_IVOR: &str = "Mana cost: {2}{W}{B}\nType: Legendary Creature — Phyrexian Knight\nPower/Toughness: 3/4\nBattle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)\nAt the beginning of combat on your turn, the next time target creature would deal combat damage to one or more players this combat, prevent that damage. If damage is prevented this way, create that many 1/1 colorless Phyrexian Mite artifact creature tokens with toxic 1 and \"This token can't block.\"";

/// A next-time combat-damage shield keyed to the target creature as the
/// damage source and to any player as recipient, whose rider creates one
/// Mite per point of damage prevented.
#[test]
fn p11_ria_ivor_shields_player_combat_damage_and_creates_that_many_mites() {
    let definition = compile_to_runtime_definition("Ria Ivor, Bane of Bladehold", P11_RIA_IVOR, false)
        .expect("Ria Ivor should compile");
    let effects = p11_effects(&definition);
    let shield = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PreventNextTimeDamageEffect>())
        .expect("a next-time prevention shield");
    assert!(shield.combat_only);
    assert!(matches!(shield.target, ironsmith::effects::PreventNextTimeDamageTarget::AnyPlayer), "{shield:?}");
    assert!(matches!(shield.source, ironsmith::effects::PreventNextTimeDamageSource::Target(_)), "{shield:?}");
    let follow_ups = format!("{:?}", shield.follow_up_effects);
    assert!(follow_ups.contains("Mite"), "{follow_ups}");
    assert!(follow_ups.contains("EventValue(Amount)"), "one token per point prevented: {follow_ups}");
}
