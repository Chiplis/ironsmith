//! CR 601.2b: a modal spell with {X} announces X ahead of its modes, so each
//! mode's legality is read under the X it will be cast with.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    DecisionContext, NumberContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definition(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}

fn kozileks_command() -> CardDefinition {
    definition(
        "Kozilek's Command",
        "Mana cost: {X}{C}{C}\n\
         Type: Kindred Instant — Eldrazi\n\
         Choose two —\n\
         • Target player creates X 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this token: Add {C}.\"\n\
         • Target player scries X, then draws a card.\n\
         • Exile target creature with mana value X or less.\n\
         • Exile up to X target cards from graveyards.",
    )
}

fn bear() -> CardDefinition {
    definition(
        "Runtime Bear",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
    )
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for symbol in [ManaSymbol::Colorless, ManaSymbol::Black] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 10);
    }
    game
}

/// Announces `x`, picks `modes`, and aims every target at `bear` when it is
/// legal (players otherwise).
struct Choices {
    x: u32,
    modes: Vec<usize>,
    bear: Option<ObjectId>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode for") {
            return self.modes.clone();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }

    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        ctx.requirements
            .iter()
            .map(|requirement| match self.bear.map(Target::Object) {
                Some(bear) if requirement.legal_targets.contains(&bear) => bear,
                _ => Target::Player(A),
            })
            .collect()
    }
}

#[derive(Debug, PartialEq)]
enum Prompt {
    X,
    Modes(Vec<bool>),
}

/// Casts `spell`, recording the X and mode prompts in the order they are
/// asked, until the cast completes or `stop_at_modes` reaches the mode prompt.
fn cast(
    game: &mut GameState,
    spell: ObjectId,
    dm: &mut Choices,
    stop_at_modes: bool,
) -> Vec<Prompt> {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        dm,
    )
    .unwrap();
    let mut prompts = Vec::new();
    for _ in 0..30 {
        if state.pending_cast.is_none() {
            return prompts;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        let ctx = match ctx {
            DecisionContext::Number(number) if number.is_x_value => {
                prompts.push(Prompt::X);
                DecisionContext::Number(number)
            }
            DecisionContext::Modes(modes) => {
                prompts.push(Prompt::Modes(
                    modes.spec.modes.iter().map(|mode| mode.legal).collect(),
                ));
                if stop_at_modes {
                    return prompts;
                }
                DecisionContext::Modes(modes)
            }
            other => other,
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast did not complete");
}

/// The announced X of the spell on the stack (it is a new object there).
fn stack_x(game: &GameState) -> Option<u32> {
    game.stack
        .iter()
        .rev()
        .find(|entry| !entry.is_ability)
        .and_then(|entry| game.object(entry.object_id))
        .and_then(|spell| spell.x_value)
}

impl Choices {
    fn new(x: u32, modes: Vec<usize>, bear: Option<ObjectId>) -> Self {
        Self { x, modes, bear }
    }
}

#[test]
fn kozileks_command_announces_x_before_modes_and_exiles_within_it() {
    let mut game = game();
    let bear = game.create_object_from_definition(&bear(), B, Zone::Battlefield);
    let spell = game.create_object_from_definition(&kozileks_command(), A, Zone::Hand);
    let mut dm = Choices::new(2, vec![1, 2], Some(bear));

    let prompts = cast(&mut game, spell, &mut dm, false);
    assert_eq!(
        prompts,
        vec![Prompt::X, Prompt::Modes(vec![true, true, true, true])],
        "X is announced first, and a mana-value-2 creature is exilable with X = 2"
    );
    assert_eq!(stack_x(&game), Some(2));

    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert!(!game.battlefield.contains(&bear), "the targeted bear is exiled");
}

#[test]
fn kozileks_command_exile_mode_reads_the_announced_x() {
    let mut game = game();
    let bear = game.create_object_from_definition(&bear(), B, Zone::Battlefield);
    let spell = game.create_object_from_definition(&kozileks_command(), A, Zone::Hand);
    let mut dm = Choices::new(1, vec![0, 1], Some(bear));

    let prompts = cast(&mut game, spell, &mut dm, true);
    assert_eq!(
        prompts,
        vec![Prompt::X, Prompt::Modes(vec![true, true, false, true])],
        "with X = 1 the mana-value-2 bear is out of reach"
    );
}

#[test]
fn exact_x_mana_value_mode_is_legal_only_at_that_x() {
    let probe = definition(
        "Exact X Probe",
        "Mana cost: {X}{B}\n\
         Type: Sorcery\n\
         Choose one —\n\
         • Destroy target creature with mana value X.\n\
         • You gain 1 life.",
    );
    for (x, legal) in [(1, false), (2, true), (3, false)] {
        let mut game = game();
        let bear = game.create_object_from_definition(&bear(), B, Zone::Battlefield);
        let spell = game.create_object_from_definition(&probe, A, Zone::Hand);
        let mut dm = Choices::new(x, vec![0], Some(bear));
        assert_eq!(
            cast(&mut game, spell, &mut dm, true),
            vec![Prompt::X, Prompt::Modes(vec![legal, true])],
            "X = {x}"
        );
    }
}

#[test]
fn choose_x_spells_still_derive_x_from_the_modes_chosen() {
    let confluence = definition(
        "Doomsday Confluence",
        "Mana cost: {X}{X}{B}\n\
         Type: Sorcery\n\
         Choose X. You may choose the same mode more than once.\n\
         • Each player sacrifices a nonartifact creature of their choice.\n\
         • Create a 3/3 black Dalek artifact creature token with menace.\n\
         • Each opponent discards a card.",
    );
    let mut game = game();
    let spell = game.create_object_from_definition(&confluence, A, Zone::Hand);
    let mut dm = Choices::new(0, vec![1, 1], None);

    let prompts = cast(&mut game, spell, &mut dm, false);
    assert_eq!(prompts.first(), Some(&Prompt::Modes(vec![true, true, true])));
    assert!(!prompts.contains(&Prompt::X), "the modes chosen announce X");
    assert_eq!(stack_x(&game), Some(2));
}
