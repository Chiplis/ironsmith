//! The counter actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CounterActionAst {
    PutCounters {
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
    },
    MoveOneCounter {
        from: TargetAst,
        to: TargetAst,
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
    TicketCounters {
        count: Value,
    },
}
