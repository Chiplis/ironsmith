use super::*;

#[derive(Debug, Clone)]
struct PayableManaUnit {
    symbol: crate::mana::ManaSymbol,
    source: Option<ObjectId>,
    provenance_index: Option<usize>,
    restricted_index: Option<usize>,
    from_snow_source: bool,
}

#[derive(Debug, Clone)]
struct ManaPaymentPlan {
    pip_payments: Vec<ManaPipCommit>,
    life_to_pay: u32,
}

#[derive(Debug, Clone, Copy)]
enum ManaPipCommit {
    ManaUnit(usize),
    Life(u32),
}

#[derive(Debug, Clone)]
struct SpentManaUnitCommit {
    symbol: crate::mana::ManaSymbol,
    restriction: Option<crate::ability::RestrictedManaUnit>,
    provenance: Option<crate::player::ManaSourceProvenance>,
}

impl GameState {
    /// Update continuous effects from static abilities on the battlefield.
    ///
    /// This scans all permanents with static abilities that generate continuous
    /// effects (anthems, abilities that grant abilities, etc.) and updates the
    /// ContinuousEffectManager with these effects.
    ///
    /// Per Rule 611.3a, static ability effects apply dynamically.
    pub fn update_static_ability_effects(&mut self) {
        #[cfg(feature = "shadow-continuous")]
        use crate::static_ability_processor::generate_continuous_effects_from_static_abilities;
        use crate::static_ability_processor::generate_continuous_effects_from_static_abilities_cached;

        self.count_static_ability_regen();
        let effects = {
            let mut cache = self.runtime_cache.static_effects_cache.borrow_mut();
            generate_continuous_effects_from_static_abilities_cached(self, &mut cache)
        };
        #[cfg(feature = "shadow-continuous")]
        {
            let expected = generate_continuous_effects_from_static_abilities(self);
            assert_eq!(
                effects, expected,
                "incremental static-ability effect generation diverged from wholesale generation"
            );
        }
        self.effect_store
            .continuous_effects
            .set_static_ability_effects(effects);
        self.mark_continuous_state_clean();
    }

    /// Update replacement effects from static abilities on the battlefield.
    ///
    /// This scans all permanents with static abilities that generate replacement
    /// effects (enters tapped, enters with counters, etc.) and updates the
    /// ReplacementEffectManager with these effects.
    pub fn update_replacement_effects(&mut self) {
        use crate::replacement_ability_processor::generate_replacement_effects_from_abilities;

        // Clear existing static ability replacement effects
        self.effect_store
            .replacement_effects
            .clear_static_ability_effects();

        // Generate and register new ones from current battlefield state
        // without eagerly calculating every permanent. Replacement generation
        // reads printed/granted source abilities; unusual dynamic generators
        // can still request characteristics through the normal on-demand cache.
        let effects = generate_replacement_effects_from_abilities(self);
        for effect in effects {
            let decline = effect.optional_decline_effect();
            self.effect_store
                .replacement_effects
                .add_static_ability_effect(effect);
            if let Some(decline) = decline {
                self.effect_store
                    .replacement_effects
                    .add_static_ability_effect(decline);
            }
        }
    }

    /// Perform a full refresh of all dynamic game state that depends on continuous effects.
    ///
    /// This should be called:
    /// - After state-based actions are checked
    /// - Before processing priority or combat decisions
    /// - After permanents enter or leave the battlefield
    ///
    /// It updates:
    /// - Static ability continuous effects (anthems, etc.)
    /// - Replacement effects from static abilities
    /// - "Can't" effect tracking
    pub fn refresh_continuous_state(&mut self) {
        if self.continuous_state_is_clean() {
            return;
        }

        // Update continuous effects from static abilities
        self.update_static_ability_effects();
        self.reconcile_continuous_control_changes();

        // A continuous effect may itself inspect summoning-sickness state.
        // Rebuild once after a controller transition so those predicates see
        // the newly sick permanent in this same refresh transaction.
        if !self.continuous_state_is_clean() {
            self.update_static_ability_effects();
            self.reconcile_continuous_control_changes();
        }

        // Update replacement effects from static abilities
        self.update_replacement_effects();

        // Update "can't" effect tracking
        self.update_cant_effects();

        if self.apply_day_nightbound_transformations_with_current_restrictions() {
            self.update_static_ability_effects();
            self.reconcile_continuous_control_changes();
            self.update_replacement_effects();
            self.update_cant_effects();
        }

        // Ascend on a permanent is a static ability, not a trigger. Its check
        // happens only after continuous effects have been reapplied. Earning
        // the blessing can itself turn on conditional continuous abilities,
        // so refresh those effects once more when a designation is granted.
        if self.grant_citys_blessings_from_permanent_ascend() {
            self.update_static_ability_effects();
            self.reconcile_continuous_control_changes();
            self.update_replacement_effects();
            self.update_cant_effects();
        }
    }

    /// Translate changes in derived control into CR 302.6 state.
    ///
    /// This comparison belongs at the continuous-state boundary rather than
    /// only in "gain control" executors: static abilities and expired effects
    /// can change a permanent's controller without executing such an effect.
    pub(crate) fn reconcile_continuous_control_changes(&mut self) {
        let controllers = self
            .battlefield
            .iter()
            .copied()
            .filter_map(|id| {
                self.current_controller(id)
                    .map(|controller| (id, controller))
            })
            .collect::<HashMap<_, _>>();
        let changed = controllers
            .iter()
            .filter_map(|(&id, &controller)| {
                self.battlefield_flags
                    .controller_at_last_refresh
                    .get(&id)
                    .is_some_and(|previous| *previous != controller)
                    .then_some(id)
            })
            .collect::<Vec<_>>();

        self.battlefield_flags_mut().controller_at_last_refresh = controllers;
        for id in changed {
            self.set_summoning_sick(id);
        }
    }

    fn grant_citys_blessings_from_permanent_ascend(&mut self) -> bool {
        let ascend_controllers = self
            .battlefield
            .iter()
            .copied()
            .filter(|&object_id| {
                self.current_has_static_ability_id(
                    object_id,
                    crate::static_abilities::StaticAbilityId::Ascend,
                )
            })
            .filter_map(|object_id| self.controller_of_id(object_id))
            .collect::<HashSet<_>>();

        let newly_blessed = ascend_controllers
            .into_iter()
            .filter(|&player| {
                !self.has_citys_blessing(player)
                    && self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&object_id| self.controller_of_id(object_id) == Some(player))
                        .count()
                        >= 10
            })
            .collect::<Vec<_>>();

