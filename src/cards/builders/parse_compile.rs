fn compile_trigger_spec(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::Attacks(filter) => Trigger::attacks(filter),
        TriggerSpec::ThisBlocks => Trigger::this_blocks(),
        TriggerSpec::ThisBlocksObject(filter) => Trigger::this_blocks_object(filter),
        TriggerSpec::ThisBecomesBlocked => Trigger::this_becomes_blocked(),
        TriggerSpec::ThisBlocksOrBecomesBlocked => Trigger::this_blocks_or_becomes_blocked(),
        TriggerSpec::ThisDies => Trigger::this_dies(),
        TriggerSpec::ThisLeavesBattlefield => Trigger::this_leaves_battlefield(),
        TriggerSpec::ThisBecomesMonstrous => Trigger::this_becomes_monstrous(),
        TriggerSpec::ThisBecomesTapped => Trigger::becomes_tapped(),
        TriggerSpec::ThisBecomesUntapped => Trigger::becomes_untapped(),
        TriggerSpec::ThisTurnedFaceUp => Trigger::this_is_turned_face_up(),
        TriggerSpec::ThisBecomesTargeted => Trigger::becomes_targeted(),
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisDealsDamageTo(filter) => Trigger::this_deals_damage_to(filter),
        TriggerSpec::DealsDamage(filter) => Trigger::deals_damage(filter),
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::PlayerLosesLife(player) => Trigger::player_loses_life(player),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::PlayerSacrifices { player, filter } => {
            Trigger::player_sacrifices(player, filter)
        }
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::SpellCast {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            from_not_hand,
        } => Trigger::spell_cast_qualified(
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            from_not_hand,
        ),
        TriggerSpec::SpellCopied { filter, copier } => Trigger::spell_copied(filter, copier),
        TriggerSpec::EntersBattlefield(filter) => Trigger::enters_battlefield(filter),
        TriggerSpec::EntersBattlefieldTapped(filter) => Trigger::enters_battlefield_tapped(filter),
        TriggerSpec::EntersBattlefieldUntapped(filter) => {
            Trigger::enters_battlefield_untapped(filter)
        }
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfDrawStep(player) => Trigger::beginning_of_draw_step(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::ThisEntersBattlefield => Trigger::this_enters_battlefield(),
        TriggerSpec::ThisDealsCombatDamageToPlayer => Trigger::this_deals_combat_damage_to_player(),
        TriggerSpec::DealsCombatDamageToPlayer(filter) => {
            Trigger::deals_combat_damage_to_player(filter)
        }
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction { action, player } => Trigger::keyword_action(action, player),
        TriggerSpec::Custom(description) => {
            Trigger::custom("unimplemented_trigger", description)
        }
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::Either(left, right) => {
            Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right))
        }
    }
}

fn compile_statement_effects(effects: &[EffectAst]) -> Result<Vec<Effect>, CardTextError> {
    let mut ctx = CompileContext::new();
    ctx.allow_life_event_value = false;
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(effects, tag) {
            if ctx.last_object_tag.is_none() {
                ctx.last_object_tag = Some(tag.to_string());
            }
            prelude.push(Effect::tag_attached_to_source(tag));
        }
    }
    let (mut compiled, _) = compile_effects(effects, &mut ctx)?;
    if !prelude.is_empty() {
        prelude.append(&mut compiled);
        Ok(prelude)
    } else {
        Ok(compiled)
    }
}

fn inferred_trigger_player_filter(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    match trigger {
        TriggerSpec::SpellCast { caster, .. } => Some(caster.clone()),
        TriggerSpec::SpellCopied { copier, .. } => Some(copier.clone()),
        TriggerSpec::PlayerLosesLife(player) => Some(player.clone()),
        TriggerSpec::PlayerSacrifices { player, .. } => Some(player.clone()),
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::KeywordAction { player, .. } => {
            if *player == PlayerFilter::Any {
                Some(PlayerFilter::Active)
            } else {
                Some(player.clone())
            }
        }
        TriggerSpec::Custom(_) => None,
        TriggerSpec::Either(left, right) => {
            let left_filter = inferred_trigger_player_filter(left);
            let right_filter = inferred_trigger_player_filter(right);
            if left_filter == right_filter {
                left_filter
            } else {
                None
            }
        }
        _ => None,
    }
}

fn trigger_supports_event_value(trigger: &TriggerSpec, spec: &EventValueSpec) -> bool {
    match spec {
        EventValueSpec::Amount | EventValueSpec::LifeAmount => match trigger {
            TriggerSpec::YouGainLife | TriggerSpec::PlayerLosesLife(_) => true,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec) && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
        EventValueSpec::BlockersBeyondFirst { .. } => match trigger {
            TriggerSpec::ThisBecomesBlocked => true,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec) && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
    }
}

fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut ctx = CompileContext::new();
    ctx.last_player_filter = trigger.and_then(inferred_trigger_player_filter);
    ctx.allow_life_event_value = trigger
        .map(|trigger| trigger_supports_event_value(trigger, &EventValueSpec::Amount))
        .unwrap_or(false);
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(effects, tag) {
            if ctx.last_object_tag.is_none() {
                ctx.last_object_tag = Some(tag.to_string());
            }
            prelude.push(Effect::tag_attached_to_source(tag));
        }
    }
    if ctx.last_object_tag.is_none()
        && (effects_reference_it_tag(effects) || effects_reference_its_controller(effects))
    {
        let default_tag = if matches!(trigger, Some(TriggerSpec::ThisDealsDamageTo(_))) {
            "damaged"
        } else {
            "triggering"
        };
        ctx.last_object_tag = Some(default_tag.to_string());
    }
    let (mut compiled, choices) = compile_effects(effects, &mut ctx)?;
    if !prelude.is_empty() {
        prelude.append(&mut compiled);
        compiled = prelude;
    }
    if effects_reference_tag(effects, "triggering")
        || matches!(ctx.last_object_tag.as_deref(), Some("triggering"))
    {
        compiled.insert(0, Effect::tag_triggering_object("triggering"));
    }
    if effects_reference_tag(effects, "damaged")
        || matches!(ctx.last_object_tag.as_deref(), Some("damaged"))
    {
        compiled.insert(0, Effect::tag_triggering_damage_target("damaged"));
    }
    Ok((compiled, choices))
}

fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

