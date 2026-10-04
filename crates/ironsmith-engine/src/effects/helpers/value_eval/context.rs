use super::*;
use crate::object::Object;
use crate::target::ObjectFilter;

pub(crate) struct EvaluationContext<'a, 'game> {
    pub game: &'a GameState,
    pub source: ObjectId,
    pub controller: PlayerId,
    mode: Mode<'a, 'game>,
}

enum Mode<'a, 'game> {
    Execution(&'a ExecutionContext<'game>),
    Continuous(LayerValueContext<'a, 'game>),
}

#[derive(Clone, Copy)]
pub(crate) enum NumericProperty {
    Power,
    Toughness,
    ManaValue,
    ManaSpent,
    ColorCount,
    KickerCount,
    BasePower,
}
#[derive(Clone, Copy)]
pub(super) enum Reduction {
    Sum,
    Min,
    Max,
}

impl<'a, 'game> EvaluationContext<'a, 'game> {
    pub(crate) fn execution_context(game: &'a GameState, ctx: &'a ExecutionContext<'game>) -> Self {
        Self {
            game,
            source: ctx.source,
            controller: ctx.controller,
            mode: Mode::Execution(ctx),
        }
    }
    pub(crate) fn continuous(layer: LayerValueContext<'a, 'game>) -> Self {
        Self {
            game: layer.calculation.game,
            source: layer.source,
            controller: layer.controller,
            mode: Mode::Continuous(layer),
        }
    }
    pub(super) fn execution(&self) -> Option<&'a ExecutionContext<'game>> {
        match self.mode {
            Mode::Execution(ctx) => Some(ctx),
            _ => None,
        }
    }
    pub(super) fn layer(&self) -> LayerValueContext<'a, 'game> {
        match self.mode {
            Mode::Continuous(layer) => layer,
            _ => unreachable!("execution context has no layer view"),
        }
    }
    pub(super) fn require_execution(
        &self,
        value: &Value,
        reason: &str,
    ) -> &'a ExecutionContext<'game> {
        self.execution()
            .unwrap_or_else(|| self.layer().unsupported(value, reason))
    }
    pub(super) fn filter_context(&self, game: &GameState) -> FilterContext {
        match self.mode {
            Mode::Execution(ctx) => ctx.filter_context(game),
            Mode::Continuous(layer) => layer.filter_context(),
        }
    }
    pub(super) fn x(&self) -> Result<i64, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => ctx
                .x_value
                .map(i64::from)
                .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".into())),
            Mode::Continuous(_) => Ok(0),
        }
    }
    pub(super) fn division_by_zero<T>(&self, inner: &Value) -> Result<T, ExecutionError> {
        match self.mode {
            Mode::Execution(_) => Err(ExecutionError::UnresolvableValue(
                "division by zero in dynamic value".into(),
            )),
            Mode::Continuous(layer) => layer.unsupported(inner, "division by zero"),
        }
    }
    pub(super) fn optional_costs_paid(&self, value: &Value) -> &OptionalCostsPaid {
        match self.mode {
            Mode::Execution(ctx) => get_optional_costs_paid(self.game, ctx),
            Mode::Continuous(layer) => {
                &self
                    .game
                    .object(self.source)
                    .unwrap_or_else(|| layer.unsupported(value, "source object is unavailable"))
                    .optional_costs_paid
            }
        }
    }
    pub(super) fn single_player(
        &self,
        value: &Value,
        filter: &PlayerFilter,
    ) -> Result<&crate::player::Player, ExecutionError> {
        let id = match self.mode {
            Mode::Execution(ctx) => resolve_player_filter(self.game, filter, ctx)?,
            Mode::Continuous(layer) => layer.single_player(value, filter),
        };
        self.game
            .player(id)
            .ok_or(ExecutionError::PlayerNotFound(id))
    }
    pub(super) fn player_ids(
        &self,
        value: &Value,
        filter: &PlayerFilter,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => resolve_player_filter_to_list(
                self.game,
                filter,
                &ctx.filter_context(self.game),
                ctx,
            ),
            Mode::Continuous(layer) => Ok(layer.players(value, filter)),
        }
    }
    pub(super) fn aggregate_player_ids(
        &self,
        value: &Value,
        filter: &PlayerFilter,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => match filter {
                // Aggregates use actual multiplayer relations, not the older
                // list adapter's "everyone other than you" opponent shortcut.
                PlayerFilter::Opponent | PlayerFilter::Teammate => {
                    let filter_ctx = ctx.filter_context(self.game);
                    Ok(self
                        .game
                        .players
                        .iter()
                        .filter(|player| {
                            player.is_in_game()
                                && crate::filter::player_filter_matches_game(
                                    filter,
                                    player.id,
                                    self.game,
                                    &filter_ctx,
                                )
                        })
                        .map(|player| player.id)
                        .collect())
                }
                PlayerFilter::Excluding { base, excluded } => {
                    let mut players = self.aggregate_player_ids(value, base)?;
                    let excluded = self.aggregate_player_ids(value, excluded)?;
                    players.retain(|player| !excluded.contains(player));
                    Ok(players)
                }
                _ => self.player_ids(value, filter),
            },
            Mode::Continuous(layer) => Ok(layer.aggregate_players(filter)),
        }
    }
    pub(super) fn counter_player_ids(
        &self,
        value: &Value,
        filter: &PlayerFilter,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        match self.mode {
            Mode::Execution(_) => self.player_ids(value, filter),
            Mode::Continuous(layer) => Ok(layer.filtered_players(filter)),
        }
    }

    pub(super) fn count_objects(&self, filter: &ObjectFilter, allow_prevented_amount: bool) -> i32 {
        let Some(ctx) = self.execution() else {
            return self.layer().count(filter);
        };
        if allow_prevented_amount
            && filter.prior_effect_action_surface()
                == Some(crate::effect::PriorEffectAction::Prevented)
            && let Some(prevented) = ctx.event_value_amount
        {
            return prevented.max(0);
        }
        let filter_ctx = ctx.filter_context(self.game);
        // A count tied specifically to a departed source uses the source's
        // final attachment set, including Auras that have since left play.
        // Unions retain this meaning only when every arm has that relation.
        fn requires_source_attachment(filter: &ObjectFilter) -> bool {
            filter
                .attached_to_object
                .as_ref()
                .is_some_and(|host| host.source)
                || (!filter.any_of.is_empty()
                    && filter.any_of.iter().all(requires_source_attachment))
        }
        if requires_source_attachment(filter)
            && let Some(snapshot) = ctx.source_snapshot.as_ref()
            && self.game.object(ctx.source).is_none_or(|object| {
                object.id != snapshot.object_id || object.zone != snapshot.zone
            })
        {
            return snapshot
                .attachment_snapshots
                .iter()
                .filter(|attachment| filter.matches_snapshot(attachment, &filter_ctx, self.game))
                .count() as i32;
        }
        if let Some(snapshots) = value_tagged_snapshots_for_filter(filter, ctx) {
            let count = snapshots
                .iter()
                .filter(|snapshot| {
                    value_tagged_snapshot_matches_filter(self.game, filter, &filter_ctx, snapshot)
                })
                .count() as i32;
            if count == 0
                && let Some(count) = source_exiled_link_count(self.game, filter, ctx, &filter_ctx)
            {
                return count;
            }
            return count;
        }
        (value_candidate_ids_for_filter(self.game, filter, ctx)
            .iter()
            .filter_map(|id| self.game.object(*id))
            .filter(|object| filter.matches(object, &filter_ctx, self.game))
            .count()
            + count_as_card_named_for_spell_effect_bonus(self.game, filter, ctx, &filter_ctx))
            as i32
    }

    /// Size of the largest group of matching objects sharing one name.
    pub(super) fn greatest_shared_name_count(&self, filter: &ObjectFilter) -> i32 {
        let filter_ctx = self.filter_context(self.game);
        let mut names: Vec<String> = Vec::new();
        match self.mode {
            Mode::Execution(ctx) => {
                for id in value_candidate_ids_for_filter(self.game, filter, ctx) {
                    if let Some(object) = self.game.object(id)
                        && filter.matches(object, &filter_ctx, self.game)
                    {
                        names.push(object.name.to_string());
                    }
                }
            }
            Mode::Continuous(layer) => {
                layer.visit_layered(filter, |object, _| names.push(object.name.to_string()));
            }
        }
        let mut counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        for name in names {
            *counts.entry(name).or_default() += 1;
        }
        counts.into_values().max().unwrap_or(0)
    }

    pub(super) fn greatest_per_controller(
        &self,
        filter: &ObjectFilter,
        shared_creature_types: bool,
    ) -> i32 {
        let filter_ctx = self.filter_context(self.game);
        let count = |filter: &ObjectFilter| match self.mode {
            Mode::Execution(ctx) if shared_creature_types => {
                greatest_shared_creature_type_count_for_filter(self.game, filter, ctx, &filter_ctx)
            }
            Mode::Execution(ctx) => value_candidate_ids_for_filter(self.game, filter, ctx)
                .iter()
                .filter_map(|id| self.game.object(*id))
                .filter(|object| filter.matches(object, &filter_ctx, self.game))
                .count() as i32,
            Mode::Continuous(layer) if shared_creature_types => layer.shared_creature_count(filter),
            Mode::Continuous(layer) => layer.aggregate_count(filter),
        };
        let Some(controller) = &filter.controller else {
            return count(filter);
        };
        self.game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game() && controller.matches_player(player.id, &filter_ctx)
            })
            .map(|player| {
                let mut filter = filter.clone();
                filter.controller = Some(PlayerFilter::Specific(player.id));
                count(&filter)
            })
            .fold(0, i32::max)
    }

    /// Aggregation owns arithmetic. The visitors only choose the authoritative
    /// object state: retained execution snapshots or in-progress layer values.
    pub(super) fn aggregate(
        &self,
        filter: &ObjectFilter,
        property: NumericProperty,
        reduction: Reduction,
    ) -> i64 {
        let mut result: Option<i64> = None;
        let mut visit = |number: Option<i32>| {
            let number = match reduction {
                Reduction::Sum => number.unwrap_or(0),
                Reduction::Min if matches!(property, NumericProperty::ManaValue) => {
                    number.unwrap_or(0)
                }
                _ => match number {
                    Some(number) => number,
                    None => return,
                },
            };
            let number = i64::from(number);
            result = Some(match (result, reduction) {
                (Some(total), Reduction::Sum) => total + number,
                (Some(min), Reduction::Min) => min.min(number),
                (Some(max), Reduction::Max) => max.max(number),
                (None, _) => number,
            });
        };
        match self.mode {
            Mode::Execution(ctx) => {
                let filter_ctx = ctx.filter_context(self.game);
                if let Some(snapshots) = value_tagged_snapshots_for_filter(filter, ctx) {
                    for snapshot in snapshots.iter().filter(|snapshot| {
                        filter.matches_snapshot(snapshot, &filter_ctx, self.game)
                    }) {
                        visit(match property {
                            NumericProperty::Power => snapshot.power,
                            NumericProperty::BasePower => snapshot.base_power,
                            NumericProperty::Toughness => snapshot.toughness,
                            NumericProperty::ManaValue => {
                                NumericProperty::ManaValue.snapshot(snapshot)
                            }
                            NumericProperty::ManaSpent => {
                                Some(snapshot.mana_spent_to_cast.total() as i32)
                            }
                            NumericProperty::KickerCount => {
                                checked_kicker_count(&snapshot.optional_costs_paid)
                            }
                            NumericProperty::ColorCount => Some(snapshot.colors.count() as i32),
                        });
                    }
                } else {
                    for id in value_candidate_ids_for_filter(self.game, filter, ctx) {
                        let Some(object) = self.game.object(id) else {
                            continue;
                        };
                        if !filter.matches(object, &filter_ctx, self.game) {
                            continue;
                        }
                        visit(match property {
                            NumericProperty::BasePower => {
                                NumericProperty::BasePower.live(self.game, object)
                            }
                            NumericProperty::Power => {
                                self.game.calculated_power(id).or_else(|| object.power())
                            }
                            NumericProperty::Toughness => self
                                .game
                                .calculated_toughness(id)
                                .or_else(|| object.toughness()),
                            NumericProperty::ManaValue => NumericProperty::ManaValue.raw(object),
                            NumericProperty::ManaSpent => {
                                Some(object.mana_spent_to_cast.total() as i32)
                            }
                            NumericProperty::KickerCount => {
                                checked_kicker_count(&object.optional_costs_paid)
                            }
                            NumericProperty::ColorCount => Some(object.colors().count() as i32),
                        });
                    }
                }
            }
            Mode::Continuous(layer) => layer.visit_layered(filter, |object, chars| {
                visit(match property {
                    NumericProperty::Power => chars.power,
                    NumericProperty::BasePower => chars.base_power,
                    NumericProperty::Toughness => chars.toughness,
                    // Absent mana costs count as zero; melded permanents and
                    // transformed back faces use their front faces' mana value
                    // (CR 712.8e, 712.8g).
                    NumericProperty::ManaValue => {
                        Some(crate::filter::object_mana_value_for_filter(object))
                    }
                    NumericProperty::ManaSpent => Some(object.mana_spent_to_cast.total() as i32),
                    NumericProperty::KickerCount => {
                        checked_kicker_count(&object.optional_costs_paid)
                    }
                    NumericProperty::ColorCount => Some(chars.colors.count() as i32),
                })
            }),
        }
        result.unwrap_or(0)
    }

    pub(super) fn visit_current_subtypes(
        &self,
        filter: &ObjectFilter,
        mut visit: impl FnMut(&[Subtype]),
    ) {
        match self.mode {
            Mode::Execution(_) => self.visit_property_objects(filter, |object| {
                visit(&object.subtypes(self.game, true));
            }),
            // Domain must see type-changing effects from layer 4 when it is
            // evaluated for power/toughness in layer 7, including batch calculations.
            Mode::Continuous(layer) => layer.visit_layered(filter, |_, chars| {
                visit(&chars.subtypes);
            }),
        }
    }

    pub(super) fn visit_property_objects(
        &self,
        filter: &ObjectFilter,
        mut visit: impl FnMut(PropertyObject<'_>),
    ) {
        match self.mode {
            Mode::Execution(ctx) => {
                let filter_ctx = ctx.filter_context(self.game);
                if let Some(snapshots) = value_tagged_snapshots_for_filter(filter, ctx) {
                    for snapshot in snapshots.iter().filter(|snapshot| {
                        filter.matches_snapshot(snapshot, &filter_ctx, self.game)
                    }) {
                        visit(PropertyObject::Snapshot(snapshot));
                    }
                } else {
                    for id in value_candidate_ids_for_filter(self.game, filter, ctx) {
                        if let Some(object) = self.game.object(id)
                            && filter.matches(object, &filter_ctx, self.game)
                        {
                            visit(PropertyObject::Live(object));
                        }
                    }
                }
            }
            Mode::Continuous(layer) => layer.visit_candidates(filter, |object| {
                visit(PropertyObject::LayerBaseline(object))
            }),
        }
    }
}

