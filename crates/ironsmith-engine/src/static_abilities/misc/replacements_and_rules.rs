use super::*;

/// "Double all damage that sources you control of the chosen type would deal."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleDamageFromSourcesYouControlOfChosenType {
    pub display: String,
}

impl DoubleDamageFromSourcesYouControlOfChosenType {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

#[derive(Debug, Clone)]
struct ChosenTypeDamageSourceMatcher {
    ability_source: ObjectId,
}

impl ReplacementMatcher for ChosenTypeDamageSourceMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::EventContext,
    ) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };
        let Some(chosen_type) = ctx.game.chosen_creature_type(self.ability_source) else {
            return false;
        };
        let Some(source_obj) = ctx.game.object(damage.source) else {
            return false;
        };

        ctx.game.current_controller(source_obj.id) == Some(ctx.controller)
            && ctx.game.current_has_subtype(source_obj.id, chosen_type)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "If a source you control of the chosen type would deal damage".to_string()
    }
}

impl StaticAbilityKind for DoubleDamageFromSourcesYouControlOfChosenType {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoubleDamageFromSourcesYouControlOfChosenType
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ChosenTypeDamageSourceMatcher {
                ability_source: source,
            },
            ReplacementAction::Double,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectDamageToSourceController {
    pub source_filter: ObjectFilter,
    pub target_player_filter: PlayerFilter,
    pub display: String,
}

impl RedirectDamageToSourceController {
    pub fn new(
        source_filter: ObjectFilter,
        target_player_filter: PlayerFilter,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            target_player_filter,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for RedirectDamageToSourceController {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RedirectDamageToSourceController
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageAmountReplacementMatcher {
                source_filter: self.source_filter.clone(),
                target_player_filter: Some(self.target_player_filter.clone()),
                target_object_filter: None,
                condition: None,
                combat_only: false,
                noncombat_only: false,
                amount_less_than: None,
            },
            ReplacementAction::Redirect {
                target: RedirectTarget::ToSourceController,
                which: RedirectWhich::First,
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifyDamageAmountReplacement {
    pub source_filter: ObjectFilter,
    pub target_player_filter: Option<PlayerFilter>,
    pub target_object_filter: Option<ObjectFilter>,
    pub delta: i32,
    pub noncombat_only: bool,
    pub display: String,
    pub condition: Option<crate::ConditionExpr>,
}

impl ModifyDamageAmountReplacement {
    pub fn new(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        delta: i32,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            target_player_filter,
            target_object_filter,
            delta,
            noncombat_only: false,
            display: display.into(),
            condition: None,
        }
    }

    pub fn with_noncombat_only(mut self, noncombat_only: bool) -> Self {
        self.noncombat_only = noncombat_only;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinimumDamageAmountReplacement {
    pub source_filter: ObjectFilter,
    pub target_player_filter: Option<PlayerFilter>,
    pub target_object_filter: Option<ObjectFilter>,
    pub floor: Value,
    pub noncombat_only: bool,
    pub display: String,
}

impl MinimumDamageAmountReplacement {
    pub fn new(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        floor: Value,
        noncombat_only: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            target_player_filter,
            target_object_filter,
            floor,
            noncombat_only,
            display: display.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DamageAmountReplacementMatcher {
    source_filter: ObjectFilter,
    target_player_filter: Option<PlayerFilter>,
    target_object_filter: Option<ObjectFilter>,
    condition: Option<crate::ConditionExpr>,
    combat_only: bool,
    noncombat_only: bool,
    amount_less_than: Option<Value>,
}

impl DamageAmountReplacementMatcher {
    fn source_matches(
        &self,
        damage: &DamageEvent,
        ctx: &crate::events::context::EventContext<'_>,
    ) -> bool {
        // Use LKI only when the source no longer exists. A still-live source
        // may have changed controller or types since its ability was put on
        // the stack; an older snapshot cannot make it match again.
        if let Some(source) = ctx.game.object(damage.source) {
            let filter_ctx = if source.zone == Zone::Stack {
                ctx.filter_ctx
                    .clone()
                    .with_caster(ctx.game.current_controller(damage.source))
            } else {
                ctx.filter_ctx.clone()
            };
            return self.source_filter.matches(source, &filter_ctx, ctx.game);
        }
        ctx
            .event_source_snapshot
            .filter(|snapshot| snapshot.object_id == damage.source)
            .is_some_and(|snapshot| {
                let filter_ctx = if snapshot.zone == Zone::Stack {
                    ctx.filter_ctx
                        .clone()
                        .with_caster(Some(snapshot.controller))
                } else {
                    ctx.filter_ctx.clone()
                };
                self.source_filter
                    .matches_snapshot(snapshot, &filter_ctx, ctx.game)
            })
    }

    fn target_matches(
        &self,
        damage: &DamageEvent,
        ctx: &crate::events::context::EventContext<'_>,
    ) -> bool {
        match damage.target {
            crate::events::DamageTarget::Player(player) => self
                .target_player_filter
                .as_ref()
                .is_some_and(|filter| filter.matches_player(player, &ctx.filter_ctx)),
            crate::events::DamageTarget::Object(object_id) => {
                let Some(filter) = &self.target_object_filter else {
                    return false;
                };
                ctx.game
                    .object(object_id)
                    .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
            }
        }
    }

    fn condition_matches(&self, ctx: &crate::events::context::EventContext<'_>) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: Some(source),
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(ctx.game, condition, &eval_ctx)
    }

    fn amount_matches(
        &self,
        damage: &DamageEvent,
        ctx: &crate::events::context::EventContext<'_>,
    ) -> bool {
        if self.combat_only && !damage.is_combat {
            return false;
        }
        if self.noncombat_only && damage.is_combat {
            return false;
        }
        let Some(value) = &self.amount_less_than else {
            return true;
        };
        let Some(source) = ctx.source else {
            return false;
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut eval_ctx = crate::effects::ExecutionContext::new(source, ctx.controller, &mut dm);
        if let Some(source_obj) = ctx.game.object(source) {
            eval_ctx.optional_costs_paid = source_obj.optional_costs_paid.clone();
            if !source_obj.cast_tagged_objects.is_empty() {
                eval_ctx = eval_ctx.with_tagged_objects(source_obj.cast_tagged_objects.clone());
            }
        }
        let Ok(floor) = crate::effects::helpers::resolve_value(ctx.game, value, &eval_ctx) else {
            return false;
        };
        (damage.amount as i32) < floor
    }
}

impl ReplacementMatcher for DamageAmountReplacementMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::EventContext<'_>,
    ) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        self.condition_matches(ctx)
            && self.source_matches(damage, ctx)
            && self.target_matches(damage, ctx)
            && self.amount_matches(damage, ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "If matching damage would be dealt".to_string()
    }
}

impl StaticAbilityKind for ModifyDamageAmountReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ModifyDamageAmountReplacement
    }

    fn display(&self) -> String {
        let Some(condition) = &self.condition else {
            return self.display.clone();
        };
        let condition = super::super::describe_static_condition(condition);
        if let Some(rest) = condition.strip_prefix("as long as ")
            && let Some(if_tail) = self.display.strip_prefix("If ")
        {
            return format!("As long as {rest}, if {if_tail}");
        }
        format!("{} {}", self.display, condition)
    }

    fn with_static_condition(
        &self,
        condition: crate::ConditionExpr,
    ) -> Option<super::StaticAbility> {
        Some(super::StaticAbility::new(
            self.clone().with_condition(condition),
        ))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        if self.delta == 0 {
            return None;
        }
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageAmountReplacementMatcher {
                source_filter: self.source_filter.clone(),
                target_player_filter: self.target_player_filter.clone(),
                target_object_filter: self.target_object_filter.clone(),
                condition: self.condition.clone(),
                combat_only: false,
                noncombat_only: self.noncombat_only,
                amount_less_than: None,
            },
            ReplacementAction::Modify(EventModification::Add(self.delta)),
        ))
    }
}

impl StaticAbilityKind for MinimumDamageAmountReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ModifyDamageAmountReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageAmountReplacementMatcher {
                source_filter: self.source_filter.clone(),
                target_player_filter: self.target_player_filter.clone(),
                target_object_filter: self.target_object_filter.clone(),
                condition: None,
                combat_only: false,
                noncombat_only: self.noncombat_only,
                amount_less_than: Some(self.floor.clone()),
            },
            ReplacementAction::Modify(EventModification::SetToAtLeast(self.floor.clone())),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleDamageAmountReplacement {
    pub source_filter: ObjectFilter,
    pub target_player_filter: Option<PlayerFilter>,
    pub target_object_filter: Option<ObjectFilter>,
    pub factor: u32,
    pub combat_only: bool,
    pub noncombat_only: bool,
    pub display: String,
}

impl DoubleDamageAmountReplacement {
    pub fn new(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        factor: u32,
        combat_only: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            target_player_filter,
            target_object_filter,
            factor,
            combat_only,
            noncombat_only: false,
            display: display.into(),
        }
    }

    pub fn noncombat_only(mut self) -> Self {
        self.noncombat_only = true;
        self
    }
}

impl StaticAbilityKind for DoubleDamageAmountReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ModifyDamageAmountReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageAmountReplacementMatcher {
                source_filter: self.source_filter.clone(),
                target_player_filter: self.target_player_filter.clone(),
                target_object_filter: self.target_object_filter.clone(),
                condition: None,
                combat_only: self.combat_only,
                noncombat_only: self.noncombat_only,
                amount_less_than: None,
            },
            ReplacementAction::Modify(EventModification::Multiply(self.factor)),
        ))
    }
}

/// "If a source would deal damage to you or a permanent you control, prevent
/// half that damage, rounded up." (Gisela, Blade of Goldnight)
#[derive(Debug, Clone, PartialEq)]
pub struct PreventHalfDamageReplacement {
    pub source_filter: ObjectFilter,
    pub target_player_filter: Option<PlayerFilter>,
    pub target_object_filter: Option<ObjectFilter>,
    pub round_up: bool,
    pub display: String,
}

