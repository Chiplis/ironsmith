use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectRole {
    Actor,
    AffectedPlayer,
    Chooser,
    LibraryOwner,
    ZoneOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectBindingMode {
    Actor,
    Chooser,
    AffectedPlayer,
    OwnedZone,
}

#[derive(Debug, Clone)]
pub(crate) struct LoweredSubject {
    role: SubjectRole,
    binding_mode: SubjectBindingMode,
    player_filter: PlayerFilter,
    choices: Vec<ChooseSpec>,
}

impl LoweredSubject {
    fn binding_mode_for_role(role: SubjectRole) -> SubjectBindingMode {
        match role {
            SubjectRole::Actor => SubjectBindingMode::Actor,
            SubjectRole::Chooser => SubjectBindingMode::Chooser,
            SubjectRole::AffectedPlayer => SubjectBindingMode::AffectedPlayer,
            SubjectRole::LibraryOwner | SubjectRole::ZoneOwner => SubjectBindingMode::OwnedZone,
        }
    }

    pub(crate) fn from_resolved(player_filter: PlayerFilter, choices: Vec<ChooseSpec>) -> Self {
        Self {
            role: SubjectRole::Actor,
            binding_mode: SubjectBindingMode::Actor,
            player_filter,
            choices,
        }
    }

    pub(crate) fn resolve(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::Actor,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    fn resolve_role(
        role: SubjectRole,
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        let (player_filter, choices) = resolve_effect_player_filter(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?;
        Ok(Self {
            role,
            binding_mode: Self::binding_mode_for_role(role),
            player_filter,
            choices,
        })
    }

    pub(crate) fn resolve_actor(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::Actor,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    pub(crate) fn resolve_chooser(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::Chooser,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    pub(crate) fn resolve_affected_player(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::AffectedPlayer,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    pub(crate) fn resolve_library_owner(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::LibraryOwner,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    pub(crate) fn resolve_zone_owner(
        player: PlayerAst,
        ctx: &mut EffectLoweringContext,
        allow_target: bool,
        allow_target_opponent: bool,
        track_last_player_filter: bool,
    ) -> Result<Self, CardTextError> {
        Self::resolve_role(
            SubjectRole::ZoneOwner,
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )
    }

    pub(crate) fn as_role(mut self, role: SubjectRole) -> Self {
        self.role = role;
        self.binding_mode = Self::binding_mode_for_role(role);
        self
    }

    pub(crate) fn role(&self) -> SubjectRole {
        self.role
    }

    pub(crate) fn binding_mode(&self) -> SubjectBindingMode {
        self.binding_mode
    }

    pub(crate) fn as_actor(&self) -> PlayerFilter {
        self.player_filter.clone()
    }

    pub(crate) fn as_chooser(&self) -> PlayerFilter {
        debug_assert_eq!(self.role, SubjectRole::Chooser);
        self.player_filter.clone()
    }

    pub(crate) fn as_affected_player(&self) -> PlayerFilter {
        debug_assert_eq!(self.role, SubjectRole::AffectedPlayer);
        self.player_filter.clone()
    }

    pub(crate) fn as_library_owner(&self) -> PlayerFilter {
        debug_assert_eq!(self.role, SubjectRole::LibraryOwner);
        self.player_filter.clone()
    }

    pub(crate) fn as_zone_owner(&self) -> PlayerFilter {
        debug_assert_eq!(self.role, SubjectRole::ZoneOwner);
        self.player_filter.clone()
    }

    pub(crate) fn player_filter(&self) -> &PlayerFilter {
        &self.player_filter
    }

    pub(crate) fn clone_player_filter(&self) -> PlayerFilter {
        self.player_filter.clone()
    }

    pub(crate) fn into_player_filter(&self) -> PlayerFilter {
        self.player_filter.clone()
    }

    pub(crate) fn into_parts(self) -> (PlayerFilter, Vec<ChooseSpec>) {
        (self.player_filter, self.choices)
    }

    pub(crate) fn choices(&self) -> &[ChooseSpec] {
        &self.choices
    }

    pub(crate) fn into_choices(&self) -> Vec<ChooseSpec> {
        self.choices.clone()
    }

    pub(crate) fn extend_choices_into(self, choices: &mut Vec<ChooseSpec>) {
        choices.extend(self.choices);
    }

    pub(crate) fn bind_player_refs_in_value(
        &self,
        value: &Value,
        ctx: &mut EffectLoweringContext,
    ) -> Result<Value, CardTextError> {
        let mut value = value.clone();
        self.apply_player_refs_to_value(&mut value, ctx);
        Ok(value)
    }

    pub(crate) fn resolve_object_refs_and_bind_player_refs_in_value(
        &self,
        value: &Value,
        ctx: &mut EffectLoweringContext,
    ) -> Result<Value, CardTextError> {
        let mut value = resolve_value_it_tag(value, &current_reference_env(ctx))?;
        self.apply_player_refs_to_value(&mut value, ctx);
        Ok(value)
    }

    pub(crate) fn bind_player_refs_in_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &EffectLoweringContext,
    ) -> ObjectFilter {
        let mut resolved = filter.clone();
        self.apply_player_refs_to_filter(&mut resolved, ctx);
        resolved
    }

    pub(crate) fn resolve_object_refs_and_bind_player_refs_in_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
        preserve_chooser_relative_player_filters(filter, &mut resolved, &self.player_filter);
        self.apply_player_refs_to_filter(&mut resolved, ctx);
        Ok(resolved)
    }

    pub(crate) fn bind_owned_zone_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
        default_zone: Zone,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = self.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
        if resolved.zone.is_none() {
            resolved.zone = Some(default_zone);
        }
        if resolved.owner.is_none() {
            resolved.owner = Some(self.player_filter.clone());
        }
        Ok(resolved)
    }

    pub(crate) fn bind_library_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = filter.clone();
        self.apply_player_refs_to_filter(&mut resolved, ctx);
        if resolved.zone.is_none() {
            resolved.zone = Some(Zone::Library);
        }
        if resolved.owner.is_none() {
            resolved.owner = Some(self.player_filter.clone());
        }
        Ok(resolved)
    }

    pub(crate) fn bind_discard_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        self.bind_owned_zone_filter(filter, ctx, Zone::Hand)
    }

    pub(crate) fn bind_sacrifice_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = self.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
        if resolved.controller.is_none() && resolved.tagged_constraints.is_empty() {
            resolved.controller = Some(self.player_filter.clone());
        }
        Ok(resolved)
    }

    pub(crate) fn bind_controlled_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = self.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
        if resolved.controller.is_none() {
            resolved.controller = Some(self.player_filter.clone());
        }
        Ok(resolved)
    }

    pub(crate) fn bind_battlefield_filter_with_default_controller(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = self.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
        if resolved.zone.is_none() {
            resolved.zone = Some(Zone::Battlefield);
        }
        if resolved.zone == Some(Zone::Battlefield)
            && resolved.controller.is_none()
            && resolved.tagged_constraints.is_empty()
            && resolved.could_be_targeted_by.is_none()
        {
            resolved.controller = Some(self.player_filter.clone());
        }
        Ok(resolved)
    }

    pub(crate) fn bind_revealed_hand_choice_filter(
        &self,
        filter: &ObjectFilter,
        ctx: &mut EffectLoweringContext,
    ) -> Result<ObjectFilter, CardTextError> {
        let mut resolved = self.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
        if resolved.zone.is_none() {
            resolved.zone = Some(Zone::Hand);
        }
        if resolved.owner.is_none() {
            resolved.owner = ctx
                .last_player_filter
                .clone()
                .or_else(|| Some(self.player_filter.clone()));
        }
        Ok(resolved)
    }

    pub(crate) fn apply_player_refs_to_value(
        &self,
        value: &mut Value,
        ctx: &EffectLoweringContext,
    ) {
        if !ctx.iterated_player {
            bind_relative_iterated_player_in_value_to_player_filter(value, &self.player_filter);
        }
    }

    pub(crate) fn apply_player_refs_to_filter(
        &self,
        filter: &mut ObjectFilter,
        ctx: &EffectLoweringContext,
    ) {
        if !ctx.iterated_player {
            bind_relative_iterated_player_filters_to_chooser(filter, &self.player_filter);
        }
    }

    pub(crate) fn target_prelude(&self) -> Vec<Effect> {
        self.choices
            .iter()
            .cloned()
            .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
            .collect()
    }

    pub(crate) fn prepend_target_prelude_if_needed(&self, effect: Effect) -> Vec<Effect> {
        let mut effects = Vec::new();
        if effect.target_spec().is_none() {
            effects.extend(self.target_prelude());
        }
        effects.push(effect);
        effects
    }

    pub(crate) fn build_target_prelude_if_needed(&self, effect: Effect) -> Vec<Effect> {
        self.prepend_target_prelude_if_needed(effect)
    }
}

