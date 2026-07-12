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
        let current = ctx.game.object(damage.source).is_some_and(|source| {
            let filter_ctx = if source.zone == Zone::Stack {
                ctx.filter_ctx
                    .clone()
                    .with_caster(ctx.game.current_controller(damage.source))
            } else {
                ctx.filter_ctx.clone()
            };
            self.source_filter.matches(source, &filter_ctx, ctx.game)
        });
        let lki = ctx
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
            });
        current || lki
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
            display: display.into(),
        }
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
                noncombat_only: false,
                amount_less_than: None,
            },
            ReplacementAction::Modify(EventModification::Multiply(self.factor)),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleCountersReplacement {
    pub filter: ObjectFilter,
    pub player_filter: Option<PlayerFilter>,
    pub counter_type: Option<CounterType>,
    pub display: String,
}

impl DoubleCountersReplacement {
    pub fn new(filter: ObjectFilter, counter_type: Option<CounterType>, display: String) -> Self {
        Self {
            filter,
            player_filter: None,
            counter_type,
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
                match put_counters.target {
                    crate::game_state::Target::Object(object) => {
                        self.player_filter.is_none()
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
                if self.player_filter.is_some() {
                    return false;
                }
                let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                    return false;
                };
                if etb.object == self.ability_source {
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
                let mut filter = self.filter.clone();
                filter.zone = None;
                ctx.game
                    .object(etb.object)
                    .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game))
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
            },
            ReplacementAction::DoubleCounters {
                counter_type: self.counter_type,
            },
        ))
    }
}

/// "If one or more [type] counters would be put on a [filter], that many plus
/// N are put on it instead." (Hardened Scales, Conclave Mentor.)
#[derive(Debug, Clone, PartialEq)]
pub struct AddCountersPlacementReplacement {
    pub filter: ObjectFilter,
    pub player_filter: Option<PlayerFilter>,
    pub counter_type: Option<CounterType>,
    pub additional: u32,
    pub display: String,
}

impl AddCountersPlacementReplacement {
    pub fn new(
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        additional: u32,
        display: String,
    ) -> Self {
        Self {
            filter,
            player_filter: None,
            counter_type,
            additional,
            display,
        }
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
            },
            ReplacementAction::AddCountersToPlacement {
                counter_type: self.counter_type,
                additional: self.additional,
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

#[derive(Debug, Clone, PartialEq)]
pub struct AddTokenCreationReplacement {
    pub controller: PlayerFilter,
    pub token_filter: ObjectFilter,
    pub additional_token: ironsmith_core::AdditionalTokenKind,
    pub additional: i32,
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
            display: display.into(),
        }
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
            ReplacementAction::AddTokens {
                token: self.additional_token,
                count: self.additional.max(0) as u32,
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
    pub display: String,
}

impl ConditionalDrawReplacement {
    pub fn new(
        condition: Condition,
        replacement_effects: Vec<Effect>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            condition,
            replacement_effects,
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
        combined.display = format!(
            "{} {}",
            combined.display,
            super::super::describe_static_condition(&combined.condition)
        );
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
            ConditionalWouldDrawCardMatcher {
                condition: self.condition.clone(),
                display: self.display.clone(),
            },
            ReplacementAction::Instead(self.replacement_effects.clone()),
        ))
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
    pub replacement_effects: Vec<Effect>,
    pub display: String,
}

impl KeywordActionReplacement {
    pub fn new(
        action: crate::events::KeywordActionKind,
        source_filter: ObjectFilter,
        replacement_effects: Vec<Effect>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            action,
            source_filter,
            replacement_effects,
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
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::other::WouldKeywordActionMatcher::new(
                self.action,
                self.source_filter.clone(),
            ),
            ReplacementAction::Instead(self.replacement_effects.clone()),
        ))
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
}

impl StaticAbilityKind for ExileToExileInsteadOfGraveyard {
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
            self.filter.description(),
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
    pub exile_with_counters: Vec<(CounterType, u32)>,
    pub follow_up_effects: Vec<Effect>,
}

impl ExileWouldDieInstead {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_by: None,
            exile_with_counters: Vec::new(),
            follow_up_effects: Vec::new(),
        }
    }

    pub fn damaged_by(filter: ObjectFilter, damaged_by: DamagedBySource) -> Self {
        Self {
            filter,
            damaged_by: Some(damaged_by),
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

    pub fn with_counters_and_follow_up(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        exile_with_counters: Vec<(CounterType, u32)>,
        follow_up_effects: Vec<Effect>,
    ) -> Self {
        Self {
            filter,
            damaged_by,
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

impl StaticAbilityKind for ExileWouldDieInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExileWouldDieInstead
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
        if let Some(damaged_by) = self.damaged_by {
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

/// Parser-backed pregame action from opening hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregameAction {
    pub kind: crate::static_abilities::PregameActionKind,
    pub text: String,
}

impl PregameAction {
    pub fn new(kind: crate::static_abilities::PregameActionKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
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
        }
    }

    fn pregame_action_kind(&self) -> Option<crate::static_abilities::PregameActionKind> {
        Some(self.kind.clone())
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
        self.marker.clone()
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