        for player in &newly_blessed {
            self.grant_citys_blessing(*player);
        }
        !newly_blessed.is_empty()
    }

    pub fn library_top_revision(&self, player: PlayerId) -> u64 {
        self.effect_store
            .library_top_revisions
            .get(&player)
            .copied()
            .unwrap_or(0)
    }

    pub(super) fn bump_library_top_revision(&mut self, player: PlayerId) {
        let revision = self
            .effect_store
            .library_top_revisions
            .entry(player)
            .or_insert(0);
        *revision = revision.saturating_add(1);
        self.mark_continuous_state_dirty();
    }

    /// Check if a player may spend mana as though it were mana of any color.
    ///
    /// If `source` is provided, this also checks for source-specific activation permissions.
    pub fn can_spend_mana_as_any_color(&self, payer: PlayerId, source: Option<ObjectId>) -> bool {
        self.effect_store
            .mana_spend_effects
            .permissions
            .iter()
            .any(|permission| {
                permission.allows(self, payer, source)
                    && permission.permission.any_color_mana_symbol.is_none()
                    && permission.permission.mode.allows_any_color()
            })
    }

    pub fn mana_spend_policy(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
    ) -> crate::player::ManaSpendPolicy {
        let mut policy = crate::player::ManaSpendPolicy::default();
        for permission in &self.effect_store.mana_spend_effects.permissions {
            if !permission.allows(self, payer, source) {
                continue;
            }
            if let Some(symbol) = permission.permission.any_color_mana_symbol {
                policy.add_symbol_as_any_color(symbol);
            } else {
                policy.allow_mode(permission.permission.mode);
            }
            policy.other_mana_only_as_colorless |=
                permission.permission.other_mana_only_as_colorless;
        }
        policy
    }

    pub fn can_spend_mana_as_any_color_from_mana_source(
        &self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        mana_source: ObjectId,
    ) -> bool {
        self.effect_store
            .mana_spend_effects
            .permissions
            .iter()
            .any(|permission| {
                permission.allows_for_mana_source(self, payer, payment_source, mana_source)
            })
    }

    pub fn has_source_filtered_mana_spend_permission(
        &self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
    ) -> bool {
        self.effect_store
            .mana_spend_effects
            .permissions
            .iter()
            .any(|permission| {
                permission.allows_with_source_filtered_mana(self, payer, payment_source)
            })
    }

    pub fn cast_origin_snapshot(&self, stack_id: ObjectId) -> Option<&ObjectSnapshot> {
        self.exile_tracking.cast_origin_snapshots.get(&stack_id)
    }

    pub fn set_cast_origin_snapshot(&mut self, stack_id: ObjectId, snapshot: ObjectSnapshot) {
        self.exile_tracking_mut()
            .cast_origin_snapshots
            .insert(stack_id, snapshot);
    }

    fn with_active_battlefield_static_abilities<T>(
        &self,
        f: impl FnMut(ObjectId, PlayerId, &crate::static_abilities::StaticAbility) -> Option<T>,
    ) -> Option<T> {
        let all_effects = self.all_continuous_effects();
        self.with_active_battlefield_static_abilities_with_effects(&all_effects, f)
    }

    fn with_active_battlefield_static_abilities_with_effects<T>(
        &self,
        all_effects: &[ContinuousEffect],
        mut f: impl FnMut(ObjectId, PlayerId, &crate::static_abilities::StaticAbility) -> Option<T>,
    ) -> Option<T> {
        for &perm_id in &self.battlefield {
            let Some(object) = self.object(perm_id) else {
                continue;
            };
            let static_abilities = self
                .calculated_characteristics_with_effects(perm_id, all_effects)
                .map(|chars| chars.static_abilities)
                .unwrap_or_default();
            for static_ability in static_abilities {
                if !static_ability.is_active(self, perm_id) {
                    continue;
                }
                if let Some(result) = f(perm_id, self.controller_of(object), &static_ability) {
                    return Some(result);
                }
            }
        }
        None
    }

    pub fn player_can_pay_black_with_life(
        &self,
        payer: PlayerId,
        _source: Option<ObjectId>,
    ) -> bool {
        self.with_active_battlefield_static_abilities(|_, controller, ability| {
            (controller == payer && ability.black_mana_may_be_paid_with_life()).then_some(true)
        })
        .unwrap_or(false)
    }

    pub fn player_can_pay_black_with_life_for_reason(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        self.player_can_pay_black_with_life(payer, source)
            && (!reason.is_cast_or_ability_payment()
                || !self.player_cant_pay_life_to_cast_or_activate(payer))
    }

    pub fn minimum_total_spell_mana_payment(&self) -> Option<u32> {
        DerivedGameView::new(self).minimum_total_spell_mana_payment()
    }

    pub fn player_cant_pay_life_to_cast_or_activate(&self, player: PlayerId) -> bool {
        if self.player(player).is_none() || !self.may_have_cast_or_activate_payment_restriction() {
            return false;
        }
        self.with_active_battlefield_static_abilities(|_, _, ability| {
            ability
                .forbids_paying_life_for_cast_or_activate()
                .then_some(true)
        })
        .unwrap_or(false)
    }

    pub(crate) fn player_cant_pay_life_to_cast_or_activate_with_effects(
        &self,
        player: PlayerId,
        all_effects: &[ContinuousEffect],
    ) -> bool {
        if self.player(player).is_none()
            || !self.may_have_cast_or_activate_payment_restriction_with_effects(all_effects)
        {
            return false;
        }
        self.with_active_battlefield_static_abilities_with_effects(all_effects, |_, _, ability| {
            ability
                .forbids_paying_life_for_cast_or_activate()
                .then_some(true)
        })
        .unwrap_or(false)
    }

    pub fn player_cant_sacrifice_nonland_to_cast_or_activate(&self, player: PlayerId) -> bool {
        if self.player(player).is_none() || !self.may_have_cast_or_activate_payment_restriction() {
            return false;
        }
        self.with_active_battlefield_static_abilities(|_, _, ability| {
            ability
                .forbids_sacrificing_nonland_for_cast_or_activate()
                .then_some(true)
        })
        .unwrap_or(false)
    }

    fn may_have_cast_or_activate_payment_restriction(&self) -> bool {
        let cache_key = PaymentRestrictionPresenceCache {
            mutation_revision: self.mutation_revision,
            effect_revision: self.effect_store.continuous_effects.revision(),
            turn_number: self.turn.turn_number,
            active_player: self.turn.active_player,
            phase: self.turn.phase,
            step: self.turn.step,
            may_have_restriction: false,
        };
        if let Some(cached) = self.runtime_cache.payment_restriction_presence.get()
            && cached.mutation_revision == cache_key.mutation_revision
            && cached.effect_revision == cache_key.effect_revision
            && cached.turn_number == cache_key.turn_number
            && cached.active_player == cache_key.active_player
            && cached.phase == cache_key.phase
            && cached.step == cache_key.step
        {
            return cached.may_have_restriction;
        }

        let all_effects = if self.continuous_state_is_clean() {
            self.cached_continuous_effects_snapshot_arc()
        } else {
            Arc::new(self.all_continuous_effects())
        };
        let may_have_restriction =
            self.may_have_cast_or_activate_payment_restriction_with_effects(&all_effects);
        self.runtime_cache.payment_restriction_presence.set(Some(
            PaymentRestrictionPresenceCache {
                may_have_restriction,
                ..cache_key
            },
        ));
        may_have_restriction
    }

    fn may_have_cast_or_activate_payment_restriction_with_effects(
        &self,
        all_effects: &[ContinuousEffect],
    ) -> bool {
        let effects_may_introduce_restriction = all_effects.iter().any(|effect| {
            Self::modification_may_introduce_payment_restriction(&effect.modification)
        });
        let printed_or_granted_restriction = self.battlefield.iter().copied().any(|object_id| {
            let Some(object) = self.object(object_id) else {
                return false;
            };
            object.abilities.iter().any(|ability| {
                ability.functions_in(&object.zone)
                    && matches!(&ability.kind, AbilityKind::Static(static_ability)
                        if Self::is_cast_or_activate_payment_restriction(static_ability))
            }) || object
                .level_granted_abilities()
                .iter()
                .any(Self::is_cast_or_activate_payment_restriction)
                || object
                    .temporary_static_ability_grants
                    .iter()
                    .filter(|grant| !grant.is_expired(self.turn.turn_number))
                    .filter_map(|grant| grant.materialize())
                    .any(|ability| Self::is_cast_or_activate_payment_restriction(&ability))
        });
        effects_may_introduce_restriction || printed_or_granted_restriction
    }

    fn modification_may_introduce_payment_restriction(modification: &Modification) -> bool {
        match modification {
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_) => true,
            Modification::AddAbility(static_ability) => {
                Self::is_cast_or_activate_payment_restriction(static_ability)
            }
            Modification::AddAbilityGeneric(ability) => {
                Self::ability_is_cast_or_activate_payment_restriction(ability)
            }
            Modification::SetAbilities(abilities) => abilities
                .iter()
                .any(Self::ability_is_cast_or_activate_payment_restriction),
            _ => false,
        }
    }

    fn ability_is_cast_or_activate_payment_restriction(ability: &crate::ability::Ability) -> bool {
        matches!(&ability.kind, AbilityKind::Static(static_ability)
            if Self::is_cast_or_activate_payment_restriction(static_ability))
    }

    fn is_cast_or_activate_payment_restriction(
        ability: &crate::static_abilities::StaticAbility,
    ) -> bool {
        ability.forbids_paying_life_for_cast_or_activate()
            || ability.forbids_sacrificing_nonland_for_cast_or_activate()
    }

    /// Return the active non-layered battlefield abilities that can make
    /// another permanent enter as a copy.
    ///
    /// `None` is a deliberate fallback signal: at least one continuous effect
    /// can introduce or remove such an ability, so the caller must inspect
    /// fully calculated characteristics. `Some` is safe to use directly and
    /// is cached by every revision that can alter ability presence/activity.
    pub(crate) fn sparse_enter_as_copy_source_abilities(
        &self,
    ) -> Option<Arc<Vec<(ObjectId, StaticAbility)>>> {
        let cache_key = EnterAsCopySourceCache {
            mutation_revision: self.mutation_revision,
            effect_revision: self.effect_store.continuous_effects.revision(),
            zone_revision: self.zone_revisions.battlefield,
            continuous_context_revision: self.continuous_context_revision(),
            turn_number: self.turn.turn_number,
            active_player: self.turn.active_player,
            phase: self.turn.phase,
            step: self.turn.step,
            sparse_candidates: None,
        };
        if let Some(cached) = self.runtime_cache.enter_as_copy_sources.borrow().as_ref()
            && cached.mutation_revision == cache_key.mutation_revision
            && cached.effect_revision == cache_key.effect_revision
            && cached.zone_revision == cache_key.zone_revision
            && cached.continuous_context_revision == cache_key.continuous_context_revision
            && cached.turn_number == cache_key.turn_number
            && cached.active_player == cache_key.active_player
            && cached.phase == cache_key.phase
            && cached.step == cache_key.step
        {
            return cached.sparse_candidates.clone();
        }

        let all_effects = if self.continuous_state_is_clean() {
            self.cached_continuous_effects_snapshot_arc()
        } else {
            Arc::new(self.all_continuous_effects())
        };
        let requires_layered_fallback = all_effects.iter().any(|effect| {
            Self::modification_may_change_enter_as_copy_presence(&effect.modification)
        });

        let sparse_candidates = (!requires_layered_fallback).then(|| {
            let mut candidates = Vec::new();
            for &object_id in &self.battlefield {
                let Some(object) = self.object(object_id) else {
                    continue;
                };
                let mut active_abilities = Vec::new();

                for ability in object.abilities.iter() {
                    let AbilityKind::Static(static_ability) = &ability.kind else {
                        continue;
                    };
                    if ability.functions_in(&object.zone)
                        && static_ability.enter_as_copy_as_enters().is_some()
                        && static_ability.is_active(self, object_id)
                        && !active_abilities.contains(static_ability)
                    {
                        active_abilities.push(static_ability.clone());
                    }
                }
                for static_ability in object.level_granted_abilities() {
                    if static_ability.enter_as_copy_as_enters().is_some()
                        && static_ability.is_active(self, object_id)
                        && !active_abilities.contains(&static_ability)
                    {
                        active_abilities.push(static_ability);
                    }
                }
                for grant in &object.temporary_static_ability_grants {
                    let Some(static_ability) = grant.materialize() else {
                        continue;
                    };
                    if static_ability.enter_as_copy_as_enters().is_some()
                        && static_ability.is_active(self, object_id)
                        && !active_abilities.contains(&static_ability)
                    {
                        active_abilities.push(static_ability);
                    }
                }

                candidates.extend(
                    active_abilities
                        .into_iter()
                        .map(|ability| (object_id, ability)),
                );
            }
            Arc::new(candidates)
        });

        *self.runtime_cache.enter_as_copy_sources.borrow_mut() = Some(EnterAsCopySourceCache {
            sparse_candidates: sparse_candidates.clone(),
            ..cache_key
        });
        sparse_candidates
    }

    fn modification_may_change_enter_as_copy_presence(modification: &Modification) -> bool {
        match modification {
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_)
            | Modification::SetAbilities(_)
            | Modification::CopyStaticAbilityVariants { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana => true,
            Modification::AddAbility(static_ability)
            | Modification::RemoveAbility(static_ability) => {
                Self::static_ability_may_provide_enter_as_copy(static_ability)
            }
            Modification::AddAbilityGeneric(ability)
            | Modification::RemoveAbilityGeneric { ability, .. } => matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if Self::static_ability_may_provide_enter_as_copy(static_ability)
            ),
            Modification::ChangeController(_)
            | Modification::SetName(_)
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::CopyActivatedAbilities { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::CantBeBlocked
            | Modification::CantAttack
            | Modification::CantBlock
            | Modification::DoesntUntap
            | Modification::SetPower { .. }
            | Modification::SetToughness { .. }
            | Modification::SetPowerToughness { .. }
            | Modification::ModifyPower(_)
            | Modification::ModifyToughness(_)
            | Modification::ModifyPowerToughness { .. }
            | Modification::ModifyPowerToughnessValue { .. }
            | Modification::ModifyPowerToughnessByColorCount { .. }
            | Modification::SwitchPowerToughness => false,
        }
    }

    fn static_ability_may_provide_enter_as_copy(static_ability: &StaticAbility) -> bool {
        static_ability.enter_as_copy_as_enters().is_some()
            || static_ability.level_abilities().is_some_and(|levels| {
                levels.iter().any(|tier| {
                    tier.abilities
                        .iter()
                        .any(Self::static_ability_may_provide_enter_as_copy)
                })
            })
    }

    pub fn player_skips_upkeep_step(&self, player: PlayerId) -> bool {
        if !self.may_have_player_skips_upkeep_static_ability() {
            return false;
        }
        self.with_active_battlefield_static_abilities(|source, controller, ability| {
            ability
                .skips_upkeep_for_player(self, source, controller, player)
                .then_some(true)
        })
        .unwrap_or(false)
            && self.player(player).is_some()
    }

    /// Whether an active battlefield static ability makes this player skip
    /// their draw step. Unlike one-shot skip effects, this is derived from the
    /// source's current controller and ends as soon as the source leaves.
    pub fn player_skips_draw_step(&self, player: PlayerId) -> bool {
        self.with_active_battlefield_static_abilities(|source, controller, ability| {
            ability
                .skips_draw_step_for_player(self, source, controller, player)
                .then_some(true)
        })
        .unwrap_or(false)
            && self.player(player).is_some()
    }

    /// Whether an active battlefield static ability replaces this player's
    /// next extra turn with skipping that turn. The query is evaluated when
    /// the queued turn would begin, so removing the source restores later
    /// extra turns immediately.
    pub fn player_skips_extra_turn(&self, player: PlayerId) -> bool {
        self.with_active_battlefield_static_abilities(|source, controller, ability| {
            ability
                .skips_extra_turn_for_player(self, source, controller, player)
                .then_some(true)
        })
        .unwrap_or(false)
            && self.player(player).is_some()
    }

    fn may_have_player_skips_upkeep_static_ability(&self) -> bool {
        use crate::ability::AbilityKind;
        use crate::static_abilities::StaticAbilityId;

        if self
            .cached_continuous_effects_snapshot()
            .iter()
            .any(|effect| {
                Self::modification_may_grant_static_ability_id(
                    &effect.modification,
                    StaticAbilityId::PlayersSkipUpkeep,
                )
            })
        {
            return true;
        }

        self.objects.values().any(|object| {
            if !matches!(object.zone, Zone::Battlefield | Zone::Stack) {
                return false;
            }
            object.abilities.iter().any(|ability| {
                ability.functions_in(&object.zone)
                    && matches!(&ability.kind, AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::PlayersSkipUpkeep)
            })
        })
    }

    fn modification_may_grant_static_ability_id(
        modification: &Modification,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        use crate::static_abilities::StaticAbilityId;
        match modification {
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_) => true,
            Modification::AddAbility(static_ability) => static_ability.id() == ability_id,
            Modification::AddAbilityGeneric(ability) => {
                Self::ability_may_grant_static_ability_id(ability, ability_id)
            }
            Modification::SetAbilities(abilities) => abilities
                .iter()
                .any(|ability| Self::ability_may_grant_static_ability_id(ability, ability_id)),
            // Restriction modifications materialize as static abilities in
            // calculated characteristics (see apply path in continuous.rs).
            Modification::CantBeBlocked => ability_id == StaticAbilityId::Unblockable,
            Modification::CantAttack => ability_id == StaticAbilityId::Defender,
            Modification::CantBlock => ability_id == StaticAbilityId::CantBlock,
            Modification::DoesntUntap => ability_id == StaticAbilityId::DoesntUntap,
            _ => false,
        }
    }

    fn ability_may_grant_static_ability_id(
        ability: &crate::ability::Ability,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ability_id)
    }

    fn object_is_land_for_cost_restrictions(&self, object_id: ObjectId) -> bool {
        let Some(object) = self.object(object_id) else {
            return false;
        };
        if object.zone == Zone::Battlefield {
            return self
                .calculated_characteristics(object_id)
                .is_some_and(|chars| chars.card_types.contains(&crate::types::CardType::Land));
        }
        object.card_types.contains(&crate::types::CardType::Land)
    }

    pub(crate) fn object_is_room_unlock_payment_source(&self, object_id: ObjectId) -> bool {
        self.room_has_locked_door(object_id)
    }

    pub(crate) fn room_has_locked_door(&self, object_id: ObjectId) -> bool {
        let Some(object) = self.object(object_id) else {
            return false;
        };
        object.zone == Zone::Battlefield
            && self.current_has_subtype(object_id, crate::types::Subtype::Room)
            && object.linked_face_layout == LinkedFaceLayout::Split
            && !self
                .battlefield_flags
                .fully_unlocked_rooms
                .contains(&object_id)
            && self
                .linked_face_definition_by_name_or_id(
                    object.other_face_name.as_deref(),
                    object.other_face,
                )
                .is_some_and(|def| def.card.subtypes.contains(&crate::types::Subtype::Room))
    }

    pub(crate) fn mark_room_fully_unlocked(&mut self, object_id: ObjectId) {
        self.battlefield_flags_mut()
            .fully_unlocked_rooms
            .insert(object_id);
    }

    fn required_sacrifice_count_for_cost(&self, cost: &crate::costs::Cost) -> usize {
        if cost.is_sacrifice_self() {
            return 1;
        }
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            .and_then(|effect| match effect.count {
                crate::effect::Value::Fixed(count) => Some(count.max(0) as usize),
                _ => None,
            })
            .unwrap_or(1)
    }

    fn legal_sacrifice_targets_for_cost(
        &self,
        payer: PlayerId,
        source: ObjectId,
        filter: &crate::filter::ObjectFilter,
        lands_only: bool,
    ) -> usize {
        let filter_ctx = crate::filter::FilterContext::new(payer).with_source(source);
        self.battlefield
            .iter()
            .filter_map(|&id| self.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                self.controller_of(obj) == payer
                    && (!lands_only || self.object_is_land_for_cost_restrictions(*id))
                    && filter.matches(obj, &filter_ctx, self)
                    && self.can_be_sacrificed(*id)
            })
            .count()
    }

    pub fn validate_cost_for_payment_reason(
        &self,
        payer: PlayerId,
        source: ObjectId,
        cost: &crate::costs::Cost,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::cost::CostPaymentError> {
        if !reason.is_cast_or_ability_payment() {
            return Ok(());
        }

        if self.player_cant_pay_life_to_cast_or_activate(payer) && cost.is_life_cost() {
            return Err(crate::cost::CostPaymentError::InsufficientLife);
        }

        let lands_only = self.player_cant_sacrifice_nonland_to_cast_or_activate(payer);

        if cost.is_sacrifice_self() {
            if lands_only && !self.object_is_land_for_cost_restrictions(source) {
                return Err(crate::cost::CostPaymentError::NoValidSacrificeTarget);
            }
            if !self.can_be_sacrificed(source) {
                return Err(crate::cost::CostPaymentError::NoValidSacrificeTarget);
            }
        }

        if let Some(filter) = cost.sacrifice_filter() {
            // Choose-then-sacrifice activation costs often use a tagged filter for the
            // follow-up sacrifice step. That tag is unresolved during precheck, so only
            // validate concrete sacrifice filters here and let the staged cost flow
            // validate the tagged selection after the player chooses an object.
            if !filter.tagged_constraints.is_empty() {
                return Ok(());
            }
            let required = self.required_sacrifice_count_for_cost(cost);
            if self.legal_sacrifice_targets_for_cost(payer, source, filter, lands_only) < required {
                return Err(crate::cost::CostPaymentError::NoValidSacrificeTarget);
            }
        }

        Ok(())
    }

    pub fn adjust_mana_cost_for_payment_reason(
        &self,
        payer: PlayerId,
        _source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        reason: crate::costs::PaymentReason,
    ) -> crate::mana::ManaCost {
        use crate::mana::ManaSymbol;

        let mut pips = cost.pips().to_vec();

        if reason.is_cast_or_ability_payment()
            && self.player_cant_pay_life_to_cast_or_activate(payer)
        {
            for pip in &mut pips {
                pip.retain(|symbol| !matches!(symbol, ManaSymbol::Life(_)));
            }
        }

        crate::mana::ManaCost::from_pips(pips)
    }

    /// Check if a player can pay a mana cost, accounting for "spend as though any color".
    pub fn can_pay_mana_cost(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        self.can_pay_mana_cost_with_reason(
            payer,
            source,
            cost,
            x_value,
            crate::costs::PaymentReason::Other,
        )
    }

    fn cast_spell_mana_rule_matches_payment_source(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        card_types: &[CardType],
        subtype_requirement: &Option<crate::ability::ManaUsageSubtypeRequirement>,
        payment_source: Option<ObjectId>,
    ) -> bool {
        let Some(source_id) = payment_source else {
            return false;
        };
        let Some(source_obj) = self.object(source_id) else {
            return false;
        };
        if source_obj.zone != Zone::Stack {
            return false;
        }
        if !card_types
            .iter()
            .all(|card_type| self.current_has_card_type(source_obj.id, *card_type))
        {
            return false;
        }

        let required_subtype = match subtype_requirement {
            Some(crate::ability::ManaUsageSubtypeRequirement::Exact(subtype)) => Some(*subtype),
            Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource) => {
                unit.source_chosen_creature_type
            }
            None => None,
        };
        required_subtype.is_none_or(|subtype| self.current_has_subtype(source_obj.id, subtype))
    }

    fn cast_spell_filter_matches_payment_source(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        filter: &crate::target::ObjectFilter,
        payment_source: Option<ObjectId>,
    ) -> bool {
        let Some(source_id) = payment_source else {
            return false;
        };
        let Some(source_obj) = self.object(source_id) else {
            return false;
        };
        if source_obj.zone != Zone::Stack {
            return false;
        }

        let Some(mana_source) = self.object(unit.source) else {
            return false;
        };
        let filter_ctx =
            self.filter_context_for(self.controller_of(mana_source), Some(unit.source));
        filter.matches(source_obj, &filter_ctx, self)
    }

    fn activate_ability_source_filter_matches_payment_source(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        filter: &crate::target::ObjectFilter,
        payment_source: Option<ObjectId>,
    ) -> bool {
        let Some(source_id) = payment_source else {
            return false;
        };
        let Some(source_obj) = self.object(source_id) else {
            return false;
        };
        if source_obj.zone == Zone::Stack {
            return false;
        }

        let Some(mana_source) = self.object(unit.source) else {
            return false;
        };
        let filter_ctx =
            self.filter_context_for(self.controller_of(mana_source), Some(unit.source));
        filter.matches(source_obj, &filter_ctx, self)
    }

    pub(crate) fn mana_payment_predicate_matches(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        predicate: &crate::ability::ManaPaymentPredicate,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        cost: Option<&crate::mana::ManaCost>,
    ) -> bool {
        let effective_cost = cost.or_else(|| {
            payment_source
                .and_then(|source| self.object(source))
                .and_then(|object| object.mana_cost.as_deref())
        });
        match predicate {
            crate::ability::ManaPaymentPredicate::Any => true,
            crate::ability::ManaPaymentPredicate::Purpose(purpose) => {
                reason.mana_payment_purpose() == *purpose
            }
            crate::ability::ManaPaymentPredicate::SourceMatches(filter) => {
                let Some(source_id) = payment_source else {
                    return false;
                };
                let Some(source_obj) = self.object(source_id) else {
                    return false;
                };
                let controller = self
                    .object(unit.source)
                    .map(|mana_source| self.controller_of(mana_source))
                    .unwrap_or_else(|| self.controller_of(source_obj));
                let filter_ctx = self.filter_context_for(controller, Some(unit.source));
                filter.matches(source_obj, &filter_ctx, self)
                    || (reason == crate::costs::PaymentReason::CastSpell
                        && source_obj.zone == Zone::Stack
                        && self.cast_origin_snapshot(source_id).is_some_and(|origin| {
                            filter.matches_snapshot(origin, &filter_ctx, self)
                        }))
            }
            crate::ability::ManaPaymentPredicate::CostContains(symbol) => effective_cost
                .is_some_and(|cost| cost.pips().iter().any(|pip| pip.contains(symbol))),
            crate::ability::ManaPaymentPredicate::CostContainsX => {
                effective_cost.is_some_and(crate::mana::ManaCost::has_x)
            }
            crate::ability::ManaPaymentPredicate::SharesCreatureTypeWithPayersCommander => {
                let Some(source_id) = payment_source else {
                    return false;
                };
                let Some(source_obj) = self.object(source_id) else {
                    return false;
                };
                let payer = self.controller_of(source_obj);
                let Some(source_subtypes) = self.current_subtypes(source_id) else {
                    return false;
                };
                self.player(payer).is_some_and(|player| {
                    player.get_commanders().iter().copied().any(|commander| {
                        self.current_commander_object(commander)
                            .is_some_and(|commander| {
                                self.current_subtypes(commander)
                                    .is_some_and(|commander_subtypes| {
                                        commander_subtypes
                                            .iter()
                                            .any(|subtype| source_subtypes.contains(subtype))
                                    })
                            })
                    })
                })
            }
            crate::ability::ManaPaymentPredicate::All(predicates) => {
                predicates.iter().all(|part| {
                    self.mana_payment_predicate_matches(
                        unit,
                        part,
                        payment_source,
                        reason,
                        effective_cost,
                    )
                })
            }
            crate::ability::ManaPaymentPredicate::AnyOf(predicates) => {
                predicates.iter().any(|part| {
                    self.mana_payment_predicate_matches(
                        unit,
                        part,
                        payment_source,
                        reason,
                        effective_cost,
                    )
                })
            }
            crate::ability::ManaPaymentPredicate::Not(predicate) => !self
                .mana_payment_predicate_matches(
                    unit,
                    predicate,
                    payment_source,
                    reason,
                    effective_cost,
                ),
        }
    }

    pub(crate) fn restricted_mana_unit_is_payable_for_reason(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        self.restricted_mana_unit_is_payable_for_transaction(unit, payment_source, reason, None)
    }

    pub(crate) fn restricted_mana_unit_is_payable_for_transaction(
        &self,
        unit: &crate::ability::RestrictedManaUnit,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        cost: Option<&crate::mana::ManaCost>,
    ) -> bool {
        unit.restrictions
            .iter()
            .all(|restriction| {
                match restriction {
            crate::ability::ManaUsageRestriction::CastSpell {
                card_types,
                subtype_requirement,
                restrict_to_matching_spell,
                ..
            } => {
                !*restrict_to_matching_spell
                    || (reason == crate::costs::PaymentReason::CastSpell
                        && self.cast_spell_mana_rule_matches_payment_source(
                            unit,
                            card_types,
                            subtype_requirement,
                            payment_source,
                        ))
            }
            crate::ability::ManaUsageRestriction::CastSpellMatching {
                filter,
                restrict_to_matching_spell,
                ..
            } => {
                !*restrict_to_matching_spell
                    || (reason == crate::costs::PaymentReason::CastSpell
                        && self.cast_spell_filter_matches_payment_source(
                            unit,
                            filter,
                            payment_source,
                        ))
            }
            crate::ability::ManaUsageRestriction::CastSpellWithManaBonus { .. } => true,
            crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
                spell_filter,
                ability_source_filter,
            } => {
                (reason == crate::costs::PaymentReason::CastSpell
                    && self.cast_spell_filter_matches_payment_source(
                        unit,
                        spell_filter,
                        payment_source,
                    ))
                    || (matches!(
                        reason,
                        crate::costs::PaymentReason::ActivateAbility
                            | crate::costs::PaymentReason::ActivateManaAbility
                    ) && self.activate_ability_source_filter_matches_payment_source(
                        unit,
                        ability_source_filter,
                        payment_source,
                    ))
            }
            crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
                spell_filter,
            } => {
                (reason == crate::costs::PaymentReason::CastSpell
                    && self.cast_spell_filter_matches_payment_source(
                        unit,
                        spell_filter,
                        payment_source,
                    ))
                    || (reason == crate::costs::PaymentReason::TurnFaceUp
                        && payment_source.is_some_and(|source_id| {
                            self.object(source_id)
                                .is_some_and(|source_obj| source_obj.zone == Zone::Battlefield)
                                && self.is_face_down(source_id)
                        }))
                    || (reason == crate::costs::PaymentReason::UnlockDoor
                        && payment_source.is_some_and(|source_id| {
                            self.object_is_room_unlock_payment_source(source_id)
                        }))
            }
            crate::ability::ManaUsageRestriction::ActivateAbility => {
                matches!(
                    reason,
                    crate::costs::PaymentReason::ActivateAbility
                        | crate::costs::PaymentReason::ActivateManaAbility
                ) && payment_source.is_some_and(|source_id| {
                    self.object(source_id)
                        .is_some_and(|source_obj| source_obj.zone != Zone::Stack)
                })
            }
            crate::ability::ManaUsageRestriction::PaymentTransaction {
                restriction, ..
            } => restriction.as_ref().is_none_or(|predicate| {
                self.mana_payment_predicate_matches(
                    unit,
                    predicate,
                    payment_source,
                    reason,
                    cost,
                )
            }),
        }
            })
    }

    fn mana_provenance_is_from_snow_source(
        &self,
        unit: &crate::player::ManaSourceProvenance,
    ) -> bool {
        unit.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.supertypes.contains(&crate::types::Supertype::Snow))
            || (unit.snapshot.is_none()
                && self.current_has_supertype(unit.source, crate::types::Supertype::Snow))
    }

    fn payable_mana_units(
        &self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        cost: &crate::mana::ManaCost,
        required_pool_symbol: Option<crate::mana::ManaSymbol>,
    ) -> Vec<PayableManaUnit> {
        use crate::mana::ManaSymbol;

        const SYMBOLS: [ManaSymbol; 6] = [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ];

        let Some(player) = self.player(payer) else {
            return Vec::new();
        };
        let mut represented = crate::player::ManaPool::default();
        let mut used_restricted = std::collections::HashSet::new();
        let mut units = Vec::new();

        for (provenance_index, provenance) in player.mana_source_provenance.iter().enumerate() {
            if !SYMBOLS.contains(&provenance.symbol)
                || represented.amount(provenance.symbol)
                    >= player.mana_pool.amount(provenance.symbol)
            {
                continue;
            }
            represented.add(provenance.symbol, 1);

            let restricted_index = if provenance.restricted {
                player
                    .restricted_mana
                    .iter()
                    .enumerate()
                    .find(|(index, unit)| {
                        !used_restricted.contains(index)
                            && unit.symbol == provenance.symbol
                            && unit.source == provenance.source
                    })
                    .map(|(index, _)| index)
            } else {
                None
            };
            if provenance.restricted && restricted_index.is_none() {
                continue;
            }
            if let Some(index) = restricted_index {
                used_restricted.insert(index);
                if !self.restricted_mana_unit_is_payable_for_transaction(
                    &player.restricted_mana[index],
                    payment_source,
                    reason,
                    Some(cost),
                ) {
                    continue;
                }
            }
            if required_pool_symbol.is_some_and(|required| required != provenance.symbol) {
                continue;
            }

            units.push(PayableManaUnit {
                symbol: provenance.symbol,
                source: Some(provenance.source),
                provenance_index: Some(provenance_index),
                restricted_index,
                from_snow_source: self.mana_provenance_is_from_snow_source(provenance),
            });
        }

        for symbol in SYMBOLS {
            if required_pool_symbol.is_some_and(|required| required != symbol) {
                continue;
            }
            let untracked = player
                .mana_pool
                .amount(symbol)
                .saturating_sub(represented.amount(symbol));
            units.extend((0..untracked).map(|_| PayableManaUnit {
                symbol,
                source: None,
                provenance_index: None,
                restricted_index: None,
                from_snow_source: false,
            }));
        }

        units
    }

    pub(crate) fn expanded_payment_pips(
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_black_life: bool,
    ) -> Vec<Vec<crate::mana::ManaSymbol>> {
        use crate::mana::ManaSymbol;

        let mut pips = Vec::new();
        for pip in cost.pips() {
            if pip.len() == 1 {
                match pip[0] {
                    ManaSymbol::Generic(amount) => {
                        pips.extend((0..amount).map(|_| vec![ManaSymbol::Generic(1)]));
                        continue;
                    }
                    ManaSymbol::X => {
                        pips.extend((0..x_value).map(|_| vec![ManaSymbol::Generic(1)]));
                        continue;
                    }
                    ManaSymbol::Black if allow_black_life => {
                        pips.push(vec![ManaSymbol::Black, ManaSymbol::Life(2)]);
                        continue;
                    }
                    _ => {}
                }
            }
            pips.push(pip.clone());
        }
        pips.sort_by_key(|pip| {
            if pip.iter().any(|symbol| matches!(symbol, ManaSymbol::Snow)) {
                0u8
            } else if pip.iter().any(|symbol| {
                matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                )
            }) {
                1
            } else if pip
                .iter()
                .any(|symbol| matches!(symbol, ManaSymbol::Generic(_)))
            {
                2
            } else {
                3
            }
        });
        pips
    }

    fn mana_unit_can_pay(
        &self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        base_policy: &crate::player::ManaSpendPolicy,
        unit: &PayableManaUnit,
        required: crate::mana::ManaSymbol,
    ) -> bool {
        use crate::mana::ManaSymbol;

        match required {
            ManaSymbol::Snow => unit.from_snow_source,
            ManaSymbol::Generic(_) => true,
            ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless => {
                let mut policy = base_policy.clone();
                if unit.source.is_some_and(|mana_source| {
                    self.can_spend_mana_as_any_color_from_mana_source(
                        payer,
                        payment_source,
                        mana_source,
                    )
                }) {
                    policy.allow_mode(ironsmith_core::value_model::ManaSpendMode::AnyColor);
                }
                policy.can_pay_symbol(unit.symbol, required)
            }
            ManaSymbol::Life(_) | ManaSymbol::X => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search_mana_payment_plan(
        &self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        pips: &[Vec<crate::mana::ManaSymbol>],
        pip_index: usize,
        units: &[PayableManaUnit],
        used: &mut [bool],
        selected: &mut Vec<ManaPipCommit>,
        life_to_pay: u32,
        base_policy: &crate::player::ManaSpendPolicy,
        allow_life_payment: bool,
        prefer_life_payment: bool,
    ) -> Option<ManaPaymentPlan> {
        use crate::mana::ManaSymbol;

        if pip_index == pips.len() {
            return self
                .can_pay_life_with_reason(payer, life_to_pay, reason)
                .then(|| ManaPaymentPlan {
                    pip_payments: selected.clone(),
                    life_to_pay,
                });
        }

        let mut alternatives = pips[pip_index].clone();
        alternatives.sort_by_key(|alternative| match alternative {
            ManaSymbol::Life(_) => u8::from(!prefer_life_payment),
            _ => u8::from(prefer_life_payment),
        });
        for alternative in alternatives {
            if let ManaSymbol::Life(amount) = alternative {
                if !allow_life_payment {
                    continue;
                }
                selected.push(ManaPipCommit::Life(amount as u32));
                if let Some(plan) = self.search_mana_payment_plan(
                    payer,
                    payment_source,
                    reason,
                    pips,
                    pip_index + 1,
                    units,
                    used,
                    selected,
                    life_to_pay.saturating_add(amount as u32),
                    base_policy,
                    allow_life_payment,
                    prefer_life_payment,
                ) {
                    return Some(plan);
                }
                selected.pop();
                continue;
            }

            // Preserve colored mana for later colored pips when paying a
            // generic pip in an earlier, separately staged cost. A payment
            // plan for a combined cost already visits colored pips first;
            // this ordering covers effects such as "pay {2}" followed by a
            // spell whose printed cost still needs {U}{U}.
            let mut unit_indices = units
                .iter()
                .enumerate()
                .filter(|(_, unit)| unit.symbol == ManaSymbol::Colorless)
                .collect::<Vec<_>>();
            unit_indices.extend(
                units
                    .iter()
                    .enumerate()
                    .filter(|(_, unit)| unit.symbol != ManaSymbol::Colorless),
            );
            for (unit_index, unit) in unit_indices {
                if used[unit_index]
                    || !self.mana_unit_can_pay(
                        payer,
                        payment_source,
                        base_policy,
                        unit,
                        alternative,
                    )
                {
                    continue;
                }
                used[unit_index] = true;
                selected.push(ManaPipCommit::ManaUnit(unit_index));
                if let Some(plan) = self.search_mana_payment_plan(
                    payer,
                    payment_source,
                    reason,
                    pips,
                    pip_index + 1,
                    units,
                    used,
                    selected,
                    life_to_pay,
                    base_policy,
                    allow_life_payment,
                    prefer_life_payment,
                ) {
                    return Some(plan);
                }
                selected.pop();
                used[unit_index] = false;
            }
        }
        None
    }

    fn mana_payment_plan(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy_override: Option<&crate::player::ManaSpendPolicy>,
        life_options: Option<(bool, bool, bool)>,
    ) -> Option<(Vec<PayableManaUnit>, ManaPaymentPlan)> {
        let default_policy = self.mana_spend_policy(payer, source);
        let policy = policy_override.unwrap_or(&default_policy);
        let engine_allows_black_life = crate::decision::mana_cost_has_black_symbol(cost)
            && self.player_can_pay_black_with_life_for_reason(payer, source, reason);
        let (allow_life_payment, prefer_life_payment, requested_black_life) =
            life_options.unwrap_or((true, false, engine_allows_black_life));
        let allow_black_life = requested_black_life && engine_allows_black_life;
        let required_symbol = source
            .and_then(|source| self.chosen_color_activation_mana_restriction(source, cost, reason));
        let units = self.payable_mana_units(payer, source, reason, cost, required_symbol);
        let pips = Self::expanded_payment_pips(cost, x_value, allow_black_life);
        let minimum_mana_units = pips
            .iter()
            .filter(|pip| {
                !pip.iter()
                    .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
            })
            .count();
        if units.len() < minimum_mana_units {
            return None;
        }
        let mut used = vec![false; units.len()];
        let mut selected = Vec::new();
        let plan = self.search_mana_payment_plan(
            payer,
            source,
            reason,
            &pips,
            0,
            &units,
            &mut used,
            &mut selected,
            0,
            policy,
            allow_life_payment,
            prefer_life_payment,
        )?;
        Some((units, plan))
    }

    /// Check if a player can pay a mana cost for a specific reason.
    pub fn can_pay_mana_cost_with_reason(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        self.mana_payment_plan(payer, source, cost, x_value, reason, None, None)
            .is_some()
    }

    /// Check a payment using a transaction-local spend policy.
    pub fn can_pay_mana_cost_with_policy(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy: &crate::player::ManaSpendPolicy,
    ) -> bool {
        self.mana_payment_plan(payer, source, cost, x_value, reason, Some(policy), None)
            .is_some()
    }

    /// Check a transaction-local plan with explicit life-payment choices.
    #[allow(clippy::too_many_arguments)]
    pub fn can_pay_mana_cost_with_payment_options(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy: &crate::player::ManaSpendPolicy,
        allow_life_payment: bool,
        allow_black_life: bool,
        prefer_life_payment: bool,
    ) -> bool {
        self.mana_payment_plan(
            payer,
            source,
            cost,
            x_value,
            reason,
            Some(policy),
            Some((allow_life_payment, prefer_life_payment, allow_black_life)),
        )
        .is_some()
    }

    /// Preview the exact expanded-pip choices selected by the bulk payer.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn preview_mana_cost_payment_with_options(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy: &crate::player::ManaSpendPolicy,
        allow_life_payment: bool,
        allow_black_life: bool,
        prefer_life_payment: bool,
    ) -> Option<(
        Vec<(
            Vec<crate::mana::ManaSymbol>,
            crate::mana_payment::PlannedPipPayment,
        )>,
        u32,
    )> {
        let actual_black_life = allow_black_life
            && crate::decision::mana_cost_has_black_symbol(cost)
            && self.player_can_pay_black_with_life_for_reason(payer, source, reason);
        let pips = Self::expanded_payment_pips(cost, x_value, actual_black_life);
        let (units, plan) = self.mana_payment_plan(
            payer,
            source,
            cost,
            x_value,
            reason,
            Some(policy),
            Some((allow_life_payment, prefer_life_payment, allow_black_life)),
        )?;
        if pips.len() != plan.pip_payments.len() {
            return None;
        }
        let allocations = pips
            .into_iter()
            .zip(plan.pip_payments.iter())
            .map(|(alternatives, payment)| {
                let payment = match payment {
                    ManaPipCommit::ManaUnit(index) => {
                        crate::mana_payment::PlannedPipPayment::Mana(units.get(*index)?.symbol)
                    }
                    ManaPipCommit::Life(amount) => {
                        crate::mana_payment::PlannedPipPayment::Life(*amount)
                    }
                };
                Some((alternatives, payment))
            })
            .collect::<Option<Vec<_>>>()?;
        Some((allocations, plan.life_to_pay))
    }

    /// Attempt to pay a mana cost, accounting for "spend as though any color".
    pub fn try_pay_mana_cost(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        self.try_pay_mana_cost_with_reason(
            payer,
            source,
            cost,
            x_value,
            crate::costs::PaymentReason::Other,
        )
    }

    /// Attempt to pay a mana cost for a specific reason.
    pub fn try_pay_mana_cost_with_reason(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        let policy = self.mana_spend_policy(payer, source);
        self.try_pay_mana_cost_with_policy(payer, source, cost, x_value, reason, &policy)
    }

    /// Commit a payment using a transaction-local spend policy.
    pub fn try_pay_mana_cost_with_policy(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy: &crate::player::ManaSpendPolicy,
    ) -> bool {
        self.try_pay_mana_cost_with_payment_options(
            payer, source, cost, x_value, reason, policy, true, true, false,
        )
    }

    /// Commit a transaction-local payment with explicit life-payment choices.
    #[allow(clippy::too_many_arguments)]
    pub fn try_pay_mana_cost_with_payment_options(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
        policy: &crate::player::ManaSpendPolicy,
        allow_life_payment: bool,
        allow_black_life: bool,
        prefer_life_payment: bool,
    ) -> bool {
        let Some((units, plan)) = self.mana_payment_plan(
            payer,
            source,
            cost,
            x_value,
            reason,
            Some(policy),
            Some((allow_life_payment, prefer_life_payment, allow_black_life)),
        ) else {
            return false;
        };
        let Some(player) = self.player(payer) else {
            return false;
        };
        let original_pool = player.mana_pool.clone();
        let original_restricted = player.restricted_mana.clone();
        let original_provenance = player.mana_source_provenance.clone();

        let selected = plan
            .pip_payments
            .iter()
            .filter_map(|payment| match payment {
                ManaPipCommit::ManaUnit(index) => units.get(*index),
                ManaPipCommit::Life(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>();
        let selected_count = plan
            .pip_payments
            .iter()
            .filter(|payment| matches!(payment, ManaPipCommit::ManaUnit(_)))
            .count();
        if selected.len() != selected_count {
            return false;
        }
        let spent_units = selected
            .iter()
            .map(|unit| SpentManaUnitCommit {
                symbol: unit.symbol,
                restriction: unit
                    .restricted_index
                    .and_then(|index| original_restricted.get(index))
                    .cloned(),
                provenance: unit
                    .provenance_index
                    .and_then(|index| original_provenance.get(index))
                    .cloned(),
            })
            .collect::<Vec<_>>();
        let mut restricted_indices = selected
            .iter()
            .filter_map(|unit| unit.restricted_index)
            .collect::<Vec<_>>();
        let mut provenance_indices = selected
            .iter()
            .filter_map(|unit| unit.provenance_index)
            .collect::<Vec<_>>();
        restricted_indices.sort_unstable();
        restricted_indices.dedup();
        provenance_indices.sort_unstable();
        provenance_indices.dedup();

        let committed = if let Some(player) = self.player_mut(payer) {
            let mut removed_all = true;
            for unit in &selected {
                removed_all &= player.mana_pool.remove(unit.symbol, 1);
            }
            if removed_all {
                for index in restricted_indices.into_iter().rev() {
                    player.restricted_mana.remove(index);
                }
                for index in provenance_indices.into_iter().rev() {
                    player.mana_source_provenance.remove(index);
                }
                player.trim_mana_source_provenance_to_pool();
            }
            removed_all
        } else {
            false
        };
        if !committed || (plan.life_to_pay > 0 && !self.pay_life(payer, plan.life_to_pay)) {
            if let Some(player) = self.player_mut(payer) {
                player.mana_pool = original_pool;
                player.restricted_mana = original_restricted;
                player.mana_source_provenance = original_provenance;
            }
            return false;
        }

        self.publish_spent_mana_units(payer, source, reason, spent_units);
        self.record_bulk_mana_sources_spent_to_cast(payer, source, reason, &original_provenance);
        true
    }

    fn publish_spent_mana_units(
        &mut self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        spent_units: Vec<SpentManaUnitCommit>,
    ) {
        for spent in spent_units {
            let mana_source = spent
                .restriction
                .as_ref()
                .map(|unit| unit.source)
                .or_else(|| spent.provenance.as_ref().map(|unit| unit.source))
                .unwrap_or_else(|| ObjectId::from_raw(0));
            let source_snapshot = spent
                .provenance
                .as_ref()
                .and_then(|unit| unit.snapshot.clone())
                .or_else(|| {
                    self.object(mana_source)
                        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, self))
                });
            self.publish_spent_mana_unit(
                payer,
                payment_source,
                reason,
                spent.symbol,
                mana_source,
                source_snapshot,
                spent.restriction,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_spent_mana_unit(
        &mut self,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        symbol: crate::mana::ManaSymbol,
        mana_source: ObjectId,
        source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        restriction: Option<crate::ability::RestrictedManaUnit>,
    ) {
        let payment_snapshot = payment_source
            .and_then(|source| self.object(source))
            .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, self));
        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::ManaSpent);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ManaUnitSpentEvent {
                player: payer,
                mana_source,
                payment_source,
                symbol,
                purpose: reason.mana_payment_purpose(),
                source_snapshot: source_snapshot.clone(),
            },
            event_provenance,
        );
        self.queue_trigger_event(event_provenance, event.clone());

        let Some(restriction) = restriction else {
            return;
        };
        for payload in restriction
            .restrictions
            .iter()
            .filter_map(|restriction| match restriction {
                crate::ability::ManaUsageRestriction::PaymentTransaction { on_spend, .. } => {
                    Some(on_spend.as_slice())
                }
                _ => None,
            })
            .flatten()
        {
            if !self.mana_payment_predicate_matches(
                &restriction,
                &payload.predicate,
                payment_source,
                reason,
                None,
            ) {
                continue;
            }
            let ability = crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::custom(
                    "mana_unit_spent_payload",
                    "When you spend this mana".to_string(),
                ),
                effects: payload.effects.clone(),
                choices: payload.choices.clone(),
                intervening_if: None,
                presentation_label: None,
            };
            let trigger_identity = crate::triggers::compute_trigger_identity(&ability);
            let mut tagged_objects = std::collections::HashMap::new();
            if let Some(snapshot) = payment_snapshot.clone() {
                tagged_objects.insert(
                    crate::tag::TagKey::from(ironsmith_core::MANA_PAID_OBJECT_TAG),
                    vec![snapshot],
                );
            }
            self.defer_trigger_entries([crate::triggers::TriggeredAbilityEntry {
                source: mana_source,
                controller: source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.controller)
                    .unwrap_or(payer),
                x_value: payment_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.x_value),
                event_value_amount: None,
                ability,
                triggering_event: event.clone(),
                source_stable_id: source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.stable_id)
                    .unwrap_or_else(|| crate::ids::StableId::from(mana_source)),
                source_name: source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.name.clone())
                    .unwrap_or_else(|| "mana ability".to_string()),
                source_snapshot: source_snapshot.clone(),
                tagged_objects,
                source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
                trigger_identity,
            }]);
        }
    }

    fn record_bulk_mana_sources_spent_to_cast(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
        before: &[crate::player::ManaSourceProvenance],
    ) {
        if reason != crate::costs::PaymentReason::CastSpell || before.is_empty() {
            return;
        }
        let Some(spell_id) = source else {
            return;
        };
        let mut remaining = self
            .player(payer)
            .map(|player| player.mana_source_provenance.clone())
            .unwrap_or_default();
        let spent = before
            .iter()
            .filter_map(|unit| {
                if let Some(index) = remaining.iter().position(|candidate| candidate == unit) {
                    remaining.remove(index);
                    None
                } else {
                    unit.snapshot.clone()
                }
            })
            .collect::<Vec<_>>();
        if spent.is_empty() {
            return;
        }
        let Some(spell) = self.object_mut(spell_id) else {
            return;
        };
        spell
            .cast_tagged_objects
            .entry(crate::tag::TagKey::from(
                ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG,
            ))
            .or_default()
            .extend(spent);
    }

    fn chosen_color_activation_mana_restriction(
        &self,
        source: ObjectId,
        cost: &crate::mana::ManaCost,
        reason: crate::costs::PaymentReason,
    ) -> Option<crate::mana::ManaSymbol> {
        if reason != crate::costs::PaymentReason::ActivateAbility {
            return None;
        }

        let object = self.object(source)?;
        let has_restricted_activation = object.abilities.iter().any(|ability| {
            let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            activated.mana_cost.costs().iter().any(|component| {
                component
                    .mana_cost_ref()
                    .is_some_and(|activation_cost| activation_cost == cost)
            }) && activated.additional_restrictions.iter().any(|restriction| {
                restriction.eq_ignore_ascii_case(
                    "spend only mana of the chosen color to activate this ability",
                )
            })
        });

        has_restricted_activation.then(|| {
            self.chosen_color(source)
                .map(crate::mana::ManaSymbol::from_color)
        })?
    }

    /// Gets a reference to a player by ID.
    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.player_last_known_information(id)
    }

    /// Gets a mutable reference to a player by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.mark_continuous_state_dirty();
        self.players.get_mut(id.index())
    }

    pub fn player_speed(&self, id: PlayerId) -> Option<u8> {
        self.player(id).and_then(|player| player.speed)
    }

    pub fn has_max_speed(&self, id: PlayerId) -> bool {
        self.player_speed(id).is_some_and(|speed| speed >= 4)
    }

    pub fn start_engines(&mut self, id: PlayerId) -> bool {
        self.player_mut(id)
            .is_some_and(|player| player.start_engines())
    }

    pub fn increase_speed(&mut self, id: PlayerId, amount: u32) -> u32 {
        self.player_mut(id)
            .map(|player| player.increase_speed(amount))
            .unwrap_or(0)
    }

    pub fn reduce_speed(&mut self, id: PlayerId, amount: u32, minimum: u8) -> u32 {
        self.player_mut(id)
            .map(|player| player.reduce_speed(amount, minimum))
            .unwrap_or(0)
    }

    pub fn speed_increase_triggered_this_turn(&self, id: PlayerId) -> bool {
        self.combat_transients
            .speed_increase_triggered_this_turn
            .contains(&id)
    }

    pub fn mark_speed_increase_triggered_this_turn(&mut self, id: PlayerId) {
        self.combat_transients_mut()
            .speed_increase_triggered_this_turn
            .insert(id);
    }

    /// Designate an object as a commander for a player.
    ///
    /// This sets the commander status on the game state and adds it to the player's commander list.
    pub fn set_as_commander(&mut self, object_id: ObjectId, owner: PlayerId) {
        // Set the commander flag in the extension map
        self.set_commander(object_id);
        // Add to the player's commander list
        if let Some(player) = self.player_mut(owner) {
            player.add_commander(object_id);
        }
    }

    /// Enable or disable the CR 704.6c commander-damage state-based action.
    pub fn set_commander_damage_loss_enabled(&mut self, enabled: bool) {
        self.commander_damage_loss_enabled = enabled;
    }

    pub fn commander_damage_loss_enabled(&self) -> bool {
        self.commander_damage_loss_enabled
    }

    /// Resolve a commander's stable identity from either its original or current object ID.
    pub fn commander_identity(&self, obj_id: ObjectId) -> Option<ObjectId> {
        if self
            .players
            .iter()
            .any(|player| player.commanders.contains(&obj_id))
        {
            return Some(obj_id);
        }

        let obj = self.object(obj_id)?;
        let stable_identity = obj.stable_id.object_id();
        if self
            .players
            .iter()
            .any(|player| player.commanders.contains(&stable_identity))
        {
            return Some(stable_identity);
        }

        // CR 903.3c: a commander that is a component of a merged permanent
        // keeps its commander designation even when it is not the top
        // component and therefore does not supply the permanent's stable id.
        self.merged_permanent(obj.stable_id).and_then(|merged| {
            merged.components.iter().find_map(|component| {
                let identity = component.object.stable_id.object_id();
                (component.is_commander
                    || self
                        .players
                        .iter()
                        .any(|player| player.commanders.contains(&identity)))
                .then_some(identity)
            })
        })
    }

    /// Resolve the current object ID for a stored commander identity.
    pub fn current_commander_object(&self, commander_id: ObjectId) -> Option<ObjectId> {
        if self.object(commander_id).is_some() {
            return Some(commander_id);
        }

        self.find_object_by_stable_id(StableId::from(commander_id))
            .or_else(|| {
                self.battlefield.iter().copied().find(|permanent_id| {
                    let Some(permanent) = self.object(*permanent_id) else {
                        return false;
                    };
                    self.merged_permanent(permanent.stable_id)
                        .is_some_and(|merged| {
                            merged.components.iter().any(|component| {
                                component.object.stable_id.object_id() == commander_id
                            })
                        })
                })
            })
    }

    /// Resolve the destination for a commander moving to hand or library.
    ///
    /// For all other zone changes, this returns `requested_zone` unchanged.
    pub fn resolve_commander_move_destination(
        &self,
        object_id: ObjectId,
        requested_zone: Zone,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) -> Zone {
        let destination_text = match requested_zone {
            Zone::Hand => "putting it into its owner's hand",
            Zone::Library => "putting it into its owner's library",
            _ => return requested_zone,
        };

        // A merged permanent moves as one object, then its components become
        // separate new objects. CR 730.3d and 903.9b let only the commander
        // component use the hand/library command-zone replacement.
        if self
            .object(object_id)
            .is_some_and(|object| self.merged_permanent(object.stable_id).is_some())
        {
            return requested_zone;
        }

        if !self.is_commander(object_id) {
            return requested_zone;
        }

        let Some(obj) = self.object(object_id) else {
            return requested_zone;
        };
        let owner = obj.owner;
        let name = obj.name.to_string();
        let choice_ctx = crate::decisions::context::BooleanContext::new(
            owner,
            Some(object_id),
            format!("move it to the command zone instead of {destination_text}"),
        )
        .with_source_name(name);

        if decision_maker.decide_boolean(self, &choice_ctx) {
            Zone::Command
        } else {
            requested_zone
        }
    }

    /// Resolve hand/library commander replacement choices independently for
    /// the physical components of a merged permanent.
    pub(crate) fn prepare_merged_component_destinations(
        &mut self,
        object_id: ObjectId,
        requested_zone: Zone,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) {
        if !matches!(requested_zone, Zone::Hand | Zone::Library) {
            return;
        }
        let Some(stable_id) = self.object(object_id).map(|object| object.stable_id) else {
            return;
        };
        let Some(merged) = self.merged_permanent(stable_id).cloned() else {
            return;
        };

        let mut destinations = Vec::with_capacity(merged.components.len());
        for component in &merged.components {
            let commander_identity = component.object.stable_id.object_id();
            let is_commander = component.is_commander
                || self.players.iter().any(|player| {
                    player
                        .commanders
                        .iter()
                        .any(|identity| *identity == commander_identity)
                });
            if !is_commander {
                destinations.push(requested_zone);
                continue;
            }
            let destination_text = match requested_zone {
                Zone::Hand => "putting it into its owner's hand",
                Zone::Library => "putting it into its owner's library",
                _ => unreachable!(),
            };
            let choice = crate::decisions::context::BooleanContext::new(
                component.object.owner,
                Some(object_id),
                format!("move it to the command zone instead of {destination_text}"),
            )
            .with_source_name(component.object.name.to_string());
            destinations.push(if decision_maker.decide_boolean(self, &choice) {
                Zone::Command
            } else {
                requested_zone
            });
        }
        self.commander_tracking_mut()
            .pending_merged_component_destinations
            .insert(stable_id, destinations);
    }

    /// Move an object while applying commander hand/library replacement choices.
    pub fn move_object_with_commander_options(
        &mut self,
        object_id: ObjectId,
        requested_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) -> Option<(ObjectId, Zone)> {
        let final_zone =
            self.resolve_commander_move_destination(object_id, requested_zone, decision_maker);
        self.prepare_merged_component_destinations(object_id, final_zone, decision_maker);
        self.move_object(object_id, final_zone, cause)
            .map(|new_id| (new_id, final_zone))
    }

    /// Returns how many times a commander has been cast from the command zone.
    pub fn commander_cast_count(&self, commander_id: ObjectId) -> u32 {
        let identity = self
            .commander_identity(commander_id)
            .unwrap_or(commander_id);
        self.commander_tracking
            .commander_casts_from_command_zone
            .get(&identity)
            .copied()
            .unwrap_or(0)
    }

    /// Returns how many times all of a player's commanders have been cast from the command zone.
    pub fn commander_cast_count_for_player(&self, player_id: PlayerId) -> u32 {
        let Some(player) = self.player(player_id) else {
            return 0;
        };

        player
            .get_commanders()
            .iter()
            .copied()
            .map(|commander_id| self.commander_cast_count(commander_id))
            .sum()
    }

    /// Records that a commander was cast from the command zone.
    pub fn record_commander_cast_from_command_zone(&mut self, commander_id: ObjectId) {
        if let Some(identity) = self.commander_identity(commander_id) {
            *self
                .commander_tracking_mut()
                .commander_casts_from_command_zone
                .entry(identity)
                .or_insert(0) += 1;
            // Commander-cast counts are read by dynamic continuous effects
            // (for example Commander's Insignia).  They do not change an
            // object's characteristics directly, so invalidate the cached
            // static effects explicitly when the count changes.
            self.mark_continuous_state_dirty();
        }
    }

    /// Records combat damage dealt to a player by a commander.
    pub fn record_commander_damage(
        &mut self,
        player_id: PlayerId,
        commander_id: ObjectId,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        let Some(identity) = self.commander_identity(commander_id) else {
            return;
        };
        if let Some(player) = self.player_mut(player_id) {
            player.record_commander_damage(identity, amount);
        }
    }

    /// Returns true if this exact commander object already declined moving to command zone.
    pub fn commander_command_zone_move_declined(&self, object_id: ObjectId) -> bool {
        self.commander_tracking
            .declined_command_zone_moves
            .contains(&object_id)
    }

    /// Mark this commander object as having declined the current command-zone move.
    pub fn decline_commander_command_zone_move(&mut self, object_id: ObjectId) {
        self.commander_tracking_mut()
            .declined_command_zone_moves
            .insert(object_id);
    }

    /// Set the current monarch designation holder.
    ///
    /// Use `None` to clear the designation.
    pub fn set_monarch(&mut self, monarch: Option<PlayerId>) {
        let changed = monarch != self.monarch;
        if changed {
            self.mark_continuous_state_dirty();
        }
        if monarch.is_some() && changed {
            self.record_ui_effect_event("monarch", monarch, None, Vec::new(), None, None);
        }
        self.monarch = monarch;
        if changed && let Some(monarch) = monarch {
            self.return_exiled_for_opponent_becoming_monarch(monarch);
        }
    }

    /// Set the current initiative designation holder.
    ///
    /// Use `None` to clear the designation.
    pub fn set_initiative(&mut self, initiative: Option<PlayerId>) {
        if initiative.is_some() && initiative != self.initiative {
            self.record_ui_effect_event("initiative", initiative, None, Vec::new(), None, None);
        }
        self.initiative = initiative;
    }

    /// Reconcile any Ring-bearers that are no longer valid.
    pub fn reconcile_ring_bearers(&mut self) {
        let player_ids = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        for player in player_ids {
            self.reconcile_ring_bearer(player);
        }
    }

    /// Reconcile one player's Ring-bearer state against the live battlefield.
    pub fn reconcile_ring_bearer(&mut self, player: PlayerId) {
        if self.current_ring_bearer(player).is_some() {
            return;
        }
        self.clear_ring_bearer(player);
    }

    /// Returns how many times the Ring has tempted this player this game.
    pub fn ring_temptations(&self, player: PlayerId) -> u32 {
        self.player(player)
            .map(|player| player.ring_temptations)
            .unwrap_or(0)
    }

    /// Returns the unlocked Ring tier for this player, capped at four.
    pub fn ring_level(&self, player: PlayerId) -> u32 {
        self.ring_temptations(player).min(4)
    }

    /// Returns the player's current Ring-bearer if it is still valid.
    pub fn current_ring_bearer(&self, player: PlayerId) -> Option<ObjectId> {
        let bearer = self.player(player)?.ring_bearer?;
        if !self.battlefield.contains(&bearer) {
            return None;
        }
        if self.current_controller(bearer) != Some(player) {
            return None;
        }
        if !self.current_is_creature(bearer) {
            return None;
        }
        Some(bearer)
    }

    /// Increments the number of times the Ring has tempted the player.
    pub fn increment_ring_temptations(&mut self, player: PlayerId) {
        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_temptations = player_state.ring_temptations.saturating_add(1);
        }
    }

    /// Clear the player's current Ring-bearer designation.
    pub fn clear_ring_bearer(&mut self, player: PlayerId) {
        let previous_legendary_added = self
            .player(player)
            .and_then(|player_state| player_state.ring_legendary_added);
        if let Some(object_id) = previous_legendary_added
            && let Some(object) = self.object_mut(object_id)
        {
            object
                .supertypes
                .retain(|supertype| *supertype != crate::types::Supertype::Legendary);
        }

        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_bearer = None;
            player_state.ring_legendary_added = None;
        }
    }

    /// Set the player's Ring-bearer designation to the given creature.
    pub fn set_ring_bearer(&mut self, player: PlayerId, bearer: ObjectId) {
        self.clear_ring_bearer(player);

        let mut legendary_added = None;
        if let Some(object) = self.object_mut(bearer)
            && !object.has_supertype(crate::types::Supertype::Legendary)
        {
            object.supertypes.push(crate::types::Supertype::Legendary);
            legendary_added = Some(bearer);
        }

        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_bearer = Some(bearer);
            player_state.ring_legendary_added = legendary_added;
        }
    }

    /// Returns true if the given player is currently the monarch.
    pub fn is_monarch(&self, player: PlayerId) -> bool {
        self.monarch == Some(player)
    }

    /// Returns true if the given player currently has the initiative.
    pub fn has_initiative(&self, player: PlayerId) -> bool {
        self.initiative == Some(player)
    }

    /// Returns the player's active dungeon progress, if any.
    pub fn active_dungeon(&self, player: PlayerId) -> Option<&ActiveDungeonProgress> {
        self.auxiliary_tracking.active_dungeons.get(&player)
    }

    /// Set the player's active dungeon progress.
    pub fn set_active_dungeon(&mut self, player: PlayerId, progress: ActiveDungeonProgress) {
        self.auxiliary_tracking_mut()
            .active_dungeons
            .insert(player, progress);
    }

    /// Clear the player's active dungeon progress.
    pub fn clear_active_dungeon(&mut self, player: PlayerId) {
        self.auxiliary_tracking_mut()
            .active_dungeons
            .remove(&player);
    }

    /// Record that the player completed the named dungeon.
    pub fn record_completed_dungeon(&mut self, player: PlayerId, dungeon_name: impl Into<String>) {
        self.auxiliary_tracking_mut()
            .completed_dungeons
            .entry(player)
            .or_default()
            .push(dungeon_name.into());
    }

    /// Returns the names of dungeons the player has completed this game.
    pub fn completed_dungeons(&self, player: PlayerId) -> &[String] {
        self.auxiliary_tracking
            .completed_dungeons
            .get(&player)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Returns true if the player has completed one or more dungeons this game.
    pub fn has_completed_dungeon(&self, player: PlayerId) -> bool {
        !self.completed_dungeons(player).is_empty()
    }

    /// Returns true if the player has completed the named dungeon this game.
    pub fn has_completed_named_dungeon(&self, player: PlayerId, dungeon_name: &str) -> bool {
        self.completed_dungeons(player)
            .iter()
            .any(|completed| completed.eq_ignore_ascii_case(dungeon_name))
    }

    /// Returns the count of differently named dungeons the player has completed this game.
    pub fn completed_different_dungeon_names_count(&self, player: PlayerId) -> usize {
        let mut seen = HashSet::new();
        for completed in self.completed_dungeons(player) {
            seen.insert(completed.to_ascii_lowercase());
        }
        seen.len()
    }

    /// Returns true if the given player has the city's blessing designation.
    pub fn has_citys_blessing(&self, player: PlayerId) -> bool {
        self.citys_blessing.contains(&player)
    }

    /// Permanently grant a player the city's blessing designation.
    pub fn grant_citys_blessing(&mut self, player: PlayerId) -> bool {
        let granted = self.citys_blessing.insert(player);
        if granted {
            self.mark_continuous_state_dirty();
        }
        granted
    }

    /// Returns all object IDs in a given zone.
    pub fn objects_in_zone(&self, zone: Zone) -> Vec<ObjectId> {
        self.zone_ids(zone).collect()
    }

    pub fn zone_ids(&self, zone: Zone) -> Box<dyn Iterator<Item = ObjectId> + '_> {
        match zone {
            Zone::Battlefield => Box::new(self.battlefield.iter().copied()),
            Zone::Graveyard => Box::new(
                self.players
                    .iter()
                    .flat_map(|player| player.graveyard.iter().copied()),
            ),
            Zone::Hand => Box::new(
                self.players
                    .iter()
                    .flat_map(|player| player.hand.iter().copied()),
            ),
            Zone::Library => Box::new(
                self.players
                    .iter()
                    .flat_map(|player| player.library.iter().copied()),
            ),
            Zone::OutsideGame => Box::new(
                self.players
                    .iter()
                    .flat_map(|player| player.sideboard.iter().copied()),
            ),
            Zone::Stack => Box::new(self.stack.iter().map(|entry| entry.object_id)),
            Zone::Exile => Box::new(self.exile.iter().copied()),
            Zone::Command => Box::new(self.command_zone.iter().copied()),
            Zone::Ante => Box::new(self.ante.iter().copied()),
        }
    }

    /// Returns all object IDs in deterministic order.
    pub fn object_ids_in_deterministic_order(&self) -> Vec<ObjectId> {
        let mut ids: Vec<_> = self.objects.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Returns all objects in deterministic order by object ID.
    pub fn objects_in_deterministic_order(&self) -> Vec<&Object> {
        self.object_ids_in_deterministic_order()
            .into_iter()
            .filter_map(|id| self.objects.get(&id).map(Arc::as_ref))
            .collect()
    }

    pub(crate) fn cached_object_snapshot_with_calculated_characteristics_and_effects(
        &self,
        object: &Object,
        effects: &[ContinuousEffect],
    ) -> ObjectSnapshot {
        let mutation_revision = self.mutation_revision;
        let effect_revision = self.effect_store.continuous_effects.revision();
        {
            let mut cache = self.runtime_cache.object_snapshot_cache.borrow_mut();
            if cache.mutation_revision != mutation_revision
                || cache.effect_revision != effect_revision
            {
                cache.entries.clear();
                cache.mutation_revision = mutation_revision;
                cache.effect_revision = effect_revision;
            }
            if let Some(snapshot) = cache.entries.get(&object.id) {
                return snapshot.as_ref().clone();
            }
        }

        let snapshot = Arc::new(
            ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                object, self, effects,
            ),
        );
        let mut cache = self.runtime_cache.object_snapshot_cache.borrow_mut();
        if cache.mutation_revision == mutation_revision && cache.effect_revision == effect_revision
        {
            cache.entries.insert(object.id, Arc::clone(&snapshot));
        }
        snapshot.as_ref().clone()
    }

    pub(crate) fn cached_object_snapshot_with_calculated_characteristics(
        &self,
        object: &Object,
    ) -> ObjectSnapshot {
        let all_effects = self.all_continuous_effects();
        self.cached_object_snapshot_with_calculated_characteristics_and_effects(
            object,
            &all_effects,
        )
    }

    pub(crate) fn trigger_source_lookback_snapshots(&self) -> Vec<ObjectSnapshot> {
        let all_effects = self.all_continuous_effects();
        let ability_effects_can_add_triggers = all_effects
            .iter()
            .any(|effect| Self::modification_can_change_triggered_abilities(&effect.modification));
        self.objects_in_deterministic_order()
            .into_iter()
            .filter(|object| {
                ability_effects_can_add_triggers
                    || object.abilities.iter().any(|ability| {
                        matches!(ability.kind, AbilityKind::Triggered(_))
                            && ability.functions_in(&object.zone)
                    })
            })
            .map(|object| {
                self.cached_object_snapshot_with_calculated_characteristics_and_effects(
                    object,
                    &all_effects,
                )
            })
            .filter(|snapshot| {
                snapshot.abilities.iter().any(|ability| {
                    matches!(ability.kind, AbilityKind::Triggered(_))
                        && ability.functions_in(&snapshot.zone)
                })
            })
            .collect()
    }

    fn modification_can_change_triggered_abilities(modification: &Modification) -> bool {
        // Exhaustive on purpose: a new Modification variant must decide this
        // explicitly. Answering `false` for a variant that can alter the
        // triggered-ability list silently drops LKI trigger snapshots.
        match modification {
            // Rewrites the whole ability list or text box, so triggered
            // abilities can appear or disappear. SetAbilities replaces the
            // existing list even when it only sets static abilities.
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_)
            | Modification::SetAbilities(_)
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana => true,
            Modification::AddAbilityGeneric(ability)
            | Modification::RemoveAbilityGeneric { ability, .. } => {
                matches!(ability.kind, AbilityKind::Triggered(_))
            }
            // Static-only ability edits and pure characteristic changes
            // cannot change the triggered-ability list.
            Modification::AddAbility(_)
            | Modification::RemoveAbility(_)
            | Modification::CopyActivatedAbilities { .. }
            | Modification::CopyStaticAbilityVariants { .. }
            | Modification::ChangeController(_)
            | Modification::SetName(_)
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::CantBeBlocked
            | Modification::CantAttack
            | Modification::CantBlock
            | Modification::DoesntUntap
            | Modification::SetPower { .. }
            | Modification::SetToughness { .. }
            | Modification::SetPowerToughness { .. }
            | Modification::ModifyPower(_)
            | Modification::ModifyToughness(_)
            | Modification::ModifyPowerToughness { .. }
            | Modification::ModifyPowerToughnessValue { .. }
            | Modification::ModifyPowerToughnessByColorCount { .. }
            | Modification::SwitchPowerToughness => false,
        }
    }

    pub(crate) fn may_have_triggered_abilities_for_event_kind(
        &self,
        event_kind: EventKind,
    ) -> bool {
        let all_effects = self.all_continuous_effects();
        if all_effects
            .iter()
            .any(|effect| Self::modification_can_change_triggered_abilities(&effect.modification))
        {
            return true;
        }

        self.objects.values().any(|object| {
            object.abilities.iter().any(|ability| {
                if !matches!(ability.kind, AbilityKind::Triggered(_))
                    || !ability.functions_in(&object.zone)
                {
                    return false;
                }
                let AbilityKind::Triggered(triggered) = &ability.kind else {
                    return false;
                };
                triggered
                    .trigger
                    .subscribed_kinds()
                    .is_none_or(|kinds| kinds.contains(&event_kind))
            })
        })
    }

    /// Returns all permanents controlled by a player.
    pub fn permanents_controlled_by(&self, controller: PlayerId) -> Vec<ObjectId> {
        self.battlefield
            .iter()
            .filter(|&&id| {
                !self.is_phased_out(id)
                    && self
                        .objects
                        .get(&id)
                        .is_some_and(|o| self.controller_of(o) == controller)
            })
            .copied()
            .collect()
    }

    /// Returns all creatures controlled by a player.
    pub fn creatures_controlled_by(&self, controller: PlayerId) -> Vec<ObjectId> {
        self.battlefield
            .iter()
            .filter(|&&id| {
                !self.is_phased_out(id)
                    && self.objects.get(&id).is_some_and(|o| {
                        self.controller_of(o) == controller && self.current_is_creature(id)
                    })
            })
            .copied()
            .collect()
    }

    /// Returns devotion to a color for permanents controlled by `controller`.
    ///
    /// Devotion counts colored mana symbols in mana costs. Hybrid symbols count
    /// if they include the queried color.
    pub fn devotion_to_color(&self, controller: PlayerId, color: crate::color::Color) -> usize {
        self.permanents_controlled_by(controller)
            .into_iter()
            .filter_map(|id| self.object(id))
            .filter_map(|obj| obj.mana_cost.as_ref())
            .map(|mana_cost| {
                mana_cost
                    .pips()
                    .iter()
                    .map(|pip| {
                        usize::from(pip.iter().copied().any(|symbol| {
                            matches!(
                                (symbol, color),
                                (crate::mana::ManaSymbol::White, crate::color::Color::White)
                                    | (crate::mana::ManaSymbol::Blue, crate::color::Color::Blue)
                                    | (crate::mana::ManaSymbol::Black, crate::color::Color::Black)
                                    | (crate::mana::ManaSymbol::Red, crate::color::Color::Red)
                                    | (crate::mana::ManaSymbol::Green, crate::color::Color::Green)
                            )
                        }))
                    })
                    .sum::<usize>()
            })
            .sum()
    }
}