impl PreventHalfDamageReplacement {
    pub fn new(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        round_up: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            target_player_filter,
            target_object_filter,
            round_up,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for PreventHalfDamageReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventHalfDamageReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageAmountReplacementMatcher {
                source_filter: self.source_filter.clone(),
                target_player_filter: self.target_player_filter.clone(),
                target_object_filter: self.target_object_filter.clone(),
                condition: None,
                combat_only: false,
                noncombat_only: false,
                amount_less_than: None,
            },
            ReplacementAction::PreventHalfDamage {
                round_up: self.round_up,
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleCountersReplacement {
    pub filter: ObjectFilter,
    pub player_filter: Option<PlayerFilter>,
    pub counter_type: Option<CounterType>,
    /// Who must be putting the counters; `None` matches any actor.
    pub actor: Option<PlayerFilter>,
    /// With `player_filter` set, also match permanents matching `filter`.
    pub includes_permanents: bool,
    /// Halve (rounded down) instead of doubling.
    pub halve: bool,
    pub display: String,
}

impl DoubleCountersReplacement {
    pub fn new(filter: ObjectFilter, counter_type: Option<CounterType>, display: String) -> Self {
        Self {
            filter,
            player_filter: None,
            counter_type,
            actor: None,
            includes_permanents: false,
            halve: false,
            display,
        }
    }

    pub fn new_for_player(
        player_filter: PlayerFilter,
        counter_type: Option<CounterType>,
        display: String,
    ) -> Self {
        Self {
            filter: ObjectFilter::default(),
            player_filter: Some(player_filter),
            counter_type,
            actor: None,
            includes_permanents: false,
            halve: false,
            display,
        }
    }

    /// "If <actor> would put one or more counters on a permanent or player,
    /// ... twice/half that many ... instead."
    pub fn new_for_actor(actor: PlayerFilter, halve: bool, display: String) -> Self {
        Self {
            filter: ObjectFilter::permanent(),
            player_filter: Some(PlayerFilter::Any),
            counter_type: None,
            actor: Some(actor),
            includes_permanents: true,
            halve,
            display,
        }
    }
}

#[derive(Debug, Clone)]
struct WouldPutCountersOrEnterWithCountersMatcher {
    ability_source: ObjectId,
    controller: PlayerId,
    filter: ObjectFilter,
    player_filter: Option<PlayerFilter>,
    counter_type: Option<CounterType>,
    actor: Option<PlayerFilter>,
    includes_permanents: bool,
}

impl WouldPutCountersOrEnterWithCountersMatcher {
    fn actor_matches(&self, actor: Option<PlayerId>, game: &crate::game_state::GameState) -> bool {
        let Some(required) = &self.actor else {
            return true;
        };
        let Some(actor) = actor else {
            return false;
        };
        player_ids_for_filter(game, required.clone(), self.controller).contains(&actor)
    }
}

impl ReplacementMatcher for WouldPutCountersOrEnterWithCountersMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::EventContext,
    ) -> bool {
        match event.event_kind() {
            EventKind::PutCounters => {
                let Some(put_counters) = downcast_event::<crate::events::PutCountersEvent>(event)
                else {
                    return false;
                };
                if self
                    .counter_type
                    .is_some_and(|counter_type| counter_type != put_counters.counter_type)
                {
                    return false;
                }
                if !self.actor_matches(put_counters.cause.source_controller, ctx.game) {
                    return false;
                }
                match put_counters.target {
                    crate::game_state::Target::Object(object) => {
                        (self.player_filter.is_none() || self.includes_permanents)
                            && ctx.game.object(object).is_some_and(|obj| {
                                self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                            })
                    }
                    crate::game_state::Target::Player(player) => {
                        self.player_filter.as_ref().is_some_and(|filter| {
                            player_ids_for_filter(ctx.game, filter.clone(), self.controller)
                                .contains(&player)
                        })
                    }
                }
            }
            EventKind::EnterBattlefield => {
                if self.player_filter.is_some() && !self.includes_permanents {
                    return false;
                }
                let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                    return false;
                };
                if etb.object == self.ability_source {
                    return false;
                }
                if self.actor.is_some()
                    && !self.actor_matches(
                        ctx.game.controller_of_id(etb.object),
                        ctx.game,
                    )
                {
                    return false;
                }
                if !etb
                    .enters_with_counters
                    .iter()
                    .any(|(counter_type, count)| {
                        *count > 0
                            && self
                                .counter_type
                                .is_none_or(|required| required == *counter_type)
                    })
                {
                    return false;
                }
                // Match the characteristics and controller the object will have
                // on the battlefield, including other entry replacements.
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    self.filter.clone(),
                ).matches_event(event, ctx)
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        "When counters would be put on a matching permanent".to_string()
    }
}

impl StaticAbilityKind for DoubleCountersReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoubleCountersReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldPutCountersOrEnterWithCountersMatcher {
                ability_source: source,
                controller,
                filter: self.filter.clone(),
                player_filter: self.player_filter.clone(),
                counter_type: self.counter_type,
                actor: self.actor.clone(),
                includes_permanents: self.includes_permanents,
            },
            if self.halve {
                ReplacementAction::HalveCounters {
                    counter_type: self.counter_type,
                }
            } else {
                ReplacementAction::DoubleCounters {
                    counter_type: self.counter_type,
                }
            },
        ))
    }
}

/// "If one or more [type] counters would be put on a [filter], that many plus
/// N (or minus N) are put on it instead.
#[derive(Debug, Clone, PartialEq)]
pub struct AddCountersPlacementReplacement {
    pub filter: ObjectFilter,
    pub player_filter: Option<PlayerFilter>,
    pub counter_type: Option<CounterType>,
    pub additional: i64,
    pub display: String,
}

impl AddCountersPlacementReplacement {
    pub fn new(
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        additional: impl Into<i64>,
        display: String,
    ) -> Self {
        Self {
            filter,
            player_filter: None,
            counter_type,
            additional: additional.into(),
            display,
        }
    }

    /// Counters a matching player would get instead of counters on permanents.
    pub fn for_player(mut self, player_filter: PlayerFilter) -> Self {
        self.player_filter = Some(player_filter);
        self
    }
}

impl StaticAbilityKind for AddCountersPlacementReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddCountersPlacementReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldPutCountersOrEnterWithCountersMatcher {
                ability_source: source,
                controller,
                filter: self.filter.clone(),
                player_filter: self.player_filter.clone(),
                counter_type: self.counter_type,
                actor: None,
                includes_permanents: false,
            },
            ReplacementAction::AddCountersToPlacement {
                counter_type: self.counter_type,
                additional: self.additional,
            },
        ))
    }
}

/// Replacement for "if you would get one or more [kind] counters ... you
/// can't get additional [kind] counters this turn." The allowance is scoped
/// to the affected player and resets with turn history.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCounterPerTurnLimitReplacement {
    pub player_filter: PlayerFilter,
    pub counter_type: CounterType,
    pub maximum: u32,
    pub display: String,
}

impl PlayerCounterPerTurnLimitReplacement {
    pub fn new(
        player_filter: PlayerFilter,
        counter_type: CounterType,
        maximum: u32,
        display: impl Into<String>,
    ) -> Self {
        Self {
            player_filter,
            counter_type,
            maximum,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for PlayerCounterPerTurnLimitReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayerCounterPerTurnLimitReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldPutCountersOrEnterWithCountersMatcher {
                ability_source: source,
                controller,
                filter: ObjectFilter::default(),
                player_filter: Some(self.player_filter.clone()),
                counter_type: Some(self.counter_type),
                actor: None,
                includes_permanents: false,
            },
            ReplacementAction::SetPlayerCountersAndLockForTurn {
                counter_type: self.counter_type,
                amount: self.maximum,
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleTokenCreationReplacement {
    pub controller: PlayerFilter,
    pub display: String,
}

impl DoubleTokenCreationReplacement {
    pub fn new(controller: PlayerFilter, display: impl Into<String>) -> Self {
        Self {
            controller,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DoubleTokenCreationReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoubleTokenCreationReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                self.controller.clone(),
            ),
            ReplacementAction::Double,
        ))
    }
}

/// "If one or more tokens would be created under your control, three times
/// that many of those tokens are created instead." (Ojer Taq)
#[derive(Debug, Clone, PartialEq)]
pub struct MultiplyTokenCreationReplacement {
    pub controller: PlayerFilter,
    pub token_filter: Option<ObjectFilter>,
    pub factor: u32,
    pub display: String,
}

impl MultiplyTokenCreationReplacement {
    pub fn new(
        controller: PlayerFilter,
        token_filter: Option<ObjectFilter>,
        factor: u32,
        display: impl Into<String>,
    ) -> Self {
        Self {
            controller,
            token_filter,
            factor,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for MultiplyTokenCreationReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MultiplyTokenCreationReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let mut matcher = crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
            self.controller.clone(),
        );
        if let Some(token_filter) = &self.token_filter {
            matcher = matcher.with_token_filter(token_filter.clone());
        }
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            matcher,
            ReplacementAction::Modify(EventModification::Multiply(self.factor)),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddTokenCreationReplacement {
    pub controller: PlayerFilter,
    pub token_filter: ObjectFilter,
    pub additional_token: ironsmith_core::AdditionalTokenKind,
    pub additional: i32,
    /// One additional token per token being created ("that many").
    pub per_created: bool,
    pub display: String,
}

impl AddTokenCreationReplacement {
    pub fn new(
        controller: PlayerFilter,
        token_filter: ObjectFilter,
        additional_token: ironsmith_core::AdditionalTokenKind,
        additional: i32,
        display: impl Into<String>,
    ) -> Self {
        Self {
            controller,
            token_filter,
            additional_token,
            additional,
            per_created: false,
            display: display.into(),
        }
    }

    pub fn per_created(mut self) -> Self {
        self.per_created = true;
        self
    }
}

impl StaticAbilityKind for AddTokenCreationReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddTokenCreationReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                self.controller.clone(),
            )
            .with_token_filter(self.token_filter.clone()),
            if self.per_created {
                ReplacementAction::AddTokensPerCreated {
                    token: self.additional_token,
                }
            } else {
                ReplacementAction::AddTokens {
                    token: self.additional_token,
                    count: self.additional.max(0) as u32,
                }
            },
        ))
    }
}

/// Can be your commander.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanBeCommander;

impl StaticAbilityKind for CanBeCommander {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBeCommander
    }

    fn display(&self) -> String {
        "Can be your commander".to_string()
    }
}

// =============================================================================
// Unified Grant System
// =============================================================================

/// Unified grant ability that grants abilities or alternative casting methods
/// to cards matching a filter in a specific zone.
///
/// This is the generic version that replaces bespoke types like `GrantEscape`
/// and `GrantFlashToNoncreatureSpells`. It provides a uniform way to express
/// "cards matching X in zone Y have Z".
///
/// # Examples
///
/// ```ignore
/// // Valley Floodcaller: "You may cast noncreature spells as though they had flash."
/// StaticAbility::grants(GrantSpec::flash_to_noncreature_spells())
///
/// // Underworld Breach: "Each nonland card in your graveyard has escape."
/// StaticAbility::grants(GrantSpec::escape_to_nonland(3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Grants {
    pub spec: GrantSpec,
    pub condition: Option<crate::ConditionExpr>,
}

