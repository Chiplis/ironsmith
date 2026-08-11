#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mob_verdict_runtime_another_player_vote_has_no_legal_self_vote() {
    let def = parse_oracle_card_definition("Mob Verdict");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Mob Verdict should compile to spell effects");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice_creature = mob_test_creature(91_305, "Solo Mob Creature");
    let filler = mob_test_card(91_306, "Solo Mob Draw Filler");
    let alice_creature_id =
        game.create_object_from_definition(&alice_creature, alice, Zone::Battlefield);
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = MobVerdictDecisionMaker { votes: vec![0] };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Mob Verdict should resolve even when no another-player vote is legal");

    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "Alice should not damage herself when another-player voting has no legal candidate"
    );
    assert_eq!(
        game.damage_on(alice_creature_id),
        0,
        "Alice's creature should not be damaged when Alice cannot vote for herself"
    );
    assert_eq!(
        mob_owner_zone_count(&game, alice, Zone::Hand),
        0,
        "Alice should not draw because no player received a vote"
    );
    assert_eq!(
        mob_owner_zone_count(&game, alice, Zone::Library),
        1,
        "the draw followup should not consume Alice's library card when no vote was cast"
    );
}

#[test]
pub(super) fn sail_into_the_west_strict_parser_text_and_structure_regression() {
    fn find_embark_conditional(
        effect: &crate::effect::Effect,
    ) -> Option<&crate::effects::ConditionalEffect> {
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>()
            && matches!(
                &conditional.condition,
                crate::ConditionExpr::VoteOptionGetsMoreVotesOrTied(option) if option == "embark"
            )
        {
            return Some(conditional);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(find_embark_conditional);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return find_embark_conditional(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return find_embark_conditional(&with_id.effect);
        }
        None
    }

    let def = parse_oracle_card_definition("Sail into the West");
    let rendered = canonical_compiled_lines(&def).join(" ");

    let program = def
        .spell_effect
        .as_ref()
        .expect("expected Sail into the West spell effects");
    let embark = program
        .iter()
        .find_map(find_embark_conditional)
        .unwrap_or_else(|| panic!("expected typed embark vote conditional, got {program:#?}"));
    let [for_players_effect] = embark.if_true.as_slice() else {
        panic!(
            "expected one per-player embark effect, got {:#?}",
            embark.if_true
        );
    };
    let for_players = for_players_effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("expected embark branch to iterate players");
    let [may_effect] = for_players.effects.as_slice() else {
        panic!(
            "expected one optional effect in embark player scope, got {:#?}",
            for_players.effects
        );
    };
    let may = may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("expected embark player action to remain optional");
    assert!(
        matches!(
            may.effects.as_slice(),
            [discard, draw]
                if discard.downcast_ref::<crate::effects::DiscardHandEffect>().is_some()
                    && draw.downcast_ref::<crate::effects::DrawCardsEffect>().is_some()
        ),
        "expected discard and draw to remain in one optional runtime scope, got {:#?}",
        may.effects
    );

    assert!(
        rendered.contains(
            "Will of the council — Starting with you, each player votes for return or embark"
        ),
        "expected will-of-the-council vote opening to render, got {rendered}"
    );
    assert!(
        rendered.contains(
            "If return gets more votes, each player returns up to two cards from that player's graveyard to that player's hand, then you exile Sail into the West"
        ),
        "expected return vote branch and self-exile to render inside one conditional clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "If embark gets more votes or the vote is tied, each player may discard their hand and draw seven cards"
        ),
        "expected embark vote branch to render, got {rendered}"
    );
    assert!(
        !rendered.contains("Unsupported effect"),
        "Sail into the West should parse strictly without unsupported placeholders, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("VoteEffect")
            && debug.contains("VoteOptionGetsMoreVotes")
            && debug.contains("VoteOptionGetsMoreVotesOrTied")
            && debug.contains("ForPlayersEffect")
            && debug.contains("ReturnFromGraveyardToHandEffect")
            && compact_debug.contains("owner:Some(IteratedPlayer")
            && debug.contains("DiscardHandEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("MoveToZoneEffect"),
        "expected vote conditions, iterated-player graveyard return, optional wheel, and self-exile structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct SailVoteDecisionMaker {
    pub(super) votes: Vec<usize>,
    pub(super) may_accept: HashMap<PlayerId, bool>,
    pub(super) object_choice_players: Vec<PlayerId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for SailVoteDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if !self.votes.is_empty() {
            vec![self.votes.remove(0)]
        } else {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.may_accept.get(&ctx.player).copied().unwrap_or(false)
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_choice_players.push(ctx.player);
        let max = ctx.max.unwrap_or(ctx.candidates.len()).max(ctx.min);
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(max)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sail_test_card(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sail_owner_zone_names(
    game: &crate::game_state::GameState,
    owner: PlayerId,
    zone: Zone,
) -> Vec<String> {
    let mut names = game
        .objects_in_zone(zone)
        .into_iter()
        .filter_map(|id| {
            game.object(id)
                .and_then(|object| (object.owner == owner).then(|| object.name.to_string()))
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_sail_into_the_west_with_votes(
    votes: Vec<usize>,
    may_accept: HashMap<PlayerId, bool>,
) -> (crate::game_state::GameState, SailVoteDecisionMaker) {
    let def = parse_oracle_card_definition("Sail into the West");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Sail into the West should compile to spell effects");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    for (idx, name) in [
        "Sail Alice Grave One",
        "Sail Alice Grave Two",
        "Sail Alice Grave Three",
    ]
    .iter()
    .enumerate()
    {
        let card = sail_test_card(92_001 + idx as u32, name);
        game.create_object_from_definition(&card, alice, Zone::Graveyard);
    }
    let bob_grave = sail_test_card(92_010, "Sail Bob Grave One");
    game.create_object_from_definition(&bob_grave, bob, Zone::Graveyard);

    for (idx, owner, zone, prefix, count) in [
        (20_u32, alice, Zone::Hand, "Sail Alice Hand", 2_u32),
        (30_u32, bob, Zone::Hand, "Sail Bob Hand", 3_u32),
        (40_u32, alice, Zone::Library, "Sail Alice Library", 7_u32),
        (50_u32, bob, Zone::Library, "Sail Bob Library", 7_u32),
    ] {
        for n in 0..count {
            let card = sail_test_card(92_000 + idx + n, &format!("{prefix} {}", n + 1));
            game.create_object_from_definition(&card, owner, zone);
        }
    }

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = SailVoteDecisionMaker {
        votes,
        may_accept,
        object_choice_players: Vec::new(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Sail into the West should resolve");
    (game, dm)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sail_into_the_west_runtime_return_votes_return_each_players_graveyard_cards_and_exile_source()
 {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (game, dm) = resolve_sail_into_the_west_with_votes(vec![0, 0], HashMap::new());

    assert_eq!(
        dm.object_choice_players,
        vec![alice, bob],
        "each player should choose cards from their own graveyard for the return branch"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Hand).len(),
        4,
        "Alice should keep two hand cards and return up to two graveyard cards"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Graveyard).len(),
        1,
        "Alice should leave the third graveyard card behind because the branch is up to two"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Hand).len(),
        4,
        "Bob should keep three hand cards and return his one graveyard card"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Graveyard).len(),
        0,
        "Bob's only graveyard card should be returned"
    );
    assert_eq!(
        travel_zone_names(&game, Zone::Exile),
        vec!["Sail into the West".to_string()],
        "return votes should exile Sail into the West after returning cards"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Library).len(),
        7,
        "embark draw branch should not run when return gets more votes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sail_into_the_west_runtime_tied_vote_runs_embark_with_each_player_may_decision() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut may_accept = HashMap::new();
    may_accept.insert(alice, true);
    may_accept.insert(bob, false);

    let (game, dm) = resolve_sail_into_the_west_with_votes(vec![0, 1], may_accept);

    assert!(
        dm.object_choice_players.is_empty(),
        "return branch should not ask for graveyard choices when the vote is tied"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Hand).len(),
        7,
        "Alice accepted the embark may choice, discarded her hand, and drew seven cards"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Library).len(),
        0,
        "Alice should draw seven cards from her library"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Hand).len(),
        3,
        "Bob declined the embark may choice and should keep his hand"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Library).len(),
        7,
        "Bob declined, so he should not draw cards"
    );
    assert!(
        travel_zone_names(&game, Zone::Exile).is_empty(),
        "embark branch should not exile Sail into the West"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sail_into_the_west_runtime_embark_more_votes_draws_for_accepting_players() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut may_accept = HashMap::new();
    may_accept.insert(alice, true);
    may_accept.insert(bob, true);

    let (game, dm) = resolve_sail_into_the_west_with_votes(vec![1, 1], may_accept);

    assert!(
        dm.object_choice_players.is_empty(),
        "return branch should not ask for graveyard choices when embark gets more votes"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Hand).len(),
        7,
        "Alice should draw seven after accepting embark"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Hand).len(),
        7,
        "Bob should draw seven after accepting embark"
    );
    assert_eq!(
        sail_owner_zone_names(&game, alice, Zone::Graveyard).len(),
        5,
        "Alice's three graveyard cards plus two discarded hand cards should stay in graveyard"
    );
    assert_eq!(
        sail_owner_zone_names(&game, bob, Zone::Graveyard).len(),
        4,
        "Bob's one graveyard card plus three discarded hand cards should stay in graveyard"
    );
}

#[test]
pub(super) fn dungeon_regression_cards_render_key_mechanics() {
    let crawler = parse_oracle_card_definition("Dungeon Crawler");
    let crawler_lines = unprocessed_compiled_lines(&crawler).join(" ");
    assert!(
        crawler_lines.contains("Whenever you complete a dungeon"),
        "expected Dungeon Crawler to keep its completion trigger, got {crawler_lines}"
    );

    let stalker = parse_oracle_card_definition("Gloom Stalker");
    let stalker_lines = unprocessed_compiled_lines(&stalker).join(" ");
    assert!(
        stalker_lines.contains("completed a dungeon"),
        "expected Gloom Stalker to keep its completed-dungeon condition, got {stalker_lines}"
    );

    let adventurer = parse_oracle_card_definition("White Plume Adventurer");
    let adventurer_lines = unprocessed_compiled_lines(&adventurer).join(" ");
    assert!(
        adventurer_lines.contains("take the initiative")
            && adventurer_lines.contains("completed a dungeon"),
        "expected White Plume Adventurer to keep initiative and completion text, got {adventurer_lines}"
    );
}

#[test]
pub(super) fn parse_oracle_undercellar_sweep_strictly_parses_and_renders_initiative_gate() {
    let def = parse_oracle_card_definition("Undercellar Sweep");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        !lower.contains("unsupported predicate") && !lower.contains("unsupported effect"),
        "expected Undercellar Sweep to parse without unsupported placeholders, got {rendered}"
    );
    assert!(
        lower.contains("you take the initiative"),
        "expected Undercellar Sweep to keep ETB initiative text, got {rendered}"
    );
    assert!(
        lower.contains("you or")
            && lower.contains("attacking")
            && lower.contains("initiative")
            && lower.contains("create two 1/1 white soldier creature token")
            && lower.contains("tapped and attacking"),
        "expected Undercellar Sweep attack trigger and initiative gate to render, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_return_to_dust_strictly_parses_and_renders_main_phase_clause() {
    let def = parse_oracle_card_definition("Return to Dust");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        !lower.contains("unsupported predicate") && !lower.contains("unsupported effect"),
        "expected Return to Dust to parse without unsupported placeholders, got {rendered}"
    );
    assert!(
        lower.contains("you cast this spell during your main phase"),
        "expected Return to Dust to render the main-phase cast clause, got {rendered}"
    );

    let raw = format!("{:#?}", def.spell_effect);
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("CastDuringYourMainPhase"),
        "expected Return to Dust to lower the branch predicate, got {raw}"
    );
}

