//! The counter actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum CounterActionAst {
    /// "The next time target creature adapts this turn, it adapts as though
    /// it had no +1/+1 counters on it." (Biomancer's Familiar)
    NextAdaptIgnoresCounters {
        target: TargetAst,
    },
    PutCounters {
        maximum_total: Option<u32>,
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    },
    PutCounterChoice {
        counter_types: Vec<CounterType>,
        count: Value,
        mode_texts: Vec<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutOrRemoveCounters {
        put_counter_type: CounterType,
        put_count: Value,
        remove_counter_type: CounterType,
        remove_count: Value,
        put_mode_text: String,
        remove_mode_text: String,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutCountersAll {
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
        distributed_across_all: bool,
        all_of_them: bool,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
        remove_from_source: bool,
    },
    MoveOneCounter {
        from: TargetAst,
        to: TargetAst,
    },
    /// "Move X +1/+1 counters from <from> onto <to>" (Blaster, Morale
    /// Booster): a counted move of one counter kind (CR 122.5).
    MoveCounters {
        counter_type: CounterType,
        count: ironsmith_core::effect::CounterMoveAmount,
        from: TargetAst,
        to: TargetAst,
        /// The complete donor set, rather than one resolution-time selection.
        from_all: bool,
    },
    ForEachCounterKindPutOrRemove {
        target: TargetAst,
        counter_source: Option<TargetAst>,
        all_kinds: bool,
        fixed_counter_type: Option<CounterType>,
        optional_action: bool,
        put_only: bool,
        choose_target_per_kind: bool,
    },
    PutCounterOfChosenKind {
        target: TargetAst,
    },
    /// "Choose a counter on <kind_source>. Put a counter of that kind on
    /// <recipients> [if it doesn't have a counter of that kind on it]."
    /// Exactly one of `target` / `each` names the recipients.
    PutCounterOfKindChosenFrom {
        kind_source: ObjectFilter,
        target: Option<TargetAst>,
        each: Option<ObjectFilter>,
        exclude_kind_object: bool,
        only_if_absent: bool,
    },
    DoubleCountersOnEach {
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    },
    DoubleCountersOnTarget {
        counter_type: Option<CounterType>,
        target: TargetAst,
    },
    RemoveCountersAll {
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    },
    PoisonCounters {
        count: Value,
    },
    EnergyCounters {
        count: Value,
    },
    ExperienceCounters {
        count: Value,
    },
    /// "gets N rad counters" (CR 122.1i, 727).
    RadCounters {
        count: Value,
    },
    TicketCounters {
        count: Value,
    },
}
