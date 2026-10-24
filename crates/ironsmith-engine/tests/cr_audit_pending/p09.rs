//! CR audit (pending-entry pass, package p09): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{self, AttackTarget};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::{
    Ability, CardDefinition, CardId, CardType, ChooseSpec, DecisionMaker, Effect, EffectExecutor,
    LegalAction, ObjectFilter, ObjectId, Phase, PlayerFilter, PowerToughness, StaticAbility, Step,
    Subtype, Until, Zone,
};

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

fn p(i: u8) -> PlayerId {
    PlayerId::from_index(i)
}

fn new_game(players: usize) -> GameState {
    let names = ["Alice", "Bob", "Charlie", "Dave"];
    let mut game = GameState::new(
        names[..players].iter().map(|n| n.to_string()).collect(),
        20,
    );
    game.turn.active_player = p(0);
    game.turn.priority_player = Some(p(0));
    game.turn.phase = Phase::FirstMain;
    game
}

fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|err| panic!("fixture: strict compiler rejected {name}: {err:?}"))
}

fn creature_def(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build()
}

fn typed_creature_def(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn card_def(name: &str, types: Vec<CardType>, mv: u32) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(types.clone());
    if mv > 0 {
        builder = builder.mana_cost(ironsmith::ManaCost::from_symbols(vec![
            ironsmith::ManaSymbol::Generic(mv as u8),
        ]));
    }
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    builder.build()
}

fn add(game: &mut GameState, def: &CardDefinition, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_definition(def, owner, zone)
}

fn names(game: &GameState, ids: &[ObjectId]) -> Vec<String> {
    ids.iter()
        .filter_map(|id| game.object(*id).map(|o| o.name.to_string()))
        .collect()
}

fn hand(game: &GameState, player: PlayerId) -> Vec<String> {
    names(game, &game.player(player).unwrap().hand)
}

fn graveyard(game: &GameState, player: PlayerId) -> Vec<String> {
    names(game, &game.player(player).unwrap().graveyard)
}

/// Library names bottom-to-top (last element is the top card).
fn library(game: &GameState, player: PlayerId) -> Vec<String> {
    names(game, &game.player(player).unwrap().library)
}

fn battlefield_named(game: &GameState, name: &str) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|o| o.name == name))
        .collect()
}

/// Generic scripted decision maker used by most tests.
#[derive(Default)]
struct Dm {
    boolean: bool,
    /// Object names to pick in object selections (in addition to the minimum).
    picks: Vec<String>,
    /// Names sent to the secondary destination in partitions.
    partition: Vec<String>,
    /// Preferred option description substring.
    option_text: Option<String>,
    log: Vec<String>,
    object_calls: Vec<(PlayerId, String, Vec<(String, bool)>)>,
    partition_calls: Vec<(PlayerId, Vec<String>)>,
    boolean_calls: Vec<(PlayerId, String)>,
    view_calls: Vec<(PlayerId, Vec<String>)>,
}

impl DecisionMaker for Dm {
    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _ctx: &ironsmith::decisions::context::ViewCardsContext,
    ) {
        self.view_calls.push((viewer, names(game, cards)));
    }

    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.boolean_calls.push((ctx.player, ctx.description.clone()));
        self.log.push(format!("bool:{:?}:{}", ctx.player, ctx.description));
        self.boolean
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_calls.push((
            ctx.player,
            ctx.description.clone(),
            ctx.candidates
                .iter()
                .map(|c| (c.name.clone(), c.legal))
                .collect(),
        ));
        self.log.push(format!("objects:{:?}:{}:min{}:max{:?}", ctx.player, ctx.description, ctx.min, ctx.max));
        let max = ctx.max.unwrap_or(usize::MAX);
        let mut chosen: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal && self.picks.iter().any(|n| *n == c.name))
            .map(|c| c.id)
            .take(max)
            .collect();
        for c in ctx.candidates.iter().filter(|c| c.legal) {
            if chosen.len() >= ctx.min {
                break;
            }
            if !chosen.contains(&c.id) {
                chosen.push(c.id);
            }
        }
        chosen
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.log.push(format!(
            "options:{:?}:{}:{:?}",
            ctx.player,
            ctx.description,
            ctx.options
                .iter()
                .map(|o| o.description.clone())
                .collect::<Vec<_>>()
        ));
        if let Some(text) = &self.option_text
            && let Some(o) = ctx
                .options
                .iter()
                .find(|o| o.legal && o.description.to_lowercase().contains(&text.to_lowercase()))
        {
            return vec![o.index];
        }
        if ctx.min == 0 {
            return Vec::new();
        }
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min)
            .map(|o| o.index)
            .collect()
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        self.partition_calls.push((
            ctx.player,
            ctx.cards.iter().map(|(_, n)| n.clone()).collect(),
        ));
        self.log.push(format!("partition:{:?}:{}", ctx.player, ctx.description));
        ctx.cards
            .iter()
            .filter(|(_, n)| self.partition.contains(n))
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Put an ability with `effects` on the stack and resolve it with `dm`.
fn resolve_effects(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: Vec<Effect>,
    targets: Vec<Target>,
    dm: &mut dyn DecisionMaker,
) {
    game.stack
        .push(StackEntry::ability(source, controller, effects).with_targets(targets));
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).expect("fixture: resolution");
}

/// Put `def` on the stack as a spell controlled by `caster` and resolve it.
fn resolve_spell(
    game: &mut GameState,
    def: &CardDefinition,
    caster: PlayerId,
    targets: Vec<Target>,
    dm: &mut dyn DecisionMaker,
) -> ObjectId {
    let spell = add(game, def, caster, Zone::Stack);
    game.stack
        .push(StackEntry::new(spell, caster).with_targets(targets));
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).expect("fixture: resolution");
    spell
}

fn drain(game: &mut GameState) -> TriggerQueue {
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(game, &mut queue)
        .expect("fixture: drain triggers");
    queue
}

fn source_artifact(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = card_def("Effect source", vec![CardType::Artifact], 0);
    add(game, &def, owner, Zone::Battlefield)
}

fn complete_priority_action<D: DecisionMaker>(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .unwrap_or(0);
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 701.2a Activate
// ---------------------------------------------------------------------------

#[test]
fn cr_701_2a_only_controller_with_priority_can_activate() {
    let def = compile(
        "Activation probe",
        "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life.",
    );
    let mut game = new_game(2);
    let obj = add(&mut game, &def, p(0), Zone::Battlefield);
    let is_activation = |a: &LegalAction| {
        matches!(a, LegalAction::ActivateAbility { source, .. } if *source == obj)
    };
    let alice_actions = ironsmith::compute_legal_actions(&game, p(0)).unwrap();
    let action = alice_actions
        .iter()
        .find(|a| is_activation(a))
        .cloned()
        .expect("CR 701.2a: the controller with priority may activate its ability");
    game.turn.priority_player = Some(p(1));
    let bob_actions = ironsmith::compute_legal_actions(&game, p(1)).unwrap();
    assert!(
        !bob_actions.iter().any(is_activation),
        "CR 701.2a: only the object's controller can activate its activated ability"
    );
    game.turn.priority_player = Some(p(0));
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    complete_priority_action(&mut game, action, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "CR 701.2a: activating puts the ability on the stack");
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(p(0)).unwrap().life, 21);
}

// ---------------------------------------------------------------------------
// 701.3d Unattach / becomes unattached
// ---------------------------------------------------------------------------

#[test]
fn cr_701_3d_unattach_and_leaving_count_as_becoming_unattached() {
    use ironsmith::effects::{DestroyEffect, UnattachObjectsEffect};
    use ironsmith::object::AttachmentTarget;
    let mut failures = Vec::new();
    for case in 0..3 {
        let mut game = new_game(2);
        let observer = CardDefinitionBuilder::new(CardId::new(), "Unattach observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::new(ironsmith::triggers::AttachmentChangedTrigger {
                    attachment: ObjectFilter::artifact(),
                    recipient: ObjectFilter::creature(),
                    attached: false,
                }),
                vec![Effect::gain_life(1)],
            ))
            .build();
        add(&mut game, &observer, p(0), Zone::Battlefield);
        let equipment = CardDefinitionBuilder::new(CardId::new(), "Test Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        let eq = add(&mut game, &equipment, p(0), Zone::Battlefield);
        let bear = add(&mut game, &creature_def("Bear", 2, 2), p(0), Zone::Battlefield);
        assert!(game.attach_object_to_target(eq, AttachmentTarget::Object(bear)));
        drain(&mut game);
        let src = source_artifact(&mut game, p(0));
        let mut ctx = EffectContext::new_default(src, p(0));
        match case {
            0 => {
                UnattachObjectsEffect::new(ChooseSpec::SpecificObject(eq))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
            }
            1 => {
                DestroyEffect::with_spec(ChooseSpec::SpecificObject(eq))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
            }
            _ => {
                DestroyEffect::with_spec(ChooseSpec::SpecificObject(bear))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
            }
        }
        let queue = drain(&mut game);
        if queue.entries.len() != 1 {
            failures.push(format!(
                "case {case}: {} becomes-unattached triggers, expected 1",
                queue.entries.len()
            ));
        }
        if case == 0 {
            let o = game.object(eq).unwrap();
            if o.zone != Zone::Battlefield || o.attached_to.is_some() {
                failures.push(format!(
                    "case 0: equipment zone {:?} attached_to {:?}",
                    o.zone, o.attached_to
                ));
            }
            if !game.object(bear).unwrap().attachments.is_empty() {
                failures.push("case 0: creature still lists the equipment".into());
            }
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.3d: unattaching or either object leaving is becoming unattached: {failures:?}"
    );
}

// ---------------------------------------------------------------------------
// 701.4 Behold
// ---------------------------------------------------------------------------

