use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, GiftTimingAst, KeywordAction, LineInfo,
    ParseAnnotations, PlayerAst, StaticAbilityAst, SubjectVerbEffectAst, TriggerSpec,
};
use crate::runtime_backend::activation_and_restrictions::last_created_token_info;
use crate::runtime_backend::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut,
};
use crate::runtime_backend::shared_types::{
    LineSemanticFacts, StatementConditionIntro, StatementLineSemanticFacts,
    StatementReplacementSurfaceKind,
};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::super::effect_pipeline::{
    NormalizedLineChunk, NormalizedParsedAbility, NormalizedPreparedAbility,
};
use super::*;

struct LineChunkLoweringInput<'a> {
    builder: CardDefinitionBuilder,
    state: &'a mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &'a LineInfo,
    semantic_facts: &'a LineSemanticFacts,
    allow_unsupported: bool,
    annotations: &'a mut ParseAnnotations,
}

fn conditional_self_replacement_followup(
    effect: &crate::effect::Effect,
) -> Option<crate::effects::ConditionalEffect> {
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Some(conditional.clone());
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| conditional_self_replacement_followup(&tagged.effect))
}

fn materialized_self_replacement_followup(
    program: &crate::resolution::ResolutionProgram,
) -> Option<crate::resolution::SelfReplacementBranch> {
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    if !segment.default_effects.is_empty() || segment.self_replacements.len() != 1 {
        return None;
    }
    Some(
        segment.self_replacements[0]
            .clone()
            .with_starts_new_source_line(segment.starts_new_source_line),
    )
}

fn source_tokens_prove_repeatable_instant_timing_until_end_of_turn(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words: Vec<String> = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect();
    words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
        && words
            .windows(4)
            .any(|window| window == ["that", "permanent", "or", "player"])
        && words
            .windows(8)
            .any(|window| window == ["any", "time", "you", "could", "cast", "an", "instant", "if"])
}

fn is_exact_permanent_or_player_object_filter(filter: &ObjectFilter) -> bool {
    if filter == &ObjectFilter::permanent() {
        return true;
    }

    // Explicit `permanent or player` target parsing retains both the
    // battlefield domain and the complete permanent card-type list. This is
    // semantically the same target domain as the older zone-only shape, but
    // keep the fusion strict by accepting only that exact explicit form.
    if filter.zone != Some(Zone::Battlefield)
        || filter.card_types != ObjectFilter::permanent_card().card_types
    {
        return false;
    }
    let mut remainder = filter.clone();
    remainder.zone = None;
    remainder.card_types.clear();
    remainder == ObjectFilter::default()
}

/// Fuse the fully typed three-part surface
///
/// `prevent ... to any target; until EOT may pay at instant timing; if paid,
/// prevent ... to that target`
///
/// into a persistent repeatable special-action grant. The lexical guard owns
/// only duration/timing provenance; every executable detail is proven from the
/// lowered effect tree.
fn fuse_repeatable_mana_payment_prevention_until_end_of_turn(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    if !source_tokens_prove_repeatable_instant_timing_until_end_of_turn(source_tokens) {
        return false;
    }
    let [initial_segment, payment_segment, followup_segment] = program.segments.as_slice() else {
        return false;
    };
    if [initial_segment, payment_segment, followup_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return false;
    }
    let initial_effect = match initial_segment.default_effects.as_slice() {
        [initial_effect] => initial_effect,
        [target_only, initial_effect] => {
            let Some(target_only) = target_only.downcast_ref::<crate::effects::TargetOnlyEffect>()
            else {
                return false;
            };
            if !matches!(target_only.target.unhinted(), ChooseSpec::AnyTarget)
                || target_only.chooser.is_some()
                || target_only.explicit_declaration
            {
                return false;
            }
            initial_effect
        }
        _ => return false,
    };
    let Some(initial_prevention) =
        initial_effect.downcast_ref::<crate::effects::PreventDamageEffect>()
    else {
        return false;
    };
    if !matches!(initial_prevention.target.unhinted(), ChooseSpec::AnyTarget)
        || initial_prevention.until != crate::effect::Until::EndOfTurn
        || !initial_prevention.follow_up_effects.is_empty()
        || initial_prevention.source_of_your_choice
        || initial_prevention.protect_you_and_permanents_you_control
    {
        return false;
    }

    let [payment_effect] = payment_segment.default_effects.as_slice() else {
        return false;
    };
    let Some(with_id) = payment_effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    let Some(may) = with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return false;
    };
    let [payment] = may.effects.as_slice() else {
        return false;
    };
    let Some(pay_mana) = payment.downcast_ref::<crate::effects::PayManaEffect>() else {
        return false;
    };
    if may.decider != Some(PlayerFilter::You)
        || !matches!(
            pay_mana.player.unhinted(),
            ChooseSpec::Player(PlayerFilter::You)
        )
        || pay_mana.x_value.is_some()
        || pay_mana.x_maximum.is_some()
    {
        return false;
    }

    let [followup_effect] = followup_segment.default_effects.as_slice() else {
        return false;
    };
    let Some(if_effect) = followup_effect.downcast_ref::<crate::effects::IfEffect>() else {
        return false;
    };
    if if_effect.condition != with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return false;
    }
    let [prevention_effect] = if_effect.then.as_slice() else {
        return false;
    };
    let prevention_effect = prevention_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(prevention_effect, |tagged| tagged.effect.as_ref());
    let Some(prevention) = prevention_effect.downcast_ref::<crate::effects::PreventDamageEffect>()
    else {
        return false;
    };
    if prevention.amount.unhinted() != &crate::effect::Value::Fixed(1)
        || prevention.until != crate::effect::Until::EndOfTurn
        || !matches!(
            prevention.target.unhinted(),
            ChooseSpec::ObjectOrPlayer(object, PlayerFilter::Any)
                if is_exact_permanent_or_player_object_filter(object)
        )
        || !prevention.follow_up_effects.is_empty()
        || prevention.source_of_your_choice
        || prevention.protect_you_and_permanents_you_control
    {
        return false;
    }

    let mut same_target_prevention = prevention.clone();
    same_target_prevention.target = ChooseSpec::AnyTarget;
    let grant = crate::effect::Effect::new(
        crate::effects::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect::new(
            PlayerFilter::You,
            pay_mana.cost.clone(),
            vec![crate::effect::Effect::new(same_target_prevention)],
        ),
    );
    let starts_new_source_line = payment_segment.starts_new_source_line;
    program.segments[1] = crate::resolution::ResolutionSegment {
        default_effects: vec![grant],
        self_replacements: Vec::new(),
        starts_new_source_line,
    };
    program.segments.truncate(2);
    *program = crate::resolution::ResolutionProgram::new(program.segments.clone());
    true
}

fn same_library_search_quality(left: &ObjectFilter, right: &ObjectFilter) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    // A search already identifies the searched library through its player
    // fields. Depending on which sentence shape produced the filter, the same
    // authored quality may redundantly retain that library's zone/owner on one
    // side but not the other.
    for filter in [&mut left, &mut right] {
        filter.zone = None;
        filter.owner = None;
        filter.controller = None;
    }
    left == right
}

fn unique_nested_library_search_choice(
    effects: &[crate::effect::Effect],
) -> Option<crate::effects::ChooseObjectsEffect> {
    fn collect(
        effect: &crate::effect::Effect,
        found: &mut Vec<crate::effects::ChooseObjectsEffect>,
    ) {
        if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && choice.is_search
            && choice.zone == Some(Zone::Library)
        {
            found.push(choice.clone());
        }
        effect.visit_child_effects(&mut |child| collect(child, found));
    }

    let mut found = Vec::new();
    for effect in effects {
        collect(effect, &mut found);
    }
    match found.as_slice() {
        [choice] => Some(choice.clone()),
        _ => None,
    }
}

fn rewrite_library_search_count_effect(
    effect: &crate::effect::Effect,
    replacement: &crate::effects::ChooseObjectsEffect,
) -> (crate::effect::Effect, usize) {
    if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choice.is_search
        && choice.zone == Some(Zone::Library)
        && same_library_search_quality(&choice.filter, &replacement.filter)
    {
        let mut rewritten = choice.clone();
        rewritten.count = replacement.count;
        rewritten.count_value = replacement.count_value.clone();
        return (crate::effect::Effect::new(rewritten), 1);
    }

    if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()
        && same_library_search_quality(&search.filter, &replacement.filter)
        && !replacement.count.dynamic_x
        && replacement.count_value.is_none()
        && let Some(count) = replacement.count.max
        && count > 0
    {
        let optional = replacement.count.min == 0
            || matches!(
                search.search_mode,
                crate::effect::SearchSelectionMode::Optional
            )
            || search.filter.has_search_stated_quality();
        let slots = (0..count)
            .map(|_| {
                if optional {
                    crate::effects::SearchLibrarySlot::optional(search.filter.clone())
                } else {
                    crate::effects::SearchLibrarySlot::required(search.filter.clone())
                }
            })
            .collect();
        let rewritten = crate::effects::SearchLibrarySlotsEffect::new(
            slots,
            search.destination,
            search.chooser.clone(),
            search.player.clone(),
            search.reveal,
            replacement.tag.clone(),
        );
        return (crate::effect::Effect::new(rewritten), 1);
    }

    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let (child, replacements) =
            rewrite_library_search_count_effect(&tagged.effect, replacement);
        if replacements > 0 {
            return (
                crate::effect::Effect::new(crate::effects::TaggedEffect::new(
                    tagged.tag.clone(),
                    child,
                )),
                replacements,
            );
        }
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let (child, replacements) =
            rewrite_library_search_count_effect(&with_id.effect, replacement);
        if replacements > 0 {
            return (
                crate::effect::Effect::with_id(with_id.id.0, child),
                replacements,
            );
        }
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut rewritten = sequence.clone();
        let mut replacements = 0;
        for child in &mut rewritten.effects {
            let (new_child, child_replacements) =
                rewrite_library_search_count_effect(child, replacement);
            *child = new_child;
            replacements += child_replacements;
        }
        if replacements > 0 {
            return (crate::effect::Effect::new(rewritten), replacements);
        }
    }

    (effect.clone(), 0)
}

/// A count-only search follow-up can be authored on a later source line than
/// the complete search procedure it modifies (Nissa's Pilgrimage), or can
/// upgrade an encapsulated one-card search to several cards (Reclaim the
/// Wastes). Attach the branch to the matching search segment and materialize
/// the complete procedure, rather than replacing an unrelated trailing
/// shuffle or leaving the selected cards stranded in the library.
fn attach_materialized_library_search_count_override(
    existing: &mut crate::resolution::ResolutionProgram,
    replacement: &crate::resolution::SelfReplacementBranch,
) -> bool {
    let Some(replacement_choice) =
        unique_nested_library_search_choice(&replacement.replacement_effects)
    else {
        return false;
    };

    for segment in existing.segments.iter_mut().rev() {
        let mut rewritten = Vec::with_capacity(segment.default_effects.len());
        let mut replacements = 0;
        for effect in &segment.default_effects {
            let (effect, count) = rewrite_library_search_count_effect(effect, &replacement_choice);
            rewritten.push(effect);
            replacements += count;
        }
        if replacements == 1 {
            let mut branch = replacement.clone();
            branch.replacement_effects = rewritten;
            segment.self_replacements.push(branch);
            return true;
        }
    }
    false
}

/// Materialize a count-only branch that was already assembled into the same
/// resolution segment as its default search. This is the single-line sibling
/// of `attach_materialized_library_search_count_override`: front-end bundles
/// such as Reclaim the Wastes are complete programs before line lowering and
/// intentionally bypass the later cross-line follow-up machinery.
fn materialize_attached_library_search_count_overrides(
    program: &mut crate::resolution::ResolutionProgram,
) {
    for segment in &mut program.segments {
        let default_effects = segment.default_effects.clone();
        for branch in &mut segment.self_replacements {
            let [replacement_effect] = branch.replacement_effects.as_slice() else {
                continue;
            };
            let Some(replacement_choice) =
                replacement_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            else {
                continue;
            };
            if !replacement_choice.is_search || replacement_choice.zone != Some(Zone::Library) {
                continue;
            }

            let mut rewritten = Vec::with_capacity(default_effects.len());
            let mut replacements = 0;
            for effect in &default_effects {
                let (effect, count) =
                    rewrite_library_search_count_effect(effect, replacement_choice);
                rewritten.push(effect);
                replacements += count;
            }
            if replacements == 1 {
                branch.replacement_effects = rewritten;
            }
        }
    }
}

/// Fold an authored prior-result `instead` clause into the successful arm of
/// the action it qualifies.
///
/// A sequence such as "You may discard ... If you do, draw one. If a
/// creature card was discarded this way, draw two instead" initially lowers
/// as three sequential segments. Leaving the last two segments sequential
/// both executes the base and replacement effects and loses the outer `may`
/// gate. Nesting the result test inside the successful prior-action arm keeps
/// decline, default, and replacement paths mutually exclusive.
fn fold_prior_result_self_replacement_into_success_arm(
    program: &mut crate::resolution::ResolutionProgram,
    statement_facts: &StatementLineSemanticFacts,
) -> bool {
    if !matches!(
        statement_facts.instead_followup.semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) || program.segments.len() < 2
    {
        return false;
    }

    let segment_count = program.segments.len();
    let replacement_effect = {
        let base_segment = &program.segments[segment_count - 2];
        let replacement_segment = &program.segments[segment_count - 1];
        if !base_segment.self_replacements.is_empty()
            || !replacement_segment.self_replacements.is_empty()
            || replacement_segment.starts_new_source_line
        {
            return false;
        }
        let [base_effect] = base_segment.default_effects.as_slice() else {
            return false;
        };
        let [replacement_effect] = replacement_segment.default_effects.as_slice() else {
            return false;
        };
        let Some(mut success_gate) = base_effect
            .downcast_ref::<crate::effects::IfEffect>()
            .cloned()
        else {
            return false;
        };
        let Some(result_gate) = replacement_effect
            .downcast_ref::<crate::effects::IfEffect>()
            .cloned()
        else {
            return false;
        };
        if success_gate.condition != result_gate.condition
            || success_gate.predicate != crate::effect::EffectPredicate::Happened
            || !success_gate.else_.is_empty()
            || success_gate.then.is_empty()
            || !matches!(
                result_gate.predicate,
                crate::effect::EffectPredicate::PriorEffectResult(_)
            )
            || result_gate.then.is_empty()
            || !result_gate.else_.is_empty()
        {
            return false;
        }

        let default_effects = std::mem::take(&mut success_gate.then);
        success_gate.then = vec![crate::effect::Effect::new(
            crate::effects::IfEffect::new(
                result_gate.condition,
                result_gate.predicate,
                result_gate.then,
                default_effects,
            )
            .with_prior_result_replacement_surface(true),
        )];
        crate::effect::Effect::new(success_gate)
    };

    program.segments[segment_count - 2].default_effects = vec![replacement_effect];
    program.segments.pop();
    let segments = std::mem::take(&mut program.segments);
    *program = crate::resolution::ResolutionProgram::new(segments);
    true
}

fn retarget_replacement_effects(
    effects: Vec<crate::effect::Effect>,
    previous_target: &ChooseSpec,
) -> Vec<crate::effect::Effect> {
    // The parser may synthesize a fresh target declaration alongside an
    // ExecuteWithSource wrapper for the same ambiguous “it deals ... instead”
    // clause. Both are artifacts in an amount replacement: reuse the default
    // effect's target and keep its source.
    if let [target_effect, with_source_effect] = effects.as_slice()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(with_source) =
            with_source_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && effect_target_uses_it_reference(&with_source.source)
        && let Some(replacement_damage) = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && replacement_damage.target == target_only.target
    {
        let mut replacement_damage = replacement_damage.clone();
        replacement_damage.target = previous_target.clone();
        return vec![crate::effect::Effect::new(replacement_damage)];
    }
    effects
        .into_iter()
        .map(|effect| {
            // A self-replacement such as “this spell deals N damage ... It
            // deals M damage instead” can arrive here with the pronoun
            // incorrectly materialized as the previously damaged object being
            // the source of a fresh damage effect. In an amount replacement,
            // the target stays the original target and the resolving spell or
            // ability remains the source. Remove that synthetic source wrapper
            // while carrying the original target into the replacement.
            if let Some(with_source) =
                effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
                && effect_target_uses_it_reference(&with_source.source)
                && let Some(replacement_damage) = with_source
                    .effect
                    .downcast_ref::<crate::effects::DealDamageEffect>()
                && replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
            {
                let mut replacement_damage = replacement_damage.clone();
                replacement_damage.target = previous_target.clone();
                return crate::effect::Effect::new(replacement_damage);
            }
            if let Some(replacement_damage) =
                effect.downcast_ref::<crate::effects::DealDamageEffect>()
                && replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
            {
                let mut replacement_damage = replacement_damage.clone();
                replacement_damage.target = previous_target.clone();
                crate::effect::Effect::new(replacement_damage)
            } else {
                super::rewrite_replacement_effect_target(&effect, previous_target).unwrap_or(effect)
            }
        })
        .collect()
}

fn damage_through_optional_tag(
    effect: &crate::effect::Effect,
) -> Option<(
    Option<&crate::tag::TagKey>,
    &crate::effects::DealDamageEffect,
)> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some((None, damage));
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let damage = tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    Some((Some(&tagged.tag), damage))
}

fn replace_damage_target_preserving_tag(
    effect: &crate::effect::Effect,
    target: ChooseSpec,
) -> Option<crate::effect::Effect> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        let mut damage = damage.clone();
        damage.target = target;
        return Some(crate::effect::Effect::new(damage));
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    Some(crate::effect::Effect::new(
        crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            replace_damage_target_preserving_tag(&tagged.effect, target)?,
        ),
    ))
}

/// A two-recipient damage instruction followed by an amount-only "instead"
/// line must reuse both announced targets. A single previous-target bridge
/// cannot represent this: the first recipient may be a player or planeswalker
/// while the second is a creature controlled by that recipient. Rebind only
/// the exact coordinated pair whose replacement arms are typed backreferences
/// to those two defaults.
fn retarget_coordinated_damage_replacement_pair(
    default_effect: &crate::effect::Effect,
    replacement_effects: &[crate::effect::Effect],
    condition: &crate::effect::Condition,
) -> Option<Vec<crate::effect::Effect>> {
    fn condition_has_authored_land_antecedent(condition: &crate::effect::Condition) -> bool {
        match condition {
            crate::effect::Condition::PlayerHadLandEnterBattlefieldThisTurn {
                player: PlayerFilter::You,
            } => true,
            crate::effect::Condition::TargetMatches(filter)
            | crate::effect::Condition::SourceMatches(filter)
            | crate::effect::Condition::TaggedObjectMatches(_, filter) => {
                filter.demonstrative_antecedent_surface()
                    == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land)
            }
            crate::effect::Condition::And(left, right)
            | crate::effect::Condition::Or(left, right) => {
                condition_has_authored_land_antecedent(left)
                    || condition_has_authored_land_antecedent(right)
            }
            crate::effect::Condition::Not(inner) => condition_has_authored_land_antecedent(inner),
            _ => false,
        }
    }
    if !condition_has_authored_land_antecedent(condition) {
        return None;
    }
    let default_effect = default_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map(|with_id| with_id.effect.as_ref())
        .unwrap_or(default_effect);
    let default_sequence = default_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [replacement_effect] = replacement_effects else {
        return None;
    };
    let replacement_sequence =
        replacement_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if default_sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || replacement_sequence.surface != default_sequence.surface
    {
        return None;
    }
    let [default_first_effect, default_second_effect] = default_sequence.effects.as_slice() else {
        return None;
    };
    let [replacement_first_effect, replacement_second_effect] =
        replacement_sequence.effects.as_slice()
    else {
        return None;
    };
    let (default_first_tag, default_first) = damage_through_optional_tag(default_first_effect)?;
    let (default_second_tag, default_second) = damage_through_optional_tag(default_second_effect)?;
    let (replacement_first_tag, replacement_first) =
        damage_through_optional_tag(replacement_first_effect)?;
    let (replacement_second_tag, replacement_second) =
        damage_through_optional_tag(replacement_second_effect)?;
    if default_first_tag.is_some()
        || replacement_first_tag.is_some()
        || default_second_tag != replacement_second_tag
        || !matches!(
            default_first.target.base(),
            ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
        )
        || !default_second.target.is_target()
    {
        return None;
    }
    let second_tag = default_second_tag?;
    let ChooseSpec::Object(replacement_second_filter) = replacement_second.target.base() else {
        return None;
    };
    let ChooseSpec::Object(default_second_filter) = default_second.target.base() else {
        return None;
    };
    let replacement_first_is_antecedent = matches!(
        replacement_first.target.base(),
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::TargetPlayerOrControllerOfTarget)
    ) || replacement_first.target == default_first.target;
    let replacement_second_is_antecedent = (replacement_second_filter.tagged_constraints.len()
        == 1
        && replacement_second_filter.tagged_constraints[0].tag == *second_tag
        && replacement_second_filter.tagged_constraints[0].relation
            == crate::target::TaggedOpbjectRelation::IsTaggedObject
        && replacement_second_filter.controller.is_none())
        || replacement_second.target == default_second.target;
    if !replacement_first_is_antecedent
        || !replacement_second_is_antecedent
        || default_second_filter.controller != Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        || !default_second_filter.tagged_constraints.is_empty()
    {
        return None;
    }
    let is_exact_battlefield_creature = |filter: &ObjectFilter| {
        let mut semantic = filter.clone();
        semantic.controller = None;
        semantic.tagged_constraints.clear();
        semantic.source_surface = None;
        semantic.union_surface = crate::filter::ObjectFilterUnionSurface::default();
        semantic == ObjectFilter::creature().in_zone(Zone::Battlefield)
    };
    if !is_exact_battlefield_creature(default_second_filter)
        || !is_exact_battlefield_creature(replacement_second_filter)
    {
        return None;
    }

    let mut rewritten = replacement_sequence.clone();
    rewritten.effects = vec![
        replace_damage_target_preserving_tag(
            replacement_first_effect,
            default_first.target.clone(),
        )?,
        replace_damage_target_preserving_tag(
            replacement_second_effect,
            default_second.target.clone(),
        )?,
    ];
    Some(vec![crate::effect::Effect::new(rewritten)])
}

fn retarget_coordinated_damage_self_replacements(
    program: &mut crate::resolution::ResolutionProgram,
) {
    for segment in &mut program.segments {
        let [default_effect] = segment.default_effects.as_slice() else {
            continue;
        };
        for branch in &mut segment.self_replacements {
            if let Some(rewritten) = retarget_coordinated_damage_replacement_pair(
                default_effect,
                &branch.replacement_effects,
                &branch.condition,
            ) {
                branch.replacement_effects = rewritten;
            }
        }
    }
}

/// Normalize an amount-only self replacement authored as damage to "each of
/// those permanents and/or players". The generic for-each lowering can only
/// iterate objects, so it narrows the prior AnyTarget set to permanents and
/// redundantly nests the branch condition. The shared WithId and damage tag
/// prove that the replacement reuses the original announced target set.
fn normalize_each_damaged_target_self_replacement(
    program: &mut crate::resolution::ResolutionProgram,
) {
    for segment in &mut program.segments {
        let [default_root] = segment.default_effects.as_slice() else {
            continue;
        };
        let (default_id, default_inner) = default_root
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map_or((None, default_root), |with_id| {
                (Some(with_id.id), with_id.effect.as_ref())
            });
        let Some(default_tagged) = default_inner.downcast_ref::<crate::effects::TaggedEffect>()
        else {
            continue;
        };
        let Some(default_damage) = default_tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        else {
            continue;
        };
        if !matches!(default_damage.target.base(), ChooseSpec::AnyTarget) {
            continue;
        }

        for branch in &mut segment.self_replacements {
            let [replacement_root] = branch.replacement_effects.as_slice() else {
                continue;
            };
            let (replacement_id, replacement_inner) = replacement_root
                .downcast_ref::<crate::effects::WithIdEffect>()
                .map_or((None, replacement_root), |with_id| {
                    (Some(with_id.id), with_id.effect.as_ref())
                });
            let (nested_condition, replacement_effects) = replacement_inner
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .map_or(
                    (None, std::slice::from_ref(replacement_inner)),
                    |conditional| {
                        (
                            Some((
                                &conditional.condition,
                                conditional.surface,
                                conditional.if_false.is_empty(),
                            )),
                            conditional.if_true.as_slice(),
                        )
                    },
                );
            let [for_each_root] = replacement_effects else {
                continue;
            };
            let Some(for_each) = for_each_root.downcast_ref::<crate::effects::ForEachObject>()
            else {
                continue;
            };
            let [replacement_tagged_root] = for_each.effects.as_slice() else {
                continue;
            };
            let Some(replacement_tagged) =
                replacement_tagged_root.downcast_ref::<crate::effects::TaggedEffect>()
            else {
                continue;
            };
            let Some(replacement_damage) = replacement_tagged
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
            else {
                continue;
            };

            let mut iterated_filter = for_each.filter.clone();
            let exact_tag = matches!(
                iterated_filter.tagged_constraints.as_slice(),
                [constraint]
                    if constraint.tag == default_tagged.tag
                        && constraint.relation
                            == crate::target::TaggedOpbjectRelation::IsTaggedObject
            );
            iterated_filter.tagged_constraints.clear();
            let exact_permanent_domain = iterated_filter.zone == Some(Zone::Battlefield)
                && iterated_filter.card_types
                    == [
                        crate::types::CardType::Artifact,
                        crate::types::CardType::Creature,
                        crate::types::CardType::Enchantment,
                        crate::types::CardType::Land,
                        crate::types::CardType::Planeswalker,
                        crate::types::CardType::Battle,
                    ]
                && iterated_filter.union_surface.connective()
                    == crate::filter::ObjectFilterUnionConnective::AndOr
                && iterated_filter.union_surface.plural_object_noun();
            iterated_filter.zone = None;
            iterated_filter.card_types.clear();
            iterated_filter.union_surface = crate::filter::ObjectFilterUnionSurface::default();
            if default_id != replacement_id
                || nested_condition.is_some_and(|(condition, surface, empty_else)| {
                    condition != &branch.condition
                        || surface != ironsmith_core::ConditionalSurface::TrailingIf
                        || !empty_else
                })
                || !exact_tag
                || !exact_permanent_domain
                || iterated_filter != ObjectFilter::default()
                || replacement_tagged.tag != default_tagged.tag
                || replacement_damage.target != ChooseSpec::Iterated
            {
                continue;
            }

            let mut rebound_damage = replacement_damage.clone();
            rebound_damage.target = default_damage.target.clone();
            let mut rebound_tagged = default_tagged.clone();
            rebound_tagged.effect = Box::new(crate::effect::Effect::new(rebound_damage));
            let rebound = crate::effect::Effect::new(rebound_tagged);
            branch.replacement_effects = if let Some(id) = default_id {
                vec![crate::effect::Effect::new(crate::effects::WithIdEffect {
                    id,
                    effect: Box::new(rebound),
                })]
            } else {
                vec![rebound]
            };
        }
    }
}

#[cfg(test)]
#[test]
fn public_two_line_damage_replacement_reuses_both_announced_targets() {
    let definition = CardDefinitionBuilder::new(crate::CardId::from_raw(1), "Damage Pair Variant")
        .parse_text(
            "This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.\nLandfall — If you had a land enter the battlefield under your control this turn, this spell deals 3 damage to that player or planeswalker and 3 damage to that creature instead.",
        )
        .expect("full public document route should lower the damage replacement");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("damage pair should produce a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one typed self-replacement: {segment:#?}");
    };

    fn damage_targets(effect: &crate::effect::Effect) -> Vec<ChooseSpec> {
        let leaf = effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map_or(effect, |with_id| with_id.effect.as_ref());
        let sequence = leaf
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("coordinated damage pair");
        sequence
            .effects
            .iter()
            .map(|effect| {
                let leaf = effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .map_or(effect, |tagged| tagged.effect.as_ref());
                leaf.downcast_ref::<crate::effects::DealDamageEffect>()
                    .expect("damage leaf")
                    .target
                    .clone()
            })
            .collect()
    }

    let [default] = segment.default_effects.as_slice() else {
        panic!("expected one coordinated default effect: {segment:#?}");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one coordinated replacement effect: {branch:#?}");
    };
    assert_eq!(damage_targets(default), damage_targets(replacement));
    assert!(matches!(
        branch.presentation_label,
        Some(crate::cards::builders::PresentationLabel::AbilityWord(ref label)) if label == "Landfall"
    ));
}

fn condition_controlled_filter(condition: &crate::effect::Condition) -> Option<&ObjectFilter> {
    match condition {
        crate::effect::Condition::PlayerControls { filter, .. }
        | crate::effect::Condition::PlayerControlsExactly { filter, .. }
        | crate::effect::Condition::PlayerControlsMost { filter, .. }
        | crate::effect::Condition::PlayerControlsMoreThanEachOtherPlayer { filter, .. }
        | crate::effect::Condition::PlayerControlsMoreThanYou { filter, .. } => Some(filter),
        crate::effect::Condition::And(left, right) | crate::effect::Condition::Or(left, right) => {
            condition_controlled_filter(left).or_else(|| condition_controlled_filter(right))
        }
        crate::effect::Condition::Not(inner) => condition_controlled_filter(inner),
        _ => None,
    }
}

fn choose_spec_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => choose_spec_object_filter(spec),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
        _ => None,
    }
}

fn retarget_damage_matching_condition_filter(
    effect: crate::effect::Effect,
    previous_target: &ChooseSpec,
    condition_filter: &ObjectFilter,
) -> crate::effect::Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let rewritten_inner = retarget_damage_matching_condition_filter(
            (*tagged.effect).clone(),
            previous_target,
            condition_filter,
        );
        return crate::effect::Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            rewritten_inner,
        ));
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && choose_spec_object_filter(&damage.target)
            .is_some_and(|target| target == condition_filter)
    {
        let mut damage = damage.clone();
        damage.target = previous_target.clone();
        return crate::effect::Effect::new(damage);
    }
    effect
}

fn retarget_replacement_effects_with_condition(
    effects: Vec<crate::effect::Effect>,
    previous_target: &ChooseSpec,
    condition: &crate::effect::Condition,
) -> Vec<crate::effect::Effect> {
    let effects = retarget_replacement_effects(effects, previous_target);
    let Some(condition_filter) = condition_controlled_filter(condition) else {
        return effects;
    };
    effects
        .into_iter()
        .map(|effect| {
            retarget_damage_matching_condition_filter(effect, previous_target, condition_filter)
        })
        .collect()
}

fn unwrap_matching_conditional_replacement_effects(
    effects: Vec<crate::effect::Effect>,
    condition: &crate::effect::Condition,
) -> Vec<crate::effect::Effect> {
    let [effect] = effects.as_slice() else {
        return effects;
    };
    let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return effects;
    };
    if conditional.condition != *condition || !conditional.if_false.is_empty() {
        return effects;
    }
    conditional.if_true.clone()
}

fn damaged_death_condition_target_filter(condition: &crate::ConditionExpr) -> Option<ObjectFilter> {
    match condition {
        crate::ConditionExpr::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } if *count == 1 => {
            let mut filter = victim.clone();
            filter.zone = Some(Zone::Graveyard);
            filter.entered_graveyard_from_battlefield_this_turn = true;
            filter.dealt_damage_by_source_this_turn = Some(*damager);
            Some(filter)
        }
        crate::ConditionExpr::And(left, right) => damaged_death_condition_target_filter(left)
            .or_else(|| damaged_death_condition_target_filter(right)),
        _ => None,
    }
}

fn retarget_source_move_to_damaged_death_card(triggered: &mut crate::ability::TriggeredAbility) {
    let Some(condition) = triggered.intervening_if.as_ref() else {
        return;
    };
    let Some(filter) = damaged_death_condition_target_filter(condition) else {
        return;
    };
    let Some(segment) = triggered.effects.segments.first_mut() else {
        return;
    };
    let Some(effect) = segment.default_effects.first_mut() else {
        return;
    };
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(move_to_zone) = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != Zone::Battlefield
    {
        return;
    }

    let mut replacement = move_to_zone.clone();
    replacement.target =
        ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(1));
    *effect = crate::effect::Effect::new(crate::effects::TaggedEffect::new(
        tagged.tag.clone(),
        crate::effect::Effect::new(replacement),
    ));
}

fn rewrite_prior_token_placeholder_effect(
    effect: &mut EffectAst,
    token_info: &(
        String,
        crate::runtime_backend::token_definition::TokenDefinitionSpec,
        PlayerAst,
    ),
) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition,
            player,
            ..
        } = &mut subject_verb.action
        && matches!(
            definition,
            crate::runtime_backend::token_definition::TokenDefinitionSpec::PriorCreated
        )
    {
        *name = token_info.0.clone();
        *definition = token_info.1.clone();
        *player = token_info.2;
        subject_verb.subject.player = token_info.2;
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            rewrite_prior_token_placeholder_effect(nested_effect, token_info);
        }
    });
}

fn rewrite_prior_token_placeholders(
    effects: &mut [EffectAst],
    token_info: &(
        String,
        crate::runtime_backend::token_definition::TokenDefinitionSpec,
        PlayerAst,
    ),
) {
    for effect in effects {
        rewrite_prior_token_placeholder_effect(effect, token_info);
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                rewrite_prior_token_placeholder_effect(nested_effect, token_info);
            }
        });
    }
}

fn rewrite_prior_token_placeholder_effect_from_template(
    effect: &mut EffectAst,
    template: &(SubjectVerbActionAst, PlayerAst),
) {
    let (template_action, template_player) = template;
    let SubjectVerbActionAst::CreateTokenWithMods {
        name: template_name,
        definition: template_definition,
        dynamic_power_toughness: template_dynamic_power_toughness,
        player: template_action_player,
        attached_to: template_attached_to,
        tapped: template_tapped,
        attacking: template_attacking,
        exile_at_end_of_combat: template_exile_at_end_of_combat,
        sacrifice_at_end_of_combat: template_sacrifice_at_end_of_combat,
        sacrifice_at_next_end_step: template_sacrifice_at_next_end_step,
        exile_at_next_end_step: template_exile_at_next_end_step,
        next_end_step_player: template_next_end_step_player,
        granted_abilities: template_granted_abilities,
        ..
    } = template_action
    else {
        return;
    };
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition,
            dynamic_power_toughness,
            player,
            attached_to,
            tapped,
            attacking,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            next_end_step_player,
            granted_abilities,
            ..
        } = &mut subject_verb.action
        && matches!(
            definition,
            crate::runtime_backend::token_definition::TokenDefinitionSpec::PriorCreated
        )
    {
        *name = template_name.clone();
        *definition = template_definition.clone();
        *dynamic_power_toughness = template_dynamic_power_toughness.clone();
        *player = *template_action_player;
        *attached_to = template_attached_to.clone();
        *tapped = *template_tapped;
        *attacking = *template_attacking;
        *exile_at_end_of_combat = *template_exile_at_end_of_combat;
        *sacrifice_at_end_of_combat = *template_sacrifice_at_end_of_combat;
        *sacrifice_at_next_end_step = *template_sacrifice_at_next_end_step;
        *exile_at_next_end_step = *template_exile_at_next_end_step;
        *next_end_step_player = template_next_end_step_player.clone();
        *granted_abilities = template_granted_abilities.clone();
        subject_verb.subject.player = *template_player;
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            rewrite_prior_token_placeholder_effect_from_template(nested_effect, template);
        }
    });
}

fn rewrite_prior_token_placeholders_from_template(
    effects: &mut [EffectAst],
    template: &(SubjectVerbActionAst, PlayerAst),
) {
    for effect in effects {
        rewrite_prior_token_placeholder_effect_from_template(effect, template);
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                rewrite_prior_token_placeholder_effect_from_template(nested_effect, template);
            }
        });
    }
}

fn effect_references_prior_token_placeholder(effect: &EffectAst) -> bool {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods { definition, .. } = &subject_verb.action
        && matches!(
            definition,
            crate::runtime_backend::token_definition::TokenDefinitionSpec::PriorCreated
        )
    {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_references_prior_token_placeholder);
        }
    });
    found
}

fn created_token_template_from_effect(
    effect: &EffectAst,
) -> Option<(SubjectVerbActionAst, PlayerAst)> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::CreateTokenWithMods { .. } => {
                Some((subject_verb.action.clone(), subject_verb.subject.player))
            }
            _ => {
                let mut found = None;
                for_each_nested_effects(effect, true, |nested| {
                    for nested_effect in nested {
                        if found.is_none() {
                            found = created_token_template_from_effect(nested_effect);
                        }
                    }
                });
                found
            }
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                for nested_effect in nested {
                    if found.is_none() {
                        found = created_token_template_from_effect(nested_effect);
                    }
                }
            });
            found
        }
    }
}

fn token_template_before_prior_token_placeholder(
    effects: &[EffectAst],
) -> Option<(SubjectVerbActionAst, PlayerAst)> {
    let mut latest_token_template = None;
    for effect in effects {
        if effect_references_prior_token_placeholder(effect) {
            // A madness/self-replacement branch commonly puts the prior-token
            // placeholder in the replacement branch and the concrete token
            // blueprint in the non-replacement branch. The branches are
            // alternatives, so that concrete blueprint is the template for
            // the placeholder even though it is not earlier in this AST.
            if let EffectAst::SelfReplacement { if_false, .. } = effect {
                let template = if_false.iter().find_map(created_token_template_from_effect);
                if let Some(template) = template {
                    return Some(template);
                }
            }
            return latest_token_template;
        }
        if let Some(token_template) = created_token_template_from_effect(effect) {
            latest_token_template = Some(token_template);
        }
    }
    None
}

fn compile_trailing_instead_if_condition(
    predicate: Option<&PredicateAst>,
    prepared: &super::super::effect_pipeline::PreparedEffectsForLowering,
) -> Result<Option<crate::effect::Condition>, CardTextError> {
    let Some(predicate) = predicate else {
        return Ok(None);
    };
    compile_condition_from_predicate_ast_with_env(
        predicate,
        &prepared.initial_env,
        prepared.imports.last_object_tag.as_ref(),
    )
    .map(Some)
}

fn with_chosen_creature_type_filter(effect: crate::effect::Effect) -> crate::effect::Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence
            .effects
            .into_iter()
            .map(with_chosen_creature_type_filter)
            .collect();
        return crate::effect::Effect::new(sequence);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return crate::effect::Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            with_chosen_creature_type_filter((*tagged.effect).clone()),
        ));
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let mut continuous = continuous.clone();
        if let crate::continuous::EffectTarget::Filter(filter) = &mut continuous.target {
            filter.chosen_creature_type = true;
        }
        return crate::effect::Effect::new(continuous);
    }
    effect
}

fn creature_type_choice_program(
    facts: &StatementLineSemanticFacts,
    compiled: &crate::resolution::ResolutionProgram,
) -> Option<crate::resolution::ResolutionProgram> {
    if !facts.creature_type_choice_buff {
        return None;
    }
    let effects = compiled.to_vec();
    if effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
            .is_some()
    }) {
        return None;
    }

    let mut patched = vec![crate::effect::Effect::choose_creature_type(
        PlayerFilter::You,
        vec![],
    )];
    patched.extend(effects.into_iter().map(with_chosen_creature_type_filter));
    Some(crate::resolution::ResolutionProgram::from_effects(patched))
}