impl Grants {
    /// Create a new Grants ability from a grant specification.
    pub fn new(spec: GrantSpec) -> Self {
        Self {
            spec,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for Grants {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Grants
    }

    fn display(&self) -> String {
        let mut text = self.spec.display();
        if let Some(condition) = &self.condition {
            let condition_text = super::super::describe_static_condition(condition);
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(
        &self,
        condition: crate::ConditionExpr,
    ) -> Option<super::StaticAbility> {
        Some(super::StaticAbility::new(
            self.clone().with_condition(condition),
        ))
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        super::super::static_condition_is_active(
            condition,
            game,
            source,
            game.controller_of(source_obj),
        )
    }

    fn grant_spec(&self) -> Option<GrantSpec> {
        Some(self.spec.clone())
    }
}

fn static_condition_is_during_your_turn(condition: &crate::ConditionExpr) -> bool {
    matches!(
        condition,
        crate::ConditionExpr::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn)
    )
}

/// Level abilities for level-up creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAbilities {
    pub levels: Vec<LevelAbility>,
}

impl LevelAbilities {
    pub fn new(levels: Vec<LevelAbility>) -> Self {
        Self { levels }
    }
}

impl StaticAbilityKind for LevelAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LevelAbilities
    }

    fn display(&self) -> String {
        if self.levels.is_empty() {
            return "Level up abilities".to_string();
        }

        let rendered_levels = self
            .levels
            .iter()
            .map(|level| {
                let range = match level.max_level {
                    Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                    Some(max) => format!("Level {}-{}", level.min_level, max),
                    None => format!("Level {}+", level.min_level),
                };
                let mut details = Vec::new();
                if let Some((power, toughness)) = level.power_toughness {
                    details.push(format!("{power}/{toughness}"));
                }
                details.extend(level.abilities.iter().map(|ability| ability.display()));
                if details.is_empty() {
                    range
                } else {
                    format!("{range}: {}", details.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ");

        format!("Level up abilities ({rendered_levels})")
    }

    fn level_abilities(&self) -> Option<&[LevelAbility]> {
        Some(&self.levels)
    }
}

/// "You have no maximum hand size"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoMaximumHandSize;

impl StaticAbilityKind for NoMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::NoMaximumHandSize
    }

    fn display(&self) -> String {
        "You have no maximum hand size".to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        if let Some(player) = game.player_mut(controller) {
            player.max_hand_size = i32::MAX;
        }
    }
}

/// "Your/Each opponent's maximum hand size is N."
#[derive(Debug, Clone, PartialEq)]
pub struct SetMaximumHandSize {
    pub player: PlayerFilter,
    pub amount: u32,
}

impl SetMaximumHandSize {
    pub fn new(player: PlayerFilter, amount: u32) -> Self {
        Self { player, amount }
    }
}

impl StaticAbilityKind for SetMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetMaximumHandSize
    }

    fn display(&self) -> String {
        let amount = number_word_u32(self.amount).unwrap_or_else(|| self.amount.to_string());
        match self.player {
            PlayerFilter::You => format!("Your maximum hand size is {amount}."),
            PlayerFilter::Opponent => {
                format!("Each opponent's maximum hand size is {amount}.")
            }
            PlayerFilter::Any => format!("Each player's maximum hand size is {amount}."),
            _ => format!("Maximum hand size is {amount}."),
        }
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        for player_id in player_ids_for_filter(game, self.player.clone(), controller) {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = self.amount as i32;
            }
        }
    }
}

/// "Your/Each opponent's maximum hand size is reduced by N."
#[derive(Debug, Clone, PartialEq)]
pub struct ReduceMaximumHandSize {
    pub player: PlayerFilter,
    pub amount: u32,
}

impl ReduceMaximumHandSize {
    pub fn new(player: PlayerFilter, amount: u32) -> Self {
        Self { player, amount }
    }
}

impl StaticAbilityKind for ReduceMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ReduceMaximumHandSize
    }

    fn display(&self) -> String {
        match self.player {
            PlayerFilter::You => {
                format!("Your maximum hand size is reduced by {}.", self.amount)
            }
            PlayerFilter::Opponent => {
                format!(
                    "Each opponent's maximum hand size is reduced by {}.",
                    self.amount
                )
            }
            PlayerFilter::Any => {
                format!(
                    "Each player's maximum hand size is reduced by {}.",
                    self.amount
                )
            }
            _ => format!("Maximum hand size is reduced by {}.", self.amount),
        }
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        use crate::game_loop::player_matches_filter_with_combat;

        let combat = game.combat.as_ref();
        let affected: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game()
                    && player_matches_filter_with_combat(
                        player.id,
                        &self.player,
                        game,
                        controller,
                        combat,
                    )
            })
            .map(|player| player.id)
            .collect();

        let reduction = self.amount as i32;
        for player_id in affected {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = player.max_hand_size.saturating_sub(reduction);
            }
        }
    }
}

/// "Your/Each opponent's maximum hand size is increased by N."
#[derive(Debug, Clone, PartialEq)]
pub struct IncreaseMaximumHandSize {
    pub player: PlayerFilter,
    pub amount: u32,
}

impl IncreaseMaximumHandSize {
    pub fn new(player: PlayerFilter, amount: u32) -> Self {
        Self { player, amount }
    }
}

impl StaticAbilityKind for IncreaseMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::IncreaseMaximumHandSize
    }

    fn display(&self) -> String {
        let amount = number_word_u32(self.amount).unwrap_or_else(|| self.amount.to_string());
        match self.player {
            PlayerFilter::You => format!("Your maximum hand size is increased by {amount}."),
            PlayerFilter::Opponent => {
                format!("Each opponent's maximum hand size is increased by {amount}.")
            }
            PlayerFilter::Any => {
                format!("Each player's maximum hand size is increased by {amount}.")
            }
            _ => format!("Maximum hand size is increased by {amount}."),
        }
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        use crate::game_loop::player_matches_filter_with_combat;

        let combat = game.combat.as_ref();
        let affected: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game()
                    && player_matches_filter_with_combat(
                        player.id,
                        &self.player,
                        game,
                        controller,
                        combat,
                    )
            })
            .map(|player| player.id)
            .collect();

        let increase = self.amount as i32;
        for player_id in affected {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = player.max_hand_size.saturating_add(increase);
            }
        }
    }
}

fn player_ids_for_filter(
    game: &GameState,
    player_filter: PlayerFilter,
    controller: PlayerId,
) -> Vec<PlayerId> {
    use crate::game_loop::player_matches_filter_with_combat;

    let combat = game.combat.as_ref();
    game.players
        .iter()
        .filter(|player| {
            player.is_in_game()
                && player_matches_filter_with_combat(
                    player.id,
                    &player_filter,
                    game,
                    controller,
                    combat,
                )
        })
        .map(|player| player.id)
        .collect()
}

fn count_distinct_card_types_in_graveyard(game: &GameState, player_id: PlayerId) -> i32 {
    use crate::types::CardType;

    let mut types: Vec<CardType> = Vec::new();
    let Some(player) = game.player(player_id) else {
        return 0;
    };
    for &card_id in &player.graveyard {
        let Some(obj) = game.object(card_id) else {
            continue;
        };
        for card_type in &obj.card_types {
            if !types.contains(card_type) {
                types.push(*card_type);
            }
        }
    }
    types.len() as i32
}

fn count_distinct_mana_values_in_graveyard(game: &GameState, player_id: PlayerId) -> i32 {
    let Some(player) = game.player(player_id) else {
        return 0;
    };

    let mut values: Vec<u32> = Vec::new();
    for &card_id in &player.graveyard {
        let Some(obj) = game.object(card_id) else {
            continue;
        };
        let mana_value = obj.mana_cost.as_ref().map_or(0, |cost| cost.mana_value());
        if !values.contains(&mana_value) {
            values.push(mana_value);
        }
    }
    values.len() as i32
}

pub(crate) fn conditional_spell_keyword_active(
    spec: ConditionalSpellKeywordSpec,
    game: &GameState,
    controller: PlayerId,
) -> bool {
    let count = match spec.metric {
        GraveyardCountMetric::CardTypes => count_distinct_card_types_in_graveyard(game, controller),
        GraveyardCountMetric::ManaValues => {
            count_distinct_mana_values_in_graveyard(game, controller)
        }
    };
    count >= spec.threshold as i32
}

/// "This spell has flash/cascade as long as there are N or more ... in your graveyard."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSpellKeyword {
    pub spec: ConditionalSpellKeywordSpec,
}

impl ConditionalSpellKeyword {
    pub const fn new(spec: ConditionalSpellKeywordSpec) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for ConditionalSpellKeyword {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ConditionalSpellKeyword
    }

    fn display(&self) -> String {
        let keyword = match self.spec.keyword {
            ConditionalSpellKeywordKind::Flash => "flash",
            ConditionalSpellKeywordKind::Cascade => "cascade",
        };
        let metric = match self.spec.metric {
            GraveyardCountMetric::CardTypes => "card types",
            GraveyardCountMetric::ManaValues => "mana values",
        };
        let threshold =
            number_word_u32(self.spec.threshold).unwrap_or_else(|| self.spec.threshold.to_string());
        format!(
            "This spell has {keyword} as long as there are {threshold} or more {metric} among cards in your graveyard."
        )
    }

    fn conditional_spell_keyword_spec(&self) -> Option<ConditionalSpellKeywordSpec> {
        Some(self.spec)
    }
}

/// "Cast this spell only ..." cast-time restriction.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCastRestriction {
    pub kind: ThisSpellCastRestrictionKind,
    pub display: String,
}

impl ThisSpellCastRestriction {
    pub fn new(kind: ThisSpellCastRestrictionKind, display: impl Into<String>) -> Self {
        Self {
            kind,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ThisSpellCastRestriction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellCastRestriction
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn this_spell_cast_restriction_kind(&self) -> Option<ThisSpellCastRestrictionKind> {
        Some(self.kind.clone())
    }
}

/// "X can't be greater than ..." spell-casting X restriction.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellXMaximum {
    pub maximum: crate::effect::Value,
    pub display: String,
}

impl ThisSpellXMaximum {
    pub fn new(maximum: crate::effect::Value, display: impl Into<String>) -> Self {
        Self {
            maximum,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ThisSpellXMaximum {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellXMaximum
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn this_spell_x_maximum_value(&self) -> Option<crate::effect::Value> {
        Some(self.maximum.clone())
    }
}

/// "X can't be less than ..." spell-casting X restriction.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellXMinimum {
    pub minimum: crate::effect::Value,
    pub display: String,
}

impl ThisSpellXMinimum {
    pub fn new(minimum: crate::effect::Value, display: impl Into<String>) -> Self {
        Self {
            minimum,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ThisSpellXMinimum {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellXMinimum
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn this_spell_x_minimum_value(&self) -> Option<crate::effect::Value> {
        Some(self.minimum.clone())
    }
}

/// "Each opponent's maximum hand size is equal to seven minus the number of card types in your graveyard."
#[derive(Debug, Clone, PartialEq)]
pub struct MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    pub player: PlayerFilter,
    pub minimum_types: u32,
}

impl MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    pub const fn new(player: PlayerFilter, minimum_types: u32) -> Self {
        Self {
            player,
            minimum_types,
        }
    }
}

impl StaticAbilityKind for MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MaximumHandSizeSevenMinusYourGraveyardCardTypes
    }

    fn display(&self) -> String {
        let who = match self.player {
            PlayerFilter::You => "Your",
            PlayerFilter::Opponent => "Each opponent's",
            PlayerFilter::Any => "Each player's",
            _ => "Affected players'",
        };
        format!(
            "As long as there are {} or more card types among cards in your graveyard, {who} maximum hand size is equal to seven minus the number of those card types.",
            self.minimum_types
        )
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let card_types = count_distinct_card_types_in_graveyard(game, controller);
        if card_types < self.minimum_types as i32 {
            return;
        }

        let max_hand_size = (7 - card_types).max(0);
        let affected = player_ids_for_filter(game, self.player.clone(), controller);
        for player_id in affected {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = max_hand_size;
            }
        }
    }
}

/// Replacement for effect-caused discards moving to the top of the library.
///
/// "If an effect causes you to discard a card, you may put it on top of
/// your library instead of into your graveyard."
///
/// Key rules:
/// - Only applies to discards from effects (not costs)
/// - Uses the composable EventCause system to filter on cause type
/// - Offers an interactive choice between graveyard and library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectDiscardToLibraryReplacement;

impl StaticAbilityKind for EffectDiscardToLibraryReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EffectDiscardToLibraryReplacement
    }

    fn display(&self) -> String {
        "If an effect causes you to discard a card, you may put it on top of your library instead"
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            // Use the composable matcher that filters on cause type
            WouldDiscardMatcher::you_from_effect(),
            ReplacementAction::InteractiveChooseDestination {
                destinations: vec![Zone::Graveyard, Zone::Library],
                description: "Put discarded card on top of library instead of graveyard?"
                    .to_string(),
            },
        ))
    }
}