pub(super) enum PropertyObject<'a> {
    Live(&'a Object),
    Snapshot(&'a ObjectSnapshot),
    LayerBaseline(&'a Object),
}

impl PropertyObject<'_> {
    pub(super) fn subtypes(&self, game: &GameState, current: bool) -> Vec<Subtype> {
        match self {
            Self::Live(object) if current => game
                .current_subtypes(object.id)
                .unwrap_or_else(|| object.subtypes.to_vec()),
            Self::Live(object) | Self::LayerBaseline(object) => object.subtypes.to_vec(),
            Self::Snapshot(snapshot) => snapshot.subtypes.to_vec(),
        }
    }
    pub(super) fn card_types(&self, game: &GameState) -> Vec<CardType> {
        match self {
            Self::Live(object) => game
                .current_card_types(object.id)
                .unwrap_or_else(|| object.card_types.to_vec()),
            Self::LayerBaseline(object) => object.card_types.to_vec(),
            Self::Snapshot(snapshot) => snapshot.card_types.to_vec(),
        }
    }
    pub(super) fn colors(&self) -> crate::color::ColorSet {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => object.colors(),
            Self::Snapshot(snapshot) => snapshot.colors,
        }
    }
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => &object.name,
            Self::Snapshot(snapshot) => &snapshot.name,
        }
    }
    pub(super) fn counters(&self) -> &std::collections::BTreeMap<crate::object::CounterType, u32> {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => &object.counters,
            Self::Snapshot(snapshot) => &snapshot.counters,
        }
    }
    pub(super) fn filter_mana_value(&self) -> i32 {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => {
                crate::filter::object_mana_value_for_filter(object)
            }
            Self::Snapshot(snapshot) => crate::filter::snapshot_mana_value_for_filter(snapshot),
        }
    }
    pub(super) fn power(&self, game: &GameState) -> Option<i32> {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => {
                game.calculated_power(object.id).or_else(|| object.power())
            }
            Self::Snapshot(snapshot) => snapshot.power,
        }
    }
    pub(super) fn has_ability(
        &self,
        game: &GameState,
        ability: ironsmith_core::StaticAbilityId,
    ) -> bool {
        match self {
            Self::Live(object) | Self::LayerBaseline(object) => {
                game.current_has_static_ability_id(object.id, ability)
            }
            Self::Snapshot(snapshot) => snapshot.has_static_ability_id(ability),
        }
    }
}