#[test]
pub(super) fn return_to_dust_main_phase_paid_label_condition_branches() {
    struct NoChoices;
    impl crate::decision::DecisionMaker for NoChoices {}

    let def = parse_oracle_card_definition("Return to Dust");
    let raw = format!("{:#?}", def.spell_effect);
    assert!(
        raw.contains("CastDuringYourMainPhase"),
        "expected Return to Dust to include CastDuringYourMainPhase condition, got {raw}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut decision_maker = NoChoices;
    let mut ctx = crate::effects::ExecutionContext::new(spell_id, alice, &mut decision_maker);

    let cond = crate::effect::Condition::ThisSpellPaidLabel("CastDuringYourMainPhase".into());
    let without_label = crate::condition_eval::evaluate_condition_resolution(&game, &cond, &ctx)
        .expect("condition evaluation should succeed");
    assert!(
        !without_label,
        "expected Return to Dust branch condition to be false when spell was not marked as cast in your main phase"
    );

    ctx.optional_costs_paid
        .mark_label_paid("CastDuringYourMainPhase");
    let with_label = crate::condition_eval::evaluate_condition_resolution(&game, &cond, &ctx)
        .expect("condition evaluation should succeed");
    assert!(
        with_label,
        "expected Return to Dust branch condition to be true when spell is marked as cast in your main phase"
    );
}

#[test]
pub(super) fn parse_oracle_careful_consideration_strictly_parses_and_renders_main_phase_replacement()
 {
    let def = parse_oracle_card_definition("Careful Consideration");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        !lower.contains("unsupported") && !lower.contains("unimplemented"),
        "expected Careful Consideration to parse without unsupported placeholders, got {rendered}"
    );
    assert!(
        lower.contains("target player draws four cards, then discards three cards"),
        "expected default draw/discard clause to render, got {rendered}"
    );
    assert!(
        lower.contains(
            "if you cast this spell during your main phase, instead that player draws four cards, then discards two cards"
        ),
        "expected main-phase replacement clause to render with that-player binding, got {rendered}"
    );

    let raw = format!("{:#?}", def.spell_effect);
    let compact_raw: String = raw.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(
        raw.contains("SelfReplacementBranch")
            && raw.contains("CastDuringYourMainPhase")
            && raw.contains("DiscardEffect")
            && compact_raw.contains("Fixed(3,")
            && compact_raw.contains("Fixed(2,")
            && !raw.contains("IteratedPlayer"),
        "expected Careful Consideration to lower as a target-player self-replacement without unbound that-player refs, got {raw}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn undercellar_sweep_attack_trigger_creates_tokens_when_you_have_initiative() {
    let def = parse_oracle_card_definition("Undercellar Sweep");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.set_initiative(Some(alice));

    let attacker = CardDefinitionBuilder::new(CardId::new(), "Attack Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker_id = game.create_object_from_definition(&attacker, alice, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);

    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[crate::decision::AttackerDeclaration {
            creature: attacker_id,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
    )
    .expect("attack declaration should succeed");
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("attack trigger should go on stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("attack trigger should resolve");

    let soldier_tokens = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id).map(|obj| (*id, obj)))
        .filter(|(_, obj)| {
            game.controller_of(obj) == alice
                && obj.kind == crate::object::ObjectKind::Token
                && obj.name == "Soldier"
        })
        .collect::<Vec<_>>();
    assert_eq!(
        soldier_tokens.len(),
        2,
        "expected initiative-on-you branch to create two Soldier tokens"
    );
    assert!(
        soldier_tokens.iter().all(|(id, _)| game.is_tapped(*id)),
        "expected created Soldier tokens to enter tapped"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn undercellar_sweep_attack_trigger_branches_on_initiative_holder() {
    let def = parse_oracle_card_definition("Undercellar Sweep");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let attacker = CardDefinitionBuilder::new(CardId::new(), "Attack Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker_id = game.create_object_from_definition(&attacker, alice, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);

    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut run_attack = |initiative: Option<PlayerId>| {
        game.set_initiative(initiative);
        game.untap(attacker_id);
        let before = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|obj| {
                obj.kind == crate::object::ObjectKind::Token && game.controller_of(obj) == alice
            })
            .count();
        let mut combat = crate::combat_state::CombatState::default();
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[crate::decision::AttackerDeclaration {
                creature: attacker_id,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
        )
        .expect("attack declaration should succeed");
        crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("attack trigger should go on stack");
        if !game.stack.is_empty() {
            crate::game_loop::resolve_stack_entry(&mut game)
                .expect("attack trigger should resolve");
        }
        game.battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|obj| {
                obj.kind == crate::object::ObjectKind::Token && game.controller_of(obj) == alice
            })
            .count()
            - before
    };

    assert_eq!(
        run_attack(Some(bob)),
        2,
        "expected defending-player initiative branch to create two tokens"
    );
    assert_eq!(
        run_attack(None),
        0,
        "expected no-initiative branch to create no tokens"
    );
}

#[test]
pub(super) fn parse_oracle_the_most_dangerous_gamer_regression() {
    let def = parse_oracle_card_definition("The Most Dangerous Gamer");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        lower.contains("whenever you open an attraction"),
        "expected open-attraction trigger to parse and render, got {rendered}"
    );
    assert!(
        lower.contains("put a +1/+1 counter on") && lower.contains("most dangerous gamer"),
        "expected +1/+1 counter trigger clause to compile, got {rendered}"
    );
    assert!(
        lower.contains("whenever you claim the prize of an attraction")
            && lower.contains("destroy target permanent"),
        "expected claim-prize trigger clause to compile, got {rendered}"
    );
    assert!(
        !lower.contains("unsupported"),
        "expected The Most Dangerous Gamer to avoid unsupported placeholders, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_most_dangerous_gamer_triggers_on_you_open_an_attraction() {
    let gamer = parse_oracle_card_definition("The Most Dangerous Gamer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let gamer_id = game.create_object_from_definition(&gamer, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::OpenAttraction,
            alice,
            gamer_id,
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in triggered
        .into_iter()
        .filter(|entry| entry.source == gamer_id)
    {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "expected The Most Dangerous Gamer to trigger when you open an Attraction"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("trigger should be placed on stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("trigger should resolve");

    let counters = game
        .object(gamer_id)
        .and_then(|obj| {
            obj.counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()
        })
        .unwrap_or(0);
    assert_eq!(
        counters, 1,
        "expected The Most Dangerous Gamer to receive a +1/+1 counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_most_dangerous_gamer_does_not_trigger_for_opponents_opened_attraction() {
    let gamer = parse_oracle_card_definition("The Most Dangerous Gamer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let gamer_id = game.create_object_from_definition(&gamer, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::OpenAttraction,
            bob,
            gamer_id,
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggered.iter().all(|entry| entry.source != gamer_id),
        "expected The Most Dangerous Gamer to ignore opponents opening Attractions"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_most_dangerous_gamer_triggers_on_you_claiming_an_attraction_prize() {
    let gamer = parse_oracle_card_definition("The Most Dangerous Gamer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let gamer_id = game.create_object_from_definition(&gamer, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::ClaimAttractionPrize,
            alice,
            gamer_id,
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggered.iter().any(|entry| entry.source == gamer_id),
        "expected The Most Dangerous Gamer to trigger when you claim an Attraction prize"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_most_dangerous_gamer_ignores_opponent_claiming_an_attraction_prize() {
    let gamer = parse_oracle_card_definition("The Most Dangerous Gamer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let gamer_id = game.create_object_from_definition(&gamer, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::ClaimAttractionPrize,
            bob,
            gamer_id,
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggered.iter().all(|entry| entry.source != gamer_id),
        "expected The Most Dangerous Gamer to ignore opponents claiming Attraction prizes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_sorcerous_spyglass_hand_inspection_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sorcerous Spyglass line")
        .parse_text("Look at an opponent's hand, then choose any card name.")
        .expect("spyglass hand-inspection line should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("look at") && rendered.contains("opponent's hand"),
        "expected opponent-hand inspection to compile, got {rendered}"
    );
    assert!(
        rendered.contains("choose any card name") || rendered.contains("choose a card name"),
        "expected follow-up card-name choice to remain present, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_liliana_last_hope_keeps_until_your_next_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Liliana PT line")
        .parse_text("Up to one target creature gets -2/-1 until your next turn.")
        .expect("next-turn PT modifier line should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("-2/-1") && rendered.contains("until your next turn"),
        "expected next-turn PT duration to survive lowering, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_tragic_slip_morbid_branch_stays_parseable() {
    let def = parse_oracle_card_definition("Tragic Slip");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("gets -1/-1 until end of turn"),
        "expected base tragic slip branch to remain, got {rendered}"
    );
    assert!(
        rendered.contains("gets -13/-13 until end of turn"),
        "expected morbid PT branch to compile, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_nexus_of_fate_keeps_extra_turn_and_replacement() {
    let def = parse_oracle_card_definition("Nexus of Fate");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ExtraTurnEffect"),
        "expected Nexus of Fate spell effect to include an extra turn, got {spell_debug}"
    );

    let has_shuffle_replacement = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
        )
    });
    assert!(
        has_shuffle_replacement,
        "expected Nexus of Fate to keep its graveyard replacement ability"
    );
}

#[test]
pub(super) fn parse_oracle_cabal_ritual_compiles_to_self_replacement_branch() {
    let def = parse_oracle_card_definition("Cabal Ritual");

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);

    let debug = format!("{program:?}");
    assert!(
        debug.contains("AddManaEffect")
            && debug.contains("GreaterThanOrEqual")
            && debug.contains("right: Fixed(7)"),
        "expected Cabal Ritual oracle text to lower into a threshold self-replacement, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_stubborn_denial_renders_base_effect_before_self_replacement() {
    let def = parse_oracle_card_definition("Stubborn Denial");

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Counter target noncreature spell unless its controller pays {1}. Ferocious — If you control a creature with power 4 or greater, instead counter that spell"
        ),
        "expected Stubborn Denial to render the base counter-unless-pays effect before the ferocious self-replacement, got {rendered}"
    );
    assert!(
        !rendered.contains("Otherwise, counter target noncreature spell"),
        "expected Stubborn Denial rendering to avoid inverted otherwise phrasing, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_stubborn_denial_ferocious_still_requires_target_choice() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ferocious_creature = crate::card::CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Probe Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let stubborn_denial = CardDefinitionBuilder::new(CardId::new(), "Stubborn Denial")
        .parse_text(
            "Mana cost: {U}\nType: Instant\nCounter target noncreature spell unless its controller pays {1}.\nFerocious \u{2014} If you control a creature with power 4 or greater, counter that spell instead.",
        )
        .expect("generated-style Stubborn Denial block should parse");
    let denial_id = game.create_object_from_definition(&stubborn_denial, alice, Zone::Stack);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        stubborn_denial.spell_effect.as_ref().expect("spell effect"),
        alice,
        Some(denial_id),
        None,
    );

    assert!(
        requirements.iter().any(|requirement| {
            requirement
                .legal_targets
                .iter()
                .any(|target| *target == crate::game_state::Target::Object(target_id))
        }),
        "ferocious self-replacement should preserve Stubborn Denial's original target choice"
    );
}

#[test]
pub(super) fn parse_oracle_stubborn_denial_ferocious_free_cast_prompts_for_target() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;

    let omniscience = parse_oracle_card_definition("Omniscience");
    game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    let ferocious_creature = crate::card::CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Probe Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let stubborn_denial = CardDefinitionBuilder::new(CardId::new(), "Stubborn Denial")
        .parse_text(
            "Mana cost: {U}\nType: Instant\nCounter target noncreature spell unless its controller pays {1}.\nFerocious \u{2014} If you control a creature with power 4 or greater, counter that spell instead.",
        )
        .expect("generated-style Stubborn Denial block should parse");
    let denial_id = game.create_object_from_definition(&stubborn_denial, alice, Zone::Hand);

    let cast_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method:
                        crate::alternative_cast::CastingMethod::PlayFrom {
                            use_alternative: Some(_),
                            ..
                        },
                } if *spell_id == denial_id
            )
        })
        .expect("Omniscience should offer Stubborn Denial as a free cast");

    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(cast_action),
        &mut decision_maker,
    )
    .expect("free-casting Stubborn Denial should not fail");

    let crate::decision::GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Targets(ctx),
    ) = progress
    else {
        panic!("expected Stubborn Denial target prompt, got {progress:?}");
    };

    assert!(
        ctx.requirements.iter().any(|requirement| {
            requirement
                .legal_targets
                .iter()
                .any(|target| *target == crate::game_state::Target::Object(target_id))
        }),
        "free-cast ferocious Stubborn Denial should expose the stack spell target"
    );
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn generated_registry_stubborn_denial_ferocious_free_cast_prompts_for_target() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;

    let omniscience = crate::cards::CardRegistry::try_compile_card("Omniscience")
        .expect("generated Omniscience should compile");
    let omniscience_id = game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    let ferocious_creature = crate::card::CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let lightning_bolt = crate::cards::CardRegistry::try_compile_card("Lightning Bolt")
        .expect("generated Lightning Bolt should compile");
    let target_id = game.create_object_from_definition(&lightning_bolt, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let stubborn_denial = crate::cards::CardRegistry::try_compile_card("Stubborn Denial")
        .expect("generated Stubborn Denial should compile");
    let denial_id = game.create_object_from_definition(&stubborn_denial, alice, Zone::Hand);

    let cast_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method:
                        crate::alternative_cast::CastingMethod::PlayFrom {
                            source,
                            use_alternative: Some(_),
                            ..
                        },
                } if *spell_id == denial_id && *source == omniscience_id
            )
        })
        .expect("generated Omniscience should offer Stubborn Denial as a free cast");

    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(cast_action),
        &mut decision_maker,
    )
    .expect("free-casting generated Stubborn Denial should not fail");

    let crate::decision::GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Targets(ctx),
    ) = progress
    else {
        panic!("expected generated Stubborn Denial target prompt, got {progress:?}");
    };

    assert!(
        ctx.requirements.iter().any(|requirement| {
            requirement
                .legal_targets
                .iter()
                .any(|target| *target == crate::game_state::Target::Object(target_id))
        }),
        "generated free-cast ferocious Stubborn Denial should expose the stack spell target"
    );
}