/// Dredge N — while this card is in its owner's graveyard, its owner may
/// replace one draw by milling exactly N cards and returning this card to hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DredgeAbility {
    pub amount: u32,
}

impl DredgeAbility {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, Copy)]
struct DredgeDrawMatcher {
    amount: u32,
}

impl ReplacementMatcher for DredgeDrawMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::EventContext,
    ) -> bool {
        if event.event_kind() != EventKind::Draw {
            return false;
        }
        let Some(draw) = downcast_event::<crate::events::cards::DrawEvent>(event) else {
            return false;
        };
        let Some(source) = ctx.source.and_then(|source| ctx.game.object(source)) else {
            return false;
        };

        draw.count == 1
            && source.zone == Zone::Graveyard
            && source.owner == draw.player
            && ctx
                .game
                .player(draw.player)
                .is_some_and(|player| player.library.len() >= self.amount as usize)
    }

    fn display(&self) -> String {
        format!("Dredge {}", self.amount)
    }
}

impl StaticAbilityKind for DredgeAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Dredge
    }

    fn display(&self) -> String {
        format!("Dredge {}", self.amount)
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                DredgeDrawMatcher {
                    amount: self.amount,
                },
                ReplacementAction::Instead(vec![
                    Effect::mill(self.amount),
                    Effect::move_to_zone(ChooseSpec::Source, Zone::Hand, false),
                ]),
            )
            .optional(),
        )
    }
}

/// Replacement for opponent-controlled effects causing this card to be discarded.
///
/// "If a spell or ability an opponent controls causes you to discard this card,
/// put it onto the battlefield instead of putting it into your graveyard."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpponentEffectDiscardThisToBattlefieldReplacement;

impl StaticAbilityKind for OpponentEffectDiscardThisToBattlefieldReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentEffectDiscardThisToBattlefieldReplacement
    }

    fn display(&self) -> String {
        "If a spell or ability an opponent controls causes you to discard this card, put it onto \
         the battlefield instead of putting it into your graveyard"
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDiscardMatcher::source_from_opponent_effect(),
            ReplacementAction::ChangeDestination(Zone::Battlefield),
        ))
    }
}

/// "If you would draw a card, exile the top card of your library face down instead."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawReplacementExileTopFaceDown;

impl StaticAbilityKind for DrawReplacementExileTopFaceDown {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementExileTopFaceDown
    }

    fn display(&self) -> String {
        "If you would draw a card, exile the top card of your library face down instead."
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        const TOP_CARD_TAG: &str = "draw_replacement_top_card";

        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        1,
                        PlayerFilter::You,
                        TOP_CARD_TAG,
                    )
                    .top_only(),
                ),
                Effect::new(
                    crate::effects::ExileEffect::with_spec(ChooseSpec::tagged(TOP_CARD_TAG))
                        .with_face_down(true),
                ),
            ]),
        ))
    }
}

/// "If you would draw a card, draw two cards instead."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawReplacementDouble;

impl StaticAbilityKind for DrawReplacementDouble {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementDouble
    }

    fn display(&self) -> String {
        "If you would draw a card, draw two cards instead.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![Effect::new(crate::effects::DrawCardsEffect::you(2))]),
        ))
    }
}

/// "If you would draw a card while your library has no cards in it, skip that draw instead."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawReplacementSkipEmptyLibrary;

impl StaticAbilityKind for DrawReplacementSkipEmptyLibrary {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementSkipEmptyLibrary
    }

    fn display(&self) -> String {
        "If you would draw a card while your library has no cards in it, skip that draw instead."
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardWhileLibraryEmptyMatcher::you(),
            ReplacementAction::Skip,
        ))
    }
}

/// "If you would draw a card while [condition], [effects] instead."
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalDrawReplacement {
    pub condition: Condition,
    pub replacement_effects: Vec<Effect>,
    pub optional: bool,
    pub display: String,
}

impl ConditionalDrawReplacement {
    pub fn new(
        condition: Condition,
        replacement_effects: Vec<Effect>,
        optional: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            condition,
            replacement_effects,
            optional,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ConditionalDrawReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ConditionalDrawReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        let mut combined = self.clone();
        combined.condition = Condition::And(Box::new(condition), Box::new(combined.condition));
        // Some authored optional draw replacements already carry the complete
        // leading static condition in their typed display (for example
        // "As long as this enchantment has six or more ...").  Re-wrapping
        // those must strengthen executable matching without appending a
        // second debug-style condition sentence.
        if !combined
            .display
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("as long as ")
        {
            combined.display = format!(
                "{} {}",
                combined.display,
                super::super::describe_static_condition(&combined.condition)
            );
        }
        Some(StaticAbility::new(combined))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            ConditionalWouldDrawCardMatcher {
                condition: self.condition.clone(),
                display: self.display.clone(),
            },
            ReplacementAction::Instead(self.replacement_effects.clone()),
        );
        Some(if self.optional {
            replacement.optional()
        } else {
            replacement
        })
    }
}

/// "If you would lose the game, instead [effects]."
#[derive(Debug, Clone, PartialEq)]
pub struct LoseGameReplacement {
    pub replacement_effects: Vec<Effect>,
    pub optional: bool,
    pub display: String,
}

impl LoseGameReplacement {
    pub fn new(
        replacement_effects: Vec<Effect>,
        optional: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            replacement_effects,
            optional,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for LoseGameReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LoseGameReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::other::WouldLoseGameMatcher,
            ReplacementAction::Instead(self.replacement_effects.clone()),
        );
        Some(if self.optional {
            replacement.optional()
        } else {
            replacement
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ConditionalWouldDrawCardMatcher {
    condition: Condition,
    display: String,
}

impl ReplacementMatcher for ConditionalWouldDrawCardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if !WouldDrawCardMatcher::you().matches_event(event, ctx) {
            return false;
        }

        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        crate::condition_eval::evaluate_condition_external(ctx.game, &self.condition, &eval_ctx)
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}

/// "If you would draw one or more cards, you draw that many cards plus one
/// instead." (Quantum Riddler). The extra cards attach to the first card of
/// each draw instruction; a leading "as long as" condition gates it.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawExtraCardsReplacement {
    pub condition: Option<Condition>,
    pub extra: u32,
    /// Skip the first card drawn in each of the player's own draw steps.
    pub except_first_of_draw_step: bool,
    /// Apply once per draw instruction rather than to every card drawn.
    pub per_instruction: bool,
    pub display: String,
}

impl DrawExtraCardsReplacement {
    pub fn new(extra: u32, display: impl Into<String>) -> Self {
        Self {
            condition: None,
            extra,
            except_first_of_draw_step: false,
            per_instruction: true,
            display: display.into(),
        }
    }

    pub fn except_first_of_draw_step(mut self) -> Self {
        self.except_first_of_draw_step = true;
        self
    }

    pub fn per_card(mut self) -> Self {
        self.per_instruction = false;
        self
    }
}

impl StaticAbilityKind for DrawExtraCardsReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawExtraCardsReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        let mut combined = self.clone();
        combined.condition = Some(match combined.condition.take() {
            Some(existing) => Condition::And(Box::new(condition), Box::new(existing)),
            None => condition,
        });
        Some(StaticAbility::new(combined))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawInstructionMatcher {
                condition: self.condition.clone(),
                except_first_of_draw_step: self.except_first_of_draw_step,
                per_instruction: self.per_instruction,
                display: self.display.clone(),
            },
            ReplacementAction::Modify(crate::replacement::EventModification::Add(
                i32::try_from(self.extra).unwrap_or(i32::MAX),
            )),
        ))
    }
}

/// The first card of one of your draw instructions, optionally gated by a
/// static condition.
#[derive(Debug, Clone)]
struct WouldDrawInstructionMatcher {
    condition: Option<Condition>,
    except_first_of_draw_step: bool,
    per_instruction: bool,
    display: String,
}

impl ReplacementMatcher for WouldDrawInstructionMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if !WouldDrawCardMatcher::you().matches_event(event, ctx) {
            return false;
        }
        let Some(draw) = crate::events::downcast_event::<crate::events::DrawEvent>(event) else {
            return false;
        };
        if (self.per_instruction && !draw.first_of_instruction)
            || (self.except_first_of_draw_step && draw.first_of_draw_step)
        {
            return false;
        }
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(ctx.game, condition, &eval_ctx)
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}

/// "If you would create a Clue, Food, or Treasure token, instead create one of
/// each." (Academy Manufactor)
#[derive(Debug, Clone, PartialEq)]
pub struct CreateOneOfEachTokenReplacement {
    pub kinds: Vec<ironsmith_core::AdditionalTokenKind>,
    pub display: String,
}

impl CreateOneOfEachTokenReplacement {
    pub fn new(kinds: Vec<ironsmith_core::AdditionalTokenKind>, display: impl Into<String>) -> Self {
        Self {
            kinds,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for CreateOneOfEachTokenReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CreateOneOfEachTokenReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let mut token_filter = ObjectFilter::default();
        token_filter.any_of = self
            .kinds
            .iter()
            .map(|kind| {
                let mut branch = ObjectFilter::default();
                branch.subtypes.push(match kind {
                    ironsmith_core::AdditionalTokenKind::Treasure => crate::types::Subtype::Treasure,
                    ironsmith_core::AdditionalTokenKind::Food => crate::types::Subtype::Food,
                    ironsmith_core::AdditionalTokenKind::Squirrel => crate::types::Subtype::Squirrel,
                    ironsmith_core::AdditionalTokenKind::Clue => crate::types::Subtype::Clue,
                });
                branch
            })
            .collect();
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                PlayerFilter::You,
            )
            .with_token_filter(token_filter),
            ReplacementAction::AddTokensOfOtherKinds {
                kinds: self.kinds.clone(),
            },
        ))
    }
}