fn effect_references_tag(effect: &EffectAst, tag: &str) -> bool {
    match effect {
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            matches!(creature1, TargetAst::Tagged(t, _) if t.as_str() == tag)
                || matches!(creature2, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::FightIterated { creature2 } => {
            matches!(creature2, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            matches!(source, TargetAst::Tagged(t, _) if t.as_str() == tag)
                || matches!(target, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::Explore { target }
        | EffectAst::Goad { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::ExileUntilSourceLeaves { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::ReturnToHand { target }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::SetBasePowerToughness { target, .. }
        | EffectAst::PumpForEach { target, .. }
        | EffectAst::PumpByLastEffect { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::CreateTokenCopyFromSource { source: target, .. } => {
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::DealDamageEach { filter, .. }
        | EffectAst::PutCountersAll { filter, .. }
        | EffectAst::DoubleCountersOnEach { filter, .. }
        | EffectAst::TapAll { filter }
        | EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PreventDamageEach { filter, .. }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::Enchant { filter }
        | EffectAst::SearchLibrary { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        EffectAst::MoveAllCounters { from, to } => {
            matches!(from, TargetAst::Tagged(t, _) if t.as_str() == tag)
                || matches!(to, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::PutIntoHand { object, .. } => {
            matches!(object, ObjectRefAst::It) && tag == IT_TAG
        }
        EffectAst::CopySpell { target, .. } => {
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::CreateTokenCopy { object, .. } => {
            matches!(object, ObjectRefAst::It) && tag == IT_TAG
        }
        EffectAst::CreateToken { count, .. } | EffectAst::CreateTokenWithMods { count, .. } => {
            value_references_tag(count, tag)
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::UnlessPays { effects, .. } => effects_reference_tag(effects, tag),
        EffectAst::ForEachObject { filter, effects } => {
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == tag)
                || effects_reference_tag(effects, tag)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => effects_reference_tag(effects, tag) || effects_reference_tag(alternative, tag),
        EffectAst::VoteOption { effects, .. } => effects_reference_tag(effects, tag),
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, tag),
        _ => false,
    }
}

fn value_references_tag(value: &Value, tag: &str) -> bool {
    match value {
        Value::Add(left, right) => value_references_tag(left, tag) || value_references_tag(right, tag),
        Value::Count(filter) | Value::CountScaled(filter, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) => choose_spec_references_tag(spec, tag),
        Value::CountersOn(spec, _) => choose_spec_references_tag(spec, tag),
        _ => false,
    }
}

fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(t) => t.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        _ => false,
    }
}

fn choose_spec_references_exiled_tag(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag.as_str().starts_with("exiled_"),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_exiled_tag(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter.tagged_constraints.iter().any(
            |constraint| {
                matches!(
                    constraint.relation,
                    TaggedOpbjectRelation::IsTaggedObject
                ) && constraint.tag.as_str().starts_with("exiled_")
            },
        ),
        _ => false,
    }
}

fn object_ref_references_tag(reference: &ObjectRef, tag: &str) -> bool {
    matches!(reference, ObjectRef::Tagged(found) if found.as_str() == tag)
}

fn player_filter_references_tag(filter: &PlayerFilter, tag: &str) -> bool {
    match filter {
        PlayerFilter::Target(inner) => player_filter_references_tag(inner, tag),
        PlayerFilter::ControllerOf(reference) | PlayerFilter::OwnerOf(reference) => {
            object_ref_references_tag(reference, tag)
        }
        _ => false,
    }
}

fn target_references_tag(target: &TargetAst, tag: &str) -> bool {
    match target {
        TargetAst::Tagged(found, _) => found.as_str() == tag,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            player_filter_references_tag(filter, tag)
        }
        TargetAst::WithCount(inner, _) => target_references_tag(inner, tag),
        TargetAst::Source(_) | TargetAst::AnyTarget(_) | TargetAst::Spell(_) => false,
    }
}

fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
}

fn value_references_event_derived_amount(value: &Value) -> bool {
    matches!(
        value,
        Value::EventValue(EventValueSpec::Amount)
            | Value::EventValue(EventValueSpec::LifeAmount)
            | Value::EventValueOffset(EventValueSpec::Amount, _)
            | Value::EventValueOffset(EventValueSpec::LifeAmount, _)
    )
}

fn effect_references_event_derived_amount(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::CreateToken { count: amount, .. }
        | EffectAst::CreateTokenWithMods { count: amount, .. } => {
            value_references_event_derived_amount(amount)
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::UnlessPays { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            effects.iter().any(effect_references_event_derived_amount)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            effects.iter().any(effect_references_event_derived_amount)
                || alternative
                    .iter()
                    .any(effect_references_event_derived_amount)
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(effect_references_event_derived_amount)
                || if_false.iter().any(effect_references_event_derived_amount)
        }
        _ => false,
    }
}

fn effect_references_its_controller(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseGame { player }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player }
        | EffectAst::RevealTop { player }
        | EffectAst::LookAtTopCards { player, .. }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::CopySpell { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipCombatPhases { player }
        | EffectAst::SkipNextCombatPhaseThisTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateToken { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleGraveyardIntoLibrary { player }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            effects_reference_its_controller(if_true) || effects_reference_its_controller(if_false)
        }
        EffectAst::MayByPlayer { player, effects } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::UnlessPays { effects, .. } => effects_reference_its_controller(effects),
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            effects_reference_its_controller(effects)
                || effects_reference_its_controller(alternative)
        }
        EffectAst::VoteOption { effects, .. } => effects_reference_its_controller(effects),
        _ => false,
    }
}

fn effect_references_it_tag(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Fight {
            creature1,
            creature2,
        } => target_references_tag(creature1, IT_TAG) || target_references_tag(creature2, IT_TAG),
        EffectAst::FightIterated { creature2 } => target_references_tag(creature2, IT_TAG),
        EffectAst::DealDamageEqualToPower { source, target } => {
            target_references_tag(source, IT_TAG) || target_references_tag(target, IT_TAG)
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::Explore { target }
        | EffectAst::Goad { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::ExileUntilSourceLeaves { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::ReturnToHand { target }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::SetBasePowerToughness { target, .. }
        | EffectAst::PumpForEach { target, .. }
        | EffectAst::PumpByLastEffect { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::CreateTokenCopyFromSource { source: target, .. } => {
            target_references_tag(target, IT_TAG)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            matches!(
                predicate,
                PredicateAst::ItIsLandCard | PredicateAst::ItMatches(_)
            ) || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::DealDamageEach { filter, .. }
        | EffectAst::PutCountersAll { filter, .. }
        | EffectAst::DoubleCountersOnEach { filter, .. }
        | EffectAst::TapAll { filter }
        | EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PreventDamageEach { filter, .. }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::Enchant { filter }
        | EffectAst::SearchLibrary { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        EffectAst::MoveAllCounters { from, to } => {
            target_references_tag(from, IT_TAG) || target_references_tag(to, IT_TAG)
        }
        EffectAst::PutIntoHand { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::CopySpell { target, .. } => target_references_tag(target, IT_TAG),
        EffectAst::CreateTokenCopy { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::CreateToken { count, .. } | EffectAst::CreateTokenWithMods { count, .. } => {
            value_references_tag(count, IT_TAG)
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::UnlessPays { effects, .. } => effects_reference_it_tag(effects),
        EffectAst::ForEachObject { filter, effects } => {
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG)
                || effects_reference_it_tag(effects)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => effects_reference_it_tag(effects) || effects_reference_it_tag(alternative),
        EffectAst::VoteOption { effects, .. } => effects_reference_it_tag(effects),
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, IT_TAG),
        _ => false,
    }
}

fn restriction_references_tag(restriction: &crate::effect::Restriction, tag: &str) -> bool {
    use crate::effect::Restriction;

    let maybe_filter = match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter) => Some(filter),
        _ => None,
    };
    let Some(filter) = maybe_filter else {
        return false;
    };

    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
}

fn compile_effects(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut idx = 0;

    while idx < effects.len() {
        if let Some((effect_sequence, effect_choices, consumed)) =
            compile_vote_sequence(&effects[idx..], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += consumed;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_if_do_with_opponent_doesnt(&effects[idx], &effects[idx + 1], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_if_do_with_player_doesnt(&effects[idx], &effects[idx + 1], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_if_do_with_opponent_did(&effects[idx], &effects[idx + 1], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_if_do_with_player_did(&effects[idx], &effects[idx + 1], ctx)?
        {
            compiled.extend(effect_sequence);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        if idx + 1 < effects.len()
            && let EffectAst::CreateTokenWithMods {
                name,
                count,
                player,
                tapped,
                attacking,
                ..
            } = &effects[idx]
            && matches!(effects[idx + 1], EffectAst::ExileThatTokenAtEndOfCombat)
        {
            let effect = EffectAst::CreateTokenWithMods {
                name: name.clone(),
                count: count.clone(),
                player: *player,
                tapped: *tapped,
                attacking: *attacking,
                exile_at_end_of_combat: true,
            };
            let (effect_list, effect_choices) = compile_effect(&effect, ctx)?;
            compiled.extend(effect_list);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        let remaining = if idx + 1 < effects.len() {
            &effects[idx + 1..]
        } else {
            &[]
        };
        ctx.auto_tag_object_targets =
            effects_reference_it_tag(remaining) || effects_reference_its_controller(remaining);

        let next_is_if_result =
            idx + 1 < effects.len() && matches!(effects[idx + 1], EffectAst::IfResult { .. });
        let next_is_if_result_with_opponent_doesnt = next_is_if_result
            && idx + 2 < effects.len()
            && matches!(effects[idx + 2], EffectAst::ForEachOpponentDoesNot { .. });
        let next_is_if_result_with_player_doesnt = next_is_if_result
            && idx + 2 < effects.len()
            && matches!(effects[idx + 2], EffectAst::ForEachPlayerDoesNot { .. });
        let next_is_if_result_with_opponent_did = next_is_if_result
            && idx + 2 < effects.len()
            && matches!(effects[idx + 2], EffectAst::ForEachOpponentDid { .. });
        let next_is_if_result_with_player_did = next_is_if_result
            && idx + 2 < effects.len()
            && matches!(effects[idx + 2], EffectAst::ForEachPlayerDid { .. });
        if next_is_if_result_with_opponent_doesnt
            || next_is_if_result_with_player_doesnt
            || next_is_if_result_with_opponent_did
            || next_is_if_result_with_player_did
        {
            let (mut effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
            if !effect_list.is_empty() {
                let id = ctx.next_effect_id();
                let last = effect_list.pop().expect("non-empty effect list");
                effect_list.push(Effect::with_id(id.0, last));
                ctx.last_effect_id = Some(id);
            } else {
                ctx.last_effect_id = None;
            }

            compiled.extend(effect_list);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }
            idx += 1;
            continue;
        }

        if next_is_if_result {
            let (mut effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
            if !effect_list.is_empty() {
                let id = ctx.next_effect_id();
                let last = effect_list.pop().expect("non-empty effect list");
                effect_list.push(Effect::with_id(id.0, last));
                ctx.last_effect_id = Some(id);
            } else {
                ctx.last_effect_id = None;
            }

            compiled.extend(effect_list);
            for choice in effect_choices {
                push_choice(&mut choices, choice);
            }

            let if_remaining = if idx + 2 < effects.len() {
                &effects[idx + 2..]
            } else {
                &[]
            };
            ctx.auto_tag_object_targets = effects_reference_it_tag(if_remaining)
                || effects_reference_its_controller(if_remaining);
            let (if_effects, if_choices) = compile_effect(&effects[idx + 1], ctx)?;
            compiled.extend(if_effects);
            for choice in if_choices {
                push_choice(&mut choices, choice);
            }
            idx += 2;
            continue;
        }

        let next_needs_event_derived_amount = idx + 1 < effects.len()
            && effect_references_event_derived_amount(&effects[idx + 1]);
        let (mut effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
        if next_needs_event_derived_amount {
            if !effect_list.is_empty() {
                let id = ctx.next_effect_id();
                let last = effect_list.pop().expect("non-empty effect list");
                effect_list.push(Effect::with_id(id.0, last));
                ctx.last_effect_id = Some(id);
            } else {
                ctx.last_effect_id = None;
            }
        }
        compiled.extend(effect_list);
        for choice in effect_choices {
            push_choice(&mut choices, choice);
        }
        idx += 1;
    }

    Ok((compiled, choices))
}

fn collect_tag_spans_from_line(
    line: &LineAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    match line {
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        LineAst::AdditionalCostChoice { options } => {
            for option in options {
                collect_tag_spans_from_effects_with_context(&option.effects, annotations, ctx);
            }
        }
        LineAst::AlternativeCost { .. }
        | LineAst::AlternativeCastingMethod(_)
        | LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::Ability(_)
        | LineAst::Abilities(_) => {}
    }
}

fn collect_tag_spans_from_effects_with_context(
    effects: &[EffectAst],
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    for effect in effects {
        collect_tag_spans_from_effect(effect, annotations, ctx);
    }
}

fn collect_tag_spans_from_effect(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    match effect {
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            collect_tag_spans_from_target(creature1, annotations, ctx);
            collect_tag_spans_from_target(creature2, annotations, ctx);
        }
        EffectAst::FightIterated { creature2 } => {
            collect_tag_spans_from_target(creature2, annotations, ctx);
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            collect_tag_spans_from_target(source, annotations, ctx);
            collect_tag_spans_from_target(target, annotations, ctx);
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::Explore { target }
        | EffectAst::Goad { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
        | EffectAst::ExileUntilSourceLeaves { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::ReturnToHand { target }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::SetBasePowerToughness { target, .. }
        | EffectAst::PumpByLastEffect { target, .. } => {
            collect_tag_spans_from_target(target, annotations, ctx);
        }
        EffectAst::MoveAllCounters { from, to } => {
            collect_tag_spans_from_target(from, annotations, ctx);
            collect_tag_spans_from_target(to, annotations, ctx);
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            collect_tag_spans_from_effects_with_context(if_true, annotations, ctx);
            collect_tag_spans_from_effects_with_context(if_false, annotations, ctx);
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::UnlessPays { effects, .. } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
            collect_tag_spans_from_effects_with_context(alternative, annotations, ctx);
        }
        EffectAst::VoteOption { effects, .. } => {
            collect_tag_spans_from_effects_with_context(effects, annotations, ctx);
        }
        _ => {}
    }
}

fn collect_tag_spans_from_target(
    target: &TargetAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    if let TargetAst::WithCount(inner, _) = target {
        collect_tag_spans_from_target(inner, annotations, ctx);
        return;
    }
    if let TargetAst::Tagged(tag, Some(span)) = target {
        let mapped = map_span_to_original(*span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(tag, mapped);
    }
    if let TargetAst::Object(filter, _, Some(it_span)) = target
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mapped = map_span_to_original(*it_span, &ctx.normalized, &ctx.original, &ctx.char_map);
        annotations.record_tag_span(&TagKey::from(IT_TAG), mapped);
    }
}

fn compile_if_do_with_opponent_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged = EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        }],
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn compile_if_do_with_player_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDoesNot {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged = EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        }],
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn compile_if_do_with_opponent_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDid {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: second_effects.clone(),
    });

    let merged = EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        }],
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn compile_if_do_with_player_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDid {
        effects: second_effects,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: second_effects.clone(),
    });

    let merged = EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        }],
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

#[derive(Debug, Clone)]
struct CompileContextState {
    iterated_player: bool,
    last_effect_id: Option<EffectId>,
    last_object_tag: Option<String>,
    last_player_filter: Option<PlayerFilter>,
}

impl CompileContextState {
    fn capture(ctx: &CompileContext) -> Self {
        Self {
            iterated_player: ctx.iterated_player,
            last_effect_id: ctx.last_effect_id,
            last_object_tag: ctx.last_object_tag.clone(),
            last_player_filter: ctx.last_player_filter.clone(),
        }
    }

    fn restore(self, ctx: &mut CompileContext) {
        ctx.iterated_player = self.iterated_player;
        ctx.last_effect_id = self.last_effect_id;
        ctx.last_object_tag = self.last_object_tag;
        ctx.last_player_filter = self.last_player_filter;
    }
}

fn with_preserved_compile_context<T, Configure, Run>(
    ctx: &mut CompileContext,
    configure: Configure,
    run: Run,
) -> Result<T, CardTextError>
where
    Configure: FnOnce(&mut CompileContext),
    Run: FnOnce(&mut CompileContext) -> Result<T, CardTextError>,
{
    let saved = CompileContextState::capture(ctx);
    configure(ctx);
    let result = run(ctx);
    saved.restore(ctx);
    result
}

fn compile_effects_preserving_last_effect(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_last_effect = ctx.last_effect_id;
    let result = compile_effects(effects, ctx);
    ctx.last_effect_id = saved_last_effect;
    result
}

fn compile_effects_with_temporary_last_effect(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
    last_effect_id: Option<EffectId>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_last_effect = ctx.last_effect_id;
    ctx.last_effect_id = last_effect_id;
    let result = compile_effects(effects, ctx);
    ctx.last_effect_id = saved_last_effect;
    result
}

fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
    tagged_object: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    with_preserved_compile_context(
        ctx,
        |ctx| {
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            if let Some(tag) = tagged_object {
                ctx.last_object_tag = Some(tag);
            }
        },
        |ctx| compile_effects(effects, ctx),
    )
}

fn force_implicit_vote_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::CreateToken { player, .. }
            | EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::MayByTaggedController { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects }
            | EffectAst::ForEachPlayerDoesNot { effects }
            | EffectAst::ForEachOpponentDid { effects }
            | EffectAst::ForEachPlayerDid { effects }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
            | EffectAst::UnlessPays { effects, .. }
            | EffectAst::VoteOption { effects, .. } => {
                force_implicit_vote_token_controller_you(effects);
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                force_implicit_vote_token_controller_you(effects);
                force_implicit_vote_token_controller_you(alternative);
            }
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                force_implicit_vote_token_controller_you(if_true);
                force_implicit_vote_token_controller_you(if_false);
            }
            _ => {}
        }
    }
}

fn compile_vote_sequence(
    effects: &[EffectAst],
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(EffectAst::VoteStart { options }) = effects.first() else {
        return Ok(None);
    };

    let mut option_effects: HashMap<String, Vec<EffectAst>> = HashMap::new();
    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let mut consumed = 1;

    while consumed < effects.len() {
        match &effects[consumed] {
            EffectAst::VoteOption { option, effects } => {
                if option_effects
                    .insert(option.clone(), effects.clone())
                    .is_some()
                {
                    return Err(CardTextError::ParseError(format!(
                        "duplicate vote option clause for '{option}'"
                    )));
                }
                consumed += 1;
            }
            EffectAst::VoteExtra { count, optional } => {
                if *optional {
                    extra_optional = extra_optional.saturating_add(*count);
                } else {
                    extra_mandatory = extra_mandatory.saturating_add(*count);
                }
                consumed += 1;
            }
            _ => break,
        }
    }

    let (vote_options, choices) = with_preserved_compile_context(
        ctx,
        |ctx| {
            ctx.iterated_player = true;
        },
        |ctx| {
            let mut vote_options = Vec::new();
            let mut choices = Vec::new();
            for option in options {
                let mut option_effects_ast = option_effects.get(option).cloned().ok_or_else(|| {
                    CardTextError::ParseError(format!("missing effects for vote option '{option}'"))
                })?;
                force_implicit_vote_token_controller_you(&mut option_effects_ast);
                ctx.last_effect_id = None;
                ctx.last_object_tag = None;
                ctx.last_player_filter = None;
                let (compiled, option_choices) = compile_effects(&option_effects_ast, ctx)?;
                for choice in option_choices {
                    push_choice(&mut choices, choice);
                }
                vote_options.push(VoteOption::new(option.clone(), compiled));
            }
            Ok((vote_options, choices))
        },
    )?;

    let effect = if extra_optional > 0 {
        Effect::vote_with_optional_extra(vote_options, extra_mandatory, extra_optional)
    } else {
        Effect::vote(vote_options, extra_mandatory)
    };

    Ok(Some((vec![effect], choices, consumed)))
}

fn choose_spec_for_targeted_player_filter(filter: &PlayerFilter) -> Option<ChooseSpec> {
    if let PlayerFilter::Target(inner) = filter {
        return Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())));
    }
    None
}

fn collect_targeted_player_specs_from_filter(filter: &ObjectFilter, specs: &mut Vec<ChooseSpec>) {
    if let Some(controller) = &filter.controller
        && let Some(spec) = choose_spec_for_targeted_player_filter(controller)
    {
        push_choice(specs, spec);
    }

    if let Some(owner) = &filter.owner
        && let Some(spec) = choose_spec_for_targeted_player_filter(owner)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_player) = &filter.targets_player
        && let Some(spec) = choose_spec_for_targeted_player_filter(targets_player)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_object) = &filter.targets_object {
        collect_targeted_player_specs_from_filter(targets_object, specs);
    }
}

fn target_context_prelude_for_filter(filter: &ObjectFilter) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let mut choices = Vec::new();
    collect_targeted_player_specs_from_filter(filter, &mut choices);
    let effects = choices
        .iter()
        .cloned()
        .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
        .collect();
    (effects, choices)
}

fn hand_exile_filter_and_count(
    target: &TargetAst,
    ctx: &CompileContext,
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::Object(filter, _, _) => (filter, ChoiceCount::exactly(1)),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => (filter, *count),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let resolved_filter = resolve_it_tag(filter, ctx)?;
    if resolved_filter.zone != Some(Zone::Hand) {
        return Ok(None);
    }
    Ok(Some((resolved_filter, count)))
}

fn lower_hand_exile_target(
    target: &TargetAst,
    ctx: &mut CompileContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let Some((mut filter, count)) = hand_exile_filter_and_count(target, ctx)? else {
        return Ok(None);
    };

    let mut chooser = filter
        .owner
        .clone()
        .or_else(|| filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(filter.owner, Some(PlayerFilter::Target(_))) {
            filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(filter.controller, Some(PlayerFilter::Target(_))) {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    prelude.push(Effect::new(
        crate::effects::ChooseObjectsEffect::new(filter, count, chooser, tag_key.clone())
            .in_zone(Zone::Hand),
    ));
    prelude.push(Effect::new(crate::effects::ExileEffect::with_spec(
        ChooseSpec::Tagged(tag_key),
    )));
    Ok(Some((prelude, choices)))
}

fn compile_effect(
    effect: &EffectAst,
    ctx: &mut CompileContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, target } => {
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::deal_damage(amount.clone(), spec)
                })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((effects, choices))
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            let (source_spec, mut choices) = resolve_target_spec_with_choices(source, ctx)?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                let (target_spec, target_choices) = resolve_target_spec_with_choices(target, ctx)?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                target_spec
            };

            let mut effects = Vec::new();
            let mut amount_source_spec = source_spec.clone();

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                amount_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            let damage_effect = tag_object_target_effect(
                Effect::deal_damage(
                    Value::PowerOf(Box::new(amount_source_spec)),
                    damage_target_spec.clone(),
                ),
                &damage_target_spec,
                ctx,
                "damaged",
            );
            effects.push(damage_effect);

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }

            Ok((effects, choices))
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            let (spec1, mut choices) = resolve_target_spec_with_choices(creature1, ctx)?;
            let (spec2, other_choices) = resolve_target_spec_with_choices(creature2, ctx)?;
            for choice in other_choices {
                push_choice(&mut choices, choice);
            }
            let effect = Effect::fight(spec1.clone(), spec2.clone());
            Ok((vec![effect], choices))
        }
        EffectAst::FightIterated { creature2 } => {
            let (spec2, choices) = resolve_target_spec_with_choices(creature2, ctx)?;
            let effect = Effect::fight(ChooseSpec::Iterated, spec2);
            Ok((vec![effect], choices))
        }
        EffectAst::DealDamageEach { amount, filter } => {
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::deal_damage(amount.clone(), ChooseSpec::Iterated)],
            );
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PutCounters {
            counter_type,
            count,
            target,
            target_count,
            distributed,
        } => {
            let (base_spec, _) = resolve_target_spec_with_choices(target, ctx)?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, count.clone(), spec.clone());
            if let Some(target_count) = target_count {
                put_counters = put_counters.with_target_count(*target_count);
            }
            if *distributed {
                put_counters = put_counters.with_distributed(true);
            }
            let effect =
                tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((vec![effect], choices))
        }
        EffectAst::PutCountersAll {
            counter_type,
            count,
            filter,
        } => {
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(
                    *counter_type,
                    count.clone(),
                    ChooseSpec::Iterated,
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::DoubleCountersOnEach {
            counter_type,
            filter,
        } => {
            let iterated = ChooseSpec::Iterated;
            let count = Value::CountersOn(Box::new(iterated.clone()), Some(*counter_type));
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(*counter_type, count, iterated)],
            );
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::Proliferate => Ok((vec![Effect::proliferate()], Vec::new())),
        EffectAst::Tap { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            Ok((vec![effect], choices))
        }
        EffectAst::TapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::tap_all(resolved_filter));
            Ok((prelude, choices))
        }
        EffectAst::Untap { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            Ok((vec![effect], choices))
        }
        EffectAst::TapOrUntap { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let tap_effect = Effect::tap(spec.clone());
            let untap_effect = Effect::untap(spec.clone());
            use crate::effect::EffectMode;
            let modes = vec![
                EffectMode {
                    description: "Tap".to_string(),
                    effects: vec![tap_effect],
                },
                EffectMode {
                    description: "Untap".to_string(),
                    effects: vec![untap_effect],
                },
            ];
            let effect = Effect::choose_one(modes);
            Ok((vec![effect], choices))
        }
        EffectAst::UntapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::untap_all(resolved_filter));
            Ok((prelude, choices))
        }
        EffectAst::GrantProtectionChoice {
            target,
            allow_colorless,
        } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    description: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];

            for (name, color) in colors {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Color(
                    ColorSet::from(color),
                ));
                modes.push(EffectMode {
                    description: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let effect = Effect::choose_one(modes);
            Ok((vec![effect], choices))
        }
        EffectAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::EarthbendEffect::new(spec.clone(), *counters)),
                &spec,
                ctx,
                "earthbend",
            );
            Ok((vec![effect], vec![spec]))
        }
        EffectAst::Explore { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            Ok((vec![Effect::explore(spec)], choices))
        }
        EffectAst::OpenAttraction => Ok((vec![Effect::open_attraction()], Vec::new())),
        EffectAst::ManifestDread => Ok((vec![Effect::manifest_dread()], Vec::new())),
        EffectAst::Bolster { amount } => Ok((vec![Effect::bolster(*amount)], Vec::new())),
        EffectAst::Support { amount } => Ok((vec![Effect::support(*amount)], Vec::new())),
        EffectAst::Adapt { amount } => Ok((vec![Effect::adapt(*amount)], Vec::new())),
        EffectAst::CounterActivatedOrTriggeredAbility => Ok((
            vec![Effect::counter_activated_or_triggered_ability()],
            Vec::new(),
        )),
        EffectAst::Draw { count, player } => {
            let count = resolve_value_it_tag(count, ctx)?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::draw(count.clone()),
                |filter| Effect::target_draws(count.clone(), filter),
            )
        }
        EffectAst::Counter { target } => {
            compile_tagged_effect_for_target(target, ctx, "countered", Effect::counter)
        }
        EffectAst::CounterUnlessPays { target, mana } => compile_tagged_effect_for_target(
            target,
            ctx,
            "countered",
            |spec| Effect::counter_unless_pays(spec, mana.clone()),
        ),
        EffectAst::LoseLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::lose_life(amount.clone()),
                |filter| Effect::lose_life_player(amount.clone(), filter),
            )
        }
        EffectAst::GainLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::gain_life(amount.clone()),
                |filter| Effect::gain_life_player(amount.clone(), ChooseSpec::Player(filter)),
            )
        }
        EffectAst::LoseGame { player } => compile_player_effect(
            *player,
            ctx,
            false,
            Effect::lose_the_game,
            Effect::lose_the_game_player,
        ),
        EffectAst::PreventAllCombatDamage { duration } => Ok((
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        )),
        EffectAst::PreventAllCombatDamageFromSource { duration, source } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::prevent_all_combat_damage_from(spec, duration.clone())
            })
        }
        EffectAst::PreventAllCombatDamageToPlayers { duration } => Ok((
            vec![Effect::prevent_all_combat_damage_to_players(
                duration.clone(),
            )],
            Vec::new(),
        )),
        EffectAst::PreventAllCombatDamageToYou { duration } => Ok((
            vec![Effect::prevent_all_combat_damage_to_you(duration.clone())],
            Vec::new(),
        )),
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::prevent_damage(amount.clone(), spec, duration.clone())
            })
        }
        EffectAst::PreventDamageEach {
            amount,
            filter,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            let filter = resolve_it_tag(filter, ctx)?;
            let effect = Effect::for_each(
                filter,
                vec![Effect::prevent_damage(
                    amount,
                    ChooseSpec::Iterated,
                    duration.clone(),
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddMana { mana, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::add_mana(mana.clone()),
            |filter| Effect::add_mana_player(mana.clone(), filter),
        ),
        EffectAst::AddManaScaled {
            mana,
            amount,
            player,
        } => compile_player_effect_from_filter(*player, ctx, false, |filter| {
            Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                mana.clone(),
                amount.clone(),
                filter,
            ))
        }),
        EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors,
        } => compile_player_effect(
            *player,
            ctx,
            false,
            || {
                if let Some(colors) = available_colors.clone() {
                    Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                } else {
                    Effect::add_mana_of_any_color(amount.clone())
                }
            },
            |filter| {
                if let Some(colors) = available_colors.clone() {
                    Effect::add_mana_of_any_color_restricted_player(amount.clone(), filter, colors)
                } else {
                    Effect::add_mana_of_any_color_player(amount.clone(), filter)
                }
            },
        ),
        EffectAst::AddManaAnyOneColor { amount, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::add_mana_of_any_one_color(amount.clone()),
            |filter| Effect::add_mana_of_any_one_color_player(amount.clone(), filter),
        ),
        EffectAst::AddManaChosenColor {
            amount,
            player,
            fixed_option,
        } => compile_player_effect_from_filter(*player, ctx, false, |filter| {
            if let Some(fixed) = fixed_option {
                Effect::new(
                    crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
                        amount.clone(),
                        filter,
                        *fixed,
                    ),
                )
            } else {
                Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
                    amount.clone(),
                    filter,
                ))
            }
        }),
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            player,
            land_filter,
            allow_colorless,
            same_type,
        } => compile_player_effect_from_filter(*player, ctx, false, |filter| {
            Effect::add_mana_of_land_produced_types_player(
                amount.clone(),
                filter,
                land_filter.clone(),
                *allow_colorless,
                *same_type,
            )
        }),
        EffectAst::AddManaCommanderIdentity { amount, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::add_mana_from_commander_color_identity(amount.clone()),
            |filter| Effect::add_mana_from_commander_color_identity_player(amount.clone(), filter),
        ),
        EffectAst::AddManaImprintedColors => Ok((
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        )),
        EffectAst::Scry { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::scry(count.clone()),
            |filter| Effect::scry_player(count.clone(), filter),
        ),
        EffectAst::Surveil { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::surveil(count.clone()),
            |filter| Effect::surveil_player(count.clone(), filter),
        ),
        EffectAst::PayMana { cost, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayManaEffect::new(
                    cost.clone(),
                    ChooseSpec::Player(filter),
                ))
            })
        }
        EffectAst::PayEnergy { amount, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayEnergyEffect::new(
                    amount.clone(),
                    ChooseSpec::Player(filter),
                ))
            })
        }
        EffectAst::Cant {
            restriction,
            duration,
        } => {
            let restriction = resolve_restriction_it_tag(restriction, ctx)?;
            Ok((
                vec![Effect::cant_until(restriction, duration.clone())],
                Vec::new(),
            ))
        }
        EffectAst::PlayFromGraveyardUntilEot { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::grant_play_from_graveyard_until_eot(player_filter);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::ExileInsteadOfGraveyardThisTurn { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::exile_instead_of_graveyard_this_turn(player_filter);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::GainControl { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "controlled", |spec| {
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec,
                    crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                    duration.clone(),
                ))
            })
        }
        EffectAst::ControlPlayer { player, duration } => {
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
                ControlDurationAst::AsLongAsYouControlSource => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilSourceLeaves,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(player.clone());
            }

            let effect = Effect::control_player(player.clone(), start, duration);
            Ok((vec![effect], choices))
        }
        EffectAst::ExtraTurnAfterTurn { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, false, true)?;
            Ok((vec![Effect::extra_turn_player(player_filter)], choices))
        }
        EffectAst::DelayedUntilNextEndStep { player, effects } => {
            let (delayed_effects, choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            Ok((vec![effect], choices))
        }
        EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, false, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            Ok((vec![effect], choices))
        }
        EffectAst::DelayedUntilEndOfCombat { effects } => {
            let (delayed_effects, choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::end_of_combat(),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            Ok((vec![effect], choices))
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { effects } => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule 'dies this turn' trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag = Some("triggering".to_string());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (delayed_effects, choices) = compiled?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                Trigger::this_dies(),
                delayed_effects,
                true,
                target_tag,
                PlayerFilter::You,
            ));
            Ok((vec![effect], choices))
        }
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            tag,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let choice_zone = resolved_filter.zone.unwrap_or(Zone::Battlefield);
            let choose_effect =
                crate::effects::ChooseObjectsEffect::new(resolved_filter, *count, chooser, tag.clone())
                    .in_zone(choice_zone);
            let effect = Effect::new(choose_effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            Ok((vec![effect], choices))
        }
        EffectAst::Sacrifice {
            filter,
            player,
            count,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut resolved_filter = match resolve_it_tag(filter, ctx) {
                Ok(resolved) => resolved,
                Err(_)
                    if filter.tagged_constraints.len() == 1
                        && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                {
                    // "Sacrifice it" can legally refer to the source itself in standalone
                    // clauses like "If there are no counters on this land, sacrifice it."
                    ObjectFilter::source()
                }
                Err(err) => return Err(err),
            };
            if resolved_filter.controller.is_none() && resolved_filter.tagged_constraints.is_empty() {
                resolved_filter.controller = Some(chooser.clone());
            }
            if resolved_filter.source {
                if *count != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "source sacrifice only supports count 1 (count: {})",
                        count
                    )));
                }
                if !matches!(chooser, PlayerFilter::You) {
                    return Err(CardTextError::ParseError(
                        "source sacrifice requires source controller chooser".to_string(),
                    ));
                }
                return Ok((vec![Effect::sacrifice_source()], choices));
            }
            if *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                return Ok((
                    vec![Effect::new(crate::effects::SacrificeTargetEffect::new(
                        ChooseSpec::tagged(tag),
                    ))],
                    choices,
                ));
            }

            let tag = ctx.next_tag("sacrificed");
            ctx.last_object_tag = Some(tag.clone());
            let choose = Effect::choose_objects(
                resolved_filter,
                *count as usize,
                chooser.clone(),
                tag.clone(),
            );
            let sacrifice =
                Effect::sacrifice_player(ObjectFilter::tagged(tag), *count, chooser.clone());
            Ok((vec![choose, sacrifice], choices))
        }
        EffectAst::SacrificeAll { filter, player } => {
            let (chooser, choices) =
                resolve_effect_player_filter(*player, ctx, true, false, true)?;
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            Ok((vec![effect], choices))
        }
        EffectAst::DiscardHand { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::discard_hand,
            Effect::discard_hand_player,
        ),
        EffectAst::Discard {
            count,
            player,
            random,
        } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::discard_player(count.clone(), PlayerFilter::You, *random),
            |filter| Effect::discard_player(count.clone(), filter, *random),
        ),
        EffectAst::Connive { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let effect = Effect::connive(spec.clone());
            Ok((vec![effect], choices))
        }
        EffectAst::ConniveIterated => Ok((vec![Effect::connive(ChooseSpec::Iterated)], Vec::new())),
        EffectAst::Goad { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            Ok((vec![Effect::goad(spec)], choices))
        }
        EffectAst::ReturnToHand { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let from_graveyard = target_mentions_graveyard(target);
            let effect = tag_object_target_effect(
                if from_graveyard {
                    Effect::return_from_graveyard_to_hand(spec.clone())
                } else {
                    Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec.clone()))
                },
                &spec,
                ctx,
                "returned",
            );
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller,
        } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let from_exile_tag = choose_spec_references_exiled_tag(&spec);
            let effect = tag_object_target_effect(
                if from_exile_tag && !*tapped {
                    // Blink-style "exile ... then return it" should move the tagged
                    // exiled object back to the battlefield from exile.
                    let move_back = crate::effects::MoveToZoneEffect::new(
                        spec.clone(),
                        Zone::Battlefield,
                        false,
                    );
                    let move_back = match controller {
                        ReturnControllerAst::Preserve => move_back,
                        ReturnControllerAst::Owner => move_back.under_owner_control(),
                        ReturnControllerAst::You => move_back.under_you_control(),
                    };
                    Effect::new(move_back)
                } else {
                    Effect::return_from_graveyard_to_battlefield(spec.clone(), *tapped)
                },
                &spec,
                ctx,
                "returned",
            );
            Ok((vec![effect], choices))
        }
        EffectAst::MoveToZone {
            target,
            zone,
            to_top,
        } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let effect = tag_object_target_effect(
                Effect::move_to_zone(spec.clone(), *zone, *to_top),
                &spec,
                ctx,
                "moved",
            );
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnAllToHand { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            Ok((vec![Effect::return_all_to_hand(resolved_filter)], Vec::new()))
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut effect = Effect::new(crate::effects::ReturnAllToBattlefieldEffect::new(
                resolve_it_tag(filter, ctx)?,
                *tapped,
            ));
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("returned");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::ExchangeControl { filter, count } => {
            let first = ChooseSpec::Object(filter.clone());
            let second = ChooseSpec::Object(filter.clone());
            let effect = Effect::exchange_control(first, second);
            let target_spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
                .with_count(ChoiceCount::exactly(*count as usize));
            Ok((vec![effect], vec![target_spec]))
        }
        EffectAst::SetLifeTotal { amount, player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            |filter| Effect::set_life_total_player(amount.clone(), filter),
        ),
        EffectAst::SkipTurn { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_turn_player,
        ),
        EffectAst::SkipCombatPhases { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_combat_phases_player,
        ),
        EffectAst::SkipNextCombatPhaseThisTurn { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_next_combat_phase_this_turn_player,
        ),
        EffectAst::SkipDrawStep { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_draw_step_player,
        ),
        EffectAst::Regenerate { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
            Ok((vec![effect], choices))
        }
        EffectAst::RegenerateAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::regenerate(
                ChooseSpec::all(filter.clone()),
                crate::effect::Until::EndOfTurn,
            ));
            Ok((prelude, choices))
        }
        EffectAst::Mill { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::mill(count.clone()),
            |filter| Effect::mill_player(count.clone(), filter),
        ),
        EffectAst::PoisonCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::poison_counters(count.clone()),
            |filter| Effect::poison_counters_player(count.clone(), filter),
        ),
        EffectAst::EnergyCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::energy_counters(count.clone()),
            |filter| Effect::energy_counters_player(count.clone(), filter),
        ),
        EffectAst::May { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::may(inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::MayByPlayer { player, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::may_player(player_filter, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::UnlessPays {
            effects,
            player,
            mana,
        } => {
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::unless_pays(inner_effects, player_filter, mana.clone());
            Ok((vec![effect], inner_choices))
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        } => {
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let (alt_effects, alt_choices) = compile_effects(alternative, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::unless_action(inner_effects, alt_effects, player_filter);
            let mut choices = inner_choices;
            choices.extend(alt_choices);
            Ok((vec![effect], choices))
        }
        EffectAst::MayByTaggedController { tag, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::for_each_controller_of_tagged(
                tag.clone(),
                vec![Effect::may(inner_effects)],
            );
            Ok((vec![effect], inner_choices))
        }
        EffectAst::IfResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for if clause".to_string())
            })?;
            let (inner_effects, inner_choices) =
                compile_effects_with_temporary_last_effect(effects, ctx, None)?;
            let predicate = match predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
            };
            let effect = Effect::if_then(condition, predicate, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponent { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_opponent(inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachPlayer { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_players(PlayerFilter::Any, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachObject { filter, effects } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let effect = Effect::for_each(resolved_filter, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "cannot resolve 'this way/it' tag without prior tagged object".to_string(),
                    )
                })?
            } else {
                tag.as_str().to_string()
            };

            let (inner_effects, inner_choices) = compile_effects_in_iterated_player_context(
                effects,
                ctx,
                Some(effective_tag.clone()),
            )?;
            let effect = Effect::for_each_tagged(effective_tag, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponentDoesNot { .. } => Err(CardTextError::ParseError(
            "for each opponent who doesn't must follow an opponent clause".to_string(),
        )),
        EffectAst::ForEachPlayerDoesNot { .. } => Err(CardTextError::ParseError(
            "for each player who doesn't must follow a player clause".to_string(),
        )),
        EffectAst::ForEachOpponentDid { .. } => Err(CardTextError::ParseError(
            "for each opponent who ... this way must follow an opponent clause".to_string(),
        )),
        EffectAst::ForEachPlayerDid { .. } => Err(CardTextError::ParseError(
            "for each player who ... this way must follow a player clause".to_string(),
        )),
        EffectAst::Destroy { target } => {
            compile_tagged_effect_for_target(target, ctx, "destroyed", Effect::destroy)
        }
        EffectAst::DestroyAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        EffectAst::ExileAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::exile_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        EffectAst::Exile { target } => {
            if let Some(compiled) = lower_hand_exile_target(target, ctx)? {
                return Ok(compiled);
            }
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let mut effect = if spec.count().is_single() {
                Effect::move_to_zone(spec.clone(), Zone::Exile, true)
            } else {
                Effect::exile(spec.clone())
            };
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ExileUntilSourceLeaves { target } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let mut effect = Effect::exile_until_source_leaves(spec.clone());
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::LookAtHand { target } => {
            let (effects, choices) = compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::LookAtHandEffect::new(spec))
            })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((effects, choices))
        }
        EffectAst::TargetOnly { target } => compile_tagged_effect_for_target(
            target,
            ctx,
            "targeted",
            |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
        ),
        EffectAst::RevealTop { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, false, true)?;
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::reveal_top(player_filter, tag);
            Ok((vec![effect], choices))
        }
        EffectAst::LookAtTopCards { player, count, tag } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            ctx.last_object_tag = Some(tag.as_str().to_string());
            let effect = Effect::look_at_top_cards(player_filter, *count as usize, tag.clone());
            Ok((vec![effect], choices))
        }
        EffectAst::RevealHand { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, false, true)?;
            let spec = if choices.is_empty() {
                ChooseSpec::Player(player_filter)
            } else {
                ChooseSpec::target_player()
            };
            let effect = Effect::new(crate::effects::LookAtHandEffect::new(spec));
            Ok((vec![effect], choices))
        }
        EffectAst::PutIntoHand { player, object } => {
            let tag = match object {
                ObjectRefAst::It => ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?,
            };
            let (_, choices) = resolve_effect_player_filter(*player, ctx, true, false, true)?;
            let effect = Effect::move_to_zone(ChooseSpec::tagged(tag), Zone::Hand, false);
            Ok((vec![effect], choices))
        }
        EffectAst::CopySpell {
            target,
            count,
            player,
            may_choose_new_targets,
        } => {
            let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let copy_effect = Effect::with_id(
                id.0,
                Effect::new(crate::effects::CopySpellEffect::new_for_player(
                    spec.clone(),
                    count.clone(),
                    player_filter.clone(),
                )),
            );
            let retarget_effect = if *may_choose_new_targets {
                Some(Effect::may_choose_new_targets_player(
                    id,
                    player_filter.clone(),
                ))
            } else {
                None
            };
            let mut compiled = vec![copy_effect];
            if let Some(retarget) = retarget_effect {
                compiled.push(retarget);
            }
            Ok((compiled, choices))
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let saved_last_tag = ctx.last_object_tag.clone();
            let (true_effects, true_choices) = compile_effects(if_true, ctx)?;
            ctx.last_object_tag = saved_last_tag.clone();
            let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
            ctx.last_object_tag = saved_last_tag.clone();
            let condition = match predicate {
                PredicateAst::ItIsLandCard => {
                    let tag = saved_last_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "conditional requires prior reference".to_string(),
                        )
                    })?;
                    Condition::TaggedObjectMatches(
                        tag.into(),
                        ObjectFilter {
                            zone: None,
                            card_types: vec![CardType::Land],
                            ..Default::default()
                        },
                    )
                }
                PredicateAst::ItMatches(filter) => {
                    let tag = saved_last_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "conditional requires prior reference".to_string(),
                        )
                    })?;
                    let mut resolved = filter.clone();
                    resolved.zone = None;
                    Condition::TaggedObjectMatches(tag.into(), resolved)
                }
                PredicateAst::TaggedMatches(tag, filter) => {
                    let mut resolved = filter.clone();
                    resolved.zone = None;
                    Condition::TaggedObjectMatches(tag.clone(), resolved)
                }
                PredicateAst::PlayerControls { player, filter } => {
                    let player = resolve_non_target_player_filter(*player, ctx)?;
                    let mut resolved = resolve_it_tag(filter, ctx)?;
                    resolved.zone = None;
                    Condition::PlayerControls { player, filter: resolved }
                }
                PredicateAst::PlayerHasLessLifeThanYou { player } => {
                    let player = resolve_non_target_player_filter(*player, ctx)?;
                    Condition::PlayerHasLessLifeThanYou { player }
                }
                PredicateAst::SourceIsTapped => Condition::SourceIsTapped,
                PredicateAst::SourceHasNoCounter(counter_type) => {
                    Condition::SourceHasNoCounter(*counter_type)
                }
                PredicateAst::YouAttackedThisTurn => Condition::AttackedThisTurn,
                PredicateAst::NoSpellsWereCastLastTurn => Condition::NoSpellsWereCastLastTurn,
                PredicateAst::TargetWasKicked => Condition::TargetWasKicked,
                PredicateAst::TargetSpellCastOrderThisTurn(order) => {
                    Condition::TargetSpellCastOrderThisTurn(*order)
                }
                PredicateAst::TargetSpellControllerIsPoisoned => {
                    Condition::TargetSpellControllerIsPoisoned
                }
                PredicateAst::TargetSpellNoManaSpentToCast => {
                    Condition::Not(Box::new(Condition::TargetSpellManaSpentToCastAtLeast {
                        amount: 1,
                        symbol: None,
                    }))
                }
                PredicateAst::YouControlMoreCreaturesThanTargetSpellController => {
                    Condition::YouControlMoreCreaturesThanTargetSpellController
                }
                PredicateAst::TargetHasGreatestPowerAmongCreatures => {
                    Condition::TargetHasGreatestPowerAmongCreatures
                }
                PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell => {
                    Condition::TargetManaValueLteColorsSpentToCastThisSpell
                }
                PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
                    Condition::ManaSpentToCastThisSpellAtLeast {
                        amount: *amount,
                        symbol: *symbol,
                    }
                }
            };
            let effect = if false_effects.is_empty() {
                Effect::conditional_only(condition, true_effects)
            } else {
                Effect::conditional(condition, true_effects, false_effects)
            };
            let mut choices = true_choices;
            choices.extend(false_choices);
            Ok((vec![effect], choices))
        }
        EffectAst::Enchant { filter } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()));
            let effect = Effect::attach_to(spec.clone());
            Ok((vec![effect], vec![spec]))
        }
        EffectAst::Investigate => Ok((vec![Effect::investigate(1)], Vec::new())),
        EffectAst::CreateTokenWithMods {
            name,
            count,
            player,
            tapped,
            attacking,
            exile_at_end_of_combat,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let count = resolve_value_it_tag(count, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, count.clone())
            } else {
                crate::effects::CreateTokenEffect::new(token, count.clone(), player_filter.clone())
            };
            if *tapped {
                effect = effect.tapped();
            }
            if *attacking {
                effect = effect.attacking();
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_end_of_combat();
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        EffectAst::CreateToken {
            name,
            count,
            player,
        } => {
            let token = token_definition_for(name.as_str())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let count = resolve_value_it_tag(count, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(player_filter, PlayerFilter::You) {
                Effect::create_tokens(token, count.clone())
            } else {
                Effect::create_tokens_player(token, count, player_filter)
            };
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::CreateTokenCopy {
            object,
            count,
            player,
            half_power_toughness_round_up,
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
        } => {
            let tag = match object {
                ObjectRefAst::It => ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'that creature' without prior reference".to_string(),
                    )
                })?,
            };
            let tag: TagKey = tag.into();
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::Tagged(tag),
                *count,
                player_filter,
            );
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        EffectAst::CreateTokenCopyFromSource {
            source,
            count,
            player,
            half_power_toughness_round_up,
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
        } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            compile_effect_for_target(source, ctx, |source_spec| {
                let mut effect =
                    crate::effects::CreateTokenCopyEffect::new(source_spec, *count, player_filter);
                if *half_power_toughness_round_up {
                    effect = effect.half_power_toughness_round_up();
                }
                if *has_haste {
                    effect = effect.haste(true);
                }
                if *sacrifice_at_next_end_step {
                    effect = effect.sacrifice_at_next_end_step(true);
                }
                if *exile_at_next_end_step {
                    effect = effect.exile_at_next_end_step(true);
                }
                Effect::new(effect)
            })
        }
        EffectAst::ExileThatTokenAtEndOfCombat => Ok((Vec::new(), Vec::new())),
        EffectAst::TokenCopyGainHasteUntilEot
        | EffectAst::TokenCopySacrificeAtNextEndStep
        | EffectAst::TokenCopyExileAtNextEndStep => {
            Ok((Vec::new(), Vec::new()))
        }
        EffectAst::Monstrosity { amount } => {
            Ok((vec![Effect::monstrosity(amount.clone())], Vec::new()))
        }
        EffectAst::RemoveUpToAnyCounters { amount, target } => {
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            compile_tagged_effect_for_target(target, ctx, "counters", |spec| {
                Effect::with_id(
                    id.0,
                    Effect::remove_up_to_any_counters(amount.clone(), spec),
                )
            })
        }
        EffectAst::MoveAllCounters { from, to } => {
            let (from_spec, mut choices) = resolve_target_spec_with_choices(from, ctx)?;
            let (to_spec, to_choices) = resolve_target_spec_with_choices(to, ctx)?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            Ok((vec![effect], choices))
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec,
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                    },
                    duration.clone(),
                )
                .require_creature_target(),
            )
        }),
        EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_pt", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        }),
        EffectAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count_filter,
            duration,
        } => {
            let resolved_count_filter = resolve_it_tag(count_filter, ctx)?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::pump_for_each(
                    spec,
                    *power_per,
                    *toughness_per,
                    Value::Count(resolved_count_filter.clone()),
                    duration.clone(),
                )
            })
        }
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::new(crate::effects::ApplyContinuousEffect::new_runtime(
                crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: power.clone(),
                    toughness: toughness.clone(),
                },
                duration.clone(),
            )
            .lock_filter_at_resolution())
            .tag_all(tag.clone());
            ctx.last_object_tag = Some(tag);
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } => {
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: power_value.clone(),
                            toughness: Value::Fixed(*toughness),
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })
        }
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            if abilities.is_empty() {
                return Ok((Vec::new(), Vec::new()));
            }

            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                crate::continuous::Modification::AddAbility(abilities[0].clone()),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for ability in abilities.iter().skip(1) {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddAbility(ability.clone()),
                );
            }

            Ok((vec![Effect::new(apply)], Vec::new()))
        }
        EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        } => {
            let Some(first_ability) = abilities.first() else {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec))
                });
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::AddAbility(first_ability.clone()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::AddAbility(ability.clone()),
                    );
                }

                Effect::new(apply)
            })
        }
        EffectAst::Transform { target } => {
            compile_tagged_effect_for_target(target, ctx, "transformed", Effect::transform)
        }
        EffectAst::GrantAbilityToSource { ability } => Ok((
            vec![Effect::grant_object_ability_to_source(ability.clone())],
            Vec::new(),
        )),
        EffectAst::SearchLibrary {
            filter,
            destination,
            player,
            reveal,
            shuffle,
            count,
            tapped,
        } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let count = *count;
            let mut filter = filter.clone();
            if filter.owner.is_none() && !matches!(player_filter, PlayerFilter::You) {
                filter.owner = Some(player_filter.clone());
            }
            let use_search_effect = count.min == 0
                && count.max == Some(1)
                && !(*destination == Zone::Battlefield && *tapped);
            if use_search_effect {
                let effects = vec![Effect::search_library(
                    filter,
                    *destination,
                    player_filter.clone(),
                    *reveal,
                )];
                Ok((effects, Vec::new()))
            } else {
                let tag = ctx.next_tag("searched");
                let choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    count,
                    player_filter.clone(),
                    tag.clone(),
                )
                .in_zone(Zone::Library)
                .with_description("cards")
                .as_search();
                let choose = if *reveal { choose.reveal() } else { choose };

                let to_top = matches!(destination, Zone::Library);
                let move_effect = if *destination == Zone::Battlefield && *tapped {
                    Effect::put_onto_battlefield(ChooseSpec::Iterated, true, player_filter.clone())
                } else {
                    Effect::move_to_zone(ChooseSpec::Iterated, *destination, to_top)
                };
                let mut sequence_effects = vec![Effect::new(choose)];
                if *shuffle && *destination == Zone::Library {
                    sequence_effects.push(Effect::shuffle_library_player(player_filter.clone()));
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                } else {
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                    if *shuffle {
                        sequence_effects.push(Effect::shuffle_library_player(player_filter));
                    }
                }
                let sequence = crate::effects::SequenceEffect::new(sequence_effects);
                Ok((vec![Effect::new(sequence)], Vec::new()))
            }
        }
        EffectAst::ShuffleGraveyardIntoLibrary { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::shuffle_graveyard_into_library_player,
        ),
        EffectAst::ShuffleLibrary { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::shuffle_library_player,
        ),
        EffectAst::VoteStart { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. } => Err(CardTextError::ParseError(
            "vote clauses must appear together".to_string(),
        )),
    }
}

