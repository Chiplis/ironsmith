//! Definition-local links for executable scalar references (CR 607).
use crate::ability::AbilityKind;
use crate::cards::CardDefinition;
use crate::effect::{Effect, Value};
use crate::resolution::ResolutionProgram;
use crate::target::ChooseSpec;
use sha2::{Digest, Sha256};

fn program(ability: &crate::ability::Ability) -> Option<&ResolutionProgram> {
    match &ability.kind {
        AbilityKind::Triggered(ability) => Some(&ability.effects),
        AbilityKind::Activated(ability) => Some(&ability.effects),
        _ => None,
    }
}

fn scalar_consumer(value: &Value) -> bool {
    match value {
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) =>
            matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG),
        Value::SurfaceHinted { value, .. } | Value::Scaled(value, _) |
        Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => scalar_consumer(value),
        Value::Add(left, right) | Value::Min(left, right) => scalar_consumer(left) || scalar_consumer(right),
        _ => false,
    }
}

fn inspect(effect: &Effect, producers: &mut usize, compatible: &mut usize, consumes: &mut bool) {
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        *producers += 1;
        if exile.duration == ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield
            && exile.leave_watcher.is_none()
        {
            *compatible += 1;
        }
    } else if effect.downcast_ref::<crate::effects::ExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>().is_some()
        || effect.downcast_ref::<crate::effects::cards::ImprintFromHandEffect>().is_some()
        || effect.downcast_ref::<crate::effects::HauntExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::ExileTaggedWhenSourceLeavesEffect>().is_some()
        || effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|effect| effect.zone == crate::zone::Zone::Exile)
    {
        *producers += 1;
    }
    crate::compile_support::visit_direct_nested_effect_values(effect, &mut |value| {
        *consumes |= scalar_consumer(value);
    });
    effect.visit_child_effects(&mut |child| inspect(child, producers, compatible, consumes));
}

/// Stamp only a proven unambiguous executable pair. Unknown, multi-producer,
/// static, and independently authored native bodies keep absent metadata and
/// cannot use the scalar reader. Run after all program-rebuilding finalizers.
pub(super) fn bind_scalar_linked_exile(definition: &mut CardDefinition) {
    // Until reference analysis exports explicit multi-pair relationships, a
    // third ability scope (including a static producer or a second pair) is
    // not evidence that these two abilities are linked. Nonmana costs may
    // themselves exile objects and need a separate typed producer inventory.
    if definition.abilities.len() != 2 || definition.abilities.iter().any(|ability| {
        match &ability.kind {
            AbilityKind::Triggered(_) => false,
            AbilityKind::Activated(ability) => ability.mana_cost.has_non_mana_costs()
                || ability.mana_cost.dynamic_mana_cost().is_some()
                || ability.mana_cost.as_one_of().is_some(),
            _ => true,
        }
    }) { return; }
    let mut producer = None;
    let mut consumers = Vec::new();
    let mut total = 0;
    let mut compatible_total = 0;
    for (slot, ability) in definition.abilities.iter().enumerate() {
        let Some(program) = program(ability) else { continue; };
        let (mut count, mut compatible, mut consumes) = (0, 0, false);
        for effect in program.all_effects() {
            inspect(effect, &mut count, &mut compatible, &mut consumes);
        }
        total += count;
        compatible_total += compatible;
        if compatible == 1 { producer = Some(slot); }
        if consumes { consumers.push(slot); }
    }
    let Some(producer) = producer else { return; };
    if total != 1 || compatible_total != 1 || consumers.len() != 1 || consumers[0] == producer { return; }
    // Hash typed executable definitions, never a caller's local card number,
    // physical host or Debug rendering. Pair compatibility above uses only
    // typed effects and values; presentation fields never decide membership.
    let Ok(bytes) = serde_json::to_vec(&definition.abilities) else { return; };
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into()),
        pair: producer as u32,
    };
    for (slot, ability) in definition.abilities.iter_mut().enumerate() {
        if slot != producer && !consumers.contains(&slot) { continue; }
        let effects = match &mut ability.kind {
            AbilityKind::Triggered(ability) => &mut ability.effects,
            AbilityKind::Activated(ability) => &mut ability.effects,
            _ => continue,
        };
        effects.linked_exile_pair = Some(pair);
    }
}
