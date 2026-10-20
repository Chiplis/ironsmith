//! Rejoin a per-player pile partition with its random pile sacrifice.
//! The first `n - 1` choices tag explicit piles; the final pile is the
//! complement. Compare full effects so other choices or costs cannot vanish.
use super::*;

pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    let [separation, sacrifice] = effects else { return None };
    let separation = separation.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let sacrifice = sacrifice.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if separation.filter != PlayerFilter::Any
        || separation.sequential
        || separation.stop_after_first_happened
        || sacrifice.filter != PlayerFilter::Any
        || sacrifice.starting_with_controller
        || sacrifice.sequential
        || sacrifice.stop_after_first_happened
    {
        return None;
    }

    let [choice_effect] = sacrifice.effects.as_slice() else { return None };
    let choice = choice_effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let pile_count = choice.modes.len();
    if !(2..=usize::from(u8::MAX)).contains(&pile_count)
        || separation.effects.len() != pile_count - 1
    {
        return None;
    }
    let mut expected_choice =
        crate::effects::ChooseModeEffect::choose_one(choice.modes.clone());
    expected_choice.chooser = Some(PlayerFilter::You);
    expected_choice.random = true;
    if choice != &expected_choice {
        return None;
    }

    let base = ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer);
    let mut pile_tags = Vec::with_capacity(pile_count - 1);
    for choose_effect in &separation.effects {
        let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if pile_tags.contains(&choose.tag) {
            return None;
        }
        let filter = pile_tags.iter().fold(base.clone(), |filter, tag| {
            filter.not_tagged(tag.clone())
        });
        let expected = crate::effects::ChooseObjectsEffect::new(
            filter,
            crate::effect::ChoiceCount::any_number(),
            PlayerFilter::IteratedPlayer,
            choose.tag.clone(),
        )
        .in_zone(Zone::Battlefield);
        if choose != &expected {
            return None;
        }
        pile_tags.push(choose.tag.clone());
    }

    for (index, mode) in choice.modes.iter().enumerate() {
        let [sacrifice_effect] = mode.effects.as_slice() else { return None };
        let sacrifice_effect = sacrifice_effect
            .downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()?;
        let filter = if let Some(tag) = pile_tags.get(index) {
            base.clone().match_tagged(
                tag.clone(),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            )
        } else {
            pile_tags.iter().fold(base.clone(), |filter, tag| {
                filter.not_tagged(tag.clone())
            })
        };
        let expected = crate::effects::zones::SacrificePlayerEffect::new(
            filter.clone(),
            Value::Count(filter),
            PlayerFilter::IteratedPlayer,
        );
        if mode.source_text != format!("Pile {}", index + 1)
            || sacrifice_effect != &expected
        {
            return None;
        }
    }

    let noun = match pile_count {
        2 => "two piles".to_string(),
        3 => "three piles".to_string(),
        4 => "four piles".to_string(),
        5 => "five piles".to_string(),
        _ => format!("{pile_count} piles"),
    };
    let order = if separation.starting_with_controller {
        "Starting with you, each player"
    } else {
        "Each player"
    };
    Some(format!(
        "{order} separates all permanents they control into {noun}. Then each player chooses one of their piles at random and sacrifices those permanents"
    ))
}
