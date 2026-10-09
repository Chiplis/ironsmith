//! Targets "chosen at random" (Goblin Test Pilot, Witch Hunt). The target is
//! still announced as the spell or ability is put on the stack (CR 601.2c,
//! 602.2b, 603.3d), but the game, not a player, picks it uniformly among the
//! legal choices. The pick consumes the game's replayable random stream, and
//! the requirement is narrowed to exactly that pick so the ordinary
//! announcement flow (and its forced-choice auto-placement) records it.
use crate::decision::TargetRequirement;
use crate::game_state::{GameState, Target};
use crate::target::ChooseSpec;

/// Whether this target specification asks for a random selection.
pub fn spec_selects_targets_at_random(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } | ChooseSpec::Target(spec) => {
            spec_selects_targets_at_random(spec)
        }
        ChooseSpec::WithCount(_, count) | ChooseSpec::WithCountValue(_, count, _) => count.random,
        _ => false,
    }
}

/// Narrow a random-target requirement to the game's random pick. A requirement
/// that is not random is left untouched.
pub fn narrow_requirement_to_random_targets(game: &GameState, requirement: &mut TargetRequirement) {
    narrow_fields(
        game,
        &requirement.spec,
        &mut requirement.legal_targets,
        &mut requirement.legal_target_sets,
        &mut requirement.min_targets,
        &mut requirement.max_targets,
    );
}

/// The same narrowing on an already range-filtered decision context.
pub fn narrow_context_to_random_targets(
    game: &GameState,
    spec: &ChooseSpec,
    context: &mut crate::decisions::context::TargetRequirementContext,
) {
    narrow_fields(
        game,
        spec,
        &mut context.legal_targets,
        &mut context.legal_target_sets,
        &mut context.min_targets,
        &mut context.max_targets,
    );
}

fn narrow_fields(
    game: &GameState,
    spec: &ChooseSpec,
    legal_targets: &mut Vec<Target>,
    legal_target_sets: &mut Vec<Vec<Target>>,
    min_targets: &mut usize,
    max_targets: &mut Option<usize>,
) {
    if !spec_selects_targets_at_random(spec) || legal_targets.is_empty() {
        return;
    }
    let wanted = max_targets
        .unwrap_or(*min_targets)
        .max(*min_targets)
        .max(1)
        .min(legal_targets.len());
    let mut pool = legal_targets.clone();
    let mut picked: Vec<Target> = Vec::with_capacity(wanted);
    while picked.len() < wanted && !pool.is_empty() {
        let index = (game.next_random_u64() % pool.len() as u64) as usize;
        picked.push(pool.swap_remove(index));
    }
    legal_target_sets.retain(|set| set.iter().all(|target| picked.contains(target)));
    *legal_targets = picked;
    *min_targets = wanted;
    *max_targets = Some(wanted);
}