fn optional_zone_rewrite_effect(
    effect: crate::effect::Effect,
    target: ChooseSpec,
    from_zone: Zone,
    to_zone: Zone,
    replacement_zone: Zone,
    choice_description: &str,
) -> crate::effect::Effect {
    let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
        target,
        Some(from_zone),
        Some(to_zone),
        replacement_zone,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    replacement.optional = true;
    replacement.choice_description = Some(choice_description.to_string());
    crate::effect::Effect::new(crate::effects::LocalRewriteEffect::new(
        effect,
        vec![replacement],
    ))
}

fn optional_returned_creature_to_battlefield_branch(
    return_effect: crate::effect::Effect,
    condition: crate::effect::Condition,
) -> crate::resolution::SelfReplacementBranch {
    let target = ChooseSpec::Object(
        crate::target::ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .with_mana_value(crate::target::Comparison::LessThanOrEqual(4)),
    );
    let replacement_effect = optional_zone_rewrite_effect(
        return_effect,
        target,
        Zone::Graveyard,
        Zone::Hand,
        Zone::Battlefield,
        "Put one returned card with mana value 4 or less onto the battlefield",
    );
    crate::resolution::SelfReplacementBranch::new(condition, vec![replacement_effect])
}

fn search_to_hand_replacement_target(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
    if let Some(search) = effect.as_search()
        && search.destination == Zone::Hand
    {
        return Some(ChooseSpec::Object(search.filter.clone()));
    }
    if let Some(search_slots) = effect.as_search_slots()
        && search_slots.destination == Zone::Hand
        && search_slots.slots.len() == 1
    {
        return Some(ChooseSpec::Object(search_slots.slots[0].filter.clone()));
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.is_search
        && choose.zone == Some(Zone::Library)
    {
        return Some(ChooseSpec::Object(choose.filter.clone()));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .find_map(search_to_hand_replacement_target);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return search_to_hand_replacement_target(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return search_to_hand_replacement_target(&with_id.effect);
    }
    None
}

fn optional_search_to_battlefield_rewrite(
    effect: &crate::effect::Effect,
) -> Option<crate::effect::Effect> {
    let target = search_to_hand_replacement_target(effect)?;
    Some(optional_zone_rewrite_effect(
        effect.clone(),
        target,
        Zone::Library,
        Zone::Hand,
        Zone::Battlefield,
        "Put that card onto the battlefield instead of into your hand",
    ))
}

fn attach_morbid_search_to_battlefield_self_replacement(
    builder: &mut CardDefinitionBuilder,
    facts: &StatementLineSemanticFacts,
) -> bool {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::MorbidSearchToBattlefield) {
        return false;
    }
    let Some(existing) = builder.spell_effect.as_mut() else {
        return false;
    };
    let Some(segment) = existing.last_segment_mut() else {
        return false;
    };
    let replacement_effects = segment
        .default_effects
        .iter()
        .map(optional_search_to_battlefield_rewrite)
        .collect::<Option<Vec<_>>>();
    let Some(replacement_effects) = replacement_effects else {
        return false;
    };
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            crate::effect::Condition::CreatureDiedThisTurn,
            replacement_effects,
        ));
    true
}

fn back_for_seconds_style_replacement_program(
    compiled: &[crate::effect::Effect],
    facts: &StatementLineSemanticFacts,
) -> Option<crate::resolution::ResolutionProgram> {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::BargainedReturnToBattlefield)
    {
        return None;
    }
    let (default_effects, return_effect, condition) = match compiled {
        [return_effect, followup] => {
            let conditional = conditional_self_replacement_followup(followup)?;
            if !conditional.if_false.is_empty() {
                return None;
            }
            (
                vec![return_effect.clone()],
                return_effect.clone(),
                conditional.condition,
            )
        }
        [target_only, return_effect, followup]
            if target_only
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            let conditional = conditional_self_replacement_followup(followup)?;
            if !conditional.if_false.is_empty() {
                return None;
            }
            (
                vec![target_only.clone(), return_effect.clone()],
                return_effect.clone(),
                conditional.condition,
            )
        }
        _ => return None,
    };
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(optional_returned_creature_to_battlefield_branch(
            return_effect,
            condition,
        ));
    Some(program)
}

fn attach_back_for_seconds_style_replacement(
    builder: &mut CardDefinitionBuilder,
    compiled: &[crate::effect::Effect],
    facts: &StatementLineSemanticFacts,
) -> bool {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::BargainedReturnToBattlefield)
    {
        return false;
    }
    let [followup] = compiled else {
        return false;
    };
    let Some(conditional) = conditional_self_replacement_followup(followup) else {
        return false;
    };
    if !conditional.if_false.is_empty() {
        return false;
    }
    let Some(existing) = builder.spell_effect.as_mut() else {
        return false;
    };
    let Some(segment) = existing.last_segment_mut() else {
        return false;
    };
    let [return_effect] = segment.default_effects.as_slice() else {
        return false;
    };
    segment
        .self_replacements
        .push(optional_returned_creature_to_battlefield_branch(
            return_effect.clone(),
            conditional.condition,
        ));
    true
}

fn kicked_count_override_self_replacement_program(
    compiled: &[crate::effect::Effect],
    facts: &StatementLineSemanticFacts,
) -> Option<crate::resolution::ResolutionProgram> {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::KickedCountOverride) {
        return None;
    }
    let [look_effect, conditional_effect] = compiled else {
        return None;
    };
    if look_effect
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .is_none()
    {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::ThisSpellWasKicked
        || conditional.if_true.is_empty()
        || conditional.if_false.is_empty()
    {
        return None;
    }
    let mut default_effects = vec![look_effect.clone()];
    default_effects.extend(conditional.if_false.clone());
    let mut replacement_effects = vec![look_effect.clone()];
    replacement_effects.extend(conditional.if_true.clone());
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        ));
    Some(program)
}

fn kicked_multi_zone_search_to_battlefield_program(
    compiled: &[crate::effect::Effect],
) -> Option<crate::resolution::ResolutionProgram> {
    let [choose, reveal, move_to_hand, shuffle, conditional] = compiled else {
        return None;
    };
    let choose_search = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose_search.is_search
        || choose_primary_zone_for_kicked_multi_zone_search(choose_search) != Some(Zone::Library)
        || !zone_list_includes(&choose_search.additional_zones, Zone::Graveyard)
    {
        return None;
    }
    let conditional = conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::ThisSpellWasKicked {
        return None;
    }

    let searched_tag = choose_search.tag.clone();
    let default_effects = vec![
        choose.clone(),
        reveal.clone(),
        move_to_hand.clone(),
        shuffle.clone(),
    ];
    let replacement_effects = vec![
        choose.clone(),
        reveal.clone(),
        crate::effect::Effect::move_to_zone(
            ChooseSpec::tagged(searched_tag),
            Zone::Battlefield,
            false,
        ),
        shuffle.clone(),
    ];
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        ));
    Some(program)
}

fn rewrite_tagged_hand_move_to_battlefield(
    effect: &crate::effect::Effect,
    tag: &crate::TagKey,
) -> (crate::effect::Effect, bool) {
    if let Some(move_effect) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_effect.target == ChooseSpec::Tagged(tag.clone())
        && move_effect.zone == Zone::Hand
    {
        let mut replacement = move_effect.clone();
        replacement.zone = Zone::Battlefield;
        return (crate::effect::Effect::new(replacement), true);
    }

    if let Some(for_each) =
        effect.downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
    {
        let mut changed = false;
        let effects = for_each
            .effects
            .iter()
            .map(|child| {
                let (rewritten, child_changed) =
                    rewrite_tagged_hand_move_to_battlefield(child, tag);
                changed |= child_changed;
                rewritten
            })
            .collect::<Vec<_>>();
        if changed {
            return (
                crate::effect::Effect::for_each_tagged(for_each.tag.clone(), effects),
                true,
            );
        }
    }

    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let (rewritten, changed) = rewrite_tagged_hand_move_to_battlefield(&tagged.effect, tag);
        if changed {
            return (
                crate::effect::Effect::new(crate::effects::TaggedEffect::new(
                    tagged.tag.clone(),
                    rewritten,
                )),
                true,
            );
        }
    }

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut changed = false;
        let effects = sequence
            .effects
            .iter()
            .map(|child| {
                let (rewritten, child_changed) =
                    rewrite_tagged_hand_move_to_battlefield(child, tag);
                changed |= child_changed;
                rewritten
            })
            .collect::<Vec<_>>();
        if changed {
            return (
                crate::effect::Effect::new(crate::effects::SequenceEffect::new(effects)),
                true,
            );
        }
    }

    (effect.clone(), false)
}

fn attach_kicked_multi_zone_search_to_battlefield_replacement(
    builder: &mut CardDefinitionBuilder,
    compiled: &[crate::effect::Effect],
    facts: &StatementLineSemanticFacts,
) -> bool {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::KickedMultiZoneToBattlefield)
    {
        return false;
    }
    let [conditional] = compiled else {
        return false;
    };
    let Some(conditional) = conditional.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return false;
    };
    if conditional.condition != crate::effect::Condition::ThisSpellWasKicked {
        return false;
    }
    let Some(existing) = builder.spell_effect.as_mut() else {
        return false;
    };
    let Some(segment) = existing.last_segment_mut() else {
        return false;
    };
    let Some(choose) = segment
        .default_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
    else {
        return false;
    };
    if !choose.is_search
        || choose_primary_zone_for_kicked_multi_zone_search(choose) != Some(Zone::Library)
        || !zone_list_includes(&choose.additional_zones, Zone::Graveyard)
    {
        return false;
    }

    let tag = choose.tag.clone();
    let mut changed = false;
    let replacement_effects = segment
        .default_effects
        .iter()
        .map(|effect| {
            let (rewritten, effect_changed) = rewrite_tagged_hand_move_to_battlefield(effect, &tag);
            changed |= effect_changed;
            rewritten
        })
        .collect::<Vec<_>>();
    if !changed {
        return false;
    }
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        ));
    true
}

fn choose_primary_zone_for_kicked_multi_zone_search(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<Zone> {
    choose.zone
}

fn zone_list_includes(zones: &[Zone], expected: Zone) -> bool {
    for zone in zones {
        if *zone == expected {
            return true;
        }
    }
    false
}

fn clash_win_optional_top_replacement_program(
    compiled: &[crate::effect::Effect],
    facts: &StatementLineSemanticFacts,
) -> Option<crate::resolution::ResolutionProgram> {
    if !facts.has_replacement_surface(StatementReplacementSurfaceKind::ClashWinTopOfLibrary) {
        return None;
    }

    // Result-id lowering may either leave the clash and tagged return as
    // separate effects (the legacy shape), or group the authored
    // "clash, then return" clause in one `CommaThen` sequence. In both cases
    // the following "if you win" must observe the clash, while the optional
    // library move rewrites the return's zone change.
    let (clash_id, clash_effect, return_effect, followup) = match compiled {
        [clash_effect, return_with_id_effect, followup]
            if clash_effect
                .downcast_ref::<crate::effects::ClashEffect>()
                .is_some() =>
        {
            let return_with_id = return_with_id_effect.as_with_id()?;
            (
                return_with_id.id,
                clash_effect,
                return_with_id.effect.as_ref(),
                followup,
            )
        }
        [grouped_effect, followup] => {
            let grouped = grouped_effect.as_with_id()?;
            let sequence = grouped
                .effect
                .downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
                return None;
            }
            let [clash_effect, return_effect] = sequence.effects.as_slice() else {
                return None;
            };
            if clash_effect
                .downcast_ref::<crate::effects::ClashEffect>()
                .is_none()
            {
                return None;
            }
            (grouped.id, clash_effect, return_effect, followup)
        }
        _ => return None,
    };

    let tagged_return = return_effect.as_tagged()?;
    if tagged_return
        .effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_none()
    {
        return None;
    }
    let return_tag = tagged_return.tag.clone();

    let followup = followup.downcast_ref::<crate::effects::IfEffect>()?;
    if followup.condition != clash_id
        || !matches!(
            followup.predicate,
            crate::effect::EffectPredicate::Happened
                | crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        )
        || !followup.else_.is_empty()
    {
        return None;
    }

    let [optional_move] = followup.then.as_slice() else {
        return None;
    };
    let optional_move =
        optional_move.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()?;
    let [move_effect] = optional_move.effects.as_slice() else {
        return None;
    };
    let move_effect = move_effect
        .as_tagged()
        .map_or(move_effect, |tagged| tagged.effect.as_ref());
    let move_to_library = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_filter = choose_spec_object_filter(&move_to_library.target)?;
    if move_to_library.zone != Zone::Library
        || !move_to_library.to_top
        || !move_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == return_tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }

    let target = ChooseSpec::Tagged(return_tag);
    let return_effect = return_effect.clone();
    let replacement_return = optional_zone_rewrite_effect(
        return_effect.clone(),
        target,
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        "Put that creature on top of its owner's library instead of into its owner's hand",
    );
    Some(crate::resolution::ResolutionProgram::from_effects(vec![
        crate::effect::Effect::with_id(clash_id.0, clash_effect.clone()),
        crate::effect::Effect::new(crate::effects::IfEffect::new(
            clash_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
            vec![replacement_return],
            vec![return_effect],
        )),
    ]))
}

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &LineInfo,
    semantic_facts: &LineSemanticFacts,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let mut builder = match parsed {
        parsed @ NormalizedLineChunk::Abilities(_) => {
            lower_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbility(_) => {
            lower_static_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbilities(_) => {
            lower_static_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Ability(_) => {
            lower_parsed_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Statement { .. } => {
            lower_statement_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCost { .. } => {
            lower_additional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCost(_) => {
            lower_optional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::GiftKeyword { .. } => {
            lower_gift_keyword_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCostWithCastTrigger { .. } => {
            lower_optional_cost_with_cast_trigger_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCostChoice { .. } => {
            lower_additional_cost_choice_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AlternativeCastingMethod(_) => {
            lower_alternative_casting_method_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Triggered { .. } => {
            lower_triggered_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                semantic_facts,
                allow_unsupported,
                annotations,
            })
        }
    }?;
    if let Some(program) = builder.spell_effect.as_mut() {
        let authored_tokens =
            crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
                .unwrap_or_else(|_| info.source_tokens.clone());
        bind_each_opponent_sacrifice_failure_half_life(program, &authored_tokens);
        bind_optional_linked_mana_value_damage(program, &info.raw_line);
        preserve_repeated_comma_then_surface(program, &info.raw_line);
    }
    Ok(builder)
}

/// Preserve the distinction between one trailing `, then` and an authored
/// chain that repeats `, then` before every later action. Both execute as the
/// same sequence, but quantified-player text needs the typed distinction to
/// avoid dropping the first connective.
fn preserve_repeated_comma_then_surface(
    program: &mut crate::resolution::ResolutionProgram,
    raw_line: &str,
) {
    let boundary_count = raw_line.to_ascii_lowercase().matches(", then").count();
    if boundary_count < 2 {
        return;
    }

    fn rewrite(effect: &crate::effect::Effect, boundary_count: usize) -> crate::effect::Effect {
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
            && sequence.surface == ironsmith_core::SequenceSurface::CommaThen
            && sequence.effects.len() == boundary_count + 1
        {
            let mut sequence = sequence.clone();
            sequence.surface = ironsmith_core::SequenceSurface::RepeatedCommaThen;
            return crate::effect::Effect::new(sequence);
        }
        if let Some(for_players) =
            effect.downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
        {
            let mut for_players = for_players.clone();
            for_players.effects = for_players
                .effects
                .iter()
                .map(|child| rewrite(child, boundary_count))
                .collect();
            return crate::effect::Effect::new(for_players);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(rewrite(&with_id.effect, boundary_count));
            return crate::effect::Effect::new(with_id);
        }
        effect.clone()
    }

    for segment in &mut program.segments {
        segment.default_effects = segment
            .default_effects
            .iter()
            .map(|effect| rewrite(effect, boundary_count))
            .collect();
    }
}

fn lower_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        ..
    } = input;
    let NormalizedLineChunk::Abilities(actions) = parsed else {
        unreachable!("abilities lowerer received mismatched chunk");
    };

    if actions.len() > 1 {
        builder = builder.with_ability(
            Ability::static_ability(
                crate::static_abilities::StaticAbility::source_line_keyword_group(actions.len()),
            )
            .in_zones(Vec::new()),
        );
    }

    for action in actions {
        match action {
            crate::payload::KeywordAction::Backup(amount) => {
                state.pending_backups.push(PendingBackup {
                    ability_boundary: builder.abilities.len(),
                    amount,
                });
            }
            crate::payload::KeywordAction::Cipher => state.pending_cipher = true,
            action => builder = builder.apply_keyword_action(action),
        }
    }
    Ok(builder)
}

fn compile_static_ability_with_zones(
    mut ability: crate::static_abilities::StaticAbility,
    facts: &crate::runtime_backend::shared_types::StaticLineSemanticFacts,
    preserve_single_ability_label: bool,
) -> Ability {
    let conditional_shape = matches!(
        ability.payload,
        crate::static_abilities::StaticAbilityPayload::Conditional { .. }
    );
    let typed_payload_owns_full_surface = matches!(
        ability.payload,
        crate::static_abilities::StaticAbilityPayload::Companion(_)
    );
    let authored_flash_permission_surface = ability.id()
        == crate::static_abilities::StaticAbilityId::Flash
        && ability.label.to_ascii_lowercase().contains("as though");
    if authored_flash_permission_surface
        && let Some(label) = facts
            .presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
        && !ability.label.starts_with(&format!("{label} —"))
    {
        ability.label = format!("{label} — {}", ability.label);
    }
    if !authored_flash_permission_surface
        && (conditional_shape
            || (preserve_single_ability_label && !typed_payload_owns_full_surface))
        && let Some(label) = facts
            .presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
    {
        // Keep the authored ability word separate from the explicit
        // condition. This marker changes only compiled-text presentation;
        // the typed condition continues to drive runtime behavior.
        ability.label = format!(
            "{}{label}",
            ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX
        );
    }
    let ability = rewrite_self_spell_cost_modifier(ability, facts);
    let mut compiled = Ability::static_ability(ability);
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_spell_only_functional_zones(static_ability)
    {
        compiled = compiled.in_zones(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_all_zone_functional_zones(static_ability)
    {
        let mut zones = vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ];
        if static_ability.id()
            == crate::static_abilities::StaticAbilityId::CountersRemainAcrossZoneChanges
        {
            zones.extend([Zone::Ante, Zone::OutsideGame]);
        }
        compiled = compiled.in_zones(zones);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_referenced_ability_functional_zones(
            static_ability,
            facts.references_this_ability_cost,
        )
    {
        compiled = compiled.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let Some(zones) = &facts.explicit_functional_zones {
        compiled = compiled.in_zones(zones.clone());
    }
    compiled
}

fn effect_returns_removed_counter_count(effect: &crate::effect::Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::RemoveUpToCountersEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
            .is_some()
    {
        return true;
    }
    if let Some(with_id) = effect.as_with_id() {
        return effect_returns_removed_counter_count(&with_id.effect);
    }
    if let Some(tagged) = effect.as_tagged() {
        return effect_returns_removed_counter_count(&tagged.effect);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return matches!(for_each.effects.as_slice(), [inner] if effect_returns_removed_counter_count(inner));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return matches!(sequence.effects.as_slice(), [inner] if effect_returns_removed_counter_count(inner));
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>() {
        return matches!(may.effects.as_slice(), [inner] if effect_returns_removed_counter_count(inner));
    }
    false
}

fn removed_counter_metric_effect_id(
    effect: &crate::effect::Effect,
) -> Option<crate::effect::EffectId> {
    if let Some(with_id) = effect.as_with_id()
        && effect_returns_removed_counter_count(&with_id.effect)
    {
        return Some(with_id.id);
    }
    if let Some(tagged) = effect.as_tagged() {
        return removed_counter_metric_effect_id(&tagged.effect);
    }
    None
}

fn max_effect_id(effect: &crate::effect::Effect) -> Option<u32> {
    let mut maximum = effect.as_with_id().map(|with_id| with_id.id.0);
    effect.visit_child_effects(&mut |child| {
        if let Some(child_maximum) = max_effect_id(child) {
            maximum = Some(maximum.map_or(child_maximum, |current| current.max(child_maximum)));
        }
    });
    maximum
}

fn bind_removed_counter_followup(
    program: &mut crate::resolution::ResolutionProgram,
    counter: crate::object::CounterType,
    count: &crate::effect::Value,
) -> bool {
    let crate::effect::Value::PendingPriorEffectMetric(query) = count.unhinted() else {
        return false;
    };
    if query.action != Some(ironsmith_core::PriorEffectAction::Removed) {
        return false;
    }

    let producer =
        program
            .segments
            .iter()
            .enumerate()
            .rev()
            .find_map(|(segment_index, segment)| {
                segment
                    .default_effects
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, effect)| effect_returns_removed_counter_count(effect))
                    .map(|(effect_index, _)| (segment_index, effect_index))
            });
    let Some((segment_index, effect_index)) = producer else {
        return false;
    };

    let existing_id = removed_counter_metric_effect_id(
        &program.segments[segment_index].default_effects[effect_index],
    );
    let effect_id = if let Some(effect_id) = existing_id {
        effect_id
    } else {
        let next_id = program
            .segments
            .iter()
            .flat_map(|segment| {
                segment.default_effects.iter().chain(
                    segment
                        .self_replacements
                        .iter()
                        .flat_map(|branch| branch.replacement_effects.iter()),
                )
            })
            .filter_map(max_effect_id)
            .max()
            .map_or(Some(0), |maximum| maximum.checked_add(1))
            .filter(|next| *next != u32::MAX);
        let Some(next_id) = next_id else {
            return false;
        };
        let producer = program.segments[segment_index].default_effects[effect_index].clone();
        program.segments[segment_index].default_effects[effect_index] =
            crate::effect::Effect::with_id(next_id, producer);
        crate::effect::EffectId(next_id)
    };

    let bound_count = crate::effect::Value::PriorEffectMetric {
        effect_id,
        query: query.clone(),
    }
    .with_surface_hints(count.surface_hints().iter().cloned());
    *program = crate::resolution::ResolutionProgram::new(program.segments.clone());
    program.push(crate::effect::Effect::put_counters(
        counter,
        bound_count,
        ChooseSpec::Source,
    ));
    true
}

fn fuse_pending_removed_counter_as_enters(builder: &mut CardDefinitionBuilder) {
    let Some((counter_ability_index, counter, count)) = builder
        .abilities
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, ability)| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let crate::static_abilities::StaticAbilityPayload::EntersWithCountersValue {
                counter,
                count,
            } = &static_ability.payload
            else {
                return None;
            };
            matches!(
                count.unhinted(),
                crate::effect::Value::PendingPriorEffectMetric(query)
                    if query.action == Some(ironsmith_core::PriorEffectAction::Removed)
            )
            .then(|| (index, *counter, count.clone()))
        })
    else {
        return;
    };

    let as_enters_index = builder
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(index, ability)| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            matches!(
                &static_ability.payload,
                crate::static_abilities::StaticAbilityPayload::AsEntersEffectProgram {
                    uses_enters_with_counter_surface: true,
                    ..
                }
            )
            .then_some(index)
        })
        .min_by_key(|index| index.abs_diff(counter_ability_index));
    let Some(as_enters_index) = as_enters_index else {
        return;
    };

    let AbilityKind::Static(as_enters) = &mut builder.abilities[as_enters_index].kind else {
        return;
    };
    let crate::static_abilities::StaticAbilityPayload::AsEntersEffectProgram { program, .. } =
        &mut as_enters.payload
    else {
        return;
    };
    if bind_removed_counter_followup(program, counter, &count) {
        builder.abilities.remove(counter_ability_index);
    }
}

fn remember_single_player_choice_as_enters(program: &mut crate::resolution::ResolutionProgram) {
    let choices =
        program
            .segments
            .iter()
            .enumerate()
            .flat_map(|(segment_index, segment)| {
                segment.default_effects.iter().enumerate().filter_map(
                    move |(effect_index, effect)| {
                        effect
                            .downcast_ref::<crate::effects::ChoosePlayerEffect>()
                            .map(|choice| (segment_index, effect_index, choice.clone()))
                    },
                )
            })
            .collect::<Vec<_>>();
    let [(segment_index, effect_index, choice)] = choices.as_slice() else {
        return;
    };
    program.segments[*segment_index].default_effects[*effect_index] =
        crate::effect::Effect::new(choice.clone().remember_as_chosen_player());
}

fn rewrite_self_spell_cost_modifier(
    ability: crate::static_abilities::StaticAbility,
    facts: &crate::runtime_backend::shared_types::StaticLineSemanticFacts,
) -> crate::static_abilities::StaticAbility {
    let Some(parsed_surface) = facts.this_spell_cost else {
        return ability;
    };

    match &ability.payload {
        ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => {
            let mut amount = reduction.amount.clone();
            if let Some(cap) = parsed_surface.reduction_cap {
                amount = crate::effect::Value::Min(
                    Box::new(amount),
                    Box::new(crate::effect::Value::Fixed(cap)),
                );
            }
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReductionManaCost::new(
                    reduction.cost.clone(),
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        _ => ability,
    }
}

fn bind_authored_chosen_creature_static_filters(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(3)
        .any(|window| window == ["the", "chosen", "creature"])
        || !words
            .iter()
            .any(|word| matches!(*word, "gets" | "get" | "has" | "have"))
    {
        return false;
    }

    fn bind_filter(filter: &mut ObjectFilter) -> bool {
        let [constraint] = filter.tagged_constraints.as_slice() else {
            return false;
        };
        if constraint.tag.as_str() != crate::cards::builders::IT_TAG
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        {
            return false;
        }
        let mut semantic = filter.clone();
        semantic.tagged_constraints.clear();
        semantic.union_surface = Default::default();
        if semantic != ObjectFilter::creature().in_zone(Zone::Battlefield) {
            return false;
        }
        filter.tagged_constraints[0].tag =
            crate::tag::TagKey::from(ironsmith_core::CHOSEN_OBJECTS_TAG);
        true
    }

    fn bind_payload(payload: &mut crate::static_abilities::StaticAbilityPayload) -> bool {
        match payload {
            crate::static_abilities::StaticAbilityPayload::Anthem(anthem) => {
                anthem.filter.as_mut().is_some_and(bind_filter)
            }
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                bind_filter(&mut grant.filter)
            }
            crate::static_abilities::StaticAbilityPayload::Conditional { ability, .. } => {
                bind_payload(&mut ability.payload)
            }
            _ => false,
        }
    }

    let mut changed = false;
    for ability in abilities {
        changed |= bind_payload(&mut ability.payload);
    }
    changed
}

fn bind_authored_chosen_creature_sacrifice(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let exact_source_leaves_trigger = match &triggered.trigger.kind {
        crate::triggers::TriggerKind::ThisLeavesBattlefield => true,
        crate::triggers::TriggerKind::ZoneChange(zone_change) => {
            zone_change.this
                && zone_change.from == Some(Zone::Battlefield)
                && zone_change.from_zones.is_none()
                && zone_change.from_excluded.is_none()
                && zone_change.to.is_none()
                && zone_change.to_excluded.is_none()
                && zone_change.count == crate::triggers::CountMode::One
        }
        _ => false,
    };
    if !words
        .windows(4)
        .any(|window| window == ["sacrifice", "the", "chosen", "creature"])
        || !words
            .windows(5)
            .any(|window| window == ["this", "creature", "leaves", "the", "battlefield"])
        || !exact_source_leaves_trigger
        || !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let (choose_effect, sacrifice_effect) = match segment.default_effects.as_slice() {
        [tag_triggering, choose_effect, sacrifice_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (choose_effect, sacrifice_effect)
        }
        [sequence] => {
            let Some(sequence) = sequence.downcast_ref::<crate::effects::SequenceEffect>() else {
                return false;
            };
            let [choose_effect, sacrifice_effect] = sequence.effects.as_slice() else {
                return false;
            };
            (choose_effect, sacrifice_effect)
        }
        _ => return false,
    };
    let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return false;
    };
    let Some(sacrifice) = sacrifice_effect.downcast_ref::<crate::effects::SacrificePlayerEffect>()
    else {
        return false;
    };
    let triggering_constraint = crate::target::TaggedObjectConstraint {
        tag: crate::tag::TagKey::from("triggering"),
        relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    };
    let mut choose_filter = choose.filter.clone();
    choose_filter.tagged_constraints.clear();
    choose_filter.union_surface = Default::default();
    if !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose.filter.tagged_constraints.as_slice() != [triggering_constraint]
        || choose_filter != ObjectFilter::creature().in_zone(Zone::Battlefield)
        || sacrifice.player != PlayerFilter::You
        || !sacrifice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == choose.tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
    {
        return false;
    }
    let chosen_creature = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .match_tagged(
            ironsmith_core::CHOSEN_OBJECTS_TAG,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
    segment.default_effects = vec![crate::effect::Effect::new(
        crate::effects::SacrificeTargetEffect::new(ChooseSpec::Object(chosen_creature)),
    )];
    true
}

fn lower_static_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbility(ability) = parsed else {
        unreachable!("static-ability lowerer received mismatched chunk");
    };

    if let StaticAbilityAst::AttachmentRestriction { filter, display } = ability {
        builder.aura_attach_filter = Some(filter);
        let _ = display;
        return Ok(builder);
    }

    // Fuse is card metadata, not a battlefield static ability.  The document
    // parser classifies the bare keyword with the other static keyword lines,
    // so handle it before the generic static-ability lowering path.
    if matches!(
        ability,
        StaticAbilityAst::KeywordAction(KeywordAction::Fuse)
    ) {
        return Ok(builder.has_fuse());
    }

    let ability = match super::rewrite_lower_static_ability_ast(ability) {
        Ok(ability) => ability,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    let mut ability = rewrite_self_spell_cost_modifier(ability, &semantic_facts.static_ability);
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    bind_fixed_reduction_to_as_long_as_draw_threshold(
        std::slice::from_mut(&mut ability),
        &info.source_tokens,
    );
    bind_fixed_reduction_to_as_long_as_draw_threshold(
        std::slice::from_mut(&mut ability),
        &authored_tokens,
    );
    bind_authored_chosen_creature_static_filters(
        std::slice::from_mut(&mut ability),
        &authored_tokens,
    );
    bind_authored_spell_cost_filter_qualifiers(
        std::slice::from_mut(&mut ability),
        &authored_tokens,
    );
    bind_authored_named_token_static_filters(std::slice::from_mut(&mut ability), &authored_tokens);
    let mut compiled =
        compile_static_ability_with_zones(ability, &semantic_facts.static_ability, true);
    preserve_as_long_as_its_your_turn_static_surface(&mut compiled, &info.source_tokens);
    preserve_as_long_as_its_your_turn_static_surface(&mut compiled, &authored_tokens);
    builder = builder.with_ability(compiled);
    fuse_pending_removed_counter_as_enters(&mut builder);
    Ok(builder)
}

/// Restore grammar-proven spell qualifiers that occur between the spell noun
/// and the cost-modification verb.  Some static-line routes finish building
/// the reduction from the leading subject before those trailing qualifiers
/// have been folded into its executable filter.
fn bind_authored_spell_cost_filter_qualifiers(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let is_named_cast_cost_modifier = words
        .windows(2)
        .any(|window| matches!(window, ["blitz", "cost" | "costs"]));
    let is_cost_modifier = words.iter().any(|word| matches!(*word, "cost" | "costs"))
        && (words.iter().any(|word| matches!(*word, "spell" | "spells"))
            || is_named_cast_cost_modifier);
    if !is_cost_modifier {
        return false;
    }
    let has_x_in_cost = [
        &["with", "x", "in", "its", "mana", "cost"][..],
        &["with", "x", "in", "their", "mana", "cost"][..],
    ]
    .iter()
    .any(|phrase| words.windows(phrase.len()).any(|window| window == *phrase));
    let kicked_spell = words
        .windows(2)
        .any(|window| matches!(window, ["kicked", "spell" | "spells"]));
    let commander_cast_count = words
        .windows(3)
        .any(|window| window == ["for", "each", "time"])
        && words.iter().any(|word| *word == "cast")
        && words.iter().any(|word| *word == "commander")
        && words
            .windows(4)
            .any(|window| window == ["from", "the", "command", "zone"]);
    if !has_x_in_cost && !kicked_spell && !commander_cast_count {
        return false;
    }

    let mut changed = false;
    for ability in abilities {
        let crate::static_abilities::StaticAbilityPayload::CostReduction(reduction) =
            &mut ability.payload
        else {
            continue;
        };
        if has_x_in_cost && !reduction.filter.has_x_in_cost {
            reduction.filter.has_x_in_cost = true;
            changed = true;
        }
        if kicked_spell
            && !reduction
                .filter
                .ability_markers
                .iter()
                .any(|marker| marker.eq_ignore_ascii_case("kicked"))
        {
            reduction.filter.ability_markers.push("kicked".to_string());
            changed = true;
        }
        if commander_cast_count
            && reduction.amount.unhinted()
                != &crate::effect::Value::CommanderCastCount(PlayerFilter::You)
        {
            reduction.amount = crate::effect::Value::CommanderCastCount(PlayerFilter::You);
            changed = true;
        }
    }
    changed
}

/// Preserve a subtype that grammatically modifies `tokens you control` across
/// every member of a compound static line. The broad token-subject route can
/// otherwise retain only `token`, making a Blood/Clue/Treasure rule apply to
/// every token the player controls.
fn bind_authored_named_token_static_filters(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let [subtype_word, "tokens", "you", "control", ..] = words.as_slice() else {
        return false;
    };
    let Some(subtype) =
        crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(subtype_word)
    else {
        return false;
    };

    fn add_subtype(filter: &mut ObjectFilter, subtype: crate::types::Subtype) -> bool {
        if !filter.token
            || filter.controller != Some(PlayerFilter::You)
            || filter.subtypes.contains(&subtype)
        {
            return false;
        }
        filter.subtypes.push(subtype);
        true
    }

    let mut changed = false;
    for ability in abilities {
        changed |= match &mut ability.payload {
            crate::static_abilities::StaticAbilityPayload::AddCardTypes { filter, .. }
            | crate::static_abilities::StaticAbilityPayload::AddSubtypes { filter, .. } => {
                add_subtype(filter, subtype)
            }
            crate::static_abilities::StaticAbilityPayload::GrantAbility(grant) => {
                add_subtype(&mut grant.filter, subtype)
            }
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                add_subtype(&mut grant.filter, subtype)
            }
            _ => false,
        };
    }
    changed
}

fn is_source_line_static_loss_group(abilities: &[crate::static_abilities::StaticAbility]) -> bool {
    let Some(first) = abilities.first() else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::RemoveAbilityForFilter {
        filter: first_filter,
        mode: ironsmith_core::AbilityLossMode::Lose,
        ..
    } = &first.payload
    else {
        return false;
    };
    abilities.len() > 1
        && abilities.iter().skip(1).all(|ability| {
            matches!(
                &ability.payload,
                crate::static_abilities::StaticAbilityPayload::RemoveAbilityForFilter {
                    filter,
                    mode: ironsmith_core::AbilityLossMode::Lose,
                    ..
                } if filter == first_filter
            )
        })
}

fn is_source_line_anthem_keyword_loss_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, second] = abilities else {
        return false;
    };
    let mut anthem = None;
    let mut removal = None;
    for ability in [first, second] {
        match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::Anthem(spec) if anthem.is_none() => {
                anthem = spec
                    .filter
                    .as_ref()
                    .map(|filter| (filter, spec.condition.as_ref()));
            }
            crate::static_abilities::StaticAbilityPayload::RemoveAbilityForFilter {
                filter,
                mode: ironsmith_core::AbilityLossMode::Lose,
                ..
            } if removal.is_none() => {
                removal = Some((filter, None));
            }
            crate::static_abilities::StaticAbilityPayload::Conditional {
                ability: inner,
                condition,
            } if removal.is_none() => {
                let crate::static_abilities::StaticAbilityPayload::RemoveAbilityForFilter {
                    filter,
                    mode: ironsmith_core::AbilityLossMode::Lose,
                    ..
                } = &inner.payload
                else {
                    return false;
                };
                removal = Some((filter, Some(condition)));
            }
            _ => return false,
        }
    }
    matches!(
        (anthem, removal),
        (Some((anthem_filter, anthem_condition)), Some((removal_filter, removal_condition)))
            if anthem_filter == removal_filter && anthem_condition == removal_condition
    )
}

fn is_source_line_grant_keyword_loss_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, second] = abilities else {
        return false;
    };
    let mut grant_filter = None;
    let mut loss_filter = None;
    for ability in [first, second] {
        match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(spec)
                if grant_filter.is_none()
                    && spec.condition.is_none()
                    && spec.additional_abilities.is_empty()
                    && spec.set_quantifier_surface.is_none()
                    && matches!(&spec.ability.kind, ironsmith_core::AbilityKind::Static(_)) =>
            {
                grant_filter = Some(&spec.filter);
            }
            crate::static_abilities::StaticAbilityPayload::RemoveAbilityForFilter {
                filter,
                mode: ironsmith_core::AbilityLossMode::Lose,
                ..
            } if loss_filter.is_none() => {
                loss_filter = Some(filter);
            }
            _ => return false,
        }
    }
    matches!(
        (grant_filter, loss_filter),
        (Some(grant_filter), Some(loss_filter)) if grant_filter == loss_filter
    )
}

fn conditional_rule_restriction(
    ability: &crate::static_abilities::StaticAbility,
) -> Option<(&crate::effect::Restriction, &crate::ConditionExpr)> {
    let crate::static_abilities::StaticAbilityPayload::Conditional {
        ability: inner,
        condition,
    } = &ability.payload
    else {
        return None;
    };
    let crate::static_abilities::StaticAbilityPayload::RuleRestriction {
        restriction,
        additional_restrictions,
        ..
    } = &inner.payload
    else {
        return None;
    };
    additional_restrictions
        .is_empty()
        .then_some((restriction, condition))
}

fn is_source_line_cast_activation_restriction_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, second] = abilities else {
        return false;
    };
    let Some((first_restriction, first_condition)) = conditional_rule_restriction(first) else {
        return false;
    };
    let Some((second_restriction, second_condition)) = conditional_rule_restriction(second) else {
        return false;
    };
    if first_condition != second_condition {
        return false;
    }
    matches!(
        (first_restriction, second_restriction),
        (
            crate::effect::Restriction::CastSpellsMatching(cast_player, _),
            crate::effect::Restriction::ActivateNonManaAbilities(activation_player),
        ) | (
            crate::effect::Restriction::ActivateNonManaAbilities(activation_player),
            crate::effect::Restriction::CastSpellsMatching(cast_player, _),
        ) if cast_player == activation_player
    )
}

fn is_source_line_spell_cost_reduction_counter_protection_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [reduction, protection] = abilities else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::CostReduction(reduction) =
        &reduction.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::RuleRestriction {
        restriction: crate::effect::Restriction::BeCountered(protected),
        additional_restrictions,
        ..
    } = &protection.payload
    else {
        return false;
    };
    additional_restrictions.is_empty() && protected == &reduction.filter
}

