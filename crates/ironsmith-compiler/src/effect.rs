use crate::effects::{
    ChooseModeEffect, ConditionalEffect, DealDamageEffect, HauntExileEffect, IfEffect,
    PutCountersEffect, SearchLibrarySlotsEffect, WithIdEffect,
};
pub use ironsmith_core::{
    ChoiceCount, Comparison, Condition, DelayedTriggerSpec, EffectId, EffectMode as CoreEffectMode,
    EffectPredicate, EventValueSpec, ManaSpendPermission, Restriction, SearchSelectionMode, Until,
    Value, ValueComparisonOperator,
};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

pub type EffectMode = CoreEffectMode<Effect>;

fn build_unless_payment_total_cost(
    mana: Vec<crate::mana::ManaSymbol>,
    life: Option<Value>,
    additional_generic: Option<Value>,
    mana_multiplier: Option<Value>,
    x_value: Option<Value>,
) -> crate::cost::TotalCost {
    let mut components = Vec::new();
    let mana_cost = crate::mana::ManaCost::from_symbols(mana);
    if !mana_cost.is_empty()
        || additional_generic.is_some()
        || mana_multiplier.is_some()
        || x_value.is_some()
    {
        if additional_generic.is_some() || mana_multiplier.is_some() || x_value.is_some() {
            components.push(crate::costs::Cost::dynamic_mana(
                ironsmith_core::DynamicManaCost::new(
                    mana_cost,
                    x_value,
                    additional_generic,
                    mana_multiplier,
                    ironsmith_core::DynamicManaDisplayHint::Default,
                ),
            ));
        } else {
            components.push(crate::costs::Cost::mana(mana_cost));
        }
    }
    if let Some(life) = life {
        components.push(crate::costs::Cost::life(life));
    }
    crate::cost::TotalCost::from_costs(components)
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmblemDescription {
    pub name: String,
    pub text: String,
    pub abilities: Vec<crate::ability::Ability>,
}

impl EmblemDescription {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            abilities: Vec::new(),
        }
    }

    pub fn with_ability(mut self, ability: crate::ability::Ability) -> Self {
        self.abilities.push(ability);
        self
    }
}

pub trait EffectPayload: Any + Debug + Send + Sync {
    fn clone_box(&self) -> Box<dyn EffectPayload>;
    fn as_any(&self) -> &dyn Any;
    fn type_name(&self) -> &'static str;
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        None
    }
}

impl<T> EffectPayload for T
where
    T: Any + Debug + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn EffectPayload> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

#[derive(Debug)]
pub struct Effect(pub Arc<dyn EffectPayload>);

impl Clone for Effect {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl PartialEq for Effect {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Effect {
    pub fn new<T>(effect: T) -> Self
    where
        T: EffectPayload + 'static,
    {
        Self(Arc::new(effect))
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.payload_any().downcast_ref::<T>()
    }

    pub fn payload_type_name(&self) -> &'static str {
        Self::unwrap_payload(&*self.0).type_name()
    }

    fn payload_any(&self) -> &dyn Any {
        Self::unwrap_payload(&*self.0).as_any()
    }

    fn unwrap_payload(payload: &dyn EffectPayload) -> &dyn EffectPayload {
        if let Some(nested) = payload.as_any().downcast_ref::<Arc<dyn EffectPayload>>() {
            Self::unwrap_payload(&**nested)
        } else {
            payload
        }
    }

    pub fn as_deal_damage(&self) -> Option<&crate::effects::DealDamageEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::DealDamageEffect>()
    }

