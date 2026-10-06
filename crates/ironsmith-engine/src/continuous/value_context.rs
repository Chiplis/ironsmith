use super::*;

#[derive(Clone, Copy)]
pub(crate) struct LayerValueContext<'a, 'game> {
    pub calculation: &'a CalculationContext<'game>,
    pub source: ObjectId,
    pub controller: PlayerId,
    pub numeric_origin: Option<&'a crate::continuous::AbilityOrigin>,
    direct_counts: Option<(&'a [ContinuousEffect], &'a HashSet<ObjectId>)>,
}

impl<'a, 'game> LayerValueContext<'a, 'game> {
    pub fn new(
        calculation: &'a CalculationContext<'game>,
        source: ObjectId,
        controller: PlayerId,
    ) -> Self {
        Self {
            calculation,
            source,
            controller,
            direct_counts: None,
            numeric_origin: None,
        }
    }
    pub fn direct(
        calculation: &'a CalculationContext<'game>,
        source: ObjectId,
        controller: PlayerId,
        effects: &'a [ContinuousEffect],
        commanders: &'a HashSet<ObjectId>,
    ) -> Self {
        Self {
            calculation,
            source,
            controller,
            direct_counts: Some((effects, commanders)),
            numeric_origin: None,
        }
    }
    pub fn with_numeric_origin(mut self,origin:Option<&'a crate::continuous::AbilityOrigin>)->Self{
        self.numeric_origin=origin;self
    }
    pub fn filter_context(&self) -> crate::filter::FilterContext {
        continuous_filter_context(self.calculation.game, self.controller, self.source)
    }
    /// Filter context for a count that may compare against "it", the object
    /// the continuous effect is currently modifying ("each other creature
    /// that shares a creature type with it", Coat of Arms). The affected
    /// object is bound with its in-progress layered subtypes so changeling
    /// and other type-changing effects already applied are respected.
    fn filter_context_for_count(&self, filter: &ObjectFilter) -> crate::filter::FilterContext {
        let mut filter_ctx = self.filter_context();
        let it = TagKey::from("__it__");
        if filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == it)
            && !filter_ctx.tagged_objects.contains_key(&it)
            && let Some(object) = self.calculation.game.object(self.calculation.current_object)
        {
            let mut snapshot = ObjectSnapshot::from_object(object, self.calculation.game);
            if let Some(chars) = in_progress_characteristics(self.calculation.game, object.id) {
                snapshot.subtypes = chars.subtypes.to_vec();
                snapshot.card_types = chars.card_types.to_vec();
            }
            filter_ctx.tagged_objects.insert(it, vec![snapshot]);
        }
        filter_ctx
    }
    pub fn count(&self, filter: &ObjectFilter) -> i32 {
        let ctx = self.calculation;
        let filter_ctx = self.filter_context_for_count(filter);
        let Some((effects, commanders)) = self.direct_counts else {
            return count_filter_matches(filter, ctx, &filter_ctx);
        };
        if let Some(count) = count_retained_tagged_snapshot_matches(filter, ctx.game, &filter_ctx) {
            return count;
        }
        let mut count = 0;
        for_each_filter_candidate(ctx, filter, |object| {
            if let Some(chars) = calculate_characteristics_with_effects_simple(
                object.id,
                ctx.objects,
                effects,
                ctx.battlefield,
                commanders,
                ctx.game,
            ) && filter_matches_with_characteristics_in_context(
                filter,
                object,
                &chars,
                ctx.game,
                &filter_ctx,
            ) {
                count += 1;
            }
        });
        count
    }
    // Greatest-count and other aggregate leaves have always used the contextual
    // path, even when reached through the direct evaluator.
    pub fn aggregate_count(&self, filter: &ObjectFilter) -> i32 {
        count_filter_matches(filter, self.calculation, &self.filter_context())
    }
    pub fn shared_creature_count(&self, filter: &ObjectFilter) -> i32 {
        greatest_shared_creature_type_count_for_filter(
            self.calculation,
            filter,
            self.controller,
            self.source,
        )
    }
    pub fn visit_candidates(&self, filter: &ObjectFilter, mut visitor: impl FnMut(&Object)) {
        let filter_ctx = self.filter_context();
        for_each_filter_candidate(self.calculation, filter, |object| {
            if filter.matches_non_recursive(object, &filter_ctx, self.calculation.game) {
                visitor(object);
            }
        });
    }
    pub fn visit_layered(
        &self,
        filter: &ObjectFilter,
        visitor: impl FnMut(&Object, &CalculatedCharacteristics),
    ) {
        for_each_matching_continuous_object(
            self.calculation,
            filter,
            self.controller,
            self.source,
            visitor,
        )
    }
    /// Numeric aggregates have an explicit empty-set identity, unlike a
    /// scalar player reference that must designate an available player.
    pub fn aggregate_players(&self, filter: &PlayerFilter) -> Vec<PlayerId> {
        super::layer_resolution::continuous_value_players(
            self.calculation,
            filter,
            self.controller,
            self.source,
        )
    }
    pub fn players(&self, value: &Value, filter: &PlayerFilter) -> Vec<PlayerId> {
        required_continuous_value_players(
            value,
            self.calculation,
            filter,
            self.controller,
            self.source,
        )
    }
    pub fn filtered_players(&self, filter: &PlayerFilter) -> Vec<PlayerId> {
        let filter_ctx = self.filter_context();
        self.calculation
            .game
            .players
            .iter()
            .filter(|p| p.is_in_game() && filter.matches_player(p.id, &filter_ctx))
            .map(|p| p.id)
            .collect()
    }
    pub fn unsupported(&self, value: &Value, reason: &str) -> ! {
        unsupported_continuous_value(value, reason)
    }

    pub fn party_size(&self, value: &Value, player_filter: &PlayerFilter) -> i32 {
        let ctx = self.calculation;
        self.players(value, player_filter)
            .into_iter()
            .map(|player| {
                // Read the layer calculation's current characteristics, avoiding a
                // fresh GameState query that could recurse into this same value.
                crate::party::party_size_from_roles(ctx.battlefield.iter().filter_map(|id| {
                    let chars = ctx.effects.calculate_characteristics(
                        *id,
                        ctx.objects,
                        ctx.battlefield,
                        ctx.game,
                    )?;
                    if chars.controller != player || !chars.card_types.contains(&CardType::Creature)
                    {
                        return None;
                    }
                    Some(crate::party::PARTY_ROLES.iter().enumerate().fold(
                        0u8,
                        |roles, (index, role)| {
                            roles
                                | if chars.subtypes.contains(role) {
                                    1 << index
                                } else {
                                    0
                                }
                        },
                    ))
                }))
            })
            .sum()
    }

    pub fn mana_symbols_in_mana_cost_of(
        &self,
        _value: &Value,
        spec: &Box<ChooseSpec>,
        color: &Color,
    ) -> i32 {
        let ctx = self.calculation;
        let controller = self.controller;
        let source = self.source;
        {
            let symbol = crate::mana::ManaSymbol::from_color(*color);
            let count_symbols = |object: &Object| {
                object
                    .mana_cost
                    .as_ref()
                    .map(|cost| {
                        cost.pips()
                            .iter()
                            .filter(|pip| pip.contains(&symbol))
                            .count() as i32
                    })
                    .unwrap_or(0)
            };
            if let ChooseSpec::All(filter) = spec.unhinted() {
                let mut total = 0;
                for_each_matching_continuous_object(
                    ctx,
                    filter,
                    controller,
                    source,
                    |object, _| total += count_symbols(object),
                );
                total
            } else {
                object_for_value_spec(spec, ctx, source)
                    .map(count_symbols)
                    .unwrap_or(0)
            }
        }
    }

    pub fn mana_from_source_spent_to_cast_this_spell(
        &self,
        _value: &Value,
        source_filter: &ObjectFilter,
    ) -> i32 {
        let ctx = self.calculation;
        let controller = self.controller;
        let source = self.source;
        {
            let filter_ctx = continuous_filter_context(ctx.game, controller, source);
            ctx.game
                .object(source)
                .and_then(|object| {
                    object
                        .cast_tagged_objects
                        .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
                })
                .map(|snapshots| {
                    snapshots
                        .iter()
                        .filter(|snapshot| {
                            source_filter.matches_snapshot(snapshot, &filter_ctx, ctx.game)
                        })
                        .count() as i32
                })
                .unwrap_or(0)
        }
    }

    pub fn counters_on_source(&self, _value: &Value, counter_type: &CounterType) -> i64 {
        let ctx = self.calculation;
        let source = self.source;
        ctx.objects
            .get(&source)
            .map(|o| i64::from(o.counters.get(counter_type).copied().unwrap_or(0)))
            .unwrap_or(0)
    }
    pub fn counters_on(
        &self,
        _value: &Value,
        spec: &Box<ChooseSpec>,
        counter_type: &Option<CounterType>,
    ) -> Result<i64, crate::effects::ExecutionError> {
        let ctx = self.calculation;
        let controller = self.controller;
        let source = self.source;
        let counter_total = |object: &Object| match counter_type {
            Some(counter_type) => Ok(i64::from(object.counters.get(counter_type).copied().unwrap_or(0))),
            None => object.counters.values().try_fold(0i64, |sum, count|
                checked_counter_total(sum, i64::from(*count))),
        };

        if let ChooseSpec::All(filter) = spec.unhinted() {
            let filter_ctx = continuous_filter_context(ctx.game, controller, source);
            let mut total = Ok(0i64);
            for_each_filter_candidate(ctx, filter, |object| {
                let subtotal = match &total { Ok(value) => *value, Err(_) => return };
                let matches = ctx.effects.calculate_characteristics(
                    object.id, ctx.objects, ctx.battlefield, ctx.game,
                ).is_some_and(|chars| {
                    filter_matches_with_characteristics(
                        filter, object, &chars, ctx.game,
                        filter_ctx.you.unwrap_or(object.owner),
                        filter_ctx.source.unwrap_or(ctx.current_object),
                    )
                });
                if matches {
                    total = counter_total(object).and_then(|count| checked_counter_total(subtotal, count));
                }
            });
            total
        } else {
            object_for_value_spec(spec, ctx, source).map(counter_total).unwrap_or(Ok(0))
        }
    }

}
/// Keep counter cardinalities wide until their consumer's checked scalar
/// boundary. Never reinterpret a u32 counter count or a multi-object sum as i32.
fn checked_counter_total(total: i64, count: i64) -> Result<i64, crate::effects::ExecutionError> {
    total.checked_add(count).ok_or(crate::effects::ExecutionError::ResourceLimitExceeded {
        resource: "continuous counter total",
        requested: total as u128 + count as u128,
        maximum: i64::MAX as u128,
    })
}