fn is_source_line_base_pt_grant_loss_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    if abilities.len() != 3 {
        return false;
    }
    let mut base_filter = None;
    let mut removal_filter = None;
    let mut grant_filter = None;
    for ability in abilities {
        match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::SetBasePowerToughness {
                filter, ..
            } if base_filter.is_none() => base_filter = Some(filter),
            crate::static_abilities::StaticAbilityPayload::RemoveAllAbilities(filter)
                if removal_filter.is_none() =>
            {
                removal_filter = Some(filter);
            }
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(spec)
                if grant_filter.is_none()
                    && spec.condition.is_none()
                    && spec.additional_abilities.is_empty()
                    && spec.set_quantifier_surface.is_none()
                    && matches!(&spec.ability.kind, ironsmith_core::AbilityKind::Static(_)) =>
            {
                grant_filter = Some(&spec.filter);
            }
            _ => return false,
        }
    }
    matches!(
        (base_filter, removal_filter, grant_filter),
        (Some(base), Some(removal), Some(grant)) if base == removal && base == grant
    )
}

fn is_source_line_grant_all_other_loss_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, second] = abilities else {
        return false;
    };
    let mut removal_filter = None;
    let mut grant_filter = None;
    for ability in [first, second] {
        match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::RemoveAllAbilities(filter)
                if removal_filter.is_none() =>
            {
                removal_filter = Some(filter);
            }
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(spec)
                if grant_filter.is_none()
                    && spec.condition.is_none()
                    && spec.additional_abilities.is_empty() =>
            {
                grant_filter = Some(&spec.filter);
            }
            _ => return false,
        }
    }
    matches!(
        (removal_filter, grant_filter),
        (Some(removal), Some(grant)) if removal == grant
    )
}

fn is_source_line_type_addition_grant_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, grants @ ..] = abilities else {
        return false;
    };
    let (affected_filter, shared_condition) = match &first.payload {
        crate::static_abilities::StaticAbilityPayload::AddCardTypes { filter, card_types }
            if !card_types.is_empty() =>
        {
            (filter, None)
        }
        crate::static_abilities::StaticAbilityPayload::AddSubtypes { filter, subtypes }
            if !subtypes.is_empty() =>
        {
            (filter, None)
        }
        crate::static_abilities::StaticAbilityPayload::Conditional { ability, condition } => {
            match &ability.payload {
                crate::static_abilities::StaticAbilityPayload::AddCardTypes {
                    filter,
                    card_types,
                } if !card_types.is_empty() => (filter, Some(condition)),
                crate::static_abilities::StaticAbilityPayload::AddSubtypes { filter, subtypes }
                    if !subtypes.is_empty() =>
                {
                    (filter, Some(condition))
                }
                _ => return false,
            }
        }
        _ => return false,
    };
    !grants.is_empty()
        && grants.iter().all(|ability| match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::GrantAbility(grant) => {
                &grant.filter == affected_filter
                    && grant.condition.as_ref() == shared_condition
                    && grant.set_quantifier_surface.is_none()
                    && matches!(&grant.ability.kind, ironsmith_core::AbilityKind::Static(_))
            }
            crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                &grant.filter == affected_filter
                    && grant.condition.as_ref() == shared_condition
                    && grant.additional_abilities.is_empty()
                    && grant.set_quantifier_surface.is_none()
            }
            _ => false,
        })
}

/// A leading turn condition can be parsed on the quoted ability grant while
/// the coordinated type addition is lowered separately. Reattach that same
/// executable condition to the type-changing member only when the authored
/// line and the complete two-member shared-filter shape prove it applies to
/// both halves.
fn bind_leading_during_your_turn_to_type_addition_group(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(2).any(|window| window == ["and", "have"]) {
        return;
    }
    let [first, second] = abilities else {
        return;
    };
    let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
        &second.payload
    else {
        return;
    };
    let Some(condition) = grant.condition.clone() else {
        return;
    };
    if condition
        != crate::effect::Condition::ActivationTiming(
            crate::ability::ActivationTiming::DuringYourTurn,
        )
        || !grant.additional_abilities.is_empty()
        || grant.set_quantifier_surface.is_some()
        || !matches!(
            &grant.ability.kind,
            crate::ability::AbilityKind::Activated(_)
        )
    {
        return;
    }
    let affected_filter = match &first.payload {
        crate::static_abilities::StaticAbilityPayload::AddCardTypes { filter, card_types }
            if !card_types.is_empty() =>
        {
            filter
        }
        crate::static_abilities::StaticAbilityPayload::AddSubtypes { filter, subtypes }
            if !subtypes.is_empty() =>
        {
            filter
        }
        _ => return,
    };
    let mut semantic_affected_filter = affected_filter.clone();
    semantic_affected_filter.union_surface = Default::default();
    let mut semantic_grant_filter = grant.filter.clone();
    semantic_grant_filter.union_surface = Default::default();
    if semantic_affected_filter != semantic_grant_filter {
        return;
    }
    *first = first.clone().with_condition(condition);
}

/// An attached-land reset is authored as one line but intentionally lowers to
/// four independent layer-six pieces: clear land subtypes, remove abilities,
/// then grant each quoted activated ability. Retain only the group boundary;
/// the runtime renderer still has to prove the complete executable shape.
fn is_source_line_attached_land_reset_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [subtype_reset, ability_loss, first_grant, second_grant] = abilities else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::SetLandSubtypes {
        filter: subtype_filter,
        subtypes,
    } = &subtype_reset.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::RemoveAllAbilities(ability_filter) =
        &ability_loss.payload
    else {
        return false;
    };
    let expected_filter = ObjectFilter::land().match_tagged(
        "enchanted",
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );
    if !subtypes.is_empty()
        || subtype_filter != ability_filter
        || subtype_filter != &expected_filter
    {
        return false;
    }

    [first_grant, second_grant].iter().all(|ability| {
        matches!(
            &ability.payload,
            crate::static_abilities::StaticAbilityPayload::AttachedAbilityGrant(grant)
                if grant.condition.is_none()
                    && grant.additional_abilities.is_empty()
                    && matches!(&grant.ability.kind, ironsmith_core::AbilityKind::Activated(_))
        )
    })
}

fn conditional_static_parts(
    ability: &crate::static_abilities::StaticAbility,
) -> Option<(
    &crate::static_abilities::StaticAbility,
    &crate::effect::Condition,
)> {
    let crate::static_abilities::StaticAbilityPayload::Conditional { ability, condition } =
        &ability.payload
    else {
        return None;
    };
    Some((ability, condition))
}

fn is_prevent_damage_rule_restriction(ability: &crate::static_abilities::StaticAbility) -> bool {
    matches!(
        &ability.payload,
        crate::static_abilities::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::PreventDamage,
            additional_restrictions,
            ..
        } if additional_restrictions.is_empty()
    )
}

/// A single conditional spell line can carry both casting and resolution
/// restrictions: "this spell can't be countered and the damage can't be
/// prevented." Keep those typed siblings grouped and give the damage rider
/// the same spell-zone lifetime as the counterability restriction.
fn is_source_line_conditional_spell_protection_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [first, second] = abilities else {
        return false;
    };
    let Some((first_inner, first_condition)) = conditional_static_parts(first) else {
        return false;
    };
    let Some((second_inner, second_condition)) = conditional_static_parts(second) else {
        return false;
    };
    if first_condition != second_condition {
        return false;
    }
    let is_uncounterable = |ability: &crate::static_abilities::StaticAbility| {
        ability.id == Some(crate::static_abilities::StaticAbilityId::CantBeCountered)
    };
    (is_uncounterable(first_inner) && is_prevent_damage_rule_restriction(second_inner))
        || (is_uncounterable(second_inner) && is_prevent_damage_rule_restriction(first_inner))
}

fn is_conditional_spell_damage_prevention(
    ability: &crate::static_abilities::StaticAbility,
) -> bool {
    conditional_static_parts(ability)
        .is_some_and(|(inner, _)| is_prevent_damage_rule_restriction(inner))
}

/// A shared commander condition can be lowered once around a self anthem and
/// once on a filtered keyword grant. Keep the two executable continuous
/// effects grouped so the authored ability-word line can be reconstructed.
fn is_source_line_conditional_self_anthem_keyword_grant_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [anthem, grant] = abilities else {
        return false;
    };
    let Some((anthem_inner, anthem_condition)) = conditional_static_parts(anthem) else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem_payload) =
        &anthem_inner.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant_payload) =
        &grant.payload
    else {
        return false;
    };
    anthem_payload.filter.is_none()
        && grant_payload.condition.as_ref() == Some(anthem_condition)
        && grant_payload.additional_abilities.is_empty()
        && matches!(
            &grant_payload.ability.kind,
            crate::ability::AbilityKind::Static(static_ability) if static_ability.id().is_keyword()
        )
}

/// A Lieutenant-style source line can lower into three layer-correct static
/// components: a source anthem, an anthem for the other matching creatures,
/// and a keyword grant to that same filtered set. Preserve the source-line
/// marker only when all three carry the identical commander condition.
fn is_source_line_conditional_self_and_other_anthem_keyword_grant_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [self_anthem, other_anthem, grant] = abilities else {
        return false;
    };
    let Some((self_inner, condition)) = conditional_static_parts(self_anthem) else {
        return false;
    };
    let Some((other_inner, other_condition)) = conditional_static_parts(other_anthem) else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(self_payload) = &self_inner.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(other_payload) = &other_inner.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant_payload) =
        &grant.payload
    else {
        return false;
    };
    condition == other_condition
        && self_payload.filter.is_none()
        && other_payload.filter.as_ref() == Some(&grant_payload.filter)
        && grant_payload.condition.as_ref() == Some(condition)
        && grant_payload.additional_abilities.is_empty()
        && matches!(
            &grant_payload.ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id().is_keyword()
        )
}

/// Preserve one authored source line when an intrinsic conditioned source
/// anthem is followed by either a conditioned source keyword grant or a
/// direct blocking-capacity rule under the exact same typed condition. The
/// executable layer components remain independent.
fn is_source_line_conditioned_source_anthem_trait_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [anthem, companion] = abilities else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem_payload) = &anthem.payload
    else {
        return false;
    };
    let Some(condition) = anthem_payload.condition.as_ref() else {
        return false;
    };
    if anthem_payload.filter.is_some()
        || anthem_payload.set_quantifier_surface.is_some()
        || anthem_payload.count_uses_where_x
        || anthem_payload.replacement_surface.is_some()
    {
        return false;
    }

    match &companion.payload {
        crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            grant.filter == ObjectFilter::source()
                && grant.condition.as_ref() == Some(condition)
                && grant.additional_abilities.is_empty()
                && grant.set_quantifier_surface.is_none()
                && matches!(
                    &grant.ability.kind,
                    crate::ability::AbilityKind::Static(granted)
                        if granted.id.is_some_and(|id| id.is_keyword())
                )
        }
        crate::static_abilities::StaticAbilityPayload::Conditional {
            ability,
            condition: companion_condition,
        } => {
            companion_condition == condition
                && matches!(
                    &ability.payload,
                    crate::static_abilities::StaticAbilityPayload::CantBeBlockedByMoreThan(_)
                )
        }
        _ => false,
    }
}

/// A shared-subject creature line can lower into three independently
/// executable continuous effects: a keyword grant, an attack requirement,
/// and a count-scaled anthem. Preserve the authored line boundary only for
/// that exact typed trio so the compiled-text renderer can safely restore the
/// single grammatical subject.
fn is_source_line_keyword_attack_dynamic_anthem_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [keyword, attack, anthem] = abilities else {
        return false;
    };

    let grant_view = |ability: &crate::static_abilities::StaticAbility| match &ability.payload {
        crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none()
                && grant.additional_abilities.is_empty()
                && grant.set_quantifier_surface.is_none() =>
        {
            let crate::ability::AbilityKind::Static(granted) = &grant.ability.kind else {
                return None;
            };
            Some((grant.filter.clone(), granted.id))
        }
        crate::static_abilities::StaticAbilityPayload::GrantAbility(grant)
            if grant.condition.is_none() && grant.set_quantifier_surface.is_none() =>
        {
            let crate::ability::AbilityKind::Static(granted) = &grant.ability.kind else {
                return None;
            };
            Some((grant.filter.clone(), granted.id))
        }
        _ => None,
    };

    let Some((keyword_filter, keyword_id)) = grant_view(keyword) else {
        return false;
    };
    let Some((attack_filter, attack_id)) = grant_view(attack) else {
        return false;
    };
    if keyword_id != Some(crate::static_abilities::StaticAbilityId::Trample)
        || attack_id != Some(crate::static_abilities::StaticAbilityId::MustAttack)
        || keyword_filter != attack_filter
    {
        return false;
    }

    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) = &anthem.payload else {
        return false;
    };
    anthem.filter.as_ref() == Some(&keyword_filter)
        && anthem.condition.is_none()
        && anthem.set_quantifier_surface == Some(ironsmith_core::SetQuantifierSurface::Each)
        && anthem.count_uses_where_x
        && !anthem.additional_surface
        && anthem.replacement_surface.is_none()
        && matches!(
            (&anthem.power, &anthem.toughness),
            (
                ironsmith_core::AnthemValue::PerCount { multiplier: 1, .. },
                ironsmith_core::AnthemValue::Fixed(0)
            )
        )
}

/// A characteristic-defining line can lower into one independently executable
/// layer component for type, base power/toughness, and subtype. Keep the
/// authored line boundary only when all three components affect the source
/// under the same off-battlefield condition.
fn is_source_line_off_battlefield_creature_characteristic_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [type_ability, pt_ability, subtype_ability] = abilities else {
        return false;
    };
    let Some((type_ability, condition)) = conditional_static_parts(type_ability) else {
        return false;
    };
    let Some((pt_ability, pt_condition)) = conditional_static_parts(pt_ability) else {
        return false;
    };
    let Some((subtype_ability, subtype_condition)) = conditional_static_parts(subtype_ability)
    else {
        return false;
    };
    if condition != pt_condition
        || condition != subtype_condition
        || !matches!(
            condition,
            crate::effect::Condition::Not(inner)
                if matches!(inner.as_ref(), crate::effect::Condition::SourceIsInZone(crate::zone::Zone::Battlefield))
        )
    {
        return false;
    }

    let crate::static_abilities::StaticAbilityPayload::AddCardTypes {
        filter: type_filter,
        card_types,
    } = &type_ability.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::SetBasePowerToughness {
        filter: pt_filter,
        ..
    } = &pt_ability.payload
    else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::AddSubtypes {
        filter: subtype_filter,
        subtypes,
    } = &subtype_ability.payload
    else {
        return false;
    };

    card_types.as_slice() == [crate::types::CardType::Creature]
        && subtypes.len() == 1
        && type_filter == pt_filter
        && type_filter == subtype_filter
        && type_filter.source
}

fn source_line_ability_word_label(info: &LineInfo) -> Option<String> {
    let split =
        crate::runtime_backend::grammar::document_shapes::parse_statement_label_split_tokens(
            &info.source_tokens,
        )?;
    if crate::runtime_backend::grammar::document_shapes::parse_preserved_keyword_label_tokens(
        split.label_tokens,
    )
    .is_some()
    {
        return None;
    }
    let label = crate::runtime_backend::lexer::render_token_slice(split.label_tokens)
        .trim()
        .to_string();
    (!label.is_empty()).then_some(label)
}

fn is_source_line_chosen_object_anthem_keyword_grant_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [anthem, grant] = abilities else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) = &anthem.payload else {
        return false;
    };
    let Some(anthem_filter) = anthem.filter.as_ref() else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
        &grant.payload
    else {
        return false;
    };
    let [constraint] = anthem_filter.tagged_constraints.as_slice() else {
        return false;
    };
    let mut semantic_filter = anthem_filter.clone();
    semantic_filter.tagged_constraints.clear();
    semantic_filter.union_surface = Default::default();
    anthem_filter == &grant.filter
        && constraint.tag.as_str() == ironsmith_core::CHOSEN_OBJECTS_TAG
        && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        && semantic_filter == ObjectFilter::creature().in_zone(Zone::Battlefield)
        && anthem.condition.is_none()
        && anthem.set_quantifier_surface.is_none()
        && !anthem.count_uses_where_x
        && !anthem.additional_surface
        && anthem.replacement_surface.is_none()
        && grant.condition.is_none()
        && grant.set_quantifier_surface.is_none()
        && grant.additional_abilities.is_empty()
        && matches!(
            &grant.ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id().is_keyword()
    )
}

fn is_source_line_first_spell_cost_reduction_and_flash_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    let [reduction_ability, flash_ability] = abilities else {
        return false;
    };
    let crate::static_abilities::StaticAbilityPayload::CostReduction(reduction) =
        &reduction_ability.payload
    else {
        return false;
    };
    if !reduction.filter.first_spell_cast_each_turn
        || reduction.filter.cast_by != Some(PlayerFilter::You)
        || reduction.condition.is_some()
        || reduction.per_target
        || reduction.characteristic_intersection.is_some()
    {
        return false;
    }
    let crate::static_abilities::StaticAbilityPayload::Grants(flash) = &flash_ability.payload else {
        return false;
    };
    **flash == crate::grant::GrantSpec::flash_to_spells_matching(reduction.filter.clone())
}

fn is_renderable_source_line_static_group(
    abilities: &[crate::static_abilities::StaticAbility],
) -> bool {
    is_source_line_static_loss_group(abilities)
        || is_source_line_anthem_keyword_loss_group(abilities)
        || is_source_line_grant_keyword_loss_group(abilities)
        || is_source_line_cast_activation_restriction_group(abilities)
        || is_source_line_spell_cost_reduction_counter_protection_group(abilities)
        || is_source_line_base_pt_grant_loss_group(abilities)
        || is_source_line_grant_all_other_loss_group(abilities)
        || is_source_line_type_addition_grant_group(abilities)
        || is_source_line_attached_land_reset_group(abilities)
        || is_source_line_conditional_spell_protection_group(abilities)
        || is_source_line_conditional_self_anthem_keyword_grant_group(abilities)
        || is_source_line_conditional_self_and_other_anthem_keyword_grant_group(abilities)
        || is_source_line_conditioned_source_anthem_trait_group(abilities)
        || is_source_line_keyword_attack_dynamic_anthem_group(abilities)
        || is_source_line_off_battlefield_creature_characteristic_group(abilities)
        || is_source_line_chosen_object_anthem_keyword_grant_group(abilities)
        || is_source_line_first_spell_cost_reduction_and_flash_group(abilities)
        || matches!(
            abilities,
            [anthem, reach, shadow]
                if matches!(
                    &anthem.payload,
                    crate::static_abilities::StaticAbilityPayload::Anthem(model)
                        if matches!(
                            (&model.power, &model.toughness),
                            (
                                ironsmith_core::AnthemValue::Fixed(1),
                                ironsmith_core::AnthemValue::Fixed(1)
                            )
                        )
                )
                    && matches!(
                        &reach.payload,
                        crate::static_abilities::StaticAbilityPayload::GrantAbility(grant)
                            if matches!(
                                &grant.ability.kind,
                                crate::ability::AbilityKind::Static(granted)
                                    if granted.id == Some(
                                        crate::static_abilities::StaticAbilityId::Reach
                                    )
                            )
                    )
                    && matches!(
                        &shadow.payload,
                        crate::static_abilities::StaticAbilityPayload::GrantAbility(grant)
                            if matches!(
                                &grant.ability.kind,
                                crate::ability::AbilityKind::Static(granted)
                                    if granted.id == Some(
                                        crate::static_abilities::StaticAbilityId::CanBlockAsThoughNoShadow
                                    )
                            )
                    )
        )
        || matches!(
            abilities,
            [permission, surcharge]
                if permission.id == Some(crate::static_abilities::StaticAbilityId::Grants)
                    && surcharge.id
                        == Some(crate::static_abilities::StaticAbilityId::CostIncrease)
        )
        || matches!(
            abilities,
            [restriction, permission]
                if restriction.id == Some(
                    crate::static_abilities::StaticAbilityId::PlayersCantSearch
                ) && permission.id == Some(
                    crate::static_abilities::StaticAbilityId::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn
                )
        )
}

/// Recover an authored fixed self-cost reduction followed by an `as long as`
/// draw-count threshold when the broad dynamic-cost parser has consumed the
/// threshold value as the reduction amount. The source syntax and the lowered
/// payload must agree on the same typed `MaxCardsDrawnThisTurn` expression;
/// unrelated dynamic reductions are left unchanged.
fn bind_fixed_reduction_to_as_long_as_draw_threshold(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.starts_with(&["this", "spell", "costs"])
        || !source_words_contain(source_tokens, &["less", "to", "cast", "as", "long", "as"])
    {
        return;
    }

    // Prefer the dedicated static-line parser's already-proven result. The
    // broad document route can lower this syntax as an unconditional dynamic
    // reduction before this final presentation pass; reusing the narrow
    // parser keeps the fixed reduction and draw threshold distinct.
    if let Ok(Some(reparsed)) =
        crate::runtime_backend::keyword_static::parse_spells_cost_modifier_line(source_tokens)
        && let crate::static_abilities::StaticAbilityPayload::ThisSpellCostReduction(
            reparsed_reduction,
        ) = &reparsed.payload
        && matches!(reparsed_reduction.amount.unhinted(), crate::effect::Value::Fixed(amount) if *amount > 0)
        && matches!(
            &reparsed_reduction.condition,
            crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
                condition: crate::effect::Condition::ValueComparison {
                    left,
                    operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                    right,
                },
                ..
            } if matches!(left.unhinted(), crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::You))
                && matches!(right.unhinted(), crate::effect::Value::Fixed(threshold) if *threshold > 0)
        )
        && let [ability] = abilities
        && let crate::static_abilities::StaticAbilityPayload::ThisSpellCostReduction(existing) =
            &ability.payload
        && existing.condition == crate::static_abilities::ThisSpellCostCondition::Always
        && existing.affinity_filter.is_none()
        && existing.alternative_cast.is_none()
        && matches!(
            existing.amount.unhinted(),
            crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::You)
        )
    {
        ability.payload = reparsed.payload;
        return;
    }

    let Some(costs_token) = source_tokens
        .iter()
        .position(|token| token.is_word("costs"))
    else {
        return;
    };
    let Some(parsed_cost) =
        crate::runtime_backend::grammar::leaf::parse_leaf_fixed_mana_cost_prefix_tokens(
            &source_tokens[costs_token + 1..],
        )
    else {
        return;
    };
    let [pip] = parsed_cost.cost.pips() else {
        return;
    };
    let [crate::mana::ManaSymbol::Generic(reduction)] = pip.as_slice() else {
        return;
    };
    if *reduction == 0 {
        return;
    }

    let Some(as_long_as_word) = words
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    else {
        return;
    };
    let Some(condition_token) =
        crate::runtime_backend::grammar::static_keyword_shapes::parse_word_token_offset(
            source_tokens,
            as_long_as_word + 3,
        )
    else {
        return;
    };
    let condition_tokens = &source_tokens[condition_token..];
    let parsed_condition =
        crate::runtime_backend::keyword_static::parse_static_condition_clause(condition_tokens)
            .ok();
    let parsed_threshold = parsed_condition
        .as_ref()
        .and_then(|condition| match condition {
            crate::effect::Condition::ValueComparison {
                left,
                operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                right,
            } if matches!(
                left.unhinted(),
                crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::You)
            ) =>
            {
                match right.unhinted() {
                    crate::effect::Value::Fixed(threshold) if *threshold > 0 => Some(*threshold),
                    _ => None,
                }
            }
            _ => None,
        });
    let lexical_threshold = words
        .iter()
        .position(|word| *word == "drawn")
        .and_then(|drawn| words.get(drawn + 1))
        .and_then(|word| {
            crate::runtime_backend::front_end::shared::util::parse_number_word_i32(word)
        })
        .filter(|threshold| *threshold > 0);
    let Some(threshold) = parsed_threshold.or(lexical_threshold) else {
        return;
    };
    let condition = parsed_condition
        .filter(|_| parsed_threshold.is_some())
        .unwrap_or_else(|| crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::You),
            operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(threshold),
        });
    let display = crate::runtime_backend::lexer::render_token_slice(condition_tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();

    let [ability] = abilities else {
        return;
    };
    let crate::static_abilities::StaticAbilityPayload::ThisSpellCostReduction(existing) =
        &ability.payload
    else {
        return;
    };
    if existing.condition != crate::static_abilities::ThisSpellCostCondition::Always
        || existing.affinity_filter.is_some()
        || existing.alternative_cast.is_some()
        || !matches!(
            existing.amount.unhinted(),
            crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::You)
        )
    {
        return;
    }

    ability.payload = crate::static_abilities::StaticAbilityPayload::ThisSpellCostReduction(
        crate::static_abilities::ThisSpellCostReduction::new(
            crate::effect::Value::Fixed(i32::from(*reduction)),
            crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
                condition,
                display,
            },
        ),
    );
}

/// Preserve the authored `additional` modifier on a typed anthem when a
/// compound static line is lowered through the broad anthem/trait family.
/// The executable delta is already correct; this records only the exact
/// source-backed presentation bit on the sole anthem member.
fn bind_authored_additional_anthem_surface(
    abilities: &mut [crate::static_abilities::StaticAbility],
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    if !(source_words_contain(source_tokens, &["gets", "an", "additional"])
        || source_words_contain(source_tokens, &["get", "an", "additional"])
        || source_words_contain(source_tokens, &["gets", "a", "additional"])
        || source_words_contain(source_tokens, &["get", "a", "additional"]))
    {
        return;
    }
    let anthem_indices = abilities
        .iter()
        .enumerate()
        .filter_map(|(index, ability)| {
            matches!(
                &ability.payload,
                crate::static_abilities::StaticAbilityPayload::Anthem(anthem)
                    if anthem.filter.is_none()
                        && !anthem.additional_surface
                        && anthem.replacement_surface.is_none()
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let [index] = anthem_indices.as_slice() else {
        return;
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) =
        &mut abilities[*index].payload
    else {
        return;
    };
    anthem.additional_surface = true;
}

fn lower_static_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbilities(abilities) = parsed else {
        unreachable!("static-abilities lowerer received mismatched chunk");
    };

    let mut lowered_abilities = Vec::new();
    let mut regular_abilities = Vec::new();
    for ability in abilities {
        match ability {
            StaticAbilityAst::AttachmentRestriction { filter, display } => {
                builder.aura_attach_filter = Some(filter);
                let _ = display;
            }
            StaticAbilityAst::KeywordAction(KeywordAction::Fuse) => {
                builder = builder.has_fuse();
            }
            other => regular_abilities.push(other),
        }
    }

    let abilities = match super::rewrite_lower_static_abilities_ast(regular_abilities) {
        Ok(abilities) => abilities,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    lowered_abilities.extend(abilities);
    lowered_abilities = lowered_abilities
        .into_iter()
        .map(|ability| rewrite_self_spell_cost_modifier(ability, &semantic_facts.static_ability))
        .collect();
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    bind_fixed_reduction_to_as_long_as_draw_threshold(&mut lowered_abilities, &info.source_tokens);
    bind_fixed_reduction_to_as_long_as_draw_threshold(&mut lowered_abilities, &authored_tokens);
    bind_authored_chosen_creature_static_filters(&mut lowered_abilities, &authored_tokens);
    bind_authored_spell_cost_filter_qualifiers(&mut lowered_abilities, &authored_tokens);
    bind_authored_named_token_static_filters(&mut lowered_abilities, &authored_tokens);
    bind_authored_additional_anthem_surface(&mut lowered_abilities, &info.source_tokens);
    bind_leading_during_your_turn_to_type_addition_group(
        &mut lowered_abilities,
        &info.source_tokens,
    );
    let conditional_spell_protection_group =
        is_source_line_conditional_spell_protection_group(&lowered_abilities);
    if is_renderable_source_line_static_group(&lowered_abilities) {
        let mut marker = crate::static_abilities::StaticAbility::source_line_static_group(
            lowered_abilities.len(),
        );
        let presentation_label = semantic_facts
            .static_ability
            .presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
            // Preprocessing strips an ability-word prefix before the static
            // family is selected. Recover it from the retained source tokens
            // only for this structurally proven, single-source-line group.
            .or_else(|| source_line_ability_word_label(info));
        if let Some(label) = presentation_label {
            marker.label = format!(
                "{}{label}",
                ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX
            );
        }
        builder = builder.with_ability(Ability::static_ability(marker).in_zones(Vec::new()));
    }
    let preserve_single_ability_label = lowered_abilities.len() == 1;
    let source_ability_word = source_line_ability_word_label(info);
    for mut ability in lowered_abilities {
        if ability.id() == crate::static_abilities::StaticAbilityId::Flash
            && ability.label.to_ascii_lowercase().contains("as though")
            && let Some(label) = source_ability_word.as_deref()
            && !ability.label.starts_with(&format!("{label} —"))
        {
            let mut chars = ability.label.chars();
            let body = chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default();
            ability.label = format!("{label} — {body}");
        }
        let spell_damage_prevention =
            conditional_spell_protection_group && is_conditional_spell_damage_prevention(&ability);
        let mut compiled = compile_static_ability_with_zones(
            ability,
            &semantic_facts.static_ability,
            preserve_single_ability_label,
        );
        if spell_damage_prevention {
            compiled = compiled.in_zones(vec![
                Zone::Hand,
                Zone::Stack,
                Zone::Graveyard,
                Zone::Exile,
                Zone::Library,
                Zone::Command,
            ]);
        }
        preserve_as_long_as_its_your_turn_static_surface(&mut compiled, &info.source_tokens);
        builder = builder.with_ability(compiled);
        fuse_pending_removed_counter_as_enters(&mut builder);
    }
    Ok(builder)
}

fn lower_parsed_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        semantic_facts,
        annotations,
        ..
    } = input;
    let NormalizedLineChunk::Ability(mut parsed_ability) = parsed else {
        unreachable!("ability lowerer received mismatched chunk");
    };

    // Canonical trigger recognition has already constructed this parsed
    // ability from typed event and result nodes. Materialize its prepared AST
    // directly and skip every legacy source-repair path below.
    if semantic_facts
        .triggered_ability
        .compiler_ability
        .is_some()
    {
        let parsed_ability = super::rewrite_lower_prepared_ability(parsed_ability)?;
        if let Some(effects_ast) = parsed_ability.effects_ast.as_ref().map(Vec::as_slice) {
            super::collect_tag_spans_from_effects_with_context(
                effects_ast,
                annotations,
                &info.normalized,
            );
        }
        return Ok(builder.with_ability(parsed_ability.into_runtime()));
    }

    // A runtime-backed prepared ability can predate a later semantic
    // reconciliation of its typed AST. Rebuild only these two exact authored
    // correlated programs from the intact physical line so the stale prepared
    // payload cannot overwrite the recovered collection/value provenance.
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    let authored_tail =
        crate::runtime_backend::grammar::semantic_lowering::parse_comma_split_tokens(
            &authored_tokens,
        )
        .map(|split| split.after);
    let source_tail = crate::runtime_backend::grammar::semantic_lowering::parse_comma_split_tokens(
        &info.source_tokens,
    )
    .map(|split| split.after);
    let reconciled_dynamic =
        crate::runtime_backend::semantic_line_parsing::
            dynamic_zone_change_group_token_creation_from_authored_trigger(&authored_tokens)?
            .or(crate::runtime_backend::semantic_line_parsing::
                dynamic_zone_change_group_token_creation_from_authored_trigger(
                    &info.source_tokens,
                )?);
    let reconciled_looked_hand = authored_tail
        .as_ref()
        .and_then(|tail| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                tail,
            )
        })
        .or_else(|| {
            source_tail.as_ref().and_then(|tail| {
                crate::runtime_backend::semantic_line_parsing::
                    exact_looked_hand_optional_cast_bundle(tail)
            })
        })
        .or_else(|| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                &authored_tokens,
            )
        })
        .or_else(|| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                &info.source_tokens,
            )
        });
    let reconciled_targeted_same_name_cast = authored_tail
        .as_ref()
        .and_then(|tail| {
            crate::runtime_backend::semantic_line_parsing::
                exact_target_same_name_graveyard_may_cast_bundle(tail)
        })
        .or_else(|| {
            source_tail.as_ref().and_then(|tail| {
                crate::runtime_backend::semantic_line_parsing::
                exact_target_same_name_graveyard_may_cast_bundle(tail)
            })
        });
    let reconciled_graveyard_copy_cast = authored_tail
        .as_ref()
        .and_then(|tail| {
            crate::runtime_backend::semantic_line_parsing::exact_graveyard_card_copy_cast_sequence(
                tail,
            )
        })
        .or_else(|| {
            source_tail.as_ref().and_then(|tail| {
                crate::runtime_backend::semantic_line_parsing::
                    exact_graveyard_card_copy_cast_sequence(tail)
            })
        });
    let reconciled_quantified_token_rules = authored_tail
        .as_ref()
        .map(|tail| {
            crate::runtime_backend::effect_sentences::
                parse_quantified_token_creation_with_embedded_rules(tail)
        })
        .transpose()?
        .flatten()
        .or(source_tail
            .as_ref()
            .map(|tail| {
                crate::runtime_backend::effect_sentences::
                        parse_quantified_token_creation_with_embedded_rules(tail)
            })
            .transpose()?
            .flatten());
    let reconciled_attacking_opponents_pump = if let Some(tail) = authored_tail.as_ref() {
        let words = crate::runtime_backend::lexer::parser_token_word_refs(tail);
        if words
            .windows(3)
            .any(|window| window == ["creatures", "attacking", "your"])
            && words.iter().any(|word| *word == "opponents")
            && words.iter().any(|word| *word == "planeswalkers")
            && words.windows(2).any(|window| window == ["they", "control"])
        {
            Some(crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(tail)?)
        } else {
            None
        }
    } else {
        None
    };
    let reconciled_library_origin =
        crate::runtime_backend::semantic_line_parsing::
            parse_library_origin_source_pump_unblockable_triggered_line(&authored_tokens)?
            .or(crate::runtime_backend::semantic_line_parsing::
                parse_library_origin_source_pump_unblockable_triggered_line(
                    &info.source_tokens,
                )?);
    let reconciled_library_effects = match reconciled_library_origin {
        Some(LineAst::Triggered {
            trigger, effects, ..
        }) => {
            parsed_ability.parsed.trigger_spec = Some(trigger);
            Some(effects)
        }
        _ => None,
    };
    let reconciled_effects = reconciled_dynamic
        .map(|effect| vec![effect])
        .or(reconciled_attacking_opponents_pump)
        .or(reconciled_looked_hand)
        .or(reconciled_targeted_same_name_cast)
        .or(reconciled_graveyard_copy_cast)
        .or(reconciled_library_effects)
        .or_else(|| reconciled_quantified_token_rules.map(|effect| vec![effect]));
    if let Some(effects) = reconciled_effects {
        parsed_ability.parsed.effects_ast = Some(effects);
        parsed_ability.prepared = None;
    }

    let restored_source_exiled_return = match parsed_ability.parsed.effects_ast.as_mut() {
        Some(effects) => restore_authored_source_exiled_return(effects, &info.source_tokens)?,
        None => false,
    };
    if restored_source_exiled_return {
        // The normalized prepared payload was built from the lossy generic
        // return. Force the ordinary prepared-ability lowerer to rebuild it
        // from the corrected typed AST.
        parsed_ability.prepared = None;
    }
    let parsed_ability = super::rewrite_lower_prepared_ability(parsed_ability)?;
    if let Some(effects_ast) = parsed_ability.effects_ast.as_ref().map(Vec::as_slice) {
        super::collect_tag_spans_from_effects_with_context(
            effects_ast,
            annotations,
            &info.normalized,
        );
    }
    let mut ability = parsed_ability.into_runtime();
    if let AbilityKind::Triggered(triggered) = &mut ability.kind {
        preserve_condition_qualified_stun_reminder(triggered, &info.raw_line);
    }
    if !preserve_exiled_last_time_counter_trigger_surface(&mut ability, &authored_tokens) {
        preserve_exiled_last_time_counter_trigger_surface(&mut ability, &info.source_tokens);
    }
    bind_attacking_group_counter_reference(&mut ability, &authored_tokens);
    if let AbilityKind::Static(static_ability) = &mut ability.kind {
        bind_fixed_reduction_to_as_long_as_draw_threshold(
            std::slice::from_mut(static_ability),
            &info.source_tokens,
        );
        bind_fixed_reduction_to_as_long_as_draw_threshold(
            std::slice::from_mut(static_ability),
            &authored_tokens,
        );
        bind_authored_chosen_creature_static_filters(
            std::slice::from_mut(static_ability),
            &authored_tokens,
        );
    }
    preserve_as_long_as_its_your_turn_static_surface(&mut ability, &info.source_tokens);
    reconcile_exactly_one_creature_intervening_condition(&mut ability, &info.source_tokens);
    if let AbilityKind::Triggered(triggered) = &mut ability.kind {
        bind_authored_otherwise_move_to_conditional_false(&mut triggered.effects, &authored_tokens);
        bind_aura_enters_sticker_pronoun_to_source(triggered, &info.source_tokens);
        dedupe_lowered_adjacent_target_declarations(&mut triggered.effects);
        bind_source_exiled_return_complement(&mut triggered.effects);
        reconcile_authored_source_exiled_return_runtime(
            &mut triggered.effects,
            &info.source_tokens,
        );
        normalize_singular_source_exiled_runtime_move(&mut triggered.effects);
        normalize_graveyard_card_copy_cast_program(&mut triggered.effects);
        preserve_separate_copy_instruction_surface(&mut triggered.effects, &authored_tokens);
        bind_each_opponent_sacrifice_failure_half_life(&mut triggered.effects, &authored_tokens);
        bind_dynamic_power_owner_exile_permission(
            &mut triggered.effects,
            &info.source_tokens,
            &info.raw_line,
        );
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut triggered.effects,
            Some(&info.source_tokens),
        );
        restore_authored_return_then_venture(triggered, &info.source_tokens);
        transport_plural_copy_retarget_into_delayed_trigger(&mut triggered.effects);
        transport_fixed_retarget_into_optional_copy(&mut triggered.effects);
        bind_exile_top_card_cast_attempt_and_fallback(&mut triggered.effects);
        bind_demonstrative_land_self_replacement_to_triggering_object(&mut triggered.effects);
        bind_unique_most_control_leader_to_controller_change(triggered);
        bind_equipped_attack_subject_to_result_pump(triggered);
        bind_combat_damage_group_controller_draw(triggered, &authored_tokens);
        bind_equipped_attack_draw_reveal_result(triggered, &info.source_tokens);
        bind_triggered_attachment_union_count(triggered, &authored_tokens);
        super::rebind_returned_attachment_history_to_triggering_object(
            &mut triggered.effects.segments,
        );
        bind_returned_card_to_hand_result_condition(&mut triggered.effects, &authored_tokens);
        bind_then_if_source_untap_and_transform(&mut triggered.effects, &authored_tokens);
        bind_attacking_opponents_result_pump(triggered, &authored_tokens);
        bind_later_attacker_choice_to_prior_target_power(triggered);
        bind_authored_single_target_spell_cast_filter(triggered, &info.source_tokens);
        bind_authored_spell_cast_color_list(triggered, &info.source_tokens);
        bind_authored_spell_cast_ability_marker(triggered, &info.source_tokens);
        bind_authored_spell_cast_relation_constraints(triggered, &info.source_tokens);
        bind_original_and_copy_plural_keyword_grant(triggered, &info.source_tokens);
        bind_reflexive_optional_cast_replacement_result(triggered);
        bind_authored_chosen_creature_sacrifice(triggered, &authored_tokens);
    }
    if let AbilityKind::Activated(activated) = &mut ability.kind {
        let authored_tokens =
            crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
                .unwrap_or_else(|_| info.source_tokens.clone());
        bind_target_player_lost_life_this_turn_qualifier(
            &mut activated.effects,
            &mut activated.choices,
            &authored_tokens,
        );
        bind_authored_source_and_each_opponent_creature_exile(
            &mut activated.effects,
            &authored_tokens,
        );
    }
    let program = match &mut ability.kind {
        AbilityKind::Triggered(triggered) => Some(&mut triggered.effects),
        AbilityKind::Activated(activated) => Some(&mut activated.effects),
        _ => None,
    };
    if let Some(program) = program {
        dedupe_lowered_adjacent_target_declarations(program);
        bind_source_exiled_return_complement(program);
        reconcile_authored_source_exiled_return_runtime(program, &info.source_tokens);
        normalize_graveyard_card_copy_cast_program(program);
        preserve_separate_copy_instruction_surface(program, &authored_tokens);
        bind_each_opponent_sacrifice_failure_half_life(program, &authored_tokens);
        bind_commander_hand_move_from_command_zone(program, &authored_tokens);
        bind_dynamic_power_owner_exile_permission(program, &info.source_tokens, &info.raw_line);
        bind_next_spell_other_opponent_copy_retarget_choice(program);
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            program,
            Some(&info.source_tokens),
        );
        fold_prior_result_self_replacement_into_success_arm(program, &semantic_facts.statement);
        bind_shared_conditional_counter_arms_to_declared_target(program);
    }
    if let AbilityKind::Activated(activated) = &mut ability.kind {
        let authored_tokens =
            crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
                .unwrap_or_else(|_| info.source_tokens.clone());
        bind_target_player_lost_life_this_turn_qualifier(
            &mut activated.effects,
            &mut activated.choices,
            &authored_tokens,
        );
    }
    if semantic_facts.triggered_ability.leading_unless_surface
        && let AbilityKind::Triggered(triggered) = &mut ability.kind
    {
        for effect in triggered
            .effects
            .segments
            .iter_mut()
            .flat_map(|segment| segment.default_effects.iter_mut())
        {
            if let Some(unless_pays) =
                effect.downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            {
                let mut preserved = unless_pays.clone();
                preserved.leading_surface = true;
                *effect = crate::effect::Effect::new(preserved);
            }
        }
    }
    if semantic_facts
        .statement
        .trailing_instead_if_predicate
        .is_some()
    {
        let program = match &mut ability.kind {
            AbilityKind::Triggered(triggered) => Some(&mut triggered.effects),
            AbilityKind::Activated(activated) => Some(&mut activated.effects),
            _ => None,
        };
        if let Some(program) = program {
            for branch in program
                .segments
                .iter_mut()
                .flat_map(|segment| segment.self_replacements.iter_mut())
            {
                branch.condition_after_replacement = true;
            }
        }
    }
    builder = builder.with_ability(ability);
    Ok(builder)
}