fn checked_kicker_count(paid: &crate::cost::OptionalCostsPaid) -> Option<i32> {
    paid.costs
        .iter()
        .filter(|(cost, _)| {
            matches!(
                cost.kind,
                crate::cost::OptionalCostKind::Kicker | crate::cost::OptionalCostKind::Multikicker
            )
        })
        .try_fold(0i32, |total, (_, count)| {
            total.checked_add(i32::try_from(*count).ok()?)
        })
}

impl NumericProperty {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::BasePower => "base power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
            Self::ManaSpent => "mana spent to cast",
            Self::KickerCount => "kicker payments",
            Self::ColorCount => "colors",
        }
    }
    pub(crate) fn snapshot(self, snapshot: &ObjectSnapshot) -> Option<i32> {
        match self {
            Self::Power => snapshot.power,
            Self::BasePower => snapshot.base_power,
            Self::Toughness => snapshot.toughness,
            Self::ManaValue => Some(crate::filter::snapshot_mana_value_for_filter(snapshot)),
            Self::ManaSpent => Some(snapshot.mana_spent_to_cast.total() as i32),
            Self::KickerCount => checked_kicker_count(&snapshot.optional_costs_paid),
            Self::ColorCount => Some(snapshot.colors.count() as i32),
        }
    }
    fn is_power_or_toughness(self) -> bool {
        matches!(self, Self::Power | Self::BasePower | Self::Toughness)
    }
    fn source_snapshot(self, snapshot: &ObjectSnapshot) -> Option<i32> {
        // CR 208.3 / 107.2: a known noncreature permanent has no P/T;
        // a required numeric read uses zero, not its printed creature stats.
        // Keep absent LKI or a missing characteristic on a known creature
        // distinguishable from this known-unavailable characteristic.
        if self.is_power_or_toughness()
            && snapshot.zone == Zone::Battlefield
            && !snapshot.card_types.contains(&CardType::Creature)
        {
            Some(0)
        } else {
            self.snapshot(snapshot)
        }
    }
    fn source_live(self, game: &GameState, object: &Object) -> Option<i32> {
        if self.is_power_or_toughness()
            && object.zone == Zone::Battlefield
            && game
                .current_card_types(object.id)
                .is_some_and(|types| !types.contains(&CardType::Creature))
        {
            Some(0)
        } else {
            self.live(game, object)
        }
    }
    pub(crate) fn raw(self, object: &Object) -> Option<i32> {
        match self {
            Self::Power => object.power(),
            Self::BasePower => object.base_power.as_ref().map(|value| value.base_value()),
            Self::Toughness => object.toughness(),
            Self::ManaValue => Some(crate::filter::object_mana_value_for_filter(object)),
            Self::ManaSpent => Some(object.mana_spent_to_cast.total() as i32),
            Self::KickerCount => checked_kicker_count(&object.optional_costs_paid),
            Self::ColorCount => Some(object.colors().count() as i32),
        }
    }
    pub(crate) fn live(self, game: &GameState, object: &Object) -> Option<i32> {
        match self {
            Self::Power => game.calculated_power(object.id).or_else(|| object.power()),
            Self::BasePower => game
                .calculated_characteristics(object.id)
                .and_then(|chars| chars.base_power)
                .or_else(|| self.raw(object)),
            Self::Toughness => game
                .calculated_toughness(object.id)
                .or_else(|| object.toughness()),
            Self::ManaValue | Self::ManaSpent | Self::ColorCount | Self::KickerCount => {
                self.raw(object)
            }
        }
    }
    pub(crate) fn characteristics(
        self,
        chars: &crate::continuous::CalculatedCharacteristics,
    ) -> Option<i32> {
        match self {
            Self::Power => chars.power,
            Self::BasePower => chars.base_power,
            Self::Toughness => chars.toughness,
            Self::ManaValue | Self::ManaSpent | Self::ColorCount | Self::KickerCount => None,
        }
    }
}