#[test]
pub(super) fn parse_oracle_stubborn_denial_lets_target_controller_pay_without_ferocious() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Probe Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    let target_stable_id = game
        .object(target_id)
        .expect("target spell should exist")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let stubborn_denial = parse_oracle_card_definition("Stubborn Denial");
    let denial_id = game.create_object_from_definition(&stubborn_denial, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(denial_id, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_id)]),
    );

    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Stubborn Denial should resolve");

    let target_after = game
        .find_object_by_stable_id(target_stable_id)
        .expect("target spell should still be tracked");
    assert_eq!(
        game.object(target_after).expect("target spell exists").zone,
        Zone::Stack,
        "without ferocious, Bob should be able to pay {{1}} and keep the target spell on the stack"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        0,
        "Bob should spend the {{1}} prevention payment"
    );
}

#[test]
pub(super) fn parse_oracle_stubborn_denial_ferocious_counters_without_payment() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ferocious_creature = crate::card::CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Probe Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    let target_stable_id = game
        .object(target_id)
        .expect("target spell should exist")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let stubborn_denial = parse_oracle_card_definition("Stubborn Denial");
    let denial_id = game.create_object_from_definition(&stubborn_denial, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(denial_id, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_id)]),
    );

    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Stubborn Denial should resolve");

    let target_after = game
        .find_object_by_stable_id(target_stable_id)
        .expect("countered target spell should still be tracked");
    assert_eq!(
        game.object(target_after).expect("target spell exists").zone,
        Zone::Graveyard,
        "ferocious should replace the unless-payment effect with an unconditional counter"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        1,
        "ferocious should not offer or spend the {{1}} prevention payment"
    );
}

pub(super) fn add_graveyard_filler(
    game: &mut crate::game_state::GameState,
    player: PlayerId,
    count: usize,
) {
    for idx in 0..count {
        let filler =
            crate::card::CardBuilder::new(CardId::new(), format!("Graveyard Filler {idx}"))
                .card_types(vec![CardType::Instant])
                .build();
        game.create_object_from_card(&filler, player, Zone::Graveyard);
    }
}

#[test]
pub(super) fn parse_oracle_anticognition_strictly_parses_and_renders_threshold_replacement() {
    let def = parse_oracle_card_definition("Anticognition");

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Counter target creature or planeswalker spell unless its controller pays {2}. If an opponent has eight or more cards in their graveyard, instead counter that spell, then scry 2"
        ),
        "expected Anticognition threshold replacement and scry wording, got {rendered}"
    );

    let debug = format!("{program:?}");
    assert!(
        debug.contains("CardsInGraveyard")
            && debug.contains("Opponent")
            && debug.contains("Fixed(8)")
            && debug.contains("ScryEffect"),
        "expected Anticognition to lower the opponent graveyard threshold and scry structurally, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_anticognition_targets_only_creature_or_planeswalker_spells() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_spell = crate::card::CardBuilder::new(CardId::new(), "Bob Creature Spell")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_id = game.create_object_from_card(&creature_spell, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(creature_id, bob));

    let planeswalker_spell = crate::card::CardBuilder::new(CardId::new(), "Bob Planeswalker Spell")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let planeswalker_id = game.create_object_from_card(&planeswalker_spell, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(planeswalker_id, bob));

    let instant_spell = crate::card::CardBuilder::new(CardId::new(), "Bob Instant Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let instant_id = game.create_object_from_card(&instant_spell, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(instant_id, bob));

    let anticognition = parse_oracle_card_definition("Anticognition");
    let anticognition_id = game.create_object_from_definition(&anticognition, alice, Zone::Stack);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        anticognition.spell_effect.as_ref().expect("spell effect"),
        alice,
        Some(anticognition_id),
        None,
    );
    let legal_targets = requirements
        .first()
        .expect("Anticognition should require a target")
        .legal_targets
        .clone();

    assert!(
        legal_targets.contains(&crate::game_state::Target::Object(creature_id)),
        "creature spells should be legal Anticognition targets"
    );
    assert!(
        legal_targets.contains(&crate::game_state::Target::Object(planeswalker_id)),
        "planeswalker spells should be legal Anticognition targets"
    );
    assert!(
        !legal_targets.contains(&crate::game_state::Target::Object(instant_id)),
        "noncreature nonplaneswalker spells should not be legal Anticognition targets"
    );
}

#[test]
pub(super) fn parse_oracle_anticognition_under_threshold_lets_target_controller_pay() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_graveyard_filler(&mut game, bob, 7);

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Creature Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    let target_stable_id = game
        .object(target_id)
        .expect("target spell should exist")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let anticognition = parse_oracle_card_definition("Anticognition");
    let anticognition_id = game.create_object_from_definition(&anticognition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(anticognition_id, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_id)]),
    );

    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Anticognition should resolve");

    let target_after = game
        .find_object_by_stable_id(target_stable_id)
        .expect("target spell should still be tracked");
    assert_eq!(
        game.object(target_after).expect("target spell exists").zone,
        Zone::Stack,
        "below threshold, Bob should be able to pay {{2}} and keep the target spell on the stack"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        0,
        "Bob should spend the {{2}} prevention payment"
    );
}

#[test]
pub(super) fn parse_oracle_anticognition_threshold_counters_without_payment_and_scries() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_graveyard_filler(&mut game, bob, 8);

    for name in ["Alice Top Card", "Alice Second Card"] {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Bob Creature Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Stack);
    let target_stable_id = game
        .object(target_id)
        .expect("target spell should exist")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, bob));

    let anticognition = parse_oracle_card_definition("Anticognition");
    let anticognition_id = game.create_object_from_definition(&anticognition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(anticognition_id, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_id)]),
    );

    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Anticognition should resolve");

    let target_after = game
        .find_object_by_stable_id(target_stable_id)
        .expect("countered target spell should still be tracked");
    assert_eq!(
        game.object(target_after).expect("target spell exists").zone,
        Zone::Graveyard,
        "eight-card opponent graveyard threshold should replace the payment branch with an unconditional counter"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        2,
        "threshold replacement should not offer or spend the {{2}} prevention payment"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        2,
        "threshold branch should execute scry 2 without moving cards out of Alice's library"
    );
}

#[test]
pub(super) fn parse_oracle_anticognition_multiplayer_checks_any_opponent_graveyard() {
    let mut game = crate::game_state::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    add_graveyard_filler(&mut game, bob, 8);
    add_graveyard_filler(&mut game, carol, 3);

    for name in ["Alice Top Card", "Alice Second Card"] {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let target_card = crate::card::CardBuilder::new(CardId::new(), "Carol Creature Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .build();
    let target_id = game.create_object_from_card(&target_card, carol, Zone::Stack);
    let target_stable_id = game
        .object(target_id)
        .expect("target spell should exist")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target_id, carol));

    let anticognition = parse_oracle_card_definition("Anticognition");
    let anticognition_id = game.create_object_from_definition(&anticognition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(anticognition_id, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_id)]),
    );

    game.player_mut(carol)
        .expect("carol exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Anticognition should resolve in multiplayer");

    let target_after = game
        .find_object_by_stable_id(target_stable_id)
        .expect("countered target spell should still be tracked");
    assert_eq!(
        game.object(target_after).expect("target spell exists").zone,
        Zone::Graveyard,
        "Anticognition should count any opponent's eight-card graveyard in multiplayer"
    );
    assert_eq!(
        game.player(carol).expect("carol exists").mana_pool.total(),
        2,
        "threshold replacement should not offer or spend Carol's {{2}} prevention payment"
    );
}

#[test]
pub(super) fn parse_oracle_future_replacement_followups_do_not_use_self_replacement_bridge() {
    for name in ["Faunsbane Troll", "Mawloc", "Nine-Ringed Bo"] {
        let def = parse_oracle_card_definition(name);
        let self_replacement_count = def
            .abilities
            .iter()
            .map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Triggered(triggered) => triggered
                    .effects
                    .segments
                    .iter()
                    .map(|segment| segment.self_replacements.len())
                    .sum::<usize>(),
                crate::ability::AbilityKind::Activated(activated) => activated
                    .effects
                    .segments
                    .iter()
                    .map(|segment| segment.self_replacements.len())
                    .sum::<usize>(),
                _ => 0,
            })
            .sum::<usize>();
        let debug = format!("{:#?}", def.abilities);

        assert!(
            self_replacement_count == 0,
            "expected future-event replacement wording on {name} to avoid self-replacement lowering, got {debug}"
        );
        assert!(
            debug.contains("Exile"),
            "expected {name} to keep its exile followup semantics after parsing, got {debug}"
        );
    }
}

#[test]
pub(super) fn parse_oracle_future_replacement_followups_register_zone_replacements() {
    for name in ["Magma Spray", "Carbonize", "Obliterating Bolt"] {
        let def = parse_oracle_card_definition(name);
        let debug = format!("{:#?}", def.spell_effect);

        assert!(
            debug.contains("RegisterZoneReplacementEffect"),
            "expected {name} to lower its future graveyard replacement as a registered zone replacement, got {debug}"
        );
        assert!(
            !debug.contains("SelfReplacementBranch"),
            "expected {name} to avoid self-replacement lowering for `would ... instead` text, got {debug}"
        );
    }
}