fn resolve_effect_player_filter(
    player: PlayerAst,
    ctx: &mut CompileContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<(PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    let (filter, choices) = match player {
        PlayerAst::Target if allow_target => {
            (PlayerFilter::target_player(), vec![ChooseSpec::target_player()])
        }
        PlayerAst::TargetOpponent if allow_target_opponent => (
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
            vec![ChooseSpec::target(ChooseSpec::Player(
                PlayerFilter::Opponent,
            ))],
        ),
        _ => (resolve_non_target_player_filter(player, ctx)?, Vec::new()),
    };

    if track_last_player_filter && !matches!(player, PlayerAst::Implicit) {
        ctx.last_player_filter = Some(filter.clone());
    }
    Ok((filter, choices))
}

fn compile_player_effect<YouBuilder, OtherBuilder>(
    player: PlayerAst,
    ctx: &mut CompileContext,
    allow_target: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce() -> Effect,
    OtherBuilder: FnOnce(PlayerFilter) -> Effect,
{
    let (filter, choices) = resolve_effect_player_filter(
        player,
        ctx,
        allow_target,
        allow_target,
        true,
    )?;
    let effect = if matches!(&filter, PlayerFilter::You) {
        build_you()
    } else {
        build_other(filter)
    };
    Ok((vec![effect], choices))
}

fn compile_player_effect_from_filter<Builder>(
    player: PlayerAst,
    ctx: &mut CompileContext,
    allow_target: bool,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(PlayerFilter) -> Effect,
{
    let (filter, choices) = resolve_effect_player_filter(
        player,
        ctx,
        allow_target,
        allow_target,
        true,
    )?;
    Ok((vec![build(filter)], choices))
}

fn resolve_non_target_player_filter(
    player: PlayerAst,
    ctx: &CompileContext,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
        PlayerAst::Any => Ok(PlayerFilter::Any),
        PlayerAst::Defending => Ok(PlayerFilter::Defending),
        PlayerAst::Target | PlayerAst::TargetOpponent => Err(CardTextError::ParseError(
            "target player requires explicit targeting".to_string(),
        )),
        PlayerAst::Opponent => Ok(PlayerFilter::Opponent),
        PlayerAst::That => {
            if ctx.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else if let Some(filter) = &ctx.last_player_filter {
                Ok(filter.clone())
            } else {
                Err(CardTextError::ParseError(
                    "cannot resolve 'that player' without context".to_string(),
                ))
            }
        }
        PlayerAst::ItsController => {
            if let Some(tag) = ctx.last_object_tag.as_ref() {
                Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)))
            } else {
                Ok(PlayerFilter::ControllerOf(ObjectRef::Target))
            }
        }
        PlayerAst::ItsOwner => {
            if let Some(tag) = ctx.last_object_tag.as_ref() {
                Ok(PlayerFilter::OwnerOf(ObjectRef::tagged(tag)))
            } else {
                Ok(PlayerFilter::OwnerOf(ObjectRef::Target))
            }
        }
        PlayerAst::Implicit => {
            if ctx.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else {
                Ok(PlayerFilter::You)
            }
        }
    }
}

