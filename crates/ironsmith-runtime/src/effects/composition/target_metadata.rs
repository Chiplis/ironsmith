//! Shared target metadata forwarding helpers for composition wrappers.

use crate::effect::{ChoiceCount, Effect};
use crate::target::ChooseSpec;

fn first_targeted_effect<'a>(groups: &[&'a [Effect]]) -> Option<&'a Effect> {
    groups
        .iter()
        .flat_map(|group| group.iter())
        .find(|effect| effect.0.get_target_spec().is_some())
}

pub(super) fn first_target_spec<'a>(groups: &[&'a [Effect]]) -> Option<&'a ChooseSpec> {
    first_targeted_effect(groups).and_then(|effect| effect.0.get_target_spec())
}

pub(super) fn first_target_description(
    groups: &[&[Effect]],
    fallback: &'static str,
) -> &'static str {
    first_targeted_effect(groups)
        .map(|effect| effect.0.target_description())
        .unwrap_or(fallback)
}

pub(super) fn first_target_count(groups: &[&[Effect]]) -> Option<ChoiceCount> {
    first_targeted_effect(groups).and_then(|effect| effect.0.get_target_count())
}
