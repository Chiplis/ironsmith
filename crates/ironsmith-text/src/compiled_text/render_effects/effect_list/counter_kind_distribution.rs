use super::*;

pub(super) fn describe_created_token_counter_kind_distribution(
    effects: &[Effect],
) -> Option<String> {
    let [create_effect, distribution_effect] = effects else {
        return None;
    };
    let (created_tag, _) = tagged_create_token_effect(create_effect)?;
    let distribution = structural_unwrap_render_wrappers(distribution_effect)
        .downcast_ref::<crate::effects::ForEachCounterKindPutOrRemoveEffect>()?;
    if !distribution.all_kinds
        || distribution.fixed_counter_type.is_some()
        || distribution.optional_action
        || !distribution.put_only
        || !distribution.choose_target_per_kind
        || !matches!(distribution.target.base(), ChooseSpec::Tagged(tag) if tag == created_tag)
    {
        return None;
    }
    let counter_source = distribution.counter_source.as_ref()?;
    if !matches!(counter_source.base(), ChooseSpec::All(_)) {
        return None;
    }

    let create = describe_effect(create_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let source = describe_choose_spec(counter_source);
    let source = source.strip_prefix("all ").unwrap_or(&source);
    let sentence_boundary = if create.ends_with(".\"") {
        " Then"
    } else {
        ". Then"
    };
    Some(format!(
        "{create}{sentence_boundary} for each kind of counter among {source}, put a counter of that kind on either of those tokens"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(target_tag: &str, put_only: bool) -> Vec<Effect> {
        let created_tag = TagKey::from("created");
        let create = Effect::new(crate::effects::TaggedEffect::new(
            created_tag,
            Effect::new(crate::effects::CreateTokenEffect::new(
                crate::cards::tokens::blood_token_definition(),
                2,
                PlayerFilter::You,
            )),
        ));
        let mut source_filter = ObjectFilter::creature();
        source_filter.controller = Some(PlayerFilter::You);
        let source = ChooseSpec::All(source_filter);
        let mut distribution =
            crate::effects::ForEachCounterKindPutOrRemoveEffect::put_each_kind_from(
                source,
                ChooseSpec::Tagged(TagKey::from(target_tag)),
            );
        distribution.put_only = put_only;
        vec![create, Effect::new(distribution)]
    }

    #[test]
    fn created_token_distribution_requires_the_exact_tag_and_put_only_semantics() {
        let rendered = describe_created_token_counter_kind_distribution(&effects("created", true))
            .expect("exact distribution should render");
        assert!(
            rendered.contains("for each kind of counter among"),
            "{rendered}"
        );
        assert!(rendered.ends_with("either of those tokens"));
        assert!(
            describe_created_token_counter_kind_distribution(&effects("other", true)).is_none()
        );
        assert!(
            describe_created_token_counter_kind_distribution(&effects("created", false)).is_none()
        );
    }

    #[test]
    fn parsed_created_token_distribution_reaches_the_structural_renderer() {
        let text = "Create two 1/1 blue Fish creature tokens with \"This token can't be blocked.\" Then for each kind of counter among creatures you control, put a counter of that kind on either of those tokens.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Exotic Pets")
                .card_types(vec![CardType::Instant])
                .parse_text(text)
                .expect("distribution should compile");
        let effects = &definition
            .spell_effect
            .as_ref()
            .expect("spell program")
            .segments[0]
            .default_effects;

        assert_eq!(
            describe_created_token_counter_kind_distribution(effects).as_deref(),
            Some(text.trim_end_matches('.')),
            "{effects:#?}"
        );
    }
}
