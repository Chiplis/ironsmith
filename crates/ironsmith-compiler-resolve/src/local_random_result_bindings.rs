//! Exact local random-result identity shared by dice and coin instructions.
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Family { Die, Coin }

impl Family {
    pub(super) fn query(self, query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
        let (action, metric) = match self {
            Self::Die => (PriorEffectAction::Rolled, query.metric == EffectMetric::Count),
            Self::Coin => (PriorEffectAction::Flipped, matches!(query.metric,
                EffectMetric::CoinFlipsWon | EffectMetric::CoinFlipsLost | EffectMetric::CoinHeads | EffectMetric::CoinTails)),
        };
        query.action == Some(action) && metric && query.source == EffectMetricSource::Outcome
            && query.filter.is_none() && query.player.is_none() && query.counter_type.is_none()
    }
    fn direct(self, effect: &EffectAst) -> bool {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Random(action), .. }) = effect else { return false };
        match self {
            Self::Die => matches!(action, RandomActionAst::RollDie { .. } | RandomActionAst::RollDiceChooseResult { .. }),
            Self::Coin => matches!(action, RandomActionAst::FlipCoin | RandomActionAst::FlipCoinFaceOnly | RandomActionAst::FlipCoins { .. }),
        }
    }
    fn compatible(self, effect: &EffectAst) -> bool {
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
        let producers = match self { Self::Die => state.die_result_producers, Self::Coin => state.coin_result_producers };
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
        }.into()))
    }
    pub(super) fn rebound(self, producer: &EffectAst, remaining: &[EffectAst]) -> Option<EffectId> {
        if !self.compatible(producer) { return None; }
        fn collect(family: Family, value: &Value, ids: &mut Vec<EffectId>) {
            match value {
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
