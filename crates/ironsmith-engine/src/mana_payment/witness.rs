//! Server-selected records for replaying compact mana decisions.
use super::ManaReplacementDecision;
use crate::events::ManaAddedEvent;
use crate::game_state::Target;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManaChoicePurpose { Production, StoredColor, ReplacementColor }

/// The actual choice offered by an executing mana instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaProductionChoice {
    pub purpose: ManaChoicePurpose,
    pub source: ObjectId,
    pub player: PlayerId,
    pub available: Vec<ManaSymbol>,
    pub count: u32,
    pub same_type: bool,
    pub distinct: bool,
}
impl std::hash::Hash for ManaProductionChoice {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        // Preserve the previous production-choice digest and therefore existing
        // prepared payment IDs. Only the new decision domain adds a prefix.
        if self.purpose == ManaChoicePurpose::StoredColor { "stored-color-choice-v1".hash(state); }
        if self.purpose == ManaChoicePurpose::ReplacementColor { "replacement-color-choice-v1".hash(state); }
        self.source.hash(state); self.player.hash(state); self.available.hash(state);
        self.count.hash(state); self.same_type.hash(state); self.distinct.hash(state);
    }
}
impl ManaProductionChoice {
    pub(crate) fn accepts(&self, selected: &[ManaSymbol]) -> bool {
        crate::effects::mana::production_resolution::ResolvedManaOutput::Choice {
            available: self.available.clone(), count: self.count,
            same_type: self.same_type, distinct: self.distinct,
        }.accepts(selected)
    }
}

