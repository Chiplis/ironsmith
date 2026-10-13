use super::*;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::decisions::{ReplacementOption, ReplacementSpec, make_decision};

/// Copy candidates are alternatives of one ability occurrence, not separate
/// replacement effects to order. Choose the occurrence first, then its outcome.
pub(super) fn choose_entry_replacement(
    game: &GameState,
    dm: &mut (impl DecisionMaker + ?Sized),
    player: PlayerId,
    effects: &[ReplacementEffect],
) -> Vec<usize> {
    let copy_keys: Vec<_> = effects
        .iter()
        .filter_map(|effect| {
            matches!(effect.replacement, ReplacementAction::EnterAsCopy { .. })
                .then(|| effect.application_key())
        })
        .collect();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, effect) in effects.iter().enumerate() {
        let key = effect.application_key();
        if copy_keys.contains(&key)
            && let Some(group) = groups
                .iter_mut()
                .find(|group| effects[group[0]].application_key() == key)
        {
            group.push(index);
        } else {
            groups.push(vec![index]);
        }
    }

    let group = if groups.len() == 1 && copy_keys.contains(&effects[0].application_key()) {
        &groups[0]
    } else {
        let options = groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let effect = &effects[group[0]];
                let description = if copy_keys.contains(&effect.application_key()) {
                    format!(
                        "Apply {} copy effect",
                        replacement_option_description(game, effect.source)
                    )
                } else {
                    replacement_effect_choice_description(game, effect)
                };
                ReplacementOption::new(index, effect.source, description).with_related_objects(
                    if copy_keys.contains(&effect.application_key()) {
                        Vec::new()
                    } else {
                        replacement_effect_related_objects(effect)
                    },
                )
            })
            .collect();
        let answer = make_decision(game, dm, player, None, ReplacementSpec::new(options));
        if dm.awaiting_choice() {
            return Vec::new();
        }
        let [index] = answer.as_slice() else {
            return Vec::new();
        };
        let Some(group) = groups.get(*index) else {
            return Vec::new();
        };
        group
    };

    if group.len() == 1 || !copy_keys.contains(&effects[group[0]].application_key()) {
        return vec![group[0]];
    }
    let options = group
        .iter()
        .map(|&index| {
            let effect = &effects[index];
            match &effect.replacement {
                ReplacementAction::EnterAsCopy { source, .. } => SelectableOption::new(
                    index,
                    replacement_effect_choice_description(game, effect),
                )
                .with_object(*source)
                .with_related_objects(replacement_effect_related_objects(effect)),
                _ => SelectableOption::new(index, "Don't copy"),
            }
        })
        .collect();
    let context = SelectOptionsContext::new(
        player,
        Some(effects[group[0]].source),
        "Choose what to copy as this permanent enters",
        options,
        1,
        1,
    );
    let answer = dm.decide_options(game, &context);
    // Do not allow an answer to escape the selected ability's alternatives.
    match answer.as_slice() {
        [index] if group.contains(index) => answer,
        _ => Vec::new(),
    }
}