fn resolve_it_tag(
    filter: &ObjectFilter,
    ctx: &CompileContext,
) -> Result<ObjectFilter, CardTextError> {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        return Ok(filter.clone());
    }

    let tag = ctx.last_object_tag.as_ref().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'it' without prior reference".to_string())
    })?;

    let mut resolved = filter.clone();
    for constraint in &mut resolved.tagged_constraints {
        if constraint.tag.as_str() == IT_TAG {
            constraint.tag = tag.into();
        }
    }
    Ok(resolved)
}

fn object_filter_as_tagged_reference(filter: &ObjectFilter) -> Option<TagKey> {
    if filter.tagged_constraints.len() != 1 {
        return None;
    }
    let constraint = &filter.tagged_constraints[0];
    if !matches!(
        constraint.relation,
        TaggedOpbjectRelation::IsTaggedObject
    ) {
        return None;
    }

    let mut bare = filter.clone();
    bare.tagged_constraints.clear();
    if bare == ObjectFilter::default() {
        Some(constraint.tag.clone())
    } else {
        None
    }
}

fn resolve_restriction_it_tag(
    restriction: &crate::effect::Restriction,
    ctx: &CompileContext,
) -> Result<crate::effect::Restriction, CardTextError> {
    use crate::effect::Restriction;

    let resolved = match restriction {
        Restriction::Attack(filter) => Restriction::attack(resolve_it_tag(filter, ctx)?),
        Restriction::Block(filter) => Restriction::block(resolve_it_tag(filter, ctx)?),
        Restriction::Untap(filter) => Restriction::untap(resolve_it_tag(filter, ctx)?),
        Restriction::BeBlocked(filter) => Restriction::be_blocked(resolve_it_tag(filter, ctx)?),
        Restriction::BeDestroyed(filter) => Restriction::be_destroyed(resolve_it_tag(filter, ctx)?),
        Restriction::BeSacrificed(filter) => {
            Restriction::be_sacrificed(resolve_it_tag(filter, ctx)?)
        }
        Restriction::HaveCountersPlaced(filter) => {
            Restriction::have_counters_placed(resolve_it_tag(filter, ctx)?)
        }
        Restriction::BeTargeted(filter) => Restriction::be_targeted(resolve_it_tag(filter, ctx)?),
        Restriction::BeCountered(filter) => Restriction::be_countered(resolve_it_tag(filter, ctx)?),
        _ => restriction.clone(),
    };
    Ok(resolved)
}

