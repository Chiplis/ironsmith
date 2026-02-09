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
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisDealsDamageTo(filter) => Trigger::this_deals_damage_to(filter),
        TriggerSpec::DealsDamage(filter) => Trigger::deals_damage(filter),
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::SpellCast { filter, caster } => Trigger::spell_cast(filter, caster),
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
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::Either(left, right) => {
            Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right))
        }
    }
}

fn compile_statement_effects(effects: &[EffectAst]) -> Result<Vec<Effect>, CardTextError> {
    let mut ctx = CompileContext::new();
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
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::KeywordAction { player, .. } => Some(player.clone()),
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

fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut ctx = CompileContext::new();
    ctx.last_player_filter = trigger.and_then(inferred_trigger_player_filter);
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(effects, tag) {
            if ctx.last_object_tag.is_none() {
                ctx.last_object_tag = Some(tag.to_string());
            }
            prelude.push(Effect::tag_attached_to_source(tag));
        }
    }
    if ctx.last_object_tag.is_none() && effects_reference_it_tag(effects) {
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
        EffectAst::DealDamageEqualToPower { source, target } => {
            matches!(source, TargetAst::Tagged(t, _) if t.as_str() == tag)
                || matches!(target, TargetAst::Tagged(t, _) if t.as_str() == tag)
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
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
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
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
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::UnlessPays { effects, .. } => effects_reference_tag(effects, tag),
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

fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
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
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::CopySpell { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateToken { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. } => matches!(player, PlayerAst::ItsController),
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            effects_reference_its_controller(if_true) || effects_reference_its_controller(if_false)
        }
        EffectAst::MayByPlayer { player, effects } => {
            matches!(player, PlayerAst::ItsController) || effects_reference_its_controller(effects)
        }
        EffectAst::May { effects }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects }
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
        } => {
            matches!(creature1, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
                || matches!(creature2, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            matches!(source, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
                || matches!(target, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
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
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
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
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::DestroyAll { filter }
        | EffectAst::ExileAll { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::SearchLibrary { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        EffectAst::MoveAllCounters { from, to } => {
            matches!(from, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
                || matches!(to, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::PutIntoHand { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::CopySpell { target, .. } => {
            matches!(target, TargetAst::Tagged(t, _) if t.as_str() == IT_TAG)
        }
        EffectAst::CreateTokenCopy { object, .. } => matches!(object, ObjectRefAst::It),
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::UnlessPays { effects, .. } => effects_reference_it_tag(effects),
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
                count: *count,
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
        if next_is_if_result_with_opponent_doesnt {
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

        let (effect_list, effect_choices) = compile_effect(&effects[idx], ctx)?;
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
        LineAst::AlternativeCost { .. }
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
        EffectAst::DealDamageEqualToPower { source, target } => {
            collect_tag_spans_from_target(source, annotations, ctx);
            collect_tag_spans_from_target(target, annotations, ctx);
        }
        EffectAst::DealDamage { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::Exile { target }
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
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponentDoesNot { effects }
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
    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: first_effects,
    } = first
    else {
        return Ok(None);
    };

    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
    } = second
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

    let saved_iterated = ctx.iterated_player;
    let saved_last_effect = ctx.last_effect_id;
    let saved_last_tag = ctx.last_object_tag.clone();
    let saved_last_player = ctx.last_player_filter.clone();
    ctx.iterated_player = true;

    let mut vote_options = Vec::new();
    let mut choices = Vec::new();
    for option in options {
        let option_effects_ast = option_effects.get(option).ok_or_else(|| {
            CardTextError::ParseError(format!("missing effects for vote option '{option}'"))
        })?;
        ctx.last_effect_id = None;
        ctx.last_object_tag = None;
        ctx.last_player_filter = None;
        let (compiled, option_choices) = compile_effects(option_effects_ast, ctx)?;
        for choice in option_choices {
            push_choice(&mut choices, choice);
        }
        vote_options.push(VoteOption::new(option.clone(), compiled));
    }

    ctx.iterated_player = saved_iterated;
    ctx.last_effect_id = saved_last_effect;
    ctx.last_object_tag = saved_last_tag;
    ctx.last_player_filter = saved_last_player;

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
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::deal_damage(amount.clone(), spec.clone()),
                &spec,
                ctx,
                "damaged",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            if let TargetAst::Player(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            let source_spec = resolve_choose_spec_it_tag(&choose_spec_for_target(source), ctx)?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                resolve_choose_spec_it_tag(&choose_spec_for_target(target), ctx)?
            };

            let mut effects = Vec::new();
            let mut choices = Vec::new();
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
                choices.push(source_spec.clone());
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

            if source != target
                && damage_target_spec.is_target()
                && !choices.contains(&damage_target_spec)
            {
                choices.push(damage_target_spec.clone());
            }

            if let TargetAst::Player(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }

            Ok((effects, choices))
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            let spec1 = resolve_choose_spec_it_tag(&choose_spec_for_target(creature1), ctx)?;
            let spec2 = resolve_choose_spec_it_tag(&choose_spec_for_target(creature2), ctx)?;
            let effect = Effect::fight(spec1.clone(), spec2.clone());
            let mut choices = Vec::new();
            if spec1.is_target() {
                choices.push(spec1);
            }
            if spec2.is_target() {
                choices.push(spec2);
            }
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
        } => {
            let mut spec = choose_spec_for_target(target);
            spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, count.clone(), spec.clone());
            if let Some(target_count) = target_count {
                put_counters = put_counters.with_target_count(*target_count);
            }
            let effect =
                tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
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
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::TapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::tap_all(resolved_filter));
            Ok((prelude, choices))
        }
        EffectAst::Untap { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::TapOrUntap { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
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
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
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
            let spec = choose_spec_for_target(target);
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
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = Effect::new(crate::effects::EarthbendEffect::new(
                spec.clone(),
                *counters,
            ));
            Ok((vec![effect], vec![spec]))
        }
        EffectAst::Draw { count, player } => match player {
            PlayerAst::Target => {
                let effect = Effect::target_draws(count.clone(), PlayerFilter::target_player());
                ctx.last_player_filter = Some(PlayerFilter::target_player());
                Ok((vec![effect], vec![ChooseSpec::target_player()]))
            }
            _ => {
                let filter = resolve_non_target_player_filter(*player, ctx)?;
                let effect = if matches!(&filter, PlayerFilter::You) {
                    Effect::draw(count.clone())
                } else {
                    Effect::target_draws(count.clone(), filter.clone())
                };
                if !matches!(*player, PlayerAst::Implicit) {
                    ctx.last_player_filter = Some(filter);
                }
                Ok((vec![effect], Vec::new()))
            }
        },
        EffectAst::Counter { target } => {
            let spec = choose_spec_for_target(target);
            let effect =
                tag_object_target_effect(Effect::counter(spec.clone()), &spec, ctx, "countered");
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::CounterUnlessPays { target, mana } => {
            let spec = choose_spec_for_target(target);
            let effect = tag_object_target_effect(
                Effect::counter_unless_pays(spec.clone(), mana.clone()),
                &spec,
                ctx,
                "countered",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::LoseLife { amount, player } => match player {
            PlayerAst::Target => Ok((vec![Effect::lose_life_target(amount.clone())], {
                ctx.last_player_filter = Some(PlayerFilter::target_player());
                vec![ChooseSpec::target_player()]
            })),
            PlayerAst::TargetOpponent => Ok((
                vec![Effect::lose_life_player(
                    amount.clone(),
                    PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
                )],
                {
                    let filter = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
                    ctx.last_player_filter = Some(filter.clone());
                    vec![ChooseSpec::target(ChooseSpec::Player(
                        PlayerFilter::Opponent,
                    ))]
                },
            )),
            _ => {
                let filter = resolve_non_target_player_filter(*player, ctx)?;
                let effect = if matches!(&filter, PlayerFilter::You) {
                    Effect::lose_life(amount.clone())
                } else {
                    Effect::lose_life_player(amount.clone(), filter.clone())
                };
                if !matches!(*player, PlayerAst::Implicit) {
                    ctx.last_player_filter = Some(filter);
                }
                Ok((vec![effect], Vec::new()))
            }
        },
        EffectAst::GainLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, ctx)?;
            match player {
                PlayerAst::Target => Ok((vec![Effect::gain_life_target(amount.clone())], {
                    ctx.last_player_filter = Some(PlayerFilter::target_player());
                    vec![ChooseSpec::target_player()]
                })),
                PlayerAst::TargetOpponent => Ok((
                    vec![Effect::gain_life_player(
                        amount.clone(),
                        ChooseSpec::Player(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
                    )],
                    {
                        let filter = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
                        ctx.last_player_filter = Some(filter.clone());
                        vec![ChooseSpec::target(ChooseSpec::Player(
                            PlayerFilter::Opponent,
                        ))]
                    },
                )),
                _ => {
                    let filter = resolve_non_target_player_filter(*player, ctx)?;
                    let effect = if matches!(&filter, PlayerFilter::You) {
                        Effect::gain_life(amount.clone())
                    } else {
                        Effect::gain_life_player(amount.clone(), ChooseSpec::Player(filter.clone()))
                    };
                    if !matches!(*player, PlayerAst::Implicit) {
                        ctx.last_player_filter = Some(filter);
                    }
                    Ok((vec![effect], Vec::new()))
                }
            }
        }
        EffectAst::LoseGame { player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::lose_the_game()
            } else {
                Effect::lose_the_game_player(filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PreventAllCombatDamage { duration } => Ok((
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        )),
        EffectAst::PreventAllCombatDamageFromSource { duration, source } => {
            let spec = choose_spec_for_target(source);
            let effect = Effect::prevent_all_combat_damage_from(spec.clone(), duration.clone());
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
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
            let spec = choose_spec_for_target(target);
            let effect = Effect::prevent_damage(amount, spec.clone(), duration.clone());
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::AddMana { mana, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana(mana.clone())
            } else {
                Effect::add_mana_player(mana.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaScaled {
            mana,
            amount,
            player,
        } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                mana.clone(),
                amount.clone(),
                filter.clone(),
            ));
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors,
        } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if let Some(colors) = available_colors.clone() {
                if matches!(&filter, PlayerFilter::You) {
                    Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                } else {
                    Effect::add_mana_of_any_color_restricted_player(
                        amount.clone(),
                        filter.clone(),
                        colors,
                    )
                }
            } else if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_of_any_color(amount.clone())
            } else {
                Effect::add_mana_of_any_color_player(amount.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaAnyOneColor { amount, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_of_any_one_color(amount.clone())
            } else {
                Effect::add_mana_of_any_one_color_player(amount.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            player,
            land_filter,
            allow_colorless,
            same_type,
        } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::add_mana_of_land_produced_types_player(
                amount.clone(),
                filter.clone(),
                land_filter.clone(),
                *allow_colorless,
                *same_type,
            );
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaCommanderIdentity { amount, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::add_mana_from_commander_color_identity(amount.clone())
            } else {
                Effect::add_mana_from_commander_color_identity_player(
                    amount.clone(),
                    filter.clone(),
                )
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::AddManaImprintedColors => Ok((
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        )),
        EffectAst::Scry { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::scry(count.clone())
            } else {
                Effect::scry_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::Surveil { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::surveil(count.clone())
            } else {
                Effect::surveil_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::PayMana { cost, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = Effect::new(crate::effects::PayManaEffect::new(
                cost.clone(),
                ChooseSpec::Player(filter.clone()),
            ));
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
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
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let mut effect = Effect::gain_control_with_duration(spec.clone(), duration.clone());
            let mut choices = Vec::new();
            if spec.is_target() {
                let tag = ctx.next_tag("controlled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
                choices.push(spec);
            }
            Ok((vec![effect], choices))
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
            let (player_filter, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(player_filter.clone()),
                vec![Effect::extra_turn_player(player_filter.clone())],
                true,
                Vec::new(),
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
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                PlayerAst::TargetOpponent => (
                    PlayerFilter::target_opponent(),
                    vec![ChooseSpec::target(ChooseSpec::Player(
                        PlayerFilter::Opponent,
                    ))],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let effect =
                Effect::choose_objects(resolved_filter, *count, chooser.clone(), tag.clone());
            ctx.last_object_tag = Some(tag.as_str().to_string());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Sacrifice {
            filter,
            player,
            count,
        } => {
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                PlayerAst::TargetOpponent => (
                    PlayerFilter::target_opponent(),
                    vec![ChooseSpec::target(ChooseSpec::Player(
                        PlayerFilter::Opponent,
                    ))],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
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
                if !matches!(*player, PlayerAst::Implicit) {
                    ctx.last_player_filter = Some(chooser);
                }
                return Ok((vec![Effect::sacrifice_source()], choices));
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
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![choose, sacrifice], choices))
        }
        EffectAst::SacrificeAll { filter, player } => {
            let (chooser, choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                _ => (resolve_non_target_player_filter(*player, ctx)?, Vec::new()),
            };
            let mut resolved_filter = resolve_it_tag(filter, ctx)?;
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(chooser);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DiscardHand { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::discard_hand()
            } else {
                Effect::discard_hand_player(filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Discard {
            count,
            player,
            random,
        } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::discard_player(count.clone(), PlayerFilter::You, *random)
            } else {
                Effect::discard_player(count.clone(), filter.clone(), *random)
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnToHand { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec.clone())),
                &spec,
                ctx,
                "returned",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnToBattlefield { target, tapped } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let from_exile_tag = matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == "exiled_0"
            );
            let effect = tag_object_target_effect(
                if from_exile_tag && !*tapped {
                    // Blink-style "exile ... then return it" should move the tagged
                    // exiled object back to the battlefield from exile.
                    Effect::move_to_zone(spec.clone(), Zone::Battlefield, false)
                } else {
                    Effect::return_from_graveyard_to_battlefield(spec.clone(), *tapped)
                },
                &spec,
                ctx,
                "returned",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec.clone());
            }
            Ok((vec![effect], choices))
        }
        EffectAst::MoveToZone {
            target,
            zone,
            to_top,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::move_to_zone(spec.clone(), *zone, *to_top),
                &spec,
                ctx,
                "moved",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::ReturnAllToHand { filter } => {
            Ok((vec![Effect::return_all_to_hand(filter.clone())], Vec::new()))
        }
        EffectAst::ExchangeControl { filter, count } => {
            let first = ChooseSpec::Object(filter.clone());
            let second = ChooseSpec::Object(filter.clone());
            let effect = Effect::exchange_control(first, second);
            let target_spec = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
                .with_count(ChoiceCount::exactly(*count as usize));
            Ok((vec![effect], vec![target_spec]))
        }
        EffectAst::SetLifeTotal { amount, player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::set_life_total_player(amount.clone(), filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::SkipTurn { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::skip_turn_player(filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::SkipDrawStep { player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = Effect::skip_draw_step_player(filter.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Regenerate { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Mill { count, player } => {
            let (filter, choices) = resolve_player_filter_with_target(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::mill(count.clone())
            } else {
                Effect::mill_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::PoisonCounters { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::poison_counters(count.clone())
            } else {
                Effect::poison_counters_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::EnergyCounters { count, player } => {
            let filter = resolve_non_target_player_filter(*player, ctx)?;
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::energy_counters(count.clone())
            } else {
                Effect::energy_counters_player(count.clone(), filter.clone())
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(filter);
            }
            Ok((vec![effect], Vec::new()))
        }
        EffectAst::May { effects } => {
            let saved_last_effect = ctx.last_effect_id;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
            let effect = Effect::may(inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::MayByPlayer { player, effects } => {
            let saved_last_effect = ctx.last_effect_id;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
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
            let saved_last_effect = ctx.last_effect_id;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
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
            let saved_last_effect = ctx.last_effect_id;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.last_effect_id = saved_last_effect;
            let predicate = match predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
            };
            let effect = Effect::if_then(condition, predicate, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponent { effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_each_opponent(inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachPlayer { effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_players(PlayerFilter::Any, inner_effects);
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

            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            ctx.last_object_tag = Some(effective_tag.clone());
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_each_tagged(effective_tag, inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let saved_iterated = ctx.iterated_player;
            let saved_last_effect = ctx.last_effect_id;
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_last_player = ctx.last_player_filter.clone();
            ctx.iterated_player = true;
            ctx.last_effect_id = None;
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            ctx.iterated_player = saved_iterated;
            ctx.last_effect_id = saved_last_effect;
            ctx.last_object_tag = saved_last_tag;
            ctx.last_player_filter = saved_last_player;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            Ok((vec![effect], inner_choices))
        }
        EffectAst::ForEachOpponentDoesNot { .. } => Err(CardTextError::ParseError(
            "for each opponent who doesn't must follow an opponent clause".to_string(),
        )),
        EffectAst::Destroy { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let mut effect = Effect::destroy(spec.clone());
            if spec.is_target() {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::DestroyAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::destroy_all(filter.clone()));
            Ok((prelude, choices))
        }
        EffectAst::ExileAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::exile_all(filter.clone()));
            Ok((prelude, choices))
        }
        EffectAst::Exile { target } => {
            if let Some(compiled) = lower_hand_exile_target(target, ctx)? {
                return Ok(compiled);
            }
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
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
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::LookAtHand { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = Effect::new(crate::effects::LookAtHandEffect::new(spec.clone()));
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            if let TargetAst::Player(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((vec![effect], choices))
        }
        EffectAst::TargetOnly { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::TargetOnlyEffect::new(spec.clone())),
                &spec,
                ctx,
                "targeted",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::RevealTop { player } => {
            let mut choices = Vec::new();
            let player_filter = match player {
                PlayerAst::Target => {
                    choices.push(ChooseSpec::target_player());
                    PlayerFilter::target_player()
                }
                _ => resolve_non_target_player_filter(*player, ctx)?,
            };
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::reveal_top(player_filter, tag);
            Ok((vec![effect], choices))
        }
        EffectAst::RevealHand { player } => {
            let mut choices = Vec::new();
            let spec = match player {
                PlayerAst::Target => {
                    choices.push(ChooseSpec::target_player());
                    ChooseSpec::target_player()
                }
                _ => {
                    let filter = resolve_non_target_player_filter(*player, ctx)?;
                    ChooseSpec::Player(filter)
                }
            };
            if !matches!(*player, PlayerAst::Implicit) {
                if let ChooseSpec::Player(filter) = &spec {
                    ctx.last_player_filter = Some(filter.clone());
                } else if matches!(*player, PlayerAst::Target) {
                    ctx.last_player_filter = Some(PlayerFilter::target_player());
                }
            }
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
            let mut choices = Vec::new();
            if matches!(player, PlayerAst::Target) {
                choices.push(ChooseSpec::target_player());
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else if !matches!(*player, PlayerAst::Implicit) {
                let resolved = resolve_non_target_player_filter(*player, ctx)?;
                ctx.last_player_filter = Some(resolved);
            }
            let effect = Effect::move_to_zone(ChooseSpec::tagged(tag), Zone::Hand, false);
            Ok((vec![effect], choices))
        }
        EffectAst::CopySpell {
            target,
            count,
            player,
            may_choose_new_targets,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
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
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
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
                PredicateAst::SourceIsTapped => Condition::SourceIsTapped,
                PredicateAst::NoSpellsWereCastLastTurn => Condition::NoSpellsWereCastLastTurn,
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
            let player_filter = if matches!(*player, PlayerAst::Implicit) {
                PlayerFilter::You
            } else {
                resolve_non_target_player_filter(*player, ctx)?
            };
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, *count)
            } else {
                crate::effects::CreateTokenEffect::new(token, *count, player_filter.clone())
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
            let player_filter = if matches!(*player, PlayerAst::Implicit) {
                PlayerFilter::You
            } else {
                resolve_non_target_player_filter(*player, ctx)?
            };
            let effect = if matches!(player_filter, PlayerFilter::You) {
                Effect::create_tokens(token, *count)
            } else {
                Effect::create_tokens_player(token, *count, player_filter)
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
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        EffectAst::CreateTokenCopyFromSource {
            source,
            count,
            player,
            half_power_toughness_round_up,
            has_haste,
            sacrifice_at_next_end_step,
        } => {
            let source_spec = choose_spec_for_target(source);
            let source_spec = resolve_choose_spec_it_tag(&source_spec, ctx)?;
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                source_spec.clone(),
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
            let mut choices = Vec::new();
            if source_spec.is_target() {
                choices.push(source_spec);
            }
            Ok((vec![Effect::new(effect)], choices))
        }
        EffectAst::ExileThatTokenAtEndOfCombat => Ok((Vec::new(), Vec::new())),
        EffectAst::TokenCopyGainHasteUntilEot | EffectAst::TokenCopySacrificeAtNextEndStep => {
            Ok((Vec::new(), Vec::new()))
        }
        EffectAst::Monstrosity { amount } => {
            Ok((vec![Effect::monstrosity(amount.clone())], Vec::new()))
        }
        EffectAst::RemoveUpToAnyCounters { amount, target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let effect = tag_object_target_effect(
                Effect::with_id(
                    id.0,
                    Effect::remove_up_to_any_counters(amount.clone(), spec.clone()),
                ),
                &spec,
                ctx,
                "counters",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::MoveAllCounters { from, to } => {
            let from_spec = choose_spec_for_target(from);
            let from_spec = resolve_choose_spec_it_tag(&from_spec, ctx)?;
            let to_spec = choose_spec_for_target(to);
            let to_spec = resolve_choose_spec_it_tag(&to_spec, ctx)?;
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
            let mut choices = Vec::new();
            if from_spec.is_target() {
                choices.push(from_spec);
            }
            if to_spec.is_target() {
                choices.push(to_spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::pump(
                    power.clone(),
                    toughness.clone(),
                    spec.clone(),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "pumped",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::set_base_power_toughness(
                    power.clone(),
                    toughness.clone(),
                    spec.clone(),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "set_base_pt",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count_filter,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let resolved_count_filter = resolve_it_tag(count_filter, ctx)?;
            let effect = tag_object_target_effect(
                Effect::pump_for_each(
                    spec.clone(),
                    *power_per,
                    *toughness_per,
                    Value::Count(resolved_count_filter),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "pumped",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, ctx)?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::pump_all(
                resolved_filter.clone(),
                power.clone(),
                toughness.clone(),
                duration.clone(),
            )
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
            let spec = choose_spec_for_target(target);
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            let effect = tag_object_target_effect(
                Effect::pump(
                    power_value,
                    Value::Fixed(*toughness),
                    spec.clone(),
                    duration.clone(),
                ),
                &spec,
                ctx,
                "pumped",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => Ok((
            vec![Effect::grant_abilities_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            )],
            Vec::new(),
        )),
        EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::GrantAbilitiesTargetEffect::new(
                    spec.clone(),
                    abilities.clone(),
                    duration.clone(),
                )),
                &spec,
                ctx,
                "granted",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
        }
        EffectAst::Transform { target } => {
            let spec = choose_spec_for_target(target);
            let spec = resolve_choose_spec_it_tag(&spec, ctx)?;
            let effect = tag_object_target_effect(
                Effect::transform(spec.clone()),
                &spec,
                ctx,
                "transformed",
            );
            let mut choices = Vec::new();
            if spec.is_target() {
                choices.push(spec);
            }
            Ok((vec![effect], choices))
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
            if filter.owner.is_none() {
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
        EffectAst::ShuffleLibrary { player } => {
            let player_filter = resolve_non_target_player_filter(*player, ctx)?;
            Ok((
                vec![Effect::shuffle_library_player(player_filter)],
                Vec::new(),
            ))
        }
        EffectAst::VoteStart { .. }
        | EffectAst::VoteOption { .. }
        | EffectAst::VoteExtra { .. } => Err(CardTextError::ParseError(
            "vote clauses must appear together".to_string(),
        )),
    }
}

fn resolve_non_target_player_filter(
    player: PlayerAst,
    ctx: &CompileContext,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
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
        PlayerAst::Implicit => {
            if ctx.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else {
                Ok(PlayerFilter::You)
            }
        }
    }
}

fn resolve_player_filter_with_target(
    player: PlayerAst,
    ctx: &CompileContext,
) -> Result<(PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    match player {
        PlayerAst::Target => Ok((
            PlayerFilter::target_player(),
            vec![ChooseSpec::target_player()],
        )),
        PlayerAst::TargetOpponent => Ok((
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
            vec![ChooseSpec::target(ChooseSpec::Player(
                PlayerFilter::Opponent,
            ))],
        )),
        _ => Ok((resolve_non_target_player_filter(player, ctx)?, Vec::new())),
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
        ChooseSpec::Object(filter) => Ok(ChooseSpec::Object(resolve_it_tag(filter, ctx)?)),
        ChooseSpec::Target(inner) => Ok(ChooseSpec::Target(Box::new(resolve_choose_spec_it_tag(
            inner, ctx,
        )?))),
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag(inner, ctx)?),
            count.clone(),
        )),
        ChooseSpec::All(filter) => Ok(ChooseSpec::All(resolve_it_tag(filter, ctx)?)),
        ChooseSpec::Player(filter) => Ok(ChooseSpec::Player(filter.clone())),
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
        Value::PowerOf(spec) => Ok(Value::PowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, ctx,
        )?))),
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

    if has_word("treasure") {
        return Some(crate::cards::tokens::treasure_token_definition());
    }
    if has_word("clue") {
        return Some(crate::cards::tokens::clue_token_definition());
    }
    if has_word("eldrazi") && has_word("spawn") {
        return Some(eldrazi_spawn_token_definition());
    }
    if has_word("eldrazi") && has_word("scion") {
        return Some(eldrazi_scion_token_definition());
    }
    if has_word("food") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Food")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food]);
        return Some(builder.build());
    }
    if has_word("blood") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Blood")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("powerstone") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Powerstone")
            .token()
            .card_types(vec![CardType::Artifact]);
        return Some(builder.build());
    }
    if has_word("angel") {
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
    if has_word("elephant") && lower.contains("3/3") && lower.contains("green") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Elephant")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3));
        return Some(builder.build());
    }
    if has_word("construct") {
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
    if has_word("vampire") && lower.contains("1/1") && lower.contains("white") {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Vampire")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Vampire])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(1, 1));
        if lower.contains("lifelink") {
            builder = builder.lifelink();
        }
        return Some(builder.build());
    }
    if has_word("human") && lower.contains("1/1") && lower.contains("white") {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Human")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(1, 1));
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
        if words.contains(&"first") && words.contains(&"strike") {
            builder = builder.first_strike();
        }
        if words.contains(&"double") && words.contains(&"strike") {
            builder = builder.double_strike();
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
        TargetAst::Spell(_) => ChooseSpec::target_spell(),
        TargetAst::Player(filter, _) => {
            if *filter == PlayerFilter::You {
                ChooseSpec::SourceController
            } else if *filter == PlayerFilter::IteratedPlayer {
                ChooseSpec::Player(filter.clone())
            } else {
                ChooseSpec::target(ChooseSpec::Player(filter.clone()))
            }
        }
        TargetAst::Object(filter, _, _) => ChooseSpec::target(ChooseSpec::Object(filter.clone())),
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone()),
        TargetAst::WithCount(inner, count) => choose_spec_for_target(inner).with_count(*count),
    }
}

fn push_choice(choices: &mut Vec<ChooseSpec>, choice: ChooseSpec) {
    if !choices.iter().any(|existing| existing == &choice) {
        choices.push(choice);
    }
}
