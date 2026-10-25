//! Bake prompt labels from executable alternatives without changing their
//! authored modal surfaces or linking the canonical renderer into gameplay.
use crate::effect::Effect;
use ironsmith_compiled_artifact::WireEffect;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub fn with_choice_decision_labels<T: Serialize + DeserializeOwned>(
    model: T,
    mut materialize: impl FnMut(WireEffect) -> Result<Effect, String>,
) -> Result<T, String> {
    fn visit(
        value: &mut Value,
        materialize: &mut impl FnMut(WireEffect) -> Result<Effect, String>,
    ) -> Result<(), String> {
        // Visit children first so nested choices carry their own labels too.
        match value {
            Value::Array(values) => {
                for value in values {
                    visit(value, materialize)?;
                }
            }
            Value::Object(fields) => {
                for value in fields.values_mut() {
                    visit(value, materialize)?;
                }
            }
            _ => return Ok(()),
        }
        if value.get("kind").and_then(Value::as_str) != Some("ChooseModeEffect") {
            return Ok(());
        }
        // Authored modal labels already cover these arms. Do not render them
        // again or replace a name such as a pile/color label with action text.
        let modes = value["payload"]["modes"].as_array();
        let existing = value["payload"]["decision_labels"].as_array();
        if modes.is_some_and(|modes| {
            modes.iter().enumerate().all(|(index, mode)| {
                existing
                    .and_then(|labels| labels.get(index))
                    .and_then(Value::as_str)
                    .is_some_and(|label| !label.trim().is_empty())
                    || mode["source_text"]
                        .as_str()
                        .is_some_and(|label| !label.trim().is_empty())
            })
        }) {
            return Ok(());
        }
        let wire: WireEffect = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        let runtime = materialize(wire)?;
        let choice = runtime
            .downcast_ref::<crate::effects::ChooseModeEffect>()
            .ok_or_else(|| "choice label materialization changed effect kind".to_string())?;
        if choice
            .modes
            .iter()
            .enumerate()
            .all(|(index, _)| !choice.decision_label(index).trim().is_empty())
        {
            return Ok(());
        }
        let labels = choice
            .modes
            .iter()
            .enumerate()
            .map(|(index, mode)| {
                let existing = choice.decision_label(index).trim();
                if !existing.is_empty() {
                    return existing.to_string();
                }
                let label = crate::compile_effect_list(&mode.effects);
                let label = label.trim().trim_end_matches('.');
                let label = label.strip_prefix("you ").unwrap_or(label);
                let mut chars = label.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_else(|| "Do nothing".to_string())
            })
            .collect::<Vec<_>>();
        value["payload"]["decision_labels"] =
            serde_json::to_value(labels).map_err(|e| e.to_string())?;
        Ok(())
    }
    let mut value = serde_json::to_value(model).map_err(|e| e.to_string())?;
    visit(&mut value, &mut materialize)?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectMode;
    use crate::effects::ChooseModeEffect;
    use ironsmith_runtime_catalog::artifact_materializer::{
        encode_runtime_effect, materialize_effect,
    };

    #[test]
    fn choice_labels_render_actions_and_preserve_authored_labels_and_empty_arms() {
        let choice = ChooseModeEffect::choose_one(vec![
            EffectMode::new("", vec![Effect::draw(2)]),
            EffectMode::new("", vec![Effect::gain_life(4)]),
            EffectMode::new("Keep this authored label", vec![]),
            EffectMode::new("", vec![]),
        ]);
        let wire = encode_runtime_effect(Effect::new(choice)).unwrap();
        let labeled = with_choice_decision_labels(wire, |wire| {
            materialize_effect(wire).map_err(|error| error.to_string())
        })
        .unwrap();
        let runtime = materialize_effect(labeled).unwrap();
        let choice = runtime.downcast_ref::<ChooseModeEffect>().unwrap();
        assert_eq!(choice.decision_label(0), "Draw two cards");
        assert_eq!(choice.decision_label(1), "Gain 4 life");
        assert_eq!(choice.decision_label(2), "Keep this authored label");
        assert_eq!(choice.decision_label(3), "Do nothing");
        assert!(choice.modes[0].source_text.is_empty());
        assert!(choice.modes[1].source_text.is_empty());
    }
}