/// Recover an untargeted union when Oracle exiles the ability's source and a
/// quantified opponent-controlled creature set. The generic noun parser can
/// otherwise interpret the source's short name as a subtype and lower the
/// whole instruction as one singular object choice.
fn bind_authored_source_and_each_opponent_creature_exile(
    program: &mut crate::resolution::ResolutionProgram,
    authored_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(authored_tokens);
    let Some(exile_index) = words.iter().position(|word| *word == "exile") else {
        return false;
    };
    let Some(tail_index) = words
        .windows(6)
        .position(|window| window == ["and", "each", "creature", "your", "opponents", "control"])
    else {
        return false;
    };
    if tail_index <= exile_index + 1 || tail_index + 6 != words.len() {
        return false;
    }
    let Some(source_surface) =
        crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
            &words[exile_index + 1..tail_index],
        )
    else {
        return false;
    };

    let [segment] = program.segments.as_mut_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [effect] = segment.default_effects.as_mut_slice() else {
        return false;
    };
    let Some(move_to_zone) = effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .cloned()
    else {
        return false;
    };
    if move_to_zone.zone != Zone::Exile
        || !matches!(move_to_zone.target.unhinted(), ChooseSpec::Object(_))
    {
        return false;
    }

    let source = ObjectFilter::source_with_surface(source_surface);
    let mut creatures = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
    creatures.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
    let mut union = ObjectFilter::default();
    union.any_of = vec![source, creatures];
    union.set_conjunctive_set_surface(true);

    let mut corrected = move_to_zone;
    corrected.target = ChooseSpec::All(union);
    corrected.target_plural_surface = false;
    *effect = crate::effect::Effect::new(corrected);
    true
}

fn bind_authored_otherwise_move_to_conditional_false(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(6)
        .any(|window| window == ["otherwise", "put", "it", "into", "your", "hand"])
    {
        return false;
    }
    for segment in &mut program.segments {
        let mut index = 0usize;
        while index + 1 < segment.default_effects.len() {
            let Some(mut conditional) = segment.default_effects[index]
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .cloned()
            else {
                index += 1;
                continue;
            };
            let Some(hand_move) = segment.default_effects[index + 1]
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .cloned()
            else {
                index += 1;
                continue;
            };
            if !conditional.if_false.is_empty()
                || hand_move.zone != Zone::Hand
                || conditional.if_true.len() != 1
            {
                index += 1;
                continue;
            }
            let true_effect = &conditional.if_true[0];
            let true_effect = true_effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| tagged.effect.as_ref())
                .unwrap_or(true_effect);
            let Some(battlefield_move) =
                true_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            else {
                index += 1;
                continue;
            };
            if battlefield_move.zone != Zone::Battlefield
                || battlefield_move.target != hand_move.target
            {
                index += 1;
                continue;
            }
            conditional.if_false = vec![crate::effect::Effect::new(hand_move)];
            segment.default_effects[index] = crate::effect::Effect::new(conditional);
            segment.default_effects.remove(index + 1);
            return true;
        }
    }
    false
}

fn preserve_exiled_last_time_counter_trigger_surface(
    ability: &mut Ability,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let exact_surface = words
        .windows(7)
        .any(|window| window == ["last", "time", "counter", "is", "removed", "from", "this"])
        && (words
            .windows(3)
            .any(|window| window == ["while", "it", "s"])
            || words
                .windows(2)
                .any(|window| matches!(window, ["while", "its" | "it's"])))
        && words.iter().any(|word| *word == "exiled");
    if !exact_surface {
        return false;
    }
    let AbilityKind::Triggered(triggered) = &mut ability.kind else {
        return false;
    };
    let crate::triggers::TriggerKind::CounterRemovedFrom(counter_removed) = &triggered.trigger.kind
    else {
        return false;
    };
    if !counter_removed.last
        || counter_removed.counter_type != Some(crate::CounterType::Time)
        || !counter_removed.filter.source
    {
        return false;
    }
    triggered.intervening_if = None;
    ability.functional_zones = vec![Zone::Exile];
    for effect in triggered
        .effects
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>().cloned() else {
            continue;
        };
        let crate::effect::Restriction::BeBlocked(mut filter) = cant.restriction.clone() else {
            continue;
        };
        if filter.card_types == [crate::types::CardType::Creature] {
            filter.set_plural_object_noun_surface(true);
            let mut preserved = cant;
            preserved.restriction = crate::effect::Restriction::BeBlocked(filter);
            *effect = crate::effect::Effect::new(preserved);
        }
    }
    true
}

fn bind_attacking_group_counter_reference(
    ability: &mut Ability,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(6)
        .any(|window| window == ["attack", "with", "two", "or", "more", "non"])
        || !words
            .windows(5)
            .any(|window| window == ["on", "each", "of", "those", "creatures"])
    {
        return false;
    }
    let AbilityKind::Triggered(triggered) = &mut ability.kind else {
        return false;
    };
    let crate::triggers::TriggerKind::AttacksOneOrMoreWithMinTotal {
        min_total_attackers,
        ..
    } = &triggered.trigger.kind
    else {
        return false;
    };
    if *min_total_attackers != 2 {
        return false;
    }

    for segment in &mut triggered.effects.segments {
        for effect in &mut segment.default_effects {
            if let Some(mut for_each) = effect
                .downcast_ref::<crate::effects::ForEachObject>()
                .cloned()
                && for_each.effects.len() == 1
                && for_each.effects[0]
                    .downcast_ref::<crate::effects::PutCountersEffect>()
                    .is_some_and(|counters| {
                        counters.counter_type == crate::CounterType::PlusOnePlusOne
                            && counters.amount == crate::Value::Fixed(1)
                    })
            {
                for_each.filter =
                    ObjectFilter::tagged(crate::TagKey::from(ironsmith_core::ATTACKING_GROUP_TAG));
                *effect = crate::effect::Effect::new(for_each);
                return true;
            }
            let Some(mut counters) = effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .cloned()
            else {
                continue;
            };
            let crate::target::ChooseSpec::All(filter) = counters.target.unhinted() else {
                continue;
            };
            if filter.zone != Some(Zone::Battlefield)
                || filter.card_types != [crate::types::CardType::Creature]
                || counters.counter_type != crate::CounterType::PlusOnePlusOne
                || counters.amount != crate::Value::Fixed(1)
            {
                continue;
            }
            counters.target = crate::target::ChooseSpec::Tagged(crate::TagKey::from(
                ironsmith_core::ATTACKING_GROUP_TAG,
            ));
            *effect = crate::effect::Effect::new(counters);
            return true;
        }
    }
    false
}

/// Preserve the authored sentence break in `Exile ... . Copy that card. You
/// may cast the copy.` after the executable copy/cast normalization has
/// collapsed the latter two instructions into one optional CastTagged
/// effect. The exact renderer still proves the shared exile tag and
/// graveyard domain before using this presentation bit.
fn preserve_separate_copy_instruction_surface(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let authored_sentences = crate::runtime_backend::lexer::split_lexed_sentences(source_tokens);
    let has_permanent_copy_reminder = authored_sentences.iter().any(|sentence| {
        crate::runtime_backend::lexer::parser_token_word_refs(sentence).as_slice()
            == [
                "a",
                "copy",
                "of",
                "a",
                "permanent",
                "spell",
                "becomes",
                "a",
                "token",
            ]
    });
    let surface = authored_sentences
        .iter()
        .find_map(|sentence| {
            let words = crate::runtime_backend::lexer::parser_token_word_refs(sentence);
            if words.starts_with(&["copy", "that", "card"]) {
                return Some(ironsmith_core::effect::CopyInstructionSurface::SeparateThatCard);
            }
            if words.starts_with(&["copy", "it"]) {
                return Some(if words.get(2) == Some(&"then") {
                    if has_permanent_copy_reminder {
                        ironsmith_core::effect::CopyInstructionSurface::SeparateItThenPermanentCopyReminder
                    } else {
                        ironsmith_core::effect::CopyInstructionSurface::SeparateItThen
                    }
                } else {
                    ironsmith_core::effect::CopyInstructionSurface::SeparateIt
                });
            }
            None
        });
    let Some(surface) = surface else {
        return;
    };
    // Triggered and activated public routes can retain the exile and the
    // optional cast in separate resolution segments. Locate the one exact
    // optional copy instruction rather than requiring a one-segment shell;
    // the renderer still proves that its tag is the exile result tag.
    for segment in &mut program.segments {
        if !segment.self_replacements.is_empty() {
            continue;
        }
        for may_root in &mut segment.default_effects {
            let Some(mut may) = may_root
                .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
                .cloned()
            else {
                continue;
            };
            let [cast_root] = may.effects.as_mut_slice() else {
                continue;
            };
            let Some(mut cast) = cast_root
                .downcast_ref::<crate::effects::CastTaggedEffect>()
                .cloned()
            else {
                continue;
            };
            if !cast.as_copy {
                continue;
            }
            cast.copy_instruction_surface = Some(surface);
            *cast_root = crate::effect::Effect::new(cast);
            *may_root = crate::effect::Effect::new(may);
            return;
        }
    }
}

fn preserve_condition_qualified_stun_reminder(
    triggered: &mut crate::ability::TriggeredAbility,
    raw_line: &str,
) {
    if !raw_line.contains(
        "If a permanent with a stun counter would become untapped, remove one from it instead.",
    ) {
        return;
    }
    let ironsmith_core::TriggerKind::ConditionQualified {
        stun_counter_reminder_surface,
        ..
    } = &mut triggered.trigger.kind
    else {
        return;
    };
    *stun_counter_reminder_surface = true;
}

/// Reapply an authored command-zone origin after the generic "put ... into"
/// destination parser has separated the trailing source phrase from its
/// object filter. The target must already be the controller's commander, so
/// ordinary hand moves cannot acquire command-zone legality accidentally.
fn bind_commander_hand_move_from_command_zone(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(4)
        .any(|window| window == ["from", "the", "command", "zone"])
    {
        return;
    }
    for segment in &mut program.segments {
        for root in &mut segment.default_effects {
            let Some(mut move_to_zone) = root
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .cloned()
            else {
                continue;
            };
            if move_to_zone.zone != Zone::Hand {
                continue;
            }
            let ChooseSpec::Object(filter) = move_to_zone.target.base() else {
                continue;
            };
            if !filter.is_commander {
                continue;
            }
            let mut filter = filter.clone();
            filter.zone = Some(Zone::Command);
            filter.owner = Some(PlayerFilter::You);
            move_to_zone.target = ChooseSpec::Object(filter);
            *root = crate::effect::Effect::new(move_to_zone);
        }
    }
}

/// Preserve the correlated failure branch in an authored per-opponent
/// sacrifice instruction. The ordinary sentence path already compiles the
/// sacrifice correctly but can discard the second sentence before it is
/// linked to that action's result.
fn bind_each_opponent_sacrifice_failure_half_life(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let has_failure_clause = words
        .windows(3)
        .any(|window| window == ["each", "opponent", "who"])
        && (words
            .iter()
            .any(|word| matches!(*word, "cant" | "can't" | "cannot"))
            || words.windows(2).any(|window| window == ["can", "t"]))
        && words
            .windows(4)
            .any(|window| window == ["half", "their", "life", "rounded"])
        && words.iter().any(|word| *word == "up");
    if !has_failure_clause || program.segments.len() != 1 {
        return;
    }
    let fresh_id = crate::effect::EffectId(
        program
            .all_effects()
            .into_iter()
            .filter_map(max_effect_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    );
    let segment = &mut program.segments[0];
    let [root] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let (id, sacrifice_root) = if let Some(with_id) = root.as_with_id() {
        (with_id.id, with_id.effect.as_ref())
    } else {
        (fresh_id, &*root)
    };
    let Some(for_players) =
        sacrifice_root.downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
    else {
        return;
    };
    if for_players.filter != PlayerFilter::Opponent {
        return;
    }
    let sacrifice_id = for_players.effects.iter().find_map(|effect| {
        let with_id = effect.as_with_id()?;
        with_id
            .effect
            .downcast_ref::<crate::effects::SacrificePlayerEffect>()
            .map(|_| with_id.id)
    });
    if let Some(sacrifice_id) = sacrifice_id {
        let mut rebound = for_players.clone();
        let mut filled = false;
        for effect in &mut rebound.effects {
            let Some(mut conditional) = effect.downcast_ref::<crate::effects::IfEffect>().cloned()
            else {
                continue;
            };
            if conditional.condition != sacrifice_id
                || conditional.predicate != crate::effect::EffectPredicate::DidNotHappen
                || !conditional.then.is_empty()
                || !conditional.else_.is_empty()
            {
                continue;
            }
            conditional.then.push(crate::effect::Effect::new(
                crate::effects::LoseLifeEffect::new(
                    crate::effect::Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer),
                    PlayerFilter::IteratedPlayer,
                ),
            ));
            *effect = crate::effect::Effect::new(conditional);
            filled = true;
        }
        if filled {
            *root = crate::effect::Effect::new(rebound);
            return;
        }
    }
    if !for_players.effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::SacrificePlayerEffect>()
            .is_some()
    }) {
        return;
    }
    if root.as_with_id().is_none() {
        let antecedent = root.clone();
        *root = crate::effect::Effect::with_id(id.0, antecedent);
    }
    let failure = crate::effects::ForPlayersEffect {
        filter: PlayerFilter::Opponent,
        effects: vec![crate::effect::Effect::new(crate::effects::IfEffect::new(
            id,
            crate::effect::EffectPredicate::DidNotHappen,
            vec![crate::effect::Effect::new(
                crate::effects::LoseLifeEffect::new(
                    crate::effect::Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer),
                    PlayerFilter::IteratedPlayer,
                ),
            )],
            vec![],
        ))],
        starting_with_controller: false,
        stop_after_first_happened: false,
    };
    segment
        .default_effects
        .push(crate::effect::Effect::new(failure));
}

/// Keep the one-shot zone replacement in a reflexive targeted graveyard cast
/// linked to the optional cast result. Some multi-sentence lowering routes
/// allocate the replacement condition before the surrounding `May` receives
/// its final effect id. The target/cast/replacement tags prove the intended
/// producer without relying on a card name or authored wording.
fn bind_reflexive_optional_cast_replacement_result(
    triggered: &mut crate::ability::TriggeredAbility,
) {
    for root in triggered
        .effects
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        let Some(mut reflexive) = root
            .downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
            .cloned()
        else {
            continue;
        };
        if reflexive.predicate != crate::effect::EffectPredicate::Happened {
            continue;
        }
        let [choice] = reflexive.choices.as_slice() else {
            continue;
        };
        let [target_root, may_root, if_root] = reflexive.effects.as_slice() else {
            continue;
        };
        let Some(target_tagged) = target_root.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        let Some(target_only) = target_tagged
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        else {
            continue;
        };
        let Some(may_with_id) = may_root.downcast_ref::<crate::effects::WithIdEffect>() else {
            continue;
        };
        let Some(may) = may_with_id
            .effect
            .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        else {
            continue;
        };
        let [cast_root] = may.effects.as_slice() else {
            continue;
        };
        let Some(cast_tagged) = cast_root.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        let Some(cast) = cast_tagged
            .effect
            .downcast_ref::<crate::effects::CastTaggedEffect>()
        else {
            continue;
        };
        let Some(if_effect) = if_root.downcast_ref::<crate::effects::IfEffect>() else {
            continue;
        };
        let [replacement_root] = if_effect.then.as_slice() else {
            continue;
        };
        let Some(replacement) =
            replacement_root.downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()
        else {
            continue;
        };
        let exact_cast_tag = replacement
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == cast_tagged.tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            });
        if &target_only.target != choice
            || cast.tag != target_tagged.tag
            || cast.player != PlayerFilter::You
            || cast.allow_land
            || cast.as_copy
            || if_effect.predicate != crate::effect::EffectPredicate::Happened
            || !if_effect.else_.is_empty()
            || replacement.from_zone != Some(Zone::Stack)
            || replacement.to_zone != Some(Zone::Graveyard)
            || replacement.replacement_zone != Zone::Exile
            || replacement.mode != crate::effects::ReplacementApplyMode::OneShot
            || replacement.filter.zone != Some(Zone::Stack)
            || !exact_cast_tag
        {
            continue;
        }

        let linked_id = may_with_id.id;
        let mut linked_if = if_effect.clone();
        linked_if.condition = linked_id;
        reflexive.effects[2] = crate::effect::Effect::new(linked_if);
        *root = crate::effect::Effect::new(reflexive);
    }
}

/// Reconcile two conditional counter arms that both refer to one declared
/// target. Generic sequential reference carry can bind the second authored
/// `it` to the result tag of the first counter placement. The exact target
/// declaration and complementary creature/planeswalker filters prove that
/// both arms instead test the same object.
fn bind_shared_conditional_counter_arms_to_declared_target(
    program: &mut crate::resolution::ResolutionProgram,
) {
    let [target_segment, counters_segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !target_segment.self_replacements.is_empty()
        || !counters_segment.self_replacements.is_empty()
        || target_segment.starts_new_source_line
        || counters_segment.starts_new_source_line
    {
        return;
    }
    let [target_root] = target_segment.default_effects.as_slice() else {
        return;
    };
    let Some(target_tagged) = target_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(target_only) = target_tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return;
    };
    let ChooseSpec::Target(target_spec) = target_only.target.unhinted() else {
        return;
    };
    let ChooseSpec::Object(target_filter) = target_spec.unhinted() else {
        return;
    };
    if target_filter.zone != Some(Zone::Battlefield)
        || !target_filter.is_commander
        || !target_filter.entered_battlefield_this_turn
    {
        return;
    }

    let [sequence_root] = counters_segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(sequence) = sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .cloned()
    else {
        return;
    };
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return;
    }
    let [creature_root, planeswalker_root] = sequence.effects.as_slice() else {
        return;
    };
    let Some(creature_tagged) = creature_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(planeswalker_tagged) =
        planeswalker_root.downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let Some(creature_counters) = creature_tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
    else {
        return;
    };
    let Some(planeswalker_counters) = planeswalker_tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
    else {
        return;
    };
    let ChooseSpec::Object(creature_filter) = creature_counters.target.unhinted() else {
        return;
    };
    let ChooseSpec::Object(planeswalker_filter) = planeswalker_counters.target.unhinted() else {
        return;
    };
    let creature_link = creature_filter.tagged_constraints.as_slice();
    let planeswalker_link = planeswalker_filter.tagged_constraints.as_slice();
    if creature_counters.counter_type != crate::CounterType::PlusOnePlusOne
        || planeswalker_counters.counter_type != crate::CounterType::Loyalty
        || creature_counters.amount.unhinted() != &crate::effect::Value::Fixed(1)
        || planeswalker_counters.amount.unhinted() != &crate::effect::Value::Fixed(1)
        || creature_counters.target_count.is_some()
        || planeswalker_counters.target_count.is_some()
        || creature_counters.distributed
        || planeswalker_counters.distributed
        || creature_filter.zone != Some(Zone::Battlefield)
        || creature_filter.card_types.as_slice() != [crate::types::CardType::Creature]
        || planeswalker_filter.zone != Some(Zone::Battlefield)
        || planeswalker_filter.card_types.as_slice() != [crate::types::CardType::Planeswalker]
        || !matches!(
            creature_link,
            [constraint]
                if constraint.tag == target_tagged.tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        )
        || !matches!(
            planeswalker_link,
            [constraint]
                if constraint.tag == creature_tagged.tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        )
    {
        return;
    }

    let mut rewritten_filter = planeswalker_filter.clone();
    rewritten_filter.tagged_constraints[0].tag = target_tagged.tag.clone();
    let mut rewritten_put = planeswalker_counters.clone();
    rewritten_put.target = ChooseSpec::Object(rewritten_filter);
    let mut rewritten_tagged = planeswalker_tagged.clone();
    rewritten_tagged.effect = Box::new(crate::effect::Effect::new(rewritten_put));
    let mut rewritten_sequence = sequence;
    rewritten_sequence.effects[1] = crate::effect::Effect::new(rewritten_tagged);
    *sequence_root = crate::effect::Effect::new(rewritten_sequence);
}

fn preserve_as_long_as_its_your_turn_static_surface(
    ability: &mut Ability,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !matches!(
        words.as_slice(),
        ["as", "long", "as", "it", "s", "your", "turn", ..]
            | ["as", "long", "as", "its", "your", "turn", ..]
    ) {
        return;
    }
    let AbilityKind::Static(static_ability) = &mut ability.kind else {
        return;
    };
    let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
        &static_ability.payload
    else {
        return;
    };
    if !grant.filter.source
        || grant.condition != Some(crate::ConditionExpr::YourTurn)
        || !matches!(
            &grant.ability.kind,
            AbilityKind::Static(granted)
                if granted.id == Some(crate::static_abilities::StaticAbilityId::FirstStrike)
        )
    {
        return;
    }
    static_ability.label = format!(
        "{}{}",
        ironsmith_core::static_ability_model::AS_LONG_AS_ITS_YOUR_TURN_STATIC_LABEL_PREFIX,
        static_ability.label
    );
}

/// Reconcile the exact-count condition when a broad public trigger path has
/// already reduced the authored `control exactly one creature` predicate to a
/// presence check. The retained source phrase and typed creature filter prove
/// the narrower executable condition without relying on the card name.
fn reconcile_exactly_one_creature_intervening_condition(
    ability: &mut Ability,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::token_word_refs(source_tokens);
    if !words
        .windows(6)
        .any(|window| window == ["if", "you", "control", "exactly", "one", "creature"])
    {
        return;
    }
    let AbilityKind::Triggered(triggered) = &mut ability.kind else {
        return;
    };
    let Some(crate::ConditionExpr::PlayerControls { player, filter }) =
        triggered.intervening_if.as_ref()
    else {
        return;
    };
    if player != &PlayerFilter::You
        || filter.card_types.as_slice() != [crate::types::CardType::Creature]
    {
        return;
    }
    triggered.intervening_if = Some(crate::ConditionExpr::PlayerControlsExactly {
        player: player.clone(),
        filter: filter.clone(),
        count: 1,
    });
}

/// A unique-most intervening condition proves that exactly one player can be
/// the authored controller in "the player who controls the most ... gains
/// control". Bind that player into the executable controller change instead
/// of leaving the imperative fallback as the effect controller.
fn bind_unique_most_control_leader_to_controller_change(
    triggered: &mut crate::ability::TriggeredAbility,
) {
    let Some(crate::effect::Condition::PlayerControlsMoreThanEachOtherPlayer { player, filter }) =
        triggered.intervening_if.as_ref()
    else {
        return;
    };
    if *player != PlayerFilter::Any || !triggered.choices.is_empty() {
        return;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() || segment.starts_new_source_line {
        return;
    }
    let [effect] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return;
    };
    if apply.target != crate::continuous::EffectTarget::Source
        || apply.until != crate::effect::Until::Forever
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.condition.is_some()
        || apply.runtime_modifications.as_slice()
            != [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
    {
        return;
    }
    let mut rebound = apply.clone();
    rebound.runtime_modifications = vec![
        crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
            PlayerFilter::ControlsMost {
                filter: Box::new(filter.clone()),
            },
        ),
    ];
    *effect = crate::effect::Effect::new(rebound);
}

/// Rebind an anaphoric pump in an equipped-creature attack trigger to the
/// attacking creature captured by the trigger. A prior optional land return
/// must not make the later `the creature` fall back to every creature on the
/// battlefield.
fn bind_equipped_attack_subject_to_result_pump(triggered: &mut crate::ability::TriggeredAbility) {
    let ironsmith_core::TriggerKind::Attacks { filter } = &triggered.trigger.kind else {
        return;
    };
    let mut expected_attacker = ObjectFilter::creature();
    expected_attacker.set_explicit_card_type_noun(Some(crate::types::CardType::Creature));
    expected_attacker = expected_attacker
        .match_tagged(
            ironsmith_core::TagKey::from("equipped"),
            ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
        )
        .in_zone(Zone::Battlefield);
    if filter != &expected_attacker || !triggered.choices.is_empty() {
        return;
    }

    let [return_segment, result_segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !return_segment.self_replacements.is_empty()
        || return_segment.starts_new_source_line
        || !result_segment.self_replacements.is_empty()
        || result_segment.starts_new_source_line
    {
        return;
    }
    let [return_effect] = return_segment.default_effects.as_slice() else {
        return;
    };
    let Some(with_id) = return_effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    let Some(may) = with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    let [return_effect] = may.effects.as_slice() else {
        return;
    };
    let Some(return_to_hand) = return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
    else {
        return;
    };
    let ChooseSpec::WithCount(return_spec, count) = return_to_hand.spec.unhinted() else {
        return;
    };
    let ChooseSpec::Object(return_filter) = return_spec.unhinted() else {
        return;
    };
    let mut expected_land = ObjectFilter::land().controlled_by(PlayerFilter::You);
    expected_land.set_explicit_card_type_noun(Some(crate::types::CardType::Land));
    if count != &ironsmith_core::ChoiceCount::exactly(1) || return_filter != &expected_land {
        return;
    }

    let [result_effect] = result_segment.default_effects.as_slice() else {
        return;
    };
    let Some(result_if) = result_effect.downcast_ref::<crate::effects::IfEffect>() else {
        return;
    };
    if result_if.condition != with_id.id
        || result_if.predicate != crate::effect::EffectPredicate::Happened
        || !result_if.else_.is_empty()
        || result_if.per_player_result
        || result_if.prior_result_replacement_surface
    {
        return;
    }
    let [pumped_effect] = result_if.then.as_slice() else {
        return;
    };
    let Some(tagged) = pumped_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(apply) = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return;
    };
    let mut expected_creature = ObjectFilter::creature();
    expected_creature.set_explicit_card_type_noun(Some(crate::types::CardType::Creature));
    if apply.target != crate::continuous::EffectTarget::Filter(expected_creature)
        || apply.target_spec.is_some()
        || apply.until != crate::effect::Until::EndOfTurn
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.condition.is_some()
        || apply.runtime_modifications.as_slice()
            != [
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: crate::effect::Value::Fixed(2),
                    toughness: crate::effect::Value::Fixed(2),
                },
            ]
    {
        return;
    }

    let equipped = ironsmith_core::TagKey::from("equipped");
    let mut rebound = apply.clone();
    rebound.target_spec = Some(ChooseSpec::Tagged(equipped).with_surface_hint(
        ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
            ironsmith_core::SourceReferenceSurface::ThisPermanentType("the creature".to_string()),
        ),
    ));
    let mut result_if = result_if.clone();
    result_if.then = vec![crate::effect::Effect::new(rebound).tag(tagged.tag.clone())];
    result_segment.default_effects = vec![crate::effect::Effect::new(result_if)];
}

/// Preserve the controller set of a coalesced combat-damage trigger for an
/// authored "you and the controller of those creatures each draw" follow-up.
/// The trigger checker snapshots the complete matching damage-source group;
/// this lowering step consumes that group once per distinct controller.
fn bind_combat_damage_group_controller_draw(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(11).any(|window| {
        window
            == [
                "you",
                "and",
                "the",
                "controller",
                "of",
                "those",
                "creatures",
                "each",
                "draw",
                "a",
                "card",
            ]
    }) || !matches!(
        &triggered.trigger.kind,
        crate::triggers::TriggerKind::DealsCombatDamageToPlayer {
            one_or_more: true,
            ..
        }
    ) || !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [draw_effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>() else {
        return false;
    };
    if draw.count != crate::effect::Value::Fixed(1) || draw.player != PlayerFilter::You {
        return false;
    }
    segment.default_effects.push(crate::effect::Effect::new(
        crate::effects::ForEachControllerOfTaggedEffect {
            tag: crate::tag::TagKey::from(ironsmith_core::COMBAT_DAMAGE_GROUP_TAG),
            effects: vec![crate::effect::Effect::target_draws(
                1,
                PlayerFilter::IteratedPlayer,
            )],
        },
    ));
    true
}

/// Preserve the two distinct antecedents in an equipped-attacker
/// draw-and-reveal trigger: `the creature` is the triggering attacker while
/// `that card` is the exact card drawn by the preceding action.  Both facts
/// are proven by typed wrappers and the authored source phrases before this
/// post-lowering reconciliation is allowed to run.
fn bind_equipped_attack_draw_reveal_result(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    // Depending on the public lowering route, sentence punctuation remains
    // between the draw/reveal sentence and the result sentence. Prove the
    // authored clauses independently; the typed program guards below still
    // require the exact attacker, draw/reveal, pump, and life-loss shape.
    let source_words =
        crate::runtime_backend::front_end::lexer::parser_token_word_refs(source_tokens);
    let source_has = |phrase: &[&str]| {
        source_words
            .windows(phrase.len())
            .any(|window| window == phrase)
    };
    if !source_has(&["equipped", "creature", "attacks"])
        || !source_has(&["draw", "a", "card"])
        || !source_has(&["reveal", "it"])
        || !source_has(&["the", "creature", "gets"])
        || !source_has(&["where", "x", "is"])
        || !source_has(&["mana", "value"])
    {
        return;
    }
    let ironsmith_core::TriggerKind::Attacks { filter } = &triggered.trigger.kind else {
        return;
    };
    let mut expected_attacker = ObjectFilter::creature();
    expected_attacker.set_explicit_card_type_noun(Some(crate::types::CardType::Creature));
    expected_attacker = expected_attacker
        .match_tagged(
            ironsmith_core::TagKey::from("equipped"),
            ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
        )
        .in_zone(Zone::Battlefield);
    if filter != &expected_attacker || !triggered.choices.is_empty() {
        return;
    }
    let [draw_segment, result_segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !draw_segment.self_replacements.is_empty()
        || draw_segment.starts_new_source_line
        || !result_segment.self_replacements.is_empty()
        || result_segment.starts_new_source_line
    {
        return;
    }
    let [tag_triggering, draw_sequence_root] = draw_segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(tag_triggering) =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    else {
        return;
    };
    let triggering_tag = tag_triggering.tag.clone();
    let Some(draw_sequence) = draw_sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .cloned()
    else {
        return;
    };
    let [draw_root, reveal_root] = draw_sequence.effects.as_slice() else {
        return;
    };
    let Some(draw) = draw_root.downcast_ref::<crate::effects::DrawCardsEffect>() else {
        return;
    };
    let Some(reveal) = reveal_root.downcast_ref::<crate::effects::RevealTaggedEffect>() else {
        return;
    };
    if draw.count != crate::effect::Value::Fixed(1)
        || draw.player != PlayerFilter::You
        || reveal.tag != triggering_tag
    {
        return;
    }

    let [result_sequence_root] = result_segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(result_sequence) = result_sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .cloned()
    else {
        return;
    };
    let [pump_root, lose_root] = result_sequence.effects.as_slice() else {
        return;
    };
    let Some(pump_tagged) = pump_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(apply) = pump_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return;
    };
    let Some(lose) = lose_root.downcast_ref::<crate::effects::LoseLifeEffect>() else {
        return;
    };
    let mut expected_creature = ObjectFilter::creature();
    expected_creature.set_explicit_card_type_noun(Some(crate::types::CardType::Creature));
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = apply.runtime_modifications.as_slice()
    else {
        return;
    };
    let exact_mana_value_tag = |value: &crate::effect::Value, expected: &str| {
        matches!(
            value.unhinted(),
            crate::effect::Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == expected)
        )
    };
    if apply.target != crate::continuous::EffectTarget::Filter(expected_creature)
        || apply.target_spec.is_some()
        || apply.until != crate::effect::Until::EndOfTurn
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.condition.is_some()
        || !exact_mana_value_tag(power, "__it__")
        || !exact_mana_value_tag(toughness, "__it__")
        || lose.player != PlayerFilter::You
        || !exact_mana_value_tag(&lose.amount, pump_tagged.tag.as_str())
    {
        return;
    }

    let drawn_tag = ironsmith_core::TagKey::from("__drawn_revealed_card__");
    let mut rebound_draw_sequence = draw_sequence.clone();
    rebound_draw_sequence.effects[0] =
        crate::effect::Effect::new(draw.clone()).tag(drawn_tag.clone());
    rebound_draw_sequence.effects[1] =
        crate::effect::Effect::new(crate::effects::RevealTaggedEffect::new(drawn_tag.clone()));
    *draw_sequence_root = crate::effect::Effect::new(rebound_draw_sequence);

    let rebound_value = |value: &crate::effect::Value| {
        crate::effect::Value::ManaValueOf(Box::new(ChooseSpec::Tagged(drawn_tag.clone())))
            .with_surface_hints(value.surface_hints().iter().cloned())
    };
    let mut rebound_apply = apply.clone();
    rebound_apply.target_spec = Some(ChooseSpec::Tagged(triggering_tag).with_surface_hint(
        ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
            ironsmith_core::SourceReferenceSurface::ThisPermanentType("the creature".to_string()),
        ),
    ));
    rebound_apply.runtime_modifications = vec![
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power: rebound_value(power),
            toughness: rebound_value(toughness),
        },
    ];
    let mut rebound_pump = pump_tagged.clone();
    rebound_pump.effect = Box::new(crate::effect::Effect::new(rebound_apply));
    let mut rebound_lose = lose.clone();
    rebound_lose.amount = rebound_value(&lose.amount);
    let mut rebound_result_sequence = result_sequence.clone();
    rebound_result_sequence.effects = vec![
        crate::effect::Effect::new(rebound_pump),
        crate::effect::Effect::new(rebound_lose),
    ];
    *result_sequence_root = crate::effect::Effect::new(rebound_result_sequence);
}

/// Preserve the relational source of `choose another ... with lesser power`
/// when an earlier target in the same triggered ability is the comparison
/// object. The ordinary filter remains generic; `ExecuteWithSource` supplies
/// the exact tagged creature whose power must be exceeded.
fn bind_later_attacker_choice_to_prior_target_power(
    triggered: &mut crate::ability::TriggeredAbility,
) {
    let ironsmith_core::TriggerKind::AttacksOneOrMore { filter } = &triggered.trigger.kind else {
        return;
    };
    if filter != &ObjectFilter::creature().controlled_by(PlayerFilter::You) {
        return;
    }
    if triggered
        .effects
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return;
    }
    let [
        target_segment,
        connive_segment,
        choice_segment,
        grant_segment,
    ] = triggered.effects.segments.as_mut_slice()
    else {
        return;
    };
    let mut attacking_creature = ObjectFilter::creature();
    attacking_creature.set_explicit_card_type_noun(Some(crate::types::CardType::Creature));
    attacking_creature.attacking = true;

    let [target_effect, restriction_effect] = target_segment.default_effects.as_slice() else {
        return;
    };
    let Some(targeted) = target_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(target_only) = targeted
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return;
    };
    let targeted_tag = targeted.tag.clone();
    if targeted_tag.as_str() != "targeted_0"
        || target_only.target != ChooseSpec::target(ChooseSpec::Object(attacking_creature.clone()))
        || triggered.choices.as_slice() != [target_only.target.clone()]
    {
        return;
    }
    let Some(cant) = restriction_effect.downcast_ref::<crate::effects::CantEffect>() else {
        return;
    };
    let crate::effect::Restriction::BeBlocked(restricted) = &cant.restriction else {
        return;
    };
    if restricted
        != &attacking_creature.clone().match_tagged(
            targeted_tag.clone(),
            ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
        )
        || cant.duration != crate::effect::Until::EndOfTurn
    {
        return;
    }

    let [connive_effect] = connive_segment.default_effects.as_slice() else {
        return;
    };
    let Some(connived) = connive_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(connive) = connived
        .effect
        .downcast_ref::<crate::effects::ConniveEffect>()
    else {
        return;
    };
    if connive.target != ChooseSpec::Tagged(targeted_tag.clone())
        || connive.count != crate::effect::Value::Fixed(1)
    {
        return;
    }

    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        return;
    };
    let Some(sequence) = choice_effect.downcast_ref::<crate::effects::SequenceEffect>() else {
        return;
    };
    let [choose_effect] = sequence.effects.as_slice() else {
        return;
    };
    let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return;
    };
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen
        || sequence.result_label.is_some()
        || choose.filter != attacking_creature
        || choose.count != ironsmith_core::ChoiceCount::exactly(1)
        || choose.chooser != PlayerFilter::You
        || choose.tag.as_str() != "__it__"
        || choose.zone != Some(Zone::Battlefield)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
    {
        return;
    }

    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        return;
    };
    let Some(grant) = grant_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return;
    };
    if grant.until != crate::effect::Until::EndOfTurn
        || !grant.target_spec.as_ref().is_some_and(
            |spec| matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"),
        )
        || !matches!(
            grant.modification.as_ref(),
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::DoubleStrike
        )
    {
        return;
    }

    let mut choose = choose.clone();
    choose.filter.other = true;
    choose.filter.power_relative_to_source =
        Some(ironsmith_core::SourcePowerRelation::LessThanSource);
    let mut sequence = sequence.clone();
    sequence.effects = vec![crate::effect::Effect::new(choose)];
    choice_segment.default_effects = vec![crate::effect::Effect::new(
        crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Tagged(targeted_tag),
            crate::effect::Effect::new(sequence),
        ),
    )];
}

