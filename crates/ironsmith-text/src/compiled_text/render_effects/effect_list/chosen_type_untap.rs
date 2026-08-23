use super::*;

/// Restores the single-clause creature-type selection from the executable
/// choice followed by an untap of exactly the chosen-type creature set.
pub(super) fn describe_choose_creature_type_then_untap_all(effects: &[Effect]) -> Option<String> {
    let [choose_effect, untap_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()?;
    let untap = structural_unwrap_render_wrappers(untap_effect)
        .downcast_ref::<crate::effects::UntapEffect>()?;
    let ChooseSpec::All(filter) = untap.target.base() else {
        return None;
    };
    if choose.family != crate::types::SubtypeFamily::Creature
        || choose.chooser != PlayerFilter::You
        || !choose.excluded_subtypes.is_empty()
        || filter.zone != Some(Zone::Battlefield)
        || filter.card_types.as_slice() != [CardType::Creature]
        || !filter.chosen_creature_type
    {
        return None;
    }

    let mut residual = filter.clone();
    residual.zone = None;
    residual.card_types.clear();
    residual.chosen_creature_type = false;
    residual.set_explicit_card_type_noun(None);
    if residual != ObjectFilter::default() {
        return None;
    }

    Some("Untap all creatures of the creature type of your choice".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(mut filter: ObjectFilter) -> Vec<Effect> {
        filter.zone = Some(Zone::Battlefield);
        filter.card_types = vec![CardType::Creature];
        filter.chosen_creature_type = true;
        filter.set_explicit_card_type_noun(Some(CardType::Creature));
        vec![
            Effect::new(crate::effects::ChooseCreatureTypeEffect::new(
                PlayerFilter::You,
                Vec::new(),
            )),
            Effect::new(crate::effects::UntapEffect::all(filter)),
        ]
    }

    #[test]
    fn renders_only_the_exact_inline_chosen_type_untap_pair() {
        assert_eq!(
            describe_choose_creature_type_then_untap_all(&fixture(ObjectFilter::default()))
                .as_deref(),
            Some("Untap all creatures of the creature type of your choice")
        );

        let mut controlled = ObjectFilter::default();
        controlled.controller = Some(PlayerFilter::You);
        assert!(describe_choose_creature_type_then_untap_all(&fixture(controlled)).is_none());
    }
}