#[test]
fn cr_701_4a_behold_offers_hand_cards_and_controlled_permanents_of_the_quality() {
    use ironsmith::effects::BeholdEffect;
    let mut game = new_game(2);
    add(&mut game, &typed_creature_def("Field Dragon", Subtype::Dragon), p(0), Zone::Battlefield);
    add(&mut game, &typed_creature_def("Hand Dragon", Subtype::Dragon), p(0), Zone::Hand);
    add(&mut game, &typed_creature_def("Field Elf", Subtype::Elf), p(0), Zone::Battlefield);
    add(&mut game, &typed_creature_def("Hand Elf", Subtype::Elf), p(0), Zone::Hand);
    add(&mut game, &typed_creature_def("Enemy Dragon", Subtype::Dragon), p(1), Zone::Battlefield);
    add(&mut game, &typed_creature_def("Grave Dragon", Subtype::Dragon), p(0), Zone::Graveyard);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        picks: vec!["Hand Dragon".into()],
        ..Dm::default()
    };
    let mut ctx = EffectContext::new_default(src, p(0)).with_decision_maker(&mut dm);
    BeholdEffect::you(Subtype::Dragon, 1)
        .execute(&mut game, &mut ctx)
        .unwrap();
    drop(ctx);
    assert_eq!(dm.object_calls.len(), 1, "fixture: one behold choice");
    let mut legal: Vec<String> = dm.object_calls[0]
        .2
        .iter()
        .filter(|(_, l)| *l)
        .map(|(n, _)| n.clone())
        .collect();
    legal.sort();
    assert_eq!(
        legal,
        vec!["Field Dragon".to_string(), "Hand Dragon".to_string()],
        "CR 701.4a: behold a Dragon = reveal a Dragon card from your hand or choose a Dragon permanent you control"
    );
    assert!(
        hand(&game, p(0)).contains(&"Hand Dragon".to_string()),
        "CR 701.4a/701.20b: revealing the beheld card leaves it in hand"
    );
}

#[test]
fn cr_701_4b_was_beheld_is_locked_in_when_the_behold_happens() {
    use ironsmith::effects::DestroyEffect;
    let def = compile(
        "Behold payoff probe",
        "Mana cost: {0}\nType: Sorcery\nAs an additional cost to cast this spell, you may behold a Dragon. (You may choose a Dragon you control or reveal a Dragon card from your hand.)\nYou gain 1 life. If a Dragon was beheld, you gain 2 life.",
    );
    let mut game = new_game(2);
    let dragon = add(&mut game, &typed_creature_def("Field Dragon", Subtype::Dragon), p(0), Zone::Battlefield);
    let spell = add(&mut game, &def, p(0), Zone::Hand);
    let action = ironsmith::compute_legal_actions(&game, p(0))
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("fixture: spell castable");
    let mut dm = Dm {
        boolean: true,
        picks: vec!["Field Dragon".into()],
        option_text: Some("behold".into()),
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    complete_priority_action(&mut game, action, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "fixture: spell on stack; log {:?}", dm.log);
    // The beheld permanent stops having the quality (it leaves) before resolution.
    let src = source_artifact(&mut game, p(1));
    let mut ctx = EffectContext::new_default(src, p(1));
    DestroyEffect::with_spec(ChooseSpec::SpecificObject(dragon))
        .execute(&mut game, &mut ctx)
        .unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(
        game.player(p(0)).unwrap().life,
        23,
        "CR 701.4b: 'if a Dragon was beheld' uses the quality when beheld, not on resolution (log {:?})",
        dm.log
    );
}

// ---------------------------------------------------------------------------
// 701.7 Create
// ---------------------------------------------------------------------------

#[test]
fn cr_701_7a_create_puts_the_specified_tokens_onto_the_battlefield() {
    let def = compile(
        "Token maker probe",
        "Mana cost: {0}\nType: Sorcery\nCreate two 1/1 white Soldier creature tokens.",
    );
    let mut game = new_game(2);
    let mut dm = Dm::default();
    resolve_spell(&mut game, &def, p(1), vec![], &mut dm);
    let tokens: Vec<ObjectId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().kind == ironsmith::ObjectKind::Token)
        .collect();
    assert_eq!(tokens.len(), 2, "CR 701.7a: create two tokens");
    for t in tokens {
        let o = game.object(t).unwrap();
        assert_eq!(game.controller_of(o), p(1), "CR 701.7a: tokens enter under the creator's control");
        assert!(game.current_is_creature(t));
        assert!(game.current_has_subtype(t, Subtype::Soldier), "CR 701.7a: specified characteristics");
    }
}

#[test]
fn cr_701_7b_token_creation_replacements_precede_and_entry_replacements_follow_continuous_effects() {
    let march = compile(
        "March of the Machines",
        "Mana cost: {3}{U}\nType: Enchantment\nEach noncreature artifact is an artifact creature with power and toughness each equal to its mana value.",
    );
    let visitation = compile(
        "Divine Visitation",
        "Mana cost: {3}{W}{W}\nType: Enchantment\nIf one or more creature tokens would be created under your control, that many 4/4 white Angel creature tokens with flying are created instead.",
    );
    let sunwing = compile(
        "Kinjalli's Sunwing",
        "Mana cost: {2}{W}\nType: Creature — Dinosaur\nPower/Toughness: 2/3\nFlying\nCreatures your opponents control enter tapped.",
    );
    let investigate = compile("Investigate probe", "Mana cost: {0}\nType: Sorcery\nInvestigate.");
    let mut failures = Vec::new();
    // (a) creation replacement: a Clue is not a creature token when created, so
    // Divine Visitation does not apply even though March would make it a creature.
    {
        let mut game = new_game(2);
        add(&mut game, &march, p(0), Zone::Battlefield);
        add(&mut game, &visitation, p(0), Zone::Battlefield);
        let mut dm = Dm::default();
        resolve_spell(&mut game, &investigate, p(0), vec![], &mut dm);
        let clues = battlefield_named(&game, "Clue Token").len();
        let angels = battlefield_named(&game, "Angel").len();
        if clues != 1 || angels != 0 {
            failures.push(format!("creation replacement: clues {clues}, angels {angels}"));
        }
    }
    // (b) entry replacement: the Clue enters as a creature under March, so
    // Sunwing's "creatures your opponents control enter tapped" applies.
    {
        let mut game = new_game(2);
        add(&mut game, &march, p(1), Zone::Battlefield);
        add(&mut game, &sunwing, p(0), Zone::Battlefield);
        let mut dm = Dm::default();
        resolve_spell(&mut game, &investigate, p(1), vec![], &mut dm);
        let clues = battlefield_named(&game, "Clue Token");
        if clues.len() != 1 || !game.is_tapped(clues[0]) {
            failures.push(format!(
                "entry replacement: clues {}, tapped {:?}",
                clues.len(),
                clues.first().map(|c| game.is_tapped(*c))
            ));
        }
    }
    assert!(failures.is_empty(), "CR 701.7b: {failures:?}");
}

#[test]
fn cr_701_16a_investigate_creates_a_clue_token() {
    let def = compile("Investigate probe", "Mana cost: {0}\nType: Sorcery\nInvestigate.");
    let mut game = new_game(2);
    let mut dm = Dm::default();
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    let clues = battlefield_named(&game, "Clue Token");
    assert_eq!(clues.len(), 1, "CR 701.16a: investigate creates one Clue");
    let clue = game.object(clues[0]).unwrap();
    assert_eq!(clue.kind, ironsmith::ObjectKind::Token);
    assert!(game.current_has_subtype(clues[0], Subtype::Clue), "CR 111.10f: Clue subtype");
    assert!(
        clue.abilities.iter().any(|a| matches!(a.kind, ironsmith::AbilityKind::Activated(_))),
        "CR 111.10f: Clue has '{{2}}, Sacrifice this token: Draw a card.'"
    );
}

// ---------------------------------------------------------------------------
// 701.8 Destroy
// ---------------------------------------------------------------------------

#[test]
fn cr_701_8a_destroy_moves_to_owners_graveyard() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DestroyEffect};
    let mut game = new_game(2);
    let bear = add(&mut game, &creature_def("Borrowed Bear", 2, 2), p(1), Zone::Battlefield);
    let src = source_artifact(&mut game, p(0));
    let mut ctx = EffectContext::new_default(src, p(0));
    ApplyContinuousEffect::new(
        EffectTarget::Specific(bear),
        Modification::ChangeController(p(0)),
        Until::Forever,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(game.controller_of(game.object(bear).unwrap()), p(0));
    DestroyEffect::with_spec(ChooseSpec::SpecificObject(bear))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(battlefield_named(&game, "Borrowed Bear").is_empty());
    assert_eq!(
        graveyard(&game, p(1)),
        vec!["Borrowed Bear".to_string()],
        "CR 701.8a: destroyed permanent goes to its owner's graveyard"
    );
    assert!(graveyard(&game, p(0)).is_empty());
}