pub(crate) fn compile_player_filter_effect<Builder>(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(PlayerFilter) -> Effect,
{
    let subject = LoweredSubject::resolve_actor(player, ctx, allow_target, allow_target, true)?;
    let effect = build(subject.clone_player_filter());
    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        effects.extend(subject.target_prelude());
    }
    effects.push(effect);
    Ok((effects, subject.into_choices()))
}

pub(crate) fn compile_player_dual_effect<YouBuilder, OtherBuilder>(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce() -> Effect,
    OtherBuilder: FnOnce(PlayerFilter) -> Effect,
{
    let subject = LoweredSubject::resolve_actor(player, ctx, allow_target, allow_target, true)?;
    let (player_filter, choices) = subject.into_parts();
    compile_player_effect_from_resolved_filter(player_filter, choices, build_you, build_other)
}

pub(crate) fn compile_player_role_effect<Builder>(
    role: SubjectRole,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(LoweredSubject) -> Effect,
{
    let subject = match role {
        SubjectRole::Actor => LoweredSubject::resolve_actor(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?,
        SubjectRole::AffectedPlayer => LoweredSubject::resolve_affected_player(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?,
        SubjectRole::Chooser => LoweredSubject::resolve_chooser(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?,
        SubjectRole::LibraryOwner => LoweredSubject::resolve_library_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?,
        SubjectRole::ZoneOwner => LoweredSubject::resolve_zone_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        )?,
    };
    let effect = build(subject.clone());
    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        effects.extend(subject.target_prelude());
    }
    effects.push(effect);
    Ok((effects, subject.into_choices()))
}

pub(crate) fn compile_player_effect_from_resolved_filter<YouBuilder, OtherBuilder>(
    filter: PlayerFilter,
    choices: Vec<ChooseSpec>,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce() -> Effect,
    OtherBuilder: FnOnce(PlayerFilter) -> Effect,
{
    let effect = if matches!(&filter, PlayerFilter::You) {
        build_you()
    } else {
        build_other(filter)
    };
    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

pub(crate) fn compile_player_value_effect<YouBuilder, OtherBuilder>(
    value: &Value,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce(Value) -> Effect,
    OtherBuilder: FnOnce(Value, PlayerFilter) -> Effect,
{
    let subject = LoweredSubject::resolve_affected_player(
        player,
        ctx,
        allow_target,
        allow_target_opponent,
        track_last_player_filter,
    )?;
    let mut value = value.clone();
    if !ctx.iterated_player {
        let binding_player = ctx
            .last_player_filter
            .as_ref()
            .filter(|filter| !filter.mentions_iterated_player())
            .unwrap_or_else(|| subject.player_filter());
        bind_relative_iterated_player_in_value_to_player_filter(&mut value, binding_player);
    }
    let you_value = value.clone();
    let (player_filter, choices) = subject.into_parts();
    compile_player_effect_from_resolved_filter(
        player_filter,
        choices,
        || build_you(you_value),
        |filter| build_other(value, filter),
    )
}
