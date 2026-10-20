use super::*;

/// Recognize the complete five-color complement relation, including its
/// choice producer. A colored permanent survives exactly when all its colors
/// are the single color chosen by its controller; colorless objects survive.
pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    let [choice, action] = effects else { return None; };
    let choice = choice.downcast_ref::<crate::effects::ChoosePlayerOptionEffect>()?;
    let colors = crate::color::Color::ALL;
    let expected_choice = crate::effects::ChoosePlayerOptionEffect::new(
        PlayerFilter::Any,
        colors.iter().map(|color| color.name().to_string()).collect(),
        ironsmith_core::PlayerOptionChooser::Participant,
    );
    if choice != &expected_choice { return None; }
    let all_colors = colors.into_iter().collect::<crate::color::ColorSet>();
    let mut filter = ObjectFilter::permanent();
    filter.any_of = colors.into_iter().map(|color| {
        let mut branch = ObjectFilter::default();
        branch.controller = Some(PlayerFilter::TaggedPlayer(
            ironsmith_core::player_option_choice_tag(color.name()),
        ));
        branch.colors = Some(all_colors.without(color));
        branch
    }).collect();
    let expected = ChooseSpec::All(filter);
    let action = structural_unwrap_render_wrappers(action);
    let verb = if let Some(exile) = action.downcast_ref::<crate::effects::ExileEffect>() {
        if exile != &crate::effects::ExileEffect::with_spec(expected) { return None; }
        "exile"
    } else if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyEffect>() {
        if destroy != &crate::effects::DestroyEffect::with_spec(expected) { return None; }
        "destroy"
    } else { return None; };
    Some(format!("Each player chooses a color. Then {verb} each permanent unless it's colorless or it's only the color its controller chose"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(verb: &str) -> Vec<Effect> {
        let colors = crate::color::Color::ALL;
        let all = colors.into_iter().collect::<crate::color::ColorSet>();
        let mut filter = ObjectFilter::permanent();
        filter.any_of = colors.into_iter().map(|color| {
            let mut branch = ObjectFilter::default();
            branch.controller = Some(PlayerFilter::TaggedPlayer(ironsmith_core::player_option_choice_tag(color.name())));
            branch.colors = Some(all.without(color));
            branch
        }).collect();
        let action = if verb == "exile" {
            Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::All(filter)))
        } else {
            Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::All(filter)))
        };
        vec![Effect::new(crate::effects::ChoosePlayerOptionEffect::new(
            PlayerFilter::Any, colors.iter().map(|color| color.name().to_string()).collect(),
            ironsmith_core::PlayerOptionChooser::Participant,
        )), action]
    }

    #[test]
    fn parsed_color_complement_sweep_renders_the_survivor_condition() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(), "Color Sweep Probe",
        ).card_types(vec![crate::types::CardType::Sorcery])
        .parse_text("Each player chooses a color. Then exile each permanent unless it's colorless or it's only the color its controller chose.")
        .expect("color sweep compiles");
        let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(text.contains("Each player chooses a color. Then exile each permanent unless it's colorless or it's only the color its controller chose"), "{text}");
    }

    #[test]
    fn color_complement_sweep_preserves_action_and_survivors() {
        for verb in ["exile", "destroy"] {
            assert_eq!(describe(&probe(verb)), Some(format!("Each player chooses a color. Then {verb} each permanent unless it's colorless or it's only the color its controller chose")));
        }
    }

    #[test]
    fn color_complement_sweep_rejects_hidden_choice_and_exile_modifiers() {
        let mut effects = probe("exile");
        let mut choice = effects[0].downcast_ref::<crate::effects::ChoosePlayerOptionEffect>().unwrap().clone();
        choice.secret = true;
        effects[0] = Effect::new(choice);
        assert!(describe(&effects).is_none());
        let mut effects = probe("exile");
        let mut exile = effects[1].downcast_ref::<crate::effects::ExileEffect>().unwrap().clone();
        exile.face_down = true;
        effects[1] = Effect::new(exile);
        assert!(describe(&effects).is_none());
    }

    #[test]
    fn color_complement_sweep_rejects_incomplete_or_broadened_union() {
        for broaden in [false, true] {
            let mut effects = probe("exile");
            let mut exile = effects[1].downcast_ref::<crate::effects::ExileEffect>().unwrap().clone();
            let ChooseSpec::All(filter) = &mut exile.spec else { panic!("all filter"); };
            if broaden { filter.any_of[0].controller = None; } else { filter.any_of.pop(); }
            effects[1] = Effect::new(exile);
            assert!(describe(&effects).is_none());
        }
    }
}