/// One exact production choice made while preparing an activation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManaProductionWitness {
    pub choice: ManaProductionChoice,
    pub chooser: PlayerId,
    pub output: Vec<ManaSymbol>,
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
pub(crate) struct WitnessDecisionMaker<'a> {
    records: Option<&'a [ManaReplacementWitness]>,
    cursor: usize,
    production_records: Option<&'a [ManaProductionWitness]>,
    production_cursor: usize,
    recording_production: bool,
    recording_payer: Option<PlayerId>,
    production_recording_supported: bool,
    recorded_production: Vec<ManaProductionWitness>,
    stored_colors: std::collections::VecDeque<crate::color::Color>,
    fallback: &'a mut dyn crate::decision::DecisionMaker,
}
impl<'a> WitnessDecisionMaker<'a> {
    pub fn new(
        records: &'a [ManaReplacementWitness],
        fallback: &'a mut dyn crate::decision::DecisionMaker,
    ) -> Self {
        Self {
            records: Some(records),
            cursor: 0,
            production_records: None,
            production_cursor: 0,
            recording_production: false,
            recording_payer: None,
            production_recording_supported: true,
            recorded_production: Vec::new(),
            stored_colors: Default::default(),
            fallback,
        }
    }
    pub fn for_activation(
        replacements: Option<&'a [ManaReplacementWitness]>,
        production: Option<&'a [ManaProductionWitness]>,
        fallback: &'a mut dyn crate::decision::DecisionMaker,
    ) -> Self {
        let mut replay = Self::new(&[], fallback);
        replay.records = replacements;
        // Projected plans encode production choices in their replacement-event
        // inputs and intentionally have no separate production records. An
        // empty auxiliary vector must not shadow those authoritative inputs.
        // Recorded native plans carry nonempty choice records (including stored
        // colors), which take precedence over event-level output reconstruction.
        replay.production_records = production.filter(|records|
            !records.is_empty() || replacements.is_none());
        replay
    }
    pub fn record_production(payer: PlayerId, fallback: &'a mut dyn crate::decision::DecisionMaker) -> Self {
        let mut recorder = Self::new(&[], fallback);
        recorder.records = None;
        recorder.recording_production = true;
        recorder.recording_payer = Some(payer);
        recorder
    }
    pub fn with_stored_colors(mut self, colors: Vec<crate::color::Color>) -> Self {
        self.stored_colors = colors.into();
        self
    }
    pub fn stored_colors_consumed(&self) -> bool { self.stored_colors.is_empty() }
    pub fn recorded_production(&self) -> Option<&[ManaProductionWitness]> {
        self.production_recording_supported.then_some(&self.recorded_production)
    }
    pub fn complete(&self) -> bool {
        self.records.is_none_or(|records| self.cursor == records.len())
            && self.production_records.is_none_or(|records| self.production_cursor == records.len())
    }
}
impl crate::decision::DecisionMaker for WitnessDecisionMaker<'_> {
    fn planned_mana_output(&mut self, _game: &crate::game_state::GameState, choice: &ManaProductionChoice)
        -> Result<Option<Vec<ManaSymbol>>, String> {
        if let Some(records) = self.production_records {
            let record = records.get(self.production_cursor)
                .ok_or_else(|| "unrecorded mana production choice".to_string())?;
            if record.choice != *choice || record.chooser != _game.controlling_player_for(choice.player)
                || !choice.accepts(&record.output) {
                return Err("mana production choice differs from selected plan".into());
            }
            self.production_cursor += 1;
            return Ok(Some(record.output.clone()));
        }
        let Some(records) = self.records else {
            if self.recording_production {
                let chooser = _game.controlling_player_for(choice.player);
                if self.recording_payer.is_none_or(|payer| _game.controlling_player_for(payer) != chooser) {
                    // Confirming one's payment cannot preselect another
                    // player's independent production decision.
                    self.production_recording_supported = false;
                    if choice.purpose == ManaChoicePurpose::ReplacementColor {
                        let error = crate::effects::ExecutionError::UnresolvedPlayerDecision {
                            player: choice.player, decision: "mana replacement color",
                        };
                        // The query meter carries typed incompleteness across
                        // legacy Option/boolean planner adapters. Do not run
                        // a default chooser and manufacture a complete plan.
                        _game.record_token_resource_failure(&error);
                        return Err(error.to_string());
                    }
                    return Ok(None);
                }
                let output = if choice.purpose == ManaChoicePurpose::StoredColor {
                    self.stored_colors.pop_front().map(ManaSymbol::from_color)
                        .or_else(|| choice.available.first().copied()).into_iter().collect()
                } else if choice.distinct && !choice.same_type {
                    choice.available.iter().copied().take(choice.count as usize).collect()
                } else {
                    choice.available.first().copied().map(|symbol| vec![symbol; choice.count as usize])
                        .unwrap_or_default()
                };
                if !choice.accepts(&output) {
                    // Keep the legacy effect/chooser path for an unsupported
                    // domain; an incomplete recording must not prove illegality.
                    self.production_recording_supported = false;
                    return Ok(None);
                }
                self.recorded_production.push(ManaProductionWitness { choice: choice.clone(), chooser, output: output.clone() });
                return Ok(Some(output));
            }
            return self.fallback.planned_mana_output(_game, choice);
        };
        if choice.purpose != ManaChoicePurpose::Production {
            return self.fallback.planned_mana_output(_game, choice);
        }
        let record = records.get(self.cursor).ok_or_else(|| "unrecorded mana production choice".to_string())?;
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
        let Some(records) = self.records else {
            return self.fallback.take_mana_replacement_witness(_game, event);
        };
        let witness = records
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

#[cfg(test)]
mod production_choice_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::effects::{AddManaOfAnyColorEffect, EffectExecutor, ExecutionContext};
    struct WhiteChooser { players: Vec<PlayerId> }
    impl DecisionMaker for WhiteChooser {
        fn decide_colors(&mut self, game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::ColorsContext) -> Vec<crate::color::Color> {
            self.players.push(game.controlling_player_for(ctx.player));
            vec![crate::color::Color::White; ctx.count as usize]
        }
    }

    #[test]
    fn stored_color_witness_preserves_manual_and_independent_chooser_decisions() {
        use crate::{color::Color, effects::ChooseColorEffect, target::PlayerFilter};
        struct BlueChooser(usize);
        impl DecisionMaker for BlueChooser {
            fn decide_options(&mut self, _: &crate::GameState,
                _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                self.0 += 1; vec![1]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(99);
        let mut dm = BlueChooser(0);
        let effect = ChooseColorEffect::new(PlayerFilter::You);
        effect.execute(&mut game, &mut ExecutionContext::new(source, alice, &mut dm)).unwrap();
        assert_eq!(dm.0, 1); assert_eq!(game.chosen_color(source), Some(Color::Blue));
        let records = {
            let mut recorder = WitnessDecisionMaker::record_production(alice, &mut dm)
                .with_stored_colors(vec![Color::Green]);
            effect.execute(&mut game, &mut ExecutionContext::new(source, alice, &mut recorder)).unwrap();
            assert!(recorder.stored_colors_consumed());
            recorder.recorded_production().unwrap().to_vec()
        };
        assert_eq!(dm.0, 1); assert_eq!(game.chosen_color(source), Some(Color::Green));
        {
            let mut replay = WitnessDecisionMaker::for_activation(Some(&[]), Some(&records), &mut dm);
            effect.execute(&mut game, &mut ExecutionContext::new(source, alice, &mut replay)).unwrap();
            assert!(replay.complete());
        }
        assert_eq!(dm.0, 1);
        let opponent_choice = ChooseColorEffect::new(PlayerFilter::Specific(bob));
        {
            let mut recorder = WitnessDecisionMaker::record_production(alice, &mut dm);
            opponent_choice.execute(&mut game, &mut ExecutionContext::new(source, alice, &mut recorder)).unwrap();
            assert!(recorder.recorded_production().is_none());
        }
        assert_eq!(dm.0, 2); assert_eq!(game.chosen_color(source), Some(Color::Blue));
        game.add_scoped_player_control(bob, alice, None);
        let mut replay = WitnessDecisionMaker::for_activation(None, Some(&records), &mut dm);
        assert!(effect.execute(&mut game, &mut ExecutionContext::new(source, alice, &mut replay)).is_err());
        assert_eq!(game.chosen_color(source), Some(Color::Blue));
    }

    #[test]
    fn production_witness_hash_preserves_legacy_choice_identity() {
        use std::hash::{Hash, Hasher};
        let mut choice = ManaProductionChoice { purpose: ManaChoicePurpose::Production,
            source: ObjectId::from_raw(99), player: PlayerId::from_index(0),
            available: vec![ManaSymbol::White, ManaSymbol::Blue], count: 1, same_type: true, distinct: false };
        let mut old = std::collections::hash_map::DefaultHasher::new();
        choice.source.hash(&mut old); choice.player.hash(&mut old); choice.available.hash(&mut old);
        choice.count.hash(&mut old); choice.same_type.hash(&mut old); choice.distinct.hash(&mut old);
        let mut current = std::collections::hash_map::DefaultHasher::new(); choice.hash(&mut current);
        assert_eq!(current.finish(), old.finish());
        choice.purpose = ManaChoicePurpose::StoredColor;
        let mut stored = std::collections::hash_map::DefaultHasher::new(); choice.hash(&mut stored);
        assert_ne!(stored.finish(), old.finish());
    }

    #[test]
    fn independently_controlled_production_is_not_committed_by_payers_plan() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut dm = WhiteChooser { players: Vec::new() };
        let mut recorder = WitnessDecisionMaker::record_production(alice, &mut dm);
        let effect = AddManaOfAnyColorEffect::restricted(1, crate::target::PlayerFilter::Specific(bob),
            vec![crate::color::Color::Green, crate::color::Color::White]);
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99), alice, &mut recorder);
            effect.execute(&mut game, &mut ctx).unwrap();
        }
        assert!(recorder.recorded_production().is_none());
        drop(recorder);
        assert_eq!(dm.players, vec![bob]);
        assert_eq!(game.player(bob).unwrap().mana_pool.white, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn production_choice_rejects_changed_decision_controller_before_credit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut dm = WhiteChooser { players: Vec::new() };
        let effect = AddManaOfAnyColorEffect::restricted(1, crate::target::PlayerFilter::Specific(alice),
            vec![crate::color::Color::Green, crate::color::Color::White]);
        let records = {
            let mut recorder = WitnessDecisionMaker::record_production(alice, &mut dm);
            {
                let mut ctx = ExecutionContext::new(ObjectId::from_raw(99), alice, &mut recorder);
                effect.execute(&mut game, &mut ctx).unwrap();
            }
            recorder.recorded_production().unwrap().to_vec()
        };
        assert!(dm.players.is_empty());
        let before = game.player(alice).unwrap().mana_pool.clone();
        game.add_scoped_player_control(bob, alice, None);
        let mut replay = WitnessDecisionMaker::for_activation(None, Some(&records), &mut dm);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(99), alice, &mut replay);
        assert!(effect.execute(&mut game, &mut ctx).is_err());
        assert_eq!(game.player(alice).unwrap().mana_pool, before);
    }
}