#[test]
pub(super) fn oracle_fight_and_damage_death_replacements_render_canonically() {
    for (name, expected) in [
        (
            "Faunsbane Troll",
            "This creature fights target creature you don't control. If that creature would die this turn, exile it instead.",
        ),
        (
            "Mawloc",
            "it fights up to one target creature an opponent controls. If that creature would die this turn, exile it instead.",
        ),
        (
            "Unnatural Aggression",
            "Target creature you control fights target creature an opponent controls. If the creature an opponent controls would die this turn, exile it instead.",
        ),
        (
            "Carbonize",
            "Carbonize deals 3 damage to any target. If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.",
        ),
        (
            "Disintegrate",
            "Disintegrate deals X damage to any target. If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.",
        ),
        (
            "Scorching Lava",
            "Scorching Lava deals 2 damage to any target. If this spell was kicked, that creature can't be regenerated this turn and if it would die this turn, exile it instead.",
        ),
    ] {
        let def = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&def);
        let rendered = compiled.join("\n");
        assert!(
            rendered.contains(expected),
            "expected {name} replacement rider to render canonically, got {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_then_exile_clause_registers_future_zone_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Counter Exile Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owners graveyard.",
        )
        .expect("counter-then-exile text should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("RegisterZoneReplacementEffect"),
        "expected countered-this-way exile clause to lower as a future zone replacement, got {debug}"
    );
    assert!(
        debug.contains("LocalRewriteEffect"),
        "expected countered-this-way exile clause to be scoped as a local self-rewrite, got {debug}"
    );
    assert!(
        !debug.contains("SelfReplacementBranch"),
        "expected countered-this-way exile clause to avoid self-replacement lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_then_exile_with_time_counters_and_suspend() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Delay Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell. If that spell is countered this way, exile it with three time counters on it instead of putting it into its owner's graveyard. If it doesn't have suspend, it gains suspend.",
        )
        .expect("Delay-style counter/exile/suspend text should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("RegisterZoneReplacementEffect"),
        "expected countered-this-way exile clause to lower as a zone replacement, got {debug}"
    );
    assert!(
        debug.contains("counters: [") && debug.contains("Time") && debug.contains("3"),
        "expected replacement to carry three time counters, got {debug}"
    );
    assert!(
        debug.contains("ApplyContinuousEffect"),
        "expected follow-up to grant suspend through a continuous effect, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("counter target spell"),
        "expected rendered text to keep the counter instruction, got {rendered}"
    );
    assert!(
        rendered_lower.contains(
            "if that spell is countered this way, exile it with three time counters on it instead of putting it into its owner's graveyard"
        ),
        "expected rendered text to preserve the time-counter replacement, got {rendered}"
    );
    assert!(
        rendered_lower.contains("if it doesn't have suspend, it gains suspend"),
        "expected rendered text to preserve the suspend grant, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("keyword:suspend"),
        "expected rendered text to hide raw suspend internals, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fatal_push_revolt_stays_self_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fatal Push Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        )
        .expect("Fatal Push variant should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch"),
        "expected revolt followup to remain a true self-replacement, got {debug}"
    );
    assert!(
        !debug.contains("RegisterZoneReplacementEffect"),
        "expected Fatal Push revolt followup to avoid future-zone-replacement lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_skyclave_apparition_where_x_uses_exiled_card_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skyclave Apparition Variant")
        .parse_text(
            "When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.\nWhen this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card.",
        )
        .expect("skyclave-style where-x clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ManaValueOf"),
        "expected exiled-card mana value binding in lowered ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn where_x_exiled_card_plus_one_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Broken Skyclave Variant")
        .parse_text(
            "When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.\nWhen this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card plus one.",
        )
        .expect_err("unsupported where-x math tail should still fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported where-x clause")
            || rendered.contains("plus one")
            || rendered.contains("pending effect metric requires a prior memory-producing effect")
            || rendered.contains(
                "pending filtered effect metric requires a prior memory-producing effect"
            ),
        "expected loud where-x failure, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_beza_relative_opponent_comparisons() {
    CardDefinitionBuilder::new(CardId::new(), "Beza Variant")
        .parse_text(
            "When this creature enters, create a Treasure token if an opponent controls more lands than you. You gain 4 life if an opponent has more life than you. Create two 1/1 blue Fish creature tokens if an opponent controls more creatures than you. Draw a card if an opponent has more cards in hand than you.",
        )
        .expect("beza-style relative comparisons should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_thieving_skydiver_equipment_followup_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thieving Skydiver Variant")
        .parse_text(
            "When this creature enters, if it was kicked, gain control of target artifact with mana value X or less. If that artifact is an Equipment, attach it to this creature.",
        )
        .expect("tagged equipment followup should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Thieving Skydiver variant should have an ETB trigger");

    assert_eq!(
        triggered.intervening_if.as_ref(),
        Some(&Condition::ThisSpellWasKicked),
        "ETB kicked followups should check whether the source spell was kicked",
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_target_spell_if_it_was_kicked_keeps_target_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Recoil Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell if it was kicked.")
        .expect("target-spell kicked followup should parse");

    let spell = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{spell:?}");
    assert!(
        debug.contains("TargetWasKicked"),
        "target-spell kicked condition should keep using TargetWasKicked, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_slinn_voda_renders_kicked_exception_bounce_cleanly() {
    let def = parse_oracle_card_definition("Slinn Voda, the Rising Deep");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "When Slinn Voda enters, if it was kicked, return all creatures to their owners' hands except for Merfolk, Krakens, Leviathans, Octopuses, and Serpents."
        ),
        "expected Slinn Voda compiled text to preserve kicked ETB exception wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_whelming_wave_renders_subtype_exceptions_cleanly() {
    let def = parse_oracle_card_definition("Whelming Wave");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Return all creatures to their owners' hands except for Krakens, Leviathans, Octopuses, and Serpents."
        ),
        "expected Whelming Wave compiled text to render subtype exclusions as an exception clause, got {rendered}"
    );
    assert!(
        !rendered.contains("non-kraken"),
        "expected Whelming Wave compiled text to avoid repeated non-subtype adjectives, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_whelming_wave_preserves_excluded_subtype_filter_shape() {
    let def = parse_oracle_card_definition("Whelming Wave");
    let spell = def.spell_effect.as_ref().expect("spell effect");
    let return_to_hand = spell
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ReturnToHandEffect>())
        .expect("Whelming Wave should lower to ReturnToHandEffect");
    let ChooseSpec::All(filter) = &return_to_hand.spec else {
        panic!(
            "Whelming Wave should return all objects matching an excluded-subtype filter, got {:?}",
            return_to_hand.spec
        );
    };

    assert!(
        filter.card_types.contains(&CardType::Creature),
        "Whelming Wave filter should still select creatures, got {filter:?}"
    );
    assert_eq!(
        filter.excluded_subtypes,
        vec![
            Subtype::Kraken,
            Subtype::Leviathan,
            Subtype::Octopus,
            Subtype::Serpent,
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_currency_converter_nonland_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Currency Converter Variant")
        .parse_text(
            "{T}: Put a card exiled with this artifact into its owner's graveyard. If it's a land card, create a Treasure token. If it's a nonland card, create a 2/2 black Rogue creature token.",
        )
        .expect("tagged nonland-card followup should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dauthi_voidwalker_void_counter_target_phrase() {
    CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Variant")
        .parse_text(
            "{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
        )
        .expect("tagged counter-state exiled-card choice should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dauthi_voidwalker_full_text_without_parser_fallback() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
        )
        .expect("Dauthi Voidwalker text should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    let abilities_debug_compact: String = abilities_debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(
        !abilities_debug.contains("UnsupportedParserLine"),
        "expected full Dauthi text to avoid unsupported parser fallbacks, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ExileToCounteredExileInsteadOfGraveyard"),
        "expected Dauthi replacement ability to lower to a real static ability, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug_compact.contains("zone:Some(Exile,)")
            && abilities_debug_compact.contains("with_counter:Some(")
            && abilities_debug.contains("Void"),
        "expected Dauthi activation to choose from exile, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect"),
        "expected Dauthi activation to preserve the free-cast clause, got {abilities_debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost"
        ),
        "expected Dauthi's linked choice and permissions to render as one clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn filtered_zone_play_permission_cards_render_structurally() {
    let abandoned = CardDefinitionBuilder::new(CardId::new(), "Abandoned Sarcophagus Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You may cast spells that have a cycling ability from your graveyard.\nIf a card that has a cycling ability would be put into your graveyard from anywhere and it wasn't cycled, exile it instead.",
        )
        .expect("cycling graveyard permissions should parse");
    let abandoned_rendered = unprocessed_compiled_lines(&abandoned).join("\n");
    assert!(
        abandoned_rendered
            .contains("You may cast spells that have a cycling ability from your graveyard"),
        "{abandoned_rendered}"
    );
    assert!(
        abandoned_rendered.contains(
            "If a card that has a cycling ability would be put into your graveyard from anywhere and it wasn't cycled, exile it instead"
        ),
        "{abandoned_rendered}"
    );
    assert!(!abandoned_rendered.contains("have play from zone"));

    let draugr_permission = "You may cast spells from among cards in exile your opponents own with ice counters on them, and you may spend mana from snow sources as though it were mana of any color to cast those spells";
    let draugr = CardDefinitionBuilder::new(CardId::new(), "Draugr Necromancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(&format!(
            "If a nontoken creature an opponent controls would die, exile that card with an ice counter on it instead.\n{draugr_permission}."
        ))
        .expect("countered-exile snow permission should parse");
    let draugr_rendered = unprocessed_compiled_lines(&draugr).join("\n");
    assert!(
        draugr_rendered.contains(draugr_permission),
        "{draugr_rendered}"
    );
    assert_eq!(draugr_rendered.matches(draugr_permission).count(), 1);
    assert!(!draugr_rendered.contains("have play from zone"));

    let haldan_permission = "You may play lands and cast noncreature spells from among cards you exiled that have fetch counters on them, and you may spend mana as though it were mana of any color to cast those spells";
    let haldan = CardDefinitionBuilder::new(CardId::new(), "Haldan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(&format!("{haldan_permission}."))
        .expect("source-linked countered-exile permission should parse");
    let haldan_rendered = unprocessed_compiled_lines(&haldan).join("\n");
    assert!(
        haldan_rendered.contains(haldan_permission),
        "{haldan_rendered}"
    );
    assert_eq!(haldan_rendered.matches(haldan_permission).count(), 1);
    assert!(!haldan_rendered.contains("have play from zone"));

    let liliana = CardDefinitionBuilder::new(CardId::new(), "Liliana Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("−3: You may cast Zombie spells from your graveyard this turn.")
        .expect("temporary Zombie graveyard permission should parse");
    let liliana_rendered = unprocessed_compiled_lines(&liliana).join("\n");
    assert!(
        liliana_rendered.contains("−3: You may cast Zombie spells from your graveyard"),
        "{liliana_rendered}"
    );
    assert!(!liliana_rendered.contains("have play from zone"));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_vaultborn_tyrant_nontoken_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Vaultborn Tyrant Variant")
        .parse_text(
            "When this creature dies, if it's not a token, create a token that's a copy of it, except it's an artifact in addition to its other types.",
        )
        .expect("tagged nontoken followup should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_time_vault_skip_that_turn_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Time Vault Variant")
        .parse_text(
            "This artifact enters tapped.\nThis artifact doesn't untap during your untap step.\nIf you would begin your turn while this artifact is tapped, you may skip that turn instead. If you do, untap this artifact.\n{T}: Take an extra turn after this one.",
        )
        .expect("time-vault skip-that-turn clause should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_portal_to_phyrexia_subtype_followup_sentence() {
    CardDefinitionBuilder::new(CardId::new(), "Portal to Phyrexia Variant")
        .parse_text(
            "At the beginning of your upkeep, put target creature card from a graveyard onto the battlefield under your control. It's a Phyrexian in addition to its other types.",
        )
        .expect("implicit tagged subtype followup should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rise_from_the_grave_color_and_subtype_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rise from the Grave")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Put target creature card from a graveyard onto the battlefield under your control. That creature is a black Zombie in addition to its other colors and types.",
        )
        .expect("Rise from the Grave should parse strictly");

    let spell_effect = def.spell_effect.as_ref().expect("expected spell effect");
    let effects = spell_effect.flattened_default_effects();
    assert_eq!(
        effects.len(),
        2,
        "expected return and coordinated followups"
    );

    let moved = effects[0]
        .downcast_ref::<TaggedEffect>()
        .expect("returned creature should be tagged");
    let move_to_battlefield = moved
        .effect
        .downcast_ref::<MoveToZoneEffect>()
        .expect("first effect should return a creature card");
    assert_eq!(move_to_battlefield.zone, Zone::Battlefield);
    match move_to_battlefield.target.base() {
        ChooseSpec::Object(filter) => {
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert!(filter.card_types.contains(&CardType::Creature));
        }
        other => panic!("Rise should target a creature card in a graveyard, got {other:?}"),
    }

    let coordinated = effects[1]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("second effect should coordinate the color and subtype followups");
    assert_eq!(coordinated.effects.len(), 2);

    let color_effect = coordinated.effects[0]
        .downcast_ref::<TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        })
        .expect("second effect should add black to the returned creature");
    assert!(matches!(
        color_effect.target_spec.as_ref(),
        Some(ChooseSpec::Tagged(tag)) if tag == &moved.tag
    ));
    assert_eq!(
        color_effect.modification,
        Some(crate::continuous::Modification::AddColors(
            crate::color::ColorSet::BLACK,
        ))
    );

    let subtype_effect = coordinated.effects[1]
        .downcast_ref::<TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        })
        .expect("third effect should add Zombie to the returned creature");
    assert!(matches!(
        subtype_effect.target_spec.as_ref(),
        Some(ChooseSpec::Tagged(tag)) if tag == &moved.tag
    ));
    assert_eq!(
        subtype_effect.modification,
        Some(crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Zombie,
        ]))
    );

    let score_path = crate::compiled_text::compile_effect_list(effects);
    assert_eq!(
        score_path,
        "Put target creature card from a graveyard onto the battlefield under your control. That creature is a black zombie in addition to its other colors and types"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("that creature is a black zombie in addition to its other colors and types"),
        "expected combined color/type followup rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_necromantic_summons_spell_mastery_counter_followup() {
    let def = parse_oracle_card_definition("Necromantic Summons");
    assert!(
        def.card.card_types.contains(&CardType::Sorcery),
        "Necromantic Summons should parse as a sorcery"
    );

    let spell_effect = def.spell_effect.as_ref().expect("expected spell effect");
    assert_eq!(
        spell_effect.segments.len(),
        1,
        "the entry-counter condition should fuse into the reanimation event"
    );
    let move_effects = &spell_effect.segments[0].default_effects;
    assert_eq!(move_effects.len(), 1);
    let moved = move_effects[0]
        .downcast_ref::<TaggedEffect>()
        .expect("returned creature should be tagged");
    let move_to_battlefield = moved
        .effect
        .downcast_ref::<MoveToZoneEffect>()
        .expect("first effect should return a creature card");
    assert_eq!(move_to_battlefield.zone, Zone::Battlefield);
    match move_to_battlefield.target.base() {
        ChooseSpec::Object(filter) => {
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert!(filter.card_types.contains(&CardType::Creature));
        }
        other => panic!(
            "Necromantic Summons should target a creature card in a graveyard, got {other:?}"
        ),
    }

    let [entry_counter] = move_to_battlefield.enters_with_counters.as_slice() else {
        panic!(
            "spell mastery should supply one entry-time counter specification: {move_to_battlefield:#?}"
        );
    };
    assert_eq!(
        entry_counter.surface,
        ironsmith_core::BattlefieldEntryCounterSurface::ThatObjectEntersIfCondition
    );
    match entry_counter
        .condition
        .as_ref()
        .expect("spell mastery condition")
    {
        crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(2),
        } => {
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            assert_eq!(
                filter.card_types,
                vec![CardType::Instant, CardType::Sorcery]
            );
        }
        other => panic!("expected spell mastery graveyard count condition, got {other:?}"),
    }
    assert_eq!(
        entry_counter.counter_type,
        crate::object::CounterType::PlusOnePlusOne
    );
    assert_eq!(
        entry_counter.amount.unhinted(),
        &crate::effect::Value::Fixed(2)
    );

    let expected_compiled = concat!(
        "Put target creature card from a graveyard onto the battlefield under your control. ",
        "Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, ",
        "that creature enters with two additional +1/+1 counters on it."
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![expected_compiled.to_string()],
        "compiled text should preserve reanimation plus spell mastery counter clause"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ghost_vacuum_base_pt_and_subtype_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghost Vacuum Variant")
        .parse_text(
            "{6}, {T}, Sacrifice this artifact: Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it. Each of them is a 1/1 Spirit in addition to its other types. Activate only as a sorcery.",
        )
        .expect("implicit tagged base-pt followup should parse");

    let debug = format!("{def:#?}");
    assert!(debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(debug.contains("target: All("), "{debug}");
    assert!(
        debug.contains("target_spec: Some") && debug.contains("moved_0"),
        "{debug}"
    );
    assert!(
        debug.contains("card_types: [") && debug.contains("Creature"),
        "{debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mass_library_placement_preserves_all_objects() {
    for (name, text, expected) in [
        (
            "Hallowed Burial Variant",
            "Put all creatures on the bottom of their owners' libraries.",
            "Put all creatures on the bottom of their owners' libraries.",
        ),
        (
            "Harmonic Convergence Variant",
            "Put all enchantments on top of their owners' libraries.",
            "Put all enchantments on top of their owners' libraries.",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("mass library placement should parse");
        let debug = format!("{def:#?}");
        assert!(debug.contains("MoveToZoneEffect"), "{debug}");
        assert!(debug.contains("target: All("), "{debug}");
        assert!(
            debug.contains("destination_player_surface: None"),
            "an explicit owners' destination must not be rebound to a contextual player: {debug}"
        );
        assert!(
            debug.contains("destination_player_reference_surface: None"),
            "the pronoun modifying owners must not become the library owner: {debug}"
        );
        assert_eq!(
            crate::compiled_text::unprocessed_compiled_lines(&def),
            vec![expected.to_string()],
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn selected_hand_discard_surfaces_keep_the_choice_bound_to_those_cards() {
    for (name, text, expected) in [
        (
            "Abandon Hope Effect Variant",
            "Look at target opponent's hand and choose X cards from it. That player discards those cards.",
            "Look at target opponent's hand and choose X cards from it. That player discards those cards.",
        ),
        (
            "Discordant Dirge Effect Variant",
            "Look at target opponent's hand and choose up to X cards from it, where X is the number of verse counters on this enchantment. That player discards those cards.",
            "Look at target opponent's hand and choose up to X cards from it, where X is the number of verse counters on this enchantment. That player discards those cards.",
        ),
        (
            "Extortion Effect Variant",
            "Look at target player's hand and choose up to two cards from it. That player discards those cards.",
            "Look at target player's hand and choose up to two cards from it. That player discards those cards.",
        ),
        (
            "Mind Warp Effect Variant",
            "Look at target player's hand and choose X cards from it. That player discards those cards.",
            "Look at target player's hand and choose X cards from it. That player discards those cards.",
        ),
        (
            "Noggin Whack Effect Variant",
            "Target player reveals three cards from their hand. You choose two of them. That player discards those cards.",
            "Target player reveals three cards from their hand. You choose two of them. That player discards those cards.",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("selected-hand discard surface should parse");
        let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
        assert_eq!(rendered, expected, "{name}: {def:#?}");
        let debug = format!("{def:#?}");
        assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
        assert!(debug.contains("DiscardEffect"), "{debug}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn selected_hand_discard_preserves_two_distinct_mana_value_filters() {
    let text = "When you cast this spell, target opponent reveals their hand. You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater. That player discards those cards.";
    let expected = "When you cast this spell, target opponent reveals their hand, choose a nonland card with mana value 3 or less, choose a card with mana value 4 or greater, then that player discards those cards.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Double Filter Hand Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("double-filter selected-hand discard should parse");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(rendered, expected, "{def:#?}");

    let debug = format!("{def:#?}");
    assert_eq!(debug.matches("ChooseObjectsEffect").count(), 2, "{debug}");
    assert!(
        debug.contains("LessThanOrEqual(\n") && debug.contains("3"),
        "{debug}"
    );
    assert!(
        debug.contains("GreaterThanOrEqual(\n") && debug.contains("4"),
        "{debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_all_sacrifice_activation_cost_preserves_set_cardinality() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tomb of Urami Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{2}{B}{B}, {T}, Sacrifice all lands: Create Urami, a legendary 5/5 black Demon Spirit creature token with flying.",
        )
        .expect("all-object sacrifice cost should parse");

    let debug = format!("{def:#?}");
    assert!(debug.contains("SacrificePlayerEffect"), "{debug}");
    assert!(debug.contains("Count("), "{debug}");
    assert!(
        crate::compiled_text::compiled_text_lines(&def)
            .join(" ")
            .contains("Sacrifice all lands"),
        "{}",
        crate::compiled_text::compiled_text_lines(&def).join(" ")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn multi_target_connive_renders_each_selected_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Change of Plans Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Each of X target creatures you control connive. You may have any number of them phase out.",
        )
        .expect("multi-target connive should parse");
    let compiled = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        compiled.contains("Each of X target creatures you control connive"),
        "{compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shuffle_all_from_target_graveyard_preserves_set_and_owner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Repopulate Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Shuffle all creature cards from target player's graveyard into that player's library.",
        )
        .expect("mass graveyard shuffle should parse");
    let debug = format!("{def:#?}");
    assert!(debug.contains("ShuffleObjectsIntoLibraryEffect"), "{debug}");
    assert!(debug.contains("target: All("), "{debug}");
    assert!(debug.contains("player: Target("), "{debug}");
    let compiled = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        compiled.contains("Shuffle all creature cards"),
        "{compiled}"
    );
    assert!(compiled.contains("target player's"), "{compiled}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn destroy_all_with_combat_history_stays_mass_destruction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Retaliate Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy all creatures that dealt damage to you this turn.")
        .expect("mass combat-history destroy should parse");
    let debug = format!("{def:#?}");
    let compact_debug = format!("{def:?}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
    assert!(debug.contains("spec: All("), "{debug}");
    assert!(
        compact_debug.contains("dealt_damage_to_player_this_turn: Some(You)"),
        "{debug}"
    );
    assert!(
        crate::compiled_text::compiled_text_lines(&def)
            .join(" ")
            .contains("Destroy all creatures that dealt damage to you this turn"),
        "{}",
        crate::compiled_text::compiled_text_lines(&def).join(" ")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sothera_supervoid_end_step_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sothera, the Supervoid")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature you control dies, each opponent chooses a creature they control and exiles it.\n\
At the beginning of your end step, if a player controls no creatures, sacrifice Sothera, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it.",
        )
        .expect("Sothera-style source-linked reanimation trigger should parse");

    let ability_debug = format!("{:#?}", def.abilities);
    let compact_debug = format!("{:?}", def.abilities);
    assert!(
        ability_debug.contains("intervening_if: Some")
            && ability_debug.contains("PlayerControlsExactly")
            && ability_debug.contains("count: 0"),
        "expected no-creatures intervening-if condition, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("SacrificeTargetEffect")
            && ability_debug.contains("MoveToZoneEffect")
            && ability_debug.contains("zone: Battlefield")
            && ability_debug.contains("battlefield_controller: You")
            && ability_debug.contains("__source_exiled__")
            && ability_debug.contains("enters_with_counters")
            && ability_debug.contains("counter_type: PlusOnePlusOne")
            && ability_debug.contains("amount: SurfaceHinted")
            && compact_debug.contains("Fixed(2)")
            && compact_debug.contains("target: Tagged(TagKey(\"__it__\"))")
            && !compact_debug.contains("target: Iterated")
            && ability_debug.contains("\"moved_0\"")
            && !ability_debug.contains("\"moved_1\""),
        "expected sacrifice, source-linked battlefield move, and two counters, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enduring_curiosity_type_removal_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Enduring Curiosity Variant")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .parse_text(
            "When this creature dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)",
        )
        .expect("enduring return line should strictly parse with a typed return followup");

    assert_eq!(
        compiled_text_lines(&def).join(" "),
        "When this creature dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment.",
        "expected the typed return and exact enchantment reset to retain their two-sentence surface"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("TaggedObjectMatchedLastKnown")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Battlefield")
            && debug.contains("battlefield_controller: Owner")
            && debug.contains("SetCardTypes")
            && debug.contains("Enchantment")
            && debug.contains("RemoveCardTypes"),
        "expected last-known creature semantics, a graveyard return, and an exact enchantment type reset, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cecil_half_starting_life_threshold() {
    CardDefinitionBuilder::new(CardId::new(), "Cecil Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Deathtouch\nWhenever this creature deals damage, you lose that much life. Then if your life total is less than or equal to half your starting life total, untap this creature and transform it.",
        )
        .expect("half-starting-life threshold should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn half_starting_life_threshold_with_extra_math_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Half Starting Threshold Negative Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals damage, if your life total is less than or equal to half your starting life total plus one, untap this creature.",
        )
        .expect_err("unsupported extra math after half-starting-life threshold should fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not find verb")
            || rendered.contains("unsupported predicate"),
        "expected loud failure for unsupported threshold math, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_magda_other_dwarves_anthem() {
    CardDefinitionBuilder::new(CardId::new(), "Magda Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Other Dwarves you control get +1/+0.\nWhenever a Dwarf you control becomes tapped, create a Treasure token.\nSacrifice five Treasures: Search your library for an artifact or Dragon card, put that card onto the battlefield, then shuffle.",
        )
        .expect("Magda rules text should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_screaming_nemesis_any_other_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Screaming Nemesis Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nWhenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.",
        )
        .expect("any-other-target damage followup should parse");
    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("AnyOtherTarget"),
        "expected any-other-target semantics to survive lowering, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn enduring_curiosity_returns_as_a_new_enchantment_object() {
    let def = parse_oracle_card_definition("Enduring Curiosity");
    let death_trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:?}", triggered.effects).contains("MoveToZoneEffect") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Enduring Curiosity should have its graveyard-return trigger");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let battlefield_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let battlefield_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(battlefield_id)
                .expect("Enduring Curiosity should exist on the battlefield"),
            &game,
        );
    let stable_id = battlefield_snapshot.stable_id;
    let graveyard_id = game
        .move_object_by_sba(battlefield_id, Zone::Graveyard)
        .expect("Enduring Curiosity should move to its owner's graveyard");
    assert_ne!(
        graveyard_id, battlefield_id,
        "the death zone change should create a new object"
    );

    let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            battlefield_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(battlefield_snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(graveyard_id, alice, &mut dm)
        .with_source_snapshot(battlefield_snapshot)
        .with_triggering_event(trigger_event);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        graveyard_id,
        &death_trigger.effects,
        None,
        &[],
    )
    .expect("Enduring Curiosity's death trigger should resolve");

    let returned_id = game
        .find_object_by_stable_id(stable_id)
        .expect("Enduring Curiosity should return to the battlefield");
    assert_ne!(
        returned_id, graveyard_id,
        "the return must create another new object"
    );
    assert_eq!(
        game.object(returned_id).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert!(
        ctx.tagged_objects.iter().any(|(tag, snapshots)| {
            tag.as_str().starts_with("returned_")
                && snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == returned_id)
        }),
        "the return-result tag must carry the battlefield object's new identity: {:?}",
        ctx.tagged_objects
    );
    assert_eq!(
        game.current_card_types(returned_id),
        Some(vec![CardType::Enchantment]),
        "the returned permanent should be an enchantment and not a creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_burst_lightning_kicker_instead_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Burst Lightning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {4} (You may pay an additional {4} as you cast this spell.)\nThis spell deals 2 damage to any target. If this spell was kicked, it deals 4 damage instead.",
        )
        .expect("kicker damage-instead followup should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("If this spell was kicked, it deals 4 damage instead"),
        "non-shuffle self-replacements should keep suffix 'instead': {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_geistblast_graveyard_copy_activation_renders_cleanly() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geistblast")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Geistblast deals 2 damage to any target.\n{2}{U}, Exile this card from your graveyard: Copy target instant or sorcery spell you control. You may choose new targets for the copy.",
        )
        .expect("Geistblast should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        (rendered.contains("{2}{U}, Exile this card from your graveyard:")
            || rendered.contains("{2}{U}, Exile Geistblast:"))
            && rendered.contains("Copy target instant or sorcery spell you control")
            && rendered.contains("You may choose new targets for the copy"),
        "expected Geistblast graveyard activation to render in Oracle style, got {rendered}"
    );
    assert!(
        !rendered.contains("Exile this spell")
            && !rendered.contains("instant and sorcery")
            && !rendered.contains("time(s)")
            && !rendered.contains(". you may"),
        "expected Geistblast rendering to avoid copy-spell compatibility artifacts, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_consult_the_star_charts_kicker_count_override() {
    CardDefinitionBuilder::new(CardId::new(), "Consult Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)\nLook at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.",
        )
        .expect("look-top X kicker count override should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_see_the_truth_cast_non_hand_self_replacement() {
    let def = parse_oracle_card_definition("See the Truth");
    let program = def
        .spell_effect
        .as_ref()
        .expect("See the Truth spell effect");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
    assert_eq!(
        program.segments[0].self_replacements[0].condition,
        crate::effect::Condition::ThisSpellWasCastFromNonHand,
        "See the Truth should branch on being cast from a non-hand zone"
    );
    assert_eq!(def.name(), "See the Truth");
    assert_eq!(
        rendered,
        "Look at the top 3 cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order. If this spell was cast from anywhere other than your hand, put each of those cards into your hand instead.",
        "expected See the Truth compiled text to preserve the exact non-hand all-cards replacement"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_aangs_journey_kicked_search_slots_as_self_replacement() {
    let def = parse_oracle_card_definition("Aang's Journey");
    assert_eq!(
        canonical_compiled_lines(&def),
        [
            "Kicker {2}",
            "Search your library for a basic land card. If this spell was kicked, instead search your library for a basic land card and a Shrine card. Reveal those cards, put them into your hand, then shuffle.",
            "You gain 2 life."
        ]
    );
    let program = def
        .spell_effect
        .as_ref()
        .expect("Aang's Journey spell effect");
    let [search_segment, life_segment] = program.segments.as_slice() else {
        panic!("expected search replacement plus life gain: {program:#?}");
    };
    let [default_effect] = search_segment.default_effects.as_slice() else {
        panic!("expected one default search: {search_segment:#?}");
    };
    let default = default_effect
        .downcast_ref::<crate::effects::SearchLibrarySlotsEffect>()
        .expect("default typed slot search");
    let [branch] = search_segment.self_replacements.as_slice() else {
        panic!("expected one kicked self-replacement: {search_segment:#?}");
    };
    assert_eq!(branch.condition, Condition::ThisSpellWasKicked);
    assert!(branch.leading_instead_surface);
    let [replacement_effect] = branch.replacement_effects.as_slice() else {
        panic!("expected one replacement search: {branch:#?}");
    };
    let replacement = replacement_effect
        .downcast_ref::<crate::effects::SearchLibrarySlotsEffect>()
        .expect("replacement typed slot search");
    assert_eq!(default.slots.len(), 1);
    assert_eq!(replacement.slots.len(), 2);
    assert!(replacement.slots.starts_with(&default.slots));
    assert_eq!(replacement.destination, default.destination);
    assert_eq!(replacement.chooser, default.chooser);
    assert_eq!(replacement.player, default.player);
    assert_eq!(replacement.reveal, default.reveal);
    assert_eq!(replacement.progress_tag, default.progress_tag);
    let [life_effect] = life_segment.default_effects.as_slice() else {
        panic!("expected one independent life gain: {life_segment:#?}");
    };
    let life = life_effect
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .expect("typed life gain");
    assert_eq!(life.amount.unhinted(), &Value::Fixed(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_entish_restoration_shared_shuffle_places_instead_before_search() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Entish Restoration Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Sacrifice a land. Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle. If you control a creature with power 4 or greater, instead search your library for up to three basic land cards, put them onto the battlefield tapped, then shuffle.",
        )
        .expect("shared-shuffle conditional search should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "If you control a creature with power 4 or greater, instead search your library for up to three basic land cards"
        ),
        "multi-effect replacement should put 'instead' before the search: {rendered}"
    );
    assert!(
        !rendered.contains("shuffle instead"),
        "shared terminal shuffle must not absorb the replacement marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_from_under_the_floorboards_madness_self_replacement() {
    let def = parse_oracle_card_definition("From Under the Floorboards");
    let debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("ThisSpellPaidLabel")
            && debug.contains("Madness")
            && debug.contains("CreateTokenEffect")
            && debug.contains("enters_tapped: true")
            && debug.contains("GainLifeEffect"),
        "expected From Under the Floorboards to lower madness as a tapped-token/life self-replacement, got {debug}"
    );
    assert!(
        rendered.contains("Create three tapped 2/2 black Zombie creature tokens and you gain 3 life")
            && rendered.contains("If this spell's madness cost was paid, instead create X tapped 2/2 black Zombie creature tokens and you gain X life")
            && rendered.contains("Madness {X}{B}{B}")
            && !rendered.contains("token tokens"),
        "expected From Under the Floorboards compiled text to preserve the madness replacement clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kicked_multi_zone_search_to_battlefield_as_self_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "The Five Doctors Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Kicker {5} (You may pay an additional {5} as you cast this spell.)\nSearch your library and/or graveyard for up to five Doctor cards, reveal them, and put them into your hand. If you search your library this way, shuffle. If this spell was kicked, put those cards onto the battlefield instead of putting them into your hand.",
        )
        .expect("kicked multi-zone search replacement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("ThisSpellWasKicked")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Battlefield"),
        "expected kicked multi-zone search override to lower as self-replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_back_for_seconds_bargain_optional_local_rewrite() {
    let def = parse_oracle_card_definition("Back for Seconds");
    let debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("ThisSpellPaidLabel")
            && debug.contains("RegisterZoneReplacementEffect")
            && debug.contains("optional: true"),
        "expected Back for Seconds bargain clause to lower as an optional local zone rewrite, got {debug}"
    );
    assert!(
        rendered.contains("If this spell was bargained, you may put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand")
            && !rendered.contains("you may return it to its owner's hand"),
        "expected Back for Seconds to render the bargain self-replacement, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kirtars_wrath_threshold_destroy_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kirtar's Wrath Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy all creatures. They can't be regenerated.\nThreshold \u{2014} If there are seven or more cards in your graveyard, instead destroy all creatures, then create two 1/1 white Spirit creature tokens with flying. Creatures destroyed this way can't be regenerated.",
        )
        .expect("threshold destroy replacement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch")
            && (debug.contains("PlayerHasCardTypesInGraveyardOrMore")
                || debug.contains("ValueComparison"))
            && debug.contains("CreateTokenEffect"),
        "expected threshold destroy upgrade to lower as self-replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_revolt_counter_upgrade_reuses_trailing_instead_if_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "That's Rough Buddy Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Put a +1/+1 counter on target creature. Put two +1/+1 counters on that creature instead if a creature left the battlefield under your control this turn.\nDraw a card.",
        )
        .expect("revolt counter replacement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("PermanentLeftBattlefieldUnderYourControlThisTurn"),
        "expected counter upgrade to lower as self-replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_void_pump_upgrade_as_self_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tragic Trajectory Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target creature gets -2/-2 until end of turn.\nVoid \u{2014} That creature gets -10/-10 until end of turn instead if a nonland permanent left the battlefield this turn or a spell was warped this turn.",
        )
        .expect("void pump replacement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("NonlandPermanentLeftBattlefieldThisTurn")
            && debug.contains("SpellWasWarpedThisTurn"),
        "expected void pump upgrade to lower as self-replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_void_draw_life_upgrade_renders_named_void_self_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Decode Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You draw two cards and lose 2 life.\nVoid \u{2014} If a nonland permanent left the battlefield this turn or a spell was warped this turn, instead you draw two cards and each opponent loses 2 life.",
        )
        .expect("void draw/life replacement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch")
            && debug.contains("NonlandPermanentLeftBattlefieldThisTurn")
            && debug.contains("SpellWasWarpedThisTurn"),
        "expected void draw/life upgrade to lower as self-replacement, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "Void — If a nonland permanent left the battlefield this turn or a spell was warped this turn, instead draw two cards and each opponent loses 2 life"
        ),
        "expected void renderer to keep the named replacement line, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn consult_the_star_charts_kicker_override_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Consult Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)\nLook at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead this turn. Put the rest on the bottom of your library in a random order.",
        )
        .expect_err("unsupported kicked looked-card tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported kicked looked-card tail, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_kindred_summons_shuffle_remainder() {
    let def = parse_oracle_card_definition("Kindred Summons");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "choose a creature type. reveal cards from the top of your library until you reveal x creature cards of the chosen type, where x is the number of creatures you control of that type. put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library"
        ),
        "expected Kindred Summons to preserve its linked counted collection, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_selvalas_stampede_preserves_vote_scoped_collection() {
    let def = parse_oracle_card_definition("Selvala's Stampede");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Council's dilemma — Starting with you, each player votes for wild or free. Reveal cards from the top of your library until you reveal a creature card for each wild vote. Put those creature cards onto the battlefield, then shuffle the rest into your library. You may put a permanent card from your hand onto the battlefield for each free vote"
        ),
        "expected Selvala's Stampede to preserve both vote-scoped collections, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_mass_polymorph_shuffle_remainder() {
    let def = parse_oracle_card_definition("Mass Polymorph");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        "Mass Polymorph should preserve its count provenance and both revealed-card partitions"
    );
}

#[test]
pub(super) fn parse_oracle_synthetic_destiny_delays_full_shuffle_remainder_bundle() {
    let def = parse_oracle_card_definition("Synthetic Destiny");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Exile all creatures you control. At the beginning of the next end step, reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        "Synthetic Destiny should preserve the delayed reveal, battlefield partition, and shuffled remainder as one provenance-linked bundle"
    );
}

#[test]
pub(super) fn parse_oracle_fathom_trawl_moves_the_matched_collection_without_loop_scaffolding() {
    let def = parse_oracle_card_definition("Fathom Trawl");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Reveal cards from the top of your library until you reveal three nonland cards. Put the nonland cards revealed this way into your hand, then put the rest of the revealed cards on the bottom of your library in any order.",
        "Fathom Trawl should render its tagged hits and tagged remainder as collections"
    );
}

#[test]
pub(super) fn parse_oracle_kethek_keeps_revealed_hit_and_remainder_in_one_clause() {
    let def = parse_oracle_card_definition("Kethek, Crucible Goliath");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "At the beginning of your end step, you may sacrifice another creature. If you do, reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value. Put it onto the battlefield, then put the rest on the bottom of your library in a random order.",
        "Kethek should render the consult match and remainder without exposing the per-object loop"
    );
}