#[test]
fn cr_701_8b_c_sacrifice_is_not_destruction_but_regeneration_replaces_destroy() {
    use ironsmith::effects::{DestroyEffect, RegenerateEffect, SacrificeTargetEffect};
    let mut failures = Vec::new();
    for case in 0..3 {
        let mut game = new_game(2);
        let def = if case == 1 {
            CardDefinitionBuilder::new(CardId::new(), "Subject")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .with_ability(Ability::static_ability(StaticAbility::indestructible()))
                .build()
        } else {
            creature_def("Subject", 2, 2)
        };
        let subject = add(&mut game, &def, p(0), Zone::Battlefield);
        let src = source_artifact(&mut game, p(0));
        let mut ctx = EffectContext::new_default(src, p(0));
        RegenerateEffect::new(ChooseSpec::SpecificObject(subject), Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let expected_on_battlefield = case == 2;
        if case == 2 {
            DestroyEffect::with_spec(ChooseSpec::SpecificObject(subject))
                .execute(&mut game, &mut ctx)
                .unwrap();
        } else {
            SacrificeTargetEffect::new(ChooseSpec::SpecificObject(subject))
                .execute(&mut game, &mut ctx)
                .unwrap();
        }
        let on_bf = !battlefield_named(&game, "Subject").is_empty();
        if on_bf != expected_on_battlefield {
            failures.push(format!("case {case}: on battlefield {on_bf}, expected {expected_on_battlefield}"));
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.8b/701.8c: sacrifice is not destruction (shield/indestructible don't apply); regeneration replaces destroy: {failures:?}"
    );
}

#[test]
fn cr_701_19b_static_regeneration_replaces_every_destruction() {
    use ironsmith::effects::{DealDamageEffect, DestroyEffect};
    let def = compile(
        "Mossbridge Troll",
        "Mana cost: {5}{G}{G}\nType: Creature — Troll\nPower/Toughness: 5/5\nIf Mossbridge Troll would be destroyed, regenerate it.",
    );
    let mut game = new_game(2);
    let troll = add(&mut game, &def, p(0), Zone::Battlefield);
    let src = source_artifact(&mut game, p(1));
    let mut failures = Vec::new();
    for round in 0..2 {
        game.untap(troll);
        let mut ctx = EffectContext::new_default(src, p(1));
        DealDamageEffect::new(2, ChooseSpec::SpecificObject(troll))
            .execute(&mut game, &mut ctx)
            .unwrap();
        if round == 1 {
            game.turn.active_player = p(0);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let mut combat = combat_state::new_combat();
            combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(troll, AttackTarget::Player(p(1)))],
            )
            .expect("fixture: troll attacks");
            game.combat = Some(combat);
        }
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(troll))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let on_bf = game.object(troll).is_some_and(|o| o.zone == Zone::Battlefield);
        if !on_bf {
            failures.push(format!("round {round}: troll left the battlefield"));
            break;
        }
        if !game.is_tapped(troll) {
            failures.push(format!("round {round}: not tapped"));
        }
        if game.damage_on(troll) != 0 {
            failures.push(format!("round {round}: damage {}", game.damage_on(troll)));
        }
        if round == 1
            && game
                .combat
                .as_ref()
                .is_some_and(|c| combat_state::is_attacking(c, troll))
        {
            failures.push("round 1: still attacking".into());
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.19b: static regeneration replaces each destruction (remove damage, tap, remove from combat): {failures:?}"
    );
}

// ---------------------------------------------------------------------------
// 701.9 Discard
// ---------------------------------------------------------------------------

#[test]
fn cr_701_9a_b_discard_defaults_to_the_discarding_players_choice() {
    use ironsmith::effects::DiscardEffect;
    let mut game = new_game(2);
    for n in ["Card A", "Card B", "Card C"] {
        add(&mut game, &card_def(n, vec![CardType::Sorcery], 1), p(1), Zone::Hand);
    }
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        picks: vec!["Card B".into()],
        ..Dm::default()
    };
    let mut ctx = EffectContext::new_default(src, p(0)).with_decision_maker(&mut dm);
    DiscardEffect::new_with_filter(1, PlayerFilter::Opponent, false, None)
        .execute(&mut game, &mut ctx)
        .unwrap();
    drop(ctx);
    assert_eq!(
        dm.object_calls.first().map(|c| c.0),
        Some(p(1)),
        "CR 701.9b: the affected player chooses the discarded card"
    );
    assert_eq!(graveyard(&game, p(1)), vec!["Card B".to_string()], "CR 701.9a: hand to owner's graveyard");
    assert_eq!(hand(&game, p(1)).len(), 2);
}

#[test]
fn cr_701_9b_random_discard_asks_no_one_and_other_player_choice_uses_that_player() {
    use ironsmith::effects::DiscardEffect;
    let mut failures = Vec::new();
    // Random discard.
    {
        let mut game = new_game(2);
        for n in ["Card A", "Card B", "Card C"] {
            add(&mut game, &card_def(n, vec![CardType::Sorcery], 1), p(0), Zone::Hand);
        }
        let src = source_artifact(&mut game, p(0));
        let mut dm = Dm::default();
        let mut ctx = EffectContext::new_default(src, p(0)).with_decision_maker(&mut dm);
        DiscardEffect::new_with_filter(1, PlayerFilter::You, true, None)
            .execute(&mut game, &mut ctx)
            .unwrap();
        drop(ctx);
        if !dm.object_calls.is_empty() {
            failures.push(format!("random discard asked for a choice: {:?}", dm.object_calls));
        }
        if graveyard(&game, p(0)).len() != 1 || hand(&game, p(0)).len() != 2 {
            failures.push("random discard did not move exactly one card".into());
        }
    }
    // Another player chooses (Thoughtseize-style).
    {
        let def = compile(
            "Duress-like probe",
            "Mana cost: {0}\nType: Sorcery\nTarget opponent reveals their hand. You choose a nonland card from it. That player discards that card.",
        );
        let mut game = new_game(2);
        for n in ["Card A", "Card B"] {
            add(&mut game, &card_def(n, vec![CardType::Sorcery], 1), p(1), Zone::Hand);
        }
        let mut dm = Dm {
            picks: vec!["Card B".into()],
            ..Dm::default()
        };
        resolve_spell(&mut game, &def, p(0), vec![Target::Player(p(1))], &mut dm);
        if dm.object_calls.first().map(|c| c.0) != Some(p(0)) {
            failures.push(format!("chooser was {:?}", dm.object_calls.first().map(|c| c.0)));
        }
        if graveyard(&game, p(1)) != vec!["Card B".to_string()] {
            failures.push(format!("opponent graveyard {:?}", graveyard(&game, p(1))));
        }
    }
    assert!(failures.is_empty(), "CR 701.9b: {failures:?}");
}

#[test]
fn cr_701_9c_discard_into_hidden_zone_cannot_pay_a_characteristic_requirement() {
    let leng = compile(
        "Library of Leng",
        "Mana cost: {1}\nType: Artifact\nYou have no maximum hand size.\nIf an effect causes you to discard a card, you may put it on top of your library instead of into your graveyard.",
    );
    let mox = compile(
        "Mox Diamond",
        "Mana cost: {0}\nType: Artifact\nIf Mox Diamond would enter, you may discard a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't, put it into its owner's graveyard.\n{T}: Add one mana of any color.",
    );
    let mut game = new_game(2);
    add(&mut game, &leng, p(0), Zone::Battlefield);
    add(&mut game, &card_def("Forest", vec![CardType::Land], 0), p(0), Zone::Hand);
    let mut dm = Dm {
        boolean: true,
        picks: vec!["Forest".into()],
        option_text: Some("Top of library".into()),
        ..Dm::default()
    };
    resolve_spell(&mut game, &mox, p(0), vec![], &mut dm);
    let mox_on_bf = !battlefield_named(&game, "Mox Diamond").is_empty();
    let forest_in_library = library(&game, p(0)).contains(&"Forest".to_string());
    assert!(
        !(mox_on_bf && forest_in_library),
        "CR 701.9c: a land discarded face-down into the library has undefined characteristics, so it cannot pay Mox Diamond's 'discard a land card' (log {:?})",
        dm.log
    );
}

// ---------------------------------------------------------------------------
// 701.15 Goad
// ---------------------------------------------------------------------------

fn goad_fixture(players: usize) -> (GameState, ObjectId) {
    let mut game = new_game(players);
    game.turn.turn_number = 1;
    let goaded = add(&mut game, &creature_def("Goaded Bear", 2, 2), p(1), Zone::Battlefield);
    (game, goaded)
}

fn goad_by(game: &mut GameState, creature: ObjectId, goader: PlayerId) {
    use ironsmith::effects::GoadEffect;
    let src = source_artifact(game, goader);
    let mut ctx = EffectContext::new_default(src, goader);
    GoadEffect::new(ChooseSpec::SpecificObject(creature))
        .execute(game, &mut ctx)
        .unwrap();
}

fn try_attack(game: &GameState, attacker: ObjectId, target: Option<PlayerId>) -> bool {
    let mut game = game.clone();
    game.turn.active_player = game.controller_of(game.object(attacker).unwrap());
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = combat_state::new_combat();
    let decls = target
        .map(|t| vec![(attacker, AttackTarget::Player(t))])
        .unwrap_or_default();
    combat_state::declare_attackers(&mut game, &mut combat, decls).is_ok()
}

#[test]
fn cr_701_15a_b_goaded_creature_must_attack_a_non_goader_until_goaders_next_turn() {
    let (mut game, bear) = goad_fixture(3);
    goad_by(&mut game, bear, p(0));
    game.turn.turn_number = 2;
    game.turn.active_player = p(1);
    let results = [
        ("no attack", try_attack(&game, bear, None), false),
        ("attack goader", try_attack(&game, bear, Some(p(0))), false),
        ("attack other", try_attack(&game, bear, Some(p(2))), true),
    ];
    for (label, actual, expected) in results {
        assert_eq!(
            actual, expected,
            "CR 701.15b: goaded creature attacks each combat and attacks a non-goader if able ({label})"
        );
    }
    game.turn.turn_number = 3;
    game.turn.active_player = p(2);
    assert!(game.is_goaded(bear), "CR 701.15a: still goaded during another player's turn");
    game.turn.turn_number = 4;
    game.turn.active_player = p(0);
    assert!(
        !game.is_goaded(bear),
        "CR 701.15a: goad ends at the goading spell/ability controller's next turn"
    );
}