/// "If an opponent would draw a card except the first one they draw in each
/// of their draw steps, instead that player skips that draw and you draw a
/// card." (Notion Thief)
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectDrawReplacement {
    pub drawer: PlayerFilter,
    pub except_first_of_draw_step: bool,
    pub display: String,
}

impl RedirectDrawReplacement {
    pub fn new(drawer: PlayerFilter, except_first_of_draw_step: bool, display: impl Into<String>) -> Self {
        Self {
            drawer,
            except_first_of_draw_step,
            display: display.into(),
        }
    }
}

/// "If you would draw a card, instead <effects>." (Underrealm Lich)
#[derive(Debug, Clone, PartialEq)]
pub struct DrawReplacementWithEffects {
    pub drawer: PlayerFilter,
    pub replacement_effects: Vec<Effect>,
    pub display: String,
}

impl DrawReplacementWithEffects {
    pub fn new(
        drawer: PlayerFilter,
        replacement_effects: Vec<Effect>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            drawer,
            replacement_effects,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DrawReplacementWithEffects {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementWithEffects
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawByPlayerMatcher {
                drawer: self.drawer.clone(),
                except_first_of_draw_step: false,
                display: self.display.clone(),
            },
            ReplacementAction::Instead(self.replacement_effects.clone()),
        ))
    }
}

impl StaticAbilityKind for RedirectDrawReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RedirectDrawReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawByPlayerMatcher {
                drawer: self.drawer.clone(),
                except_first_of_draw_step: self.except_first_of_draw_step,
                display: self.display.clone(),
            },
            ReplacementAction::RedirectDrawToController,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct WouldDrawByPlayerMatcher {
    drawer: PlayerFilter,
    except_first_of_draw_step: bool,
    display: String,
}

impl ReplacementMatcher for WouldDrawByPlayerMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if !WouldDrawCardMatcher::new(self.drawer.clone()).matches_event(event, ctx) {
            return false;
        }
        let Some(draw) = crate::events::downcast_event::<crate::events::DrawEvent>(event) else {
            return false;
        };
        !(self.except_first_of_draw_step && draw.first_of_draw_step)
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}

/// "If you would draw a card, exile the top N cards of your library instead. You may play those
/// cards this turn."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawReplacementExileTopAndPlay {
    pub count: u32,
}

impl DrawReplacementExileTopAndPlay {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

impl StaticAbilityKind for DrawReplacementExileTopAndPlay {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementExileTopAndPlay
    }

    fn display(&self) -> String {
        let cards = if self.count == 1 { "card" } else { "cards" };
        format!(
            "If you would draw a card, exile the top {} {} of your library instead. You may play those cards this turn.",
            self.count, cards
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        const TOP_CARDS_TAG: &str = "draw_replacement_top_cards";

        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        self.count as usize,
                        PlayerFilter::You,
                        TOP_CARDS_TAG,
                    )
                    .top_only(),
                ),
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::tagged(
                    TOP_CARDS_TAG,
                ))),
                Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    TOP_CARDS_TAG,
                    PlayerFilter::You,
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                    true,
                    false,
                )),
            ]),
        ))
    }
}

/// "If you would draw a card, instead reveal the top N cards of your library. Put all matching
/// cards revealed this way into your hand and the rest on the bottom of your library."
#[derive(Debug, Clone, PartialEq)]
pub struct DrawReplacementRevealTopMatchingToHandRestBottom {
    pub count: u32,
    pub filter: ObjectFilter,
    pub order: crate::effects::consult_helpers::LibraryBottomOrder,
    pub display: String,
}

impl DrawReplacementRevealTopMatchingToHandRestBottom {
    pub fn new(
        count: u32,
        filter: ObjectFilter,
        order: crate::effects::consult_helpers::LibraryBottomOrder,
        display: impl Into<String>,
    ) -> Self {
        Self {
            count,
            filter,
            order,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DrawReplacementRevealTopMatchingToHandRestBottom {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementRevealTopMatchingToHandRestBottom
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        const REVEALED_TAG: &str = "draw_replacement_revealed";
        const MATCHED_TAG: &str = "draw_replacement_matched";

        let mut matching_filter = self.filter.clone();
        matching_filter.zone = None;
        matching_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(REVEALED_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });

        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![
                Effect::reveal_top_cards(
                    PlayerFilter::You,
                    Value::Fixed(self.count as i32),
                    TagKey::from(REVEALED_TAG),
                ),
                Effect::new(
                    crate::effects::TagMatchingObjectsEffect::new(matching_filter, MATCHED_TAG)
                        .in_zones(vec![Zone::Library]),
                ),
                Effect::for_each_tagged(
                    MATCHED_TAG,
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                ),
                Effect::put_tagged_remainder_on_library_bottom(
                    TagKey::from(REVEALED_TAG),
                    Some(TagKey::from(MATCHED_TAG)),
                    self.order,
                    PlayerFilter::You,
                ),
            ]),
        ))
    }
}

/// "If [object] would [keyword action], instead [effects]."
#[derive(Debug, Clone, PartialEq)]
pub struct KeywordActionReplacement {
    pub action: crate::events::KeywordActionKind,
    pub source_filter: ObjectFilter,
    pub performer_filter: Option<PlayerFilter>,
    pub replacement_effects: Vec<Effect>,
    pub optional: bool,
    pub display: String,
}

impl KeywordActionReplacement {
    pub fn new(
        action: crate::events::KeywordActionKind,
        source_filter: ObjectFilter,
        performer_filter: Option<PlayerFilter>,
        replacement_effects: Vec<Effect>,
        optional: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            action,
            source_filter,
            performer_filter,
            replacement_effects,
            optional,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for KeywordActionReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordActionReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::other::WouldKeywordActionMatcher::new(
                self.action,
                self.source_filter.clone(),
            )
            .with_performer_filter(self.performer_filter.clone()),
            ReplacementAction::Instead(self.replacement_effects.clone()),
        );
        Some(if self.optional {
            replacement.optional()
        } else {
            replacement
        })
    }
}

/// "If a card would be put into an opponent's graveyard from anywhere, instead exile it with a
/// void counter on it."
#[derive(Debug, Clone, PartialEq)]
pub struct ExileToCounteredExileInsteadOfGraveyard {
    pub player: PlayerFilter,
    pub counter_type: CounterType,
}

impl ExileToCounteredExileInsteadOfGraveyard {
    pub fn new(player: PlayerFilter, counter_type: CounterType) -> Self {
        Self {
            player,
            counter_type,
        }
    }

    fn graveyard_owner_phrase(&self) -> &'static str {
        match self.player {
            PlayerFilter::You => "your",
            PlayerFilter::Opponent => "an opponent's",
            _ => "a player's",
        }
    }
}

impl StaticAbilityKind for ExileToCounteredExileInsteadOfGraveyard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
    }

    fn display(&self) -> String {
        let counter = self.counter_type.description().into_owned();
        format!(
            "If a card would be put into {} graveyard from anywhere, instead exile it with a {} counter on it.",
            self.graveyard_owner_phrase(),
            counter
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldGoToGraveyardFromAnywhereMatcher::new(
                ObjectFilter::default().owned_by(self.player.clone()),
                false,
            ),
            ReplacementAction::ExileWithSourceLinkCountersThen {
                counters: vec![(self.counter_type, 1)],
                effects: Vec::new(),
            },
        ))
    }
}

/// "If [objects] would be put into a graveyard from anywhere, exile them instead."
#[derive(Debug, Clone, PartialEq)]
pub struct ExileToExileInsteadOfGraveyard {
    pub filter: ObjectFilter,
    pub graveyard_owner: PlayerFilter,
    pub exclude_cycled: bool,
}

impl ExileToExileInsteadOfGraveyard {
    pub fn new(filter: ObjectFilter, graveyard_owner: PlayerFilter) -> Self {
        Self {
            filter,
            graveyard_owner,
            exclude_cycled: false,
        }
    }

    pub fn unless_cycled(filter: ObjectFilter, graveyard_owner: PlayerFilter) -> Self {
        Self {
            filter,
            graveyard_owner,
            exclude_cycled: true,
        }
    }

    fn graveyard_owner_phrase(&self) -> &'static str {
        match self.graveyard_owner {
            PlayerFilter::You => "your",
            PlayerFilter::Opponent => "an opponent's",
            _ => "a",
        }
    }

    fn replacement_subject_phrase(&self) -> String {
        if self.filter.has_explicit_card_noun()
            && let [marker] = self.filter.ability_markers.as_slice()
        {
            let mut normalized = self.filter.clone();
            normalized.ability_markers.clear();
            if normalized == ObjectFilter::default() {
                let marker = marker.to_ascii_lowercase();
                let article = match marker.chars().next() {
                    Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
                    _ => "a",
                };
                return format!("a card that has {article} {marker} ability");
            }
        }
        self.filter.description()
    }
}

impl StaticAbilityKind for ExileToExileInsteadOfGraveyard {
    fn is_source_only_graveyard_replacement(&self) -> bool { self.filter.source }

    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExileToExileInsteadOfGraveyard
    }

    fn display(&self) -> String {
        let cycled_clause = if self.exclude_cycled {
            " and it wasn't cycled"
        } else {
            ""
        };
        format!(
            "If {} would be put into {} graveyard from anywhere{}, exile it instead.",
            self.replacement_subject_phrase(),
            self.graveyard_owner_phrase(),
            cycled_clause
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let mut filter = self.filter.clone();
        if filter.owner.is_none() {
            filter.owner = Some(self.graveyard_owner.clone());
        }
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldGoToGraveyardFromAnywhereMatcher::new(filter, self.exclude_cycled),
            ReplacementAction::ChangeDestination(Zone::Exile),
        ))
    }
}

#[derive(Debug, Clone)]
struct WouldGoToGraveyardFromAnywhereMatcher {
    filter: ObjectFilter,
    exclude_cycled: bool,
}

impl WouldGoToGraveyardFromAnywhereMatcher {
    fn new(filter: ObjectFilter, exclude_cycled: bool) -> Self {
        Self {
            filter,
            exclude_cycled,
        }
    }