fn transport_plural_copy_retarget_into_delayed_trigger(
    program: &mut crate::resolution::ResolutionProgram,
) {
    fn exact_copied_ability_leaf(
        effect: &crate::effect::Effect,
        triggering_source_tag: &crate::TagKey,
        copied_tag_seen: bool,
        with_id_seen: bool,
    ) -> bool {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            if tagged.tag.as_str() != crate::cards::builders::COPIED_STACK_OBJECT_TAG {
                return false;
            }
            return exact_copied_ability_leaf(
                &tagged.effect,
                triggering_source_tag,
                true,
                with_id_seen,
            );
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return exact_copied_ability_leaf(
                &with_id.effect,
                triggering_source_tag,
                copied_tag_seen,
                true,
            );
        }
        let Some(copy) = effect.downcast_ref::<crate::effects::CopySpellEffect>() else {
            return false;
        };
        copied_tag_seen
            && with_id_seen
            && copy.target_reference_kind == Some(crate::filter::StackObjectKind::Ability)
            && !copy.target_reference_pronoun
            && copy.count == crate::effect::Value::Fixed(1)
            && copy.count_surface.is_none()
            && copy.copier == PlayerFilter::You
            && copy.removed_supertypes.is_empty()
            && matches!(
                copy.target.base(),
                ChooseSpec::Tagged(tag) if tag == triggering_source_tag
            )
    }

    fn delayed_payload_creates_exact_tagged_ability_copy(
        schedule: &crate::effects::ScheduleDelayedTriggerEffect,
    ) -> bool {
        let [tag_triggering_source, tagged_copy] = schedule.effects.as_slice() else {
            return false;
        };
        let Some(tag_triggering_source) =
            tag_triggering_source.downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
        else {
            return false;
        };
        exact_copied_ability_leaf(tagged_copy, &tag_triggering_source.tag, false, false)
    }

    fn exact_plural_copy_retarget_followup(effect: &crate::effect::Effect) -> bool {
        let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        else {
            return false;
        };
        let [retarget] = may.effects.as_slice() else {
            return false;
        };
        let Some(retarget) = retarget.downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        else {
            return false;
        };
        may.decider == Some(PlayerFilter::You)
            && retarget.copy_reference_plural
            && retarget.mode == crate::effects::RetargetMode::All
            && retarget.chooser == PlayerFilter::You
            && !retarget.require_change
            && retarget.new_target_restriction.is_none()
            && matches!(
                retarget.target.base(),
                ChooseSpec::Tagged(tag)
                    if tag.as_str() == crate::cards::builders::COPIED_STACK_OBJECT_TAG
            )
    }

    loop {
        let mut segments = program.segments.clone();
        let positions = segments
            .iter()
            .enumerate()
            .flat_map(|(segment_index, segment)| {
                segment
                    .default_effects
                    .iter()
                    .enumerate()
                    .map(move |(effect_index, _)| (segment_index, effect_index))
            })
            .collect::<Vec<_>>();
        let mut transported = false;
        for pair in positions.windows(2) {
            let [
                (previous_segment, previous_effect),
                (followup_segment, followup_effect),
            ] = pair
            else {
                continue;
            };
            if !segments[*previous_segment].self_replacements.is_empty()
                || !segments[*followup_segment].self_replacements.is_empty()
                || !exact_plural_copy_retarget_followup(
                    &segments[*followup_segment].default_effects[*followup_effect],
                )
            {
                continue;
            }
            let Some(schedule) = segments[*previous_segment].default_effects[*previous_effect]
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            else {
                continue;
            };
            if !delayed_payload_creates_exact_tagged_ability_copy(schedule) {
                continue;
            }
            let mut schedule = schedule.clone();
            schedule
                .effects
                .push(segments[*followup_segment].default_effects[*followup_effect].clone());
            segments[*previous_segment].default_effects[*previous_effect] =
                crate::effect::Effect::new(schedule);
            segments[*followup_segment]
                .default_effects
                .remove(*followup_effect);
            if segments[*followup_segment].default_effects.is_empty() {
                segments.remove(*followup_segment);
            }
            *program = crate::resolution::ResolutionProgram::new(segments);
            transported = true;
            break;
        }
        if !transported {
            break;
        }
    }
}

/// Preserve the source pronoun in an exact Aura-enters sticker instruction.
/// The surrounding Aura attachment line seeds `enchanted` as the most recent
/// object, but authored "When this Aura enters ... put ... on it" refers to
/// the entering Aura itself, not the enchanted creature.
fn bind_aura_enters_sticker_pronoun_to_source(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(4).any(|window| {
        window == ["when", "this", "aura", "enters"]
            || window == ["whenever", "this", "aura", "enters"]
    }) || !words
        .windows(6)
        .any(|window| window == ["put", "a", "name", "sticker", "on", "it"])
    {
        return;
    }
    let crate::triggers::TriggerKind::ZoneChange(enters) = &triggered.trigger.kind else {
        return;
    };
    if !enters.this
        || enters.from.is_some()
        || enters.from_zones.is_some()
        || enters.from_excluded.is_some()
        || enters.to != Some(crate::zone::Zone::Battlefield)
        || enters.to_excluded.is_some()
        || enters.this_surface
            != Some(ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                "this Aura".to_string(),
            ))
    {
        return;
    }

    fn rewrite(effect: &crate::effect::Effect, rewrites: &mut usize) -> crate::effect::Effect {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(rewrite(&with_id.effect, rewrites));
            return crate::effect::Effect::new(with_id);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        {
            let mut may = may.clone();
            may.effects = may
                .effects
                .iter()
                .map(|inner| rewrite(inner, rewrites))
                .collect();
            return crate::effect::Effect::new(may);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(rewrite(&tagged.effect, rewrites));
            return crate::effect::Effect::new(tagged);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence
                .effects
                .iter()
                .map(|inner| rewrite(inner, rewrites))
                .collect();
            return crate::effect::Effect::new(sequence);
        }
        let Some(sticker) = effect.downcast_ref::<crate::effects::PutStickerEffect>() else {
            return effect.clone();
        };
        if sticker.action != crate::events::KeywordActionKind::NameSticker
            || !matches!(&sticker.target, ChooseSpec::Tagged(tag) if tag.as_str() == "enchanted")
        {
            return effect.clone();
        }
        *rewrites += 1;
        let mut sticker = sticker.clone();
        sticker.target = ChooseSpec::Source;
        crate::effect::Effect::new(sticker)
    }

    let original = triggered.effects.clone();
    let mut rewrites = 0usize;
    for segment in &mut triggered.effects.segments {
        segment.default_effects = segment
            .default_effects
            .iter()
            .map(|effect| rewrite(effect, &mut rewrites))
            .collect();
    }
    if rewrites != 1 {
        triggered.effects = original;
    }
}

/// Keep the fixed retarget in an exact per-opponent delayed copy loop bound to
/// the choice made inside that loop. Ordinary reference resolution can
/// advance `it` back to the triggering spell before lowering the fixed target,
/// even though the authored antecedent is "the chosen player or permanent."
fn bind_next_spell_other_opponent_copy_retarget_choice(
    program: &mut crate::resolution::ResolutionProgram,
) {
    fn corrected_schedule(
        schedule: &crate::effects::ScheduleDelayedTriggerEffect,
    ) -> Option<crate::effects::ScheduleDelayedTriggerEffect> {
        if !schedule.one_shot
            || !schedule.until_end_of_turn
            || schedule.start_next_turn
            || schedule.until_end_of_combat
            || schedule.leading_duration_surface
            || schedule.watch_ability_source
            || schedule.watch_all_object_targets
            || schedule.either_of_watched_objects
            || schedule.duration != ironsmith_core::DelayedTriggerDuration::EndOfTurn
            || schedule.while_any_tagged_object_in_zone.is_some()
            || !schedule.target_choices.is_empty()
            || schedule.target_tag.is_some()
            || schedule.target_filter.is_some()
            || schedule.controller != PlayerFilter::You
            || schedule.prepayment.is_some()
            || schedule.event_value_from_prior_prevention
        {
            return None;
        }

        let crate::effect::DelayedTriggerSpec::SpellCast {
            filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
            first_spell_of_game,
        } = &schedule.trigger
        else {
            return None;
        };
        let mut expected_spell = ObjectFilter::instant_or_sorcery()
            .targeting_only(
                Some(PlayerFilter::Opponent),
                Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
            )
            .target_count_exact(1);
        expected_spell.has_mana_cost = true;
        if filter.as_ref()? != &expected_spell
            || caster != &PlayerFilter::You
            || timing.is_some()
            || during_turn.is_some()
            || min_spells_this_turn.is_some()
            || exact_spells_this_turn.is_some()
            || *from_not_hand
            || *first_spell_of_game
        {
            return None;
        }

        let [tag_triggering_effect, for_players_effect] = schedule.effects.as_slice() else {
            return None;
        };
        let tag_triggering =
            tag_triggering_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
        if tag_triggering.tag.as_str() != "triggering" {
            return None;
        }
        let for_players = for_players_effect
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()?;
        if for_players.filter
            != PlayerFilter::excluding(
                PlayerFilter::Opponent,
                PlayerFilter::TargetPlayerOrControllerOfTarget,
            )
        {
            return None;
        }
        let [choice_effect, copy_effect, retarget_effect] = for_players.effects.as_slice() else {
            return None;
        };
        let tagged_choice = choice_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
        let choice = tagged_choice
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let expected_permanent =
            ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer);
        if choice.chooser.is_some()
            || choice.explicit_declaration
            || !matches!(
                &choice.target,
                ChooseSpec::ObjectOrPlayer(object, PlayerFilter::IteratedPlayer)
                    if object == &expected_permanent
            )
        {
            return None;
        }

        fn exact_copy(
            effect: &crate::effect::Effect,
            copied_tag: &mut Option<crate::TagKey>,
        ) -> bool {
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                if tagged.tag.as_str() != crate::cards::builders::COPIED_STACK_OBJECT_TAG
                    || copied_tag.replace(tagged.tag.clone()).is_some()
                {
                    return false;
                }
                return exact_copy(&tagged.effect, copied_tag);
            }
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return exact_copy(&with_id.effect, copied_tag);
            }
            let Some(copy) = effect.downcast_ref::<crate::effects::CopySpellEffect>() else {
                return false;
            };
            copy.count == crate::effect::Value::Fixed(1)
                && copy.copier == PlayerFilter::You
                && copy.target_reference_kind == Some(crate::filter::StackObjectKind::Spell)
                && !copy.target_reference_pronoun
                && matches!(
                    &copy.target,
                    ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
                )
                && copy.removed_supertypes.is_empty()
                && !copy.has_characteristic_modifiers()
        }
        let mut copied_tag = None;
        if !exact_copy(copy_effect, &mut copied_tag) {
            return None;
        }
        let copied_tag = copied_tag?;

        let retarget =
            retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
        if retarget.chooser != PlayerFilter::You
            || retarget.require_change
            || retarget.new_target_restriction.is_some()
            || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == &copied_tag)
        {
            return None;
        }
        let crate::effects::RetargetMode::OneToFixed(ChooseSpec::ObjectOrPlayer(
            chosen_object,
            PlayerFilter::IteratedPlayer,
        )) = &retarget.mode
        else {
            return None;
        };
        let [constraint] = chosen_object.tagged_constraints.as_slice() else {
            return None;
        };
        let mut semantic_chosen = chosen_object.clone();
        semantic_chosen.tagged_constraints.clear();
        if semantic_chosen != expected_permanent
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            || (constraint.tag != tagged_choice.tag && constraint.tag.as_str() != "triggering")
        {
            return None;
        }

        let mut corrected_object = semantic_chosen;
        corrected_object
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: tagged_choice.tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let mut corrected_retarget = retarget.clone();
        corrected_retarget.mode = crate::effects::RetargetMode::OneToFixed(
            ChooseSpec::ObjectOrPlayer(corrected_object, PlayerFilter::IteratedPlayer),
        );
        let mut corrected_for_players = for_players.clone();
        corrected_for_players.effects[2] = crate::effect::Effect::new(corrected_retarget);
        let mut corrected = schedule.clone();
        corrected.effects[1] = crate::effect::Effect::new(corrected_for_players);
        Some(corrected)
    }

    for segment in &mut program.segments {
        for effect in &mut segment.default_effects {
            let Some(schedule) = effect
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
                .and_then(corrected_schedule)
            else {
                continue;
            };
            *effect = crate::effect::Effect::new(schedule);
        }
    }
}

fn transport_fixed_retarget_into_optional_copy(program: &mut crate::resolution::ResolutionProgram) {
    fn exact_optional_single_spell_copy(
        effect: &crate::effect::Effect,
    ) -> Option<crate::effects::MayEffect<crate::effect::Effect>> {
        let may = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()?;
        if may.decider != Some(PlayerFilter::You) {
            return None;
        }
        let [tagged_copy] = may.effects.as_slice() else {
            return None;
        };
        let tagged_copy = tagged_copy.downcast_ref::<crate::effects::TaggedEffect>()?;
        if tagged_copy.tag.as_str() != crate::cards::builders::COPIED_STACK_OBJECT_TAG {
            return None;
        }
        let with_id = tagged_copy
            .effect
            .downcast_ref::<crate::effects::WithIdEffect>()?;
        let copy = with_id
            .effect
            .downcast_ref::<crate::effects::CopySpellEffect>()?;
        (copy.target_reference_kind == Some(crate::filter::StackObjectKind::Spell)
            && !copy.target_reference_pronoun
            && copy.count == crate::effect::Value::Fixed(1)
            && copy.count_surface.is_none()
            && copy.copier == PlayerFilter::You
            && copy.removed_supertypes.is_empty())
        .then(|| may.clone())
    }

    fn exact_fixed_source_copy_retarget(effect: &crate::effect::Effect) -> bool {
        let Some(retarget) = effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        else {
            return false;
        };
        retarget.chooser == PlayerFilter::You
            && !retarget.require_change
            && !retarget.copy_reference_plural
            && retarget.new_target_restriction.is_none()
            && matches!(
                retarget.target.base(),
                ChooseSpec::Tagged(tag)
                    if tag.as_str() == crate::cards::builders::COPIED_STACK_OBJECT_TAG
            )
            && matches!(
                &retarget.mode,
                crate::effects::RetargetMode::OneToFixed(fixed)
                    if matches!(fixed.base(), ChooseSpec::Source)
            )
    }

    let mut segments = program.segments.clone();
    let positions = segments
        .iter()
        .enumerate()
        .flat_map(|(segment_index, segment)| {
            segment
                .default_effects
                .iter()
                .enumerate()
                .map(move |(effect_index, _)| (segment_index, effect_index))
        })
        .collect::<Vec<_>>();
    for pair in positions.windows(2) {
        let [
            (may_segment, may_effect),
            (retarget_segment, retarget_effect),
        ] = pair
        else {
            continue;
        };
        if !segments[*may_segment].self_replacements.is_empty()
            || !segments[*retarget_segment].self_replacements.is_empty()
            || !exact_fixed_source_copy_retarget(
                &segments[*retarget_segment].default_effects[*retarget_effect],
            )
        {
            continue;
        }
        let Some(mut may) =
            exact_optional_single_spell_copy(&segments[*may_segment].default_effects[*may_effect])
        else {
            continue;
        };
        may.effects
            .push(segments[*retarget_segment].default_effects[*retarget_effect].clone());
        segments[*may_segment].default_effects[*may_effect] = crate::effect::Effect::new(may);
        segments[*retarget_segment]
            .default_effects
            .remove(*retarget_effect);
        if segments[*retarget_segment].default_effects.is_empty() {
            segments.remove(*retarget_segment);
        }
        *program = crate::resolution::ResolutionProgram::new(segments);
        return;
    }
}

/// Preserve a terminal spell-target arity phrase at the public ability
/// boundary. Some trigger-family routes have already built the stack-spell
/// domain before the ordinary object-filter parser sees this terminal clause.
fn bind_authored_single_target_spell_cast_filter(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    if !source_words_contain(source_tokens, &["with", "a", "single", "target"]) {
        return;
    }
    let filter = match &mut triggered.trigger.kind {
        ironsmith_core::TriggerKind::SpellCast {
            filter: Some(filter),
            ..
        }
        | ironsmith_core::TriggerKind::SpellCastQualified {
            filter: Some(filter),
            ..
        } => filter,
        _ => return,
    };
    if filter.zone != Some(Zone::Stack)
        || filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || filter.target_count.is_some()
    {
        return;
    }
    filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
}

/// Reapply an authored disjunctive color list after public trigger
/// normalization. The direct trigger parser retains these colors, but a
/// runtime-backed ability shell can already contain only its first arm by the
/// time surface reconciliation runs.
fn bind_authored_spell_cast_color_list(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.iter().any(|word| matches!(*word, "cast" | "casts"))
        || !words.iter().any(|word| matches!(*word, "spell" | "spells"))
    {
        return;
    }
    let mut colors = crate::color::ColorSet::new();
    let mut count = 0usize;
    for &word in &words {
        let color = match word {
            "white" => Some(crate::color::ColorSet::WHITE),
            "blue" => Some(crate::color::ColorSet::BLUE),
            "black" => Some(crate::color::ColorSet::BLACK),
            "red" => Some(crate::color::ColorSet::RED),
            "green" => Some(crate::color::ColorSet::GREEN),
            _ => None,
        };
        if let Some(color) = color
            && !colors.contains_all(color)
        {
            colors = colors.union(color);
            count += 1;
        }
    }
    let filter = match &mut triggered.trigger.kind {
        ironsmith_core::TriggerKind::SpellCast {
            filter: Some(filter),
            ..
        }
        | ironsmith_core::TriggerKind::SpellCastQualified {
            filter: Some(filter),
            ..
        } => filter,
        _ => return,
    };
    let explicit_spell_colors = words.windows(2).any(|window| {
        matches!(
            window,
            [
                "white" | "blue" | "black" | "red" | "green",
                "spell" | "spells"
            ]
        )
    }) || words.windows(3).any(|window| {
        matches!(
            window,
            [
                "white" | "blue" | "black" | "red" | "green",
                "or",
                "white" | "blue" | "black" | "red" | "green"
            ]
        )
    });
    if count > 0 && explicit_spell_colors {
        filter.colors = Some(colors);
    } else if count > 0
        && filter.colors.is_some()
        && !explicit_spell_colors
        && words
            .iter()
            .any(|word| matches!(*word, "create" | "creates"))
    {
        // Colors that appear only in a token-creation consequence describe
        // that token, not the spell whose cast caused the trigger.
        filter.colors = None;
    }
}

fn bind_authored_spell_cast_ability_marker(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(2)
        .any(|window| matches!(window, ["kicked", "spell" | "spells"]))
    {
        return;
    }
    let filter = match &mut triggered.trigger.kind {
        ironsmith_core::TriggerKind::SpellCast {
            filter: Some(filter),
            ..
        }
        | ironsmith_core::TriggerKind::SpellCastQualified {
            filter: Some(filter),
            ..
        } => filter,
        _ => return,
    };
    if !filter
        .ability_markers
        .iter()
        .any(|marker| marker.eq_ignore_ascii_case("kicked"))
    {
        filter.ability_markers.push("kicked".to_string());
    }
}

/// Recover spell-cast constraints that are evaluated when the cast event
/// happens, rather than folding their nouns into the triggering spell's own
/// object filter. This keeps a same-named graveyard card as a second object
/// and keeps a source-counter threshold as an intervening-if condition.
fn bind_authored_spell_cast_relation_constraints(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    fn exact_optional_quest_counter_on_source(
        triggered: &crate::ability::TriggeredAbility,
    ) -> bool {
        if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
            return false;
        }
        let [segment] = triggered.effects.segments.as_slice() else {
            return false;
        };
        let [effect] = segment.default_effects.as_slice() else {
            return false;
        };
        if !segment.self_replacements.is_empty() || segment.starts_new_source_line {
            return false;
        }
        let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        else {
            return false;
        };
        let [put] = may.effects.as_slice() else {
            return false;
        };
        let Some(put) = put.downcast_ref::<crate::effects::PutCountersEffect>() else {
            return false;
        };
        matches!(may.decider, None | Some(PlayerFilter::You))
            && put.counter_type == crate::CounterType::Quest
            && put.amount == crate::effect::Value::Fixed(1)
            && matches!(put.target.unhinted(), ChooseSpec::Source)
            && put.target_count.is_none()
            && !put.distributed
    }

    fn spell_cast_by_you(trigger: &crate::triggers::Trigger) -> bool {
        matches!(
            &trigger.kind,
            ironsmith_core::TriggerKind::SpellCast {
                caster: PlayerFilter::You,
                ..
            } | ironsmith_core::TriggerKind::SpellCastQualified {
                caster: PlayerFilter::You,
                ..
            }
        )
    }

    let intro_surface = triggered.trigger.intro_surface;
    if source_words_contain(
        source_tokens,
        &[
            "instant", "or", "sorcery", "spell", "that", "has", "the", "same", "name",
        ],
    ) && source_words_contain(
        source_tokens,
        &["as", "a", "card", "in", "your", "graveyard"],
    ) && spell_cast_by_you(&triggered.trigger)
        && exact_optional_quest_counter_on_source(triggered)
    {
        let mut trigger = crate::triggers::Trigger::spell_cast_same_name_card_in_zone(
            Some(ObjectFilter::instant_or_sorcery()),
            PlayerFilter::You,
            Zone::Graveyard,
            PlayerFilter::You,
        );
        trigger.intro_surface = intro_surface;
        triggered.trigger = trigger;
        return;
    }

    if source_words_contain(
        source_tokens,
        &[
            "instant",
            "or",
            "sorcery",
            "spell",
            "while",
            "this",
            "enchantment",
            "has",
        ],
    ) && source_words_contain(
        source_tokens,
        &["two", "or", "more", "quest", "counters", "on", "it"],
    ) && spell_cast_by_you(&triggered.trigger)
        && matches!(
            triggered.intervening_if,
            Some(crate::effect::Condition::SourceHasCounterAtLeast {
                counter_type: crate::CounterType::Quest,
                count: 2,
                ..
            })
        )
    {
        let mut trigger = crate::triggers::Trigger::spell_cast(
            Some(ObjectFilter::instant_or_sorcery()),
            PlayerFilter::You,
        );
        trigger.intro_surface = intro_surface;
        triggered.trigger = trigger;
    }

    if source_words_contain(
        source_tokens,
        &[
            "spell", "that", "shares", "a", "color", "or", "mana", "value", "with", "the",
            "exiled", "card",
        ],
    ) && matches!(
        &triggered.trigger.kind,
        ironsmith_core::TriggerKind::SpellCast {
            caster: PlayerFilter::Any,
            ..
        } | ironsmith_core::TriggerKind::SpellCastQualified {
            caster: PlayerFilter::Any,
            ..
        }
    ) && triggered.intervening_if.is_none()
        && triggered.choices.is_empty()
    {
        let [segment] = triggered.effects.segments.as_slice() else {
            return;
        };
        let [effect] = segment.default_effects.as_slice() else {
            return;
        };
        let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() else {
            return;
        };
        let Some(damage) = execute
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        else {
            return;
        };
        if !segment.self_replacements.is_empty()
            || segment.starts_new_source_line
            || damage.amount != crate::effect::Value::Fixed(2)
            || damage.target != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        {
            return;
        }
        let comparison = ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
        let mut filter = ObjectFilter::spell();
        filter
            .characteristic_relations
            .push(ironsmith_core::ObjectCharacteristicRelation::shares(
                vec![
                    ironsmith_core::ObjectCharacteristic::Color,
                    ironsmith_core::ObjectCharacteristic::ManaValue,
                ],
                comparison,
            ));
        let mut trigger = crate::triggers::Trigger::spell_cast(Some(filter), PlayerFilter::Any);
        trigger.intro_surface = intro_surface;
        triggered.trigger = trigger;
    }
}

/// Bind an authored plural stack-object grant to both members of the exact
/// optional-copy result set. The generic subject carry resolves `those
/// spells` to the triggering spell alone; the tagged copy plus shared result
/// id prove the only second member that may receive the same keyword grant.
fn bind_original_and_copy_plural_keyword_grant(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    if !source_words_contain(source_tokens, &["those", "spells", "gain"])
        || triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
    {
        return;
    }
    let caster = match &triggered.trigger.kind {
        ironsmith_core::TriggerKind::SpellCast { caster, .. }
        | ironsmith_core::TriggerKind::SpellCastQualified { caster, .. } => caster,
        _ => return,
    };
    if caster != &PlayerFilter::You {
        return;
    }
    let [copy_segment, grant_segment, retarget_segment] = triggered.effects.segments.as_mut_slice()
    else {
        return;
    };
    if !copy_segment.self_replacements.is_empty()
        || !grant_segment.self_replacements.is_empty()
        || !retarget_segment.self_replacements.is_empty()
    {
        return;
    }

    let [tag_triggering, copy_root] = copy_segment.default_effects.as_slice() else {
        return;
    };
    let Some(tag_triggering) =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    else {
        return;
    };
    let Some(copy_result) = copy_root.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    let Some(may_copy) = copy_result
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    let [tagged_copy] = may_copy.effects.as_slice() else {
        return;
    };
    let Some(tagged_copy) = tagged_copy.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(copy_with_id) = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
    else {
        return;
    };
    let Some(copy) = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
    else {
        return;
    };
    if may_copy.decider != Some(PlayerFilter::You)
        || tagged_copy.tag.as_str() != crate::cards::builders::COPIED_STACK_OBJECT_TAG
        || copy_with_id.id != copy_result.id
        || copy.copier != PlayerFilter::You
        || copy.target_reference_kind != Some(crate::filter::StackObjectKind::Spell)
        || copy.count != crate::effect::Value::Fixed(1)
        || copy.count_surface.is_some()
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
        || !matches!(
            copy.target.base(),
            ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag
        )
    {
        return;
    }

    let [grant_root] = grant_segment.default_effects.as_slice() else {
        return;
    };
    let Some(result) = grant_root.downcast_ref::<crate::effects::IfEffect>() else {
        return;
    };
    let [grant_root] = result.then.as_slice() else {
        return;
    };
    let Some(grant) = grant_root.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return;
    };
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return;
    };
    if result.condition != copy_result.id
        || result.predicate != crate::effect::EffectPredicate::Happened
        || !result.else_.is_empty()
        || grant.until != crate::effect::Until::Forever
        || grant.condition.is_some()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
        || grant.set_quantifier_surface != Some(ironsmith_core::SetQuantifierSurface::Those)
        || !ability.id().is_keyword()
        || !matches!(
            grant.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &tag_triggering.tag
        )
    {
        return;
    }

    let [retarget_root] = retarget_segment.default_effects.as_slice() else {
        return;
    };
    let Some(may_retarget) =
        retarget_root.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    let [retarget] = may_retarget.effects.as_slice() else {
        return;
    };
    let Some(retarget) = retarget.downcast_ref::<crate::effects::RetargetStackObjectEffect>()
    else {
        return;
    };
    if may_retarget.decider != Some(PlayerFilter::You)
        || retarget.chooser != PlayerFilter::You
        || retarget.require_change
        || retarget.copy_reference_plural
        || retarget.new_target_restriction.is_some()
        || retarget.mode != crate::effects::RetargetMode::All
        || !matches!(
            retarget.target.base(),
            ChooseSpec::Tagged(tag) if tag == &tagged_copy.tag
        )
    {
        return;
    }

    let mut copied_grant = grant.clone();
    copied_grant.target_spec = Some(ChooseSpec::Tagged(tagged_copy.tag.clone()));
    let mut result = result.clone();
    result.then.push(crate::effect::Effect::new(copied_grant));
    grant_segment.default_effects = vec![crate::effect::Effect::new(result)];
}

/// Remove the obsolete stack-copy action from the exact lowered representation
/// of "exile [a graveyard card] and copy it; you may cast the copy." A card in
/// a graveyard is not a spell on the stack: `CastTagged(as_copy)` is the
/// executable copy action. Some public statement routes lower the authored
/// conjunction before the cross-sentence family can claim it, leaving both an
/// invalid `CopySpellEffect` and the correct copied-card cast. Reconcile that
/// exact typed program after lowering, while retaining the coordinated surface
/// in the dedicated exile/copy/cast renderer.
fn normalize_graveyard_card_copy_cast_program(program: &mut crate::resolution::ResolutionProgram) {
    fn is_graveyard_domain(filter: &ObjectFilter) -> bool {
        if filter.zone == Some(Zone::Graveyard) {
            return true;
        }
        filter.zone.is_none()
            && !filter.any_of.is_empty()
            && filter.any_of.iter().all(is_graveyard_domain)
    }

    fn transparent_inner(effect: &crate::effect::Effect) -> &crate::effect::Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return transparent_inner(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return transparent_inner(&with_id.effect);
        }
        effect
    }

    fn optional_copy_cast_tag(effect: &crate::effect::Effect) -> Option<crate::TagKey> {
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
            && matches!(may.decider, None | Some(PlayerFilter::You))
            && let [cast] = may.effects.as_slice()
            && let Some(cast) =
                transparent_inner(cast).downcast_ref::<crate::effects::CastTaggedEffect>()
            && cast.player == PlayerFilter::You
            && !cast.allow_land
            && cast.as_copy
            && cast.cost_reduction.is_none()
        {
            return Some(cast.tag.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = optional_copy_cast_tag(child);
            }
        });
        found
    }

    fn tagged_graveyard_exile_tag(effect: &crate::effect::Effect) -> Option<crate::TagKey> {
        let tagged = if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            with_id
                .effect
                .downcast_ref::<crate::effects::TaggedEffect>()?
        } else {
            effect.downcast_ref::<crate::effects::TaggedEffect>()?
        };
        if !crate::runtime_backend::util::is_sentence_helper_tag(tagged.tag.as_str(), "exiled") {
            return None;
        }
        let inner = transparent_inner(&tagged.effect);
        let spec = if let Some(exile) = inner.downcast_ref::<crate::effects::ExileEffect>() {
            if exile.face_down {
                return None;
            }
            &exile.spec
        } else if let Some(moved) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            if moved.zone != Zone::Exile {
                return None;
            }
            &moved.target
        } else {
            return None;
        };
        let ChooseSpec::Object(filter) = spec.base() else {
            return None;
        };
        is_graveyard_domain(filter).then(|| tagged.tag.clone())
    }

    fn obsolete_copy_result_id(
        effect: &crate::effect::Effect,
        exiled_tag: &crate::TagKey,
    ) -> Option<crate::effect::EffectId> {
        let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            return None;
        };
        if tagged.tag.as_str() != crate::cards::builders::COPIED_STACK_OBJECT_TAG {
            return None;
        }
        let Some(with_id) = tagged.effect.downcast_ref::<crate::effects::WithIdEffect>() else {
            return None;
        };
        let copy = with_id
            .effect
            .downcast_ref::<crate::effects::CopySpellEffect>()?;
        (copy.target_reference_kind.is_none()
            && copy.target_reference_pronoun
            && copy.count.unhinted() == &crate::effect::Value::Fixed(1)
            && copy.count_surface.is_none()
            && copy.copier == PlayerFilter::You
            && copy.removed_supertypes.is_empty()
            && !copy.has_characteristic_modifiers()
            && matches!(copy.target.unhinted(), ChooseSpec::Tagged(tag) if tag == exiled_tag))
        .then_some(with_id.id)
    }

    fn obsolete_copy_marker(effect: &crate::effect::Effect, exiled_tag: &crate::TagKey) -> bool {
        obsolete_copy_result_id(effect, exiled_tag).is_some()
    }

    let copied_cast_tags = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .filter_map(optional_copy_cast_tag)
        .collect::<Vec<_>>();
    if copied_cast_tags.is_empty() {
        return;
    }

    // Conditional wording can split the obsolete copy marker into its own
    // `IfEffect`: `exile ... . If you do, copy it. You may cast the copy.`
    // Move the executable copied-card cast into that exact success gate and
    // discard the invalid stack-copy marker. The producer id and shared tag
    // prove both the condition and cast refer to the same graveyard card.
    let mut conditional_segments = program.segments.clone();
    for index in 0..conditional_segments.len().saturating_sub(2) {
        let [producer_root] = conditional_segments[index].default_effects.as_slice() else {
            continue;
        };
        let Some(producer) = producer_root.downcast_ref::<crate::effects::WithIdEffect>() else {
            continue;
        };
        let Some(exiled_tag) = tagged_graveyard_exile_tag(producer_root) else {
            continue;
        };
        let [gate_root] = conditional_segments[index + 1].default_effects.as_slice() else {
            continue;
        };
        let Some(gate) = gate_root.downcast_ref::<crate::effects::IfEffect>() else {
            continue;
        };
        let [copy_marker] = gate.then.as_slice() else {
            continue;
        };
        let [may_root] = conditional_segments[index + 2].default_effects.as_slice() else {
            continue;
        };
        if gate.condition != producer.id
            || gate.predicate != crate::effect::EffectPredicate::Happened
            || !gate.else_.is_empty()
            || gate.per_player_result
            || gate.prior_result_replacement_surface
            || !obsolete_copy_marker(copy_marker, &exiled_tag)
            || optional_copy_cast_tag(may_root).as_ref() != Some(&exiled_tag)
        {
            continue;
        }
        let mut gate = gate.clone();
        gate.then = vec![may_root.clone()];
        conditional_segments[index + 1].default_effects = vec![crate::effect::Effect::new(gate)];
        conditional_segments.remove(index + 2);
        *program = crate::resolution::ResolutionProgram::new(conditional_segments);
        return;
    }

    let mut cast_result_bindings = Vec::new();
    for segment in &mut program.segments {
        for root in &mut segment.default_effects {
            let Some(sequence) = root.downcast_ref::<crate::effects::SequenceEffect>() else {
                continue;
            };
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
                || sequence.result_label.is_some()
            {
                continue;
            }
            let [exile, copy] = sequence.effects.as_slice() else {
                continue;
            };
            let Some(exiled_tag) = tagged_graveyard_exile_tag(exile) else {
                continue;
            };
            if copied_cast_tags.contains(&exiled_tag)
                && let Some(copy_result_id) = obsolete_copy_result_id(copy, &exiled_tag)
            {
                cast_result_bindings.push((exiled_tag, copy_result_id));
                *root = exile.clone();
            }
        }
    }
    for (tag, result_id) in cast_result_bindings {
        for root in program
            .segments
            .iter_mut()
            .flat_map(|segment| segment.default_effects.iter_mut())
        {
            let Some(may) = root
                .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
                .cloned()
            else {
                continue;
            };
            let [cast_root] = may.effects.as_slice() else {
                continue;
            };
            let cast_root = cast_root.clone();
            let Some(cast) = cast_root.downcast_ref::<crate::effects::CastTaggedEffect>() else {
                continue;
            };
            if cast.tag != tag || !cast.as_copy {
                continue;
            }
            let mut may = may;
            may.effects = vec![crate::effect::Effect::with_id(result_id.0, cast_root)];
            *root = crate::effect::Effect::new(may);
            break;
        }
    }
}

/// Recover the returned-set identity in an exact source-linked exile partition.
///
/// The sentence parser gives the counted return its own result tag, then loops
/// over the source's original exiled set and bottoms every member not in that
/// result. Generic reference normalization can otherwise rewrite both sides of
/// the membership test to `__it__`, making it a tautology and leaving the
/// complement in exile. Only repair that exact, fully typed tautological shape.
/// Rebuild the authored dynamic top-of-library exile permission at the last
/// public lowering boundary that still owns the complete source line. Some
/// document routes split the two resolution sentences before the ordinary
/// sequence rule sees them, leaving a fixed top-card exile and a singular
/// permission. This exact lexical grammar proves the reusable correlated
/// form; the replacement remains entirely typed and executable.
fn bind_dynamic_power_owner_exile_permission(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
    raw_line: &str,
) {
    let authored_tokens = crate::runtime_backend::lexer::lex_line(raw_line, 0).ok();
    let words = crate::runtime_backend::lexer::parser_token_word_refs(
        authored_tokens.as_deref().unwrap_or(source_tokens),
    );
    const EXILE: &[&str] = &[
        "exile", "cards", "equal", "to", "its", "power", "from", "the", "top", "of", "its",
        "owners", "library",
    ];
    const PERMISSION: &[&str] = &[
        "you", "may", "cast", "spells", "from", "among", "those", "cards", "for", "as", "long",
        "as", "they", "remain", "exiled", "and", "mana", "of", "any", "type", "can", "be", "spent",
        "to", "cast", "them",
    ];
    let Some(exile_start) = words
        .windows(EXILE.len())
        .position(|window| window == EXILE)
    else {
        return;
    };
    let permission_start = exile_start + EXILE.len();
    if !words[permission_start..]
        .windows(PERMISSION.len())
        .any(|window| window == PERMISSION)
        || program.segments.is_empty()
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return;
    }

    let triggering_tag = crate::TagKey::from("triggering");
    let exiled_tag = crate::TagKey::from(crate::tag::SOURCE_EXILED_TAG);
    let count = crate::effect::Value::PowerOf(Box::new(ChooseSpec::Tagged(triggering_tag.clone())))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);
    let owner = PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(triggering_tag.clone()));
    let exile =
        crate::effects::ExileTopOfLibraryEffect::new(count, owner).tag_moved(exiled_tag.clone());
    let permission = crate::effects::GrantPlayTaggedEffect::new(
        exiled_tag,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
        false,
        ironsmith_core::value_model::ManaSpendMode::AnyType,
    )
    .cast_pool_is_plural(true)
    .with_surface(
        ironsmith_core::GrantPlayTaggedSurface::default()
            .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards),
    );
    let starts_new_source_line = program
        .segments
        .first()
        .is_some_and(|segment| segment.starts_new_source_line);
    let mut segment = crate::resolution::ResolutionSegment::from_effects(vec![
        crate::effect::Effect::new(crate::effects::TagTriggeringObjectEffect::new(
            triggering_tag,
        )),
        crate::effect::Effect::new(exile),
        crate::effect::Effect::new(permission),
    ]);
    segment.starts_new_source_line = starts_new_source_line;
    *program = crate::resolution::ResolutionProgram::new(vec![segment]);
}