#[test]
fn cr_701_15b_goaded_is_not_copiable() {
    let (mut game, bear) = goad_fixture(3);
    goad_by(&mut game, bear, p(0));
    let src = source_artifact(&mut game, p(1));
    let mut dm = Dm::default();
    resolve_effects(
        &mut game,
        src,
        p(1),
        vec![Effect::create_token_copy(ChooseSpec::SpecificObject(bear))],
        vec![],
        &mut dm,
    );
    let copies: Vec<ObjectId> = battlefield_named(&game, "Goaded Bear")
        .into_iter()
        .filter(|id| *id != bear)
        .collect();
    assert_eq!(copies.len(), 1, "fixture: one token copy");
    assert!(game.is_goaded(bear));
    assert!(
        !game.is_goaded(copies[0]),
        "CR 701.15b: goaded is not part of copiable values"
    );
}

#[test]
fn cr_701_15c_multiple_goaders_create_additional_requirements() {
    let (mut game, bear) = goad_fixture(4);
    goad_by(&mut game, bear, p(0));
    goad_by(&mut game, bear, p(2));
    game.turn.turn_number = 2;
    game.turn.active_player = p(1);
    let results = [
        ("no attack", try_attack(&game, bear, None), false),
        ("attack first goader", try_attack(&game, bear, Some(p(0))), false),
        ("attack second goader", try_attack(&game, bear, Some(p(2))), false),
        ("attack non-goader", try_attack(&game, bear, Some(p(3))), true),
    ];
    for (label, actual, expected) in results {
        assert_eq!(
            actual, expected,
            "CR 701.15c: each goading player adds requirements ({label})"
        );
    }
}

#[test]
fn cr_701_15d_same_player_goading_again_adds_no_requirement() {
    let (mut game, bear) = goad_fixture(3);
    goad_by(&mut game, bear, p(0));
    goad_by(&mut game, bear, p(0));
    goad_by(&mut game, bear, p(2));
    game.turn.turn_number = 2;
    game.turn.active_player = p(1);
    // With one goad from each of Alice and Charlie, attacking either obeys the
    // same number of requirements. A duplicated Alice goad would forbid attacking Alice.
    assert!(
        try_attack(&game, bear, Some(p(0))),
        "CR 701.15d: re-goading by the same player creates no extra requirement"
    );
    assert!(try_attack(&game, bear, Some(p(2))));
}

// ---------------------------------------------------------------------------
// Batch 2 helpers
// ---------------------------------------------------------------------------

/// Compile real rules text, asserting support with a CR-citing message.
fn compile_cr(rule: &str, name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false);
    assert!(
        def.is_ok(),
        "CR {rule}: engine must support {name}: {:?}",
        def.as_ref().err()
    );
    let def = def.unwrap();
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "CR {rule}: {name} compiled with unimplemented content"
    );
    def
}

/// Put cards into `owner`'s library; the last name ends up on top.
fn stock_library(game: &mut GameState, owner: PlayerId, cards: &[(&str, u32)]) -> Vec<ObjectId> {
    cards
        .iter()
        .map(|(n, mv)| add(game, &card_def(n, vec![CardType::Sorcery], *mv), owner, Zone::Library))
        .collect()
}

fn keyword_observer(game: &mut GameState, owner: PlayerId, trigger: Trigger) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Keyword observer")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(trigger, vec![Effect::gain_life(1)]))
        .build();
    add(game, &def, owner, Zone::Battlefield)
}

fn trigger_count(game: &mut GameState) -> usize {
    drain(game).entries.len()
}

fn basic_land(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .supertypes(vec![ironsmith::Supertype::Basic])
        .subtypes(vec![subtype])
        .build()
}

// ---------------------------------------------------------------------------
// 701.22 Scry
// ---------------------------------------------------------------------------

#[test]
fn cr_701_22a_scry_looks_at_top_n_and_splits_between_bottom_and_top() {
    let mut game = new_game(2);
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1), ("L3", 1), ("L4", 1), ("L5", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        partition: vec!["L4".into()],
        ..Dm::default()
    };
    resolve_effects(&mut game, src, p(0), vec![Effect::scry(3)], vec![], &mut dm);
    assert_eq!(dm.partition_calls.len(), 1, "fixture: one scry partition");
    let mut seen = dm.partition_calls[0].1.clone();
    seen.sort();
    assert_eq!(seen, vec!["L3", "L4", "L5"], "CR 701.22a: scry 3 looks at the top three cards");
    let lib = library(&game, p(0));
    assert_eq!(lib[0], "L4", "CR 701.22a: chosen card goes to the bottom");
    assert_eq!(&lib[1..3], &["L1".to_string(), "L2".to_string()]);
    let mut top: Vec<String> = lib[3..].to_vec();
    top.sort();
    assert_eq!(top, vec!["L3", "L5"], "CR 701.22a: the rest stay on top");
}

#[test]
fn cr_701_22b_d_scry_zero_is_no_event_but_empty_library_scry_still_triggers() {
    use ironsmith::events::KeywordActionKind;
    let mut results = Vec::new();
    for (count, library_size, expected) in [(0, 3, 0), (2, 0, 1), (2, 3, 1)] {
        let mut game = new_game(2);
        keyword_observer(&mut game, p(0), Trigger::keyword_action(KeywordActionKind::Scry, PlayerFilter::You));
        for i in 0..library_size {
            stock_library(&mut game, p(0), &[(&format!("C{i}"), 1)]);
        }
        let src = source_artifact(&mut game, p(0));
        let mut dm = Dm::default();
        resolve_effects(&mut game, src, p(0), vec![Effect::scry(count)], vec![], &mut dm);
        let n = trigger_count(&mut game);
        if n != expected {
            results.push(format!("scry {count} with {library_size} cards: {n} triggers, expected {expected}"));
        }
    }
    assert!(
        results.is_empty(),
        "CR 701.22b/701.22d: scry 0 doesn't trigger; scry with an impossible look still triggers: {results:?}"
    );
}

#[test]
fn cr_701_22c_native_each_player_scry_decides_in_apnap_order() {
    use ironsmith::effects::EachPlayerScryEffect;
    let mut game = new_game(2);
    game.turn.active_player = p(1);
    stock_library(&mut game, p(0), &[("A1", 1), ("A2", 1)]);
    stock_library(&mut game, p(1), &[("B1", 1), ("B2", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        partition: vec!["A2".into(), "B2".into()],
        ..Dm::default()
    };
    resolve_effects(
        &mut game,
        src,
        p(0),
        vec![Effect::new(EachPlayerScryEffect::new(1, PlayerFilter::Any))],
        vec![],
        &mut dm,
    );
    let order: Vec<PlayerId> = dm.partition_calls.iter().map(|c| c.0).collect();
    assert_eq!(
        order,
        vec![p(1), p(0)],
        "CR 701.22c: simultaneous scry decisions are made in APNAP order (log {:?})",
        dm.log
    );
    assert_eq!(library(&game, p(0))[0], "A2");
    assert_eq!(library(&game, p(1))[0], "B2");
}

// ---------------------------------------------------------------------------
// 701.25 Surveil
// ---------------------------------------------------------------------------

#[test]
fn cr_701_25a_surveil_splits_between_graveyard_and_top() {
    let mut game = new_game(2);
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1), ("L3", 1), ("L4", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        partition: vec!["L3".into()],
        ..Dm::default()
    };
    resolve_effects(&mut game, src, p(0), vec![Effect::surveil(2)], vec![], &mut dm);
    let mut seen = dm.partition_calls.first().map(|c| c.1.clone()).unwrap_or_default();
    seen.sort();
    assert_eq!(seen, vec!["L3", "L4"], "CR 701.25a: surveil 2 looks at the top two");
    assert_eq!(graveyard(&game, p(0)), vec!["L3".to_string()], "CR 701.25a: chosen cards to graveyard");
    assert_eq!(library(&game, p(0)), vec!["L1", "L2", "L4"], "CR 701.25a: rest on top");
}

#[test]
fn cr_701_25b_extra_looked_cards_join_the_surveil() {
    let enhanced = compile_cr(
        "701.25b",
        "Enhanced Surveillance",
        "Mana cost: {1}{U}\nType: Enchantment\nYou may look at an additional two cards each time you surveil.",
    );
    let mut game = new_game(2);
    add(&mut game, &enhanced, p(0), Zone::Battlefield);
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1), ("L3", 1), ("L4", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        boolean: true,
        partition: vec!["L2".into()],
        ..Dm::default()
    };
    resolve_effects(&mut game, src, p(0), vec![Effect::surveil(1)], vec![], &mut dm);
    let mut seen = dm.partition_calls.first().map(|c| c.1.clone()).unwrap_or_default();
    seen.sort();
    assert_eq!(
        seen,
        vec!["L2", "L3", "L4"],
        "CR 701.25b: additional looked-at cards are included in the surveil (log {:?})",
        dm.log
    );
    assert_eq!(graveyard(&game, p(0)), vec!["L2".to_string()], "CR 701.25b");
}

#[test]
fn cr_701_25c_d_surveil_zero_is_no_event_but_empty_library_surveil_still_triggers() {
    use ironsmith::events::KeywordActionKind;
    let mut results = Vec::new();
    for (count, library_size, expected) in [(0, 3, 0), (2, 0, 1), (2, 3, 1)] {
        let mut game = new_game(2);
        keyword_observer(&mut game, p(0), Trigger::keyword_action(KeywordActionKind::Surveil, PlayerFilter::You));
        for i in 0..library_size {
            stock_library(&mut game, p(0), &[(&format!("C{i}"), 1)]);
        }
        let src = source_artifact(&mut game, p(0));
        let mut dm = Dm::default();
        resolve_effects(&mut game, src, p(0), vec![Effect::surveil(count)], vec![], &mut dm);
        let n = trigger_count(&mut game);
        if n != expected {
            results.push(format!("surveil {count} with {library_size} cards: {n} triggers, expected {expected}"));
        }
    }
    assert!(
        results.is_empty(),
        "CR 701.25c/701.25d: surveil 0 doesn't trigger; impossible surveil still triggers: {results:?}"
    );
}

