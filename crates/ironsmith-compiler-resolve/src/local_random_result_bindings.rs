//! Exact local random-result identity shared by dice and coin instructions.
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Family { Die, Coin, Number, Color, Reveal }

/// "for each card of the chosen type / with the chosen name revealed this way"
/// (Blood Oath, Thought Hemorrhage): a filtered count of the cards an exact
/// local hand reveal showed. Bound only when such a reveal precedes it; other
/// reveal shapes keep the generic prior-result binding.
pub(super) fn filtered_hand_reveal_query(query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
    query.action == Some(PriorEffectAction::Revealed)
        && query.source == EffectMetricSource::AffectedObjects
        && query.metric == EffectMetric::Count
        && query.color_choice.is_none()
        && query.filter.is_some()
        && query.player.is_none()
        && query.counter_type.is_none()
}

impl Family {
    pub(super) fn query(self, query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
        if self == Self::Color { return false; }
        if self == Self::Reveal {
            return query.action == Some(PriorEffectAction::Revealed)
                && query.source == EffectMetricSource::AffectedObjects
                && query.metric == EffectMetric::Count && query.color_choice.is_some();
        }
        let (action, metric) = match self {
            Self::Die => (PriorEffectAction::Rolled, query.metric == EffectMetric::Count),
            Self::Number => (PriorEffectAction::ChosenNumber, query.metric == EffectMetric::Count),
            Self::Color | Self::Reveal => unreachable!(),
            Self::Coin => (PriorEffectAction::Flipped, matches!(query.metric,
                EffectMetric::CoinFlipsTotal | EffectMetric::CoinFlipsWon | EffectMetric::CoinFlipsLost | EffectMetric::CoinHeads | EffectMetric::CoinTails)),
        };
        query.action == Some(action) && metric && query.source == EffectMetricSource::Outcome
            && query.filter.is_none() && query.player.is_none() && query.counter_type.is_none()
    }
    fn direct(self, effect: &EffectAst) -> bool {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else { return false; };
        match self {
            Self::Die => matches!(action, SubjectVerbActionAst::Random(RandomActionAst::RollDie { .. } | RandomActionAst::RollDiceChooseResult { .. })),
            Self::Coin => matches!(action, SubjectVerbActionAst::Random(RandomActionAst::FlipCoin | RandomActionAst::FlipCoinFaceOnly | RandomActionAst::FlipCoins { .. })),
            Self::Number => matches!(action, SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber { .. })),
            Self::Color => matches!(action, SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor)),
            Self::Reveal => matches!(action, SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealHand)),
        }
    }
    pub(super) fn compatible(self, effect: &EffectAst) -> bool {
        if self.direct(effect) { return true; }
        match effect {
            EffectAst::Sequence { effects } | EffectAst::CommaThen { effects }
            | EffectAst::SourceSentence { effects, .. } | EffectAst::Coordinated { effects, .. } => {
                matches!(effects.as_slice(), [only] if self.compatible(only))
            }
            EffectAst::Permissions(PermissionEffectAst::May { effects } | PermissionEffectAst::MayByPlayer { effects, .. })
                if self == Self::Coin => matches!(effects.as_slice(), [only] if self.compatible(only)),
            EffectAst::ControlFlow(control) if self == Self::Coin => {
                if let crate::model::ControlFlowNodeAst::Permission(permission) = &control.node {
                    control.programs.get(permission.program).is_some_and(|program| {
                        matches!(program.effects.as_slice(), [only] if self.compatible(only))
                    })
                } else { false }
            }
            _ => false,
        }
    }
    fn contains(self, effect: &EffectAst) -> bool {
        if self.direct(effect) { return true; }
        let mut found = false;
        for_each_nested_effects(effect, true, |effects| {
            found |= effects.iter().any(|effect| self.contains(effect));
        });
        found
    }
    pub(super) fn remember(self, producers: &mut Vec<Option<EffectId>>, id: Option<EffectId>, effect: &EffectAst) {
        if self.compatible(effect) { producers.push(id); }
        else if self.contains(effect) { producers.push(None); }
    }
    pub(super) fn bind(self, query: &ironsmith_core::PriorEffectMetricQuery, state: EffectReferenceResolutionState<'_>) -> Result<Value, CardTextError> {
        let producers = match self {
            Self::Die => state.die_result_producers, Self::Coin => state.coin_result_producers,
            Self::Number => state.number_result_producers, Self::Color => state.color_result_producers,
            Self::Reveal => state.reveal_result_producers,
        };
        let mut query = query.clone();
        if query.color_choice == Some(ironsmith_core::ColorChoiceReference::Pending) {
            query.color_choice = Some(ironsmith_core::ColorChoiceReference::Effect(
                state.color_result_producers.last().copied().flatten().ok_or_else(||
                    CardTextError::ParseError("revealed color count requires its exact local color choice".into()))?,
            ));
        }
        if let Some(id) = producers.last() {
            return id.map(|effect_id| Value::PriorEffectMetric { effect_id, query: query.clone() })
                .ok_or_else(|| CardTextError::ParseError("the local random result is not exported by its enclosing instruction".into()));
        }
        if self == Self::Die && state.dice_event_grouped == Some(false) {
            return Ok(Value::EventValue(EventValueSpec::DieResult));
        }
        Err(CardTextError::ParseError(match self {
            Self::Die => "die-result predicate requires a compatible local roll or singular numeric roll trigger",
            Self::Coin => "coin-result predicate requires a compatible local flip instruction",
            Self::Number => "chosen-number quantity requires its exact local numeric producer",
            Self::Color => "chosen-color quantity requires its exact local color producer",
            Self::Reveal => "revealed-card count requires its exact local reveal producer",
        }.into()))
    }
    /// Whether an exact-choice producer must export its result ID. A revealed
    /// hand is only a result producer when a later instruction counts the
    /// cards revealed this way; an unconsumed reveal keeps its plain shape.
    pub(super) fn exported_for(self, remaining: &[EffectAst]) -> bool {
        if self != Self::Reveal { return true; }
        fn consumes(family: Family, value: &Value) -> bool {
            match value {
                Value::PendingPriorEffectMetric(query) | Value::PriorEffectMetric { query, .. } => {
                    family.query(query)
                        || (family == Family::Reveal && filtered_hand_reveal_query(query))
                }
                Value::SurfaceHinted { value, .. } | Value::Scaled(value, _)
                | Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => consumes(family, value),
                Value::Add(a, b) | Value::Min(a, b) => consumes(family, a) || consumes(family, b),
                _ => false,
            }
        }
        remaining.iter().any(|later| {
            let mut found = false;
            visit_effect_values(later, &mut |value| found |= consumes(self, value));
            found
        })
    }
    pub(super) fn rebound(self, producer: &EffectAst, remaining: &[EffectAst]) -> Option<EffectId> {
        if !self.compatible(producer) { return None; }
        fn collect(family: Family, value: &Value, ids: &mut Vec<EffectId>) {
            match value {
                Value::PriorEffectMetric { query, .. } if family == Family::Color => {
                    if let Some(ironsmith_core::ColorChoiceReference::Effect(id)) = query.color_choice {
                        if !ids.contains(&id) { ids.push(id); }
                    }
                }
                Value::PriorEffectMetric { effect_id, query } if family.query(query) => {
                    if !ids.contains(effect_id) { ids.push(*effect_id); }
                }
                Value::SurfaceHinted { value, .. } | Value::Scaled(value, _)
                | Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => collect(family, value, ids),
                Value::Add(a, b) | Value::Min(a, b) => { collect(family, a, ids); collect(family, b, ids); }
                _ => {}
            }
        }
        for consumer in remaining {
            if self.contains(consumer) { return None; }
            let mut ids = Vec::new();
            visit_effect_values(consumer, &mut |value| collect(self, value, &mut ids));
            if ids.len() == 1 { return Some(ids[0]); }
        }
        None
    }
}