fn resolve_choose_spec_it_tag(
    spec: &ChooseSpec,
    ctx: &CompileContext,
) -> Result<ChooseSpec, CardTextError> {
    match spec {
        ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG => {
            let resolved = ctx.last_object_tag.as_ref().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'it' without prior reference".to_string(),
                )
            })?;
            Ok(ChooseSpec::Tagged(TagKey::from(resolved.as_str())))
        }
        ChooseSpec::Tagged(tag) => Ok(ChooseSpec::Tagged(tag.clone())),
        ChooseSpec::Object(filter) => {
            let resolved = resolve_it_tag(filter, ctx)?;
            if let Some(tag) = object_filter_as_tagged_reference(&resolved) {
                Ok(ChooseSpec::Tagged(tag))
            } else {
                Ok(ChooseSpec::Object(resolved))
            }
        }
        ChooseSpec::Target(inner) => Ok(ChooseSpec::Target(Box::new(resolve_choose_spec_it_tag(
            inner, ctx,
        )?))),
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag(inner, ctx)?),
            count.clone(),
        )),
        ChooseSpec::All(filter) => Ok(ChooseSpec::All(resolve_it_tag(filter, ctx)?)),
        ChooseSpec::Player(filter) => Ok(ChooseSpec::Player(filter.clone())),
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            Ok(ChooseSpec::PlayerOrPlaneswalker(filter.clone()))
        }
        ChooseSpec::SpecificObject(id) => Ok(ChooseSpec::SpecificObject(*id)),
        ChooseSpec::SpecificPlayer(id) => Ok(ChooseSpec::SpecificPlayer(*id)),
        ChooseSpec::AnyTarget => Ok(ChooseSpec::AnyTarget),
        ChooseSpec::Source => Ok(ChooseSpec::Source),
        ChooseSpec::SourceController => Ok(ChooseSpec::SourceController),
        ChooseSpec::SourceOwner => Ok(ChooseSpec::SourceOwner),
        ChooseSpec::EachPlayer(filter) => Ok(ChooseSpec::EachPlayer(filter.clone())),
        ChooseSpec::Iterated => Ok(ChooseSpec::Iterated),
    }
}