// ---------------------------------------------------------------------------
// 701.29 Fateseal
// ---------------------------------------------------------------------------

#[test]
fn cr_701_29a_fateseal_arranges_the_opponents_library() {
    let mut game = new_game(2);
    stock_library(&mut game, p(0), &[("A1", 1), ("A2", 1)]);
    stock_library(&mut game, p(1), &[("B1", 1), ("B2", 1), ("B3", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm {
        partition: vec!["B3".into()],
        ..Dm::default()
    };
    resolve_effects(&mut game, src, p(0), vec![Effect::fateseal(2)], vec![], &mut dm);
    assert_eq!(dm.partition_calls.len(), 1, "fixture: one fateseal decision");
    assert_eq!(dm.partition_calls[0].0, p(0), "CR 701.29a: the fatesealing player decides");
    let mut seen = dm.partition_calls[0].1.clone();
    seen.sort();
    assert_eq!(seen, vec!["B2", "B3"], "CR 701.29a: top N of an opponent's library");
    assert_eq!(library(&game, p(1)), vec!["B3", "B1", "B2"], "CR 701.29a: bottom/top of that library");
    assert_eq!(library(&game, p(0)), vec!["A1", "A2"]);
}

// ---------------------------------------------------------------------------
// 701.30 Clash
// ---------------------------------------------------------------------------

fn clash_case(alice_mv: u32, bob_mv: u32, active: PlayerId) -> (GameState, Dm) {
    let def = compile_cr(
        "701.30",
        "Clash probe",
        "Mana cost: {0}\nType: Sorcery\nClash with an opponent. If you win, you gain 3 life.",
    );
    let mut game = new_game(2);
    game.turn.active_player = active;
    stock_library(&mut game, p(0), &[("A under", 1), ("A top", alice_mv)]);
    stock_library(&mut game, p(1), &[("B under", 1), ("B top", bob_mv)]);
    let mut dm = Dm {
        partition: vec!["A top".into(), "B top".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    (game, dm)
}

#[test]
fn cr_701_30a_d_clash_reveals_top_cards_and_higher_mana_value_wins() {
    let mut failures = Vec::new();
    for (a, b, wins) in [(3, 2, true), (2, 2, false), (1, 4, false), (0, 0, false)] {
        let (game, _dm) = clash_case(a, b, p(0));
        let life = game.player(p(0)).unwrap().life;
        if (life == 23) != wins {
            failures.push(format!("A mv{a} vs B mv{b}: life {life}"));
        }
        if library(&game, p(0))[0] != "A top" || library(&game, p(1))[0] != "B top" {
            failures.push(format!(
                "A mv{a} vs B mv{b}: revealed cards not put on bottom: {:?} {:?}",
                library(&game, p(0)),
                library(&game, p(1))
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.30a/701.30d: reveal top card, may bottom it; win only with strictly higher mana value: {failures:?}"
    );
}

#[test]
fn cr_701_30c_clash_bottom_decisions_follow_apnap_order() {
    let (_game, dm) = clash_case(3, 2, p(1));
    let order: Vec<PlayerId> = dm.partition_calls.iter().map(|c| c.0).collect();
    assert_eq!(
        order,
        vec![p(1), p(0)],
        "CR 701.30c: clashing players decide where to put their cards in APNAP order (log {:?})",
        dm.log
    );
}

#[test]
fn cr_701_30b_clash_with_an_opponent_involves_only_you_and_the_chosen_opponent() {
    let def = compile_cr(
        "701.30b",
        "Clash probe",
        "Mana cost: {0}\nType: Sorcery\nClash with an opponent. If you win, you gain 3 life.",
    );
    let mut game = new_game(3);
    stock_library(&mut game, p(0), &[("A top", 3)]);
    stock_library(&mut game, p(1), &[("B top", 1)]);
    stock_library(&mut game, p(2), &[("C top", 1)]);
    let mut dm = Dm {
        option_text: Some("Charlie".into()),
        partition: vec!["A top".into(), "B top".into(), "C top".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    let mut deciders: Vec<PlayerId> = dm.partition_calls.iter().map(|c| c.0).collect();
    deciders.sort_by_key(|p| p.index());
    assert_eq!(
        deciders,
        vec![p(0), p(2)],
        "CR 701.30b: you and the chosen opponent each clash (log {:?})",
        dm.log
    );
    assert_eq!(game.player(p(0)).unwrap().life, 23);
}

// ---------------------------------------------------------------------------
// 701.23 Search
// ---------------------------------------------------------------------------

#[test]
fn cr_701_23c_searching_for_an_undefined_quality_finds_nothing() {
    let lobotomy = compile_cr(
        "701.23c",
        "Lobotomy",
        "Mana cost: {2}{U}{B}\nType: Sorcery\nTarget player reveals their hand, then you choose a card other than a basic land card from it. Search that player's graveyard, hand, and library for all cards with the same name as the chosen card and exile them. Then that player shuffles.",
    );
    let mut game = new_game(2);
    stock_library(&mut game, p(1), &[("Same", 1), ("Same", 1), ("Other", 1)]);
    add(&mut game, &card_def("Same", vec![CardType::Sorcery], 1), p(1), Zone::Graveyard);
    let mut dm = Dm::default();
    resolve_spell(&mut game, &lobotomy, p(0), vec![Target::Player(p(1))], &mut dm);
    assert_eq!(library(&game, p(1)).len(), 3, "CR 701.23c: nothing is found in the library");
    assert_eq!(graveyard(&game, p(1)), vec!["Same".to_string()], "CR 701.23c: nothing found in the graveyard");
    assert!(game.exile.is_empty(), "CR 701.23c: no cards exiled");
}

#[test]
fn cr_701_23d_quantity_search_must_find_that_many_cards() {
    let tutor = compile_cr(
        "701.23d",
        "Demonic Tutor",
        "Mana cost: {1}{B}\nType: Sorcery\nSearch your library for a card, put that card into your hand, then shuffle.",
    );
    let mut game = new_game(2);
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1), ("L3", 1)]);
    let mut dm = Dm::default();
    resolve_spell(&mut game, &tutor, p(0), vec![], &mut dm);
    assert_eq!(
        hand(&game, p(0)).len(),
        1,
        "CR 701.23d: searching for 'a card' must find a card when one exists (calls {:?})",
        dm.object_calls
    );
    assert_eq!(library(&game, p(0)).len(), 2);
}

#[test]
fn cr_701_23e_found_cards_are_revealed_only_when_instructed() {
    let mut counts = Vec::new();
    for reveal in [false, true] {
        let text = if reveal {
            "Mana cost: {0}\nType: Sorcery\nSearch your library for a card, reveal it, put it into your hand, then shuffle."
        } else {
            "Mana cost: {0}\nType: Sorcery\nSearch your library for a card, put that card into your hand, then shuffle."
        };
        let def = compile_cr("701.23e", "Search reveal probe", text);
        let mut game = new_game(2);
        keyword_observer(
            &mut game,
            p(1),
            Trigger::player_reveals_card(PlayerFilter::Any, ObjectFilter::default(), false),
        );
        stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1)]);
        let mut dm = Dm::default();
        resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
        counts.push(trigger_count(&mut game));
    }
    assert_eq!(
        counts,
        vec![0, 1],
        "CR 701.23e: a found card is revealed only if the effect says to reveal it"
    );
}

#[test]
fn cr_701_23f_partial_search_still_searches_the_library_and_triggers() {
    let mindcensor = compile_cr(
        "701.23f",
        "Aven Mindcensor",
        "Mana cost: {2}{W}\nType: Creature — Bird Wizard\nPower/Toughness: 2/1\nFlash\nFlying\nIf an opponent would search a library, that player searches the top four cards of that library instead.",
    );
    let tutor = compile("Demonic Tutor", "Mana cost: {0}\nType: Sorcery\nSearch your library for a card, put that card into your hand, then shuffle.");
    let mut game = new_game(2);
    add(&mut game, &mindcensor, p(0), Zone::Battlefield);
    keyword_observer(&mut game, p(0), Trigger::player_searches_library(PlayerFilter::Any));
    stock_library(
        &mut game,
        p(1),
        &[("L1", 1), ("L2", 1), ("L3", 1), ("L4", 1), ("L5", 1), ("L6", 1), ("L7", 1), ("L8", 1)],
    );
    let mut dm = Dm::default();
    resolve_spell(&mut game, &tutor, p(1), vec![], &mut dm);
    let searched: Vec<String> = dm
        .object_calls
        .iter()
        .find(|c| c.0 == p(1))
        .map(|c| {
            let mut v: Vec<String> = c.2.iter().filter(|(_, l)| *l).map(|(n, _)| n.clone()).collect();
            v.sort();
            v
        })
        .unwrap_or_default();
    assert_eq!(searched, vec!["L5", "L6", "L7", "L8"], "fixture/CR 701.23f: searches the top four");
    assert_eq!(
        trigger_count(&mut game),
        1,
        "CR 701.23f: abilities that trigger on a library being searched still trigger"
    );
}

#[test]
fn cr_701_23g_may_search_even_when_follow_up_actions_are_impossible() {
    let def = compile_cr(
        "701.23g",
        "Optional search probe",
        "Mana cost: {0}\nType: Sorcery\nYou may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    );
    let mut game = new_game(2);
    keyword_observer(&mut game, p(1), Trigger::player_searches_library(PlayerFilter::Any));
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1)]);
    let mut dm = Dm {
        boolean: true,
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    assert_eq!(
        trigger_count(&mut game),
        1,
        "CR 701.23g: the player may choose to search even with nothing to find (log {:?})",
        dm.log
    );
}

#[test]
fn cr_701_23h_two_search_instructions_before_shuffle_are_one_search() {
    let def = compile_cr(
        "701.23h",
        "Krosan Verge search",
        "Mana cost: {0}\nType: Sorcery\nSearch your library for a Forest card and a Plains card, put them onto the battlefield tapped, then shuffle.",
    );
    let mut game = new_game(2);
    keyword_observer(&mut game, p(1), Trigger::player_searches_library(PlayerFilter::Any));
    add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Library);
    add(&mut game, &basic_land("Plains", Subtype::Plains), p(0), Zone::Library);
    stock_library(&mut game, p(0), &[("L1", 1)]);
    let mut dm = Dm {
        picks: vec!["Forest".into(), "Plains".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    assert_eq!(battlefield_named(&game, "Forest").len(), 1, "fixture: Forest found");
    assert_eq!(
        trigger_count(&mut game),
        1,
        "CR 701.23h: the player searches that library only once"
    );
}

#[test]
fn cr_701_23i_simultaneous_searches_choose_in_apnap_order() {
    let def = compile_cr(
        "701.23i",
        "Veteran Explorer effect",
        "Mana cost: {0}\nType: Sorcery\nEach player may search their library for up to two basic land cards, put them onto the battlefield, then shuffle.",
    );
    let mut game = new_game(2);
    game.turn.active_player = p(1);
    for owner in [p(0), p(1)] {
        add(&mut game, &basic_land("Forest", Subtype::Forest), owner, Zone::Library);
        stock_library(&mut game, owner, &[("Filler", 1)]);
    }
    let mut dm = Dm {
        boolean: true,
        picks: vec!["Forest".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    let order: Vec<PlayerId> = dm.object_calls.iter().map(|c| c.0).collect();
    assert_eq!(battlefield_named(&game, "Forest").len(), 2, "fixture: both players found a Forest (log {:?})", dm.log);
    assert_eq!(
        order,
        vec![p(1), p(0)],
        "CR 701.23i: simultaneous searchers decide in APNAP order"
    );
}

#[test]
fn cr_701_23j_searching_outside_the_game_chooses_an_owned_appropriate_card() {
    let wish = compile_cr(
        "701.23j",
        "Living Wish",
        "Mana cost: {1}{G}\nType: Sorcery\nYou may reveal a creature or land card you own from outside the game and put it into your hand. Exile Living Wish.",
    );
    let mut game = new_game(2);
    add(&mut game, &creature_def("Owned Sideboard Creature", 2, 2), p(0), Zone::OutsideGame);
    add(&mut game, &card_def("Owned Sideboard Sorcery", vec![CardType::Sorcery], 1), p(0), Zone::OutsideGame);
    add(&mut game, &creature_def("Opponent Sideboard Creature", 2, 2), p(1), Zone::OutsideGame);
    let mut dm = Dm {
        boolean: true,
        picks: vec!["Owned Sideboard Creature".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &wish, p(0), vec![], &mut dm);
    let legal: Vec<String> = dm
        .object_calls
        .iter()
        .flat_map(|c| c.2.iter().filter(|(_, l)| *l).map(|(n, _)| n.clone()))
        .collect();
    assert_eq!(
        legal,
        vec!["Owned Sideboard Creature".to_string()],
        "CR 701.23j: only appropriate cards the player owns outside the game (log {:?})",
        dm.log
    );
    assert_eq!(hand(&game, p(0)), vec!["Owned Sideboard Creature".to_string()]);
}

// ---------------------------------------------------------------------------
// 701.24 Shuffle
// ---------------------------------------------------------------------------

fn shuffle_observer(game: &mut GameState, owner: PlayerId) {
    keyword_observer(
        game,
        owner,
        Trigger::player_shuffles_library(PlayerFilter::Any, false, false),
    );
}

#[test]
fn cr_701_24b_found_card_is_excluded_from_the_shuffle_and_shuffle_still_triggers() {
    let vampiric = compile_cr(
        "701.24b",
        "Vampiric Tutor",
        "Mana cost: {B}\nType: Instant\nSearch your library for a card, then shuffle and put that card on top. You lose 2 life.",
    );
    let mut game = new_game(2);
    shuffle_observer(&mut game, p(1));
    stock_library(&mut game, p(0), &[("L1", 1), ("Wanted", 1), ("L3", 1), ("L4", 1), ("L5", 1), ("L6", 1)]);
    let mut dm = Dm {
        picks: vec!["Wanted".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &vampiric, p(0), vec![], &mut dm);
    let lib = library(&game, p(0));
    assert_eq!(lib.last().map(String::as_str), Some("Wanted"), "CR 701.24b: found card ends on top");
    assert_eq!(lib.len(), 6);
    assert_eq!(trigger_count(&mut game), 1, "CR 701.24b: shuffle triggers still trigger");
}

#[test]
fn cr_701_24d_shuffling_an_empty_set_into_a_library_still_shuffles() {
    let def = compile_cr(
        "701.24d",
        "Loaming Shaman effect",
        "Mana cost: {0}\nType: Sorcery\nTarget player shuffles any number of target cards from their graveyard into their library.",
    );
    let mut game = new_game(2);
    shuffle_observer(&mut game, p(1));
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1)]);
    add(&mut game, &card_def("Grave card", vec![CardType::Sorcery], 1), p(0), Zone::Graveyard);
    let mut dm = Dm::default();
    resolve_spell(&mut game, &def, p(0), vec![Target::Player(p(0))], &mut dm);
    assert!(graveyard(&game, p(0)).contains(&"Grave card".to_string()), "fixture: untargeted card stays");
    assert_eq!(
        trigger_count(&mut game),
        1,
        "CR 701.24d: the library is shuffled even if no objects are shuffled into it"
    );
}

fn darksteel() -> CardDefinition {
    compile_cr(
        "701.24",
        "Darksteel Colossus",
        "Mana cost: {11}\nType: Artifact Creature — Golem\nPower/Toughness: 11/11\nTrample, indestructible\nIf Darksteel Colossus would be put into a graveyard from anywhere, reveal Darksteel Colossus and shuffle it into its owner's library instead.",
    )
}

#[test]
fn cr_701_24f_simultaneous_shuffles_trigger_once_each() {
    let mut game = new_game(2);
    shuffle_observer(&mut game, p(1));
    let colossus = darksteel();
    let a = add(&mut game, &colossus, p(0), Zone::Hand);
    let b = add(&mut game, &colossus, p(0), Zone::Hand);
    stock_library(&mut game, p(0), &[("L1", 1), ("L2", 1)]);
    drain(&mut game);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm::default();
    resolve_effects(
        &mut game,
        src,
        p(0),
        vec![Effect::discard_hand()],
        vec![],
        &mut dm,
    );
    let _ = (a, b);
    assert_eq!(library(&game, p(0)).len(), 4, "fixture: both Colossi shuffled in");
    assert_eq!(
        trigger_count(&mut game),
        2,
        "CR 701.24f: two simultaneous shuffles of the library trigger twice"
    );
}

#[test]
fn cr_701_24g_shuffle_and_positioned_card_at_once_keep_the_position() {
    let zombie = compile_cr(
        "701.24g",
        "Gravebane Zombie",
        "Mana cost: {3}{B}\nType: Creature — Zombie\nPower/Toughness: 3/2\nIf this creature would die, put it on top of its owner's library instead.",
    );
    let colossus = CardDefinitionBuilder::new(CardId::new(), "Destroyable Colossus").build();
    let _ = colossus;
    let colossus = compile_cr(
        "701.24g",
        "Shuffling Golem",
        "Mana cost: {3}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3\nIf Shuffling Golem would be put into a graveyard from anywhere, reveal Shuffling Golem and shuffle it into its owner's library instead.",
    );
    let mut failures = Vec::new();
    for variant in 0..3 {
        let mut game = new_game(2);
        let names: Vec<String> = (0..12).map(|i| format!("F{variant}_{i}")).collect();
        for n in &names {
            stock_library(&mut game, p(0), &[(n.as_str(), 1)]);
        }
        if variant % 2 == 0 {
            add(&mut game, &zombie, p(0), Zone::Battlefield);
            add(&mut game, &colossus, p(0), Zone::Battlefield);
        } else {
            add(&mut game, &colossus, p(0), Zone::Battlefield);
            add(&mut game, &zombie, p(0), Zone::Battlefield);
        }
        let src = source_artifact(&mut game, p(1));
        let mut dm = Dm::default();
        resolve_effects(
            &mut game,
            src,
            p(1),
            vec![Effect::destroy_all(ObjectFilter::creature())],
            vec![],
            &mut dm,
        );
        let lib = library(&game, p(0));
        if lib.last().map(String::as_str) != Some("Gravebane Zombie") || !lib.contains(&"Shuffling Golem".to_string()) {
            failures.push(format!("variant {variant}: top {:?}, len {}", lib.last(), lib.len()));
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.24g: simultaneous shuffle and top placement leaves the placed card on top: {failures:?}"
    );
}

// ---------------------------------------------------------------------------
// 701.27 Transform / 701.28 Convert
// ---------------------------------------------------------------------------

fn dfc_pair(
    front_name: &str,
    back_name: &str,
    back_types: Vec<CardType>,
    front_subtypes: Vec<Subtype>,
    front_abilities: Vec<Ability>,
) -> (CardDefinition, CardDefinition) {
    use ironsmith::card::LinkedFaceLayout;
    let front_id = CardId::new();
    let back_id = CardId::new();
    let mut fb = CardDefinitionBuilder::new(front_id, front_name)
        .card_types(vec![CardType::Creature])
        .subtypes(front_subtypes)
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(back_id)
        .other_face_name(back_name)
        .linked_face_layout(LinkedFaceLayout::TransformLike);
    for a in front_abilities {
        fb = fb.with_ability(a);
    }
    let mut front = fb.build();
    let mut bb = CardDefinitionBuilder::new(back_id, back_name)
        .card_types(back_types.clone())
        .other_face(front_id)
        .other_face_name(front_name)
        .linked_face_layout(LinkedFaceLayout::TransformLike);
    if back_types.contains(&CardType::Creature) {
        bb = bb.power_toughness(PowerToughness::fixed(4, 4));
    }
    let mut back = bb.build();
    front.card.transforming_dfc = true;
    back.card.transforming_dfc = true;
    (front, back)
}

fn dfc_on_battlefield(
    game: &mut GameState,
    back_types: Vec<CardType>,
    front_subtypes: Vec<Subtype>,
    front_abilities: Vec<Ability>,
) -> ObjectId {
    let (front, back) = dfc_pair("Front Face", "Back Face", back_types, front_subtypes, front_abilities);
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    add(game, &front, p(0), Zone::Battlefield)
}

fn face_name(game: &GameState, id: ObjectId) -> String {
    game.object(id).map(|o| o.name.to_string()).unwrap_or_default()
}

fn run_transform_like(game: &mut GameState, target: ObjectId, convert: bool) {
    let src = source_artifact(game, p(0));
    let effect = if convert {
        Effect::convert(ChooseSpec::SpecificObject(target))
    } else {
        Effect::transform(ChooseSpec::SpecificObject(target))
    };
    let mut dm = Dm::default();
    resolve_effects(game, src, p(0), vec![effect], vec![], &mut dm);
}

#[test]
fn cr_701_27a_28a_transform_and_convert_turn_a_dfc_to_its_other_face() {
    let mut failures = Vec::new();
    for convert in [false, true] {
        let mut game = new_game(2);
        let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![], vec![]);
        run_transform_like(&mut game, id, convert);
        if face_name(&game, id) != "Back Face" {
            failures.push(format!("convert={convert}: first action shows {}", face_name(&game, id)));
        }
        run_transform_like(&mut game, id, convert);
        if face_name(&game, id) != "Front Face" {
            failures.push(format!("convert={convert}: second action shows {}", face_name(&game, id)));
        }
    }
    assert!(failures.is_empty(), "CR 701.27a/701.28a: {failures:?}");
}

#[test]
fn cr_701_27b_28b_transforming_or_converting_is_not_turning_face_up_or_down() {
    let mut failures = Vec::new();
    for convert in [false, true] {
        let mut game = new_game(2);
        keyword_observer(
            &mut game,
            p(1),
            Trigger::player_turns_face_up(PlayerFilter::Any, ObjectFilter::default()),
        );
        let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![], vec![]);
        drain(&mut game);
        run_transform_like(&mut game, id, convert);
        let n = trigger_count(&mut game);
        if face_name(&game, id) != "Back Face" || n != 0 {
            failures.push(format!("convert={convert}: face {} face-up triggers {n}", face_name(&game, id)));
        }
    }
    assert!(failures.is_empty(), "CR 701.27b/701.28b: {failures:?}");
}

#[test]
fn cr_701_27c_d_28c_d_non_dfc_or_instant_sorcery_face_does_nothing() {
    let mut failures = Vec::new();
    for convert in [false, true] {
        // Not a double-faced card.
        let mut game = new_game(2);
        let plain = add(&mut game, &creature_def("Plain Bear", 2, 2), p(0), Zone::Battlefield);
        run_transform_like(&mut game, plain, convert);
        if face_name(&game, plain) != "Plain Bear" || game.transform_count(plain) != 0 {
            failures.push(format!("convert={convert}: non-DFC changed"));
        }
        // Back face is a sorcery.
        let mut game = new_game(2);
        let id = dfc_on_battlefield(&mut game, vec![CardType::Sorcery], vec![], vec![]);
        run_transform_like(&mut game, id, convert);
        if face_name(&game, id) != "Front Face" || game.transform_count(id) != 0 {
            failures.push(format!("convert={convert}: sorcery back face shows {}", face_name(&game, id)));
        }
    }
    assert!(failures.is_empty(), "CR 701.27c/d, 701.28c/d: {failures:?}");
}

#[test]
fn cr_701_27e_transforms_into_triggers_on_transform_and_on_convert() {
    let mut counts = Vec::new();
    for convert in [false, true] {
        let mut game = new_game(2);
        keyword_observer(
            &mut game,
            p(0),
            Trigger::permanent_transforms_into(
                ObjectFilter::permanent().you_control(),
                ObjectFilter::creature(),
            ),
        );
        let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![], vec![]);
        drain(&mut game);
        run_transform_like(&mut game, id, convert);
        assert_eq!(face_name(&game, id), "Back Face", "fixture: face changed");
        counts.push(trigger_count(&mut game));
    }
    assert_eq!(
        counts,
        vec![1, 1],
        "CR 701.27e: a 'transforms into' ability triggers when the object transforms or converts"
    );
}

#[test]
fn cr_701_27f_28e_self_transform_ability_is_ignored_if_already_transformed() {
    let mut failures = Vec::new();
    for convert in [false, true] {
        let effect = if convert {
            Effect::convert(ChooseSpec::Source)
        } else {
            Effect::transform(ChooseSpec::Source)
        };
        let ability = Ability::activated(ironsmith::TotalCost::free(), vec![effect]);
        let mut game = new_game(2);
        let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![], vec![ability]);
        for _ in 0..2 {
            let action = ironsmith::compute_legal_actions(&game, p(0))
                .unwrap()
                .into_iter()
                .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == id))
                .expect("fixture: self-transform ability activatable");
            let mut dm = Dm::default();
            let mut queue = TriggerQueue::new();
            complete_priority_action(&mut game, action, &mut dm, &mut queue).unwrap();
            game.turn.priority_player = Some(p(0));
        }
        assert_eq!(game.stack.len(), 2, "fixture: two activations on the stack");
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        if face_name(&game, id) != "Back Face" || game.transform_count(id) != 1 {
            failures.push(format!(
                "convert={convert}: face {} after two activations, count {}",
                face_name(&game, id),
                game.transform_count(id)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.27f/701.28e: a permanent's own ability transforms it only if it hasn't transformed since the ability was put on the stack: {failures:?}"
    );
}

#[test]
fn cr_701_27f_28e_delayed_self_transform_is_ignored_if_transformed_since_creation() {
    use ironsmith::effects::ScheduleDelayedTriggerEffect;
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let mut failures = Vec::new();
    for convert in [false, true] {
        for intervening in [false, true] {
            let mut game = new_game(2);
            let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![], vec![]);
            let delayed_effect = if convert {
                Effect::convert(ChooseSpec::Source)
            } else {
                Effect::transform(ChooseSpec::Source)
            };
            let mut dm = Dm::default();
            resolve_effects(
                &mut game,
                id,
                p(0),
                vec![Effect::new(ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(PlayerFilter::Any),
                    vec![delayed_effect],
                    true,
                    vec![],
                    PlayerFilter::You,
                ))],
                vec![],
                &mut dm,
            );
            if intervening {
                run_transform_like(&mut game, id, convert);
            }
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
            let mut queue = TriggerQueue::new();
            let _ = runner.advance(&mut game, &mut queue).unwrap();
            if queue.entries.len() != 1 {
                failures.push(format!("convert={convert} intervening={intervening}: {} delayed triggers", queue.entries.len()));
                continue;
            }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let expected_count = 1;
            let expected_face = "Back Face";
            if game.transform_count(id) != expected_count || face_name(&game, id) != expected_face {
                failures.push(format!(
                    "convert={convert} intervening={intervening}: face {}, count {}",
                    face_name(&game, id),
                    game.transform_count(id)
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "CR 701.27f/701.28e: a delayed trigger transforms its permanent only if it hasn't transformed or converted since the delayed trigger was created: {failures:?}"
    );
}

#[test]
fn cr_701_28f_cant_transform_also_means_cant_convert() {
    let immerwolf = compile_cr(
        "701.28f",
        "Immerwolf",
        "Mana cost: {1}{R}{G}\nType: Creature — Wolf\nPower/Toughness: 2/2\nNon-Human Werewolves you control can't transform.",
    );
    let mut failures = Vec::new();
    for convert in [false, true] {
        let mut game = new_game(2);
        add(&mut game, &immerwolf, p(0), Zone::Battlefield);
        let id = dfc_on_battlefield(&mut game, vec![CardType::Creature], vec![Subtype::Werewolf], vec![]);
        run_transform_like(&mut game, id, convert);
        if face_name(&game, id) != "Front Face" {
            failures.push(format!("convert={convert}: face {}", face_name(&game, id)));
        }
    }
    assert!(failures.is_empty(), "CR 701.28f: a permanent that can't transform can't convert: {failures:?}");
}

// ---------------------------------------------------------------------------
// 701.5 Cast
// ---------------------------------------------------------------------------

#[test]
fn cr_701_5a_casting_moves_the_card_to_the_stack_and_pays_costs() {
    let def = compile(
        "Cast probe",
        "Mana cost: {0}\nType: Sorcery\nYou gain 2 life.",
    );
    let mut game = new_game(2);
    let card = add(&mut game, &def, p(0), Zone::Hand);
    let action = ironsmith::compute_legal_actions(&game, p(0))
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .expect("CR 701.5a: a player with priority may cast a spell");
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    complete_priority_action(&mut game, action, &mut dm, &mut queue).unwrap();
    assert!(hand(&game, p(0)).is_empty(), "CR 701.5a: the card leaves its zone");
    assert_eq!(game.stack.len(), 1, "CR 701.5a: the spell is on the stack");
    let spell = game.stack[0].object_id;
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(p(0)).unwrap().life, 22, "CR 701.5a: it resolves and has its effect");
}

// ---------------------------------------------------------------------------
// 701.18 Play
// ---------------------------------------------------------------------------

fn can_play_land(game: &GameState, player: PlayerId, land: ObjectId) -> bool {
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == land))
}

#[test]
fn cr_701_18a_playing_a_land_is_a_timed_special_action_and_effects_dont_count() {
    let mut failures = Vec::new();
    // Normal play; doesn't use the stack; only one per turn.
    {
        let mut game = new_game(2);
        let forest = add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Hand);
        let forest2 = add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Hand);
        if !can_play_land(&game, p(0), forest) {
            failures.push("main phase, empty stack: land not playable".to_string());
        } else {
            let mut dm = Dm::default();
            let mut queue = TriggerQueue::new();
            complete_priority_action(&mut game, LegalAction::PlayLand { land_id: forest }, &mut dm, &mut queue).unwrap();
            if battlefield_named(&game, "Forest").len() != 1 || !game.stack.is_empty() {
                failures.push("playing a land used the stack or failed".into());
            }
            game.turn.priority_player = Some(p(0));
            if can_play_land(&game, p(0), forest2) {
                failures.push("second land playable in the same turn".into());
            }
        }
    }
    // Non-empty stack, non-main phase, opponent's turn.
    for case in 0..3 {
        let mut game = new_game(2);
        let owner = if case == 2 { p(1) } else { p(0) };
        let forest = add(&mut game, &basic_land("Forest", Subtype::Forest), owner, Zone::Hand);
        match case {
            0 => {
                let src = source_artifact(&mut game, p(1));
                game.stack.push(StackEntry::ability(src, p(1), vec![Effect::gain_life(1)]));
            }
            1 => {
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
            }
            _ => {
                game.turn.priority_player = Some(p(1));
            }
        }
        if can_play_land(&game, owner, forest) {
            failures.push(format!("restricted case {case}: land playable"));
        }
    }
    // Putting a land onto the battlefield with an effect isn't playing a land.
    {
        let put = compile(
            "Put land probe",
            "Mana cost: {0}\nType: Sorcery\nYou may put a land card from your hand onto the battlefield.",
        );
        let mut game = new_game(2);
        add(&mut game, &basic_land("Plains", Subtype::Plains), p(0), Zone::Hand);
        let forest = add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Hand);
        let mut dm = Dm {
            boolean: true,
            picks: vec!["Plains".into()],
            ..Dm::default()
        };
        resolve_spell(&mut game, &put, p(0), vec![], &mut dm);
        if battlefield_named(&game, "Plains").len() != 1 {
            failures.push(format!("fixture: effect did not put the land (log {:?})", dm.log));
        } else if !can_play_land(&game, p(0), forest) {
            failures.push("land put onto battlefield by an effect consumed the land play".into());
        }
    }
    assert!(failures.is_empty(), "CR 701.18a: {failures:?}");
}