#[test]
pub(super) fn parse_oracle_spinner_keeps_optional_singular_consult_surface() {
    let def = parse_oracle_card_definition("Spinner of Souls");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Reach Whenever another nontoken creature you control dies, you may reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        "a direct singular consult move should retain its established optional surface and sentence boundary"
    );
}

#[test]
pub(super) fn parse_oracle_gonti_preserves_looked_partition_and_cast_permission() {
    let def = parse_oracle_card_definition("Gonti, Lord of Luxury");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Deathtouch When Gonti enters, look at the top four cards of target opponent's library, exile one of them face down, then put the rest on the bottom of that library in a random order. You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell.",
        "Gonti's complete target/look/partition/permission program should reach the exact singular renderer"
    );
}

#[test]
pub(super) fn debug_surface_keeps_complex_source_text_regressions() {
    let cases = [
        (
            "Divergent Transformations",
            "Exile two target creatures",
            "Spells cost {X} less to cast",
        ),
        (
            "Mirror of Life Trapping",
            "Whenever a creature enters",
            "return all other permanent cards exiled with this artifact to the battlefield under their owners' control",
        ),
        (
            "Gandalf, Westward Voyager",
            "Whenever you cast a spell with mana value 5 or greater",
            "Otherwise, draw a card",
        ),
        (
            "Necromentia",
            "Choose a card name other than a basic land card name",
            "for each card exiled from their hand this way",
        ),
    ];

    for (name, first, second) in cases {
        let def = parse_oracle_card_definition(name);
        let rendered = debug_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains(first)
                && rendered.contains(second)
                && !rendered.to_ascii_lowercase().contains("unsupported"),
            "expected {name} debug text to render AST-owned fragments '{first}' and '{second}', got {rendered}"
        );
    }

    let delifs_cone = debug_compiled_lines(&parse_oracle_card_definition("Delif's Cone"))
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        delifs_cone.contains("attacks and isn't blocked")
            && delifs_cone.contains("gain life equal to its power")
            && delifs_cone.contains("it assigns no combat damage this turn")
            && !delifs_cone.contains("unsupported"),
        "expected Delif's Cone debug text to preserve the unblocked delayed trigger, life gain, and damage-prevention follow-up, got {delifs_cone}"
    );

    let demon = parse_oracle_card_definition("Burning-Rune Demon");
    let rendered = debug_compiled_lines(&demon).join("\n");
    assert!(
        rendered.contains("Flying"),
        "expected Flying, got {rendered}"
    );
    assert_eq!(
        rendered.matches("When this creature enters").count(),
        1,
        "expected one Burning-Rune Demon trigger, got {rendered}"
    );
}