/// Repair the exact quantified-player inversion produced when an explicit
/// controller action is followed by a participant-relative unless cost:
///
/// `for each opponent, you create ... unless that player sacrifices ...`
///
/// The outer loop is already executable, but a broad participant-subject
/// normalization can assign the token to the opponent and the sacrifice to
/// the effect controller. `actor_surface_explicit` proves that Oracle named
/// `you` as the token actor; the paired inverse player bindings and exact
/// creature-sacrifice cost keep this correction from changing legitimate
/// participant-created tokens or controller-paid costs.
fn bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: Option<&[crate::runtime_backend::lexer::OwnedLexToken]>,
) {
    fn nested_sacrifice_filter(
        effect: &crate::effect::Effect,
    ) -> Option<&crate::target::ObjectFilter> {
        if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
            return Some(&sacrifice.filter);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return nested_sacrifice_filter(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return nested_sacrifice_filter(&with_id.effect);
        }
        None
    }

    let authored_explicit_you = source_tokens.is_some_and(|tokens| {
        source_words_contain(tokens, &["for", "each", "opponent", "you", "create"])
    });
    for root in program
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        let Some(mut for_players) = root
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .cloned()
        else {
            continue;
        };
        if for_players.filter != PlayerFilter::Opponent
            || for_players.starting_with_controller
            || for_players.stop_after_first_happened
        {
            continue;
        }
        let [unless_root] = for_players.effects.as_slice() else {
            continue;
        };
        let Some(mut unless) = unless_root
            .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            .cloned()
        else {
            continue;
        };
        let [token_root] = unless.effects.as_slice() else {
            continue;
        };
        let Some(mut token) = token_root
            .downcast_ref::<crate::effects::CreateTokenEffect>()
            .cloned()
        else {
            continue;
        };
        let Some([cost]) = unless.cost.as_all() else {
            continue;
        };
        let Some(sacrifice_filter) = cost.effect_ref().and_then(nested_sacrifice_filter) else {
            continue;
        };
        let payer_relative_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
        let iterated_filter = ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer);
        if !matches!(
            unless.player,
            PlayerFilter::You | PlayerFilter::IteratedPlayer
        ) || token.controller != PlayerFilter::Opponent
            || token.controller_target.is_some()
            || !(token.actor_surface_explicit || authored_explicit_you)
            || (sacrifice_filter != &payer_relative_filter && sacrifice_filter != &iterated_filter)
        {
            continue;
        }

        token.controller = PlayerFilter::You;
        token.controller_target = None;
        token.actor_surface_explicit = true;
        unless.player = PlayerFilter::IteratedPlayer;
        // The outer UnlessPays chooses the iterated opponent as the payer. Cost
        // execution then rebases `You` to that payer, so the nested filter must
        // remain payer-relative instead of consulting IteratedPlayer a second time.
        unless.cost =
            crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(payer_relative_filter));
        unless.effects = vec![crate::effect::Effect::new(token)];
        for_players.effects = vec![crate::effect::Effect::new(unless)];
        *root = crate::effect::Effect::new(for_players);
    }
}

/// Preserve a coordinated venture action when a broad intervening-if route
/// has already lowered the leading source return on its own. The authored
/// tail and the typed completed-dungeon gate jointly prove this shape; an
/// ordinary conditional return must not acquire an unrelated venture.
fn restore_authored_return_then_venture(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    if !source_tokens.iter().any(|token| token.is_word("return"))
        || !source_tokens.iter().any(|token| token.is_word("hand"))
        || !source_words_contain(source_tokens, &["and", "venture", "into", "the", "dungeon"])
        || !matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::Not(inner))
                if matches!(
                    inner.as_ref(),
                    crate::ConditionExpr::PlayerCompletedDungeon {
                        player: PlayerFilter::You,
                        ..
                    }
                )
        )
        || triggered
            .effects
            .segments
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
            .any(|effect| {
                effect
                    .downcast_ref::<crate::effects::VentureIntoDungeonEffect>()
                    .is_some()
            })
    {
        return;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [return_effect] = segment.default_effects.as_slice() else {
        return;
    };
    let Some(return_to_hand) = return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
    else {
        return;
    };
    if !matches!(return_to_hand.spec.unhinted(), ChooseSpec::Source) {
        return;
    }
    segment.default_effects = vec![crate::effect::Effect::new(
        crate::effects::SequenceEffect::coordinated(vec![
            return_effect.clone(),
            crate::effect::Effect::new(crate::effects::VentureIntoDungeonEffect::new(
                PlayerFilter::You,
            )),
        ]),
    )];
    triggered.effects =
        crate::resolution::ResolutionProgram::new(triggered.effects.segments.clone());
}

#[cfg(test)]
mod quantified_unless_actor_binding_tests {
    use super::*;

    fn inverted_program(explicit_you: bool) -> crate::resolution::ResolutionProgram {
        let token_definition =
            CardDefinitionBuilder::new(crate::ids::CardId::new(), "Quantified Zombie")
                .card_types(vec![crate::types::CardType::Creature])
                .subtypes(vec![crate::types::Subtype::Zombie])
                .build();
        let mut token = crate::effects::CreateTokenEffect::new(
            token_definition,
            crate::effect::Value::Fixed(1),
            PlayerFilter::Opponent,
        );
        if explicit_you {
            token = token.with_explicit_actor_surface();
        }
        let unless = crate::effects::UnlessPaysEffect {
            player: PlayerFilter::You,
            effects: vec![crate::effect::Effect::new(token)],
            cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
            )),
            leading_surface: false,
            before_delayed_step: false,
        };
        let for_players = crate::effects::ForPlayersEffect {
            filter: PlayerFilter::Opponent,
            effects: vec![crate::effect::Effect::new(unless)],
            starting_with_controller: false,
            stop_after_first_happened: false,
        };
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![crate::effect::Effect::new(
                for_players,
            )]),
        ])
    }

    fn sacrifice_cost(
        program: &crate::resolution::ResolutionProgram,
    ) -> &crate::effects::SacrificeEffect {
        let for_players = program.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player loop");
        let unless = for_players.effects[0]
            .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            .expect("unless payment");
        let [cost] = unless.cost.as_all().expect("all-cost branch") else {
            panic!("expected one sacrifice cost: {unless:#?}");
        };
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            .expect("typed sacrifice cost")
    }

    #[test]
    fn explicit_you_token_and_that_opponent_sacrifice_recover_distinct_actors() {
        let mut program = inverted_program(true);
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(&mut program, None);
        let debug = format!("{program:#?}");
        assert!(debug.contains("filter: Opponent"), "{debug}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
        assert_eq!(
            sacrifice_cost(&program).filter.controller,
            Some(PlayerFilter::You),
            "the sacrifice cost must stay payer-relative after the outer loop chooses the opponent"
        );

        let mut participant_created = inverted_program(false);
        let before = format!("{participant_created:#?}");
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut participant_created,
            None,
        );
        assert_eq!(
            format!("{participant_created:#?}"),
            before,
            "a participant-created token must not inherit the controller-action correction"
        );

        let authored = crate::runtime_backend::lex_line(
            "For each opponent, you create a Zombie token unless that player sacrifices a creature of their choice.",
            0,
        )
        .expect("authored quantified token sentence should lex");
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut participant_created,
            Some(&authored),
        );
        let debug = format!("{participant_created:#?}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
    }

    #[test]
    fn authoritative_iterated_player_and_tagged_sacrifice_cost_are_normalized() {
        let mut program = inverted_program(true);
        let root = &mut program.segments[0].default_effects[0];
        let mut for_players = root
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player loop")
            .clone();
        let mut unless = for_players.effects[0]
            .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            .expect("unless payment")
            .clone();
        unless.player = PlayerFilter::IteratedPlayer;
        unless.cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
            crate::effect::Effect::sacrifice(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                1,
            )
            .tag("sacrifice_cost_0"),
        ));
        for_players.effects = vec![crate::effect::Effect::new(unless)];
        *root = crate::effect::Effect::new(for_players);

        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(&mut program, None);
        let debug = format!("{program:#?}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
        assert_eq!(
            sacrifice_cost(&program).filter.controller,
            Some(PlayerFilter::You),
            "the wrapped cost must normalize to payer-relative You"
        );
        assert!(
            !debug.contains("sacrifice_cost_0"),
            "the transparent provenance wrapper should normalize to an executable cost: {debug}"
        );
    }
}

#[cfg(test)]
mod dynamic_power_owner_exile_permission_tests {
    use super::*;

    fn placeholder_program() -> crate::resolution::ResolutionProgram {
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![crate::effect::Effect::new(
                crate::effects::DrawCardsEffect::new(
                    crate::effect::Value::Fixed(1),
                    PlayerFilter::You,
                ),
            )]),
        ])
    }

    #[test]
    fn exact_authored_bundle_reconciles_to_dynamic_owner_linked_permission() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
            0,
        )
        .expect("linked dynamic permission should lex");
        let mut program = placeholder_program();
        bind_dynamic_power_owner_exile_permission(
            &mut program,
            &tokens,
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
        );
        let debug = format!("{program:#?}");
        for required in [
            "TagTriggeringObjectEffect",
            "PowerOf",
            "OwnerOf",
            "ExileTopOfLibraryEffect",
            "GrantPlayTaggedEffect",
            "ForAsLongAsExiled",
            "AnyType",
            "cast_pool_is_plural: true",
        ] {
            assert!(debug.contains(required), "missing {required}: {debug}");
        }

        let lossy_tokens = crate::runtime_backend::lexer::lex_line(
            "exile the top card of your library. You may cast that card for as long as it remains exiled.",
            0,
        )
        .expect("prepared lossy effect slice should lex");
        let mut recovered_from_raw_line = placeholder_program();
        bind_dynamic_power_owner_exile_permission(
            &mut recovered_from_raw_line,
            &lossy_tokens,
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
        );
        let recovered_debug = format!("{recovered_from_raw_line:#?}");
        assert!(recovered_debug.contains("PowerOf"), "{recovered_debug}");
        assert!(recovered_debug.contains("OwnerOf"), "{recovered_debug}");
        assert!(
            recovered_debug.contains("cast_pool_is_plural: true"),
            "{recovered_debug}"
        );

        let near_miss = crate::runtime_backend::lexer::lex_line(
            "When enchanted creature dies, exile the top card of its owner's library. You may cast that card for as long as it remains exiled.",
            0,
        )
        .expect("fixed-card near miss should lex");
        let mut unchanged = placeholder_program();
        let before = format!("{unchanged:#?}");
        bind_dynamic_power_owner_exile_permission(
            &mut unchanged,
            &near_miss,
            "When enchanted creature dies, exile the top card of its owner's library. You may cast that card for as long as it remains exiled.",
        );
        assert_eq!(format!("{unchanged:#?}"), before);
    }
}

fn bind_source_exiled_return_complement(program: &mut crate::resolution::ResolutionProgram) {
    for root in program
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        let Some(sequence) = root.downcast_ref::<crate::effects::SequenceEffect>() else {
            continue;
        };
        if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
            || sequence.result_label.is_some()
        {
            continue;
        }
        let [returned_root, remainder_root] = sequence.effects.as_slice() else {
            continue;
        };
        let Some(returned) = returned_root.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        let Some(move_returned) = returned
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
        else {
            continue;
        };
        let ChooseSpec::WithCount(returned_set, count) = move_returned.target.unhinted() else {
            continue;
        };
        let ChooseSpec::Tagged(source_set_tag) = returned_set.unhinted() else {
            continue;
        };
        if source_set_tag.as_str() != crate::tag::SOURCE_EXILED_TAG
            || count.min == 0
            || count.max != Some(count.min)
            || move_returned.zone != Zone::Battlefield
            || move_returned.battlefield_controller != crate::effects::BattlefieldController::Owner
        {
            continue;
        }

        let Some(remainder) = remainder_root
            .downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
        else {
            continue;
        };
        if remainder.tag.as_str() != crate::tag::SOURCE_EXILED_TAG {
            continue;
        }
        let [conditional_root] = remainder.effects.as_slice() else {
            continue;
        };
        let Some(conditional) =
            conditional_root.downcast_ref::<crate::effects::ConditionalEffect>()
        else {
            continue;
        };
        let crate::effect::Condition::TaggedObjectMatches(candidate_tag, filter) =
            &conditional.condition
        else {
            continue;
        };
        let [constraint] = filter.tagged_constraints.as_slice() else {
            continue;
        };
        let mut semantic_base = filter.clone();
        semantic_base.tagged_constraints.clear();
        if candidate_tag.as_str() != "__it__"
            || constraint.tag.as_str() != "__it__"
            || constraint.relation != crate::target::TaggedOpbjectRelation::SameStableId
            || semantic_base != ObjectFilter::default()
            || !conditional.if_true.is_empty()
            || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        {
            continue;
        }
        let [bottom_root] = conditional.if_false.as_slice() else {
            continue;
        };
        let Some(bottom) = bottom_root.downcast_ref::<crate::effects::MoveToZoneEffect>() else {
            continue;
        };
        if bottom.target.unhinted() != &ChooseSpec::Iterated || bottom.zone != Zone::Library {
            continue;
        }

        let mut corrected_filter = filter.clone();
        corrected_filter.tagged_constraints[0].tag = returned.tag.clone();
        let mut corrected_conditional = conditional.clone();
        corrected_conditional.condition =
            crate::effect::Condition::TaggedObjectMatches(candidate_tag.clone(), corrected_filter);
        let mut corrected_remainder = remainder.clone();
        corrected_remainder.effects = vec![crate::effect::Effect::new(corrected_conditional)];
        let mut corrected_sequence = sequence.clone();
        corrected_sequence.effects[1] = crate::effect::Effect::new(corrected_remainder);
        *root = crate::effect::Effect::new(corrected_sequence);
    }
}

/// Restore an authored source-linked complement after conditional antecedent
/// lowering has merged the source noun and the immediately exiled result into
/// the return filter. The exact source grammar plus the tagged exile-until
/// producer prove both persistent membership and the one-result exclusion.
fn reconcile_authored_source_exiled_return_runtime(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let Some(return_idx) = source_tokens
        .iter()
        .rposition(|token| token.is_word("return"))
    else {
        return;
    };
    let Some(surface) =
        crate::runtime_backend::effect_sentences::parse_exiled_with_source_move_surface(
            &source_tokens[return_idx..],
        )
    else {
        return;
    };
    if surface.subject
        != ironsmith_core::ExiledWithSourceSubjectSurface::Custom("each other card".to_string())
        || surface.destination != ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner
    {
        return;
    }

    fn exile_until_tag(effect: &crate::effect::Effect, tags: &mut Vec<crate::TagKey>) {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && tagged
                .effect
                .downcast_ref::<crate::effects::ExileUntilEffect>()
                .is_some()
        {
            tags.push(tagged.tag.clone());
        }
        effect.visit_child_effects(&mut |child| exile_until_tag(child, tags));
    }

    let mut exile_tags = Vec::new();
    for effect in program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
    {
        exile_until_tag(effect, &mut exile_tags);
    }
    let [current_exile_tag] = exile_tags.as_slice() else {
        return;
    };

    let corrected_filter = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .match_tagged(
            crate::TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        )
        .match_tagged(
            current_exile_tag.clone(),
            crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
        );

    fn rewrite_effect(
        effect: &crate::effect::Effect,
        current_exile_tag: &crate::TagKey,
        corrected_filter: &ObjectFilter,
        surface: &ironsmith_core::ExiledWithSourceMoveSurface,
        rewrites: &mut usize,
    ) -> crate::effect::Effect {
        if let Some(returned) =
            effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
            && !returned.tapped
            && !returned.face_down
            && returned.battlefield_controller == crate::effects::BattlefieldController::Owner
            && returned.controller_surface_explicit
            && returned.verb_surface == ironsmith_core::MoveToZoneVerbSurface::Return
            && returned.filter.zone == Some(Zone::Exile)
            && returned.filter.other
            && returned.filter.card_types.is_empty()
            && returned.filter.subtypes == [crate::types::Subtype::Vehicle]
            && returned.filter.tagged_constraints.len() == 1
            && returned.filter.tagged_constraints[0].tag == *current_exile_tag
            && returned.filter.tagged_constraints[0].relation
                == crate::target::TaggedOpbjectRelation::IsTaggedObject
        {
            // The broad return-all lowering inherited the source noun
            // (`Vehicle`) and interpreted `other` relative to the source. The
            // authored source-linked collection instead means every older
            // object exiled with this source, excluding only the object just
            // exiled by this resolution. Convert that proven complement to
            // the ordinary typed move effect, which can carry the exact
            // source-linked presentation as well as its executable filter.
            let moved = crate::effects::MoveToZoneEffect::new(
                ChooseSpec::all(corrected_filter.clone()),
                Zone::Battlefield,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
            .with_target_plural_surface()
            .with_exiled_with_source_surface(surface.clone())
            .under_owner_control();
            *rewrites += 1;
            return crate::effect::Effect::new(moved);
        }
        if let Some(moved) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            && moved.zone == Zone::Battlefield
            && moved.battlefield_controller == crate::effects::BattlefieldController::Owner
            && moved.exiled_with_source_surface.is_none()
            && let ChooseSpec::All(filter) = moved.target.unhinted()
            && filter.zone == Some(Zone::Exile)
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *current_exile_tag
                    && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
            })
        {
            let mut moved = moved.clone();
            moved.target = ChooseSpec::all(corrected_filter.clone());
            moved.target_plural_surface = true;
            moved.exiled_with_source_surface = Some(surface.clone());
            *rewrites += 1;
            return crate::effect::Effect::new(moved);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            conditional.if_true = conditional
                .if_true
                .iter()
                .map(|child| {
                    rewrite_effect(
                        child,
                        current_exile_tag,
                        corrected_filter,
                        surface,
                        rewrites,
                    )
                })
                .collect();
            conditional.if_false = conditional
                .if_false
                .iter()
                .map(|child| {
                    rewrite_effect(
                        child,
                        current_exile_tag,
                        corrected_filter,
                        surface,
                        rewrites,
                    )
                })
                .collect();
            return crate::effect::Effect::new(conditional);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            let mut if_effect = if_effect.clone();
            if_effect.then = if_effect
                .then
                .iter()
                .map(|child| {
                    rewrite_effect(
                        child,
                        current_exile_tag,
                        corrected_filter,
                        surface,
                        rewrites,
                    )
                })
                .collect();
            if_effect.else_ = if_effect
                .else_
                .iter()
                .map(|child| {
                    rewrite_effect(
                        child,
                        current_exile_tag,
                        corrected_filter,
                        surface,
                        rewrites,
                    )
                })
                .collect();
            return crate::effect::Effect::new(if_effect);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence
                .effects
                .iter()
                .map(|child| {
                    rewrite_effect(
                        child,
                        current_exile_tag,
                        corrected_filter,
                        surface,
                        rewrites,
                    )
                })
                .collect();
            return crate::effect::Effect::new(sequence);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(rewrite_effect(
                &tagged.effect,
                current_exile_tag,
                corrected_filter,
                surface,
                rewrites,
            ));
            return crate::effect::Effect::new(tagged);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(rewrite_effect(
                &with_id.effect,
                current_exile_tag,
                corrected_filter,
                surface,
                rewrites,
            ));
            return crate::effect::Effect::new(with_id);
        }
        effect.clone()
    }

    let original = program.clone();
    let mut rewrites = 0usize;
    for effect in program
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        *effect = rewrite_effect(
            effect,
            current_exile_tag,
            &corrected_filter,
            &surface,
            &mut rewrites,
        );
    }
    if rewrites != 1 {
        *program = original;
    }
}

/// A singular source-linked exile subject is an object choice, not an `all`
/// selection. Reference resolution can discover the durable source-exile tag
/// only after AST normalization, so make the same exact correction on the
/// completed runtime program while preserving wrappers and result IDs.
fn normalize_singular_source_exiled_runtime_move(
    program: &mut crate::resolution::ResolutionProgram,
) {
    fn rewrite(effect: &crate::effect::Effect) -> crate::effect::Effect {
        if let Some(moved) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            && !moved.target_plural_surface
            && moved.zone == Zone::Graveyard
            && let ChooseSpec::All(filter) = moved.target.unhinted()
            && filter.zone == Some(Zone::Exile)
            && let [constraint] = filter.tagged_constraints.as_slice()
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation
                == crate::target::TaggedOpbjectRelation::IsTaggedObject
        {
            let mut semantic = filter.clone();
            semantic.zone = None;
            semantic.tagged_constraints.clear();
            semantic.union_surface = Default::default();
            if semantic == ObjectFilter::default() {
                let mut moved = moved.clone();
                moved.target = ChooseSpec::Object(filter.clone());
                return crate::effect::Effect::new(moved);
            }
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(rewrite(&with_id.effect));
            return crate::effect::Effect::new(with_id);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(rewrite(&tagged.effect));
            return crate::effect::Effect::new(tagged);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence.effects.iter().map(rewrite).collect();
            return crate::effect::Effect::new(sequence);
        }
        effect.clone()
    }

    for effect in program
        .segments
        .iter_mut()
        .flat_map(|segment| segment.default_effects.iter_mut())
    {
        *effect = rewrite(effect);
    }
}

/// Remove a sentence-parser target prelude when the immediately following
/// effect is the authored counted declaration of the same target domain. The
/// counted declaration remains in both the executable program and announced
/// choices; retaining the plain prelude would render and consume a duplicate
/// target selection.
fn dedupe_lowered_adjacent_target_declarations(program: &mut crate::resolution::ResolutionProgram) {
    for segment in &mut program.segments {
        let mut index = 0usize;
        while index + 1 < segment.default_effects.len() {
            let first =
                segment.default_effects[index].downcast_ref::<crate::effects::TargetOnlyEffect>();
            let second = segment.default_effects[index + 1]
                .downcast_ref::<crate::effects::TargetOnlyEffect>();
            let duplicate = matches!((first, second), (Some(first), Some(second))
                if !first.explicit_declaration
                    && second.explicit_declaration
                    && first.chooser.is_none()
                    && second.chooser.is_none()
                    && first.target.is_target()
                    && second.target.is_target()
                    && first.target.base() == second.target.base()
                    && first.target.count().min == 1
                    && first.target.count().max == Some(1)
                    && second.target.count().min == 0
                    && second.target.count().max == Some(1));
            if duplicate {
                segment.default_effects.remove(index);
            } else {
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod graveyard_card_copy_cast_program_normalization_tests {
    use super::*;
    use crate::CardType;

    fn program(exile_tag: &str, cast_tag: &str) -> crate::resolution::ResolutionProgram {
        let exile =
            crate::effect::Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::target(
                ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard)),
            )))
            .tag(exile_tag);
        let copy = crate::effect::Effect::with_id(
            7,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(crate::TagKey::from(
                    exile_tag,
                )))
                .with_target_reference_pronoun(true),
            ),
        )
        .tag(crate::cards::builders::COPIED_STACK_OBJECT_TAG);
        let producer =
            crate::effect::Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                exile, copy,
            ]));
        let cast = crate::effect::Effect::new(
            crate::effects::CastTaggedEffect::new(cast_tag, PlayerFilter::You).as_copy(),
        );
        let may = crate::effect::Effect::may(vec![cast]);
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![producer]),
            crate::resolution::ResolutionSegment::from_effects(vec![may]),
        ])
    }

    #[test]
    fn exact_shared_tag_replaces_the_invalid_stack_copy_with_the_card_copy_cast() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut normalized = program(tag, tag);
        normalize_graveyard_card_copy_cast_program(&mut normalized);
        let debug = format!("{normalized:#?}");
        assert!(!debug.contains("CopySpellEffect"), "{debug}");
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");

        let mut wrong_tag = program(tag, "__sentence_helper_exiled_l0_s9_e9");
        normalize_graveyard_card_copy_cast_program(&mut wrong_tag);
        assert!(
            format!("{wrong_tag:#?}").contains("CopySpellEffect"),
            "an unrelated copied-card cast must not consume the producer marker"
        );
    }

    #[test]
    fn conditional_union_graveyard_domain_uses_the_copied_card_cast() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut union = ObjectFilter::default();
        union.any_of = vec![
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_type(CardType::Creature),
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_ability_marker("freerunning"),
        ];
        let producer = crate::effect::Effect::with_id(
            11,
            crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::target(ChooseSpec::Object(union)),
                Zone::Exile,
                true,
            ))
            .tag(tag),
        );
        let obsolete_copy = crate::effect::Effect::with_id(
            11,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from(tag)))
                    .with_target_reference_pronoun(true),
            ),
        )
        .tag(crate::cards::builders::COPIED_STACK_OBJECT_TAG);
        let gate = crate::effect::Effect::new(crate::effects::IfEffect::if_then(
            crate::effect::EffectId(11),
            crate::effect::EffectPredicate::Happened,
            vec![obsolete_copy],
        ));
        let may_cast = crate::effect::Effect::may(vec![crate::effect::Effect::new(
            crate::effects::CastTaggedEffect::new(tag, PlayerFilter::You).as_copy(),
        )]);
        let mut program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![producer]),
            crate::resolution::ResolutionSegment::from_effects(vec![gate]),
            crate::resolution::ResolutionSegment::from_effects(vec![may_cast]),
        ]);

        normalize_graveyard_card_copy_cast_program(&mut program);
        let debug = format!("{program:#?}");
        assert!(!debug.contains("CopySpellEffect"), "{debug}");
        assert!(debug.contains("IfEffect"), "{debug}");
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert_eq!(program.segments.len(), 2, "{debug}");
    }
}

/// Reconcile one face-up card exiled from a targeted opponent's library with
/// the conditional cast attempt and failed-attempt permission that follow it.
/// Sentence-local parsers may allocate fresh helper tags (or inherit the
/// intervening Treasure tag); this exact typed three-segment shape proves all
/// three references denote the sole card moved by the producer.
fn bind_exile_top_card_cast_attempt_and_fallback(
    program: &mut crate::resolution::ResolutionProgram,
) {
    let [producer_segment, cast_segment, fallback_segment] = program.segments.as_slice() else {
        return;
    };
    if [producer_segment, cast_segment, fallback_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return;
    }
    let [producer] = producer_segment.default_effects.as_slice() else {
        return;
    };
    let Some(producer) = producer.downcast_ref::<crate::effects::SequenceEffect>() else {
        return;
    };
    if producer.surface != ironsmith_core::SequenceSurface::Coordinated
        || producer.result_label.is_some()
    {
        return;
    }
    let [target, exile, created] = producer.effects.as_slice() else {
        return;
    };
    let Some(target) = target.downcast_ref::<crate::effects::TargetOnlyEffect>() else {
        return;
    };
    let Some(exile) = exile.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() else {
        return;
    };
    let Some(created) = created.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(token) = created
        .effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()
    else {
        return;
    };
    let [exile_tag] = exile.moved_tags.as_slice() else {
        return;
    };
    if target.target != ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent))
        || target.chooser.is_some()
        || target.explicit_declaration
        || exile.count.unhinted() != &crate::effect::Value::Fixed(1)
        || exile.player != PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
        || exile.surface.is_some()
        || !exile.accumulated_tags.is_empty()
        || exile.face_down
        || created.tag == *exile_tag
        || token.count.unhinted() != &crate::effect::Value::Fixed(1)
        || token.controller != PlayerFilter::You
        || token.token.card.card_types != [crate::CardType::Artifact]
        || token.token.card.subtypes != [crate::Subtype::Treasure]
    {
        return;
    }

    let [cast_root] = cast_segment.default_effects.as_slice() else {
        return;
    };
    let Some(cast_sequence) = cast_root.downcast_ref::<crate::effects::SequenceEffect>() else {
        return;
    };
    if cast_sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen
        || cast_sequence.result_label.is_some()
    {
        return;
    }
    let [with_id_root] = cast_sequence.effects.as_slice() else {
        return;
    };
    let Some(with_id) = with_id_root.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    let Some(conditional) = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return;
    };
    let crate::effect::Condition::TaggedObjectMatches(_, predicate) = &conditional.condition else {
        return;
    };
    if conditional.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || !conditional.if_false.is_empty()
    {
        return;
    }
    let [may_root] = conditional.if_true.as_slice() else {
        return;
    };
    let Some(may) = may_root.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    let [cast_root] = may.effects.as_slice() else {
        return;
    };
    let Some(cast) = cast_root.downcast_ref::<crate::effects::CastTaggedEffect>() else {
        return;
    };
    if cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
    {
        return;
    }

    let [fallback_root] = fallback_segment.default_effects.as_slice() else {
        return;
    };
    let Some(fallback) = fallback_root.downcast_ref::<crate::effects::IfEffect>() else {
        return;
    };
    let [grant_root] = fallback.then.as_slice() else {
        return;
    };
    let Some(grant) = grant_root.downcast_ref::<crate::effects::GrantPlayTaggedEffect>() else {
        return;
    };
    if fallback.condition != with_id.id
        || fallback.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
    {
        return;
    }

    let mut cast = cast.clone();
    cast.tag = exile_tag.clone();
    let mut may = may.clone();
    may.effects = vec![crate::effect::Effect::new(cast)];
    let mut conditional = conditional.clone();
    conditional.condition =
        crate::effect::Condition::TaggedObjectMatches(exile_tag.clone(), predicate.clone());
    conditional.if_true = vec![crate::effect::Effect::new(may)];
    let mut with_id = with_id.clone();
    with_id.effect = Box::new(crate::effect::Effect::new(conditional));
    let mut cast_sequence = cast_sequence.clone();
    cast_sequence.effects = vec![crate::effect::Effect::new(with_id)];

    let mut grant = grant.clone();
    grant.tag = exile_tag.clone();
    let mut fallback = fallback.clone();
    fallback.then = vec![crate::effect::Effect::new(grant)];

    program.segments[1].default_effects = vec![crate::effect::Effect::new(cast_sequence)];
    program.segments[2].default_effects = vec![crate::effect::Effect::new(fallback)];
}

fn source_words_contain(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
    phrase: &[&str],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    words.windows(phrase.len()).any(|window| window == phrase)
}

/// Recover a lexical negated-control predicate that the generic sentence
/// carry path can otherwise reinterpret as failure of the preceding zone
/// move. The runtime shape remains wholly typed: the moved-object tag proves
/// the toughness antecedent and the source tokens prove the independent
/// `you don't control` clause.
fn bind_negated_control_condition_after_tagged_zone_move(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    fn exact_tagged_exile_result(
        effect: &crate::effect::Effect,
    ) -> Option<(crate::effect::EffectId, crate::TagKey)> {
        fn inspect(
            effect: &crate::effect::Effect,
            id: Option<crate::effect::EffectId>,
            tag: Option<crate::TagKey>,
        ) -> Option<(crate::effect::EffectId, crate::TagKey)> {
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return inspect(&with_id.effect, Some(with_id.id), tag);
            }
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                return inspect(&tagged.effect, id, Some(tagged.tag.clone()));
            }
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                let [inner] = sequence.effects.as_slice() else {
                    return None;
                };
                return inspect(inner, id, tag);
            }
            let moved = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
            if moved.zone != Zone::Exile || !moved.target.is_target() {
                return None;
            }
            Some((id?, tag?))
        }

        inspect(effect, None, None)
    }

    fn rewrite_exact_failure_branch(
        effect: &crate::effect::Effect,
        move_id: crate::effect::EffectId,
        moved_tag: &crate::TagKey,
        condition: &crate::effect::Condition,
    ) -> Option<crate::effect::Effect> {
        fn is_exact_tagged_toughness_loss(
            effect: &crate::effect::Effect,
            moved_tag: &crate::TagKey,
        ) -> bool {
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return is_exact_tagged_toughness_loss(&with_id.effect, moved_tag);
            }
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                return is_exact_tagged_toughness_loss(&tagged.effect, moved_tag);
            }
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                let [inner] = sequence.effects.as_slice() else {
                    return false;
                };
                return is_exact_tagged_toughness_loss(inner, moved_tag);
            }
            effect
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .is_some_and(|loss| {
                    loss.player == PlayerFilter::You
                        && matches!(
                            loss.amount.unhinted(),
                            crate::effect::Value::ToughnessOf(spec)
                                if matches!(
                                    spec.unhinted(),
                                    ChooseSpec::Tagged(tag) if tag == moved_tag
                                )
                        )
                })
        }

        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut rewritten = with_id.clone();
            rewritten.effect = Box::new(rewrite_exact_failure_branch(
                &with_id.effect,
                move_id,
                moved_tag,
                condition,
            )?);
            return Some(crate::effect::Effect::new(rewritten));
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut rewritten = tagged.clone();
            rewritten.effect = Box::new(rewrite_exact_failure_branch(
                &tagged.effect,
                move_id,
                moved_tag,
                condition,
            )?);
            return Some(crate::effect::Effect::new(rewritten));
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let [inner] = sequence.effects.as_slice() else {
                return None;
            };
            let mut rewritten = sequence.clone();
            rewritten.effects = vec![rewrite_exact_failure_branch(
                inner, move_id, moved_tag, condition,
            )?];
            return Some(crate::effect::Effect::new(rewritten));
        }

        let failed = effect.downcast_ref::<crate::effects::IfEffect>()?;
        if failed.condition != move_id
            || failed.predicate != crate::effect::EffectPredicate::DidNotHappen
            || !failed.else_.is_empty()
        {
            return None;
        }
        let [loss_root] = failed.then.as_slice() else {
            return None;
        };
        if !is_exact_tagged_toughness_loss(loss_root, moved_tag) {
            return None;
        }

        Some(crate::effect::Effect::new(
            crate::effects::ConditionalEffect::new(
                condition.clone(),
                failed.then.clone(),
                Vec::new(),
            ),
        ))
    }

    let source_words = crate::runtime_backend::token_word_refs(source_tokens);
    if !source_words.contains(&"human")
        || !source_words.contains(&"control")
        || !source_words.contains(&"if")
    {
        return;
    }
    let [move_segment, condition_segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !move_segment.self_replacements.is_empty() || !condition_segment.self_replacements.is_empty()
    {
        return;
    }
    let [move_root] = move_segment.default_effects.as_slice() else {
        return;
    };
    let Some((move_id, moved_tag)) = exact_tagged_exile_result(move_root) else {
        return;
    };
    let [condition_root] = condition_segment.default_effects.as_slice() else {
        return;
    };

    let human = ObjectFilter::creature()
        .with_subtype(crate::types::Subtype::Human)
        .controlled_by(PlayerFilter::You);
    let condition =
        crate::effect::Condition::Not(Box::new(crate::effect::Condition::PlayerControls {
            player: PlayerFilter::You,
            filter: human,
        }));
    let Some(rewritten) =
        rewrite_exact_failure_branch(condition_root, move_id, &moved_tag, &condition)
    else {
        return;
    };
    condition_segment.default_effects = vec![rewritten];
}

/// Normalize an authored target-characteristic OR paid-mana gate after the
/// broad conditional chain has nested the characteristic test inside the
/// paid-mana arm. The authoritative lowering route emits a tagged target-only
/// declaration followed by the characteristic-gated action. Preserve that
/// declaration, move the characteristic test before the action, and make the
/// destroy consume the declared tag so the target is announced exactly once.
/// Unrelated nested conditions are left untouched.
fn bind_target_characteristic_or_paid_mana_condition(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let source_words = crate::runtime_backend::token_word_refs(source_tokens);
    if !source_words.contains(&"creature")
        || !source_words.contains(&"or")
        || !source_words_contain(
            source_tokens,
            &["was", "spent", "to", "cast", "this", "spell"],
        )
    {
        return;
    }
    let [segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [root] = segment.default_effects.as_slice() else {
        return;
    };
    let Some(with_id) = root.downcast_ref::<crate::effects::WithIdEffect>() else {
        return;
    };
    let Some(outer) = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return;
    };
    if outer.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || !outer.if_false.is_empty()
        || !matches!(
            &outer.condition,
            crate::effect::Condition::And(left, right)
                if matches!(left.as_ref(), crate::effect::Condition::ManaSpentToCastThisSpellAtLeast { amount: 1, symbol: Some(crate::mana::ManaSymbol::Green) })
                    && matches!(right.as_ref(), crate::effect::Condition::ManaSpentToCastThisSpellAtLeast { amount: 1, symbol: Some(crate::mana::ManaSymbol::White) })
        )
    {
        return;
    }
    let [target_root, characteristic_root] = outer.if_true.as_slice() else {
        return;
    };
    let Some(target_tagged) = target_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(target_only) = target_tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return;
    };
    let Some(characteristic) =
        characteristic_root.downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return;
    };
    let crate::effect::Condition::TaggedObjectMatches(tag, creature_filter) =
        &characteristic.condition
    else {
        return;
    };
    let [replacement_destroy_root] = characteristic.if_true.as_slice() else {
        return;
    };
    let Some(replacement_tagged) =
        replacement_destroy_root.downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let Some(replacement_destroy) = replacement_tagged
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()
    else {
        return;
    };
    if tag != &target_tagged.tag
        || !characteristic.if_false.is_empty()
        || characteristic.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || target_only.target != replacement_destroy.spec
        || !target_only.target.is_target()
        || target_only.chooser.is_some()
        || target_only.explicit_declaration
        || creature_filter.card_types != [crate::types::CardType::Creature]
    {
        return;
    }

    let mut outer = outer.clone();
    outer.condition = crate::effect::Condition::Or(
        Box::new(crate::effect::Condition::TaggedObjectMatches(
            tag.clone(),
            creature_filter.clone(),
        )),
        Box::new(outer.condition),
    );
    let mut replacement_destroy = replacement_destroy.clone();
    replacement_destroy.spec = ChooseSpec::Tagged(tag.clone());
    outer.if_true = vec![crate::effect::Effect::new(replacement_destroy).tag(tag.clone())];
    let mut with_id = with_id.clone();
    with_id.effect = Box::new(crate::effect::Effect::new(outer));
    segment.default_effects = vec![target_root.clone(), crate::effect::Effect::new(with_id)];
}