#[test]
fn cr_701_18b_play_permission_allows_land_play_or_spell_cast() {
    let future = compile_cr(
        "701.18b",
        "Future Sight",
        "Mana cost: {2}{U}{U}{U}\nType: Enchantment\nPlay with the top card of your library revealed.\nYou may play lands and cast spells from the top of your library.",
    );
    let mut failures = Vec::new();
    for land_on_top in [true, false] {
        let mut game = new_game(2);
        add(&mut game, &future, p(0), Zone::Battlefield);
        stock_library(&mut game, p(0), &[("Filler", 1)]);
        let top = if land_on_top {
            add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Library)
        } else {
            add(&mut game, &compile("Free spell", "Mana cost: {0}\nType: Sorcery\nYou gain 1 life."), p(0), Zone::Library)
        };
        let actions = ironsmith::compute_legal_actions(&game, p(0)).unwrap();
        let ok = actions.iter().any(|a| match a {
            LegalAction::PlayLand { land_id } => land_on_top && *land_id == top,
            LegalAction::CastSpell { spell_id, .. } => !land_on_top && *spell_id == top,
            _ => false,
        });
        if !ok {
            failures.push(format!("land_on_top={land_on_top}: top card not playable"));
        }
    }
    assert!(failures.is_empty(), "CR 701.18b: playing a card means playing it as a land or casting it: {failures:?}");
}

