#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::PutCountersEffect;
use ironsmith_core::{TurnHistoryCount, ValueSurfaceHint};

fn find_nested_effect<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested_effect(child);
        }
    });
    found
}

fn find_definition_effect<T: Clone + 'static>(name: &str) -> T {
    let definition = parse_oracle_card_definition(name);
    let mut programs = definition.spell_effect.iter().collect::<Vec<_>>();
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Triggered(triggered) => programs.push(&triggered.effects),
            AbilityKind::Activated(activated) => programs.push(&activated.effects),
            _ => {}
        }
    }
    programs
        .into_iter()
        .flat_map(|program| &program.segments)
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested_effect::<T>)
        .unwrap_or_else(|| panic!("{name} should contain {}", std::any::type_name::<T>()))
}

fn compiled_text(name: &str) -> String {
    unprocessed_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn dynamic_counter_amounts_render_the_authored_per_basis_surface() {
    for (name, expected) in [
        (
            "Outlaw Stitcher",
            "put two +1/+1 counters on that token for each spell you've cast this turn other than the first",
        ),
        (
            "Geralf, the Fleshwright",
            "put a +1/+1 counter on it for each other Zombie that entered the battlefield under your control this turn",
        ),
        (
            "Furious Spinesplitter",
            "put a +1/+1 counter on this creature for each opponent who was dealt damage this turn",
        ),
        (
            "The Elderspell",
            "Put two loyalty counters on it for each planeswalker destroyed this way",
        ),
        (
            "Azula, Ruthless Firebender",
            "you get an experience counter for each player who discarded a card this turn",
        ),
        (
            "Sheriff of Safe Passage",
            "This creature enters with a +1/+1 counter on it plus an additional +1/+1 counter on it for each other creature you control",
        ),
    ] {
        let text = compiled_text(name);
        assert!(
            text.contains(expected),
            "{name} should contain {expected:?}; got:\n{text}"
        );
    }
}

#[test]
fn counter_multipliers_and_filtered_turn_history_bases_remain_typed() {
    let outlaw: PutCountersEffect = find_definition_effect("Outlaw Stitcher");
    assert!(outlaw.amount.has_surface_hint(ValueSurfaceHint::ForEach));
    let Value::Add(left, right) = outlaw.amount.unhinted() else {
        panic!("Outlaw Stitcher should retain two equal history-count addends: {outlaw:#?}");
    };
    assert_eq!(left, right);
    assert!(matches!(
        left.unhinted(),
        Value::Add(count, offset)
            if matches!(
                count.unhinted(),
                Value::SpellsCastThisTurn(PlayerFilter::You)
            ) && matches!(offset.unhinted(), Value::Fixed(-1))
    ));

    let elderspell: PutCountersEffect = find_definition_effect("The Elderspell");
    assert!(
        elderspell
            .amount
            .has_surface_hint(ValueSurfaceHint::ForEach)
    );
    let Value::Add(left, right) = elderspell.amount.unhinted() else {
        panic!("The Elderspell should retain two equal prior-result counts: {elderspell:#?}");
    };
    assert_eq!(left, right);
    assert!(matches!(
        left.unhinted(),
        Value::PriorEffectMetric { query, .. }
            if query.action == Some(crate::effect::PriorEffectAction::Destroyed)
    ));

    let geralf: PutCountersEffect = find_definition_effect("Geralf, the Fleshwright");
    assert!(geralf.amount.has_surface_hint(ValueSurfaceHint::ForEach));
    assert!(matches!(
        geralf.amount.unhinted(),
        Value::TurnHistoryCount(TurnHistoryCount::EnteredBattlefield(filter))
            if filter.controller == Some(PlayerFilter::You)
                && filter.other
                && filter.subtypes.contains(&Subtype::Zombie)
    ));

    let furious: PutCountersEffect = find_definition_effect("Furious Spinesplitter");
    assert!(furious.amount.has_surface_hint(ValueSurfaceHint::ForEach));
    assert!(matches!(
        furious.amount.unhinted(),
        Value::TurnHistoryCount(TurnHistoryCount::PlayersDealtDamage(PlayerFilter::Opponent))
    ));

    let azula: crate::effects::ExperienceCountersEffect =
        find_definition_effect("Azula, Ruthless Firebender");
    assert!(azula.count.has_surface_hint(ValueSurfaceHint::ForEach));
    assert!(matches!(
        azula.count.unhinted(),
        Value::TurnHistoryCount(TurnHistoryCount::PlayersDiscarded(PlayerFilter::Any))
    ));

    let sheriff = parse_oracle_card_definition("Sheriff of Safe Passage");
    let static_ability = sheriff
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::EnterWithCounters =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("Sheriff should retain a typed entry-counter ability");
    let model = static_ability
        .compiled_model()
        .expect("Sheriff entry counters should retain a compiled model");
    let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { count, .. } =
        &model.payload
    else {
        panic!("Sheriff should lower to a typed entry-counter value");
    };
    let Value::Add(left, right) = count.unhinted() else {
        panic!("Sheriff should retain the fixed base plus dynamic addend: {count:#?}");
    };
    assert!(matches!(left.unhinted(), Value::Fixed(1)));
    assert!(right.has_surface_hint(ValueSurfaceHint::ForEach));
    assert!(matches!(
        right.unhinted(),
        Value::Count(filter)
            if filter.other
                && filter.card_types == vec![CardType::Creature]
                && filter.controller == Some(PlayerFilter::You)
    ));
}