#[test]
pub(super) fn colfenors_urn_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Colfenor's Urn");
    let rendered = debug_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported"),
        "Colfenor's Urn should strictly parse without unsupported output, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever a creature with toughness 4 or greater is put into your graveyard from the battlefield, you may exile it."
        ),
        "expected Colfenor's Urn death trigger text, got {rendered}"
    );
    assert!(
        rendered.contains("if three or more cards have been exiled with this artifact")
            || rendered.contains("if there are three or more cards exiled with this artifact"),
        "expected source-linked exile count condition, got {rendered}"
    );
    assert!(
        rendered.contains(
            "sacrifice it. If you do, return those cards to the battlefield under their owners' control"
        ),
        "expected source sacrifice and source-linked return text, got {rendered}"
    );
    assert!(
        ability_debug.contains("ValueComparison")
            && ability_debug.contains("__source_exiled__")
            && ability_debug.contains("SacrificeTargetEffect")
            && ability_debug.contains("target: Source")
            && ability_debug.contains("ReturnAllToBattlefieldEffect"),
        "expected Colfenor's Urn to structurally count and return cards exiled with its source, got {ability_debug}"
    );
}

#[test]
pub(super) fn necromentia_counts_only_cards_exiled_from_hand_this_way() {
    struct NecromentiaDecisionMaker;

    impl crate::decision::DecisionMaker for NecromentiaDecisionMaker {
        fn decide_text(
            &mut self,
            _game: &crate::GameState,
            _ctx: &crate::decisions::context::TextInputContext,
        ) -> String {
            "Duress".to_string()
        }

        fn decide_objects(
            &mut self,
            _game: &crate::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.max.unwrap_or(ctx.candidates.len()))
                .collect()
        }
    }

    fn simple_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build()
    }

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = parse_oracle_card_definition("Necromentia");
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);

    let duress = simple_card("Duress");
    let opt = simple_card("Opt");
    game.create_object_from_card(&duress, bob, Zone::Hand);
    game.create_object_from_card(&duress, bob, Zone::Graveyard);
    game.create_object_from_card(&duress, bob, Zone::Library);
    game.create_object_from_card(&opt, bob, Zone::Hand);
    game.create_object_from_card(&opt, bob, Zone::Exile);

    let mut dm = NecromentiaDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        def.spell_effect.as_ref().expect("Necromentia spell effect"),
        None,
        &[],
    )
    .expect("Necromentia should resolve");

    let zombie_count = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| game.controller_of(object) == bob && object.name == "Zombie")
        .count();
    assert_eq!(
        zombie_count, 1,
        "Necromentia should create one Zombie only for the matching card exiled from hand"
    );

    let exiled_duress_count = game
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|object| {
            object.owner == bob && object.name == "Duress" && object.zone == Zone::Exile
        })
        .count();
    assert_eq!(
        exiled_duress_count, 3,
        "Necromentia should exile matching cards from hand, graveyard, and library"
    );
}