impl EvaluationContext<'_, '_> {
    pub(super) fn source_number(&self, property: NumericProperty) -> Result<i32, ExecutionError> {
        let Some(ctx) = self.execution() else {
            return Ok(self.layer().source_number(property));
        };
        let missing = |tense| {
            ExecutionError::UnresolvableValue(format!("Source {tense} no {}", property.label()))
        };
        if self.game.is_phased_out(self.source) {
            let snapshot = self
                .game
                .turn_store
                .turn_history
                .source_last_known_snapshot(self.source)
                .or_else(|| {
                    ctx.source_snapshot
                        .as_ref()
                        .filter(|snapshot| snapshot.object_id == self.source)
                })
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "phased damage/ability source has no exact last-known characteristics"
                            .into(),
                    )
                })?;
            return property
                .source_snapshot(snapshot)
                .ok_or_else(|| missing("had"));
        }
        if let Some(snapshot) = source_lki_for_moved_current_object(self.game, ctx) {
            property
                .source_snapshot(snapshot)
                .ok_or_else(|| missing("had"))
        } else if let Some(object) = self.game.object(self.source) {
            property
                .source_live(self.game, object)
                .ok_or_else(|| missing("has"))
        } else if let Some(snapshot) = &ctx.source_snapshot {
            property
                .source_snapshot(snapshot)
                .ok_or_else(|| missing("had"))
        } else {
            Err(ExecutionError::ObjectNotFound(self.source))
        }
    }
    pub(super) fn object_number(
        &self,
        spec: &ChooseSpec,
        property: NumericProperty,
    ) -> Result<i32, ExecutionError> {
        let Some(ctx) = self.execution() else {
            return Ok(self.layer().object_number(spec, property));
        };
        if matches!(spec.base(), ChooseSpec::Source) && property.is_power_or_toughness() {
            return self.source_number(property);
        }
        let missing = |tense| {
            ExecutionError::UnresolvableValue(format!("Target {tense} no {}", property.label()))
        };
        if matches!(spec.base(), ChooseSpec::Source)
            && let Some(snapshot) = source_lki_for_moved_current_object(self.game, ctx)
        {
            return property.snapshot(snapshot).ok_or_else(|| missing("had"));
        }
        // A tagged object that left its zone is read from its last known
        // information before any live lookup, which can no longer find it
        // (CR 608.2h).
        if let Some(snapshot) = tagged_lki_when_object_left(self.game, ctx, spec) {
            return property.snapshot(snapshot).ok_or_else(|| missing("had"));
        }
        let tagged = if let ChooseSpec::Tagged(tag) = spec.base() {
            ctx.get_tagged(tag)
        } else {
            None
        };
        let id = resolve_primary_object_from_value_spec(self.game, spec, ctx)?;
        if let Some(object) = self.game.object(id) {
            property
                .live(self.game, object)
                .ok_or_else(|| missing("has"))
        } else if let Some(snapshot) = tagged.or_else(|| object_lki_snapshot(ctx, id)) {
            property.snapshot(snapshot).ok_or_else(|| missing("had"))
        } else {
            Err(ExecutionError::ObjectNotFound(id))
        }
    }
}