// ---------------------------------------------------------------------------
// 701.20 Reveal / look
// ---------------------------------------------------------------------------

#[test]
fn cr_701_20b_revealing_does_not_move_the_card() {
    let mut game = new_game(2);
    keyword_observer(
        &mut game,
        p(1),
        Trigger::player_reveals_card(PlayerFilter::Any, ObjectFilter::default(), false),
    );
    stock_library(&mut game, p(0), &[("Under", 1), ("Top", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm::default();
    resolve_effects(&mut game, src, p(0), vec![Effect::reveal_top(PlayerFilter::You, "revealed")], vec![], &mut dm);
    assert_eq!(trigger_count(&mut game), 1, "fixture: the card was revealed");
    assert_eq!(library(&game, p(0)), vec!["Under", "Top"], "CR 701.20b: the revealed card stays where it was");
}

#[test]
fn cr_701_20c_an_already_revealed_card_can_be_revealed_again_for_a_cost() {
    let telepathy = compile_cr(
        "701.20c",
        "Telepathy",
        "Mana cost: {U}\nType: Enchantment\nYour opponents play with their hands revealed.",
    );
    let adept = compile_cr(
        "701.20c",
        "Silvergill Adept",
        "Mana cost: {0}\nType: Creature — Merfolk Wizard\nPower/Toughness: 2/1\nAs an additional cost to cast this spell, reveal a Merfolk card from your hand or pay {3}.\nWhen this creature enters, draw a card.",
    );
    let mut game = new_game(2);
    add(&mut game, &telepathy, p(1), Zone::Battlefield);
    add(&mut game, &typed_creature_def("Revealed Merfolk", Subtype::Merfolk), p(0), Zone::Hand);
    let spell = add(&mut game, &adept, p(0), Zone::Hand);
    let action = ironsmith::compute_legal_actions(&game, p(0))
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
    assert!(
        action.is_some(),
        "CR 701.20c: an already revealed Merfolk card may pay the reveal cost (no mana available)"
    );
    let mut dm = Dm {
        boolean: true,
        picks: vec!["Revealed Merfolk".into()],
        option_text: Some("reveal".into()),
        ..Dm::default()
    };
    let mut queue = TriggerQueue::new();
    let result = complete_priority_action(&mut game, action.unwrap(), &mut dm, &mut queue);
    assert!(
        result.is_ok() && game.stack.len() == 1,
        "CR 701.20c: casting by revealing an already revealed card succeeds: {result:?} (log {:?})",
        dm.log
    );
    assert!(hand(&game, p(0)).contains(&"Revealed Merfolk".to_string()));
}

#[test]
fn cr_701_20d_revealed_library_cards_become_new_objects_when_shuffled() {
    let mut game = new_game(2);
    let ids = stock_library(&mut game, p(0), &[("C1", 1), ("C2", 1), ("C3", 1), ("Top", 1)]);
    let top = ids[3];
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm::default();
    resolve_effects(
        &mut game,
        src,
        p(0),
        vec![Effect::reveal_top(PlayerFilter::You, "revealed"), Effect::shuffle_library()],
        vec![],
        &mut dm,
    );
    assert_eq!(library(&game, p(0)).len(), 4, "fixture: library intact");
    let same_object = game.object(top).is_some_and(|o| o.zone == Zone::Library)
        && game.player(p(0)).unwrap().library.contains(&top);
    assert!(
        !same_object,
        "CR 701.20d: a revealed card that is shuffled stops being revealed and becomes a new object (old object id still identifies it)"
    );
}

#[test]
fn cr_701_20e_looking_shows_cards_only_to_the_specified_player() {
    let mut game = new_game(2);
    keyword_observer(
        &mut game,
        p(1),
        Trigger::player_reveals_card(PlayerFilter::Any, ObjectFilter::default(), false),
    );
    stock_library(&mut game, p(0), &[("Under", 1), ("Top", 1)]);
    let src = source_artifact(&mut game, p(0));
    let mut dm = Dm::default();
    resolve_effects(
        &mut game,
        src,
        p(0),
        vec![Effect::look_at_top_cards(PlayerFilter::You, 1, "looked")],
        vec![],
        &mut dm,
    );
    assert_eq!(trigger_count(&mut game), 0, "CR 701.20e: looking is not revealing to all players");
    assert_eq!(library(&game, p(0)), vec!["Under", "Top"], "CR 701.20e/701.20b: the card does not move");
    assert!(
        dm.view_calls.iter().all(|(viewer, _)| *viewer == p(0)),
        "CR 701.20e: only the specified player sees the card: {:?}",
        dm.view_calls
    );
}

#[test]
fn cr_701_23h_single_search_for_a_forest_and_a_plains_finds_both() {
    // Krosan Verge's search: the combined "for a Forest card and a Plains card" instruction is
    // the single-search form 701.23h equates multiple search instructions with.
    let def = compile_cr(
        "701.23h",
        "Krosan Verge combined search",
        "Mana cost: {0}\nType: Sorcery\nSearch your library for a Forest card and a Plains card, put them onto the battlefield tapped, then shuffle.",
    );
    let mut game = new_game(2);
    keyword_observer(&mut game, p(1), Trigger::player_searches_library(PlayerFilter::Any));
    add(&mut game, &basic_land("Forest", Subtype::Forest), p(0), Zone::Library);
    add(&mut game, &basic_land("Plains", Subtype::Plains), p(0), Zone::Library);
    stock_library(&mut game, p(0), &[("L1", 1)]);
    let mut dm = Dm {
        picks: vec!["Forest".into(), "Plains".into()],
        ..Dm::default()
    };
    resolve_spell(&mut game, &def, p(0), vec![], &mut dm);
    assert_eq!(
        (
            battlefield_named(&game, "Forest").len(),
            battlefield_named(&game, "Plains").len(),
            trigger_count(&mut game),
        ),
        (1, 1, 1),
        "CR 701.23h: one search finds both the Forest and the Plains (log {:?})",
        dm.log
    );
}