#[test]
pub(super) fn parse_oracle_necromentia_uses_shared_subject_role_lowering() {
    let def = parse_oracle_card_definition("Necromentia");
    let program = def.spell_effect.as_ref().expect("Necromentia spell effect");
    let mut effects = Vec::new();
    for effect in program.flattened_default_effects() {
        effects.push(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            effects.extend(sequence.effects.iter());
        }
    }

    let choose_name = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseCardNameEffect>())
        .expect("Necromentia should start by choosing a card name");
    assert_eq!(choose_name.chooser, PlayerFilter::You);
    let choose_name_filter = choose_name
        .filter
        .as_ref()
        .expect("Necromentia should carry the non-basic-land name restriction");
    assert!(
        choose_name_filter.any_of.iter().any(|filter| {
            filter.excluded_card_types.contains(&CardType::Land)
                || filter
                    .excluded_supertypes
                    .contains(&crate::types::Supertype::Basic)
        }),
        "chosen-name restriction should exclude basic land names, got {choose_name_filter:#?}"
    );

    let search = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("Necromentia should lower the multi-zone search through ChooseObjectsEffect");
    assert!(
        search.is_search,
        "search choice should be marked as search: {search:#?}"
    );
    assert_eq!(search.chooser, PlayerFilter::You);
    assert_eq!(search.filter.owner, Some(PlayerFilter::target_opponent()));
    assert_eq!(search.zone, Some(Zone::Graveyard));
    assert_eq!(search.additional_zones, vec![Zone::Hand, Zone::Library]);
    assert_eq!(search.count.min, 0);
    assert_eq!(search.count.max, None);
    assert!(
        search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "__chosen_name__"
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                )
        }),
        "search filter should use the chosen-name tag, got {search:#?}"
    );

    let move_to_exile = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ForEachTaggedEffect>())
        .and_then(|for_each| for_each.effects.first())
        .and_then(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .map(|tagged| tagged.effect.as_ref())
                .or(Some(effect))
        })
        .and_then(|effect| effect.downcast_ref::<MoveToZoneEffect>())
        .expect("Necromentia should move searched cards to exile through tagged iteration");
    assert_eq!(move_to_exile.zone, Zone::Exile);
    assert!(
        matches!(move_to_exile.target, ChooseSpec::Tagged(_)),
        "exile move should consume the searched-card tag, got {move_to_exile:#?}"
    );

    let shuffle = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>())
        .expect("Necromentia should shuffle that opponent's library");
    assert_eq!(shuffle.player, PlayerFilter::target_opponent());

    let create = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| tagged.effect.downcast_ref::<CreateTokenEffect>())
        })
        .expect("Necromentia should create Zombie tokens for that player");
    assert!(
        matches!(
            &create.controller,
            PlayerFilter::Target(player) | PlayerFilter::AliasedTarget(player)
                if **player == PlayerFilter::Opponent
        ),
        "the token controller should remain the targeted opponent: {create:#?}"
    );
    assert_eq!(create.token.card.name, "Zombie");
    assert_eq!(create.token.card.card_types, vec![CardType::Creature]);
    assert!(create.token.card.subtypes.contains(&Subtype::Zombie));
    match create.count.unhinted() {
        Value::Count(filter) => {
            assert_eq!(filter.zone, Some(Zone::Hand));
            assert!(
                matches!(
                    filter.owner.as_ref(),
                    Some(PlayerFilter::Target(player) | PlayerFilter::AliasedTarget(player))
                        if **player == PlayerFilter::Opponent
                ),
                "the hand-count owner should remain the targeted opponent: {filter:#?}"
            );
            assert!(
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag == search.tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                }),
                "token count should count cards searched/exiled from that player's hand, got {filter:#?}"
            );
        }
        other => panic!("Zombie count should be a tagged hand-count value, got {other:#?}"),
    }

    let compiler_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("crates/ironsmith-compiler/src/runtime_backend");
    let mut stack = vec![compiler_src];
    let mut card_specific_hits = Vec::new();
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).expect("read compiler source") {
            let path = entry.expect("read compiler entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.extension().is_some_and(|ext| ext == "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read compiler file");
            for (line_index, line) in source.lines().enumerate() {
                if line.to_ascii_lowercase().contains("necromentia") {
                    card_specific_hits.push(format!("{}:{}", path.display(), line_index + 1));
                }
            }
        }
    }
    assert!(
        card_specific_hits.is_empty(),
        "Necromentia must compile through generic subject/choice/search/token systems, found card-specific compiler hooks: {card_specific_hits:?}"
    );
}

#[test]
pub(super) fn parse_oracle_garruk_emblem_search_stays_inside_quoted_text() {
    let cases = [
        (
            "Garruk, Unleashed",
            "You get an emblem with \"At the beginning of your end step, you may search your library for a creature card, put it onto the battlefield, then shuffle.\"",
        ),
        (
            "Garruk, Caller of Beasts",
            "You get an emblem with \"Whenever you cast a creature spell, you may search your library for a creature card, put it onto the battlefield, then shuffle.\"",
        ),
    ];

    for (name, expected_text) in cases {
        let def = parse_oracle_card_definition(name);
        let rendered = debug_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains(expected_text),
            "expected {name} to keep quoted emblem search text, got {rendered}"
        );
        assert!(
            !rendered.contains("you.\". You may search"),
            "expected {name} not to split the quoted search clause out of the emblem, got {rendered}"
        );

        let mut found_emblem = false;
        for ability in &def.abilities {
            let AbilityKind::Activated(activated) = &ability.kind else {
                continue;
            };
            let default_effects = activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .collect::<Vec<_>>();
            let emblem_effects = default_effects
                .iter()
                .filter_map(|effect| effect.downcast_ref::<crate::effects::CreateEmblemEffect>())
                .collect::<Vec<_>>();
            if emblem_effects.is_empty() {
                continue;
            }

            found_emblem = true;
            assert_eq!(
                emblem_effects.len(),
                1,
                "expected one emblem effect for {name}, got {default_effects:#?}"
            );
            assert_eq!(
                default_effects.len(),
                1,
                "quoted search should stay in the emblem, not become a sibling effect for {name}: {default_effects:#?}"
            );
            let emblem = emblem_effects[0];
            assert!(
                emblem.emblem.text.contains("may search your library"),
                "emblem text should retain search clause for {name}: {:#?}",
                emblem.emblem
            );
            assert!(
                !emblem.emblem.abilities.is_empty(),
                "emblem rules text should compile into emblem abilities for {name}: {:#?}",
                emblem.emblem
            );
            assert!(
                default_effects
                    .iter()
                    .all(|effect| effect.downcast_ref::<crate::effects::MayEffect>().is_none()),
                "parent ability should not contain escaped MayEffect for {name}: {default_effects:#?}"
            );
        }
        assert!(found_emblem, "expected to find emblem effect for {name}");
    }
}

#[test]
pub(super) fn parse_oracle_tezzeret_emblem_conditional_stays_inside_quoted_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tezzeret, Cruel Captain")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "Whenever an artifact you control enters, put a loyalty counter on Tezzeret.\n\
0: Untap target artifact or creature. If it's an artifact creature, put a +1/+1 counter on it.\n\
−3: Search your library for an artifact card with mana value 1 or less, reveal it, put it into your hand, then shuffle.\n\
−7: You get an emblem with \"At the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. If it's not a creature, it becomes a 0/0 Robot artifact creature.\"",
        )
        .expect("Tezzeret should parse");
    let rendered = debug_compiled_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    let expected = "−7: you get an emblem with \"at the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. if it's not a creature, it becomes a 0/0 robot artifact creature.\"";
    assert!(
        rendered_lower.contains(expected),
        "expected Tezzeret emblem to keep the quoted conditional, got {rendered}"
    );
    assert!(
        !rendered.contains("If not, You get an emblem"),
        "quoted conditional should not escape as a parent condition, got {rendered}"
    );

    let mut found_emblem = false;
    for ability in &def.abilities {
        let AbilityKind::Activated(activated) = &ability.kind else {
            continue;
        };
        let default_effects = activated
            .effects
            .segments
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
            .collect::<Vec<_>>();
        let emblem_effects = default_effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<crate::effects::CreateEmblemEffect>())
            .collect::<Vec<_>>();
        if emblem_effects.is_empty() {
            continue;
        }

        found_emblem = true;
        assert_eq!(
            emblem_effects.len(),
            1,
            "expected one direct emblem effect, got {default_effects:#?}"
        );
        assert_eq!(
            default_effects.len(),
            1,
            "quoted conditional should stay inside the emblem, not wrap it: {default_effects:#?}"
        );
        let emblem = emblem_effects[0];
        assert!(
            emblem.emblem.text.contains("if it's not a creature"),
            "emblem text should retain the conditional sentence: {:#?}",
            emblem.emblem
        );
        let emblem_abilities_debug = format!("{:?}", emblem.emblem.abilities);
        assert!(
            emblem_abilities_debug.contains("ApplyContinuousEffect")
                && emblem_abilities_debug.contains("AddCardTypes")
                && emblem_abilities_debug.contains("SetPowerToughness")
                && emblem_abilities_debug.contains("Robot"),
            "emblem rules text should compile the conditional animation effect: {:#?}",
            emblem.emblem
        );
    }

    assert!(found_emblem, "expected to find Tezzeret emblem effect");
}