/// Preserve the owner of a watched object inside a delayed graveyard return.
/// The delayed trigger's event snapshot is tagged explicitly so the nested
/// graveyard filter can resolve the watched creature's owner after that
/// creature has left the battlefield.
fn bind_delayed_return_to_watched_object_owner(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    fn target_only(effect: &crate::effect::Effect) -> Option<&crate::effects::TargetOnlyEffect> {
        if let Some(target) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
            return Some(target);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return target_only(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return target_only(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        target_only(inner)
    }

    fn tagged_target(
        effect: &crate::effect::Effect,
    ) -> Option<(&crate::TagKey, &crate::effects::TargetOnlyEffect)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return Some((&tagged.tag, target_only(&tagged.effect)?));
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return tagged_target(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        tagged_target(inner)
    }

    fn delayed_schedule(
        effect: &crate::effect::Effect,
    ) -> Option<&crate::effects::ScheduleDelayedTriggerEffect> {
        if let Some(schedule) =
            effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        {
            return Some(schedule);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return delayed_schedule(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return delayed_schedule(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        delayed_schedule(inner)
    }

    fn delayed_payload(
        effects: &[crate::effect::Effect],
    ) -> Option<(&crate::effects::ChooseObjectsEffect, &crate::effect::Effect)> {
        fn choose_objects(
            effect: &crate::effect::Effect,
        ) -> Option<&crate::effects::ChooseObjectsEffect> {
            if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
                return Some(choose);
            }
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                return choose_objects(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return choose_objects(&with_id.effect);
            }
            let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            let [inner] = sequence.effects.as_slice() else {
                return None;
            };
            choose_objects(inner)
        }

        fn payload_effects(effects: &[crate::effect::Effect]) -> &[crate::effect::Effect] {
            let [root] = effects else {
                return effects;
            };
            fn paired_sequence(effect: &crate::effect::Effect) -> Option<&[crate::effect::Effect]> {
                if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                    return paired_sequence(&tagged.effect);
                }
                if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                    return paired_sequence(&with_id.effect);
                }
                let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
                (sequence.effects.len() == 2).then_some(sequence.effects.as_slice())
            }
            paired_sequence(root).unwrap_or(effects)
        }

        let effects = payload_effects(effects);
        let [choose_root, move_root] = effects else {
            return None;
        };
        let choose = choose_objects(choose_root)?;
        Some((choose, move_root))
    }

    fn tagged_move_to_zone(
        effect: &crate::effect::Effect,
    ) -> Option<(&crate::TagKey, &crate::effects::MoveToZoneEffect)> {
        fn move_to_zone(
            effect: &crate::effect::Effect,
        ) -> Option<&crate::effects::MoveToZoneEffect> {
            if let Some(moved) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
                return Some(moved);
            }
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return move_to_zone(&with_id.effect);
            }
            let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            let [inner] = sequence.effects.as_slice() else {
                return None;
            };
            move_to_zone(inner)
        }

        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return Some((&tagged.tag, move_to_zone(&tagged.effect)?));
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return tagged_move_to_zone(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        tagged_move_to_zone(inner)
    }

    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let from_watched_owner_graveyard = words.windows(4).any(|window| {
        matches!(
            window,
            [
                "from",
                "its",
                "owner" | "owners" | "owner's" | "owners'",
                "graveyard"
            ]
        )
    }) || words
        .windows(5)
        .any(|window| window == ["from", "its", "owner", "s", "graveyard"]);
    if !from_watched_owner_graveyard
        || !words
            .windows(6)
            .any(|window| window == ["when", "that", "creature", "dies", "this", "turn"])
    {
        return;
    }
    let (target_effect, delayed_root, single_segment) = match program.segments.as_slice() {
        [segment] if segment.self_replacements.is_empty() => {
            let [target_effect, delayed_root] = segment.default_effects.as_slice() else {
                return;
            };
            (target_effect.clone(), delayed_root.clone(), true)
        }
        [target_segment, delayed_segment]
            if target_segment.self_replacements.is_empty()
                && delayed_segment.self_replacements.is_empty() =>
        {
            let [target_effect] = target_segment.default_effects.as_slice() else {
                return;
            };
            let [delayed_root] = delayed_segment.default_effects.as_slice() else {
                return;
            };
            (target_effect.clone(), delayed_root.clone(), false)
        }
        _ => return,
    };
    let (existing_target_tag, target_only) =
        if let Some((tag, target)) = tagged_target(&target_effect) {
            (Some(tag.clone()), target.clone())
        } else if let Some(target) = target_only(&target_effect) {
            (None, target.clone())
        } else {
            return;
        };
    let ChooseSpec::Target(target_inner) = target_only.target.unhinted() else {
        return;
    };
    let ChooseSpec::Object(target_filter) = target_inner.unhinted() else {
        return;
    };
    let mut semantic_target_filter = target_filter.clone();
    semantic_target_filter.union_surface = Default::default();
    if semantic_target_filter != ObjectFilter::creature().in_zone(crate::zone::Zone::Battlefield)
        || target_only.chooser.is_some()
        || !target_only.explicit_declaration
    {
        return;
    }
    let Some(schedule) = delayed_schedule(&delayed_root) else {
        return;
    };
    let target_tag = existing_target_tag
        .or_else(|| schedule.target_tag.clone())
        .unwrap_or_else(|| crate::TagKey::from("targeted_0"));
    if schedule
        .target_tag
        .as_ref()
        .is_some_and(|tag| tag != &target_tag)
        || !schedule.one_shot
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::EndOfTurn
        || !schedule.until_end_of_turn
        || schedule.start_next_turn
        || schedule.controller != PlayerFilter::You
    {
        return;
    }
    let Some((choose, move_root)) = delayed_payload(&schedule.effects) else {
        return;
    };
    let Some((_moved_tag, moved)) = tagged_move_to_zone(move_root) else {
        return;
    };
    let mut semantic_choice_filter = choose.filter.clone();
    semantic_choice_filter.union_surface = Default::default();
    if semantic_choice_filter != ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard)
        || choose.count != crate::ChoiceCount::exactly(1)
        || choose.chooser != PlayerFilter::You
        || !matches!(moved.target.unhinted(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || moved.zone != crate::zone::Zone::Battlefield
        || moved.battlefield_controller != crate::effects::BattlefieldController::Owner
        || !moved.controller_surface_explicit
    {
        return;
    }

    let triggering_tag = crate::TagKey::from("triggering");
    let owner =
        PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::tagged(triggering_tag.clone()));
    let mut choose = choose.clone();
    choose.filter.owner = Some(owner);
    let mut schedule = schedule.clone();
    schedule.target_tag = Some(target_tag.clone());
    schedule.effects = vec![
        crate::effect::Effect::new(crate::effects::TagTriggeringObjectEffect::new(
            triggering_tag,
        )),
        crate::effect::Effect::new(choose),
        move_root.clone(),
    ];
    let target_effect = crate::effect::Effect::new(target_only).tag(target_tag);
    let delayed_effect = crate::effect::Effect::new(schedule);
    if single_segment {
        program.segments[0].default_effects = vec![target_effect, delayed_effect];
    } else {
        program.segments[0].default_effects = vec![target_effect];
        program.segments[1].default_effects = vec![delayed_effect];
    }
}

fn normalize_each_creature_except_controlled_flying_damage(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    if !source_words_contain(
        source_tokens,
        &[
            "except",
            "for",
            "creatures",
            "you",
            "control",
            "with",
            "flying",
        ],
    ) {
        return;
    }
    let [segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [root] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let (for_each, outer_id) =
        if let Some(with_id) = root.downcast_ref::<crate::effects::WithIdEffect>() {
            let Some(for_each) = with_id
                .effect
                .downcast_ref::<crate::effects::ForEachObject>()
            else {
                return;
            };
            (for_each, Some(with_id.id))
        } else if let Some(for_each) = root.downcast_ref::<crate::effects::ForEachObject>() {
            (for_each, None)
        } else {
            return;
        };
    let expected_exception = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .you_control()
        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    let [damage_root] = for_each.effects.as_slice() else {
        return;
    };
    let Some(tagged_damage) = damage_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(damage) = tagged_damage
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return;
    };
    let mut semantic_exception = for_each.filter.clone();
    semantic_exception.union_surface = Default::default();
    if semantic_exception != expected_exception
        || !matches!(damage.target.unhinted(), ChooseSpec::Iterated)
    {
        return;
    }

    let mut affected = ObjectFilter::creature().in_zone(Zone::Battlefield);
    affected.any_of = vec![
        ObjectFilter::default().controlled_by(PlayerFilter::NotYou),
        ObjectFilter::default()
            .without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
    ];
    let replacement = crate::effect::Effect::new(crate::effects::ForEachObject::new(
        affected,
        for_each.effects.clone(),
    ));
    *root = outer_id.map_or(replacement.clone(), |id| {
        crate::effect::Effect::with_id(id.0, replacement)
    });
}

/// Preserve the target-eligibility qualifier in "target player who lost life
/// this turn" after the generic subject parser has reduced that subject to a
/// plain target player. The complete typed target declaration and its linked
/// life-loss consumer must both carry the same filter so legality is enforced
/// when the target is announced as well as when the effect resolves.
fn bind_target_player_lost_life_this_turn_qualifier(
    program: &mut crate::resolution::ResolutionProgram,
    choices: &mut Vec<ChooseSpec>,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    fn target_only(effect: &crate::effect::Effect) -> Option<&crate::effects::TargetOnlyEffect> {
        if let Some(target) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
            return Some(target);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return target_only(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return target_only(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        target_only(inner)
    }

    fn lose_life(effect: &crate::effect::Effect) -> Option<&crate::effects::LoseLifeEffect> {
        if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
            return Some(lose);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return lose_life(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return lose_life(&with_id.effect);
        }
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [inner] = sequence.effects.as_slice() else {
            return None;
        };
        lose_life(inner)
    }

    fn paired_effects(effects: &[crate::effect::Effect]) -> &[crate::effect::Effect] {
        let [root] = effects else {
            return effects;
        };
        fn sequence_pair(effect: &crate::effect::Effect) -> Option<&[crate::effect::Effect]> {
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                return sequence_pair(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return sequence_pair(&with_id.effect);
            }
            let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            (sequence.effects.len() == 2).then_some(sequence.effects.as_slice())
        }
        sequence_pair(root).unwrap_or(effects)
    }

    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(8).any(|window| {
        window
            == [
                "target", "player", "who", "lost", "life", "this", "turn", "loses",
            ]
    }) {
        return;
    }
    let [segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let effects = paired_effects(&segment.default_effects);
    let (target_root, lose_root) = match effects {
        [target_root, lose_root] => (Some(target_root), lose_root),
        [lose_root] => (None, lose_root),
        _ => return,
    };
    let [declared_choice] = choices.as_mut_slice() else {
        return;
    };
    let mut target_only = target_root.and_then(target_only).cloned();
    let Some(mut lose_life) = lose_life(lose_root).cloned() else {
        return;
    };
    if target_only.as_ref().is_some_and(|target_only| {
        !matches!(
            target_only.target.unhinted(),
            ChooseSpec::Target(inner)
                if matches!(inner.unhinted(), ChooseSpec::Player(PlayerFilter::Any))
        ) || target_only.chooser.is_some()
    }) || !matches!(
        declared_choice.unhinted(),
        ChooseSpec::Target(inner)
            if matches!(inner.unhinted(), ChooseSpec::Player(PlayerFilter::Any))
    ) || !(lose_life.player == PlayerFilter::Any
        || matches!(
            &lose_life.player,
            PlayerFilter::Target(inner) if inner.as_ref() == &PlayerFilter::Any
        ))
    {
        return;
    }

    let eligible = PlayerFilter::lost_life_this_turn(PlayerFilter::Any);
    let declared_target = ChooseSpec::target(ChooseSpec::Player(eligible.clone()));
    if let Some(target_only) = target_only.as_mut() {
        target_only.target = declared_target.clone();
    }
    *declared_choice = declared_target;
    lose_life.player = PlayerFilter::Target(Box::new(eligible));
    segment.default_effects = if let Some(target_only) = target_only {
        vec![
            crate::effect::Effect::new(target_only),
            crate::effect::Effect::new(lose_life),
        ]
    } else {
        vec![crate::effect::Effect::new(lose_life)]
    };
}

#[cfg(test)]
mod delayed_copy_retarget_transport_tests {
    use super::*;

    fn copy_schedule(
        copied_tag: &str,
        reference_kind: crate::filter::StackObjectKind,
    ) -> crate::effect::Effect {
        let triggering_source_tag = crate::TagKey::from("triggering_source");
        let copy = crate::effect::Effect::with_id(
            0,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::new(
                    ChooseSpec::Tagged(triggering_source_tag.clone()),
                    crate::effect::Value::Fixed(1),
                )
                .with_target_reference_kind(reference_kind),
            ),
        )
        .tag(crate::TagKey::from(copied_tag));
        crate::effect::Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
            crate::effect::DelayedTriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            vec![
                crate::effect::Effect::new(crate::effects::TagTriggeringSourceEffect::new(
                    triggering_source_tag,
                )),
                copy,
            ],
            false,
            Vec::new(),
            PlayerFilter::You,
        ))
    }

    fn plural_copy_retarget() -> crate::effect::Effect {
        crate::effect::Effect::may_player(
            PlayerFilter::You,
            vec![crate::effect::Effect::new(
                crate::effects::RetargetStackObjectEffect::new(ChooseSpec::Tagged(
                    crate::TagKey::from(crate::cards::builders::COPIED_STACK_OBJECT_TAG),
                ))
                .with_plural_copy_reference(),
            )],
        )
    }

    fn two_segment_program(
        schedule: crate::effect::Effect,
    ) -> crate::resolution::ResolutionProgram {
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![schedule]),
            crate::resolution::ResolutionSegment::from_effects(vec![plural_copy_retarget()]),
        ])
    }

    #[test]
    fn tagged_with_id_ability_copy_owns_its_plural_retarget() {
        let mut program = two_segment_program(copy_schedule(
            crate::cards::builders::COPIED_STACK_OBJECT_TAG,
            crate::filter::StackObjectKind::Ability,
        ));
        transport_plural_copy_retarget_into_delayed_trigger(&mut program);

        let [segment] = program.segments.as_slice() else {
            panic!("retarget sibling should be absorbed into its schedule: {program:#?}");
        };
        let [schedule] = segment.default_effects.as_slice() else {
            panic!("outer program should retain only the schedule: {segment:#?}");
        };
        let schedule = schedule
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .expect("outer effect should remain a delayed trigger schedule");
        assert_eq!(schedule.effects.len(), 3, "{schedule:#?}");
        assert!(
            schedule.effects[2]
                .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
                .is_some(),
            "the plural retarget May must execute after the delayed copy: {schedule:#?}"
        );
    }

    #[test]
    fn wrong_copy_tag_or_stack_kind_does_not_transport_retarget() {
        for schedule in [
            copy_schedule("ordinary_result", crate::filter::StackObjectKind::Ability),
            copy_schedule(
                crate::cards::builders::COPIED_STACK_OBJECT_TAG,
                crate::filter::StackObjectKind::Spell,
            ),
        ] {
            let mut program = two_segment_program(schedule);
            transport_plural_copy_retarget_into_delayed_trigger(&mut program);
            assert_eq!(
                program.segments.len(),
                2,
                "near miss must keep its outer retarget sibling: {program:#?}"
            );
        }
    }
}

fn bind_demonstrative_land_self_replacement_to_triggering_object(
    program: &mut crate::resolution::ResolutionProgram,
) {
    fn effect_has_explicit_land_target(effect: &crate::effect::Effect) -> bool {
        if effect
            .target_spec()
            .and_then(choose_spec_object_filter)
            .is_some_and(|filter| {
                (filter.card_types.contains(&crate::CardType::Land)
                    && !filter.excluded_card_types.contains(&crate::CardType::Land))
                    || filter
                        .subtypes
                        .iter()
                        .any(crate::Subtype::is_basic_land_type)
            })
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found {
                found = effect_has_explicit_land_target(child);
            }
        });
        found
    }

    for segment in &mut program.segments {
        let has_explicit_land_target = segment
            .default_effects
            .iter()
            .any(effect_has_explicit_land_target);
        if has_explicit_land_target {
            continue;
        }

        for branch in &mut segment.self_replacements {
            let triggering_filter = match &branch.condition {
                crate::effect::Condition::TargetMatches(filter)
                | crate::effect::Condition::SourceMatches(filter)
                    if filter.demonstrative_antecedent_surface()
                        == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land) =>
                {
                    Some(filter.clone())
                }
                crate::effect::Condition::TaggedObjectMatches(tag, filter)
                    if tag.as_str() != "triggering"
                        && filter.demonstrative_antecedent_surface()
                            == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land) =>
                {
                    Some(filter.clone())
                }
                _ => None,
            };
            if let Some(filter) = triggering_filter {
                branch.condition = crate::effect::Condition::TaggedObjectMatches(
                    crate::TagKey::from("triggering"),
                    filter,
                );
            }
        }
    }
}

/// Keep a final "that creature would die" replacement bound to the creature
/// damaged in the first sentence when an intervening sentence targets an
/// Equipment attached to it. The ordinary last-object reference is the
/// Equipment in the public document route, so require the complete authored
/// wording and the exact typed three-segment relationship before correcting
/// only the replacement target tag.
fn bind_linked_damage_attachment_death_replacement(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(5)
        .any(|window| window == ["attached", "to", "that", "creature", "if"])
        || !words
            .windows(7)
            .any(|window| window == ["if", "that", "creature", "would", "die", "this", "turn"])
    {
        return;
    }

    let [damage_segment, attachment_segment, replacement_segment] = program.segments.as_mut_slice()
    else {
        return;
    };
    if [
        &*damage_segment,
        &*attachment_segment,
        &*replacement_segment,
    ]
    .iter()
    .any(|segment| {
        !segment.self_replacements.is_empty()
            || segment.starts_new_source_line
            || segment.default_effects.len() != 1
    }) {
        return;
    }

    let Some(damage_tagged) =
        damage_segment.default_effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let damage_tag = damage_tagged.tag.clone();
    let Some(damage) = damage_tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return;
    };
    let mut expected_creature = ObjectFilter::creature();
    expected_creature.set_explicit_card_type_noun(Some(crate::CardType::Creature));
    if !matches!(
        damage.target.unhinted(),
        ChooseSpec::Target(inner)
            if matches!(inner.unhinted(), ChooseSpec::Object(filter) if filter == &expected_creature)
    ) {
        return;
    }

    let Some(attachment_tagged) =
        attachment_segment.default_effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let attachment_tag = attachment_tagged.tag.clone();
    let Some(exile) = attachment_tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
    else {
        return;
    };
    let ChooseSpec::WithCount(attachment_target, count) = &exile.spec else {
        return;
    };
    let ChooseSpec::Target(attachment_target) = attachment_target.as_ref() else {
        return;
    };
    let ChooseSpec::Object(attachment_filter) = attachment_target.unhinted() else {
        return;
    };
    let expected_attachment = ObjectFilter::default()
        .with_subtype(crate::Subtype::Equipment)
        .in_zone(Zone::Battlefield)
        .match_tagged(
            damage_tag.clone(),
            crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject,
        );
    if exile.face_down
        || attachment_filter != &expected_attachment
        || count.min != 0
        || count.max != Some(1)
        || count.dynamic_x
        || count.up_to_x
        || count.random
        || count.explicit_exactly
    {
        return;
    }

    let Some(replacement) = replacement_segment.default_effects[0]
        .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()
    else {
        return;
    };
    if replacement.target != ChooseSpec::Tagged(attachment_tag)
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.mode != crate::effects::ReplacementApplyMode::UntilEndOfTurn
        || replacement.optional
        || replacement.library_placement.is_some()
        || !replacement.counters.is_empty()
        || replacement.linked_exile_follow_up.is_some()
    {
        return;
    }

    let mut replacement = replacement.clone();
    replacement.target = ChooseSpec::Tagged(damage_tag);
    replacement_segment.default_effects[0] = crate::effect::Effect::new(replacement);
}

/// Preserve the singular pronoun in an attack-triggered attachment count as
/// executable attachment provenance. Broad union parsing can retain the Aura
/// and Equipment arms while dropping the trailing `attached to it` relation,
/// which would count every matching permanent on the battlefield.
fn bind_triggered_attachment_union_count(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(11).any(|window| {
        window
            == [
                "draw",
                "a",
                "card",
                "for",
                "each",
                "aura",
                "and",
                "equipment",
                "attached",
                "to",
                "it",
            ]
    }) || !match &triggered.trigger.kind {
        crate::triggers::TriggerKind::ThisAttacks => true,
        crate::triggers::TriggerKind::Attacks { filter } => filter.source,
        _ => false,
    } {
        return;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() || segment.default_effects.len() != 1 {
        return;
    }
    let Some(draw) = segment.default_effects[0]
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .cloned()
    else {
        return;
    };
    if draw.player != PlayerFilter::You {
        return;
    }
    let crate::effect::Value::Count(filter) = draw.count.unhinted() else {
        return;
    };
    if filter.zone != Some(Zone::Battlefield)
        || !filter.tagged_constraints.is_empty()
        || filter.any_of.len() != 2
        || !filter.any_of.iter().any(|branch| {
            branch.subtypes == [crate::Subtype::Aura] && branch.tagged_constraints.is_empty()
        })
        || !filter.any_of.iter().any(|branch| {
            branch.subtypes == [crate::Subtype::Equipment] && branch.tagged_constraints.is_empty()
        })
    {
        return;
    }
    let mut linked = filter.clone();
    for branch in &mut linked.any_of {
        branch
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::TagKey::from("__it__"),
                relation: crate::target::TaggedOpbjectRelation::AttachedToTaggedObject,
            });
    }
    let hints = draw.count.surface_hints().to_vec();
    let mut rebound = draw;
    rebound.count = crate::effect::Value::Count(linked).with_surface_hints(hints);
    segment.default_effects[0] = crate::effect::Effect::new(rebound);
}

fn rewrite_optional_damage_target_preserving_wrappers(
    effect: &crate::effect::Effect,
) -> Option<crate::effect::Effect> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let mut rebound = with_id.clone();
        rebound.effect = Box::new(rewrite_optional_damage_target_preserving_wrappers(
            &with_id.effect,
        )?);
        return Some(crate::effect::Effect::new(rebound));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let mut rebound = tagged.clone();
        rebound.effect = Box::new(rewrite_optional_damage_target_preserving_wrappers(
            &tagged.effect,
        )?);
        return Some(crate::effect::Effect::new(rebound));
    }
    let damage = effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let ChooseSpec::WithCount(target, count) = &damage.target else {
        return None;
    };
    if count != &crate::effect::ChoiceCount::exactly(1) {
        return None;
    }
    let ChooseSpec::Target(target) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        return None;
    };
    let mut semantic_filter = filter.clone();
    semantic_filter.union_surface = Default::default();
    let mut expected = ObjectFilter::default().in_zone(Zone::Battlefield);
    expected.card_types = vec![crate::CardType::Creature, crate::CardType::Planeswalker];
    if semantic_filter != expected {
        return None;
    }
    let mut rebound = damage.clone();
    rebound.target = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
        .with_count(crate::effect::ChoiceCount::up_to(1));
    Some(crate::effect::Effect::new(rebound))
}

/// A linked return/damage sentence carries two independent facts: the damage
/// amount comes from the returned card, and the recipient is optional. The
/// generic amount-first parser can preserve the former while normalizing the
/// latter to a mandatory single target. Restore only this exact authored and
/// typed two-segment program.
fn bind_optional_linked_mana_value_damage(
    program: &mut crate::resolution::ResolutionProgram,
    authored_line: &str,
) {
    if !authored_line
        .to_ascii_lowercase()
        .contains("up to one target creature or planeswalker")
    {
        return;
    }
    let [_, damage_segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !damage_segment.self_replacements.is_empty() || damage_segment.default_effects.len() != 1 {
        return;
    }
    let Some(rewritten) =
        rewrite_optional_damage_target_preserving_wrappers(&damage_segment.default_effects[0])
    else {
        return;
    };
    damage_segment.default_effects[0] = rewritten;
}

/// Restore the destination-qualified attacking-creature set after an
/// otherwise complete optional-payment trigger has fallen through to the
/// generic source-pump action. The authored destination phrase, result id,
/// exact +2/+0 modification, and one-turn duration jointly prove the rewrite.
fn bind_attacking_opponents_result_pump(
    triggered: &mut crate::ability::TriggeredAbility,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(3)
        .any(|window| window == ["creatures", "attacking", "your"])
        || !words.iter().any(|word| *word == "opponents")
        || !words
            .windows(3)
            .any(|window| window == ["planeswalkers", "they", "control"])
    {
        return;
    }
    let [segment] = triggered.effects.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [producer_root, consumer_root] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(producer) = producer_root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .cloned()
    else {
        return;
    };
    let Some(may) = producer
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return;
    };
    if !matches!(
        may.effects.as_slice(),
        [effect] if effect.downcast_ref::<crate::effects::PayManaEffect>().is_some()
    ) {
        return;
    }
    let Some(mut consumer) = consumer_root
        .downcast_ref::<crate::effects::IfEffect>()
        .cloned()
    else {
        return;
    };
    if consumer.condition != producer.id
        || consumer.predicate != crate::effect::EffectPredicate::Happened
        || !consumer.else_.is_empty()
    {
        return;
    }
    let [pump_root] = consumer.then.as_mut_slice() else {
        return;
    };
    let Some(mut pump) = pump_root
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .cloned()
    else {
        return;
    };
    if pump.target != crate::continuous::EffectTarget::Source
        || pump.target_spec.as_ref() != Some(&ChooseSpec::Source)
        || pump.until != crate::effect::Until::EndOfTurn
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !matches!(
            pump.runtime_modifications.as_slice(),
            [
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: crate::effect::Value::Fixed(2),
                    toughness: crate::effect::Value::Fixed(0),
                }
            ]
        )
    {
        return;
    }

    let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    filter.attacking = true;
    filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Opponent);
    filter.attacking_player_only = false;
    let target = ChooseSpec::Object(filter);
    pump.target = target.clone().into();
    pump.target_spec = Some(target);
    pump.lock_filter_at_resolution = true;
    *pump_root = crate::effect::Effect::new(pump);
    *consumer_root = crate::effect::Effect::new(consumer);
}

/// Preserve an immediately preceding return-to-hand result as a typed
/// condition instead of a generic current-characteristic check. This exact
/// three-segment shell is produced when a triggered return is followed by an
/// optional payment and an `if you do` action.
fn bind_returned_card_to_hand_result_condition(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words.windows(13).any(|window| {
        window
            == [
                "if", "that", "card", "is", "returned", "to", "its", "owner", "s", "hand", "this",
                "way", "you",
            ]
    }) {
        return;
    }
    let [return_segment, condition_segment, result_segment] = program.segments.as_mut_slice()
    else {
        return;
    };
    if [&*return_segment, &*condition_segment, &*result_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.default_effects.len() != 1)
    {
        return;
    }
    let [return_root] = return_segment.default_effects.as_slice() else {
        return;
    };
    if return_root
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_none()
        && return_root
            .downcast_ref::<crate::effects::TaggedEffect>()
            .and_then(|tagged| {
                tagged
                    .effect
                    .downcast_ref::<crate::effects::ReturnToHandEffect>()
            })
            .is_none()
    {
        return;
    }
    let Some(with_id) = condition_segment.default_effects[0]
        .downcast_ref::<crate::effects::WithIdEffect>()
        .cloned()
    else {
        return;
    };
    let Some(mut conditional) = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .cloned()
    else {
        return;
    };
    let crate::effect::Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        return;
    };
    if tag.as_str() != "triggering" || filter != &ObjectFilter::default() {
        return;
    }
    let Some(result) = result_segment.default_effects[0].downcast_ref::<crate::effects::IfEffect>()
    else {
        return;
    };
    if result.condition != with_id.id
        || result.predicate != crate::effect::EffectPredicate::Happened
    {
        return;
    }
    let mut returned = ObjectFilter::default().in_zone(Zone::Hand);
    returned.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Returned));
    returned.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Card,
    ));
    conditional.condition = crate::effect::Condition::TaggedObjectMatches(tag.clone(), returned);
    let mut rebound = with_id;
    rebound.effect = Box::new(crate::effect::Effect::new(conditional));
    condition_segment.default_effects[0] = crate::effect::Effect::new(rebound);
}

/// Keep every action following an authored `Then if ...` inside that
/// conditional. A sentence-leading wrapper can otherwise leave a coordinated
/// transform as an unconditional sibling of the guarded untap.
fn bind_then_if_source_untap_and_transform(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(3)
        .any(|window| window == ["then", "if", "your"])
        || !words.iter().any(|word| *word == "untap")
        || !words.iter().any(|word| *word == "transform")
    {
        return;
    }
    let Some(segment) = program.segments.last_mut() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [sequence_root] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(mut sequence) = sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .cloned()
    else {
        return;
    };
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen {
        return;
    }
    let [conditional_root, transform_root] = sequence.effects.as_mut_slice() else {
        return;
    };
    let Some(mut conditional) = conditional_root
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .cloned()
    else {
        return;
    };
    let Some(transform) = transform_root.downcast_ref::<crate::effects::TransformEffect>() else {
        return;
    };
    let [untap_root] = conditional.if_true.as_slice() else {
        return;
    };
    let Some(untap) = untap_root.downcast_ref::<crate::effects::UntapEffect>() else {
        return;
    };
    if !conditional.if_false.is_empty()
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || untap.target.base() != &ChooseSpec::Source
        || transform.target != ChooseSpec::Source
    {
        return;
    }

    conditional.if_true.push(transform_root.clone());
    sequence.effects = vec![crate::effect::Effect::new(conditional)];
    *sequence_root = crate::effect::Effect::new(sequence);
}

/// Keep an optional cast tied to the opponent explicitly chosen earlier in
/// the same resolution. The broad lexical `Opponent` filter is not sufficient
/// in multiplayer games: both the decision and the cast must use the durable
/// chosen-player tag created by the preceding choice.
fn bind_optional_cast_to_previously_chosen_opponent(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(4)
        .any(|window| window == ["that", "opponent", "may", "cast"])
    {
        return;
    }
    let [segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [_, choose_player_root, choose_objects_root, _, _, may_root] =
        segment.default_effects.as_mut_slice()
    else {
        return;
    };
    let Some(choose_player) = choose_player_root
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()
        .cloned()
    else {
        return;
    };
    let Some(choose_objects) =
        choose_objects_root.downcast_ref::<crate::effects::ChooseObjectsEffect>()
    else {
        return;
    };
    let Some(mut may) = may_root
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        .cloned()
    else {
        return;
    };
    let [cast_root] = may.effects.as_mut_slice() else {
        return;
    };
    let Some(mut cast) = cast_root
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .cloned()
    else {
        return;
    };
    let chosen_player = PlayerFilter::TaggedPlayer(choose_player.tag.clone());
    if choose_player.chooser != PlayerFilter::You
        || choose_player.filter != PlayerFilter::Opponent
        || choose_player.random
        || !choose_player.excluded_tags.is_empty()
        || choose_objects.chooser != chosen_player
        || cast.tag != choose_objects.tag
        || cast.player != PlayerFilter::Opponent
        || may.decider != Some(PlayerFilter::Opponent)
    {
        return;
    }

    cast.player = chosen_player.clone();
    *cast_root = crate::effect::Effect::new(cast);
    may.decider = Some(chosen_player);
    *may_root = crate::effect::Effect::new(may);
}

/// Rebind a pump and its delayed sacrifice to the creature declared by the
/// first effect in the same instruction. This repairs the generic `it`
/// fallback without introducing another target or watching the spell source.
fn bind_until_end_of_turn_pump_and_delayed_sacrifice_to_declared_creature(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let authored_shape = words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
        && words
            .windows(3)
            .any(|window| window == ["gains", "trample", "and"])
        && words
            .windows(5)
            .any(|window| window == ["where", "x", "is", "its", "power"])
        && words
            .windows(7)
            .any(|window| window == ["sacrifice", "it", "at", "the", "beginning", "of", "the"]);
    if !authored_shape {
        return;
    }
    let [segment] = program.segments.as_mut_slice() else {
        return;
    };
    if !segment.self_replacements.is_empty() {
        return;
    }
    let [grant_root, pump_root, schedule_root] = segment.default_effects.as_mut_slice() else {
        return;
    };
    let Some(grant) = grant_root.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let grant_tag = grant.tag.clone();
    let Some(grant_continuous) = grant
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return;
    };
    if grant_continuous.until != crate::effect::Until::EndOfTurn
        || !grant_continuous
            .target_spec
            .as_ref()
            .is_some_and(ChooseSpec::is_target)
    {
        return;
    }
    let Some(pump_tagged) = pump_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .cloned()
    else {
        return;
    };
    let Some(mut pump) = pump_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .cloned()
    else {
        return;
    };
    if pump.target_spec.as_ref() != Some(&ChooseSpec::Source)
        || pump.until != crate::effect::Until::EndOfTurn
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !pump.require_creature_target
        || !matches!(
            pump.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness: crate::effect::Value::Fixed(0),
            }] if matches!(power.unhinted(), crate::effect::Value::SourcePower)
        )
    {
        return;
    }
    let Some(mut schedule) = schedule_root
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        .cloned()
    else {
        return;
    };
    let [sacrifice_root] = schedule.effects.as_mut_slice() else {
        return;
    };
    let Some(sacrifice) = sacrifice_root.downcast_ref::<crate::effects::SacrificeTargetEffect>()
    else {
        return;
    };
    if sacrifice.target != ChooseSpec::Source
        || !schedule.one_shot
        || schedule.start_next_turn
        || schedule.target_tag.is_some()
        || !schedule.target_choices.is_empty()
    {
        return;
    }

    let tagged_target = ChooseSpec::tagged(grant_tag.clone());
    pump.target_spec = Some(tagged_target.clone());
    let mut rebound_pump = pump_tagged;
    rebound_pump.effect = Box::new(crate::effect::Effect::new(pump));
    *pump_root = crate::effect::Effect::new(rebound_pump);
    *sacrifice_root =
        crate::effect::Effect::new(crate::effects::SacrificeTargetEffect::new(tagged_target));
    schedule.target_tag = Some(grant_tag);
    *schedule_root = crate::effect::Effect::new(schedule);
}

fn preserve_latest_self_replacement_presentation(
    builder: &mut CardDefinitionBuilder,
    statement_facts: &crate::runtime_backend::shared_types::StatementLineSemanticFacts,
) {
    if let Some(program) = builder.spell_effect.as_mut() {
        // Complete typed SelfReplacement programs bypass the later
        // follow-up attachment branches. Normalize the same exact
        // two-recipient damage relationship at this common boundary too.
        retarget_coordinated_damage_self_replacements(program);
        normalize_each_damaged_target_self_replacement(program);
    }
    let Some(branch) = builder.spell_effect.as_mut().and_then(|program| {
        program
            .segments
            .iter_mut()
            .rev()
            .find_map(|segment| segment.self_replacements.last_mut())
    }) else {
        return;
    };

    if let Some(presentation_label) = statement_facts.presentation_label.as_ref() {
        if branch.presentation_label.is_none() {
            branch.presentation_label = Some(presentation_label.clone());
        }
        branch.starts_new_source_line = true;
        branch.condition_after_replacement = statement_facts.leading_condition_intro.is_none();
    }
    if statement_facts.trailing_instead_if_predicate.is_some() {
        branch.condition_after_replacement = true;
    }
    if statement_facts.instead_followup.leading_instead_surface {
        branch.leading_instead_surface = true;
    }
}

fn bind_as_enters_counter_grants_to_source(effects: &mut [EffectAst]) {
    fn bind(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::GrantAbilitiesToTarget {
                target, abilities, ..
            } = &mut subject_verb.action
            && abilities.iter().any(|ability| {
                matches!(
                    ability,
                    crate::cards::builders::GrantedAbilityAst::StaticAbility(static_ability)
                        if static_ability.id()
                            == crate::static_abilities::StaticAbilityId::EnterWithCounters
                ) || matches!(
                    ability,
                    crate::cards::builders::GrantedAbilityAst::ParsedObjectAbility {
                        ability: parsed,
                        ..
                    } if matches!(
                        parsed.kind(),
                        AbilityKind::Static(static_ability)
                            if static_ability.id()
                                == crate::static_abilities::StaticAbilityId::EnterWithCounters
                    )
                )
            })
        {
            *target = crate::TargetAst::Source(None);
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind(nested_effect);
            }
        });
    }

    for effect in effects {
        bind(effect);
    }
}

/// Recover an authored relation in a named-source compound damage sentence at
/// the last public boundary that still owns both the normalized source noun
/// and the original recipient phrase. Earlier name/reference normalization
/// may simplify `that player or that planeswalker's controller controls` to a
/// broad creature filter. The compound parser remains the semantic guard: it
/// must accept the complete two-recipient sentence after only the source-name
/// prefix is taken from normalized tokens.
fn authored_compound_damage_fanout_effects(
    info: &LineInfo,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This repair owns one complete compound-damage sentence. Parsing a
    // multi-sentence physical line here would accept only that prefix and
    // discard a later result consumer such as "You gain life equal to the
    // damage dealt this way."
    if crate::runtime_backend::lexer::split_lexed_sentences(&info.source_tokens).len() != 1 {
        return Ok(None);
    }
    let normalized_tokens =
        crate::runtime_backend::lexer::lex_line(&info.normalized.normalized, info.line_index)?;
    let raw_verb = info
        .source_tokens
        .iter()
        .position(|token| token.is_any_word(&["deal", "deals"]));
    let normalized_verb = normalized_tokens
        .iter()
        .position(|token| token.is_any_word(&["deal", "deals"]));
    let (Some(raw_verb), Some(normalized_verb)) = (raw_verb, normalized_verb) else {
        return Ok(None);
    };

    let mut hybrid = normalized_tokens[..=normalized_verb].to_vec();
    hybrid.extend_from_slice(&info.source_tokens[raw_verb + 1..]);
    crate::runtime_backend::effect_sentences::parse_compound_damage_fanout_sentence(&hybrid)
}

fn preserve_authored_unenchanted_destroy_filter(
    effects: &mut [EffectAst],
    authored_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(authored_tokens);
    let says_unenchanted = words
        .windows(3)
        .any(|window| window == ["that", "isn't", "enchanted"])
        || words
            .windows(3)
            .any(|window| window == ["that", "isnt", "enchanted"])
        || words
            .windows(4)
            .any(|window| window == ["that", "is", "not", "enchanted"])
        || words
            .windows(3)
            .any(|window| window == ["that", "aren't", "enchanted"])
        || words
            .windows(3)
            .any(|window| window == ["that", "arent", "enchanted"])
        || words
            .windows(4)
            .any(|window| window == ["that", "are", "not", "enchanted"]);
    if !says_unenchanted
        || !words.iter().any(|word| *word == "destroy")
        || !words.iter().any(|word| *word == "regenerated")
    {
        return false;
    }

    fn qualifying_destroy(effect: &EffectAst) -> bool {
        fn target_has_object_filter(target: &crate::cards::builders::TargetAst) -> bool {
            match target {
                crate::cards::builders::TargetAst::Object(..) => true,
                crate::cards::builders::TargetAst::WithCount(inner, _) => {
                    target_has_object_filter(inner)
                }
                _ => false,
            }
        }
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Destroy {
                        no_regeneration: true,
                        target,
                        ..
                    },
                ..
            }) => target_has_object_filter(target),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll {
                        no_regeneration: true,
                        ..
                    },
                ..
            }) => true,
            _ => false,
        }
    }

    fn count_qualifying(effect: &EffectAst) -> usize {
        let mut count = usize::from(qualifying_destroy(effect));
        for_each_nested_effects(effect, true, |nested| {
            count += nested.iter().map(count_qualifying).sum::<usize>();
        });
        count
    }

    if effects.iter().map(count_qualifying).sum::<usize>() != 1 {
        return false;
    }

    fn mark(effect: &mut EffectAst, aura: &ObjectFilter) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Destroy {
                        no_regeneration: true,
                        target,
                        ..
                    },
                ..
            }) => {
                if let Some(filter) =
                    crate::runtime_backend::effect_sentences::target_object_filter_mut(target)
                    && filter.without_attached_object.is_none()
                {
                    filter.without_attached_object = Some(Box::new(aura.clone()));
                    return true;
                }
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll {
                        filter,
                        no_regeneration: true,
                        ..
                    },
                ..
            }) if filter.without_attached_object.is_none() => {
                filter.without_attached_object = Some(Box::new(aura.clone()));
                return true;
            }
            _ => {}
        }
        let mut changed = false;
        for_each_nested_effects_mut(effect, true, |nested| {
            for child in nested {
                changed |= mark(child, aura);
            }
        });
        changed
    }

    let mut aura = ObjectFilter::enchantment();
    aura.subtypes.push(crate::types::Subtype::Aura);
    effects.iter_mut().any(|effect| mark(effect, &aura))
}

