use super::*;
use ironsmith_compiler_resolve::selection_scope::{
    validate_card_selection_filter, validate_card_selection_spec,
};

/// Validate the executable selections after lowering has supplied implicit
/// source zones (for example a library search's zone). Do not walk arbitrary
/// object filters: replacement matchers and trigger predicates are not choices.
pub(super) fn validate_card_selections(
    effects: &[Effect],
    choices: &[ChooseSpec],
) -> Result<(), CardTextError> {
    fn validate_effect(effect: &Effect) -> Result<(), CardTextError> {
        if let Some(spec) = effect.target_spec() {
            validate_card_selection_spec(spec)?;
        }
        if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            validate_card_selection_filter(&choose.filter, choose.zone)?;
        }
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
            validate_card_selection_spec(&exile.spec)?;
            if let Some(watcher) = &exile.leave_watcher {
                validate_card_selection_spec(watcher)?;
            }
        }
        let mut result = Ok(());
        effect.visit_child_effects(&mut |child| {
            if result.is_ok() {
                result = validate_effect(child);
            }
        });
        result
    }
    for choice in choices {
        validate_card_selection_spec(choice)?;
    }
    for effect in effects {
        validate_effect(effect)?;
    }
    Ok(())
}