#[cfg(test)]
mod choice_tests {
    use super::*;
    #[test]
    fn three_exact_choice_bindings_survive_frames_without_using_the_latest_generic_result() {
        let env=ReferenceEnv{number_result_producers:std::sync::Arc::new(vec![Some(EffectId(2))]),
            color_result_producers:std::sync::Arc::new(vec![Some(EffectId(4))]),
            reveal_result_producers:std::sync::Arc::new(vec![Some(EffectId(7))]),
            last_effect_id:RefState::Known(EffectId(99)),..Default::default()};
        let restored=ReferenceEnv::from_frame(&ReferenceFrame::from_lowering_frame(&env.to_lowering_frame(false,false)));
        let number=ironsmith_core::PriorEffectMetricQuery::new(EffectMetricSource::Outcome,EffectMetric::Count).with_action(PriorEffectAction::ChosenNumber);
        assert_eq!(Family::Number.bind(&number,effect_reference_resolution_state(&restored)).unwrap(),Value::PriorEffectMetric{effect_id:EffectId(2),query:number});
        let mut revealed=ironsmith_core::PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects,EffectMetric::Count)
            .with_action(PriorEffectAction::Revealed).with_filter(ObjectFilter::default().of_chosen_color());
        revealed.color_choice=Some(ironsmith_core::ColorChoiceReference::Pending);
        let Value::PriorEffectMetric{effect_id,query}=Family::Reveal.bind(&revealed,effect_reference_resolution_state(&restored)).unwrap() else{panic!("exact reveal required")};
        assert_eq!(effect_id,EffectId(7));assert_eq!(query.color_choice,Some(ironsmith_core::ColorChoiceReference::Effect(EffectId(4))));
        let mut missing=restored;missing.color_result_producers=std::sync::Arc::new(vec![None]);
        assert!(Family::Reveal.bind(&revealed,effect_reference_resolution_state(&missing)).is_err());
    }
}
