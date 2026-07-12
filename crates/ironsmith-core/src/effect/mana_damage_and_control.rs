use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfChosenColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub fixed_option: Option<crate::color::Color>,
}

impl AddManaOfChosenColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: None,
        }
    }

    pub fn with_fixed_option(
        amount: impl Into<Value>,
        player: PlayerFilter,
        fixed: crate::color::Color,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: Some(fixed),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfImprintedColorsEffect;

impl AddManaOfImprintedColorsEffect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AddManaOfImprintedColorsEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfColorsAmongEffect {
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
}

impl AddManaOfColorsAmongEffect {
    pub fn new(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self { filter, player }
    }

    pub fn you(filter: ObjectFilter) -> Self {
        Self::new(filter, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddScaledManaEffect {
    pub mana: Vec<crate::mana::ManaSymbol>,
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddScaledManaEffect {
    pub fn new(mana: Vec<crate::mana::ManaSymbol>, amount: Value, player: PlayerFilter) -> Self {
        Self {
            mana,
            amount: amount.into_unhinted(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayEnergyEffect {
    pub amount: Value,
    pub player: ChooseSpec,
}

impl PayEnergyEffect {
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayAnyEnergyEffect {
    pub player: ChooseSpec,
    pub min_amount: u32,
}

impl PayAnyEnergyEffect {
    pub fn new(player: ChooseSpec, min_amount: u32) -> Self {
        Self { player, min_amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayAnyLifeEffect {
    pub player: ChooseSpec,
    pub min_amount: u32,
}

impl PayAnyLifeEffect {
    pub fn new(player: ChooseSpec, min_amount: u32) -> Self {
        Self { player, min_amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsultTopOfLibraryStopRule {
    FirstMatch,
    MatchCount(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestroyEffect {
    pub spec: ChooseSpec,
    pub target: ChooseSpec,
    pub no_regen: bool,
}

impl DestroyEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            spec: target.clone(),
            target,
            no_regen: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestroyNoRegenerationEffect {
    pub filter: Option<ObjectFilter>,
    pub target: Option<ChooseSpec>,
}

impl DestroyNoRegenerationEffect {
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            filter: Some(filter),
            target: None,
        }
    }

    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            filter: None,
            target: Some(target),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeEffect {
    pub filter: ObjectFilter,
    pub count: i32,
    pub event_object_tags: Vec<TagKey>,
    pub event_source_tags: Vec<TagKey>,
}

impl SacrificeEffect {
    pub fn with_event_object_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_object_tags.push(tag.into());
        self
    }

    pub fn with_event_source_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_source_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscardEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub random: bool,
    pub any_number: bool,
    pub card_filter: Option<ObjectFilter>,
    pub tag: Option<crate::tag::TagKey>,
}

impl DiscardEffect {
    pub fn new_with_filter(
        count: impl Into<Value>,
        player: PlayerFilter,
        random: bool,
        card_filter: Option<ObjectFilter>,
    ) -> Self {
        Self {
            count: count.into(),
            player,
            random,
            any_number: false,
            card_filter,
            tag: None,
        }
    }

    pub fn with_any_number(mut self, any_number: bool) -> Self {
        self.any_number = any_number;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.tag = Some(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersFromSourceEffect {
    pub counter_type: Option<crate::counter::CounterType>,
    pub display_x: bool,
    pub remove_all: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DealDistributedDamageEffect {
    pub amount: Value,
    pub target: ChooseSpec,
}

impl DealDistributedDamageEffect {
    pub fn new(amount: Value, target: ChooseSpec) -> Self {
        Self { amount, target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachCounterKindPutOrRemoveEffect {
    pub target: ChooseSpec,
    pub all_kinds: bool,
    pub fixed_counter_type: Option<crate::counter::CounterType>,
    pub optional_action: bool,
}

impl ForEachCounterKindPutOrRemoveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            all_kinds: true,
            fixed_counter_type: None,
            optional_action: false,
        }
    }

    pub fn one_kind(target: ChooseSpec) -> Self {
        Self {
            target,
            all_kinds: false,
            fixed_counter_type: None,
            optional_action: false,
        }
    }

    pub fn fixed_counter_type(
        target: ChooseSpec,
        counter_type: crate::counter::CounterType,
        optional_action: bool,
    ) -> Self {
        Self {
            target,
            all_kinds: false,
            fixed_counter_type: Some(counter_type),
            optional_action,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutCounterOfChosenKindEffect {
    pub target: ChooseSpec,
}

impl PutCounterOfChosenKindEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOutEffect {
    pub target: ChooseSpec,
}

impl PhaseOutEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseInEffect {
    pub target: ChooseSpec,
}

impl PhaseInEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromCombatEffect {
    pub target: ChooseSpec,
}

impl RemoveFromCombatEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawForEachTaggedMatchingEffect {
    pub tag: crate::tag::TagKey,
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
}

impl DrawForEachTaggedMatchingEffect {
    pub fn new(player: PlayerFilter, tag: crate::tag::TagKey, filter: ObjectFilter) -> Self {
        Self {
            tag,
            filter,
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub moved_tags: Vec<crate::tag::TagKey>,
    pub accumulated_tags: Vec<crate::tag::TagKey>,
}

impl ExileTopOfLibraryEffect {
    pub fn new(count: Value, player: PlayerFilter) -> Self {
        Self {
            count,
            player,
            moved_tags: Vec::new(),
            accumulated_tags: Vec::new(),
        }
    }

    pub fn tag_moved(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.moved_tags.push(tag.into());
        self
    }

    pub fn append_tagged(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.accumulated_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageEffect<E> {
    pub amount: Value,
    pub target: ChooseSpec,
    pub until: Until,
    pub follow_up_effects: Vec<E>,
    pub source_of_your_choice: bool,
    pub protect_you_and_permanents_you_control: bool,
}

impl<E> PreventDamageEffect<E> {
    pub fn new(amount: Value, target: ChooseSpec, until: Until) -> Self {
        Self {
            amount,
            target,
            until,
            follow_up_effects: Vec::new(),
            source_of_your_choice: false,
            protect_you_and_permanents_you_control: false,
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    pub fn with_source_of_your_choice(mut self) -> Self {
        self.source_of_your_choice = true;
        self
    }

    pub fn protecting_you_and_permanents_you_control(mut self) -> Self {
        self.protect_you_and_permanents_you_control = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageToTargetEffect<E> {
    pub target: ChooseSpec,
    pub until: Until,
    pub follow_up_effects: Vec<E>,
}

impl<E> PreventAllDamageToTargetEffect<E> {
    pub fn new(target: ChooseSpec, until: Until) -> Self {
        Self {
            target,
            until,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventNextTimeDamageEffect<E = ()> {
    pub source: PreventNextTimeDamageSource,
    pub target: PreventNextTimeDamageTarget,
    pub reflect_damage_to_source_controller: bool,
    pub follow_up_effects: Vec<E>,
}

impl<E> PreventNextTimeDamageEffect<E> {
    pub fn new(source: PreventNextTimeDamageSource, target: PreventNextTimeDamageTarget) -> Self {
        Self {
            source,
            target,
            reflect_damage_to_source_controller: false,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    pub fn reflecting_to_source_controller(mut self) -> Self {
        self.reflect_damage_to_source_controller = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextDamageDestination {
    Controller,
    TargetObject,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextDamageToTargetEffect {
    pub amount: Option<Value>,
    pub protected_target: Option<ChooseSpec>,
    pub destination: RedirectNextDamageDestination,
    pub destination_target: Option<ChooseSpec>,
}

impl RedirectNextDamageToTargetEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: Some(amount.into()),
            protected_target: None,
            destination: RedirectNextDamageDestination::TargetObject,
            destination_target: Some(target),
        }
    }

    pub fn to_controller(amount: impl Into<Value>, protected_target: ChooseSpec) -> Self {
        Self {
            amount: Some(amount.into()),
            protected_target: Some(protected_target),
            destination: RedirectNextDamageDestination::Controller,
            destination_target: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextTimeDamageToSourceEffect {
    pub source: RedirectNextTimeDamageSource,
    pub target: Option<ChooseSpec>,
    pub destination: RedirectNextTimeDamageDestination,
    pub destination_target: Option<ChooseSpec>,
    pub all_this_turn: bool,
}

impl RedirectNextTimeDamageToSourceEffect {
    pub fn new(source: RedirectNextTimeDamageSource, target: ChooseSpec) -> Self {
        Self {
            source,
            target: Some(target),
            destination: RedirectNextTimeDamageDestination::SourceObject,
            destination_target: None,
            all_this_turn: false,
        }
    }

    pub fn from_source_target(source: ChooseSpec) -> Self {
        Self {
            source: RedirectNextTimeDamageSource::Target(source),
            target: None,
            destination: RedirectNextTimeDamageDestination::SourceController,
            destination_target: None,
            all_this_turn: false,
        }
    }

    pub fn to_controller(mut self) -> Self {
        self.destination = RedirectNextTimeDamageDestination::Controller;
        self.destination_target = None;
        self
    }

    pub fn to_source_controller(mut self) -> Self {
        self.destination = RedirectNextTimeDamageDestination::SourceController;
        self.destination_target = None;
        self
    }

    pub fn to_target(mut self, target: ChooseSpec) -> Self {
        self.destination = RedirectNextTimeDamageDestination::TargetObject;
        self.destination_target = Some(target);
        self
    }

    pub fn all_this_turn(mut self) -> Self {
        self.all_this_turn = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectAllDamageThisTurnToTargetEffect {
    pub player_filter: PlayerFilter,
    pub object_filter: ObjectFilter,
    pub target: ChooseSpec,
}

impl RedirectAllDamageThisTurnToTargetEffect {
    pub fn new(
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: ChooseSpec,
    ) -> Self {
        Self {
            player_filter,
            object_filter,
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayTaggedEffect {
    pub tag: crate::tag::TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
    pub allow_land: bool,
    pub allow_any_color_for_cast: bool,
    pub while_on_top_of_library: bool,
    pub filter: Option<ObjectFilter>,
    /// True when the granted pool holds more than one card, selecting plural
    /// "cast spells from among those exiled cards" wording over the singular
    /// "cast that card this turn". Purely cosmetic; resolution is unaffected.
    pub cast_pool_is_plural: bool,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: crate::tag::TagKey,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self {
            tag,
            player,
            duration,
            allow_land,
            allow_any_color_for_cast,
            while_on_top_of_library: false,
            filter: None,
            cast_pool_is_plural: false,
        }
    }

    pub fn cast_pool_is_plural(mut self, plural: bool) -> Self {
        self.cast_pool_is_plural = plural;
        self
    }

    pub fn while_on_top_of_library(mut self) -> Self {
        self.while_on_top_of_library = true;
        self
    }

    pub fn with_filter(mut self, filter: ObjectFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
    pub optional: bool,
    pub choice_description: Option<String>,
    pub counters: Vec<(CounterType, u32)>,
}

impl RegisterZoneReplacementEffect {
    pub fn new(
        target: ChooseSpec,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
            optional: false,
            choice_description: None,
            counters: Vec::new(),
        }
    }

    pub fn optional(mut self, description: impl Into<String>) -> Self {
        self.optional = true;
        self.choice_description = Some(description.into());
        self
    }

    pub fn with_counters(mut self, counters: Vec<(CounterType, u32)>) -> Self {
        self.counters = counters;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterFutureZoneReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
    pub cause_filter: Option<crate::cause_model::CauseFilter>,
    pub require_cause_source_match: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterDrawReplacementEffect<E = ()> {
    pub player: PlayerFilter,
    pub replacement_effects: Vec<E>,
    pub mode: ReplacementApplyMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterManaReplacementEffect {
    pub source_filter: crate::filter_model::ObjectFilter,
    pub replacement_mana: Vec<ManaSymbol>,
    pub mode: ReplacementApplyMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterDamagedBySourceZoneReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterEnterUnderControlReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub mode: ReplacementApplyMode,
}

impl RegisterEnterUnderControlReplacementEffect {
    pub fn new(filter: crate::filter_model::ObjectFilter, mode: ReplacementApplyMode) -> Self {
        Self { filter, mode }
    }
}

impl RegisterFutureZoneReplacementEffect {
    pub fn new(
        filter: crate::filter_model::ObjectFilter,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
            cause_filter: None,
            require_cause_source_match: false,
        }
    }

    pub fn with_cause_filter(mut self, cause_filter: crate::cause_model::CauseFilter) -> Self {
        self.cause_filter = Some(cause_filter);
        self
    }

    pub fn requiring_cause_source_match(mut self) -> Self {
        self.require_cause_source_match = true;
        self
    }
}

impl<E> RegisterDrawReplacementEffect<E> {
    pub fn new(
        player: PlayerFilter,
        replacement_effects: Vec<E>,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            player,
            replacement_effects,
            mode,
        }
    }
}

impl RegisterManaReplacementEffect {
    pub fn new(
        source_filter: crate::filter_model::ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            source_filter,
            replacement_mana,
            mode,
        }
    }
}

impl RegisterDamagedBySourceZoneReplacementEffect {
    pub fn new(
        filter: crate::filter_model::ObjectFilter,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnEffect;

impl LearnEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvestigateEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl InvestigateEffect {
    pub fn new(count: Value, player: PlayerFilter) -> Self {
        Self { count, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GainLifeEffect {
    pub amount: Value,
    pub player: ChooseSpec,
}

impl GainLifeEffect {
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn with_filter(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(amount, ChooseSpec::Player(player))
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::with_filter(amount, PlayerFilter::You)
    }

    pub fn target_player(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::target_player())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncreaseSpeedEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl IncreaseSpeedEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReduceSpeedEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub minimum: u8,
}

impl ReduceSpeedEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter, minimum: u8) -> Self {
        Self {
            amount: amount.into(),
            player,
            minimum,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CipherEffect;

impl CipherEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnearthEffect;

impl UnearthEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuCostEffect;

impl NinjutsuCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SneakCostEffect;

impl SneakCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConspireCostEffect;

impl ConspireCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuEffect;

impl NinjutsuEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegenerateEffect<E = ()> {
    pub target: ChooseSpec,
    pub duration: Until,
    pub follow_up_effects: Vec<E>,
}

impl<E> RegenerateEffect<E> {
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn source(duration: Until) -> Self {
        Self::new(ChooseSpec::Source, duration)
    }

    pub fn target_creature(duration: Until) -> Self {
        Self::new(ChooseSpec::creature(), duration)
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaFromCommanderColorIdentityEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddManaFromCommanderColorIdentityEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub available_colors: Option<Vec<Color>>,
    pub distinct_colors: bool,
}

impl AddManaOfAnyColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: None,
            distinct_colors: false,
        }
    }

    pub fn distinct(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: None,
            distinct_colors: true,
        }
    }

    pub fn restricted(
        amount: impl Into<Value>,
        player: PlayerFilter,
        available_colors: Vec<Color>,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: Some(available_colors),
            distinct_colors: false,
        }
    }

    pub fn restricted_distinct(
        amount: impl Into<Value>,
        player: PlayerFilter,
        available_colors: Vec<Color>,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: Some(available_colors),
            distinct_colors: true,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }

    pub fn you_distinct(amount: impl Into<Value>) -> Self {
        Self::distinct(amount, PlayerFilter::You)
    }

    pub fn you_restricted(amount: impl Into<Value>, available_colors: Vec<Color>) -> Self {
        Self::restricted(amount, PlayerFilter::You, available_colors)
    }

    pub fn you_restricted_distinct(amount: impl Into<Value>, available_colors: Vec<Color>) -> Self {
        Self::restricted_distinct(amount, PlayerFilter::You, available_colors)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyOneColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddManaOfAnyOneColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaEffect {
    pub mana: Vec<ManaSymbol>,
    pub player: PlayerFilter,
}

impl AddManaEffect {
    pub fn new(mana: Vec<ManaSymbol>, player: PlayerFilter) -> Self {
        Self { mana, player }
    }

    pub fn you(mana: Vec<ManaSymbol>) -> Self {
        Self::new(mana, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenownEffect {
    pub amount: u32,
}

impl RenownEffect {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveCaseEffect;

impl SolveCaseEffect {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BolsterEffect {
    pub amount: u32,
}

impl BolsterEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipEffect {
    pub target: ChooseSpec,
}

impl FlipEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoseTheGameEffect {
    pub player: PlayerFilter,
}

impl LoseTheGameEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseColorEffect {
    pub chooser: PlayerFilter,
}

impl ChooseColorEffect {
    pub fn new(chooser: PlayerFilter) -> Self {
        Self { chooser }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseLandTypeEffect {
    pub chooser: PlayerFilter,
    pub exclude_basic: bool,
}

impl ChooseLandTypeEffect {
    pub fn new(chooser: PlayerFilter, exclude_basic: bool) -> Self {
        Self {
            chooser,
            exclude_basic,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCreatureTypeEffect {
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<Subtype>,
}

impl ChooseCreatureTypeEffect {
    pub fn new(chooser: PlayerFilter, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            chooser,
            excluded_subtypes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    pub player: PlayerFilter,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetLifeTotalEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl SetLifeTotalEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeLifeTotalsEffect {
    pub player1: PlayerFilter,
    pub player2: PlayerFilter,
}

impl ExchangeLifeTotalsEffect {
    pub fn new(player1: PlayerFilter, player2: PlayerFilter) -> Self {
        Self { player1, player2 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleManaPoolEffect {
    pub player: PlayerFilter,
}

impl DoubleManaPoolEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyManaPoolEffect {
    pub player: PlayerFilter,
}

impl EmptyManaPoolEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndTurnEffect {
    pub player: PlayerFilter,
}

impl EndTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipTurnEffect {
    pub player: PlayerFilter,
}

impl SkipTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipDrawStepEffect {
    pub player: PlayerFilter,
}

impl SkipDrawStepEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipNextCombatPhaseThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipNextCombatPhaseThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalLandPlaysEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub duration: Until,
}

impl AdditionalLandPlaysEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter, duration: Until) -> Self {
        Self {
            count: count.into(),
            player,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeMonarchEffect {
    pub player: PlayerFilter,
}

impl BecomeMonarchEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RingTemptsYouEffect {
    pub player: PlayerFilter,
}

impl RingTemptsYouEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentureIntoDungeonEffect {
    pub player: PlayerFilter,
    pub undercity_if_no_active: bool,
}

impl VentureIntoDungeonEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: false,
        }
    }

    pub fn via_initiative(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakeInitiativeEffect {
    pub player: PlayerFilter,
}

impl TakeInitiativeEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoisonCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl PoisonCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlCombatChoicesThisTurnEffect {
    pub attackers: bool,
    pub blockers: bool,
    pub this_combat: bool,
}

impl ControlCombatChoicesThisTurnEffect {
    pub fn new(attackers: bool, blockers: bool) -> Self {
        Self::new_with_surface(attackers, blockers, false)
    }

    pub fn new_with_surface(attackers: bool, blockers: bool, this_combat: bool) -> Self {
        Self {
            attackers,
            blockers,
            this_combat,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollDieEffect {
    pub player: PlayerFilter,
    pub sides: u32,
    pub die_text: Option<String>,
}

impl RollDieEffect {
    pub fn new(player: PlayerFilter, sides: u32) -> Self {
        Self {
            player,
            sides,
            die_text: None,
        }
    }

    pub fn new_with_die_text(player: PlayerFilter, sides: u32, die_text: Option<String>) -> Self {
        Self {
            player,
            sides,
            die_text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollDiceChooseResultEffect {
    pub player: PlayerFilter,
    pub count: u32,
    pub sides: u32,
    pub die_text: Option<String>,
}

impl RollDiceChooseResultEffect {
    pub fn new(player: PlayerFilter, count: u32, sides: u32) -> Self {
        Self {
            player,
            count,
            sides,
            die_text: None,
        }
    }

    pub fn new_with_die_text(
        player: PlayerFilter,
        count: u32,
        sides: u32,
        die_text: Option<String>,
    ) -> Self {
        Self {
            player,
            count,
            sides,
            die_text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitGiftGivenEffect {
    pub recipient: PlayerFilter,
}

impl EmitGiftGivenEffect {
    pub fn new(recipient: PlayerFilter) -> Self {
        Self { recipient }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNamedOptionEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<String>,
}

impl ChooseNamedOptionEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<String>) -> Self {
        Self { chooser, options }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipCombatPhasesEffect {
    pub player: PlayerFilter,
}

impl SkipCombatPhasesEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipMainPhasesThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipMainPhasesThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipCombatPhasesThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipCombatPhasesThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeZonesEffect {
    pub player: PlayerFilter,
    pub zone1: crate::zone::Zone,
    pub zone2: crate::zone::Zone,
}

impl ExchangeZonesEffect {
    pub fn new(player: PlayerFilter, zone1: crate::zone::Zone, zone2: crate::zone::Zone) -> Self {
        Self {
            player,
            zone1,
            zone2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuraSwapEffect;

impl AuraSwapEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeValuesEffect {
    pub left: ExchangeValueOperand,
    pub right: ExchangeValueOperand,
    pub duration: Until,
}

impl ExchangeValuesEffect {
    pub fn new(left: ExchangeValueOperand, right: ExchangeValueOperand, duration: Until) -> Self {
        Self {
            left,
            right,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub land_filter: ObjectFilter,
    pub allow_colorless: bool,
    pub same_type: bool,
}

impl AddManaOfLandProducedTypesEffect {
    pub fn new(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringDamageTargetEffect {
    pub tag: TagKey,
}

impl TagTriggeringDamageTargetEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertEffect {
    pub target: ChooseSpec,
}

impl ConvertEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutStickerEffect {
    pub target: ChooseSpec,
    pub action: crate::event_model::KeywordActionKind,
}

impl PutStickerEffect {
    pub fn new(target: ChooseSpec, action: crate::event_model::KeywordActionKind) -> Self {
        Self { target, action }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReorderGraveyardEffect {
    pub player: PlayerFilter,
}

impl ReorderGraveyardEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnEffect {
    pub player: PlayerFilter,
}

impl ExtraTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnAfterNextTurnEffect {
    pub player: PlayerFilter,
}

impl ExtraTurnAfterNextTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalPhase {
    Combat,
    Main,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditionalPhasesEffect {
    pub phases: Vec<AdditionalPhase>,
}

impl AdditionalPhasesEffect {
    pub fn new(phases: Vec<AdditionalPhase>) -> Self {
        Self { phases }
    }

    pub fn combat() -> Self {
        Self::new(vec![AdditionalPhase::Combat])
    }

    pub fn combat_then_main() -> Self {
        Self::new(vec![AdditionalPhase::Combat, AdditionalPhase::Main])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringObjectEffect {
    pub tag: TagKey,
}

impl TagTriggeringObjectEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringSourceEffect {
    pub tag: TagKey,
}

impl TagTriggeringSourceEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringBlockersEffect {
    pub tag: TagKey,
    pub filter: Option<ObjectFilter>,
}

impl TagTriggeringBlockersEffect {
    pub fn new(tag: impl Into<TagKey>, filter: Option<ObjectFilter>) -> Self {
        Self {
            tag: tag.into(),
            filter,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagAttachedToSourceEffect {
    pub tag: TagKey,
}

impl TagAttachedToSourceEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveAllCountersEffect {
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveAllCountersEffect {
    pub fn new(from: ChooseSpec, to: ChooseSpec) -> Self {
        Self { from, to }
    }

    pub fn between_creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveOneCounterEffect {
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveOneCounterEffect {
    pub fn new(from: ChooseSpec, to: ChooseSpec) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub count: Value,
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        count: impl Into<Value>,
        from: ChooseSpec,
        to: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            count: count.into(),
            from,
            to,
        }
    }

    pub fn plus_one_counters(count: impl Into<Value>) -> Self {
        Self::new(
            crate::counter::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::creature(),
            ChooseSpec::creature(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProliferateEffect {
    pub count: Value,
}

impl ProliferateEffect {
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

impl Default for ProliferateEffect {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WinTheGameEffect {
    pub player: PlayerFilter,
}

impl WinTheGameEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastSourceEffect {
    pub without_paying_mana_cost: bool,
    pub require_exile: bool,
    pub cast_as_suspend: bool,
}

impl CastSourceEffect {
    pub fn new() -> Self {
        Self {
            without_paying_mana_cost: false,
            require_exile: false,
            cast_as_suspend: false,
        }
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn require_exile(mut self) -> Self {
        self.require_exile = true;
        self
    }

    pub fn cast_as_suspend(mut self) -> Self {
        self.cast_as_suspend = true;
        self
    }
}

impl Default for CastSourceEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionObjectTag {
    pub effect_id: EffectId,
    pub tag: TagKey,
    pub use_affected_memory: bool,
}

impl EmitKeywordActionObjectTag {
    pub fn affected(effect_id: EffectId, tag: impl Into<TagKey>) -> Self {
        Self {
            effect_id,
            tag: tag.into(),
            use_affected_memory: true,
        }
    }

    pub fn chosen(effect_id: EffectId, tag: impl Into<TagKey>) -> Self {
        Self {
            effect_id,
            tag: tag.into(),
            use_affected_memory: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionEffect {
    pub action: crate::event_model::KeywordActionKind,
    pub amount: u32,
    pub object_tags: Vec<EmitKeywordActionObjectTag>,
}

impl EmitKeywordActionEffect {
    pub fn new(action: crate::event_model::KeywordActionKind, amount: u32) -> Self {
        Self {
            action,
            amount,
            object_tags: Vec::new(),
        }
    }

    pub fn with_affected_object_memory_tag(
        mut self,
        effect_id: EffectId,
        tag: impl Into<TagKey>,
    ) -> Self {
        self.object_tags
            .push(EmitKeywordActionObjectTag::affected(effect_id, tag));
        self
    }

    pub fn with_chosen_object_memory_tag(
        mut self,
        effect_id: EffectId,
        tag: impl Into<TagKey>,
    ) -> Self {
        self.object_tags
            .push(EmitKeywordActionObjectTag::chosen(effect_id, tag));
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscardHandEffect {
    pub player: PlayerFilter,
}

impl DiscardHandEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardTypeEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<CardType>,
}

impl ChooseCardTypeEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<CardType>) -> Self {
        Self { chooser, options }
    }

    pub fn all_card_types() -> &'static [CardType] {
        &[
            CardType::Artifact,
            CardType::Battle,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Instant,
            CardType::Kindred,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Sorcery,
        ]
    }

    pub fn card_type_options(&self) -> Vec<CardType> {
        if self.options.is_empty() {
            Self::all_card_types().to_vec()
        } else {
            self.options.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardNameEffect {
    pub chooser: PlayerFilter,
    pub filter: Option<ObjectFilter>,
    pub tag: TagKey,
}

impl ChooseCardNameEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: Option<ObjectFilter>,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
        }
    }
}

/// When a player-control effect starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlStart {
    /// Starts immediately when the effect resolves.
    Immediate,
    /// Starts at the beginning of the target player's next turn.
    NextTurn,
}

/// How long a player-control effect lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlDuration {
    /// Until end of the current turn.
    UntilEndOfTurn,
    /// Until the source leaves the battlefield.
    UntilSourceLeaves,
    /// No duration limit.
    Forever,
}

/// Effect that lets a player control another player's decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlPlayerEffect {
    /// Which player is controlled.
    pub player: PlayerFilter,
    /// When control begins.
    pub start: PlayerControlStart,
    /// How long control lasts.
    pub duration: PlayerControlDuration,
    /// Target spec if this effect targets a player.
    pub target_spec: Option<ChooseSpec>,
}

impl ControlPlayerEffect {
    /// Create a new control player effect.
    pub fn new(
        player: PlayerFilter,
        start: PlayerControlStart,
        duration: PlayerControlDuration,
    ) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            start,
            duration,
            target_spec,
        }
    }

    /// Control a player until end of turn.
    pub fn until_end_of_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player during their next turn.
    pub fn during_next_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::NextTurn,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player indefinitely.
    pub fn forever(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::Forever,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatDamagePreventionTarget {
    All,
    Players,
    You,
    From(ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageEffect {
    pub target: CombatDamagePreventionTarget,
    pub until: Until,
}

impl PreventAllCombatDamageEffect {
    pub fn new(target: CombatDamagePreventionTarget, until: Until) -> Self {
        Self { target, until }
    }
}

/// What a prevention shield protects.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventionTarget {
    /// Protects a specific player.
    Player(crate::ids::PlayerId),
    /// Protects a specific permanent.
    Permanent(crate::ids::ObjectId),
    /// Protects all permanents matching a filter.
    PermanentsMatching(ObjectFilter),
    /// Protects all players.
    Players,
    /// Protects "you" (the shield's controller).
    You,
    /// Protects "you and permanents you control".
    YouAndPermanentsYouControl,
    /// Protects everything.
    All,
}

/// Filter for what kind of damage a prevention shield applies to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DamageFilter {
    /// Only prevent combat damage.
    pub combat_only: bool,
    /// Only prevent noncombat damage.
    pub noncombat_only: bool,
    /// Only prevent damage from sources matching this filter.
    pub from_source: Option<ObjectFilter>,
    /// Only prevent damage from sources of these colors.
    pub from_colors: Option<Vec<Color>>,
    /// Only prevent damage from sources of these card types.
    pub from_card_types: Option<Vec<CardType>>,
    /// Only prevent damage from a specific source.
    pub from_specific_source: Option<crate::ids::ObjectId>,
}

impl DamageFilter {
    /// Create a filter that matches all damage.
    pub fn all() -> Self {
        Self::default()
    }

    /// Create a filter for combat damage only.
    pub fn combat() -> Self {
        Self {
            combat_only: true,
            ..Default::default()
        }
    }

    /// Create a filter for noncombat damage only.
    pub fn noncombat() -> Self {
        Self {
            noncombat_only: true,
            ..Default::default()
        }
    }

    /// Create a filter for damage from sources of a specific color.
    pub fn from_color(color: Color) -> Self {
        Self {
            from_colors: Some(vec![color]),
            ..Default::default()
        }
    }

    /// Create a filter for damage from a specific source.
    pub fn from_source(source: crate::ids::ObjectId) -> Self {
        Self {
            from_specific_source: Some(source),
            ..Default::default()
        }
    }

    /// Check if this filter matches the given damage parameters.
    pub fn matches(
        &self,
        is_combat: bool,
        source: crate::ids::ObjectId,
        source_colors: &ColorSet,
        source_card_types: &[CardType],
    ) -> bool {
        if self.combat_only && !is_combat {
            return false;
        }
        if self.noncombat_only && is_combat {
            return false;
        }
        if let Some(specific) = self.from_specific_source
            && source != specific
        {
            return false;
        }
        if let Some(ref colors) = self.from_colors {
            let matches_color = colors.iter().any(|c| source_colors.contains(*c));
            if !matches_color {
                return false;
            }
        }
        if let Some(ref types) = self.from_card_types {
            let matches_type = types.iter().any(|t| source_card_types.contains(t));
            if !matches_type {
                return false;
            }
        }
        true
    }
}

/// Effect that prevents all damage until a duration expires.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageEffect {
    /// What this shield protects.
    pub target: PreventionTarget,
    /// What kinds of damage this shield prevents.
    pub damage_filter: DamageFilter,
    /// Whether the source is chosen as the effect resolves.
    pub source_of_your_choice: bool,
    pub until: Until,
}

impl PreventAllDamageEffect {
    /// Create a new prevent-all-damage effect.
    pub fn new(target: PreventionTarget, damage_filter: DamageFilter, until: Until) -> Self {
        Self {
            target,
            damage_filter,
            source_of_your_choice: false,
            until,
        }
    }

    /// Restrict this prevention shield to a source chosen as the effect resolves.
    pub fn with_source_of_your_choice(mut self) -> Self {
        self.source_of_your_choice = true;
        self
    }

    /// Prevent all damage to everything.
    pub fn all(until: Until) -> Self {
        Self::new(PreventionTarget::All, DamageFilter::all(), until)
    }

    /// Prevent all damage to the controller.
    pub fn to_you(until: Until) -> Self {
        Self::new(PreventionTarget::You, DamageFilter::all(), until)
    }

    /// Prevent all damage to permanents matching the filter.
    pub fn matching(filter: ObjectFilter, until: Until) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            DamageFilter::all(),
            until,
        )
    }

    /// Prevent all damage to everything with a damage filter.
    pub fn all_with_filter(damage_filter: DamageFilter, until: Until) -> Self {
        Self::new(PreventionTarget::All, damage_filter, until)
    }

    /// Prevent all damage to permanents matching the filter with a damage filter.
    pub fn matching_with_filter(
        filter: ObjectFilter,
        damage_filter: DamageFilter,
        until: Until,
    ) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            damage_filter,
            until,
        )
    }

    /// Prevent all damage to creatures you control.
    pub fn your_creatures(until: Until) -> Self {
        Self::matching(ObjectFilter::creature().you_control(), until)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateEmblemEffect<E> {
    pub emblem: E,
}

impl<E> CreateEmblemEffect<E> {
    pub fn new(emblem: E) -> Self {
        Self { emblem }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantEffect<G, D> {
    pub grantable: G,
    pub target: ChooseSpec,
    pub duration: D,
}

impl<G, D> GrantEffect<G, D> {
    pub fn new(grantable: G, target: ChooseSpec, duration: D) -> Self {
        Self {
            grantable,
            target,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantBySpecEffect<S, D> {
    pub spec: S,
    pub player: PlayerFilter,
    pub duration: D,
}

impl<S, D> GrantBySpecEffect<S, D> {
    pub fn new(spec: S, player: PlayerFilter, duration: D) -> Self {
        Self {
            spec,
            player,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleObjectsIntoLibraryEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
}

impl ShuffleObjectsIntoLibraryEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter) -> Self {
        Self { target, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeTextBoxesEffect {
    pub target: ChooseSpec,
}

impl ExchangeTextBoxesEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl EnergyCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl ExperienceCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TicketCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl TicketCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoverEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl DiscoverEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessEffect {
    pub target: ChooseSpec,
    pub power: Value,
    pub toughness: Value,
    pub duration: Until,
}

impl SetBasePowerToughnessEffect {
    pub fn new(
        target: ChooseSpec,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessForEachEffect {
    pub target: ChooseSpec,
    pub power_per: i32,
    pub toughness_per: i32,
    pub count: Value,
    pub duration: Until,
}

impl ModifyPowerToughnessForEachEffect {
    pub fn new(
        target: ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: Value,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power_per,
            toughness_per,
            count,
            duration,
        }
    }

    pub fn symmetric(target: ChooseSpec, per: i32, count: Value, duration: Until) -> Self {
        Self::new(target, per, per, count, duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToAnyCountersEffect {
    pub max_count: Value,
    pub target: ChooseSpec,
}

impl RemoveUpToAnyCountersEffect {
    pub fn new(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            max_count: max_count.into(),
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub max_count: Value,
    pub target: ChooseSpec,
}

impl RemoveUpToCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        max_count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            max_count: max_count.into(),
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    pub target: ChooseSpec,
}

impl AttachToEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconfigureEffect {
    pub target: ChooseSpec,
}

impl ReconfigureEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    pub objects: ChooseSpec,
    pub target: ChooseSpec,
}

impl AttachObjectsEffect {
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self { objects, target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnattachObjectsEffect {
    pub objects: ChooseSpec,
}

impl UnattachObjectsEffect {
    pub fn new(objects: ChooseSpec) -> Self {
        Self { objects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevealTopEffect {
    pub player: PlayerFilter,
    pub tag: Option<TagKey>,
}

impl RevealTopEffect {
    pub fn new(player: PlayerFilter, tag: Option<TagKey>) -> Self {
        Self { player, tag }
    }

    pub fn tagged(player: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        Self::new(player, Some(tag.into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAsAuraOptions {
    pub attachment_filter: ObjectFilter,
    pub remove_all_abilities: bool,
}

impl ReturnAsAuraOptions {
    pub fn new(attachment_filter: ObjectFilter) -> Self {
        Self {
            attachment_filter,
            remove_all_abilities: false,
        }
    }

    pub fn remove_all_abilities(mut self) -> Self {
        self.remove_all_abilities = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToBattlefieldEffect {
    pub target: ChooseSpec,
    pub tapped: bool,
    pub as_aura: Option<ReturnAsAuraOptions>,
}

impl ReturnFromGraveyardToBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool) -> Self {
        Self {
            target,
            tapped,
            as_aura: None,
        }
    }

    pub fn as_aura(mut self, attachment_filter: ObjectFilter) -> Self {
        self.as_aura = Some(ReturnAsAuraOptions::new(attachment_filter));
        self
    }

    pub fn as_aura_removing_all_abilities(mut self, attachment_filter: ObjectFilter) -> Self {
        self.as_aura = Some(ReturnAsAuraOptions::new(attachment_filter).remove_all_abilities());
        self
    }

    pub fn creature() -> Self {
        Self::new(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard)),
            false,
        )
    }

    pub fn creature_tapped() -> Self {
        Self::new(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard)),
            true,
        )
    }

    pub fn any_card() -> Self {
        Self::new(
            ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard),
            false,
        )
    }

    pub fn any_card_tapped() -> Self {
        Self::new(ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard), true)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToHandEffect {
    pub target: ChooseSpec,
    pub random: bool,
}

impl ReturnFromGraveyardToHandEffect {
    pub fn new(target: ChooseSpec, random: bool) -> Self {
        Self { target, random }
    }

    pub fn any_card() -> Self {
        Self::new(
            ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard),
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutOntoBattlefieldEffect {
    pub target: ChooseSpec,
    pub tapped: bool,
    pub controller: PlayerFilter,
}

impl PutOntoBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool, controller: PlayerFilter) -> Self {
        Self {
            target,
            tapped,
            controller,
        }
    }

    pub fn you_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::You)
    }

    pub fn owner_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::OwnerOf(ObjectRef::Target))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleLibraryEffect {
    pub player: PlayerFilter,
    pub target_spec: Option<ChooseSpec>,
}

impl ShuffleLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            target_spec,
        }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayMoveToZoneEffect {
    pub target: ChooseSpec,
    pub zone: crate::zone::Zone,
    pub decider: PlayerFilter,
}

impl MayMoveToZoneEffect {
    pub fn new(target: ChooseSpec, zone: crate::zone::Zone, decider: PlayerFilter) -> Self {
        Self {
            target,
            zone,
            decider,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookAtTopCardsEffect {
    pub player: PlayerFilter,
    pub count: Value,
    pub tag: TagKey,
    pub reveal: bool,
}

impl LookAtTopCardsEffect {
    pub fn new(player: PlayerFilter, count: impl Into<Value>, tag: impl Into<TagKey>) -> Self {
        Self {
            player,
            count: count.into(),
            tag: tag.into(),
            reveal: false,
        }
    }

    pub fn revealing(
        player: PlayerFilter,
        count: impl Into<Value>,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            player,
            count: count.into(),
            tag: tag.into(),
            reveal: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonstrosityEffect {
    pub n: Value,
}

impl MonstrosityEffect {
    pub fn new(n: impl Into<Value>) -> Self {
        Self { n: n.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolveEffect;

impl EvolveEffect {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoulbondPairEffect;

impl SoulbondPairEffect {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    pub target: ChooseSpec,
}

impl TransformEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub allow_land: bool,
    pub as_copy: bool,
    pub without_paying_mana_cost: bool,
    pub cost_reduction: Option<ManaCost>,
}

impl CastTaggedEffect {
    pub fn new(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            cost_reduction: None,
        }
    }

    pub fn allow_land(mut self) -> Self {
        self.allow_land = true;
        self
    }

    pub fn as_copy(mut self) -> Self {
        self.as_copy = true;
        self
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn cost_reduction(mut self, reduction: ManaCost) -> Self {
        self.cost_reduction = Some(reduction);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryBottomOrder {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryConsultMode {
    Reveal,
    Exile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutTaggedRemainderOnLibraryBottomEffect {
    pub tag: TagKey,
    pub keep_tagged: Option<TagKey>,
    pub order: LibraryBottomOrder,
    pub player: PlayerFilter,
}

impl PutTaggedRemainderOnLibraryBottomEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrder,
        player: PlayerFilter,
    ) -> Self {
        Self {
            tag: tag.into(),
            keep_tagged,
            order,
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersAmongEffect {
    pub count: u32,
    pub min_count: u32,
    pub dynamic_count: bool,
    pub display_x: bool,
    pub filter: ObjectFilter,
    pub counter_type: Option<crate::counter::CounterType>,
}

impl RemoveAnyCountersAmongEffect {
    pub fn new(count: u32, filter: ObjectFilter) -> Self {
        Self {
            count,
            min_count: count,
            dynamic_count: false,
            display_x: false,
            filter,
            counter_type: None,
        }
    }

    pub fn dynamic(min_count: u32, max_count: u32, filter: ObjectFilter, display_x: bool) -> Self {
        Self {
            count: max_count,
            min_count,
            dynamic_count: true,
            display_x,
            filter,
            counter_type: None,
        }
    }

    pub fn with_counter_type(mut self, counter_type: Option<crate::counter::CounterType>) -> Self {
        self.counter_type = counter_type;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificePlayerEffect {
    pub filter: ObjectFilter,
    pub count: Value,
    pub player: PlayerFilter,
}

impl SacrificePlayerEffect {
    pub fn new(filter: ObjectFilter, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            filter,
            count: count.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RearrangeLookedCardsInLibraryEffect {
    pub tag: TagKey,
    pub chooser: PlayerFilter,
    pub count: ChoiceCount,
}

impl RearrangeLookedCardsInLibraryEffect {
    pub fn new(tag: impl Into<TagKey>, chooser: PlayerFilter, count: ChoiceCount) -> Self {
        Self {
            tag: tag.into(),
            chooser,
            count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNewTargetsEffect {
    pub from_effect: EffectId,
    pub may: bool,
    pub chooser: Option<PlayerFilter>,
}

impl ChooseNewTargetsEffect {
    pub fn new(from_effect: EffectId, may: bool) -> Self {
        Self {
            from_effect,
            may,
            chooser: None,
        }
    }

    pub fn new_for_player(from_effect: EffectId, may: bool, chooser: PlayerFilter) -> Self {
        Self {
            from_effect,
            may,
            chooser: Some(chooser),
        }
    }

    pub fn may(from_effect: EffectId) -> Self {
        Self::new(from_effect, true)
    }

    pub fn may_for_player(from_effect: EffectId, chooser: PlayerFilter) -> Self {
        Self::new_for_player(from_effect, true, chooser)
    }

    pub fn must(from_effect: EffectId) -> Self {
        Self::new(from_effect, false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileInsteadOfGraveyardEffect {
    pub player: PlayerFilter,
}

impl ExileInsteadOfGraveyardEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTopOfLibraryEffect {
    pub player: PlayerFilter,
    pub mode: LibraryConsultMode,
    pub filter: ObjectFilter,
    pub stop_rule: ConsultTopOfLibraryStopRule,
    /// Optional cap on the total number of exposed cards. This combines with
    /// `FirstMatch` for "a matching card or N cards, whichever comes first."
    pub max_exposed: Option<Value>,
    pub all_tag: TagKey,
    pub match_tag: TagKey,
}

impl ConsultTopOfLibraryEffect {
    pub fn new(
        player: PlayerFilter,
        mode: LibraryConsultMode,
        filter: ObjectFilter,
        stop_rule: ConsultTopOfLibraryStopRule,
        all_tag: impl Into<TagKey>,
        match_tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            player,
            mode,
            filter,
            stop_rule,
            max_exposed: None,
            all_tag: all_tag.into(),
            match_tag: match_tag.into(),
        }
    }

    pub fn with_max_exposed(mut self, max_exposed: impl Into<Value>) -> Self {
        self.max_exposed = Some(max_exposed.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessEffect<E> {
    pub effects: Vec<E>,
    pub condition: EffectId,
    pub predicate: EffectPredicate,
}

impl<E> RepeatProcessEffect<E> {
    pub fn new(effects: Vec<E>, condition: EffectId, predicate: EffectPredicate) -> Self {
        Self {
            effects,
            condition,
            predicate,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatEffectsEffect<E> {
    pub count: Value,
    pub effects: Vec<E>,
}

impl<E> RepeatEffectsEffect<E> {
    pub fn new(count: impl Into<Value>, effects: Vec<E>) -> Self {
        Self {
            count: count.into(),
            effects,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeBidStart {
    Fixed(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BidLifeEffect<E> {
    pub target: ChooseSpec,
    pub starting_bid: LifeBidStart,
    pub winner_effects: Vec<E>,
}

impl<E> BidLifeEffect<E> {
    pub fn new(target: ChooseSpec, starting_bid: LifeBidStart, winner_effects: Vec<E>) -> Self {
        Self {
            target,
            starting_bid,
            winner_effects,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoseLifeEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl LoseLifeEffect {
    pub fn new(amount: Value, player: PlayerFilter) -> Self {
        Self { amount, player }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatProcessPromptKind {
    MayRepeatAnyNumberOfTimes,
}

impl RepeatProcessPromptKind {
    pub fn prompt_text(self) -> &'static str {
        match self {
            Self::MayRepeatAnyNumberOfTimes => "You may repeat this process any number of times",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessPromptEffect {
    pub kind: RepeatProcessPromptKind,
}

impl RepeatProcessPromptEffect {
    pub fn new(kind: RepeatProcessPromptKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoosePlayerEffect {
    pub chooser: PlayerFilter,
    pub filter: PlayerFilter,
    pub tag: crate::tag::TagKey,
    pub excluded_tags: Vec<crate::tag::TagKey>,
    pub random: bool,
    pub remember_as_chosen_player: bool,
}

impl ChoosePlayerEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
            excluded_tags: Vec::new(),
            random: false,
            remember_as_chosen_player: false,
        }
    }

    pub fn excluding_tags(mut self, tags: Vec<crate::tag::TagKey>) -> Self {
        self.excluded_tags = tags;
        self
    }

    pub fn at_random(mut self) -> Self {
        self.random = true;
        self
    }

    pub fn remember_as_chosen_player(mut self) -> Self {
        self.remember_as_chosen_player = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect<E> {
    pub effects: Vec<E>,
}

impl<E> SequenceEffect<E> {
    pub fn new(effects: Vec<E>) -> Self {
        Self { effects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaRestrictedEffect<E> {
    pub effects: Vec<E>,
    pub restrictions: Vec<crate::ManaUsageRestriction>,
}

impl<E> ManaRestrictedEffect<E> {
    pub fn new(effects: Vec<E>, restrictions: Vec<crate::ManaUsageRestriction>) -> Self {
        Self {
            effects,
            restrictions,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayEffect<E> {
    pub decider: Option<PlayerFilter>,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlessPaysEffect<E> {
    pub player: PlayerFilter,
    pub effects: Vec<E>,
    pub cost: crate::cost_model::TotalCost<crate::cost_model::Cost<E>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CumulativeUpkeepEffect<E> {
    pub player: PlayerFilter,
    pub payment: Vec<E>,
    pub failure: Vec<E>,
}

impl<E> CumulativeUpkeepEffect<E> {
    pub fn new(player: PlayerFilter, payment: Vec<E>, failure: Vec<E>) -> Self {
        Self {
            player,
            payment,
            failure,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlessActionEffect<E> {
    pub player: PlayerFilter,
    pub effects: Vec<E>,
    pub alternative: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect<E> {
    pub filter: PlayerFilter,
    pub effects: Vec<E>,
    pub starting_with_controller: bool,
    pub stop_after_first_happened: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachControllerOfTaggedEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedPlayerEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveTriggerEffect<E> {
    pub condition: EffectId,
    pub predicate: EffectPredicate,
    pub effects: Vec<E>,
    pub choices: Vec<ChooseSpec>,
}