fn resolve_value_it_tag(value: &Value, ctx: &CompileContext) -> Result<Value, CardTextError> {
    match value {
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(resolve_value_it_tag(left, ctx)?),
            Box::new(resolve_value_it_tag(right, ctx)?),
        )),
        Value::Count(filter) => Ok(Value::Count(resolve_it_tag(filter, ctx)?)),
        Value::CountScaled(filter, multiplier) => {
            Ok(Value::CountScaled(resolve_it_tag(filter, ctx)?, *multiplier))
        }
        Value::PowerOf(spec) => Ok(Value::PowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        Value::ManaValueOf(spec) => Ok(Value::ManaValueOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        Value::EventValue(EventValueSpec::Amount) | Value::EventValue(EventValueSpec::LifeAmount) => {
            if !ctx.allow_life_event_value {
                if let Some(id) = ctx.last_effect_id {
                    return Ok(Value::EffectValue(id));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            if !ctx.allow_life_event_value {
                if let Some(id) = ctx.last_effect_id {
                    return Ok(Value::EffectValueOffset(id, *offset));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        _ => Ok(value.clone()),
    }
}

fn choose_spec_targets_object(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::Object(_)
        | ChooseSpec::Tagged(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source => true,
        _ => false,
    }
}

fn tag_object_target_effect(
    effect: Effect,
    spec: &ChooseSpec,
    ctx: &mut CompileContext,
    prefix: &str,
) -> Effect {
    if ctx.auto_tag_object_targets && spec.is_target() && choose_spec_targets_object(spec) {
        let tag = ctx.next_tag(prefix);
        ctx.last_object_tag = Some(tag.clone());
        effect.tag(tag)
    } else {
        effect
    }
}

fn eldrazi_spawn_or_scion_mana_ability() -> Ability {
    Ability {
        kind: AbilityKind::Mana(ManaAbility::with_cost_effects(
            TotalCost::free(),
            vec![Effect::sacrifice_source()],
            vec![ManaSymbol::Colorless],
        )),
        functional_zones: vec![Zone::Battlefield],
        text: Some("Sacrifice this creature: Add {C}.".to_string()),
    }
}

fn eldrazi_spawn_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Spawn")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Spawn])
        .power_toughness(PowerToughness::fixed(0, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

fn eldrazi_scion_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Scion")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Scion])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

fn parse_number_word(word: &str) -> Option<i32> {
    match word {
        "zero" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => word.parse::<i32>().ok(),
    }
}

fn parse_deals_damage_amount(words: &[&str]) -> Option<i32> {
    for window in words.windows(3) {
        if (window[0] == "deals" || window[0] == "deal") && window[2] == "damage" {
            return parse_number_word(window[1]);
        }
    }
    None
}

fn token_inline_noncreature_spell_each_opponent_damage_amount(name: &str) -> Option<i32> {
    let lower_name = name.to_ascii_lowercase();
    let words: Vec<&str> = lower_name
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let has_noncreature_cast_trigger = words
        .windows(6)
        .any(|window| window == ["whenever", "you", "cast", "a", "noncreature", "spell"])
        || words
            .windows(5)
            .any(|window| window == ["whenever", "you", "cast", "noncreature", "spell"]);
    if !has_noncreature_cast_trigger {
        return None;
    }
    let has_damage_subject = words
        .windows(3)
        .any(|window| {
            window == ["this", "token", "deals"]
                || window == ["this", "creature", "deals"]
                || window == ["this", "token", "deal"]
                || window == ["this", "creature", "deal"]
        })
        || words
            .windows(2)
            .any(|window| window == ["it", "deals"] || window == ["it", "deal"]);
    if !has_damage_subject {
        return None;
    }
    if !words
        .windows(3)
        .any(|window| window == ["to", "each", "opponent"])
    {
        return None;
    }
    parse_deals_damage_amount(&words)
}

fn parse_crew_amount(words: &[&str]) -> Option<u32> {
    let crew_idx = words.iter().position(|word| *word == "crew")?;
    let amount_word = words.get(crew_idx + 1)?;
    let amount = parse_number_word(amount_word)?;
    u32::try_from(amount).ok()
}

fn token_dies_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: vec![Effect::deal_damage(Value::Fixed(amount), target.clone())],
            choices: vec![target],
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "When this token dies, it deals {amount} damage to any target."
        )),
    }
}

fn token_leaves_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_leaves_battlefield(),
            effects: vec![Effect::deal_damage(Value::Fixed(amount), target.clone())],
            choices: vec![target],
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "When this token leaves the battlefield, it deals {amount} damage to any target."
        )),
    }
}

