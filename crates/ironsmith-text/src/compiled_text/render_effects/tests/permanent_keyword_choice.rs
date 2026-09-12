use super::*;

#[test]
fn permanent_keyword_choice_zero_color_exceptions_render_as_universal_condition() {
    let text = "At the beginning of your upkeep, if all nonland permanents you control are white, you gain 1 life.";
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Color Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        text
    );
    let mut filter = ObjectFilter::default();
    filter.excluded_colors = crate::color::ColorSet::from(crate::color::Color::White);
    let condition = |count| Condition::PlayerControlsExactly {
        player: PlayerFilter::You,
        filter: filter.clone(),
        count,
    };
    assert_eq!(
        describe_condition(&condition(0)),
        "all permanents you control are white"
    );
    assert_eq!(
        describe_condition(&condition(1)),
        "you control exactly one nonwhite permanent"
    );
}

#[test]
fn permanent_keyword_choice_renders_shared_target_and_source_alternatives() {
    for text in [
        "{G}{W}, Discard a card: Put a +1/+1 counter on target creature or that creature gains banding, first strike, or trample.",
        "{2}, Discard a card: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample.",
    ] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text
        );
    }
}

struct NestedChoices {
    outer: usize,
    inner: usize,
    calls: usize,
}
impl crate::decision::DecisionMaker for NestedChoices {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let choice = if self.calls == 0 {
            assert_eq!(ctx.options.len(), 2);
            self.outer
        } else {
            assert_eq!(ctx.options.len(), 3);
            self.inner
        };
        self.calls += 1;
        vec![choice]
    }
}

#[test]
fn permanent_keyword_choice_resolves_each_branch_independently_of_announced_modes() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{2}: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample.")
        .unwrap();
    let ability = card
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            crate::ability::AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    use crate::static_abilities::StaticAbilityId;
    let keywords = [
        StaticAbilityId::Flying,
        StaticAbilityId::FirstStrike,
        StaticAbilityId::Trample,
    ];
    for (outer, inner) in [(0, 0), (1, 0), (1, 1), (1, 2)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let mut decision = NestedChoices {
            outer,
            inner,
            calls: 0,
        };
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decision)
            .with_chosen_modes(Some(vec![0]));
        for effect in ability.effects.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(decision.calls, if outer == 0 { 1 } else { 2 });
        assert_eq!(
            game.counter_count(source, CounterType::PlusOnePlusOne),
            u32::from(outer == 0)
        );
        for (index, keyword) in keywords.iter().enumerate() {
            assert_eq!(
                game.object_has_static_ability_id(source, *keyword),
                outer == 1 && inner == index
            );
        }
    }
}
