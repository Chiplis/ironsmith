//! The replacements actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
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
        /// "If an instant or sorcery spell cast this way would be put into
        /// your graveyard, exile it instead" wording.
        cast_this_way_surface: bool,
    },
    RegisterDrawReplacement {
        player: PlayerFilter,
        replacement_effects: Vec<EffectAst>,
        duration: ZoneReplacementDurationAst,
        player_target: Option<TargetAst>,
        display: Option<String>,
    },
    RegisterManaReplacement {
        source_filter: ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: crate::effects::ReplacementApplyMode,
    },
    /// "Until end of turn, if you would put one or more <kind> counters on
    /// <filter>, put that many plus N <kind> counters on it instead."
    RegisterCounterPlacementReplacement {
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        additional: u32,
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
    RegisterEnterWithCountersReplacement {
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
        mode: crate::effects::ReplacementApplyMode,
    },
    RegisterNextBatchEnterWithCounters {
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
    },
    RegisterDamageMultiplier {
        spec: ironsmith_core::RegisterDamageMultiplierEffect,
    },
    RegisterDamageAddition {
        spec: ironsmith_core::RegisterDamageAdditionEffect,
    },
    RegisterManaRewrite {
        rule: ironsmith_core::ManaOutputRewrite,
        target: Option<TargetAst>,
        mode: crate::effects::ReplacementApplyMode,
        display: String,
    },
    RegisterManaSpendPermission { permission: ironsmith_core::ManaSpendPermission, until: Until, display: String },
}