fn token_dies_target_creature_gets_minus_one_minus_one_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: vec![Effect::pump(-1, -1, target.clone(), Until::EndOfTurn)],
            choices: vec![target],
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "When this token dies, target creature gets -1/-1 until end of turn.".to_string(),
        ),
    }
}

fn token_red_pump_ability() -> Ability {
    Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])),
                Vec::new(),
            ),
            effects: vec![Effect::pump(1, 0, ChooseSpec::Source, Until::EndOfTurn)],
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{R}: This creature gets +1/+0 until end of turn.".to_string()),
    }
}

fn token_white_tap_target_creature_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::White]])),
                vec![Effect::tap_source()],
            ),
            effects: vec![Effect::tap(target.clone())],
            choices: vec![target],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{W}, {T}: Tap target creature.".to_string()),
    }
}

fn token_damage_to_player_poison_counter_ability() -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_deals_combat_damage_to_player(),
            effects: vec![Effect::poison_counters_player(1, PlayerFilter::DamagedPlayer)],
            choices: Vec::new(),
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "Whenever this creature deals damage to a player, that player gets a poison counter."
                .to_string(),
        ),
    }
}

fn token_noncreature_spell_each_opponent_damage_ability(amount: i32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::spell_cast(
                Some(ObjectFilter::spell().without_type(CardType::Creature)),
                PlayerFilter::You,
            ),
            effects: vec![Effect::for_each_opponent(vec![Effect::deal_damage(
                Value::Fixed(amount),
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )])],
            choices: Vec::new(),
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "Whenever you cast a noncreature spell, this token deals {amount} damage to each opponent."
        )),
    }
}

fn parse_token_mana_symbol(word: &str) -> Option<ManaSymbol> {
    match word {
        "w" => Some(ManaSymbol::White),
        "u" => Some(ManaSymbol::Blue),
        "b" => Some(ManaSymbol::Black),
        "r" => Some(ManaSymbol::Red),
        "g" => Some(ManaSymbol::Green),
        "c" => Some(ManaSymbol::Colorless),
        "x" => Some(ManaSymbol::X),
        _ => word
            .parse::<u8>()
            .ok()
            .map(ManaSymbol::Generic),
    }
}

fn title_case_words(words: &[&str]) -> String {
    words.iter()
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut out = first.to_uppercase().to_string();
                out.push_str(chars.as_str());
                out
            } else {
                String::new()
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_named_card_name(words: &[&str]) -> Option<String> {
    let named_idx = words.iter().position(|word| *word == "named")?;
    let stop_words = [
        "from", "to", "and", "with", "that", "it", "at", "until", "if", "where",
    ];
    let mut end = named_idx + 1;
    while end < words.len() && !stop_words.contains(&words[end]) {
        end += 1;
    }
    if end <= named_idx + 1 {
        return None;
    }
    Some(title_case_words(&words[named_idx + 1..end]))
}

fn token_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    mana_symbols: Vec<ManaSymbol>,
    tap_cost: bool,
) -> Ability {
    let mut cost_effects = Vec::new();
    if tap_cost {
        cost_effects.push(Effect::tap_source());
    }
    cost_effects.push(Effect::sacrifice_source());
    let mana_cost = if mana_symbols.is_empty() {
        ManaCost::new()
    } else {
        ManaCost::from_pips(mana_symbols.into_iter().map(|symbol| vec![symbol]).collect())
    };
    let mut cost_parts = Vec::new();
    if !mana_cost.is_empty() {
        cost_parts.push(mana_cost.to_oracle());
    }
    if tap_cost {
        cost_parts.push("{T}".to_string());
    }
    cost_parts.push("Sacrifice this token".to_string());
    let cost_text = cost_parts.join(", ");
    let target = ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    );
    Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: ability::merge_cost_effects(TotalCost::mana(mana_cost), cost_effects),
            effects: vec![Effect::return_from_graveyard_to_battlefield(
                target.clone(),
                false,
            )],
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!(
            "{cost_text}: Return a card named {card_name} from your graveyard to the battlefield."
        )),
    }
}

fn token_upkeep_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    grants_haste: bool,
) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    ));
    let mut effects = vec![
        Effect::sacrifice_source(),
        Effect::return_from_graveyard_to_battlefield(target.clone(), false),
    ];
    if grants_haste {
        effects.push(Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            target.clone(),
            crate::continuous::Modification::AddAbility(StaticAbility::haste()),
            Until::EndOfTurn,
        )));
    }
    let mut text = format!(
        "At the beginning of your upkeep, sacrifice this token and return target card named {card_name} from your graveyard to the battlefield."
    );
    if grants_haste {
        text.push_str(" It gains haste until end of turn.");
    }
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects,
            choices: vec![target],
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(text),
    }
}

fn token_dies_create_dragon_with_firebreathing_ability() -> Ability {
    let dragon = CardDefinitionBuilder::new(CardId::new(), "Dragon")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .with_ability(token_red_pump_ability())
        .build();
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: vec![Effect::create_tokens(dragon, Value::Fixed(1))],
            choices: Vec::new(),
            intervening_if: None,
            once_each_turn: false,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'".to_string(),
        ),
    }
}

