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

/// Adds exactly one mana whose color is chosen from the colors of objects
/// matching `filter`.
///
/// This is distinct from [`AddManaOfColorsAmongEffect`], which adds one mana
/// of *each* represented color.
#[derive(Debug, Clone, PartialEq)]
pub struct AddOneManaOfAnyColorAmongEffect {
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
    /// Preserve the authored two-step surface "Choose a color of ... Add one
    /// mana of that color" while executing the same restricted color choice.
    pub choose_color_of_object_surface: bool,
}

impl AddOneManaOfAnyColorAmongEffect {
    pub fn new(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            choose_color_of_object_surface: false,
        }
    }

    pub fn you(filter: ObjectFilter) -> Self {
        Self::new(filter, PlayerFilter::You)
    }

    pub fn with_choose_color_of_object_surface(mut self, enabled: bool) -> Self {
        self.choose_color_of_object_surface = enabled;
        self
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
            amount,
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

/// A fixed life payment made by a player while an effect resolves.
///
/// This is deliberately distinct from [`LoseLifeEffect`]: paying life is only
/// legal when the player has enough life and no rule or effect forbids the
/// payment, while ordinary life loss can reduce a player below zero.
#[derive(Debug, Clone, PartialEq)]
pub struct PayLifeEffect {
    pub amount: Value,
    pub player: ChooseSpec,
}

impl PayLifeEffect {
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
    pub creature_destroyed_this_way_surface: bool,
}

impl DestroyNoRegenerationEffect {
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            filter: Some(filter),
            target: None,
            creature_destroyed_this_way_surface: false,
        }
    }

    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            filter: None,
            target: Some(target),
            creature_destroyed_this_way_surface: false,
        }
    }

    pub fn with_creature_destroyed_this_way_surface(mut self, present: bool) -> Self {
        self.creature_destroyed_this_way_surface = present;
        self
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DamageDistributionMode {
    /// The casting player assigns the total among the announced targets.
    #[default]
    Chosen,
    /// Every announced target receives the same rounded-down share.
    EvenRoundedDown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DealDistributedDamageEffect {
    pub amount: Value,
    pub target: ChooseSpec,
    pub source: ChooseSpec,
    pub chooser: PlayerFilter,
    pub distribution: DamageDistributionMode,
}

impl DealDistributedDamageEffect {
    pub fn new(amount: Value, target: ChooseSpec) -> Self {
        Self {
            amount,
            target,
            source: ChooseSpec::Source,
            chooser: PlayerFilter::You,
            distribution: DamageDistributionMode::Chosen,
        }
    }

    pub fn with_source(mut self, source: ChooseSpec) -> Self {
        self.source = source;
        self
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    pub fn with_distribution(mut self, distribution: DamageDistributionMode) -> Self {
        self.distribution = distribution;
        self
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

/// How long an effect keeps a permanent phased out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhaseOutDuration {
    /// The normal phasing rule: phase in during its controller's next untap step.
    #[default]
    UntilNextUntap,
    /// Keep it phased out until the ability source leaves the battlefield.
    UntilSourceLeaves,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOutEffect {
    pub target: ChooseSpec,
    pub duration: PhaseOutDuration,
    /// Printed source wording for source-linked durations (for example,
    /// "this enchantment"). Runtime identity still comes from the effect source.
    pub source_surface: Option<crate::SourceReferenceSurface>,
}

impl PhaseOutEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            target,
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }

    pub fn until_source_leaves(mut self) -> Self {
        self.duration = PhaseOutDuration::UntilSourceLeaves;
        self
    }

    pub fn with_source_surface(mut self, surface: crate::SourceReferenceSurface) -> Self {
        self.source_surface = Some(surface);
        self
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

/// Oracle-facing actor placement for an exile-top-library instruction.
///
/// The library owner remains semantic in `player`; this only distinguishes
/// “Target opponent exiles … from their library” from the imperative
/// “Exile … from target opponent's library” surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileTopLibrarySurface {
    LibraryOwnerAsActor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub surface: Option<ExileTopLibrarySurface>,
    pub moved_tags: Vec<crate::tag::TagKey>,
    pub accumulated_tags: Vec<crate::tag::TagKey>,
    /// Whether the cards are exiled face down without being revealed.
    pub face_down: bool,
}

impl ExileTopOfLibraryEffect {
    pub fn new(count: Value, player: PlayerFilter) -> Self {
        Self {
            count,
            player,
            surface: None,
            moved_tags: Vec::new(),
            accumulated_tags: Vec::new(),
            face_down: false,
        }
    }

    pub fn tag_moved(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.moved_tags.push(tag.into());
        self
    }

    pub fn with_surface(mut self, surface: ExileTopLibrarySurface) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn append_tagged(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.accumulated_tags.push(tag.into());
        self
    }

    pub fn face_down(mut self) -> Self {
        self.face_down = true;
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

/// Register a one-shot replacement that performs typed effects instead of the
/// next damage event to the chosen target this turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceNextDamageToTargetEffect<E> {
    pub target: ChooseSpec,
    pub replacement_effects: Vec<E>,
}

impl<E> ReplaceNextDamageToTargetEffect<E> {
    pub fn new(target: ChooseSpec, replacement_effects: Vec<E>) -> Self {
        Self {
            target,
            replacement_effects,
        }
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
    /// Authored duration placement and tagged-card reference wording.
    /// Gameplay semantics remain in the ordinary typed grant fields.
    pub surface: Option<crate::GrantPlayTaggedSurface>,
    pub allow_land: bool,
    /// Semantic mana conversion used while casting the granted cards.
    pub mana_spend_mode: crate::value_model::ManaSpendMode,
    /// Compatibility flag for older render-pattern predicates. This is true
    /// for both flexible modes; new code must inspect `mana_spend_mode` when
    /// the distinction between color and type matters.
    pub allow_any_color_for_cast: bool,
    pub while_on_top_of_library: bool,
    pub filter: Option<ObjectFilter>,
    pub during_turns_counter_put_on_source: Option<CounterType>,
    /// Additional mana cost imposed on nonland cards cast through this exact
    /// tagged play permission.
    pub spell_cost_increase: Option<ManaCost>,
    /// Whether a land played through this exact tagged permission enters
    /// tapped.
    pub lands_enter_tapped: bool,
    /// True when the granted pool holds more than one card, selecting plural
    /// "cast spells from among those exiled cards" wording over the singular
    /// "cast that card this turn". Purely cosmetic; resolution is unaffected.
    pub cast_pool_is_plural: bool,
    /// Total number of plays shared by the tagged collection. `None` grants
    /// each tagged card independently; `Some(1)` models "play one of those
    /// cards" while deferring the choice until a card is actually played.
    pub max_plays: Option<u32>,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: crate::tag::TagKey,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
        allow_land: bool,
        mana_spend_mode: impl Into<crate::value_model::ManaSpendMode>,
    ) -> Self {
        let mana_spend_mode = mana_spend_mode.into();
        Self {
            tag,
            player,
            duration,
            surface: None,
            allow_land,
            mana_spend_mode,
            allow_any_color_for_cast: mana_spend_mode.allows_any_color(),
            while_on_top_of_library: false,
            filter: None,
            during_turns_counter_put_on_source: None,
            spell_cost_increase: None,
            lands_enter_tapped: false,
            cast_pool_is_plural: false,
            max_plays: None,
        }
    }

    pub fn cast_pool_is_plural(mut self, plural: bool) -> Self {
        self.cast_pool_is_plural = plural;
        self
    }

    pub fn with_max_plays(mut self, max_plays: Option<u32>) -> Self {
        self.max_plays = max_plays;
        self
    }

    pub fn with_surface(mut self, surface: crate::GrantPlayTaggedSurface) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn with_mana_spend_mode(mut self, mode: crate::value_model::ManaSpendMode) -> Self {
        self.mana_spend_mode = mode;
        self.allow_any_color_for_cast = mode.allows_any_color();
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

    pub fn during_turns_counter_put_on_source(mut self, counter_type: CounterType) -> Self {
        self.during_turns_counter_put_on_source = Some(counter_type);
        self
    }

    pub fn with_spell_cost_increase(mut self, cost: ManaCost) -> Self {
        self.spell_cost_increase = Some(cost);
        self
    }

    pub fn with_lands_enter_tapped(mut self, enabled: bool) -> Self {
        self.lands_enter_tapped = enabled;
        self
    }
}

/// Where a zone-change replacement puts an object when the replacement
/// destination is its owner's library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneReplacementLibraryPlacement {
    Top,
    Bottom,
    TopOrBottom,
}

/// A follow-up that is created only when a concrete zone replacement actually
/// exiles its watched object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedExileFollowUp {
    /// Return that exact exiled object to its owner's hand at the beginning of
    /// the next end step.
    ReturnToHandAtNextEndStep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub library_placement: Option<ZoneReplacementLibraryPlacement>,
    pub mode: ReplacementApplyMode,
    pub optional: bool,
    pub choice_description: Option<String>,
    pub counters: Vec<(CounterType, u32)>,
    pub linked_exile_follow_up: Option<LinkedExileFollowUp>,
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
            library_placement: None,
            mode,
            optional: false,
            choice_description: None,
            counters: Vec::new(),
            linked_exile_follow_up: None,
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

    pub fn with_library_placement(mut self, placement: ZoneReplacementLibraryPlacement) -> Self {
        self.library_placement = Some(placement);
        self
    }

    pub fn with_linked_exile_follow_up(mut self, follow_up: LinkedExileFollowUp) -> Self {
        self.linked_exile_follow_up = Some(follow_up);
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
    pub link_exiled_to_source: bool,
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

/// Registers a temporary replacement that makes matching permanents enter tapped.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterEnterTappedReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub mode: ReplacementApplyMode,
}

/// Registers a turn-scoped replacement for the next simultaneous batch in
/// which one or more matching permanents would enter. Every matching member
/// of that batch enters with the additional counters, then the replacement is
/// consumed. If no matching batch occurs, it expires during cleanup.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterNextBatchEnterWithCountersEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub counter_type: CounterType,
    pub count: Value,
}

impl RegisterNextBatchEnterWithCountersEffect {
    pub fn new(
        filter: crate::filter_model::ObjectFilter,
        counter_type: CounterType,
        count: Value,
    ) -> Self {
        Self {
            filter,
            counter_type,
            count,
        }
    }
}

impl RegisterEnterTappedReplacementEffect {
    pub fn new(filter: crate::filter_model::ObjectFilter, mode: ReplacementApplyMode) -> Self {
        Self { filter, mode }
    }
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
            link_exiled_to_source: false,
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

    pub fn linking_exiled_to_source(mut self) -> Self {
        self.link_exiled_to_source = true;
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
    pub family: crate::types::SubtypeFamily,
}

impl ChooseCreatureTypeEffect {
    pub fn new(chooser: PlayerFilter, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            chooser,
            excluded_subtypes,
            family: crate::types::SubtypeFamily::Creature,
        }
    }

    pub fn for_family(chooser: PlayerFilter, family: crate::types::SubtypeFamily) -> Self {
        Self {
            chooser,
            excluded_subtypes: Vec::new(),
            family,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoinFace {
    Heads,
    Tails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoinFlipKind {
    Called,
    FaceOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    pub player: PlayerFilter,
    pub kind: CoinFlipKind,
    pub forced_face: Option<CoinFace>,
    pub forced_winner: Option<PlayerFilter>,
    pub forced_loser: Option<PlayerFilter>,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            kind: CoinFlipKind::Called,
            forced_face: None,
            forced_winner: None,
            forced_loser: None,
        }
    }

    pub fn face_only(player: PlayerFilter) -> Self {
        Self {
            player,
            kind: CoinFlipKind::FaceOnly,
            forced_face: None,
            forced_winner: None,
            forced_loser: None,
        }
    }

    pub fn with_forced_face(mut self, face: CoinFace) -> Self {
        self.forced_face = Some(face);
        self
    }

    pub fn with_forced_winner(mut self, winner: PlayerFilter) -> Self {
        self.forced_winner = Some(winner);
        self
    }

    pub fn with_forced_loser(mut self, loser: PlayerFilter) -> Self {
        self.forced_loser = Some(loser);
        self
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

/// Ends the current combat phase using the ordered CR 724.2 procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EndCombatPhaseEffect;

impl EndCombatPhaseEffect {
    pub const fn new() -> Self {
        Self
    }
}

/// Restarts the game, optionally exempting a set of cards from the restart.
///
/// The exempt set remains in exile while every other physical card involved
/// in the game is returned to its appropriate new-game starting zone. The
/// effect reports the exempt cards as its affected objects so a following
/// tagged instruction can continue to act on them after the restart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RestartGameEffect {
    pub cards_left_in_exile: Option<ChooseSpec>,
    /// Oracle-facing source wording for an "exiled with ..." exemption.
    /// Runtime identity is carried by the tagged constraint in the choose
    /// spec; this field is presentation metadata only.
    pub source_surface: Option<SourceReferenceSurface>,
}

impl RestartGameEffect {
    pub fn new(cards_left_in_exile: Option<ChooseSpec>) -> Self {
        Self {
            cards_left_in_exile,
            source_surface: None,
        }
    }

    pub fn with_source_surface(mut self, source_surface: SourceReferenceSurface) -> Self {
        self.source_surface = Some(source_surface);
        self
    }
}

/// Reverses the game's normal turn order without changing the active player.
///
/// Turn advancement and priority ordering derive from the shared turn-order
/// sequence, so this single state transition applies to multiplayer games and
/// remains a no-op in practice for one- and two-player games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReverseTurnOrderEffect;

impl ReverseTurnOrderEffect {
    pub const fn new() -> Self {
        Self
    }
}

/// Suspends the current game and creates a completely isolated child game.
///
/// `nonwinner_effects` are continuation effects executed in the resumed parent
/// once for each participant who did not win the child game. They are part of
/// the creating instruction, rather than effects that exist inside the child.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlaySubgameEffect<E> {
    pub nonwinner_effects: Vec<E>,
}

impl<E> PlaySubgameEffect<E> {
    pub fn new(nonwinner_effects: Vec<E>) -> Self {
        Self { nonwinner_effects }
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

/// Where an effect gets the set of mana types a player may choose from.
///
/// `MatchingLandsCouldProduce` is prospective. `TriggeringEventProduced`
/// consumes the mana symbols captured by the event that caused the ability to
/// trigger instead of recalculating the source's current capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ManaTypeSource {
    #[default]
    MatchingLandsCouldProduce,
    TriggeringEventProduced,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub land_filter: ObjectFilter,
    pub allow_colorless: bool,
    pub same_type: bool,
    pub mana_type_source: ManaTypeSource,
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
            mana_type_source: ManaTypeSource::MatchingLandsCouldProduce,
        }
    }

    pub fn from_triggering_event(
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
            mana_type_source: ManaTypeSource::TriggeringEventProduced,
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

/// Unlock a locked door of a Room matching `room_filter` during resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct UnlockRoomDoorEffect {
    pub player: PlayerFilter,
    pub room_filter: ObjectFilter,
}

impl UnlockRoomDoorEffect {
    pub fn new(player: PlayerFilter, room_filter: ObjectFilter) -> Self {
        Self {
            player,
            room_filter,
        }
    }
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

/// Tag the attacking creature from the block event that caused a trigger.
///
/// This is the attacking-participant counterpart to
/// [`TagTriggeringBlockersEffect`]. It preserves the exact event identity even
/// when combat state changes before the triggered ability resolves.
#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringAttackerEffect {
    pub tag: TagKey,
    pub filter: Option<ObjectFilter>,
}

impl TagTriggeringAttackerEffect {
    pub fn new(tag: impl Into<TagKey>, filter: Option<ObjectFilter>) -> Self {
        Self {
            tag: tag.into(),
            filter,
        }
    }
}

/// Tag the combat participant on the other side of the source's block event.
///
/// When the source is blocking, this tags the attacker. When the source is
/// attacking, this tags the blocker or blockers. This gives an `Either`
/// trigger such as "this creature blocks or becomes blocked" one stable
/// referent for its shared "that creature" body.
#[derive(Debug, Clone, PartialEq)]
pub struct TagOtherBlockParticipantEffect {
    pub tag: TagKey,
    pub filter: Option<ObjectFilter>,
}

impl TagOtherBlockParticipantEffect {
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

/// An effect that states that the game is a draw.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrawTheGameEffect;

#[derive(Debug, Clone, PartialEq)]
pub struct CastSourceEffect {
    pub without_paying_mana_cost: bool,
    pub require_exile: bool,
    pub cast_as_suspend: bool,
    /// Cast the source's linked transform-like face instead of its current face.
    pub cast_other_face: bool,
}

impl CastSourceEffect {
    pub fn new() -> Self {
        Self {
            without_paying_mana_cost: false,
            require_exile: false,
            cast_as_suspend: false,
            cast_other_face: false,
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

    pub fn other_face(mut self) -> Self {
        self.cast_other_face = true;
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

/// Makes the chosen object assign no combat damage for the specified duration.
///
/// This is an assignment rule, not prevention: no damage event is created, so
/// effects that make damage unpreventable do not override it.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignNoCombatDamageEffect {
    pub source: ChooseSpec,
    pub until: Until,
}

impl AssignNoCombatDamageEffect {
    pub fn new(source: ChooseSpec, until: Until) -> Self {
        Self { source, until }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageEffect {
    pub target: CombatDamagePreventionTarget,
    pub until: Until,
    /// Preserve the active-voice oracle surface, "<sources> would deal".
    pub source_would_deal_surface: bool,
}

impl PreventAllCombatDamageEffect {
    pub fn new(target: CombatDamagePreventionTarget, until: Until) -> Self {
        Self {
            target,
            until,
            source_would_deal_surface: false,
        }
    }

    pub fn with_source_would_deal_surface(mut self) -> Self {
        self.source_would_deal_surface = true;
        self
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
    /// Protects the controller and permanents matching a filter.
    YouAndPermanentsMatching(ObjectFilter),
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
    /// Do not prevent damage from this independently chosen source.
    pub excluded_specific_source: Option<crate::ids::ObjectId>,
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
        if self.excluded_specific_source == Some(source) {
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
    /// Whether the chosen source must share a color with mana spent on the
    /// activation that created this prevention effect.
    pub source_choice_shares_activation_mana_color: bool,
    /// An independently targeted source whose damage this shield applies to.
    pub source_target: Option<ChooseSpec>,
    /// An independently targeted source whose damage this shield excludes.
    pub excluded_source_target: Option<ChooseSpec>,
    /// Protect the resolving ability's source object.
    pub protect_source: bool,
    pub until: Until,
}

impl PreventAllDamageEffect {
    /// Create a new prevent-all-damage effect.
    pub fn new(target: PreventionTarget, damage_filter: DamageFilter, until: Until) -> Self {
        Self {
            target,
            damage_filter,
            source_of_your_choice: false,
            source_choice_shares_activation_mana_color: false,
            source_target: None,
            excluded_source_target: None,
            protect_source: false,
            until,
        }
    }

    /// Restrict this prevention shield to a source chosen as the effect resolves.
    pub fn with_source_of_your_choice(mut self) -> Self {
        self.source_of_your_choice = true;
        self
    }

    /// Restrict the chosen source to colors represented in this activation's
    /// mana payment.
    pub fn with_source_choice_sharing_activation_mana_color(mut self) -> Self {
        self.source_of_your_choice = true;
        self.source_choice_shares_activation_mana_color = true;
        self
    }

    pub fn with_target_source(mut self, source: ChooseSpec) -> Self {
        self.source_target = Some(source);
        self
    }

    pub fn excluding_target_source(mut self, source: ChooseSpec) -> Self {
        self.excluded_source_target = Some(source);
        self
    }

    pub fn protecting_source(mut self) -> Self {
        self.protect_source = true;
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
    /// The printed destination explicitly names the affected objects' owners'
    /// libraries (for example, "into its owner's library").
    ///
    /// `player` still carries the executable owner relation. This flag keeps
    /// the destination surface distinct from clauses whose grammatical subject
    /// is the owner, such as "Its owner shuffles it into their library."
    pub owner_library_destination: bool,
    /// Preserve a possessive grammatical subject such as "Target creature's
    /// owner" instead of the equivalent "The owner of target creature."
    pub possessive_owner_subject: bool,
}

impl ShuffleObjectsIntoLibraryEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter) -> Self {
        Self {
            target,
            player,
            owner_library_destination: false,
            possessive_owner_subject: false,
        }
    }

    pub fn with_owner_library_destination(mut self) -> Self {
        self.owner_library_destination = true;
        self
    }

    pub fn with_possessive_owner_subject(mut self) -> Self {
        self.possessive_owner_subject = true;
        self
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

#[derive(Debug, Clone, PartialEq, Default)]
pub enum RestrictionStart {
    #[default]
    Immediate,
    NextTurn(PlayerFilter),
}

/// Authored placement of a temporary restriction's duration.
///
/// This affects compiled text only; runtime expiration is still governed by
/// `CantEffect::duration`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RestrictionDurationSurface {
    #[default]
    Default,
    LeadingUntilEndOfTurn,
    LeadingUntilYourNextTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
    pub start: RestrictionStart,
    pub duration_surface: RestrictionDurationSurface,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
            start: RestrictionStart::Immediate,
            duration_surface: RestrictionDurationSurface::Default,
        }
    }

    pub fn starting(restriction: Restriction, duration: Until, start: RestrictionStart) -> Self {
        Self {
            restriction,
            duration,
            start,
            duration_surface: RestrictionDurationSurface::Default,
        }
    }

    pub fn with_duration_surface(mut self, surface: RestrictionDurationSurface) -> Self {
        self.duration_surface = surface;
        self
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }

    pub fn during_next_turn(restriction: Restriction, player: PlayerFilter) -> Self {
        Self::starting(
            restriction,
            Until::EndOfTurn,
            RestrictionStart::NextTurn(player),
        )
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
    /// Whether removing fewer than the available maximum is allowed.
    pub up_to: bool,
}

impl RemoveUpToAnyCountersEffect {
    pub fn new(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            max_count: max_count.into(),
            target,
            up_to: true,
        }
    }

    pub fn exact(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            max_count: max_count.into(),
            target,
            up_to: false,
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
    /// Choose a legal destination separately for each object being attached.
    ///
    /// This models plural-destination instructions such as "return any
    /// number of Aura cards ... attached to creatures you control." A single
    /// destination remains the default for ordinary attach instructions.
    pub individual_targets: bool,
}

impl AttachObjectsEffect {
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self {
            objects,
            target,
            individual_targets: false,
        }
    }

    pub fn with_individual_targets(mut self) -> Self {
        self.individual_targets = true;
        self
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
    /// Counters that are part of this object's battlefield-entry event.
    pub enters_with_counters: Vec<BattlefieldEntryCounterSpec>,
}

/// Return the resolving ability's source from either its owner's graveyard or
/// exile to the battlefield. This is intentionally source-only: the two-zone
/// origin is what makes the ability functional outside the battlefield.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardOrExileToBattlefieldEffect {
    pub tapped: bool,
}

impl ReturnFromGraveyardOrExileToBattlefieldEffect {
    pub fn new(tapped: bool) -> Self {
        Self { tapped }
    }
}

impl ReturnFromGraveyardToBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool) -> Self {
        Self {
            target,
            tapped,
            as_aura: None,
            enters_with_counters: Vec::new(),
        }
    }

    pub fn with_entry_counter(mut self, counter: BattlefieldEntryCounterSpec) -> Self {
        self.enters_with_counters.push(counter);
        self
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
    /// Player explicitly presented as performing the return action. Runtime
    /// movement remains object-based; this preserves causative clauses such
    /// as "you may have that player return ...".
    pub actor_surface: Option<PlayerFilter>,
    /// Contextual graveyard owner named by the oracle origin. Runtime movement
    /// continues to use object ownership; this is presentation-only.
    pub graveyard_player_surface: Option<PlayerFilter>,
    /// Contextual player named by the oracle destination. Runtime movement
    /// remains owner-based; this field is presentation-only.
    pub destination_player_surface: Option<PlayerFilter>,
}

impl ReturnFromGraveyardToHandEffect {
    pub fn new(target: ChooseSpec, random: bool) -> Self {
        Self {
            target,
            random,
            actor_surface: None,
            graveyard_player_surface: None,
            destination_player_surface: None,
        }
    }

    pub fn with_actor_surface(mut self, player: PlayerFilter) -> Self {
        self.actor_surface = Some(player);
        self
    }

    pub fn with_graveyard_player_surface(mut self, player: PlayerFilter) -> Self {
        self.graveyard_player_surface = Some(player);
        self
    }

    pub fn with_destination_player_surface(mut self, player: PlayerFilter) -> Self {
        self.destination_player_surface = Some(player);
        self
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
    /// Counters that are part of this object's battlefield-entry event.
    pub enters_with_counters: Vec<BattlefieldEntryCounterSpec>,
}

impl PutOntoBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool, controller: PlayerFilter) -> Self {
        Self {
            target,
            tapped,
            controller,
            enters_with_counters: Vec::new(),
        }
    }

    pub fn with_entry_counter(mut self, counter: BattlefieldEntryCounterSpec) -> Self {
        self.enters_with_counters.push(counter);
        self
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

/// Look at the top `count` cards of a planar deck and choose exactly one of
/// them to move to that planar deck's bottom. The other cards keep their
/// relative order on top.
#[derive(Debug, Clone, PartialEq)]
pub struct ReorderTopPlanarDeckEffect {
    pub player: PlayerFilter,
    pub chooser: PlayerFilter,
    pub count: u32,
}

impl ReorderTopPlanarDeckEffect {
    pub fn new(player: PlayerFilter, chooser: PlayerFilter, count: u32) -> Self {
        Self {
            player,
            chooser,
            count,
        }
    }

    pub fn you(count: u32) -> Self {
        Self::new(PlayerFilter::You, PlayerFilter::You, count)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyInstructionSurface {
    /// `Copy it. You may cast the copy.`
    SeparateIt,
    /// `Copy that card. You may cast the copy.`
    SeparateThatCard,
    /// `Copy it, then you may cast the copy.`
    SeparateItThen,
    /// `Copy it, then you may cast the copy. (A copy of a permanent spell
    /// becomes a token.)`
    SeparateItThenPermanentCopyReminder,
}

#[derive(Debug, Clone)]
pub struct CastTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub allow_land: bool,
    pub as_copy: bool,
    /// The authored cast-copy instruction carried the standard reminder that
    /// ordinary costs are still paid and permanent-spell copies become
    /// tokens. This is presentation metadata; `as_copy` owns the executable
    /// semantics.
    pub copy_cast_reminder_surface: bool,
    /// The authored copy instruction was separate from the exile action.
    /// This is presentation-only; `as_copy` owns the executable semantics.
    pub copy_instruction_surface: Option<CopyInstructionSurface>,
    pub without_paying_mana_cost: bool,
    /// A mandatory mana cost imposed by the resolving instruction in
    /// addition to the spell's ordinary costs.
    pub additional_mana_cost: Option<ManaCost>,
    pub cost_reduction: Option<ManaCost>,
    pub mana_spend_mode: crate::value_model::ManaSpendMode,
}

impl PartialEq for CastTaggedEffect {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
            && self.player == other.player
            && self.allow_land == other.allow_land
            && self.as_copy == other.as_copy
            && self.without_paying_mana_cost == other.without_paying_mana_cost
            && self.additional_mana_cost == other.additional_mana_cost
            && self.cost_reduction == other.cost_reduction
            && self.mana_spend_mode == other.mana_spend_mode
    }
}

impl CastTaggedEffect {
    pub fn new(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
            allow_land: false,
            as_copy: false,
            copy_cast_reminder_surface: false,
            copy_instruction_surface: None,
            without_paying_mana_cost: false,
            additional_mana_cost: None,
            cost_reduction: None,
            mana_spend_mode: crate::value_model::ManaSpendMode::Normal,
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

    pub fn with_copy_cast_reminder_surface(mut self) -> Self {
        self.copy_cast_reminder_surface = true;
        self
    }

    pub fn with_separate_copy_instruction_surface(mut self) -> Self {
        self.copy_instruction_surface = Some(CopyInstructionSurface::SeparateThatCard);
        self
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn additional_mana_cost(mut self, cost: ManaCost) -> Self {
        self.additional_mana_cost = Some(cost);
        self
    }

    pub fn cost_reduction(mut self, reduction: ManaCost) -> Self {
        self.cost_reduction = Some(reduction);
        self
    }

    pub fn mana_spend_mode(mut self, mode: crate::value_model::ManaSpendMode) -> Self {
        self.mana_spend_mode = mode;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryBottomOrder {
    Random,
    ChooserChooses,
}

/// Authored surface for disposing of the exact complement of a tagged
/// looked/revealed collection. Execution is identical for both variants, but
/// retaining the surface lets compiled text distinguish a terse "the rest"
/// instruction from an explicit revealed-card complement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryRemainderSurface {
    Rest,
    /// Authored bare "the rest" ("then put the rest on the bottom of your
    /// library in a random order").
    RestBare,
    /// A new authored sentence beginning "Then put the rest ...".
    SentenceLeadingThenRest,
    /// "the rest of the cards revealed this way"
    RestOfCardsRevealedThisWay,
    /// "the cards you revealed this way"
    CardsYouRevealedThisWay,
    RevealedCardsNotPutOntoBattlefield,
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
    pub surface: LibraryRemainderSurface,
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
            surface: LibraryRemainderSurface::Rest,
        }
    }

    pub fn with_surface(mut self, surface: LibraryRemainderSurface) -> Self {
        self.surface = surface;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersAmongEffect {
    pub count: u32,
    pub min_count: u32,
    pub dynamic_count: bool,
    pub display_x: bool,
    /// All counters paid by this effect must come from one chosen object.
    ///
    /// This preserves the distinction between "from a permanent" and
    /// "from among permanents", which have different payment semantics.
    pub single_object: bool,
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
            single_object: false,
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
            single_object: false,
            filter,
            counter_type: None,
        }
    }

    pub fn with_counter_type(mut self, counter_type: Option<crate::counter::CounterType>) -> Self {
        self.counter_type = counter_type;
        self
    }

    pub fn from_single_object(mut self) -> Self {
        self.single_object = true;
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
    /// Preserve oracle's singular "a new target" surface when the copied
    /// spell has one target. Runtime legality still comes from the copied
    /// stack object's target requirements.
    pub single_target_surface: bool,
}

impl ChooseNewTargetsEffect {
    pub fn new(from_effect: EffectId, may: bool) -> Self {
        Self {
            from_effect,
            may,
            chooser: None,
            single_target_surface: false,
        }
    }

    pub fn new_for_player(from_effect: EffectId, may: bool, chooser: PlayerFilter) -> Self {
        Self {
            from_effect,
            may,
            chooser: Some(chooser),
            single_target_surface: false,
        }
    }

    pub fn with_single_target_surface(mut self) -> Self {
        self.single_target_surface = true;
        self
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

/// Grants a repeatable mana-payment special action through end of turn.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<E> {
    pub player: PlayerFilter,
    pub cost: ManaCost,
    pub effects: Vec<E>,
}

impl<E> GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<E> {
    pub fn new(player: PlayerFilter, cost: ManaCost, effects: Vec<E>) -> Self {
        Self {
            player,
            cost,
            effects,
        }
    }
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

/// Printed relationship among the child effects of a sequence.
///
/// All variants execute in order. `Coordinated` records that the children
/// came from one Oracle clause joined by "and" (rather than from successive
/// sentences), allowing typed renderers to preserve the shared verb/subject
/// without guessing from adjacent runtime effects. `ResultConjunction` is the
/// narrower grammar-confirmed form created for an explicit "If/When you do"
/// result body; it lets that wrapper restore the source conjunction without
/// overriding older coordinated specialist surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SequenceSurface {
    #[default]
    Sequential,
    /// The sequence begins an authored source sentence introduced by
    /// "Then". It executes like an ordinary sequential sequence; the
    /// distinction exists only so compiled text can preserve that explicit
    /// ordering connective.
    SentenceLeadingThen,
    /// The sequence contains an authored same-sentence `, then` boundary.
    /// It executes in ordinary sequential scope; the distinction exists so
    /// compiled text does not normalize the connective to `and`.
    CommaThen,
    /// Every boundary in a three-or-more-action sequence was authored as
    /// `, then`. This is distinct from `CommaThen`, which can also represent
    /// a single trailing `, then` after an ordinary comma-separated list.
    RepeatedCommaThen,
    Coordinated,
    CoordinatedLeadingDuration,
    ResultConjunction {
        leading_duration: bool,
    },
}

impl SequenceSurface {
    /// Whether the sequence's children share one coordinated target scope.
    ///
    /// Sentence-leading and same-sentence "then" surfaces remain ordinary
    /// sequential scopes even though they carry compiled-text provenance.
    pub const fn is_coordinated(self) -> bool {
        matches!(
            self,
            Self::Coordinated | Self::CoordinatedLeadingDuration | Self::ResultConjunction { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect<E> {
    pub effects: Vec<E>,
    pub surface: SequenceSurface,
    /// Optional authored label on a numeric result-table row.
    pub result_label: Option<String>,
}

impl<E> SequenceEffect<E> {
    pub fn new(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::Sequential,
            result_label: None,
        }
    }

    pub fn sentence_leading_then(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::SentenceLeadingThen,
            result_label: None,
        }
    }

    pub fn comma_then(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::CommaThen,
            result_label: None,
        }
    }

    pub fn repeated_comma_then(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::RepeatedCommaThen,
            result_label: None,
        }
    }

    pub fn coordinated(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::Coordinated,
            result_label: None,
        }
    }

    pub fn coordinated_with_leading_duration(effects: Vec<E>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::CoordinatedLeadingDuration,
            result_label: None,
        }
    }

    pub fn result_conjunction(effects: Vec<E>, leading_duration: bool) -> Self {
        Self {
            effects,
            surface: SequenceSurface::ResultConjunction { leading_duration },
            result_label: None,
        }
    }

    pub fn result_labeled(effects: Vec<E>, label: impl Into<String>) -> Self {
        Self {
            effects,
            surface: SequenceSurface::Sequential,
            result_label: Some(label.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaRestrictedEffect<E> {
    pub effects: Vec<E>,
    pub restrictions: Vec<crate::ManaUsageRestriction<E>>,
}

impl<E> ManaRestrictedEffect<E> {
    pub fn new(effects: Vec<E>, restrictions: Vec<crate::ManaUsageRestriction<E>>) -> Self {
        Self {
            effects,
            restrictions,
        }
    }
}

/// How long mana produced by a wrapped effect is retained in its owner's mana
/// pool as steps and phases end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManaRetentionDuration {
    EndOfCombat,
    EndOfTurn,
}

/// Runs child effects while marking every mana unit they produce with a
/// retention duration.
///
/// Retention is carried by the individual mana units rather than by a player
/// or color, so unrelated mana in the same pool still empties normally.
#[derive(Debug, Clone, PartialEq)]
pub struct ManaRetainedEffect<E> {
    pub effects: Vec<E>,
    pub duration: ManaRetentionDuration,
}

impl<E> ManaRetainedEffect<E> {
    pub fn new(effects: Vec<E>, duration: ManaRetentionDuration) -> Self {
        Self { effects, duration }
    }

    pub fn until_end_of_combat(effects: Vec<E>) -> Self {
        Self::new(effects, ManaRetentionDuration::EndOfCombat)
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
    /// Preserve an authored leading surface such as
    /// "unless you sacrifice ..., sacrifice this creature" instead of the
    /// ordinary trailing "sacrifice this creature unless ..." surface.
    pub leading_surface: bool,
    /// The payment is available before a surrounding delayed step rather than
    /// when this wrapper would resolve.
    pub before_delayed_step: bool,
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
    /// When present, bind the iterated player to the controller recorded by
    /// the latest block event in which the iterated creature was blocked by
    /// an object from this tagged set. This is distinct from the controller
    /// captured when the outer effect later tagged the creature.
    pub controller_at_last_blocked_by: Option<crate::tag::TagKey>,
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