fn lower_statement_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Statement {
        mut effects_ast,
        mut prepared,
    } = parsed
    else {
        unreachable!("statement lowerer received mismatched chunk");
    };

    let normalized_raw = info.raw_line.trim().to_ascii_lowercase();
    let commander_tax_life_prefix = "rather than pay {2} for each previous time you've cast this spell from the command zone this game, pay ";
    if let Some(life_text) = normalized_raw
        .strip_prefix(commander_tax_life_prefix)
        .and_then(|rest| rest.strip_suffix(" life that many times."))
        .or_else(|| {
            normalized_raw
                .strip_prefix(commander_tax_life_prefix)
                .and_then(|rest| rest.strip_suffix(" life that many times"))
        })
        && let Ok(life_per_previous_cast) = life_text.parse::<u32>()
        && life_per_previous_cast > 0
    {
        return Ok(builder.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::commander_tax_life_substitution(
                    life_per_previous_cast,
                ),
            )
            .in_zones(vec![crate::zone::Zone::Command]),
        ));
    }

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty effect statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty effect statement: '{}'",
            info.raw_line
        )));
    }
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    let authored_sentences = crate::runtime_backend::lexer::split_lexed_sentences(&authored_tokens);
    if let [choose, reciprocal_damage] = authored_sentences.as_slice()
        && crate::runtime_backend::lexer::parser_token_word_refs(choose).as_slice()
            == [
                "choose", "target", "creature", "you", "control", "and", "target", "creature",
                "an", "opponent", "controls",
            ]
        && crate::runtime_backend::lexer::parser_token_word_refs(reciprocal_damage).as_slice()
            == [
                "each",
                "of",
                "those",
                "creatures",
                "deals",
                "damage",
                "equal",
                "to",
                "its",
                "toughness",
                "to",
                "the",
                "other",
            ]
    {
        let Some(mut authored_reciprocal) =
            crate::runtime_backend::effect_sentences::parse_choose_target_prelude_sentence(choose)?
        else {
            return Err(CardTextError::InvariantViolation(
                "authored reciprocal target declaration stopped parsing".to_string(),
            ));
        };
        authored_reciprocal.extend(
            crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(
                reciprocal_damage,
            )?,
        );
        effects_ast = authored_reciprocal;
        prepared =
            super::rewrite_prepare_effects_for_lowering(&effects_ast, prepared.imports.clone())?;
    }
    if authored_sentences.len() == 4 {
        let sentence_inputs = authored_sentences
            .iter()
            .map(|tokens| {
                crate::runtime_backend::effect_sentences::SentenceInput::from_lexed(tokens)
            })
            .collect::<Vec<_>>();
        if let Some(authored_tempting_offer) =
            crate::runtime_backend::effect_sentences::parse_tempting_offer_copy_spell_sequence(
                &sentence_inputs,
                0,
            )?
        {
            effects_ast = authored_tempting_offer;
            prepared = super::rewrite_prepare_effects_for_lowering(
                &effects_ast,
                prepared.imports.clone(),
            )?;
        }
    }
    if authored_sentences.len() == 3
        && source_words_contain(&authored_tokens, &["choose", "target", "creature"])
        && source_words_contain(
            &authored_tokens,
            &["instead", "choose", "target", "creature"],
        )
        && source_words_contain(&authored_tokens, &["exile", "the", "chosen", "creature"])
        && source_words_contain(
            &authored_tokens,
            &["controller", "gains", "life", "equal", "to"],
        )
        && source_words_contain(&authored_tokens, &["mana", "value"])
        && let Ok(authored_replacement) =
            crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(&authored_tokens)
        && matches!(
            authored_replacement.as_slice(),
            [EffectAst::SelfReplacement {
                if_true,
                if_false,
                ..
            }] if !if_true.is_empty()
                && !if_false.is_empty()
                && format!("{authored_replacement:#?}").contains("GainLife")
        )
    {
        effects_ast = authored_replacement;
        prepared =
            super::rewrite_prepare_effects_for_lowering(&effects_ast, prepared.imports.clone())?;
    }
    let authored_words = crate::runtime_backend::lexer::parser_token_word_refs(&authored_tokens);
    if authored_words.starts_with(&["destroy", "target", "nonland", "permanent"])
        && (authored_words
            .windows(4)
            .any(|window| matches!(window, ["if", "it" | "its", "a", "creature"]))
            || authored_words
                .windows(5)
                .any(|window| window == ["if", "it", "s", "a", "creature"]))
        && authored_words
            .windows(5)
            .any(|window| window == ["spent", "to", "cast", "this", "spell"])
    {
        let mut authored_destroy =
            crate::runtime_backend::effect_sentences::parse_destroy(&authored_tokens[1..])?;
        if let EffectAst::Conditional {
            predicate: PredicateAst::Or(left, _),
            ..
        } = &mut authored_destroy
            && let PredicateAst::ItMatches(filter) = left.as_ref()
        {
            **left = PredicateAst::TargetMatches(filter.clone());
        }
        if matches!(
            &authored_destroy,
            EffectAst::Conditional {
                predicate: PredicateAst::Or(_, _),
                if_true,
                if_false,
            } if if_true.len() == 1 && if_false.is_empty()
        ) {
            effects_ast = vec![authored_destroy];
            prepared = super::rewrite_prepare_effects_for_lowering(
                &effects_ast,
                prepared.imports.clone(),
            )?;
        }
    }
    if let Some(authored_damage) = authored_compound_damage_fanout_effects(info)? {
        effects_ast = authored_damage;
        prepared =
            super::rewrite_prepare_effects_for_lowering(&effects_ast, prepared.imports.clone())?;
    }
    // Document/semantic reconstruction can consume a quoted token rule while
    // preserving the executable create-token-copy AST. Reattach the quoted
    // rule from the retained authored tokens at the final public lowering
    // boundary, then rebuild reference preparation only when the typed copy
    // was actually enriched. This also reaches replacement branches.
    if crate::runtime_backend::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
        &mut prepared.effects,
        &authored_tokens,
    ) {
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    }
    if preserve_authored_unenchanted_destroy_filter(&mut prepared.effects, &authored_tokens) {
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    }
    // Some conditional sentence routes consume the coordinated prevention
    // rider while parsing the damage clause. The original line tokens remain
    // authoritative here, so carry that rider onto the typed damage leaf
    // before lowering it into a replacement branch.
    if crate::runtime_backend::effect_sentences::damage_clause_has_terminal_unpreventable_rider(
        &info.source_tokens,
    ) {
        let mut marked_replacement = false;
        for effect in &mut prepared.effects {
            if let EffectAst::SelfReplacement { if_true, .. } = effect {
                for replacement_effect in if_true {
                    crate::runtime_backend::effect_sentences::mark_damage_ast_unpreventable(
                        replacement_effect,
                    );
                }
                marked_replacement = true;
            }
        }
        if !marked_replacement {
            for effect in &mut prepared.effects {
                crate::runtime_backend::effect_sentences::mark_damage_ast_unpreventable(effect);
            }
        }
    }
    if let Some(enchant_filter) = effects_ast.iter().find_map(|effect| {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return None;
        };
        match &subject_verb.action {
            SubjectVerbActionAst::Enchant { filter } => Some(filter.clone()),
            _ => None,
        }
    }) {
        builder.aura_attach_filter = Some(enchant_filter);
    }
    let prior_token_template = token_template_before_prior_token_placeholder(&prepared.effects);
    if let Some(token_template) = prior_token_template {
        rewrite_prior_token_placeholders_from_template(&mut prepared.effects, &token_template);
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    } else if let Some(token_info) = state.latest_created_token.clone() {
        rewrite_prior_token_placeholders(&mut prepared.effects, &token_info);
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    }

    if semantic_facts
        .statement
        .as_enters_effect_program
        .as_ref()
        .is_some_and(|facts| facts.uses_enters_with_counter_surface)
    {
        let imports = prepared.imports.clone();
        bind_as_enters_counter_grants_to_source(&mut prepared.effects);
        prepared = super::rewrite_prepare_effects_for_lowering(&prepared.effects, imports)?;
    }

    if semantic_facts.statement.as_enters_effect_program.is_some() && prepared.effects.len() == 2 {
        let imports = prepared.imports.clone();
        let effects = std::mem::take(&mut prepared.effects);
        prepared.effects = super::rewrite_normalize_selected_sacrifice_tags(effects);
        prepared = super::rewrite_prepare_effects_for_lowering(&prepared.effects, imports)?;
    }

    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    super::rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "spell text effects",
    )?;
    if let Some(token_info) =
        last_created_token_info(&effects_ast).or_else(|| last_created_token_info(&prepared.effects))
    {
        state.latest_created_token = Some(token_info);
    }
    let mut compiled = lowered.effects;
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    materialize_attached_library_search_count_overrides(&mut compiled);
    bind_linked_damage_attachment_death_replacement(&mut compiled, &authored_tokens);
    bind_optional_cast_to_previously_chosen_opponent(&mut compiled, &authored_tokens);
    bind_until_end_of_turn_pump_and_delayed_sacrifice_to_declared_creature(
        &mut compiled,
        &authored_tokens,
    );
    bind_excess_damage_exile_top_permission(&mut compiled, &authored_tokens);
    state.latest_spell_exports = lowered.exports;
    if builder
        .spell_effect
        .as_ref()
        .is_some_and(|program| !program.is_empty())
        && state
            .latest_statement_line_index
            .is_some_and(|previous| previous != info.line_index)
        && let Some(first_segment) = compiled.segments.first_mut()
    {
        first_segment.starts_new_source_line = true;
    }
    state.latest_statement_line_index = Some(info.line_index);

    // A front-end bundle that already owns both sides of a self-replacement is
    // a complete semantic program. Do not reinterpret a conditional in its
    // default arm as another follow-up to the program it just built. Doing so
    // replaces the already-materialized branch and loses the authored
    // paid-cost alternative when both arms also have their own thresholds.
    if matches!(
        effects_ast.as_slice(),
        [EffectAst::SelfReplacement {
            attach_to_previous_ability: false,
            ..
        }]
    ) {
        if let Some(existing) = builder.spell_effect.as_mut() {
            existing.extend(compiled);
        } else {
            builder.spell_effect = Some(compiled);
        }
        if let Some(program) = builder.spell_effect.as_mut() {
            dedupe_lowered_adjacent_target_declarations(program);
            bind_source_exiled_return_complement(program);
            normalize_graveyard_card_copy_cast_program(program);
            preserve_separate_copy_instruction_surface(program, &authored_tokens);
            super::super::battlefield_entry_counter_fusion::fuse_program(program);
        }
        preserve_latest_self_replacement_presentation(&mut builder, &semantic_facts.statement);
        return Ok(builder);
    }

    let statement_facts = &semantic_facts.statement;
    bind_negated_control_condition_after_tagged_zone_move(&mut compiled, &info.source_tokens);
    bind_target_characteristic_or_paid_mana_condition(&mut compiled, &info.source_tokens);
    bind_delayed_return_to_watched_object_owner(&mut compiled, &authored_tokens);
    normalize_each_creature_except_controlled_flying_damage(&mut compiled, &info.source_tokens);
    fold_prior_result_self_replacement_into_success_arm(&mut compiled, statement_facts);
    fuse_repeatable_mana_payment_prevention_until_end_of_turn(&mut compiled, &info.source_tokens);
    if let Some(as_enters) = statement_facts.as_enters_effect_program.as_ref() {
        // A single player selected by an as-enters program is persistent
        // card state. Later static abilities and triggers resolve
        // `PlayerFilter::ChosenPlayer` through the entering source.
        remember_single_player_choice_as_enters(&mut compiled);
        // The reveal-opponents-hands + choose-a-revealed-nonland-name bundle
        // has a dedicated typed static with the complete ETB semantics
        // (Alhammarret, High Arbiter); prefer it over the generic program.
        if compiled.len() == 2
            && compiled[0]
                .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
                .is_some_and(|for_players| {
                    for_players.filter == PlayerFilter::Opponent
                        && matches!(
                            for_players.effects.as_slice(),
                            [reveal] if reveal
                                .downcast_ref::<crate::effects::LookAtHandEffect>()
                                .is_some_and(|look| look.reveal)
                        )
                })
            && compiled[1]
                .downcast_ref::<crate::effects::ChooseCardNameEffect>()
                .is_some_and(|choose| {
                    choose.filter.as_ref().is_some_and(|filter| {
                        filter.excluded_card_types == vec![crate::types::CardType::Land]
                            && filter.prior_effect_action_surface()
                                == Some(ironsmith_core::PriorEffectAction::Revealed)
                    })
                })
        {
            let static_ability =
                crate::static_abilities::StaticAbility::choose_revealed_hand_nonland_card_name_as_enters(
                    format!(
                        "As {} enters, each opponent reveals their hand. You choose the name of a nonland card revealed this way.",
                        as_enters.subject
                    ),
                );
            builder = builder.with_ability(crate::ability::Ability::static_ability(static_ability));
            return Ok(builder);
        }
        let static_ability = if as_enters.turns_face_up_only {
            crate::static_abilities::StaticAbility::as_turns_face_up_effect_program(
                compiled,
                as_enters.subject.clone(),
                statement_facts.presentation_label.clone(),
            )
        } else {
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                compiled,
                as_enters.subject.clone(),
                as_enters.also_turns_face_up,
                as_enters.uses_enters_with_counter_surface,
                statement_facts.presentation_label.clone(),
            )
        };
        builder = builder.with_ability(crate::ability::Ability::static_ability(static_ability));
        fuse_pending_removed_counter_as_enters(&mut builder);
        return Ok(builder);
    }
    if let Some(as_transforms) = statement_facts.as_transforms_effect_program.as_ref() {
        // A choice made as the permanent transforms is persistent card state
        // in the same way as an as-enters choice. Later abilities resolve
        // `PlayerFilter::ChosenPlayer` through the transformed source.
        remember_single_player_choice_as_enters(&mut compiled);
        let static_ability = crate::static_abilities::StaticAbility::as_transforms_effect_program(
            compiled,
            as_transforms.subject.clone(),
            as_transforms.destination.clone(),
            statement_facts.presentation_label.clone(),
        );
        builder = builder.with_ability(crate::ability::Ability::static_ability(static_ability));
        return Ok(builder);
    }
    let instead_semantics = statement_facts.instead_followup.semantics;
    let trailing_instead_if_condition = if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) {
        compile_trailing_instead_if_condition(
            statement_facts.trailing_instead_if_predicate.as_ref(),
            &prepared,
        )?
    } else {
        None
    };
    if let Some(program) = back_for_seconds_style_replacement_program(&compiled, statement_facts) {
        builder.spell_effect = Some(program);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if attach_back_for_seconds_style_replacement(&mut builder, &compiled, statement_facts) {
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if let Some(program) =
        kicked_count_override_self_replacement_program(&compiled, statement_facts)
    {
        builder.spell_effect = Some(program);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if let Some(program) = kicked_multi_zone_search_to_battlefield_program(&compiled) {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if attach_kicked_multi_zone_search_to_battlefield_replacement(
        &mut builder,
        &compiled,
        statement_facts,
    ) {
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if let Some(program) = clash_win_optional_top_replacement_program(&compiled, statement_facts) {
        builder.spell_effect = Some(program);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if attach_morbid_search_to_battlefield_self_replacement(&mut builder, statement_facts) {
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    }
    if let Some(program) = creature_type_choice_program(statement_facts, &compiled) {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && builder.spell_effect.is_none()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let previous = compiled[0].clone();
        let mut replacement = replacement;
        if let Some(previous_target) = super::extract_previous_replacement_target(&previous) {
            replacement.if_true = retarget_replacement_effects_with_condition(
                replacement.if_true,
                &previous_target,
                &replacement.condition,
            );
        }
        let mut spell_effect = crate::resolution::ResolutionProgram::from_effects(vec![previous]);
        let Some(segment) = spell_effect.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for inline self-replacement"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
        builder.spell_effect = Some(spell_effect);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement.if_true = retarget_replacement_effects_with_condition(
                replacement.if_true,
                &previous_target,
                &replacement.condition,
            );
        }
        let replacement = crate::resolution::SelfReplacementBranch::new(
            replacement.condition,
            replacement.if_true,
        );
        if attach_materialized_library_search_count_override(existing, &replacement) {
            preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
            return Ok(builder);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for repeated self-replacement"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && builder.spell_effect.is_none()
        && let Some(condition) = trailing_instead_if_condition
    {
        let previous = compiled[0].clone();
        let mut replacement_effects = vec![compiled[1].clone()];
        if let Some(previous_target) = super::extract_previous_replacement_target(&previous) {
            replacement_effects = retarget_replacement_effects_with_condition(
                replacement_effects,
                &previous_target,
                &condition,
            );
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let mut spell_effect = crate::resolution::ResolutionProgram::from_effects(vec![previous]);
        let Some(segment) = spell_effect.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for inline plain instead-if follow-up"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                condition,
                replacement_effects,
            ));
        builder.spell_effect = Some(spell_effect);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(condition) = trailing_instead_if_condition
    {
        let mut replacement_effects = vec![compiled[1].clone()];
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement_effects = retarget_replacement_effects_with_condition(
                replacement_effects,
                &previous_target,
                &condition,
            );
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let replacement =
            crate::resolution::SelfReplacementBranch::new(condition, replacement_effects);
        if attach_materialized_library_search_count_override(existing, &replacement) {
            preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
            return Ok(builder);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for plain instead-if follow-up"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(condition) = trailing_instead_if_condition
    {
        let mut replacement_effects = vec![compiled[0].clone()];
        let coordinated_retarget = existing.last().and_then(|default_effect| {
            retarget_coordinated_damage_replacement_pair(
                default_effect,
                &replacement_effects,
                &condition,
            )
        });
        if let Some(rewritten) = coordinated_retarget {
            replacement_effects = rewritten;
        } else if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement_effects = retarget_replacement_effects_with_condition(
                replacement_effects,
                &previous_target,
                &condition,
            );
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let replacement =
            crate::resolution::SelfReplacementBranch::new(condition, replacement_effects);
        if attach_materialized_library_search_count_override(existing, &replacement) {
            preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
            return Ok(builder);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for single-effect instead-if follow-up"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && let Some(mut replacement) = materialized_self_replacement_followup(&compiled)
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
    {
        if attach_materialized_library_search_count_override(existing, &replacement) {
            preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
            return Ok(builder);
        }
        let coordinated_retarget = existing.last().and_then(|default_effect| {
            retarget_coordinated_damage_replacement_pair(
                default_effect,
                &replacement.replacement_effects,
                &replacement.condition,
            )
        });
        if let Some(rewritten) = coordinated_retarget {
            replacement.replacement_effects = rewritten;
        } else if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.replacement_effects = retarget_replacement_effects_with_condition(
                replacement.replacement_effects,
                &previous_target,
                &replacement.condition,
            );
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for materialized self-replacement"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && builder.spell_effect.is_none()
        && ((statement_facts.instead_followup.conditional_intro
            && matches!(
                statement_facts.leading_condition_intro,
                Some(StatementConditionIntro::If)
            ))
            || conditional_self_replacement_followup(&compiled[0])
                .is_some_and(|replacement| replacement.if_false.is_empty()))
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && builder.spell_effect.is_none()
        && materialized_self_replacement_followup(&compiled).is_some()
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    }
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[0])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.if_true = replacement
                .if_true
                .into_iter()
                .map(|effect| {
                    if let Some(replacement_damage) =
                        effect.downcast_ref::<crate::effects::DealDamageEffect>()
                        && replacement_damage.target
                            == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                    {
                        let mut replacement_damage = replacement_damage.clone();
                        replacement_damage.target = previous_target.clone();
                        crate::effect::Effect::new(replacement_damage)
                    } else {
                        super::rewrite_replacement_effect_target(&effect, &previous_target)
                            .unwrap_or(effect)
                    }
                })
                .collect();
        }
        let replacement = crate::resolution::SelfReplacementBranch::new(
            replacement.condition,
            replacement.if_true,
        );
        if attach_materialized_library_search_count_override(existing, &replacement) {
            preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
            return Ok(builder);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for self-replacement".to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(compiled);
    } else {
        builder.spell_effect = Some(compiled);
    }
    if let Some(program) = builder.spell_effect.as_mut() {
        // A sentence-leading negated-control clause is lowered after the
        // preceding zone move has already been appended to the builder.  Run
        // the exact typed reconciliation at this public program boundary as
        // well as on single-line compiled programs, so it can see both the
        // successful exile result tag and the authored independent condition.
        bind_negated_control_condition_after_tagged_zone_move(program, &info.source_tokens);
        dedupe_lowered_adjacent_target_declarations(program);
        bind_source_exiled_return_complement(program);
        normalize_graveyard_card_copy_cast_program(program);
        preserve_separate_copy_instruction_surface(program, &authored_tokens);
        bind_commander_hand_move_from_command_zone(program, &authored_tokens);
        // A delayed instruction may be reconstructed as a second statement
        // after its target declaration has already been appended to the
        // builder. Run the exact watched-object owner binder at this combined
        // public-program boundary as well as on a single compiled chunk.
        bind_delayed_return_to_watched_object_owner(program, &authored_tokens);
        bind_optional_linked_mana_value_damage(program, &info.raw_line);
        super::super::battlefield_entry_counter_fusion::fuse_program(program);
    }
    preserve_latest_self_replacement_presentation(&mut builder, statement_facts);
    Ok(builder)
}

/// Recover the exact damage-outcome dependency when document-level sentence
/// preparation has already lowered the dynamic exile sentence as a generic
/// top-card choice. This is deliberately guarded by both the authored excess
/// damage wording and the complete observed three-segment executable shell.
fn bind_excess_damage_exile_top_permission(
    program: &mut crate::resolution::ResolutionProgram,
    source_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    if !words
        .windows(4)
        .any(|window| window == ["excess", "damage", "dealt", "to"])
        || !words
            .windows(6)
            .any(|window| window == ["until", "the", "end", "of", "your", "next"])
    {
        return;
    }
    let [damage_segment, exile_segment, permission_segment] = program.segments.as_slice() else {
        return;
    };
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return;
    }
    let [damage_root] = damage_segment.default_effects.as_slice() else {
        return;
    };
    let damage_inner = damage_root
        .as_tagged()
        .map_or(damage_root, |tagged| &tagged.effect);
    if damage_inner
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_none()
    {
        return;
    }
    let [choose_root, exile_root] = exile_segment.default_effects.as_slice() else {
        return;
    };
    let Some(choose) = choose_root.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return;
    };
    let Some(exile) = exile_root.downcast_ref::<crate::effects::ExileEffect>() else {
        return;
    };
    if choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::You)
        || !choose.top_only
        || choose.count != crate::effect::ChoiceCount::exactly(1)
        || exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return;
    }
    let [grant_root] = permission_segment.default_effects.as_slice() else {
        return;
    };
    let Some(mut grant) = grant_root
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        .cloned()
    else {
        return;
    };
    if grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
        || !grant.allow_land
    {
        return;
    }

    let next_id = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .filter_map(max_effect_id)
        .max()
        .and_then(|maximum| maximum.checked_add(1))
        .unwrap_or(0);
    let effect_id = crate::effect::EffectId(next_id);
    let count = crate::effect::Value::EffectMetric {
        effect_id,
        source: ironsmith_core::EffectMetricSource::Outcome,
        metric: ironsmith_core::EffectMetric::ExcessDamage,
    }
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);
    let exiled_tag = choose.tag.clone();
    let exile = crate::effects::ExileTopOfLibraryEffect::new(count, PlayerFilter::You)
        .tag_moved(exiled_tag.clone());
    grant.tag = exiled_tag;
    grant.cast_pool_is_plural = true;
    grant.surface = Some(
        ironsmith_core::GrantPlayTaggedSurface::default()
            .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards),
    );
    let damage = crate::effect::Effect::with_id(next_id, damage_root.clone());
    *program = crate::resolution::ResolutionProgram::from_effects(vec![
        damage,
        crate::effect::Effect::new(exile),
        crate::effect::Effect::new(grant),
    ]);
}

fn lower_additional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCost {
        effects_ast,
        prepared,
    } = parsed
    else {
        unreachable!("additional-cost lowerer received mismatched chunk");
    };

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty additional cost statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty additional-cost statement: '{}'",
            info.raw_line
        )));
    }
    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    let compiled = super::runtime_effects_to_costs(lowered.effects.to_vec())?;
    state.latest_additional_cost_exports = lowered.exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.extend(compiled);
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_optional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder, parsed, ..
    } = input;
    let NormalizedLineChunk::OptionalCost(cost) = parsed else {
        unreachable!("optional-cost lowerer received mismatched chunk");
    };
    let kind = cost.kind.clone();
    let reference = cost.cost_ref();
    let mut builder = builder.optional_cost(cost);
    match kind {
        crate::cost::OptionalCostKind::Squad => {
            builder = builder.with_ability(Ability::triggered(
                crate::triggers::Trigger::this_enters_battlefield(),
                vec![crate::effect::Effect::new(
                    crate::effects::CreateTokenCopyEffect::new(
                        ChooseSpec::Source,
                        crate::effect::Value::TimesPaidLabel(reference),
                        PlayerFilter::You,
                    ),
                )],
            ));
        }
        crate::cost::OptionalCostKind::Offspring => {
            builder = builder.with_ability(Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: crate::triggers::Trigger::this_enters_battlefield(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        crate::effect::Effect::new(
                            crate::effects::CreateTokenCopyEffect::new(
                                ChooseSpec::Source,
                                crate::effect::Value::WasPaidLabel(reference.clone()),
                                PlayerFilter::You,
                            )
                            .set_base_power_toughness(1, 1),
                        ),
                    ]),
                    choices: vec![],
                    intervening_if: Some(crate::effect::Condition::ThisSpellPaidLabel(reference)),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        }
        _ => {}
    }
    Ok(builder)
}

fn lower_gift_keyword_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::GiftKeyword {
        cost,
        prepared,
        followup_text,
        timing,
    } = parsed
    else {
        unreachable!("gift-keyword lowerer received mismatched chunk");
    };

    builder = builder.optional_cost(cost);
    match timing {
        GiftTimingAst::SpellResolution => {
            let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut gift_effects = lowered.effects.to_vec();
            gift_effects.push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            let gift_effect = crate::effect::Effect::conditional(
                crate::ConditionExpr::ThisSpellPaidLabel("Gift".into()),
                gift_effects,
                Vec::new(),
            );
            if let Some(ref mut existing) = builder.spell_effect {
                existing.push(gift_effect);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        gift_effect,
                    ]));
            }
        }
        GiftTimingAst::PermanentEtb => {
            let parsed = super::rewrite_parsed_triggered_ability(
                TriggerSpec::ThisEntersBattlefield {
                    origin_condition: None,
                },
                prepared.effects.clone(),
                vec![Zone::Battlefield],
                Some(format!(
                    "When this permanent enters, if the gift was promised, {followup_text}"
                )),
                Some(crate::ConditionExpr::ThisSpellPaidLabel("Gift".into())),
                None,
                prepared.imports.clone(),
            );
            let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered {
                    trigger: TriggerSpec::ThisEntersBattlefield {
                        origin_condition: None,
                    },
                    prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                        prepared,
                        intervening_if: None,
                    },
                }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut parsed = parsed;
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed.into_runtime());
        }
    }
    Ok(builder)
}

fn lower_optional_cost_with_cast_trigger_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::OptionalCostWithCastTrigger {
        cost,
        prepared,
        followup_text,
    } = parsed
    else {
        unreachable!("optional-cost-cast-trigger lowerer received mismatched chunk");
    };

    let cost_ref = cost.cost_ref();
    builder = builder.optional_cost(cost);
    let parsed = super::rewrite_parsed_triggered_ability(
        TriggerSpec::YouCastThisSpell,
        prepared.effects.clone(),
        vec![Zone::Stack],
        Some(followup_text),
        Some(crate::ConditionExpr::ThisSpellPaidLabel(cost_ref)),
        None,
        prepared.imports.clone(),
    );
    let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered {
            trigger: TriggerSpec::YouCastThisSpell,
            prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            },
        }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    Ok(builder.with_ability(parsed.into_runtime()))
}

fn lower_additional_cost_choice_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCostChoice { options } = parsed else {
        unreachable!("additional-cost-choice lowerer received mismatched chunk");
    };

    if options.len() < 2 {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "additional cost choice requires at least two options".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to invalid additional-cost choice (line: '{}')",
            info.raw_line
        )));
    }
    for option in &options {
        if option.effects_ast.is_empty() {
            if allow_unsupported {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    "additional cost choice option produced no effects".to_string(),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "line parsed to empty additional-cost option (line: '{}')",
                info.raw_line
            )));
        }
    }
    let (modes, exports) =
        match super::rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options) {
            Ok(outputs) => outputs,
            Err(err) if allow_unsupported => {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    format!("{err:?}"),
                ));
            }
            Err(err) => return Err(err),
        };
    state.latest_additional_cost_exports = exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.push(
        crate::costs::payment_effect_to_cost(crate::effect::Effect::choose_one(modes))
            .map_err(CardTextError::ParseError)?,
    );
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_alternative_casting_method_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        ..
    } = input;
    let NormalizedLineChunk::AlternativeCastingMethod(mut method) = parsed else {
        unreachable!("alternative-casting-method lowerer received mismatched chunk");
    };
    if let crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost {
        additional_cost,
        ..
    } = &method
    {
        let printed_cost = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut pips = printed_cost.pips().to_vec();
        pips.extend(additional_cost.pips().iter().cloned());
        let total_mana_cost = crate::mana::ManaCost::from_pips(pips);
        method = crate::alternative_cast::AlternativeCastingMethod::flash_with_additional_cost(
            additional_cost.clone(),
            crate::cost::TotalCost::mana(total_mana_cost),
        );
    }
    if let crate::alternative_cast::AlternativeCastingMethod::Retrace { total_cost } = &method {
        let printed_cost = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut costs = vec![crate::costs::Cost::mana(printed_cost)];
        costs.extend(total_cost.costs().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::Retrace {
            total_cost: crate::cost::TotalCost::from_costs(costs),
        };
    }
    builder.alternative_casts.push(method);
    Ok(builder)
}

fn trigger_frequency_condition_from_facts(
    max_triggers_per_turn: Option<u32>,
    facts: &crate::runtime_backend::shared_types::TriggerFrequencyFacts,
) -> Option<crate::ConditionExpr> {
    max_triggers_per_turn.map(|limit| {
        if limit == 1 && facts.first_time_each_or_this_turn && facts.becomes_crewed {
            crate::ConditionExpr::SourceFirstCrewedThisTurn
        } else if limit == 1 && facts.first_time_each_or_this_turn {
            crate::ConditionExpr::FirstTimeThisTurn
        } else if facts.do_this_limit_each_turn.is_some() {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit)
        } else {
            crate::ConditionExpr::MaxTimesEachTurn(limit)
        }
    })
}

fn restore_authored_source_exiled_return(
    effects: &mut [EffectAst],
    authored_tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> Result<bool, CardTextError> {
    let Some(return_index) = authored_tokens
        .iter()
        .rposition(|token| token.is_word("return"))
    else {
        return Ok(false);
    };
    if !authored_tokens[..return_index]
        .iter()
        .any(|token| token.is_word("until"))
        || !authored_tokens[..return_index]
            .iter()
            .any(|token| token.is_word("exile"))
    {
        return Ok(false);
    }
    let Ok(parsed_return) = crate::runtime_backend::effect_sentences::parse_effect_clause_lexed(
        &authored_tokens[return_index..],
    ) else {
        return Ok(false);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            replacement @ SubjectVerbActionAst::MoveToZone {
                zone: Zone::Battlefield,
                exiled_with_source_surface: Some(_),
                battlefield_controller: crate::cards::builders::ReturnControllerAst::Owner,
                ..
            },
        ..
    }) = parsed_return
    else {
        return Ok(false);
    };

    fn is_exile_until(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileUntilSourceLeaves {
                    duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                    ..
                },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(is_exile_until);
        });
        found
    }
    if effects
        .iter()
        .filter(|effect| is_exile_until(effect))
        .count()
        != 1
    {
        return Ok(false);
    }

    fn count_source_linked_returns(effect: &EffectAst) -> usize {
        let mut count = usize::from(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    battlefield_controller: crate::cards::builders::ReturnControllerAst::Owner,
                    ..
                },
                ..
            })
        ));
        for_each_nested_effects(effect, true, |nested| {
            count += nested
                .iter()
                .map(count_source_linked_returns)
                .sum::<usize>();
        });
        count
    }
    if effects
        .iter()
        .map(count_source_linked_returns)
        .sum::<usize>()
        != 1
    {
        return Ok(false);
    }

    fn replace_return(effect: &mut EffectAst, replacement: &SubjectVerbActionAst) -> bool {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect
            && matches!(
                action,
                SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    battlefield_controller: crate::cards::builders::ReturnControllerAst::Owner,
                    ..
                }
            )
        {
            *action = replacement.clone();
            return true;
        }
        let mut changed = false;
        for_each_nested_effects_mut(effect, true, |nested| {
            for child in nested {
                changed |= replace_return(child, replacement);
            }
        });
        changed
    }

    Ok(effects
        .iter_mut()
        .any(|effect| replace_return(effect, &replacement)))
}

fn lower_triggered_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder,
        state,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Triggered {
        mut trigger,
        mut prepared,
        max_triggers_per_turn,
    } = parsed
    else {
        unreachable!("triggered lowerer received mismatched chunk");
    };

    // Prepared triggered chunks can still contain the earlier broad parse of
    // these two correlated authored programs. Rebuild the prepared payload
    // from the intact source before it is lowered, so the public Triggered
    // route retains the same typed collection/value provenance as the direct
    // semantic-line route.
    let authored_tokens = crate::runtime_backend::lexer::lex_line(&info.raw_line, info.line_index)
        .unwrap_or_else(|_| info.source_tokens.clone());
    let authored_tail =
        crate::runtime_backend::grammar::semantic_lowering::parse_comma_split_tokens(
            &authored_tokens,
        )
        .map(|split| split.after);
    let source_tail = crate::runtime_backend::grammar::semantic_lowering::parse_comma_split_tokens(
        &info.source_tokens,
    )
    .map(|split| split.after);
    let reconciled_dynamic =
        crate::runtime_backend::semantic_line_parsing::
            dynamic_zone_change_group_token_creation_from_authored_trigger(&authored_tokens)?
            .or(crate::runtime_backend::semantic_line_parsing::
                dynamic_zone_change_group_token_creation_from_authored_trigger(
                    &info.source_tokens,
                )?);
    let reconciled_looked_hand = authored_tail
        .as_ref()
        .and_then(|tail| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                tail,
            )
        })
        .or_else(|| {
            source_tail.as_ref().and_then(|tail| {
                crate::runtime_backend::semantic_line_parsing::
                    exact_looked_hand_optional_cast_bundle(tail)
            })
        })
        .or_else(|| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                &authored_tokens,
            )
        })
        .or_else(|| {
            crate::runtime_backend::semantic_line_parsing::exact_looked_hand_optional_cast_bundle(
                &info.source_tokens,
            )
        });
    let reconciled_graveyard_copy_cast = authored_tail
        .as_ref()
        .and_then(|tail| {
            crate::runtime_backend::semantic_line_parsing::exact_graveyard_card_copy_cast_sequence(
                tail,
            )
        })
        .or_else(|| {
            source_tail.as_ref().and_then(|tail| {
                crate::runtime_backend::semantic_line_parsing::
                    exact_graveyard_card_copy_cast_sequence(tail)
            })
        });
    let reconciled_quantified_token_rules = authored_tail
        .as_ref()
        .map(|tail| {
            crate::runtime_backend::effect_sentences::
                parse_quantified_token_creation_with_embedded_rules(tail)
        })
        .transpose()?
        .flatten()
        .or(source_tail
            .as_ref()
            .map(|tail| {
                crate::runtime_backend::effect_sentences::
                    parse_quantified_token_creation_with_embedded_rules(tail)
            })
            .transpose()?
            .flatten());
    let reconciled_library_origin =
        crate::runtime_backend::semantic_line_parsing::
            parse_library_origin_source_pump_unblockable_triggered_line(&authored_tokens)?
            .or(crate::runtime_backend::semantic_line_parsing::
                parse_library_origin_source_pump_unblockable_triggered_line(
                    &info.source_tokens,
                )?);
    let reconciled_library_effects = match reconciled_library_origin {
        Some(LineAst::Triggered {
            trigger: recovered_trigger,
            effects,
            ..
        }) => {
            trigger = recovered_trigger;
            Some(effects)
        }
        _ => None,
    };
    let reconciled_effects = reconciled_dynamic
        .map(|effect| vec![effect])
        .or(reconciled_looked_hand)
        .or(reconciled_graveyard_copy_cast)
        .or(reconciled_library_effects)
        .or_else(|| reconciled_quantified_token_rules.map(|effect| vec![effect]));
    if let Some(effects) = reconciled_effects {
        prepared.prepared = super::rewrite_prepare_effects_for_lowering(
            &effects,
            prepared.prepared.imports.clone(),
        )?;
    }

    fn contains_haunted_creature_dies(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::HauntedCreatureDies => true,
            TriggerSpec::WithIntro { trigger, .. } => contains_haunted_creature_dies(trigger),
            TriggerSpec::Either(left, right) => {
                contains_haunted_creature_dies(left) || contains_haunted_creature_dies(right)
            }
            _ => false,
        }
    }
    let contains_haunted_creature_dies = contains_haunted_creature_dies(&trigger);
    if restore_authored_source_exiled_return(&mut prepared.prepared.effects, &info.source_tokens)? {
        prepared.prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.prepared.effects,
            prepared.prepared.imports.clone(),
        )?;
    }
    let trigger_facts = &semantic_facts.triggered_ability;
    let functional_zones = super::infer_triggered_ability_functional_zones_from_facts(
        &trigger,
        &trigger_facts.functional_zones,
    );
    let mut intervening_if =
        trigger_frequency_condition_from_facts(max_triggers_per_turn, &trigger_facts.frequency);
    if trigger_facts.becomes_tapped_during_your_turn {
        let condition = crate::ConditionExpr::YourTurn;
        intervening_if = Some(match intervening_if {
            Some(existing) => crate::ConditionExpr::And(Box::new(condition), Box::new(existing)),
            None => condition,
        });
    }

    let parsed = super::rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.prepared.effects.clone(),
        functional_zones,
        Some(info.raw_line.clone()),
        intervening_if,
        trigger_facts.presentation_label.as_ref(),
        prepared.prepared.imports.clone(),
    );
    let mut parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
        preserve_condition_qualified_stun_reminder(triggered, &info.raw_line);
        dedupe_lowered_adjacent_target_declarations(&mut triggered.effects);
        bind_source_exiled_return_complement(&mut triggered.effects);
        reconcile_authored_source_exiled_return_runtime(
            &mut triggered.effects,
            &info.source_tokens,
        );
        normalize_graveyard_card_copy_cast_program(&mut triggered.effects);
        preserve_separate_copy_instruction_surface(&mut triggered.effects, &authored_tokens);
        bind_each_opponent_sacrifice_failure_half_life(&mut triggered.effects, &authored_tokens);
        bind_dynamic_power_owner_exile_permission(
            &mut triggered.effects,
            &info.source_tokens,
            &info.raw_line,
        );
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut triggered.effects,
            Some(&info.source_tokens),
        );
        restore_authored_return_then_venture(triggered, &info.source_tokens);
        transport_plural_copy_retarget_into_delayed_trigger(&mut triggered.effects);
        transport_fixed_retarget_into_optional_copy(&mut triggered.effects);
        bind_exile_top_card_cast_attempt_and_fallback(&mut triggered.effects);
        retarget_source_move_to_damaged_death_card(triggered);
        bind_equipped_attack_subject_to_result_pump(triggered);
        bind_combat_damage_group_controller_draw(triggered, &authored_tokens);
        bind_equipped_attack_draw_reveal_result(triggered, &info.source_tokens);
        bind_later_attacker_choice_to_prior_target_power(triggered);
        bind_authored_single_target_spell_cast_filter(triggered, &info.source_tokens);
        bind_authored_spell_cast_color_list(triggered, &info.source_tokens);
        bind_authored_spell_cast_ability_marker(triggered, &info.source_tokens);
        bind_authored_spell_cast_relation_constraints(triggered, &info.source_tokens);
        bind_original_and_copy_plural_keyword_grant(triggered, &info.source_tokens);
        bind_authored_chosen_creature_sacrifice(triggered, &authored_tokens);
    }
    if contains_haunted_creature_dies && let AbilityKind::Triggered(triggered) = parsed.kind() {
        state.haunt_linkage = Some((
            triggered
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter().cloned())
                .collect(),
            triggered.choices.clone(),
        ));
    }
    Ok(builder.with_ability(parsed.into_runtime()))
}
