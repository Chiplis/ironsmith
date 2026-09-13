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
    pub(super) fn x(&self) -> Result<i32, ExecutionError> {
        match self.mode {
            Mode::Execution(ctx) => ctx
                .x_value
                .map(|x| x as i32)
                .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".into())),
            Mode::Continuous(_) => Ok(0),
        }
    }
    pub(super) fn division_by_zero(&self, inner: &Value) -> Result<i32, ExecutionError> {
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
    ) -> i32 {
        let mut result: Option<i32> = None;
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
                            NumericProperty::Toughness => snapshot.toughness,
                            NumericProperty::ManaValue => snapshot
                                .mana_cost
                                .as_ref()
                                .map(|cost| cost.mana_value() as i32),
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
                            NumericProperty::Power => {
                                self.game.calculated_power(id).or_else(|| object.power())
                            }
                            NumericProperty::Toughness => self
                                .game
                                .calculated_toughness(id)
                                .or_else(|| object.toughness()),
                            NumericProperty::ManaValue => object
                                .mana_cost
                                .as_ref()
                                .map(|cost| cost.mana_value() as i32),
                        });
                    }
                }
            }
            Mode::Continuous(layer) => layer.visit_layered(filter, |object, chars| {
                visit(match property {
                    NumericProperty::Power => chars.power,
                    NumericProperty::Toughness => chars.toughness,
                    // Layer extrema have always treated absent mana costs as zero.
                    NumericProperty::ManaValue => Some(
                        object
                            .mana_cost
                            .as_ref()
                            .map_or(0, |cost| cost.mana_value() as i32),
                    ),
                })
            }),
        }
        result.unwrap_or(0)
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
    pub(super) fn counters(&self) -> &HashMap<crate::object::CounterType, u32> {
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

impl NumericProperty {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
        }
    }
    pub(crate) fn snapshot(self, snapshot: &ObjectSnapshot) -> Option<i32> {
        match self {
            Self::Power => snapshot.power,
            Self::Toughness => snapshot.toughness,
            Self::ManaValue => snapshot
                .mana_cost
                .as_ref()
                .map(|cost| cost.mana_value() as i32),
        }
    }
    pub(crate) fn raw(self, object: &Object) -> Option<i32> {
        match self {
            Self::Power => object.power(),
            Self::Toughness => object.toughness(),
            Self::ManaValue => object
                .mana_cost
                .as_ref()
                .map(|cost| cost.mana_value() as i32),
        }
    }
    pub(crate) fn live(self, game: &GameState, object: &Object) -> Option<i32> {
        match self {
            Self::Power => game.calculated_power(object.id).or_else(|| object.power()),
            Self::Toughness => game
                .calculated_toughness(object.id)
                .or_else(|| object.toughness()),
            Self::ManaValue => self.raw(object),
        }
    }
    pub(crate) fn characteristics(
        self,
        chars: &crate::continuous::CalculatedCharacteristics,
    ) -> Option<i32> {
        match self {
            Self::Power => chars.power,
            Self::Toughness => chars.toughness,
            Self::ManaValue => None,
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
        if let Some(snapshot) = source_lki_for_moved_current_object(self.game, ctx) {
            property.snapshot(snapshot).ok_or_else(|| missing("had"))
        } else if let Some(object) = self.game.object(self.source) {
            property
                .live(self.game, object)
                .ok_or_else(|| missing("has"))
        } else if let Some(snapshot) = &ctx.source_snapshot {
            property.snapshot(snapshot).ok_or_else(|| missing("had"))
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
        let id = resolve_primary_object_from_value_spec(self.game, spec, ctx)?;
        let tagged = if let ChooseSpec::Tagged(tag) = spec.base() {
            ctx.get_tagged(tag)
        } else {
            None
        };
        let missing = |tense| {
            ExecutionError::UnresolvableValue(format!("Target {tense} no {}", property.label()))
        };
        if matches!(spec.base(), ChooseSpec::Source)
            && let Some(snapshot) = source_lki_for_moved_current_object(self.game, ctx)
        {
            property.snapshot(snapshot).ok_or_else(|| missing("had"))
        } else if let Some(snapshot) = tagged
            && self
                .game
                .object(snapshot.object_id)
                .is_none_or(|object| object.zone != snapshot.zone)
        {
            property
                .snapshot(latest_tagged_lki_snapshot(self.game, snapshot).unwrap_or(snapshot))
                .ok_or_else(|| missing("had"))
        } else if let Some(object) = self.game.object(id) {
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
    pub(super) fn unavailable(
        &self,
        value: &Value,
        execution_reason: &str,
        layer_reason: &str,
    ) -> Result<i32, ExecutionError> {
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
    pub(super) fn add_spell_metric(&self, total: i32, value: i32) -> i32 {
        match self.mode {
            Mode::Execution(_) => total.saturating_add(value),
            Mode::Continuous(_) => total + value,
        }
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