fn token_definition_for(name: &str) -> Option<CardDefinition> {
    let lower = name.trim().to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-'
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let has_word = |needle: &str| words.iter().any(|word| *word == needle);
    let has_explicit_pt = words.iter().any(|word| parse_token_pt(word).is_some());

    if has_word("treasure") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::treasure_token_definition());
    }
    if has_word("clue") && !words.contains(&"creature") {
        return Some(crate::cards::tokens::clue_token_definition());
    }
    if has_word("eldrazi") && has_word("spawn") {
        return Some(eldrazi_spawn_token_definition());
    }
    if has_word("eldrazi") && has_word("scion") {
        return Some(eldrazi_scion_token_definition());
    }
    if has_word("food") && !words.contains(&"creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Food")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food]);
        return Some(builder.build());
    }
    if has_word("blood") && !words.contains(&"creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Blood")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("powerstone") && !words.contains(&"creature") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Powerstone")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("vehicle") && has_word("artifact") && !words.contains(&"creature") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Vehicle")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle]);
        if let Some((power, toughness)) = words.iter().find_map(|word| parse_token_pt(word)) {
            builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
        }
        if let Some(crew_amount) = parse_crew_amount(&words) {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::custom(
                "crew",
                format!("crew {crew_amount}"),
            )));
        }
        return Some(builder.build());
    }
    if has_word("artifact") && !words.contains(&"creature") && !has_explicit_pt {
        let mut subtypes = Vec::new();
        for word in &words {
            if let Some(subtype) = parse_subtype_word(word)
                && !subtype.is_creature_type()
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }
        let token_name = words
            .iter()
            .find(|word| {
                !matches!(
                    **word,
                    "artifact"
                        | "token"
                        | "tokens"
                        | "named"
                        | "colorless"
                        | "white"
                        | "blue"
                        | "black"
                        | "red"
                        | "green"
                )
            })
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        let mut name = first.to_uppercase().to_string();
                        name.push_str(chars.as_str());
                        name
                    }
                    None => "Artifact".to_string(),
                }
            })
            .unwrap_or_else(|| "Artifact".to_string());
        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(vec![CardType::Artifact]);
        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"leaves")
            && words.contains(&"battlefield")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"target")
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(token_leaves_deals_damage_any_target_ability(amount));
        }
        return Some(builder.build());
    }
    if has_word("angel") && !has_explicit_pt {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(4, 4))
            .flying();
        return Some(builder.build());
    }
    if has_word("wall")
        && lower.contains("0/4")
        && lower.contains("artifact")
        && lower.contains("creature")
    {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Wall")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Wall])
            .power_toughness(PowerToughness::fixed(0, 4))
            .defender();
        return Some(builder.build());
    }
    if has_word("squirrel") && lower.contains("1/1") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Squirrel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Squirrel])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(1, 1));
        return Some(builder.build());
    }
    let is_dragon_egg_death_spawn_pattern = has_word("dragon")
        && has_word("egg")
        && lower.contains("0/2")
        && words.contains(&"when")
        && words.contains(&"token")
        && words.contains(&"dies")
        && words.contains(&"create")
        && words.contains(&"2/2")
        && words.contains(&"flying")
        && words.contains(&"r")
        && words.contains(&"+1/+0");
    if is_dragon_egg_death_spawn_pattern {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Dragon Egg")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Dragon])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(0, 2))
            .defender()
            .with_ability(token_dies_create_dragon_with_firebreathing_ability());
        return Some(builder.build());
    }
    if has_word("elephant") && lower.contains("3/3") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Elephant")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3));
        return Some(builder.build());
    }
    let has_construct_cda_words = words.contains(&"power")
        && words.contains(&"toughness")
        && words.contains(&"equal")
        && words.contains(&"number")
        && words.contains(&"artifacts")
        && words.contains(&"you")
        && words.contains(&"control");
    if has_word("construct") && (!has_explicit_pt || has_construct_cda_words) {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Construct")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Construct])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(
                StaticAbility::characteristic_defining_pt(
                    Value::Count(ObjectFilter::artifact().you_control()),
                    Value::Count(ObjectFilter::artifact().you_control()),
                ),
            ));
        return Some(builder.build());
    }
    if has_word("shapeshifter") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(3, 2));
        if lower.contains("changeling") || lower == "shapeshifter" {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }
        return Some(builder.build());
    }
    if has_word("astartes")
        && has_word("warrior")
        && lower.contains("2/2")
        && lower.contains("white")
    {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Astartes Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(2, 2));
        if lower.contains("vigilance") {
            builder = builder.vigilance();
        }
        return Some(builder.build());
    }
    if words.contains(&"creature") {
        let mut card_types = vec![CardType::Creature];
        if words.contains(&"artifact") {
            card_types.insert(0, CardType::Artifact);
        }
        if words.contains(&"enchantment") {
            card_types.insert(0, CardType::Enchantment);
        }

        let (power, toughness) = words.iter().find_map(|word| parse_token_pt(word))?;

        let mut subtypes = Vec::new();
        for word in &words {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }

        let token_name = subtypes
            .first()
            .map(|subtype| format!("{subtype:?}"))
            .unwrap_or_else(|| "Token".to_string());

        let mut builder = CardDefinitionBuilder::new(CardId::new(), token_name)
            .token()
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(power, toughness));

        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }

        let mut colors = ColorSet::new();
        if words.contains(&"white") {
            colors = colors.union(ColorSet::WHITE);
        }
        if words.contains(&"blue") {
            colors = colors.union(ColorSet::BLUE);
        }
        if words.contains(&"black") {
            colors = colors.union(ColorSet::BLACK);
        }
        if words.contains(&"red") {
            colors = colors.union(ColorSet::RED);
        }
        if words.contains(&"green") {
            colors = colors.union(ColorSet::GREEN);
        }
        if !colors.is_empty() {
            builder = builder.color_indicator(colors);
        }

        if words.contains(&"flying") {
            builder = builder.flying();
        }
        if words.contains(&"prowess") {
            builder = builder.prowess();
        }
        if words.contains(&"vigilance") {
            builder = builder.vigilance();
        }
        if words.contains(&"trample") {
            builder = builder.trample();
        }
        if words.contains(&"lifelink") {
            builder = builder.lifelink();
        }
        if words.contains(&"deathtouch") {
            builder = builder.deathtouch();
        }
        if words.contains(&"haste") {
            builder = builder.haste();
        }
        if words.contains(&"menace") {
            builder = builder.menace();
        }
        if words.contains(&"reach") {
            builder = builder.reach();
        }
        if words.contains(&"hexproof") {
            builder = builder.hexproof();
        }
        if words.contains(&"indestructible") {
            builder = builder.indestructible();
        }
        if words.contains(&"sacrifice")
            && words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"return")
            && words.contains(&"named")
            && words.contains(&"graveyard")
            && words.contains(&"battlefield")
            && !words.contains(&"beginning")
            && let Some(card_name) = extract_named_card_name(&words)
            && let Some(sacrifice_idx) = words.iter().position(|word| *word == "sacrifice")
        {
            let mut mana_symbols = Vec::new();
            let mut tap_cost = false;
            for word in &words[..sacrifice_idx] {
                if *word == "t" {
                    tap_cost = true;
                    continue;
                }
                if let Some(symbol) = parse_token_mana_symbol(word) {
                    mana_symbols.push(symbol);
                }
            }
            builder = builder.with_ability(token_sacrifice_return_named_from_graveyard_ability(
                &card_name,
                mana_symbols,
                tap_cost,
            ));
        }
        if words.windows(5).any(|window| window == ["at", "the", "beginning", "of", "your"])
            && words.contains(&"upkeep")
            && words.contains(&"sacrifice")
            && words.contains(&"this")
            && words.contains(&"token")
            && words.contains(&"return")
            && words.contains(&"named")
            && words.contains(&"graveyard")
            && words.contains(&"battlefield")
            && let Some(card_name) = extract_named_card_name(&words)
        {
            builder = builder.with_ability(
                token_upkeep_sacrifice_return_named_from_graveyard_ability(
                    &card_name,
                    words.contains(&"haste"),
                ),
            );
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"create")
            && words.contains(&"2/2")
            && words.contains(&"red")
            && words.contains(&"dragon")
            && words.contains(&"flying")
            && words.contains(&"r")
            && words.contains(&"+1/+0")
        {
            builder = builder.with_ability(token_dies_create_dragon_with_firebreathing_ability());
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"target")
            && let Some(amount) = parse_deals_damage_amount(&words)
        {
            builder = builder.with_ability(token_dies_deals_damage_any_target_ability(amount));
        }
        if words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"target")
            && words.contains(&"creature")
            && words.contains(&"gets")
            && words.contains(&"-1/-1")
        {
            builder = builder.with_ability(token_dies_target_creature_gets_minus_one_minus_one_ability());
        }
        if words.contains(&"bands")
            && words.contains(&"other")
            && words.contains(&"creatures")
            && words.contains(&"named")
            && words.contains(&"wolves")
        {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::custom(
                "bands_with_other",
                "bands with other creatures named Wolves of the Hunt".to_string(),
            )));
        }
        if words.contains(&"r")
            && words.contains(&"this")
            && words.contains(&"creature")
            && words.contains(&"gets")
            && words.contains(&"+1/+0")
            && !(words.contains(&"when")
                && words.contains(&"token")
                && words.contains(&"dies")
                && words.contains(&"create"))
        {
            builder = builder.with_ability(token_red_pump_ability());
        }
        if words.contains(&"w")
            && words.contains(&"t")
            && words.contains(&"tap")
            && words.contains(&"target")
            && words.contains(&"creature")
        {
            builder = builder.with_ability(token_white_tap_target_creature_ability());
        }
        if words.contains(&"deals")
            && words.contains(&"damage")
            && words.contains(&"player")
            && words.contains(&"poison")
            && words.contains(&"counter")
        {
            builder = builder.with_ability(token_damage_to_player_poison_counter_ability());
        }
        if let Some(amount) = token_inline_noncreature_spell_each_opponent_damage_amount(lower.as_str())
        {
            builder =
                builder.with_ability(token_noncreature_spell_each_opponent_damage_ability(amount));
        }
        if has_word("pest")
            && words.contains(&"when")
            && words.contains(&"token")
            && words.contains(&"dies")
            && words.contains(&"gain")
            && words.contains(&"1")
            && words.contains(&"life")
        {
            let ability = Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::this_dies(),
                    effects: vec![Effect::gain_life(1)],
                    choices: Vec::new(),
                    intervening_if: None,
                    once_each_turn: false,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("When this token dies, you gain 1 life.".to_string()),
            };
            builder = builder.with_ability(ability);
        }
        if words.contains(&"first") && words.contains(&"strike") {
            builder = builder.first_strike();
        }
        if words.contains(&"double") && words.contains(&"strike") {
            builder = builder.double_strike();
        }
        if has_word("mercenary")
            && words.contains(&"creature")
            && words.contains(&"1/1")
            && words.contains(&"red")
        {
            let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
            let ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::free(),
                        vec![Effect::tap_source()],
                    ),
                    effects: vec![Effect::pump(1, 0, target.clone(), Until::EndOfTurn)],
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec!["activate only as a sorcery".to_string()],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(ability);
        }
        if words.contains(&"cant") && words.contains(&"block") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::cant_block()));
        }
        if words.contains(&"can")
            && words.contains(&"block")
            && words.contains(&"only")
            && words.contains(&"creatures")
            && words.contains(&"flying")
        {
            builder =
                builder.with_ability(Ability::static_ability(StaticAbility::can_block_only_flying()));
        }
        if words.contains(&"counter")
            && words.contains(&"noncreature")
            && words.contains(&"spell")
            && words.contains(&"sacrifice")
            && words.contains(&"token")
            && words.contains(&"unless")
            && words.contains(&"controller")
            && words.contains(&"pays")
            && words.contains(&"1")
        {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::spell().without_type(CardType::Creature),
            ));
            let counter_ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
                        vec![Effect::sacrifice_source()],
                    ),
                    effects: vec![Effect::counter_unless_pays(
                        target.clone(),
                        vec![ManaSymbol::Generic(1)],
                    )],
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(
                    "{1}, Sacrifice this token: Counter target noncreature spell unless its controller pays {1}."
                        .to_string(),
                ),
            };
            builder = builder.with_ability(counter_ability);
        }
        if words.contains(&"changeling") {
            builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
        }

        return Some(builder.build());
    }
    None
}

fn parse_token_pt(word: &str) -> Option<(i32, i32)> {
    let (left, right) = word.split_once('/')?;
    let power = left.parse::<i32>().ok()?;
    let toughness = right.parse::<i32>().ok()?;
    Some((power, toughness))
}

fn choose_spec_for_target(target: &TargetAst) -> ChooseSpec {
    match target {
        TargetAst::Source(_) => ChooseSpec::Source,
        TargetAst::AnyTarget(_) => ChooseSpec::AnyTarget,
        TargetAst::PlayerOrPlaneswalker(filter, _) => {
            ChooseSpec::PlayerOrPlaneswalker(filter.clone())
        }
        TargetAst::Spell(_) => ChooseSpec::target_spell(),
        TargetAst::Player(filter, explicit_target_span) => {
            if *filter == PlayerFilter::You {
                ChooseSpec::SourceController
            } else if *filter == PlayerFilter::IteratedPlayer {
                ChooseSpec::Player(filter.clone())
            } else if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Player(filter.clone()))
            } else {
                ChooseSpec::Player(filter.clone())
            }
        }
        TargetAst::Object(filter, explicit_target_span, _) => {
            if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Object(filter.clone()))
            } else {
                ChooseSpec::Object(filter.clone())
            }
        }
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone()),
        TargetAst::WithCount(inner, count) => choose_spec_for_target(inner).with_count(*count),
    }
}

fn target_mentions_graveyard(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => filter.zone == Some(Zone::Graveyard),
        TargetAst::WithCount(inner, _) => target_mentions_graveyard(inner),
        _ => false,
    }
}

fn resolve_target_spec_with_choices(
    target: &TargetAst,
    ctx: &CompileContext,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    let spec = choose_spec_for_target(target);
    let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
    let choices = if spec.is_target() {
        vec![spec.clone()]
    } else {
        Vec::new()
    };
    Ok((spec, choices))
}

fn compile_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &CompileContext,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
    Ok((vec![build(spec)], choices))
}

fn compile_tagged_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &mut CompileContext,
    tag_prefix: &str,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let (spec, choices) = resolve_target_spec_with_choices(target, ctx)?;
    let effect = tag_object_target_effect(build(spec.clone()), &spec, ctx, tag_prefix);
    Ok((vec![effect], choices))
}

fn push_choice(choices: &mut Vec<ChooseSpec>, choice: ChooseSpec) {
    if !choices.iter().any(|existing| existing == &choice) {
        choices.push(choice);
    }
}
