//! Shared gameplay harness for the cf8 p08 round-4 scenarios. Source-authored,
//! UNRUN.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

pub const A: PlayerId = PlayerId::from_index(0);
pub const B: PlayerId = PlayerId::from_index(1);
pub const C: PlayerId = PlayerId::from_index(2);

/// A three-player game in A's first main phase. Only A has mana.
pub fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}

pub fn give_mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}

pub fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}

/// A scripted decision maker that records which player each prompt was
/// addressed to.
#[derive(Default)]
pub struct Script {
    pub targets: Vec<Target>,
    pub modes: Vec<usize>,
    pub numbers: Vec<u32>,
    pub decline: bool,
    /// (player the prompt was addressed to, prompt description) for every
    /// option prompt.
    pub option_prompts: Vec<(PlayerId, String)>,
    pub number_prompts: Vec<(PlayerId, String)>,
    /// Legal targets offered for each target requirement, in prompt order.
    pub offered_targets: Vec<Vec<Target>>,
}

impl DecisionMaker for Script {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.option_prompts
            .push((ctx.player, ctx.description.clone()));
        if ctx.description.starts_with("Choose mode") && !self.modes.is_empty() {
            return self.modes.clone();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        self.number_prompts
            .push((ctx.player, ctx.description.clone()));
        if !self.numbers.is_empty() {
            let value = self.numbers.remove(0);
            assert!(value >= ctx.min && value <= ctx.max, "{value} outside {}..={}", ctx.min, ctx.max);
            return value;
        }
        SelectFirstDecisionMaker.decide_number(game, ctx)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        for requirement in &context.requirements {
            self.offered_targets.push(requirement.legal_targets.clone());
        }
        if self.targets.is_empty() {
            return SelectFirstDecisionMaker.decide_targets(game, context);
        }
        self.targets.clone()
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
}

pub fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}

pub fn damage(game: &mut GameState, source: ObjectId, target: Target, amount: i32) {
    let choose = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(id) => ChooseSpec::SpecificPlayer(id),
    };
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::DealDamageEffect::new(amount, choose)),
    );
}

pub fn resolve_all(game: &mut GameState, dm: &mut Script) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("unexpected continuing trigger chain");
}

/// Cast `definition` from `player`'s hand, driving every casting decision
/// through `dm`. Returns the spell's stack object.
pub fn cast(
    game: &mut GameState,
    player: PlayerId,
    definition: &CardDefinition,
    dm: &mut Script,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, player, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    assert!(
        compute_legal_actions(game, player)
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

pub fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Script) {
    game.turn.priority_player = Some(A);
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}

/// Index of the `position`-th activated ability.
pub fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}

pub fn life(game: &GameState, player: PlayerId) -> i32 {
    game.player(player).unwrap().life
}

/// Execute one effect directly with a scripted decision maker.
pub fn apply_with(
    game: &mut GameState,
    source: ObjectId,
    effect: Effect,
    dm: &mut Script,
) -> EffectOutcome {
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(game, &effect, &mut EffectContext::new(source, controller, dm)).unwrap()
}
