//! The replacements actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ReplacementActionAst {
    RegisterZoneReplacement {
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
        duration: ZoneReplacementDurationAst,
        optional: bool,
        choice_description: Option<String>,
        counters: Vec<(CounterType, u32)>,
        linked_exile_follow_up: Option<ironsmith_core::LinkedExileFollowUp>,
    },
    RegisterFutureZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        cause_policy: FutureZoneReplacementCausePolicyAst,
        link_exiled_to_source: bool,
    },
    RegisterDrawReplacement {
        player: PlayerFilter,
        replacement_effects: Vec<EffectAst>,
        duration: ZoneReplacementDurationAst,
    },
    RegisterManaReplacement {
        source_filter: ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: crate::effects::ReplacementApplyMode,
    },
    RegisterDamagedBySourceZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    },
    RegisterEnterUnderControlReplacement {
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    },
    RegisterEnterTappedReplacement {
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    },
    RegisterNextBatchEnterWithCounters {
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
    },
}