fn object_for_value_spec<'a>(
    spec: &ChooseSpec,
    ctx: &'a CalculationContext<'_>,
    source: ObjectId,
) -> Option<&'a Object> {
    match spec.base() {
        ChooseSpec::Iterated => ctx.objects.get(&ctx.current_object).map(|object| &**object),
        ChooseSpec::Source => ctx.objects.get(&source).map(|object| &**object),
        ChooseSpec::SpecificObject(object_id) => ctx.objects.get(object_id).map(|object| &**object),
        _ => None,
    }
}

impl LayerValueContext<'_, '_> {
    pub fn source_number(
        &self,
        property: crate::effects::helpers::value_eval::NumericProperty,
    ) -> i32 {
        self.number_at(self.source, property)
    }
    pub fn object_number(
        &self,
        spec: &ChooseSpec,
        property: crate::effects::helpers::value_eval::NumericProperty,
    ) -> i32 {
        object_for_value_spec(spec, self.calculation, self.source)
            .map(|object| self.number_at(object.id, property))
            .unwrap_or(0)
    }
    fn number_at(
        &self,
        id: ObjectId,
        property: crate::effects::helpers::value_eval::NumericProperty,
    ) -> i32 {
        use crate::effects::helpers::value_eval::NumericProperty;
        let ctx = self.calculation;
        if matches!(property, NumericProperty::ManaValue) {
            let Some(object) = ctx.objects.get(&id) else {
                return 0;
            };
            // Layer-1 copies, face-down values and linked-face rules are
            // already present in the in-progress view at the P/T layer.
            // Reading the printed object here loses that mana cost.
            return in_progress_characteristics(ctx.game, id)
                .or_else(|| {
                    ctx.effects.calculate_characteristics(
                        id,
                        ctx.objects,
                        ctx.battlefield,
                        ctx.game,
                    )
                })
                .map_or_else(
                    || crate::filter::object_mana_value_for_filter(object),
                    |chars| crate::filter::calculated_mana_value_for_filter(object, &chars),
                );
        }
        if matches!(
            property,
            NumericProperty::ManaSpent | NumericProperty::KickerCount
        ) {
            return ctx
                .objects
                .get(&id)
                .and_then(|object| property.raw(object))
                .unwrap_or(0);
        }
        in_progress_characteristics(ctx.game, id)
            .and_then(|chars| property.characteristics(&chars))
            .or_else(|| {
                ctx.effects
                    .calculate_characteristics(id, ctx.objects, ctx.battlefield, ctx.game)
                    .and_then(|chars| property.characteristics(&chars))
            })
            .or_else(|| ctx.objects.get(&id).and_then(|object| property.raw(object)))
            .unwrap_or(0)
    }
}

impl LayerValueContext<'_, '_> {
    pub fn matching_players(&self, filter: &PlayerFilter) -> Vec<PlayerId> {
        continuous_value_players(self.calculation, filter, self.controller, self.source)
    }
    pub fn controlled_object_count(&self, filter: &ObjectFilter, player: PlayerId) -> usize {
        let mut filter = filter.clone();
        filter.controller = Some(PlayerFilter::Specific(player));
        count_filter_matches(&filter, self.calculation, &self.filter_context()) as usize
    }
}