impl EvaluationContext<'_, '_> {
    pub(super) fn unavailable<T>(
        &self,
        value: &Value,
        execution_reason: &str,
        layer_reason: &str,
    ) -> Result<T, ExecutionError> {
        match self.mode {
            Mode::Execution(_) => Err(ExecutionError::UnresolvableValue(execution_reason.into())),
            Mode::Continuous(layer) => layer.unsupported(value, layer_reason),
        }
    }
    pub(super) fn matching_player_ids(&self, filter: &PlayerFilter) -> Vec<PlayerId> {
        match self.mode {
            Mode::Execution(ctx) => {
                let filter_ctx = ctx.filter_context(self.game);
                self.game
                    .players
                    .iter()
                    .filter(|p| p.is_in_game() && filter.matches_player(p.id, &filter_ctx))
                    .map(|p| p.id)
                    .collect()
            }
            Mode::Continuous(layer) => layer.matching_players(filter),
        }
    }
    pub(super) fn library_player_ids(
        &self,
        value: &Value,
        filter: &PlayerFilter,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => Ok(vec![resolve_player_filter(self.game, filter, ctx)?]),
            Mode::Continuous(layer) => Ok(layer.players(value, filter)),
        }
    }
    pub(super) fn controlled_object_count(&self, filter: &ObjectFilter, player: PlayerId) -> usize {
        match self.mode {
            Mode::Execution(ctx) => {
                count_matching_objects_for_player(self.game, filter, player, ctx)
            }
            Mode::Continuous(layer) => layer.controlled_object_count(filter, player),
        }
    }
    pub(super) fn add_spell_metric(&self, total: i64, value: i64) -> Result<i64, ExecutionError> {
        total.checked_add(value).ok_or_else(|| ExecutionError::UnresolvableValue(
            "spell metric total exceeds the wide value range".into()))
    }
    pub(super) fn tagged_spell_id(
        &self,
        value: &Value,
        tag: &crate::tag::TagKey,
    ) -> Result<ObjectId, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => ctx.get_tagged(tag.as_str()).map(|snapshot| snapshot.object_id).ok_or_else(|| ExecutionError::UnresolvableValue(format!("DamageDealtThisTurnByTaggedSpellCast requires tagged spell snapshot '{tag}'"))),
            Mode::Continuous(layer) => Ok(self.game.object(self.source).and_then(|object| object.cast_tagged_objects.get(tag)).and_then(|snapshots| snapshots.first()).unwrap_or_else(|| layer.unsupported(value, "tagged spell cast is not retained on the continuous-effect source")).object_id),
        }
    }
}