    fn is_excluded_cycled_discard(
        &self,
        card: ObjectId,
        cause: &crate::events::cause::EventCause,
        ctx: &EventContext,
    ) -> bool {
        if !self.exclude_cycled {
            return false;
        }
        if cause.cause_type != CauseType::Cost || cause.source != Some(card) {
            return false;
        }
        let cycling_filter = ObjectFilter::default().with_ability_marker("cycling");
        if let Some(snapshot) = ctx
            .event_source_snapshot
            .filter(|snapshot| snapshot.object_id == card)
        {
            return cycling_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }
        ctx.game
            .object(card)
            .is_some_and(|obj| cycling_filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }
}

impl ReplacementMatcher for WouldGoToGraveyardFromAnywhereMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        match event.event_kind() {
            EventKind::Discard => {
                let Some(discard) = downcast_event::<DiscardEvent>(event) else {
                    return false;
                };
                if discard.destination != Zone::Graveyard {
                    return false;
                }
                if self.is_excluded_cycled_discard(discard.card, &discard.cause, ctx) {
                    return false;
                }
                ctx.game
                    .object(discard.card)
                    .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
            }
            EventKind::ZoneChange => {
                let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                    return false;
                };
                if zone_change.to != Zone::Graveyard {
                    return false;
                }
                if zone_change.objects.first().is_some_and(|card| {
                    self.is_excluded_cycled_discard(*card, &zone_change.cause, ctx)
                }) {
                    return false;
                }
                if let Some(snapshot) = zone_change.snapshot.as_ref().or(ctx.event_source_snapshot)
                {
                    let mut filter_ctx = ctx.filter_ctx.clone();
                    filter_ctx.caster.get_or_insert(snapshot.controller);
                    return self
                        .filter
                        .matches_snapshot(snapshot, &filter_ctx, ctx.game);
                }
                zone_change
                    .objects
                    .first()
                    .and_then(|id| ctx.game.object(*id))
                    .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        "If an object would be put into a graveyard from anywhere".to_string()
    }
}

/// "If [objects] would die, exile them instead."
#[derive(Debug, Clone, PartialEq)]
pub struct ExileWouldDieInstead {
    pub filter: ObjectFilter,
    pub damaged_by: Option<DamagedBySource>,
    pub damager_filter: Option<ObjectFilter>,
    pub damager_filter_surface: Option<String>,
    pub exile_with_counters: Vec<(CounterType, u32)>,
    pub follow_up_effects: Vec<Effect>,
}

impl ExileWouldDieInstead {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_by: None,
            damager_filter: None,
            damager_filter_surface: None,
            exile_with_counters: Vec::new(),
            follow_up_effects: Vec::new(),
        }
    }

    pub fn damaged_by(filter: ObjectFilter, damaged_by: DamagedBySource) -> Self {
        Self {
            filter,
            damaged_by: Some(damaged_by),
            damager_filter: None,
            damager_filter_surface: None,
            exile_with_counters: Vec::new(),
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        follow_up_effects: Vec<Effect>,
    ) -> Self {
        Self::with_counters_and_follow_up(filter, damaged_by, Vec::new(), follow_up_effects)
    }

    pub fn damaged_by_filter(filter: ObjectFilter, damager_filter: ObjectFilter) -> Self {
        Self::damaged_by_filter_with_surface(filter, damager_filter, None)
    }

    pub fn damaged_by_filter_with_surface(
        filter: ObjectFilter,
        damager_filter: ObjectFilter,
        damager_filter_surface: Option<String>,
    ) -> Self {
        Self {
            filter,
            damaged_by: None,
            damager_filter: Some(damager_filter),
            damager_filter_surface,
            exile_with_counters: Vec::new(),
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_counters_and_follow_up(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        exile_with_counters: Vec<(CounterType, u32)>,
        follow_up_effects: Vec<Effect>,
    ) -> Self {
        Self {
            filter,
            damaged_by,
            damager_filter: None,
            damager_filter_surface: None,
            exile_with_counters,
            follow_up_effects,
        }
    }
}

fn is_simple_source_would_die_filter(filter: &ObjectFilter) -> bool {
    if !filter.source || filter.card_types.len() > 1 {
        return false;
    }

    let mut filter_without_type = filter.clone();
    filter_without_type.card_types.clear();
    filter_without_type == ObjectFilter::source()
}

fn with_indefinite_article(text: String) -> String {
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
    {
        return text;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|first| matches!(first, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {text}")
}

impl StaticAbilityKind for ExileWouldDieInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExileWouldDieInstead
    }

    fn exile_would_die_instead_spec(
        &self,
    ) -> Option<(
        &ObjectFilter,
        Option<DamagedBySource>,
        Option<&ObjectFilter>,
        &[(CounterType, u32)],
        &[Effect],
    )> {
        Some((
            &self.filter,
            self.damaged_by,
            self.damager_filter.as_ref(),
            &self.exile_with_counters,
            &self.follow_up_effects,
        ))
    }

    fn display(&self) -> String {
        let counter_suffix = if self.exile_with_counters.is_empty() {
            String::new()
        } else {
            let counter_phrases: Vec<String> = self
                .exile_with_counters
                .iter()
                .map(|(counter_type, count)| describe_counter_phrase(counter_type, *count))
                .collect();
            format!(" with {} on it", join_english(&counter_phrases))
        };
        if let Some(damager_filter) = &self.damager_filter {
            let source_text = self.damager_filter_surface.clone().unwrap_or_else(|| {
                let mut source_filter = damager_filter.clone();
                source_filter.zone = None;
                source_filter
                    .description()
                    .replace(" you control", " you controlled")
                    .replace(" an opponent controls", " an opponent controlled")
            });
            format!(
                "If {} dealt damage this turn by {} would die, exile it{} instead.",
                with_indefinite_article(self.filter.description()),
                source_text,
                counter_suffix
            )
        } else if let Some(damaged_by) = self.damaged_by {
            let source_text = match damaged_by {
                DamagedBySource::ThisCreature => "this creature",
                DamagedBySource::EquippedCreature => "equipped creature",
                DamagedBySource::EnchantedCreature => "enchanted creature",
            };
            format!(
                "If {} dealt damage by {} this turn would die, exile it{} instead.",
                self.filter.description(),
                source_text,
                counter_suffix
            )
        } else {
            format!(
                "If {} would die, exile it{} instead.",
                self.filter.description(),
                counter_suffix
            )
        }
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        if let Some(damager_filter) = &self.damager_filter {
            return Some(ReplacementEffect::with_matcher(
                source,
                controller,
                WouldDieDamagedByFilteredSourceThisTurnMatcher::new(
                    self.filter.clone(),
                    damager_filter.clone(),
                ),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: self.exile_with_counters.clone(),
                    effects: self.follow_up_effects.clone(),
                },
            ));
        }
        if let Some(damaged_by) = self.damaged_by {
            return Some(ReplacementEffect::with_matcher(
                source,
                controller,
                WouldDieDamagedBySourceThisTurnMatcher::new(self.filter.clone(), damaged_by),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: self.exile_with_counters.clone(),
                    effects: self.follow_up_effects.clone(),
                },
            ));
        }

        if is_simple_source_would_die_filter(&self.filter)
            && self.exile_with_counters.is_empty()
            && self.follow_up_effects.is_empty()
        {
            return Some(ReplacementEffect::exile_instead_of_dying(
                source, controller,
            ));
        }

        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                self.filter.clone(),
                Some(Zone::Battlefield),
                Some(Zone::Graveyard),
            ),
            ReplacementAction::ExileWithSourceLinkCountersThen {
                counters: self.exile_with_counters.clone(),
                effects: self.follow_up_effects.clone(),
            },
        ))
    }
}

// =============================================================================
// Interactive ETB Replacement Abilities (Unified System)
// =============================================================================

/// "You may discard a card matching [filter]. If you don't, put this into [zone]."
///
/// Used by: Mox Diamond (discard land or goes to graveyard)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardOrRedirectReplacement {
    /// Filter for cards that can be discarded to satisfy the replacement.
    pub filter: ObjectFilter,
    /// Where the permanent goes if no card is discarded.
    pub redirect_zone: Zone,
}

impl DiscardOrRedirectReplacement {
    /// Create a new discard-or-redirect replacement ability.
    pub fn new(filter: ObjectFilter, redirect_zone: Zone) -> Self {
        Self {
            filter,
            redirect_zone,
        }
    }
}

impl StaticAbilityKind for DiscardOrRedirectReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DiscardOrRedirectReplacement
    }

    fn display(&self) -> String {
        let discard_phrase = describe_discard_filter_card_phrase(&self.filter);
        let redirect_phrase = describe_redirect_zone_phrase(self.redirect_zone);
        format!(
            "If this would enter the battlefield, you may discard {} instead. If you do, put it onto the battlefield. If you don't, put it into {}.",
            discard_phrase, redirect_phrase
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::InteractiveDiscardOrRedirect {
                filter: self.filter.clone(),
                redirect_zone: self.redirect_zone,
            },
        ))
    }
}

/// "Sacrifice [count] permanents matching [filter], or put this into [zone]."
///
/// Used by the family of lands whose entry is replaced by sacrificing
/// untapped lands (Scorched Ruins, Lotus Vale, and related cards).
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeOrRedirectReplacement {
    pub filter: ObjectFilter,
    pub count: u32,
    pub redirect_zone: Zone,
}

impl SacrificeOrRedirectReplacement {
    pub fn new(filter: ObjectFilter, count: u32, redirect_zone: Zone) -> Self {
        Self {
            filter,
            count,
            redirect_zone,
        }
    }
}

impl StaticAbilityKind for SacrificeOrRedirectReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SacrificeOrRedirectReplacement
    }

    fn display(&self) -> String {
        let count =
            ironsmith_core::cardinal_word(self.count).unwrap_or_else(|| self.count.to_string());
        let sacrifice_subject = if self.count == 1 {
            self.filter.description()
        } else {
            pluralize_filter_description(&self.filter.description())
        };
        let redirect_phrase = describe_redirect_zone_phrase(self.redirect_zone);
        format!(
            "If this land would enter, sacrifice {} {} instead. If you do, put this land onto the battlefield. If you don't, put it into {}.",
            count, sacrifice_subject, redirect_phrase
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::InteractiveSacrificeOrRedirect {
                filter: self.filter.clone(),
                count: self.count,
                redirect_zone: self.redirect_zone,
            },
        ))
    }
}

/// "As this enters the battlefield, you may pay N life. If you don't, it enters tapped."
///
/// Used by: Shock lands (Godless Shrine, etc.), slow fetches (Vault of Champions, etc.)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayLifeOrEnterTappedReplacement {
    /// The amount of life to pay to enter untapped.
    pub life_cost: u32,
}

impl PayLifeOrEnterTappedReplacement {
    /// Create a new pay-life-or-enter-tapped replacement ability.
    pub fn new(life_cost: u32) -> Self {
        Self { life_cost }
    }
}

impl StaticAbilityKind for PayLifeOrEnterTappedReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PayLifeOrEnterTappedReplacement
    }

    fn display(&self) -> String {
        format!(
            "As this enters the battlefield, you may pay {} life. If you don't, it enters tapped.",
            self.life_cost
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::InteractivePayLifeOrEnterTapped {
                life_cost: self.life_cost,
            },
        ))
    }

    fn enters_tapped(&self) -> bool {
        // This is conditionally enters tapped, so we return false here
        // The actual tapped state is determined by the replacement effect
        false
    }
}

