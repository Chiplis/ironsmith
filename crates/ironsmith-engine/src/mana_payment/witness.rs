//! Server-selected records for replaying compact mana decisions.
use super::ManaReplacementDecision;
use crate::events::ManaAddedEvent;
use crate::game_state::Target;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;

/// The actual choice offered by an executing mana instruction.
#[derive(Debug, Clone)]
pub struct ManaProductionChoice {
    pub source: ObjectId,
    pub player: PlayerId,
    pub available: Vec<ManaSymbol>,
    pub count: u32,
    pub same_type: bool,
    pub distinct: bool,
}
impl ManaProductionChoice {
    pub(crate) fn accepts(&self, selected: &[ManaSymbol]) -> bool {
        crate::effects::mana::production_resolution::ResolvedManaOutput::Choice {
            available: self.available.clone(), count: self.count,
            same_type: self.same_type, distinct: self.distinct,
        }.accepts(selected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManaReplacementWitness {
    pub source: ObjectId,
    pub controller: PlayerId,
    pub player: PlayerId,
    pub input: Vec<ManaSymbol>,
    pub provenance: crate::events::mana::ManaProductionProvenance,
    pub decisions: Vec<ManaReplacementDecision>,
}

impl ManaReplacementWitness {
    pub(crate) fn from_event(
        event: &ManaAddedEvent,
        decisions: Vec<ManaReplacementDecision>,
    ) -> Self {
        Self {
            source: event.source,
            controller: event.controller,
            player: event.player,
            input: event.mana.clone(),
            provenance: event.provenance,
            decisions,
        }
    }

    pub(crate) fn matches(&self, event: &ManaAddedEvent) -> bool {
        self.source == event.source
            && self.controller == event.controller
            && self.player == event.player
            && self.input == event.mana
            && self.provenance == event.provenance
    }
}

/// Validate the complete record first, then consume only one-shot resources
/// actually applied by that record. The enclosing event owner is transactional.
pub(crate) fn replay_replacements(
    game: &mut crate::game_state::GameState,
    event: &ManaAddedEvent,
    witness: &ManaReplacementWitness,
) -> Result<ManaAddedEvent, crate::effects::ExecutionError> {
    let invalid = || {
        crate::effects::ExecutionError::InternalError(
            "stale or incomplete mana replacement witness".into(),
        )
    };
    if !witness.matches(event) {
        return Err(invalid());
    }
    let program =
        super::replacement_program::CompiledManaReplacements::compile(game).ok_or_else(invalid)?;
    let branch = program
        .replay(
            game,
            event.clone(),
            &super::replacement_program::ReplacementResources::default(),
            &witness.decisions,
        )
        .ok_or_else(invalid)?;
    let ids = branch
        .resources
        .consumed
        .iter()
        .map(|key| {
            game.effect_store
                .replacement_effects
                .effects()
                .iter()
                .find(|effect| effect.application_key() == *key)
                .map(|effect| effect.id)
                .ok_or_else(invalid)
        })
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids {
        game.effect_store.replacement_effects.mark_effect_used(id);
    }
    Ok(branch.event)
}

/// A plan owns only the recorded mana decisions. All other decisions retain
/// the caller's existing channel, including pending interactive responses.
pub(super) struct WitnessDecisionMaker<'a> {
    records: &'a [ManaReplacementWitness],
    cursor: usize,
    fallback: &'a mut dyn crate::decision::DecisionMaker,
}
impl<'a> WitnessDecisionMaker<'a> {
    pub fn new(
        records: &'a [ManaReplacementWitness],
        fallback: &'a mut dyn crate::decision::DecisionMaker,
    ) -> Self {
        Self {
            records,
            cursor: 0,
            fallback,
        }
    }
    pub fn complete(&self) -> bool {
        self.cursor == self.records.len()
    }
}
impl crate::decision::DecisionMaker for WitnessDecisionMaker<'_> {
    fn planned_mana_output(&mut self, _game: &crate::game_state::GameState, choice: &ManaProductionChoice)
        -> Result<Option<Vec<ManaSymbol>>, String> {
        let record = self.records.get(self.cursor).ok_or_else(|| "unrecorded mana production choice".to_string())?;
        if record.source != choice.source || record.player != choice.player {
            return Err("mana production choice differs from selected plan".into());
        }
        // Some native same-color effects ask for one color, then repeat it for
        // the full amount. The event record still contains the complete output.
        let output = if choice.same_type && choice.count == 1
            && record.input.windows(2).all(|pair| pair[0] == pair[1]) {
            record.input.first().copied().into_iter().collect()
        } else { record.input.clone() };
        if !choice.accepts(&output) { return Err("planned mana output is no longer a legal choice".into()); }
        Ok(Some(output))
    }

    fn take_mana_replacement_witness(
        &mut self,
        _game: &crate::game_state::GameState,
        event: &ManaAddedEvent,
    ) -> Result<Option<ManaReplacementWitness>, String> {
        let witness = self
            .records
            .get(self.cursor)
            .ok_or_else(|| "unrecorded mana production".to_string())?;
        if !witness.matches(event) {
            return Err("mana production differs from selected plan".into());
        }
        self.cursor += 1;
        Ok(Some(witness.clone()))
    }
    fn on_auto_pass(&mut self, _game: &crate::game_state::GameState, _player: PlayerId) {
        self.fallback.on_auto_pass(_game, _player)
    }
    fn on_action_cancelled(&mut self, _game: &crate::game_state::GameState, _reason: &str) {
        self.fallback.on_action_cancelled(_game, _reason)
    }
    fn awaiting_choice(&self) -> bool {
        self.fallback.awaiting_choice()
    }
    fn answers_player_choices(&self) -> bool {
        self.fallback.answers_player_choices()
    }
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.fallback.decide_boolean(_game, _ctx)
    }
    fn decide_number(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.fallback.decide_number(_game, ctx)
    }
    fn decide_text(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        self.fallback.decide_text(_game, ctx)
    }
    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.fallback.decide_objects(_game, ctx)
    }
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.fallback.decide_options(_game, ctx)
    }
    fn decide_mana_payment(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::ManaPaymentContext,
    ) -> crate::mana_payment::ManaPaymentResponse {
        self.fallback.decide_mana_payment(game, ctx)
    }
    fn decide_order(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        self.fallback.decide_order(_game, ctx)
    }
    fn view_cards(
        &mut self,
        _game: &crate::game_state::GameState,
        _viewer: PlayerId,
        _cards: &[ObjectId],
        _ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.fallback.view_cards(_game, _viewer, _cards, _ctx)
    }
    fn decide_attackers(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        self.fallback.decide_attackers(_game, _ctx)
    }
    fn decide_blockers(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        self.fallback.decide_blockers(_game, _ctx)
    }
    fn decide_distribute(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.fallback.decide_distribute(_game, _ctx)
    }
    fn decide_colors(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        self.fallback.decide_colors(_game, ctx)
    }
    fn decide_counters(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        self.fallback.decide_counters(_game, _ctx)
    }
    fn decide_partition(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        self.fallback.decide_partition(_game, _ctx)
    }
    fn decide_proliferate(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        self.fallback.decide_proliferate(_game, _ctx)
    }
    fn decide_priority(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> crate::decision::LegalAction {
        self.fallback.decide_priority(_game, ctx)
    }
    fn decide_targets(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.fallback.decide_targets(_game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::super::replacement_program::{CompiledManaReplacements, ReplacementResources};
    use super::*;
    use crate::effects::{AddManaEffect, EffectExecutor, ExecutionContext};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};

    struct Recorded {
        witness: Option<ManaReplacementWitness>,
    }
    impl crate::decision::DecisionMaker for Recorded {
        fn take_mana_replacement_witness(
            &mut self,
            _game: &crate::game_state::GameState,
            _event: &ManaAddedEvent,
        ) -> Result<Option<ManaReplacementWitness>, String> {
            self.witness
                .take()
                .map(Some)
                .ok_or_else(|| "unexpected production event".into())
        }
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            panic!("a complete replacement witness must not ask an arbitrary chooser")
        }
    }

    fn fixture() -> (crate::game_state::GameState, ManaAddedEvent) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = crate::CardBuilder::new(crate::ids::CardId::new(), "Witness source")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, alice, crate::Zone::Battlefield);
        for action in [
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
            ReplacementAction::Modify(EventModification::Multiply(3)),
        ] {
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                        crate::target::ObjectFilter::default(),
                    ),
                    action,
                ));
        }
        game.refresh_continuous_state().unwrap();
        (
            game,
            ManaAddedEvent::new(source, alice, alice, vec![ManaSymbol::Green; 2]),
        )
    }

    #[test]
    fn native_execution_replays_each_noncommuting_replacement_order() {
        for amount in [1, 3] {
            let (mut game, event) = fixture();
            let program = CompiledManaReplacements::compile(&game).unwrap();
            let branch = program
                .branches(&game, event.clone(), &ReplacementResources::default(), 16)
                .unwrap()
                .into_iter()
                .find(|branch| branch.event.mana.len() == amount)
                .unwrap();
            let mut dm = Recorded {
                witness: Some(ManaReplacementWitness::from_event(&event, branch.decisions)),
            };
            let mut ctx = ExecutionContext::new_default(event.source, event.controller)
                .with_decision_maker(&mut dm);
            let result = AddManaEffect::you(event.mana.clone())
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(
                game.player(event.player).unwrap().mana_pool.blue,
                amount as u32
            );
            assert_eq!(game.player(event.player).unwrap().mana_pool.green, 0);
            assert_eq!(result.events.len(), 1);
            assert_eq!(
                result.events[0].downcast::<ManaAddedEvent>().unwrap().mana,
                vec![ManaSymbol::Blue; amount]
            );
            drop(ctx);
            assert!(dm.witness.is_none());
        }
    }

    #[test]
    fn native_production_rejects_invalid_recorded_choices_before_credit() {
        use crate::effects::AddManaOfAnyColorEffect;
        for (effect, selected) in [
            (AddManaOfAnyColorEffect::you(2), vec![ManaSymbol::White]),
            (AddManaOfAnyColorEffect::you(2), vec![ManaSymbol::Colorless; 2]),
            (AddManaOfAnyColorEffect::you_distinct(2), vec![ManaSymbol::White; 2]),
        ] {
            let (mut game, mut event) = fixture();
            event.mana = selected;
            let records = vec![ManaReplacementWitness::from_event(&event, vec![])];
            let mut fallback = crate::decision::SelectFirstDecisionMaker;
            let mut dm = WitnessDecisionMaker::new(&records, &mut fallback);
            let mut ctx = ExecutionContext::new_default(event.source, event.controller)
                .with_decision_maker(&mut dm);
            assert!(effect.execute(&mut game, &mut ctx).is_err());
            assert_eq!(game.player(event.player).unwrap().mana_pool.total(), 0);
            drop(ctx);
            assert!(!dm.complete());
        }
    }

    #[test]
    fn native_execution_rejects_stale_witness_without_crediting_or_consuming() {
        let (mut game, event) = fixture();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let branch = program
            .branches(&game, event.clone(), &ReplacementResources::default(), 16)
            .unwrap()
            .remove(0);
        let mut witness = ManaReplacementWitness::from_event(&event, branch.decisions);
        witness.input.push(ManaSymbol::Red);
        let ids = game
            .effect_store
            .replacement_effects
            .effects()
            .iter()
            .map(|effect| effect.id)
            .collect::<Vec<_>>();
        let mut dm = Recorded {
            witness: Some(witness),
        };
        let mut ctx = ExecutionContext::new_default(event.source, event.controller)
            .with_decision_maker(&mut dm);
        assert!(
            AddManaEffect::you(event.mana.clone())
                .execute(&mut game, &mut ctx)
                .is_err()
        );
        assert_eq!(game.player(event.player).unwrap().mana_pool.total(), 0);
        assert_eq!(
            game.effect_store
                .replacement_effects
                .effects()
                .iter()
                .map(|effect| effect.id)
                .collect::<Vec<_>>(),
            ids
        );
    }
}