#[cfg(test)]
mod referenced_kicker_count_tests {
    use super::*;
    #[test]
    fn counts_both_kicker_kinds_but_never_wraps_out_of_value_range() {
        let mut paid = crate::cost::OptionalCostsPaid::default();
        paid.costs = vec![
            ("Kicker".into(), 1),
            ("Multikicker".into(), 3),
            ("Buyback".into(), 7),
        ];
        assert_eq!(checked_kicker_count(&paid), Some(4));
        paid.costs[1].1 = i32::MAX as u32;
        assert_eq!(checked_kicker_count(&paid), None);
        paid.costs[1].1 = u32::MAX;
        assert_eq!(checked_kicker_count(&paid), None);
    }
}

#[cfg(test)]
mod aggregate_life_scope_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::CardType;
    #[test]
    fn team_scoped_life_aggregates_agree_in_resolution_and_continuous_contexts() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Teammate".into(),
                "Bob".into(),
                "Charlie".into(),
            ],
            41,
        );
        let [alice, teammate, bob, charlie] = std::array::from_fn(|i| game.players[i].id);
        game.restore_alternating_teams(
            vec![vec![alice, teammate], vec![bob, charlie]],
            vec![alice, bob, teammate, charlie],
            alice,
            crate::game_state::FreeForAllAttackOption::MultiplePlayers,
            None,
            false,
        )
        .unwrap();
        let card = CardBuilder::new(CardId::new(), "Team life source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        for (player, life) in [(alice, 20), (teammate, 50), (bob, 10), (charlie, 12)] {
            game.write_life_total(player, life);
        }
        let check = |game: &GameState, controller: PlayerId, value: Value, expected: i32| {
            let ctx = ExecutionContext::new_default(source, controller);
            assert_eq!(
                super::super::resolve(&value, &EvaluationContext::execution_context(game, &ctx))
                    .unwrap(),
                expected
            );
            assert_eq!(
                crate::continuous::resolve_value_direct(
                    &value,
                    game.objects_map(),
                    &[],
                    &game.battlefield,
                    &std::collections::HashSet::new(),
                    source,
                    controller,
                    game
                ),
                expected
            );
        };
        check(
            &game,
            alice,
            Value::MaximumLifeTotal(PlayerFilter::Opponent),
            12,
        );
        check(&game, alice, Value::MaximumLifeTotal(PlayerFilter::Any), 50);
        check(
            &game,
            alice,
            Value::MaximumLifeTotal(PlayerFilter::Teammate),
            50,
        );
        check(
            &game,
            alice,
            Value::MaximumLifeTotal(PlayerFilter::Excluding {
                base: Box::new(PlayerFilter::Any),
                excluded: Box::new(PlayerFilter::Opponent),
            }),
            50,
        );
        check(
            &game,
            alice,
            Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
            2,
        );
        game.write_life_total(teammate, 5);
        check(
            &game,
            alice,
            Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
            2,
        );
        check(
            &game,
            alice,
            Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Any),
            4,
        );
        check(
            &game,
            bob,
            Value::MaximumLifeTotal(PlayerFilter::Opponent),
            20,
        );
        check(
            &game,
            bob,
            Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
            2,
        );
    }
}