/// "As this land enters, you may reveal a <type> card from your hand. If you
/// don't, this land enters tapped." (Port Town, Frostboil Snarl, ...)
///
/// Interactive replacement effect on the unified replacement system, parallel
/// to [`PayLifeOrEnterTappedReplacement`].
#[derive(Debug, Clone, PartialEq)]
pub struct RevealCardOrEnterTappedReplacement {
    /// The hand card that may be revealed to enter untapped.
    pub filter: ObjectFilter,
    /// Authored subject phrase ("this land").
    pub subject: String,
    /// Authored tail subject phrase ("this land" or "it").
    pub tail_subject: String,
}

impl RevealCardOrEnterTappedReplacement {
    pub fn new(
        filter: ObjectFilter,
        subject: impl Into<String>,
        tail_subject: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            subject: subject.into(),
            tail_subject: tail_subject.into(),
        }
    }
}

impl StaticAbilityKind for RevealCardOrEnterTappedReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RevealCardOrEnterTappedReplacement
    }

    fn display(&self) -> String {
        format!(
            "As {} enters, you may reveal {} from your hand. If you don't, {} enters tapped.",
            self.subject,
            with_indefinite_article(self.filter.description()),
            self.tail_subject
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::InteractiveRevealCardOrEnterTapped {
                filter: self.filter.clone(),
            },
        ))
    }

    fn enters_tapped(&self) -> bool {
        // Conditionally enters tapped; the replacement effect decides.
        false
    }
}

/// "If a nontoken creature would enter and it wasn't cast, exile it instead."
/// (Containment Priest). Matching objects that would enter the battlefield
/// move to `destination` instead.
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectWouldEnterReplacement {
    pub filter: ObjectFilter,
    /// Only objects that are not entering from the stack (i.e. weren't cast).
    pub not_cast: bool,
    pub destination: Zone,
    pub display: String,
}

impl RedirectWouldEnterReplacement {
    pub fn new(
        filter: ObjectFilter,
        not_cast: bool,
        destination: Zone,
        display: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            not_cast,
            destination,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for RedirectWouldEnterReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RedirectWouldEnterReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterFromZoneMatcher {
                enter_matcher: crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    self.filter.clone(),
                ),
                not_cast: self.not_cast,
            },
            ReplacementAction::ChangeDestination(self.destination),
        ))
    }
}

/// A would-enter matcher that can additionally require the object not to be
/// entering from the stack ("and it wasn't cast").
#[derive(Debug, Clone)]
struct WouldEnterFromZoneMatcher {
    enter_matcher: crate::events::zones::matchers::WouldEnterBattlefieldMatcher,
    not_cast: bool,
}

impl WouldEnterFromZoneMatcher {
    fn origin_allowed(&self, event: &dyn GameEventType) -> bool {
        if !self.not_cast {
            return true;
        }
        let from = match event.event_kind() {
            EventKind::ZoneChange => {
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event)
                    .map(|zone_change| zone_change.from)
            }
            EventKind::EnterBattlefield => {
                crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event)
                    .map(|etb| etb.from)
            }
            _ => None,
        };
        from.is_some_and(|from| from != Zone::Stack)
    }
}

impl ReplacementMatcher for WouldEnterFromZoneMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        self.enter_matcher.matches_event(event, ctx) && self.origin_allowed(event)
    }

    fn priority(&self) -> ReplacementPriority {
        self.enter_matcher.priority()
    }

    fn display(&self) -> String {
        self.enter_matcher.display()
    }
}

/// "If a land is tapped for two or more mana, it produces {C} instead of any
/// other type and amount." (Damping Sphere)
#[derive(Debug, Clone, PartialEq)]
pub struct ManaProductionReplacement {
    pub source_filter: ObjectFilter,
    pub minimum_amount: u32,
    pub replacement_mana: Vec<crate::mana::ManaSymbol>,
    pub display: String,
}

impl ManaProductionReplacement {
    pub fn new(
        source_filter: ObjectFilter,
        minimum_amount: u32,
        replacement_mana: Vec<crate::mana::ManaSymbol>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            source_filter,
            minimum_amount,
            replacement_mana,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ManaProductionReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ManaProductionReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            TappedForMinimumManaMatcher {
                inner: crate::events::mana::matchers::ManaProducedBySourceMatcher::tapped_source_for_mana(
                    self.source_filter.clone(),
                ),
                minimum_amount: self.minimum_amount,
            },
            ReplacementAction::ReplaceManaExact(self.replacement_mana.clone()),
        ))
    }
}

/// "If you tap a permanent for mana, it produces three times as much of that
/// mana instead." (Nyxbloom Ancient, Mana Reflection)
#[derive(Debug, Clone, PartialEq)]
pub struct ManaProductionMultiplierReplacement {
    pub source_filter: ObjectFilter,
    pub factor: u32,
    pub display: String,
}

impl ManaProductionMultiplierReplacement {
    pub fn new(source_filter: ObjectFilter, factor: u32, display: impl Into<String>) -> Self {
        Self {
            source_filter,
            factor,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ManaProductionMultiplierReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ManaProductionMultiplierReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            TappedForMinimumManaMatcher {
                inner: crate::events::mana::matchers::ManaProducedBySourceMatcher::tapped_source_for_mana(
                    self.source_filter.clone(),
                ),
                minimum_amount: 1,
            },
            ReplacementAction::Modify(EventModification::Multiply(self.factor)),
        ))
    }
}

/// Mana a matching source is tapped for, when the event adds at least
/// `minimum_amount` mana.
#[derive(Debug, Clone)]
struct TappedForMinimumManaMatcher {
    inner: crate::events::mana::matchers::ManaProducedBySourceMatcher,
    minimum_amount: u32,
}

impl ReplacementMatcher for TappedForMinimumManaMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        self.inner.matches_event(event, ctx)
            && crate::events::downcast_event::<crate::events::ManaAddedEvent>(event)
                .is_some_and(|added| added.mana.len() >= self.minimum_amount as usize)
    }

    fn priority(&self) -> ReplacementPriority {
        self.inner.priority()
    }

    fn display(&self) -> String {
        self.inner.display()
    }
}

/// "If you would gain life, you gain twice that much life instead." (Boon
/// Reflection); `loss` selects the life-loss form (Bloodletter of Aclazotz).
/// A leading "as long as" / "during your turn" condition gates it.
#[derive(Debug, Clone, PartialEq)]
pub struct DoubleLifeChangeReplacement {
    pub player: PlayerFilter,
    pub loss: bool,
    pub condition: Option<Condition>,
    pub display: String,
}

impl DoubleLifeChangeReplacement {
    pub fn new(player: PlayerFilter, loss: bool, display: impl Into<String>) -> Self {
        Self {
            player,
            loss,
            condition: None,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DoubleLifeChangeReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoubleLifeChangeReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        let mut combined = self.clone();
        combined.condition = Some(match combined.condition.take() {
            Some(existing) => Condition::And(Box::new(condition), Box::new(existing)),
            None => condition,
        });
        Some(StaticAbility::new(combined))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ConditionalWouldChangeLifeMatcher {
                player: self.player.clone(),
                loss: self.loss,
                condition: self.condition.clone(),
                display: self.display.clone(),
            },
            ReplacementAction::Double,
        ))
    }
}

/// A life-gain or life-loss event for the matching player, optionally gated
/// by a static condition evaluated for the replacement's source.
#[derive(Debug, Clone)]
struct ConditionalWouldChangeLifeMatcher {
    player: PlayerFilter,
    loss: bool,
    condition: Option<Condition>,
    display: String,
}

impl ReplacementMatcher for ConditionalWouldChangeLifeMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        let matches_change = if self.loss {
            crate::events::life::matchers::WouldLoseLifeMatcher::new(self.player.clone())
                .matches_event(event, ctx)
        } else {
            crate::events::life::matchers::WouldGainLifeMatcher::new(self.player.clone())
                .matches_event(event, ctx)
        };
        if !matches_change {
            return false;
        }
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(ctx.game, condition, &eval_ctx)
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}

/// Parser-backed pregame action from opening hand.
#[derive(Debug, Clone, PartialEq)]
pub struct PregameAction {
    pub kind: crate::static_abilities::PregameActionKind,
    pub text: String,
    pub effects: Vec<crate::effect::Effect>,
}

impl PregameAction {
    pub fn new(kind: crate::static_abilities::PregameActionKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            effects: Vec::new(),
        }
    }

    pub fn with_effects(
        kind: crate::static_abilities::PregameActionKind,
        text: impl Into<String>,
        effects: Vec<crate::effect::Effect>,
    ) -> Self {
        Self {
            kind,
            text: text.into(),
            effects,
        }
    }
}

impl StaticAbilityKind for PregameAction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PregameAction
    }

    fn display(&self) -> String {
        match &self.kind {
            crate::static_abilities::PregameActionKind::BeginOnBattlefield(spec) => {
                render_begin_on_battlefield_pregame(spec)
            }
            crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount => {
                self.text.clone()
            }
            crate::static_abilities::PregameActionKind::ChooseColor => self.text.clone(),
            crate::static_abilities::PregameActionKind::RevealFromOpeningHand(spec) => {
                render_reveal_from_opening_hand_pregame(*spec, &self.effects)
            }
        }
    }

    fn pregame_action_kind(&self) -> Option<crate::static_abilities::PregameActionKind> {
        Some(self.kind.clone())
    }

    fn pregame_action_effects(&self) -> Option<&[crate::effect::Effect]> {
        Some(&self.effects)
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_lowercase().collect::<String>() + chars.as_str()
}

fn contextualize_delayed_spell_consequence(text: &str) -> String {
    for prefix in [
        "Counter it unless that object's controller pays",
        "Counter it unless that player pays",
        "Counter it unless they pay",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return format!("Counter that spell unless that player pays{rest}");
        }
    }
    text.to_string()
}

fn render_reveal_from_opening_hand_pregame(
    spec: crate::static_abilities::PregameRevealFromOpeningHandSpec,
    effects: &[crate::effect::Effect],
) -> String {
    let prefix = "You may reveal this card from your opening hand. If you do";
    let Some(schedule) = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
    else {
        return format!("{prefix}.");
    };

    let first_opponent_spell = schedule
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .is_some_and(|trigger| {
            trigger.first_spell_of_game && trigger.caster == crate::target::PlayerFilter::Opponent
        });
    let timing = if let Some(upkeep) = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
    {
        match &upkeep.player {
            crate::target::PlayerFilter::You => "at the beginning of your first upkeep".to_string(),
            crate::target::PlayerFilter::Any => "at the beginning of the first upkeep".to_string(),
            _ => lowercase_first(&schedule.trigger.display()),
        }
    } else if let Some(main) = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()
    {
        match (main.player.clone(), main.phase_type) {
            (
                crate::target::PlayerFilter::You,
                crate::triggers::phase_step::MainPhaseType::Precombat,
            ) => "at the beginning of your first main phase of the game".to_string(),
            _ => lowercase_first(&schedule.trigger.display()),
        }
    } else if first_opponent_spell {
        "when each opponent casts their first spell of the game".to_string()
    } else {
        lowercase_first(&schedule.trigger.display())
    };

    let mut consequence =
        crate::runtime_display::compile_effect_list(schedule.effects.flattened_default_effects());
    if first_opponent_spell {
        consequence = contextualize_delayed_spell_consequence(&consequence);
    }
    let consequence = consequence.trim().trim_end_matches('.');
    if spec.effect_before_timing {
        format!("{prefix}, {} {timing}.", lowercase_first(consequence))
    } else {
        format!("{prefix}, {timing}, {}.", lowercase_first(consequence))
    }
}