#[test]
pub(super) fn parse_oracle_garruk_caller_reveal_top_matching_creatures_to_hand() {
    let def = parse_oracle_card_definition("Garruk, Caller of Beasts");
    let rendered = debug_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "Reveal the top five cards of your library. Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order"
        ),
        "expected Garruk Caller +1 to preserve reveal-top matching creature clause, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_garruk_unleashed_compacts_pump_and_trample() {
    let def = parse_oracle_card_definition("Garruk, Unleashed");
    let rendered = debug_compiled_lines(&def).join("\n");

    assert!(
        rendered
            .contains("Up to one target creature gets +3/+3 and gains trample until end of turn"),
        "expected Garruk Unleashed +1 to render shared pump/trample duration, got {rendered}"
    );
    assert!(
        !rendered.contains("get +3/+3 until end of turn. it gains Trample"),
        "expected Garruk Unleashed +1 not to split tagged trample grant, got {rendered}"
    );
}

#[test]
pub(super) fn mandatory_pump_trample_spells_keep_mandatory_target_surface() {
    for name in [
        "Awaken the Bear",
        "Blitzball Shot",
        "Crash the Ramparts",
        "Fanatical Strength",
        "Predator's Strike",
        "Staggering Size",
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = debug_compiled_lines(&def).join("\n");

        assert!(
            rendered.contains("Target creature gets +3/+3 and gains trample until end of turn"),
            "{name} should retain its mandatory target surface, got {rendered}"
        );
        assert!(
            !rendered.contains(
                "Up to one target creature gets +3/+3 and gains trample until end of turn"
            ),
            "{name} must not be rewritten as an optional-target spell, got {rendered}"
        );
    }
}

#[test]
pub(super) fn pump_trample_target_requirements_distinguish_mandatory_and_up_to_one() {
    let mandatory = CardDefinitionBuilder::new(CardId::new(), "Mandatory Pump Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +3/+3 and gains trample until end of turn.")
        .expect("mandatory pump/trample spell should parse");
    let optional = CardDefinitionBuilder::new(CardId::new(), "Optional Pump Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Up to one target creature gets +3/+3 and gains trample until end of turn.")
        .expect("optional pump/trample spell should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Target Creature")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_definition(&creature, alice, Zone::Battlefield);

    for (definition, expected_min) in [(&mandatory, 1usize), (&optional, 0usize)] {
        let source = game.create_object_from_definition(definition, alice, Zone::Stack);
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            definition
                .spell_effect
                .as_ref()
                .expect("pump/trample spell should have a resolution program"),
            alice,
            Some(source),
            None,
        );

        assert_eq!(requirements.len(), 1, "{definition:#?}");
        assert_eq!(requirements[0].min_targets, expected_min, "{definition:#?}");
        assert_eq!(requirements[0].max_targets, Some(1), "{definition:#?}");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_tainted_strike_compiles_strictly() {
    let def = parse_oracle_card_definition("Tainted Strike");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Target creature gets +1/+0 and gains infect until end of turn"),
        "expected Tainted Strike to keep shared pump/infect clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_inkmoth_nexus_animation_keeps_types_and_keywords() {
    let def = parse_oracle_card_definition("Inkmoth Nexus");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "{1}: This land becomes a 1/1 Phyrexian Blinkmoth artifact creature with flying and infect until end of turn. It's still a land"
        ),
        "expected Inkmoth Nexus source animation to preserve artifact type, subtypes, keywords, and still-land text, got {rendered}"
    );

    let activated = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| activated.mana_cost.display() == "{1}")
        .expect("Inkmoth Nexus should have a {1} animation ability");
    let apply = activated.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("Inkmoth animation should lower to one continuous effect");

    assert!(
        matches!(
            apply.modification.as_ref(),
            Some(crate::continuous::Modification::AddCardTypes(card_types))
                if card_types.contains(&CardType::Creature)
                    && card_types.contains(&CardType::Artifact)
        ),
        "expected Inkmoth animation to add artifact creature types, got {apply:#?}"
    );
    assert!(
        apply.additional_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::continuous::Modification::AddSubtypes(subtypes)
                    if subtypes.contains(&Subtype::Phyrexian)
                        && subtypes.contains(&Subtype::Blinkmoth)
            )
        }) && apply.additional_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::continuous::Modification::AddAbility(ability)
                    if ability.id() == StaticAbilityId::Flying
            )
        }) && apply.additional_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::continuous::Modification::AddAbility(ability)
                    if ability.id() == StaticAbilityId::Infect
            )
        }),
        "expected Inkmoth animation to add Phyrexian Blinkmoth, flying, and infect, got {apply:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_mutavault_animation_keeps_all_creature_types() {
    let def = parse_oracle_card_definition("Mutavault");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land"
        ),
        "expected Mutavault source animation to preserve all creature types and still-land text, got {rendered}"
    );

    let activated = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| activated.mana_cost.display() == "{1}")
        .expect("Mutavault should have a {1} animation ability");
    let apply = activated.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("Mutavault animation should lower to one continuous effect");

    assert!(
        apply.additional_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::continuous::Modification::AddAllSubtypesOfFamily(
                    crate::types::SubtypeFamily::Creature
                )
            )
        }),
        "expected Mutavault animation to add every creature subtype, got {apply:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_neighboring_manlands_keep_animation_details() {
    for (name, expected) in [
        (
            "Soulstone Sanctuary",
            "{4}: this land becomes a 3/3 creature with vigilance and all creature types. it's still a land",
        ),
        (
            "Faceless Haven",
            "{s}{s}{s}: this land becomes a 4/3 creature with vigilance and all creature types until end of turn. it's still a land",
        ),
        (
            "Dread Statuary",
            "{4}: this land becomes a 4/2 golem artifact creature until end of turn. it's still a land",
        ),
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&def)
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(expected),
            "expected {name} animation to preserve manland details, got {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mutavault_runtime_animation_grants_all_creature_types_until_eot() {
    let def = parse_oracle_card_definition("Mutavault");
    let activated = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| activated.mana_cost.display() == "{1}")
        .expect("Mutavault should have a {1} animation ability");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        game.current_card_types(source)
            .is_some_and(|types| types == vec![CardType::Land]),
        "Mutavault should start as only a land"
    );
    assert!(!game.current_has_subtype(source, Subtype::Elf));

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Mutavault animation should resolve");

    assert!(
        game.current_card_types(source).is_some_and(|types| {
            types.contains(&CardType::Land) && types.contains(&CardType::Creature)
        }),
        "Mutavault should remain a land and become a creature"
    );
    assert_eq!(game.current_power(source), Some(2));
    assert_eq!(game.current_toughness(source), Some(2));
    assert!(game.current_has_subtype(source, Subtype::Elf));
    assert!(game.current_has_subtype(source, Subtype::Goblin));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn score_improver_target_set_cauldron_of_souls_preserves_persist_set() {
    assert_oracle_card_parses_strict("Cauldron of Souls");
    let def = parse_oracle_card_definition("Cauldron of Souls");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Choose any number of target creatures")
            && rendered.contains("Each of them gains"),
        "expected Cauldron of Souls to render the persistent target set, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("targetonlyeffect")
            && debug.contains("applycontinuouseffect")
            && debug.contains("targeted_0")
            && debug.contains("persist"),
        "expected Cauldron of Souls to target once and apply persist to that tagged set, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn score_improver_target_set_run_away_together_preserves_those_return_reference() {
    let def = parse_oracle_card_definition("Run Away Together");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Choose two target creatures")
            && rendered.contains("Return those creatures to their owners' hands"),
        "expected Run Away Together to reuse the target set for the return clause, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("targetonlyeffect")
            && debug.contains("returntohandeffect")
            && debug.contains("targeted_0"),
        "expected Run Away Together to lower return-those-creatures through the tagged target set, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn score_improver_target_set_hog_monkey_rampage_preserves_counter_and_fight() {
    assert_oracle_card_parses_strict("Hog-Monkey Rampage");
    let def = parse_oracle_card_definition("Hog-Monkey Rampage");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Choose target creature you control and target creature an opponent controls"
        ) && rendered.contains("Put a +1/+1 counter on the creature you control")
            && rendered.contains("Then those creatures fight each other"),
        "expected Hog-Monkey Rampage to preserve the two-target counter/fight sequence, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.matches("targetonlyeffect").count() >= 2
            && debug.contains("conditionaleffect")
            && debug.contains("putcounterseffect")
            && debug.contains("fighteffect")
            && debug.contains("targeted_0")
            && debug.contains("targeted_1"),
        "expected Hog-Monkey Rampage to target both creatures once and reuse them for counter/fight, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn score_improver_target_set_ever_after_applies_color_and_type_to_returned_set() {
    assert_oracle_card_parses_strict("Ever After");
    let def = parse_oracle_card_definition("Ever After");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Return up to two target creature cards from your graveyard to the battlefield"
        ) && rendered.contains("Each of those creatures is a black Zombie"),
        "expected Ever After to apply black Zombie to the returned set, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("returnfromgraveyardtobattlefieldeffect")
            && debug.contains("applycontinuouseffect")
            && debug.contains("returned_0")
            && debug.contains("addcolors")
            && debug.contains("zombie"),
        "expected Ever After to tag returned creatures before color/type modification, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn score_improver_target_set_fall_of_the_titans_preserves_surge_and_damage() {
    assert_oracle_card_parses_strict("Fall of the Titans");
    let def = parse_oracle_card_definition("Fall of the Titans");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Fall of the Titans deals X damage to each of up to two targets")
            && rendered.contains("Surge {X}{R}"),
        "expected Fall of the Titans to preserve Surge and counted any-target damage, got {rendered}"
    );

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    let surge_debug = format!("{:#?}", def.alternative_casts).to_ascii_lowercase();
    assert!(
        spell_debug.contains("dealdamageeffect")
            && spell_debug.contains("withcount")
            && spell_debug.contains("anytarget")
            && spell_debug.contains("min: 0")
            && spell_debug.contains("max: some")
            && spell_debug.contains("2")
            && surge_debug.contains("surge"),
        "expected Fall of the Titans to model up-to-two any-target damage plus named Surge, got spell={spell_debug}; surge={surge_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tainted_strike_runtime_grants_infect_and_pump_until_end_of_turn() {
    let def = parse_oracle_card_definition("Tainted Strike");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Tainted Strike should produce spell effects")
        .clone();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(93_001), "Test Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    assert_eq!(game.current_power(target), Some(2));
    assert!(
        !game.object_has_static_ability_id(target, StaticAbilityId::Infect),
        "target should not start with infect"
    );

    let normal_damage = crate::rules::damage::apply_processed_damage_assignment(
        &mut game,
        target,
        crate::events::DamageTarget::Player(bob),
        2,
        crate::rules::damage::SourceDamageKeywords::default(),
        crate::events::cause::EventCause::from_effect(target, alice),
    );
    assert!(
        normal_damage.applied,
        "baseline damage assignment to player should apply"
    );
    assert_eq!(
        game.players[1].life, 18,
        "non-infect damage should reduce life"
    );
    assert_eq!(
        game.players[1].poison_counters, 0,
        "non-infect damage should not add poison counters"
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_creature(),
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &spell,
        None,
        &[],
    )
    .expect("Tainted Strike should resolve");

    assert_eq!(game.current_power(target), Some(3));
    assert!(game.object_has_static_ability_id(target, StaticAbilityId::Infect));

    let keywords = crate::rules::damage::SourceDamageKeywords {
        has_infect: true,
        ..crate::rules::damage::SourceDamageKeywords::default()
    };
    let player_damage = crate::rules::damage::apply_processed_damage_assignment(
        &mut game,
        target,
        crate::events::DamageTarget::Player(bob),
        3,
        keywords,
        crate::events::cause::EventCause::from_effect(target, alice),
    );
    assert!(
        player_damage.applied,
        "damage assignment to player should apply"
    );
    assert_eq!(
        game.players[1].life, 18,
        "infect damage to player should not reduce life"
    );
    assert_eq!(
        game.players[1].poison_counters, 3,
        "infect damage should add poison counters"
    );

    let blocker = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(93_002), "Test Blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let creature_damage = crate::rules::damage::apply_processed_damage_assignment(
        &mut game,
        target,
        crate::events::DamageTarget::Object(blocker),
        3,
        keywords,
        crate::events::cause::EventCause::from_effect(target, alice),
    );
    assert!(
        creature_damage.applied,
        "damage assignment to creature should apply"
    );
    assert_eq!(
        game.object(blocker).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::MinusOneMinusOne)
            .copied()),
        Some(3),
        "infect damage to creature should use -1/-1 counters"
    );
}