#[cfg(test)]
mod known_noncreature_source_tests {
    use super::*;

    #[test]
    fn source_statistics_use_known_zero_after_type_loss_live_departed_or_phased() {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            let source = game.create_object_from_definition(
                &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Printed creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(7, 9))
                    .build(),
                player,
                Zone::Battlefield,
            );
            crate::effects::execute_effect(
                &mut game,
                &crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Specific(source),
                    crate::continuous::Modification::SetCardTypes(vec![CardType::Artifact]),
                    crate::effect::Until::EndOfTurn,
                )),
                &mut ExecutionContext::new_default(source, player),
            )
            .unwrap();
            assert!(!game.current_is_creature(source));
            if mode == 1 {
                crate::effects::execute_effect(
                    &mut game,
                    &crate::effect::Effect::exile(ChooseSpec::SpecificObject(source)),
                    &mut ExecutionContext::new_default(source, player),
                )
                .unwrap();
            } else if mode == 2 {
                game.phase_out(source);
            }
            let mut ctx = ExecutionContext::new_default(source, player);
            if mode != 0 {
                ctx.source_snapshot = game
                    .turn_store
                    .turn_history
                    .source_last_known_snapshot(source)
                    .cloned();
                assert!(ctx.source_snapshot.is_some());
            }
            for value in [
                Value::SourcePower,
                Value::SourceToughness,
                Value::PowerOf(Box::new(ChooseSpec::Source)),
                Value::BasePowerOf(Box::new(ChooseSpec::Source)),
                Value::ToughnessOf(Box::new(ChooseSpec::Source)),
            ] {
                assert_eq!(
                    crate::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
                    0,
                    "mode {mode}: {value:?} must not resurrect printed statistics"
                );
            }
        }
    }

    #[test]
    fn absent_source_evidence_and_missing_creature_statistics_remain_errors() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        let source = ObjectId(997);
        let mut ctx = ExecutionContext::new_default(source, player);
        assert!(crate::effects::helpers::resolve_value(&game, &Value::SourcePower, &ctx).is_err());
        let mut snapshot =
            ObjectSnapshot::for_testing(source, player, "Missing creature statistic");
        snapshot.card_types = vec![CardType::Creature];
        snapshot.power = None;
        ctx.source_snapshot = Some(snapshot);
        assert!(matches!(
            crate::effects::helpers::resolve_value(&game, &Value::SourcePower, &ctx),
            Err(ExecutionError::UnresolvableValue(_))
        ));
    }
}