    pub fn as_put_counters(&self) -> Option<&crate::effects::PutCountersEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::PutCountersEffect>()
    }

    pub fn as_choose_mode(&self) -> Option<&crate::effects::ChooseModeEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::ChooseModeEffect>()
    }

    pub fn as_conditional(&self) -> Option<&crate::effects::ConditionalEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::ConditionalEffect>()
    }

    pub fn as_if_effect(&self) -> Option<&crate::effects::IfEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::IfEffect>()
    }

    pub fn as_with_id(&self) -> Option<&crate::effects::WithIdEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::WithIdEffect>()
    }

    pub fn as_haunt_exile(&self) -> Option<&crate::effects::HauntExileEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::HauntExileEffect>()
    }

    pub fn as_target_only(&self) -> Option<&crate::effects::TargetOnlyEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
    }

    pub fn as_search(&self) -> Option<&crate::effects::SearchLibraryEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::SearchLibraryEffect>()
    }

    pub fn as_search_slots(&self) -> Option<&crate::effects::SearchLibrarySlotsEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::SearchLibrarySlotsEffect>()
    }

    pub fn as_draw_cards(&self) -> Option<&crate::effects::DrawCardsEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::DrawCardsEffect>()
    }

    pub fn as_create_token(&self) -> Option<&crate::effects::CreateTokenEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::CreateTokenEffect>()
    }

    pub fn as_tap(&self) -> Option<&crate::effects::TapEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::TapEffect>()
    }

    pub fn as_untap(&self) -> Option<&crate::effects::UntapEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::UntapEffect>()
    }

    pub fn as_remove_counters(&self) -> Option<&crate::effects::RemoveCountersEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
    }

    pub fn as_counter(&self) -> Option<&crate::effects::CounterEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::CounterEffect>()
    }

    pub fn as_tagged(&self) -> Option<&crate::effects::TaggedEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::TaggedEffect>()
    }

    pub fn as_schedule_delayed_trigger(
        &self,
    ) -> Option<&crate::effects::ScheduleDelayedTriggerEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    }

    pub fn as_grant_abilities_target(&self) -> Option<&crate::effects::GrantAbilitiesTargetEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
    }

    pub fn as_create_token_copy(&self) -> Option<&crate::effects::CreateTokenCopyEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
    }

    pub fn as_grant_next_spell_ability(
        &self,
    ) -> Option<&crate::effects::GrantNextSpellAbilityEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>()
    }

    pub fn as_apply_continuous(&self) -> Option<&crate::effects::ApplyContinuousEffect> {
        self.payload_any()
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    }

    pub fn mana_restricted(
        effects: Vec<Effect>,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
    ) -> Self {
        Self::new(crate::effects::ManaRestrictedEffect::new(
            effects,
            restrictions,
        ))
    }

    pub fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        if let Some(sequence) = self.downcast_ref::<crate::effects::SequenceEffect>() {
            for effect in &sequence.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(restricted) = self.downcast_ref::<crate::effects::ManaRestrictedEffect>() {
            for effect in &restricted.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(may) = self.downcast_ref::<crate::effects::MayEffect<Effect>>() {
            for effect in &may.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(unless_pays) = self.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() {
            for effect in &unless_pays.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(unless_action) =
            self.downcast_ref::<crate::effects::UnlessActionEffect<Effect>>()
        {
            for effect in &unless_action.effects {
                visitor(effect);
            }
            for effect in &unless_action.alternative {
                visitor(effect);
            }
            return;
        }
        if let Some(for_players) = self.downcast_ref::<crate::effects::ForPlayersEffect<Effect>>() {
            for effect in &for_players.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(for_each_object) = self.downcast_ref::<crate::effects::ForEachObject>() {
            for effect in &for_each_object.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(for_each_tagged) =
            self.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>()
        {
            for effect in &for_each_tagged.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(for_each_controller) =
            self.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect<Effect>>()
        {
            for effect in &for_each_controller.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(for_each_player) =
            self.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect<Effect>>()
        {
            for effect in &for_each_player.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(conditional) = self.downcast_ref::<crate::effects::ConditionalEffect>() {
            for effect in &conditional.if_true {
                visitor(effect);
            }
            for effect in &conditional.if_false {
                visitor(effect);
            }
            return;
        }
        if let Some(if_effect) = self.downcast_ref::<crate::effects::IfEffect>() {
            for effect in &if_effect.then {
                visitor(effect);
            }
            for effect in &if_effect.else_ {
                visitor(effect);
            }
            return;
        }
        if let Some(choose_mode) = self.downcast_ref::<crate::effects::ChooseModeEffect>() {
            for mode in &choose_mode.modes {
                for effect in &mode.effects {
                    visitor(effect);
                }
            }
            return;
        }
        if let Some(tagged) = self.downcast_ref::<crate::effects::TaggedEffect>() {
            visitor(&tagged.effect);
            return;
        }
        if let Some(with_id) = self.downcast_ref::<crate::effects::WithIdEffect>() {
            visitor(&with_id.effect);
            return;
        }
        if let Some(local_rewrite) = self.downcast_ref::<crate::effects::LocalRewriteEffect>() {
            visitor(&local_rewrite.effect);
            return;
        }
        if let Some(execute_with_source) =
            self.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        {
            visitor(&execute_with_source.effect);
            return;
        }
        if let Some(repeat_effects) = self.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
            for effect in &repeat_effects.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(repeat_process) = self.downcast_ref::<crate::effects::RepeatProcessEffect>() {
            for effect in &repeat_process.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(haunt) = self.downcast_ref::<crate::effects::HauntExileEffect>() {
            for effect in &haunt.haunt_effects {
                visitor(effect);
            }
            return;
        }
        if let Some(prevent) = self.downcast_ref::<crate::effects::PreventDamageEffect>() {
            for effect in &prevent.follow_up_effects {
                visitor(effect);
            }
            return;
        }
        if let Some(prevent) = self.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()
        {
            for effect in &prevent.follow_up_effects {
                visitor(effect);
            }
            return;
        }
        if let Some(schedule) = self.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        {
            for effect in &schedule.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(schedule) =
            self.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
        {
            for effect in &schedule.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(cumulative) = self.downcast_ref::<crate::effects::CumulativeUpkeepEffect>() {
            for effect in &cumulative.payment {
                visitor(effect);
            }
            for effect in &cumulative.failure {
                visitor(effect);
            }
            return;
        }
        if let Some(reflexive) = self.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
            for effect in &reflexive.effects {
                visitor(effect);
            }
            return;
        }
        if let Some(vote) = self.downcast_ref::<crate::effects::VoteEffect>() {
            if let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice {
                for option in options {
                    for effect in &option.effects_per_vote {
                        visitor(effect);
                    }
                }
            }
        }
    }

    pub fn directly_produces_mana(&self) -> bool {
        self.downcast_ref::<crate::effects::AddManaEffect>()
            .is_some()
            || self
                .downcast_ref::<crate::effects::mana::AddScaledManaEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                .is_some_and(|effect| {
                    effect
                        .available_colors
                        .as_ref()
                        .is_none_or(|colors| !colors.is_empty())
                })
            || self
                .downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::mana::AddManaOfChosenColorEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::mana::AddManaOfImprintedColorsEffect>()
                .is_some()
            || self
                .downcast_ref::<crate::effects::DoubleManaPoolEffect>()
                .is_some()
    }

    pub fn contains_mana_production(&self) -> bool {
        if self.directly_produces_mana() {
            return true;
        }

        let mut found = false;
        self.visit_child_effects(&mut |effect| {
            if !found && effect.contains_mana_production() {
                found = true;
            }
        });
        found
    }

    pub fn target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        if let Some(payload) = self.as_tagged() {
            return payload.effect.target_spec();
        }
        if let Some(payload) = self.as_with_id() {
            return payload.effect.target_spec();
        }
        if let Some(payload) = self.as_target_only() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_deal_damage() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_put_counters() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::DestroyEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
            && let Some(target) = &payload.target
        {
            return Some(target);
        }
        if let Some(payload) = self.as_tap() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_untap() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_remove_counters() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_counter() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.as_grant_abilities_target() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) =
            self.downcast_ref::<crate::effects::MoveToLibraryTopOrBottomChoiceEffect>()
        {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::ReturnToHandEffect>() {
            return Some(&payload.spec);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::ExileEffect>() {
            return Some(&payload.spec);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::AttachToEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::ReconfigureEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::AttachObjectsEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) =
            self.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::GrantEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::ExchangeControlEffect>() {
            return Some(&payload.permanent1);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::MoveAllCountersEffect>() {
            return Some(&payload.from);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::MoveOneCounterEffect>() {
            return Some(&payload.from);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::TransformEffect>() {
            return Some(&payload.target);
        }
        if let Some(payload) = self.downcast_ref::<crate::effects::IncubateEffect>() {
            return payload.controller_target.as_ref();
        }
        None
    }

    pub fn deal_damage(amount: impl Into<Value>, target: crate::target::ChooseSpec) -> Self {
        Self::new(DealDamageEffect::new(amount.into(), target))
    }

    pub fn choose_one(modes: Vec<EffectMode>) -> Self {
        Self::new(ChooseModeEffect::new(
            modes,
            Value::Fixed(1),
            Value::Fixed(1),
            false,
        ))
    }

    pub fn choose_up_to(max: Value, modes: Vec<EffectMode>) -> Self {
        Self::new(ChooseModeEffect::new(modes, Value::Fixed(0), max, false))
    }

    pub fn choose_up_to_with_min(max: Value, min: Value, modes: Vec<EffectMode>) -> Self {
        Self::new(ChooseModeEffect::new(modes, min, max, false))
    }

    pub fn choose_exactly(count: Value, modes: Vec<EffectMode>) -> Self {
        Self::new(ChooseModeEffect::new(modes, count.clone(), count, false))
    }

    pub fn choose_exactly_allow_repeated_modes(count: Value, modes: Vec<EffectMode>) -> Self {
        Self::new(ChooseModeEffect::new(modes, count.clone(), count, true))
    }

    pub fn conditional(condition: Condition, if_true: Vec<Effect>, if_false: Vec<Effect>) -> Self {
        Self::new(ConditionalEffect::new(condition, if_true, if_false))
    }

    pub fn if_then(
        effect_id: impl Into<EffectId>,
        predicate: EffectPredicate,
        effects: Vec<Effect>,
    ) -> Self {
        Self::new(IfEffect::new(
            effect_id.into(),
            predicate,
            effects,
            Vec::new(),
        ))
    }

    pub fn with_id(effect_id: u32, effect: Effect) -> Self {
        Self::new(WithIdEffect::new(EffectId(effect_id), effect))
    }

    pub fn haunt_exile(
        haunt_effects: Vec<Effect>,
        haunt_choices: Vec<crate::target::ChooseSpec>,
    ) -> Self {
        Self::new(HauntExileEffect::new(haunt_effects, haunt_choices))
    }

    pub fn put_counters(
        counter_type: crate::object::CounterType,
        amount: impl Into<Value>,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(PutCountersEffect::new(counter_type, amount.into(), target))
    }

    pub fn put_counters_on_source(counter_type: crate::object::CounterType, amount: i32) -> Self {
        Self::put_counters(counter_type, amount, crate::target::ChooseSpec::Source)
    }

    pub fn remove_counters(
        counter_type: crate::object::CounterType,
        amount: impl Into<Value>,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::RemoveCountersEffect::new(
            counter_type,
            amount,
            target,
        ))
    }

    pub fn remove_up_to_counters(
        counter_type: crate::object::CounterType,
        amount: impl Into<Value>,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::RemoveUpToCountersEffect::new(
            counter_type,
            amount,
            target,
        ))
    }

    pub fn remove_up_to_any_counters(
        amount: impl Into<Value>,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::RemoveUpToAnyCountersEffect::new(
            amount, target,
        ))
    }

    pub fn search_library_to_hand(filter: crate::target::ObjectFilter, optional: bool) -> Self {
        let mut effect = crate::effects::SearchLibraryEffect::to_hand(
            filter,
            crate::target::PlayerFilter::You,
            false,
        );
        if optional {
            effect = effect.with_search_mode(SearchSelectionMode::Optional);
        }
        Self::new(effect)
    }

    pub fn search_library_slots_to_hand(
        slots: Vec<crate::effects::SearchLibrarySlot>,
        player: crate::target::PlayerFilter,
        reveal: bool,
        tag: crate::tag::TagKey,
    ) -> Self {
        Self::new(SearchLibrarySlotsEffect::to_hand(
            slots, player, reveal, tag,
        ))
    }

    pub fn move_to_zone(
        target: crate::target::ChooseSpec,
        zone: crate::zone::Zone,
        to_top: bool,
    ) -> Self {
        Self::new(crate::effects::MoveToZoneEffect::new(target, zone, to_top))
    }

    pub fn move_to_library_top_or_bottom_choice(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(
            target,
        ))
    }

    pub fn for_each_tagged(_tag: impl Into<crate::tag::TagKey>, _effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::ForEachTaggedEffect {
            tag: _tag.into(),
            effects: _effects,
        })
    }

    pub fn for_each_tagged_player(
        tag: impl Into<crate::tag::TagKey>,
        effects: Vec<Effect>,
    ) -> Self {
        Self::new(crate::effects::ForEachTaggedPlayerEffect {
            tag: tag.into(),
            effects,
        })
    }

    pub fn choose_objects(
        filter: crate::target::ObjectFilter,
        count: impl Into<crate::effect::ChoiceCount>,
        chooser: crate::target::PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(crate::effects::ChooseObjectsEffect::new(
            filter, count, chooser, tag,
        ))
    }

    pub fn emit_keyword_action(kind: crate::events::KeywordActionKind, count: u32) -> Self {
        Self::new(crate::effects::EmitKeywordActionEffect::new(kind, count))
    }

    pub fn emit_keyword_action_with_affected_object_memory_tag(
        kind: crate::events::KeywordActionKind,
        count: u32,
        effect_id: crate::effect::EffectId,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(
            crate::effects::EmitKeywordActionEffect::new(kind, count)
                .with_affected_object_memory_tag(effect_id, tag),
        )
    }

    pub fn renown_source(amount: u32) -> Self {
        Self::new(crate::effects::RenownEffect::new(amount))
    }

    pub fn for_each(_filter: crate::target::ObjectFilter, _effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::ForEachObject {
            filter: _filter,
            effects: _effects,
        })
    }

    pub fn tap(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::TapEffect::with_spec(target))
    }

    pub fn untap(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::UntapEffect::with_spec(target))
    }

    pub fn tap_all(_filter: crate::target::ObjectFilter) -> Self {
        Self::new(crate::effects::TapEffect::all(_filter))
    }

    pub fn untap_all(_filter: crate::target::ObjectFilter) -> Self {
        Self::new(crate::effects::UntapEffect::all(_filter))
    }

    pub fn shuffle_library_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ShuffleLibraryEffect::new(player))
    }

    pub fn sacrifice_source() -> Self {
        Self::new(crate::effects::SacrificeTargetEffect::source())
    }

    pub fn sacrifice_player(
        filter: crate::target::ObjectFilter,
        count: impl Into<Value>,
        chooser: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::SacrificePlayerEffect::new(
            filter, count, chooser,
        ))
    }

    pub fn sacrifice(filter: crate::target::ObjectFilter, count: u32) -> Self {
        Self::new(crate::effects::SacrificeEffect {
            filter,
            count: count as i32,
            event_object_tags: Vec::new(),
            event_source_tags: Vec::new(),
        })
    }

    pub fn sacrifice_with_event_tags(
        filter: crate::target::ObjectFilter,
        count: u32,
        object_tag: impl Into<crate::tag::TagKey>,
        source_tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(
            crate::effects::SacrificeEffect {
                filter,
                count: count as i32,
                event_object_tags: Vec::new(),
                event_source_tags: Vec::new(),
            }
            .with_event_object_tag(object_tag)
            .with_event_source_tag(source_tag),
        )
    }

    pub fn pump(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        target: crate::target::ChooseSpec,
        until: Until,
    ) -> Self {
        Self::new(crate::effects::ModifyPowerToughnessEffect::new(
            target, power, toughness, until,
        ))
    }

    pub fn return_from_graveyard_to_battlefield(
        target: crate::target::ChooseSpec,
        tapped: bool,
    ) -> Self {
        Self::new(crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
            target, tapped,
        ))
    }

    pub fn put_onto_battlefield(
        target: crate::target::ChooseSpec,
        tapped: bool,
        controller: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::PutOntoBattlefieldEffect::new(
            target, tapped, controller,
        ))
    }

    pub fn look_at_top_cards(
        player: crate::target::PlayerFilter,
        count: Value,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(crate::effects::LookAtTopCardsEffect::new(
            player, count, tag,
        ))
    }

    pub fn reveal_top_cards(
        player: crate::target::PlayerFilter,
        count: Value,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(crate::effects::LookAtTopCardsEffect::revealing(
            player, count, tag,
        ))
    }

    pub fn for_players(_filter: crate::target::PlayerFilter, _effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::ForPlayersEffect {
            filter: _filter,
            effects: _effects,
        })
    }

    pub fn for_each_opponent(effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::ForPlayersEffect {
            filter: crate::target::PlayerFilter::Opponent,
            effects,
        })
    }

    pub fn may(_effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::MayEffect {
            decider: None,
            effects: _effects,
        })
    }

    pub fn may_player(_player: crate::target::PlayerFilter, _effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::MayEffect {
            decider: Some(_player),
            effects: _effects,
        })
    }

    pub fn unless_pays(
        effects: Vec<Effect>,
        player: crate::target::PlayerFilter,
        mana: Vec<crate::mana::ManaSymbol>,
    ) -> Self {
        Self::new(crate::effects::UnlessPaysEffect {
            player,
            effects,
            cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
        })
    }

    pub fn unless_pays_total_cost(
        effects: Vec<Effect>,
        player: crate::target::PlayerFilter,
        cost: crate::cost::TotalCost,
    ) -> Self {
        Self::new(crate::effects::UnlessPaysEffect {
            player,
            effects,
            cost,
        })
    }

    pub fn cumulative_upkeep(
        payment: Vec<Effect>,
        player: crate::target::PlayerFilter,
        failure: Vec<Effect>,
    ) -> Self {
        Self::new(crate::effects::CumulativeUpkeepEffect::new(
            player, payment, failure,
        ))
    }

    pub fn unless_action(
        _effects: Vec<Effect>,
        _alternative: Vec<Effect>,
        _player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::UnlessActionEffect {
            player: _player,
            effects: _effects,
            alternative: _alternative,
        })
    }

    pub fn reflexive_trigger(
        effect_id: impl Into<EffectId>,
        predicate: EffectPredicate,
        effects: Vec<Effect>,
        choices: Vec<crate::target::ChooseSpec>,
    ) -> Self {
        Self::new(crate::effects::ReflexiveTriggerEffect {
            condition: effect_id.into(),
            predicate,
            effects,
            choices,
        })
    }

    pub fn destroy_all(filter: crate::target::ObjectFilter) -> Self {
        Self::new(crate::effects::DestroyEffect::with_spec(
            crate::target::ChooseSpec::all(filter),
        ))
    }

    pub fn vote_objects_with_optional_extra(
        filter: crate::target::ObjectFilter,
        count: crate::effect::ChoiceCount,
        extra_mandatory: u32,
        extra_optional: u32,
    ) -> Self {
        Self::new(crate::effects::VoteEffect::objects(
            filter,
            count,
            extra_mandatory,
            extra_optional,
        ))
    }

    pub fn vote_objects(
        filter: crate::target::ObjectFilter,
        count: crate::effect::ChoiceCount,
        extra_mandatory: u32,
    ) -> Self {
        Self::new(crate::effects::VoteEffect::objects(
            filter,
            count,
            extra_mandatory,
            0,
        ))
    }

    pub fn vote_with_optional_extra(
        vote_options: Vec<crate::effects::composition::VoteOption>,
        extra_mandatory: u32,
        extra_optional: u32,
    ) -> Self {
        Self::new(crate::effects::VoteEffect::named(
            vote_options,
            extra_mandatory,
            extra_optional,
        ))
    }

    pub fn vote(
        vote_options: Vec<crate::effects::composition::VoteOption>,
        extra_mandatory: u32,
    ) -> Self {
        Self::new(crate::effects::VoteEffect::named(
            vote_options,
            extra_mandatory,
            0,
        ))
    }

    pub fn proliferate(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::ProliferateEffect::new(count))
    }

    pub fn phase_out(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::PhaseOutEffect::with_spec(target))
    }

    pub fn phase_in(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::PhaseInEffect::with_spec(target))
    }

    pub fn explore(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ExploreEffect::new(target))
    }

    pub fn open_attraction() -> Self {
        Self::new(crate::effects::OpenAttractionEffect::new())
    }

    pub fn manifest_top_card_of_library(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ManifestTopCardOfLibraryEffect::new(player))
    }

    pub fn manifest_card_from_hand() -> Self {
        Self::new(crate::effects::ManifestCardFromHandEffect::new())
    }

    pub fn manifest_dread() -> Self {
        Self::new(crate::effects::ManifestDreadEffect::new())
    }

    pub fn bolster(amount: u32) -> Self {
        Self::new(crate::effects::BolsterEffect::new(amount))
    }

    pub fn support(amount: u32) -> Self {
        Self::new(crate::effects::SupportEffect::new(amount))
    }

    pub fn adapt(amount: u32) -> Self {
        Self::new(crate::effects::AdaptEffect::new(amount))
    }

    pub fn attach_to(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::AttachToEffect::new(target))
    }

    pub fn attach_objects(
        objects: crate::target::ChooseSpec,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::AttachObjectsEffect::new(objects, target))
    }

    pub fn transform(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::TransformEffect::new(target))
    }

    pub fn flip(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::FlipEffect::new(target))
    }

    pub fn return_all_to_hand(filter: crate::target::ObjectFilter) -> Self {
        Self::new(crate::effects::ReturnToHandEffect::all(filter))
    }

    pub fn return_to_hand(target: impl Into<crate::target::ChooseSpec>) -> Self {
        Self::new(crate::effects::ReturnToHandEffect::with_spec(target.into()))
    }

    pub fn repeat_effects(count: Value, effects: Vec<Effect>) -> Self {
        Self::new(crate::effects::RepeatEffectsEffect::new(count, effects))
    }

    pub fn repeat_process(
        effects: Vec<Effect>,
        condition: EffectId,
        predicate: EffectPredicate,
    ) -> Self {
        Self::new(crate::effects::RepeatProcessEffect::new(
            effects, condition, predicate,
        ))
    }

    pub fn regenerate(target: crate::target::ChooseSpec, until: Until) -> Self {
        Self::new(crate::effects::RegenerateEffect::new(target, until))
    }

    pub fn prevent_damage(amount: Value, target: crate::target::ChooseSpec, until: Until) -> Self {
        Self::new(crate::effects::PreventDamageEffect::new(
            amount, target, until,
        ))
    }

    pub fn prevent_distributed_damage(
        amount: Value,
        target: crate::target::ChooseSpec,
        until: Until,
    ) -> Self {
        Self::new(crate::effects::PreventDistributedDamageEffect::new(
            amount, target, until,
        ))
    }

    pub fn prevent_damage_with_source_choice(
        amount: Value,
        target: crate::target::ChooseSpec,
        until: Until,
    ) -> Self {
        Self::new(
            crate::effects::PreventDamageEffect::new(amount, target, until)
                .with_source_of_your_choice(),
        )
    }

    pub fn prevent_all_combat_damage(until: Until) -> Self {
        Self::new(crate::effects::PreventAllCombatDamageEffect::new(
            crate::effects::CombatDamagePreventionTarget::All,
            until,
        ))
    }

    pub fn prevent_all_combat_damage_from(target: crate::target::ChooseSpec, until: Until) -> Self {
        Self::new(crate::effects::PreventAllCombatDamageEffect::new(
            crate::effects::CombatDamagePreventionTarget::From(target),
            until,
        ))
    }

    pub fn prevent_all_combat_damage_to_players(until: Until) -> Self {
        Self::new(crate::effects::PreventAllCombatDamageEffect::new(
            crate::effects::CombatDamagePreventionTarget::Players,
            until,
        ))
    }

    pub fn prevent_all_combat_damage_to_you(until: Until) -> Self {
        Self::new(crate::effects::PreventAllCombatDamageEffect::new(
            crate::effects::CombatDamagePreventionTarget::You,
            until,
        ))
    }

    pub fn prevent_all_combat_damage_from_filter(
        source_filter: crate::target::ObjectFilter,
        until: Until,
    ) -> Self {
        let mut damage_filter = ironsmith_core::DamageFilter::combat();
        damage_filter.from_source = Some(source_filter);
        Self::new(crate::effects::PreventAllDamageEffect::all_with_filter(
            damage_filter,
            until,
        ))
    }

    pub fn prevent_all_damage_to(target: crate::target::ObjectFilter, until: Until) -> Self {
        Self::new(crate::effects::PreventAllDamageEffect::matching(
            target, until,
        ))
    }

    pub fn prevent_all_damage_to_target(target: crate::target::ChooseSpec, until: Until) -> Self {
        Self::new(crate::effects::PreventAllDamageToTargetEffect::new(
            target, until,
        ))
    }

    pub fn prevent_all_damage_from_filter(
        source_filter: crate::target::ObjectFilter,
        until: Until,
    ) -> Self {
        let mut damage_filter = ironsmith_core::DamageFilter::all();
        damage_filter.from_source = Some(source_filter);
        Self::new(crate::effects::PreventAllDamageEffect::all_with_filter(
            damage_filter,
            until,
        ))
    }

    pub fn gain_life(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::GainLifeEffect::new(
            amount.into(),
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
        ))
    }

    pub fn roll_die(sides: u32, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::RollDieEffect::new(player, sides))
    }

    pub fn roll_die_with_die_text(
        sides: u32,
        player: crate::target::PlayerFilter,
        die_text: Option<String>,
    ) -> Self {
        Self::new(crate::effects::RollDieEffect::new_with_die_text(
            player, sides, die_text,
        ))
    }

    pub fn roll_dice_choose_result_with_die_text(
        count: u32,
        sides: u32,
        player: crate::target::PlayerFilter,
        die_text: Option<String>,
    ) -> Self {
        Self::new(
            crate::effects::RollDiceChooseResultEffect::new_with_die_text(
                player, count, sides, die_text,
            ),
        )
    }

    pub fn fight(a: crate::target::ChooseSpec, b: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::FightEffect::new(a, b))
    }

    pub fn exile(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ExileEffect::with_spec(target))
    }

    pub fn emit_gift_given(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::EmitGiftGivenEffect::new(player))
    }

    pub fn draw(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::DrawCardsEffect::new(
            count.into(),
            crate::target::PlayerFilter::You,
        ))
    }

    pub fn target_draws(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::DrawCardsEffect::new(count.into(), player))
    }

    pub fn counter(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::CounterEffect::new(target))
    }

    pub fn counter_unless_pays_with_life_and_additional_and_x(
        target: crate::target::ChooseSpec,
        mana: Vec<crate::mana::ManaSymbol>,
        life: Option<Value>,
        additional: Option<Value>,
        x: Option<Value>,
    ) -> Self {
        let player = match target.base() {
            crate::target::ChooseSpec::SpecificObject(id) => {
                crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Specific(*id))
            }
            crate::target::ChooseSpec::Tagged(tag) => crate::target::PlayerFilter::ControllerOf(
                crate::target::ObjectRef::Tagged(tag.clone()),
            ),
            _ => crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Target),
        };
        let cost = build_unless_payment_total_cost(mana, life, additional, None, x);
        Self::new(crate::effects::UnlessPaysEffect {
            player,
            effects: vec![Self::counter(target)],
            cost,
        })
    }

    pub fn lose_life(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::LoseLifeEffect::new(
            amount.into(),
            crate::target::PlayerFilter::You,
        ))
    }

    pub fn lose_life_player(amount: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::LoseLifeEffect::new(amount.into(), player))
    }

    pub fn gain_life_player(amount: impl Into<Value>, target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::GainLifeEffect::new(amount.into(), target))
    }

    pub fn lose_the_game() -> Self {
        Self::new(crate::effects::LoseTheGameEffect::you())
    }

    pub fn lose_the_game_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::LoseTheGameEffect::new(player))
    }

    pub fn win_the_game() -> Self {
        Self::win_the_game_player(crate::target::PlayerFilter::You)
    }

    pub fn win_the_game_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::WinTheGameEffect::new(player))
    }

    pub fn rearrange_looked_cards_in_library(
        tag: impl Into<crate::tag::TagKey>,
        player: crate::target::PlayerFilter,
        count: crate::effect::ChoiceCount,
    ) -> Self {
        Self::new(crate::effects::RearrangeLookedCardsInLibraryEffect::new(
            tag, player, count,
        ))
    }

    pub fn create_emblem(description: EmblemDescription) -> Self {
        Self::new(crate::effects::CreateEmblemEffect::new(description))
    }

    pub fn convert(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ConvertEffect::new(target))
    }

    pub fn connive(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ConniveEffect::new(target))
    }

    pub fn connive_with_count(
        target: crate::target::ChooseSpec,
        count: impl Into<crate::effect::Value>,
    ) -> Self {
        Self::new(crate::effects::ConniveEffect::new_with_count(target, count))
    }

    pub fn behold(subtype: crate::types::Subtype, count: u32) -> Self {
        Self::new(crate::effects::BeholdEffect::you(subtype, count))
    }

    pub fn add_mana(mana: Vec<crate::mana::ManaSymbol>) -> Self {
        Self::new(crate::effects::AddManaEffect::new(
            mana,
            crate::target::PlayerFilter::You,
        ))
    }

    pub fn add_mana_player(
        mana: Vec<crate::mana::ManaSymbol>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::AddManaEffect::new(mana, player))
    }

    pub fn add_mana_of_any_color_restricted(
        amount: impl Into<Value>,
        colors: Vec<crate::color::Color>,
    ) -> Self {
        Self::new(crate::effects::AddManaOfAnyColorEffect::restricted(
            amount,
            crate::target::PlayerFilter::You,
            colors,
        ))
    }

    pub fn add_mana_of_any_color(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::AddManaOfAnyColorEffect::new(
            amount,
            crate::target::PlayerFilter::You,
        ))
    }

    pub fn add_mana_of_any_color_restricted_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
        colors: Vec<crate::color::Color>,
    ) -> Self {
        Self::new(crate::effects::AddManaOfAnyColorEffect::restricted(
            amount, player, colors,
        ))
    }

    pub fn add_mana_of_any_color_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::AddManaOfAnyColorEffect::new(amount, player))
    }

    pub fn add_mana_of_any_one_color(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::AddManaOfAnyOneColorEffect::you(amount))
    }

    pub fn add_mana_of_any_one_color_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::AddManaOfAnyOneColorEffect::new(
            amount, player,
        ))
    }

    pub fn add_mana_of_land_produced_types_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
        filter: crate::target::ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self::new(crate::effects::AddManaOfLandProducedTypesEffect::new(
            amount,
            player,
            filter,
            allow_colorless,
            same_type,
        ))
    }

    pub fn add_mana_from_commander_color_identity(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::AddManaFromCommanderColorIdentityEffect::you(amount))
    }

    pub fn add_mana_from_commander_color_identity_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::AddManaFromCommanderColorIdentityEffect::new(amount, player))
    }

    pub fn discard_hand() -> Self {
        Self::new(crate::effects::DiscardHandEffect::you())
    }

    pub fn discard_hand_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::DiscardHandEffect::new(player))
    }

    pub fn discard_player_filtered(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
        random: bool,
        filter: Option<crate::target::ObjectFilter>,
    ) -> Self {
        Self::new(crate::effects::DiscardEffect::new_with_filter(
            count, player, random, filter,
        ))
    }

    pub fn ticket_counters(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::TicketCountersEffect::you(count))
    }

    pub fn ticket_counters_player(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::TicketCountersEffect::new(count, player))
    }

    pub fn detain(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::DetainEffect::new(target))
    }

    pub fn goad(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::GoadEffect::new(target))
    }

    pub fn suspect(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::SuspectEffect::new(target))
    }

    pub fn clear_suspected(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ClearSuspectedEffect::new(target))
    }

    pub fn clear_all_suspected() -> Self {
        Self::new(crate::effects::ClearSuspectedEffect::all())
    }

    pub fn return_from_graveyard_to_hand(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
            target, false,
        ))
    }

    pub fn may_move_to_zone(
        target: crate::target::ChooseSpec,
        zone: crate::zone::Zone,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::MayMoveToZoneEffect::new(
            target, zone, player,
        ))
    }

    pub fn return_from_graveyard_to_hand_with_random(
        target: crate::target::ChooseSpec,
        random: bool,
    ) -> Self {
        Self::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
            target, random,
        ))
    }

    pub fn exile_top_of_library_player(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
        accumulated_tag: Option<crate::tag::TagKey>,
    ) -> Self {
        let mut effect =
            crate::effects::ExileTopOfLibraryEffect::new(count.into(), player).tag_moved(tag);
        if let Some(accumulated_tag) = accumulated_tag {
            effect = effect.append_tagged(accumulated_tag);
        }
        Self::new(effect)
    }

    pub fn remove_any_counters_among(
        count: u32,
        filter: crate::target::ObjectFilter,
        counter_type: Option<crate::object::CounterType>,
    ) -> Self {
        Self::new(
            crate::effects::RemoveAnyCountersAmongEffect::new(count, filter)
                .with_counter_type(counter_type),
        )
    }

    pub fn remove_dynamic_counters_among(
        min_count: u32,
        max_count: u32,
        filter: crate::target::ObjectFilter,
        counter_type: Option<crate::object::CounterType>,
        display_x: bool,
    ) -> Self {
        Self::new(
            crate::effects::RemoveAnyCountersAmongEffect::dynamic(
                min_count, max_count, filter, display_x,
            )
            .with_counter_type(counter_type),
        )
    }

    pub fn choose_card_name(
        player: crate::target::PlayerFilter,
        filter: Option<crate::target::ObjectFilter>,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(crate::effects::ChooseCardNameEffect::new(
            player, filter, tag,
        ))
    }

    pub fn choose_color(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ChooseColorEffect::new(player))
    }

    pub fn choose_card_type(
        player: crate::target::PlayerFilter,
        options: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(crate::effects::ChooseCardTypeEffect::new(player, options))
    }

    pub fn choose_named_option(player: crate::target::PlayerFilter, options: Vec<String>) -> Self {
        Self::new(crate::effects::ChooseNamedOptionEffect::new(
            player, options,
        ))
    }

    pub fn choose_creature_type(
        player: crate::target::PlayerFilter,
        excluded_subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self::new(crate::effects::ChooseCreatureTypeEffect::new(
            player,
            excluded_subtypes,
        ))
    }

    pub fn may_choose_new_targets_player(
        effect_id: impl Into<EffectId>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
            effect_id.into(),
            player,
        ))
    }

    pub fn conditional_only(condition: Condition, if_true: Vec<Effect>) -> Self {
        Self::conditional(condition, if_true, Vec::new())
    }

    pub fn put_sticker(
        target: crate::target::ChooseSpec,
        action: crate::events::KeywordActionKind,
    ) -> Self {
        Self::new(crate::effects::PutStickerEffect::new(target, action))
    }

    pub fn investigate_player(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::InvestigateEffect::new(
            count.into().into_unhinted(),
            player,
        ))
    }

    pub fn incubate(amount: impl Into<Value>, count: impl Into<Value>) -> Self {
        Self::new(crate::effects::IncubateEffect::you(amount, count))
    }

    pub fn incubate_player(
        amount: impl Into<Value>,
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::IncubateEffect::new(amount, count, player))
    }

    pub fn learn() -> Self {
        Self::new(crate::effects::LearnEffect::new())
    }

    pub fn amass(subtype: Option<crate::types::Subtype>, amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::AmassEffect::new(subtype, amount))
    }

    pub fn amplify(amount: u32) -> Self {
        Self::new(crate::effects::AmplifyEffect::new(amount))
    }

    pub fn devour(multiplier: u32) -> Self {
        Self::new(crate::effects::DevourEffect::new(multiplier))
    }

    pub fn reveal_top(
        player: crate::target::PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(crate::effects::RevealTopEffect::tagged(player, tag))
    }

    pub fn reveal_source_from_hand() -> Self {
        Self::new(crate::effects::RevealSourceFromHandEffect::new())
    }

    pub fn cipher() -> Self {
        Self::new(crate::effects::CipherEffect::new())
    }

    pub fn backup(amount: u32, abilities: Vec<crate::ability::Ability>) -> Self {
        Self::new(crate::effects::BackupEffect::new(amount, abilities))
    }

    pub fn flip_coin(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::FlipCoinEffect::new(player))
    }

    pub fn shuffle_objects_into_library(
        target: crate::target::ChooseSpec,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            target, player,
        ))
    }

    pub fn exchange_life_totals(
        player1: crate::target::PlayerFilter,
        player2: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::ExchangeLifeTotalsEffect::new(
            player1, player2,
        ))
    }

    pub fn exchange_zones(
        player: crate::target::PlayerFilter,
        left: crate::zone::Zone,
        right: crate::zone::Zone,
    ) -> Self {
        Self::new(crate::effects::ExchangeZonesEffect::new(
            player, left, right,
        ))
    }

    pub fn aura_swap() -> Self {
        Self::new(crate::effects::AuraSwapEffect::new())
    }

    pub fn exchange_text_boxes(target: crate::target::ChooseSpec) -> Self {
        Self::new(crate::effects::ExchangeTextBoxesEffect::new(target))
    }

    pub fn exchange_values(
        left: crate::effects::ExchangeValueOperand,
        right: crate::effects::ExchangeValueOperand,
        until: Until,
    ) -> Self {
        Self::new(crate::effects::ExchangeValuesEffect::new(
            left, right, until,
        ))
    }

    pub fn grant_play_from_graveyard_until_eot(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::GrantBySpecEffect::new(
            crate::grant::GrantSpec::play_from_graveyard(),
            player,
            crate::grant::GrantDuration::UntilEndOfTurn,
        ))
    }

    pub fn double_mana_pool_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::DoubleManaPoolEffect::new(player))
    }

    pub fn empty_mana_pool_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::EmptyManaPoolEffect::new(player))
    }

    pub fn set_life_total_player(
        amount: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::SetLifeTotalEffect::new(amount, player))
    }

    pub fn end_turn_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::EndTurnEffect::new(player))
    }

    pub fn skip_turn_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::SkipTurnEffect::new(player))
    }

    pub fn skip_combat_phases_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::SkipCombatPhasesEffect::new(player))
    }

    pub fn skip_next_combat_phase_this_turn_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::SkipNextCombatPhaseThisTurnEffect::new(
            player,
        ))
    }

    pub fn skip_draw_step_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::SkipDrawStepEffect::new(player))
    }

    pub fn mill(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::MillEffect::you(count))
    }

    pub fn mill_player(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::MillEffect::new(count, player))
    }

    pub fn additional_land_plays(
        count: Value,
        player: crate::target::PlayerFilter,
        until: Until,
    ) -> Self {
        Self::new(crate::effects::AdditionalLandPlaysEffect::new(
            count, player, until,
        ))
    }

    pub fn grant_next_spell_ability_this_turn(
        player: crate::target::PlayerFilter,
        filter: crate::target::ObjectFilter,
        ability: crate::static_abilities::StaticAbility,
    ) -> Self {
        Self::new(crate::effects::GrantNextSpellAbilityEffect::new(
            player, filter, ability,
        ))
    }

    pub fn grant(
        grantable: crate::grant::Grantable,
        target: crate::target::ChooseSpec,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::new(crate::effects::GrantEffect::new(
            grantable, target, duration,
        ))
    }

    pub fn grant_by_spec(
        spec: crate::grant::GrantSpec,
        player: crate::target::PlayerFilter,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::new(crate::effects::GrantBySpecEffect::new(
            spec, player, duration,
        ))
    }

    pub fn grant_object_ability_to_source(ability: impl Into<crate::ability::Ability>) -> Self {
        Self::new(crate::effects::ApplyContinuousEffect::with_spec(
            crate::target::ChooseSpec::Source,
            crate::continuous::Modification::AddAbilityGeneric(ability.into()),
            Until::Forever,
        ))
    }

    pub fn unearth() -> Self {
        Self::new(crate::effects::UnearthEffect::new())
    }

    pub fn ninjutsu() -> Self {
        Self::new(crate::effects::NinjutsuEffect::new())
    }

    pub fn ring_tempts_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::RingTemptsYouEffect::new(player))
    }

    pub fn venture_into_undercity_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::VentureIntoDungeonEffect::via_initiative(
            player,
        ))
    }

    pub fn venture_into_dungeon_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::VentureIntoDungeonEffect::new(player))
    }

    pub fn become_monarch_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::BecomeMonarchEffect::new(player))
    }

    pub fn take_initiative_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::TakeInitiativeEffect::new(player))
    }

    pub fn cast_tagged(
        tag: crate::tag::TagKey,
        player: crate::target::PlayerFilter,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        cost_reduction: Option<crate::mana::ManaCost>,
    ) -> Self {
        Self::new(crate::effects::CastTaggedEffect {
            tag,
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            cost_reduction,
        })
    }

    pub fn exile_instead_of_graveyard_this_turn(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ExileInsteadOfGraveyardEffect::new(player))
    }

    pub fn control_player(
        player: crate::target::PlayerFilter,
        start: crate::game_state::PlayerControlStart,
        duration: crate::game_state::PlayerControlDuration,
    ) -> Self {
        Self::new(crate::effects::ControlPlayerEffect::new(
            player, start, duration,
        ))
    }

    pub fn control_combat_choices_this_turn(attackers: bool, blockers: bool) -> Self {
        Self::new(crate::effects::ControlCombatChoicesThisTurnEffect::new(
            attackers, blockers,
        ))
    }

    pub fn create_tokens(token: crate::cards::CardDefinition, count: impl Into<Value>) -> Self {
        Self::new(crate::effects::CreateTokenEffect::you(token, count.into()))
    }

    pub fn poison_counters_player(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::PoisonCountersEffect::new(count, player))
    }

    pub fn poison_counters(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::PoisonCountersEffect::you(count))
    }

    pub fn energy_counters(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::EnergyCountersEffect::you(
            count.into().into_unhinted(),
        ))
    }

    pub fn energy_counters_player(
        count: impl Into<Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::EnergyCountersEffect::new(
            count.into().into_unhinted(),
            player,
        ))
    }

    pub fn scry(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::ScryEffect::you(count))
    }

    pub fn scry_player(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ScryEffect::new(count, player))
    }

    pub fn fateseal(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::FatesealEffect::you(count))
    }

    pub fn fateseal_player(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::FatesealEffect::new(count, player))
    }

    pub fn discover(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::DiscoverEffect::you(count))
    }

    pub fn discover_player(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::DiscoverEffect::new(count, player))
    }

    pub fn consult_top_of_library(
        player: crate::target::PlayerFilter,
        mode: crate::effects::consult_helpers::LibraryConsultMode,
        filter: crate::target::ObjectFilter,
        stop_rule: crate::effects::ConsultTopOfLibraryStopRule,
        all_tag: crate::tag::TagKey,
        match_tag: crate::tag::TagKey,
    ) -> Self {
        Self::new(crate::effects::ConsultTopOfLibraryEffect::new(
            player, mode, filter, stop_rule, all_tag, match_tag,
        ))
    }

    pub fn monstrosity(amount: impl Into<Value>) -> Self {
        Self::new(crate::effects::MonstrosityEffect::new(amount))
    }

    pub fn set_base_power_toughness(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        target: crate::target::ChooseSpec,
        until: Until,
    ) -> Self {
        Self::new(crate::effects::SetBasePowerToughnessEffect::new(
            target, power, toughness, until,
        ))
    }

    pub fn shuffle_hand_and_graveyard_into_library_player(
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect::new(player))
    }

    pub fn shuffle_graveyard_into_library_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ShuffleGraveyardIntoLibraryEffect::new(
            player,
        ))
    }

    pub fn reorder_graveyard_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ReorderGraveyardEffect::new(player))
    }

    pub fn cant_until(restriction: Restriction, until: Until) -> Self {
        Self::new(crate::effects::CantEffect::new(restriction, until))
    }

    pub fn extra_turn_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ExtraTurnEffect::new(player))
    }

    pub fn extra_turn_after_next_turn_player(player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::ExtraTurnAfterNextTurnEffect::new(player))
    }

    pub fn additional_phases(phases: Vec<crate::effects::AdditionalPhase>) -> Self {
        Self::new(crate::effects::AdditionalPhasesEffect::new(phases))
    }

    pub fn move_all_counters(
        from: crate::target::ChooseSpec,
        to: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::MoveAllCountersEffect::new(from, to))
    }

    pub fn move_one_counter(
        from: crate::target::ChooseSpec,
        to: crate::target::ChooseSpec,
    ) -> Self {
        Self::new(crate::effects::MoveOneCounterEffect::new(from, to))
    }

    pub fn pump_for_each(
        target: crate::target::ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: Value,
        duration: Until,
    ) -> Self {
        Self::new(crate::effects::ModifyPowerToughnessForEachEffect::new(
            target,
            power_per,
            toughness_per,
            count,
            duration,
        ))
    }

    pub fn surveil(count: impl Into<Value>) -> Self {
        Self::new(crate::effects::SurveilEffect::you(count))
    }

    pub fn surveil_player(count: impl Into<Value>, player: crate::target::PlayerFilter) -> Self {
        Self::new(crate::effects::SurveilEffect::new(count, player))
    }

    pub fn counter_unless_pays(
        target: crate::target::ChooseSpec,
        mana: Vec<crate::mana::ManaSymbol>,
    ) -> Self {
        Self::unless_pays(
            vec![Self::counter(target)],
            crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Target),
            mana,
        )
    }

    pub fn counter_unless_pays_total_cost(
        target: crate::target::ChooseSpec,
        cost: crate::cost::TotalCost,
    ) -> Self {
        let player = match target.base() {
            crate::target::ChooseSpec::SpecificObject(id) => {
                crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Specific(*id))
            }
            crate::target::ChooseSpec::Tagged(tag) => crate::target::PlayerFilter::ControllerOf(
                crate::target::ObjectRef::Tagged(tag.clone()),
            ),
            _ => crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Target),
        };
        Self::unless_pays_total_cost(vec![Self::counter(target)], player, cost)
    }

    pub fn unless_pays_with_life_additional_and_multiplier(
        effects: Vec<Effect>,
        player: crate::target::PlayerFilter,
        mana_symbols_per_counter: Vec<crate::mana::ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
        multiplier: Option<Value>,
    ) -> Self {
        let cost = build_unless_payment_total_cost(
            mana_symbols_per_counter,
            life,
            additional_generic,
            multiplier,
            None,
        );
        Self::new(crate::effects::UnlessPaysEffect {
            player,
            effects,
            cost,
        })
    }

    pub fn tag(self, tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(crate::effects::TaggedEffect::new(tag.into(), self))
    }

    pub fn tag_all(self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.tag(tag)
    }

    pub fn tag_attached_to_source(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(crate::effects::TagAttachedToSourceEffect::new(tag))
    }

    pub fn tag_triggering_object(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(crate::effects::TagTriggeringObjectEffect::new(tag))
    }

    pub fn tag_triggering_source(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(crate::effects::TagTriggeringSourceEffect::new(tag))
    }

    pub fn tag_triggering_damage_target(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(crate::effects::TagTriggeringDamageTargetEffect::new(tag))
    }

    pub fn put_tagged_remainder_on_library_bottom(
        tag: impl Into<crate::tag::TagKey>,
        keep_tagged: Option<crate::tag::TagKey>,
        order: crate::effects::consult_helpers::LibraryBottomOrder,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                tag,
                keep_tagged,
                order,
                player,
            ),
        )
    }
}

impl From<crate::effects::CrewCostEffect> for Effect {
    fn from(value: crate::effects::CrewCostEffect) -> Self {
        Self::new(value)
    }
}

impl From<crate::effects::ExertCostEffect> for Effect {
    fn from(value: crate::effects::ExertCostEffect) -> Self {
        Self::new(value)
    }
}

impl From<crate::effects::ChoosePlayerEffect> for Effect {
    fn from(value: crate::effects::ChoosePlayerEffect) -> Self {
        Self::new(value)
    }
}