fn render_begin_on_battlefield_pregame(
    spec: &crate::static_abilities::PregameBeginOnBattlefieldSpec,
) -> String {
    let mut clause = String::from("If this card is in your opening hand");
    if spec.require_not_starting_player {
        clause.push_str(" and you're not the starting player");
    }
    let simple_begin_on_battlefield = !spec.require_not_starting_player
        && spec.counters.is_empty()
        && spec.exile_cards_from_hand == 0;
    if simple_begin_on_battlefield {
        clause.push_str(", you may begin the game with it on the battlefield");
    } else {
        clause.push_str(", you may begin the game with this on the battlefield");
    }
    if !spec.counters.is_empty() {
        clause.push_str(" with ");
        let counter_phrases: Vec<String> = spec
            .counters
            .iter()
            .map(|(counter_type, count)| describe_counter_phrase(counter_type, *count))
            .collect();
        clause.push_str(&join_english(&counter_phrases));
        clause.push_str(" on it");
    }
    clause.push('.');
    if spec.exile_cards_from_hand > 0 {
        let count = spec.exile_cards_from_hand;
        let card_word = if count == 1 { "card" } else { "cards" };
        let count_word = if count == 1 {
            "a".to_string()
        } else {
            count.to_string()
        };
        clause.push_str(&format!(
            " If you do, exile {count_word} {card_word} from your hand."
        ));
    }
    clause
}

fn describe_counter_phrase(counter_type: &crate::object::CounterType, count: u32) -> String {
    let counter_name = counter_type.description();
    if count == 1 {
        format!("a {counter_name} counter")
    } else {
        format!("{count} {counter_name} counters")
    }
}

fn join_english(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, rest) = items.split_last().expect("nonempty");
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// Supported keyword-like text that should compile cleanly even before it has
/// dedicated runtime hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordText {
    pub text: String,
}

impl KeywordText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for KeywordText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Draft-only rule text from Conspiracy-style cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRuleText {
    pub text: String,
}

impl DraftRuleText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for DraftRuleText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DraftRuleText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Marker for CR 702.106 hidden agenda setup and reveal semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenAgenda;

impl StaticAbilityKind for HiddenAgenda {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::HiddenAgenda
    }

    fn display(&self) -> String {
        "Hidden agenda".to_string()
    }
}

/// Marker for the two-name hidden-agenda variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleAgenda;

impl StaticAbilityKind for DoubleAgenda {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoubleAgenda
    }

    fn display(&self) -> String {
        "Double agenda".to_string()
    }
}

/// Deck-construction rule text with no in-game rules impact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckConstructionRuleText {
    pub text: String,
}

impl DeckConstructionRuleText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for DeckConstructionRuleText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DeckConstructionRuleText
    }

    fn display(&self) -> String {
        fn title_case_name_fragment(name: &str) -> String {
            name.split_whitespace()
                .map(|part| {
                    let mut chars = part.chars();
                    let Some(first) = chars.next() else {
                        return String::new();
                    };
                    format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        chars.as_str().to_ascii_lowercase()
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        }

        if let Some((prefix, name)) = self.text.rsplit_once("cards named ") {
            let name = name.trim_end_matches('.');
            return format!("{prefix}cards named {}.", title_case_name_fragment(name));
        }
        self.text.clone()
    }
}

// =============================================================================
// Placeholder / Marker Abilities
// =============================================================================

/// CR 702.47 splice ability that functions from its card's owner's hand.
#[derive(Debug, Clone, PartialEq)]
pub struct SpliceAbility {
    pub spec: crate::static_abilities::SpliceSpec<crate::costs::Cost>,
}

impl SpliceAbility {
    pub fn new(spec: crate::static_abilities::SpliceSpec<crate::costs::Cost>) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for SpliceAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Splice
    }

    fn display(&self) -> String {
        let separator = if self.spec.cost.has_non_mana_costs() {
            "—"
        } else {
            " "
        };
        let rendered_cost = self
            .spec
            .cost_surface
            .clone()
            .unwrap_or_else(|| self.spec.cost.display());
        format!(
            "Splice onto {}{separator}{}",
            self.spec.quality.oracle_surface(),
            rendered_cost
        )
    }

    fn splice_spec(&self) -> Option<&crate::static_abilities::SpliceSpec<crate::costs::Cost>> {
        Some(&self.spec)
    }
}

/// CR 702.120 escalate ability that adds its typed cost per mode beyond the first.
#[derive(Debug, Clone, PartialEq)]
pub struct EscalateAbility {
    pub spec: crate::static_abilities::EscalateSpec<crate::costs::Cost>,
}

impl EscalateAbility {
    pub fn new(spec: crate::static_abilities::EscalateSpec<crate::costs::Cost>) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for EscalateAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Escalate
    }

    fn display(&self) -> String {
        let separator = if self.spec.cost.has_non_mana_costs() {
            "—"
        } else {
            " "
        };
        let rendered_cost = self
            .spec
            .cost_surface
            .clone()
            .unwrap_or_else(|| self.spec.cost.display());
        format!("Escalate{separator}{rendered_cost}")
    }

    fn escalate_spec(&self) -> Option<&crate::static_abilities::EscalateSpec<crate::costs::Cost>> {
        Some(&self.spec)
    }
}

/// Semantic keyword label for a keyword whose runtime semantics are implemented elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordMarker {
    pub marker: String,
}

impl KeywordMarker {
    pub fn new(marker: impl Into<String>) -> Self {
        Self {
            marker: marker.into(),
        }
    }
}

impl StaticAbilityKind for KeywordMarker {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordMarker
    }

    fn display(&self) -> String {
        const MORE_THAN_MEETS_THE_EYE: &str = "more than meets the eye";
        let trimmed = self.marker.trim();
        if let Some(prefix) = trimmed.get(..MORE_THAN_MEETS_THE_EYE.len())
            && prefix.eq_ignore_ascii_case(MORE_THAN_MEETS_THE_EYE)
            && trimmed[MORE_THAN_MEETS_THE_EYE.len()..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
        {
            return format!(
                "More Than Meets the Eye{}",
                &trimmed[MORE_THAN_MEETS_THE_EYE.len()..]
            );
        }
        self.marker.clone()
    }
}

/// Inert compiled-text provenance for keywords authored on one source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLineKeywordGroup {
    pub keyword_count: usize,
}

impl SourceLineKeywordGroup {
    pub const fn new(keyword_count: usize) -> Self {
        Self { keyword_count }
    }
}

impl StaticAbilityKind for SourceLineKeywordGroup {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SourceLineKeywordGroup
    }

    fn display(&self) -> String {
        String::new()
    }
}

/// Inert compiled-text provenance for static models from one source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLineStaticGroup {
    pub member_count: usize,
}

impl SourceLineStaticGroup {
    pub const fn new(member_count: usize) -> Self {
        Self { member_count }
    }
}

impl StaticAbilityKind for SourceLineStaticGroup {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SourceLineStaticGroup
    }

    fn display(&self) -> String {
        String::new()
    }
}

/// Allows a player to continuously see the top card of their library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookAtTopCardOfLibrary;

impl StaticAbilityKind for LookAtTopCardOfLibrary {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LookAtTopCardOfLibrary
    }

    fn display(&self) -> String {
        "You may look at the top card of your library any time.".to_string()
    }
}

/// Allows a player to continuously see face-down creatures they do not control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookAtFaceDownCreaturesYouDontControl;

impl StaticAbilityKind for LookAtFaceDownCreaturesYouDontControl {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LookAtFaceDownCreaturesYouDontControl
    }

    fn display(&self) -> String {
        "You may look at face-down creatures you don't control any time.".to_string()
    }
}

/// Allows every player to continuously see the top card of every library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllPlayersLookAtTopCardsOfLibraries;

impl StaticAbilityKind for AllPlayersLookAtTopCardsOfLibraries {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries
    }

    fn display(&self) -> String {
        "Players play with the top card of their libraries revealed.".to_string()
    }
}

/// Allows every player to continuously see the controller's top library card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllPlayersLookAtYourTopLibraryCard;

impl StaticAbilityKind for AllPlayersLookAtYourTopLibraryCard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AllPlayersLookAtYourTopLibraryCard
    }

    fn display(&self) -> String {
        "Play with the top card of your library revealed.".to_string()
    }
}

/// Makes the controller's opponents play with revealed hands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentsPlayWithHandsRevealed;

impl StaticAbilityKind for OpponentsPlayWithHandsRevealed {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentsPlayWithHandsRevealed
    }

    fn display(&self) -> String {
        "Your opponents play with their hands revealed.".to_string()
    }
}

/// Controls opponents during their library searches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlOpponentsWhileSearchingLibraries;

impl StaticAbilityKind for ControlOpponentsWhileSearchingLibraries {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ControlOpponentsWhileSearchingLibraries
    }

    fn display(&self) -> String {
        "You control your opponents while they're searching their libraries.".to_string()
    }
}

/// Replaces opponents' found search cards with exile and grants play permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentSearchExileFoundCards;

impl StaticAbilityKind for OpponentSearchExileFoundCards {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentSearchExileFoundCards
    }

    fn display(&self) -> String {
        "While an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.".to_string()
    }
}

/// Allows this card to be cast from the library while its owner is searching it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastThisCardFromLibraryWhileSearching;

impl StaticAbilityKind for CastThisCardFromLibraryWhileSearching {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CastThisCardFromLibraryWhileSearching
    }

    fn display(&self) -> String {
        "While you're searching your library, you may cast this card from your library.".to_string()
    }
}

/// Typed fallback keyword text preserved from parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordFallbackText {
    pub text: String,
}

impl KeywordFallbackText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for KeywordFallbackText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordFallbackText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Typed fallback static rule text preserved from parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFallbackText {
    pub text: String,
}

impl RuleFallbackText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for RuleFallbackText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RuleFallbackText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Parser fallback marker used in allow-unsupported mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedParserLine {
    pub raw_line: String,
    pub reason: String,
}

impl UnsupportedParserLine {
    pub fn new(raw_line: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            raw_line: raw_line.into(),
            reason: reason.into(),
        }
    }
}

impl StaticAbilityKind for UnsupportedParserLine {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::UnsupportedParserLine
    }

    fn display(&self) -> String {
        format!(
            "Unsupported parser line fallback: {} ({})",
            self.raw_line.trim(),
            self.reason
        )
    }
}
