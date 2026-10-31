//! Small executable contracts for parser changes with ordered runtime behavior.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, PowerToughness, Zone};

pub fn check(name: &str, definition: &CardDefinition) -> Vec<String> {
    match name {
        "Unification" => {
            let mut failures = check_owner_draw_then_shuffle(definition);
            failures.extend(check_owner_program_audit_mutations(definition));
            failures
        }
        "Kes, Evolving Telepath" => check_counter_draw_then_sacrifice(definition),
        _ => vec![format!("no runtime contract registered for {name}")],
    }
}

fn check_owner_draw_then_shuffle(definition: &CardDefinition) -> Vec<String> {
    let Some(chapter) = definition.abilities.iter().find_map(|ability| {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            let debug = format!("{:?}", triggered.effects);
            if debug.contains("DrawCardsEffect")
                && debug.contains("ShuffleObjectsIntoLibraryEffect")
            {
                return Some(triggered);
            }
        }
        None
    }) else {
        return vec!["chapter I must contain both draw and shuffle effects".into()];
    };
    let mut failures = Vec::new();
    for choose_target in [true, false] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(
            vec!["Ability controller".into(), "Creature owner".into()],
            20,
        );
        let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
        let creature = CardDefinitionBuilder::new(CardId::new(), "Owned creature fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        // Make controller and owner differ, so an accidental controller draw
        // cannot satisfy this contract.
        game.object_mut(target).unwrap().initial_controller = alice;
        let stable = game.object(target).unwrap().stable_id;
        let card = CardDefinitionBuilder::new(CardId::new(), "Library fixture")
            .card_types(vec![CardType::Land])
            .build();
        for _ in 0..2 {
            game.create_object_from_definition(&card, bob, Zone::Library);
        }
        let mut ctx = EffectContext::new_default(source, alice);
        if choose_target {
            ctx.targets = vec![ResolvedTarget::Object(target)];
        }
        let mut events = Vec::new();
        for effect in &chapter.effects {
            match execute_effect(&mut game, effect, &mut ctx) {
                Ok(outcome) => events.extend(outcome.events),
                Err(error) => {
                    failures.push(format!("target={choose_target}: execution error {error:?}"));
                    break;
                }
            }
        }
        let expected_draws = if choose_target { 2 } else { 0 };
        if game.player(bob).unwrap().hand.len() != expected_draws {
            failures.push(format!(
                "target={choose_target}: owner must draw {expected_draws} cards"
            ));
        }
        if !game.player(alice).unwrap().hand.is_empty() {
            failures.push(format!("target={choose_target}: controller must not draw"));
        }
        let final_target = game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id));
        let expected_zone = if choose_target {
            Zone::Library
        } else {
            Zone::Battlefield
        };
        if final_target.is_none_or(|object| object.zone != expected_zone || object.owner != bob) {
            failures.push(format!(
                "target={choose_target}: target must finish in owner's {expected_zone:?}"
            ));
        }
        if choose_target {
            let draw_indices: Vec<_> = events
                .iter()
                .enumerate()
                .filter_map(|(index, event)| {
                    event
                        .downcast::<ironsmith::events::CardsDrawnEvent>()
                        .filter(|draw| draw.player == bob)
                        .map(|_| index)
                })
                .collect();
            // The shuffle executor commits the zone move and randomization
            // as one instruction. Its aggregate outcome exposes the completed
            // shuffle event; zone-change receipts use retained output channels.
            let shuffle_index = events.iter().position(|event| {
                event
                    .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                    .is_some_and(|shuffle| shuffle.player == bob)
            });
            if draw_indices.is_empty()
                || shuffle_index
                    .is_none_or(|shuffle| draw_indices.iter().any(|draw| *draw >= shuffle))
            {
                failures.push(
                    "owner's completed draws must precede the owner's completed library shuffle"
                        .into(),
                );
            }
        } else if events.iter().any(|event| {
            event
                .downcast::<ironsmith::events::CardsDrawnEvent>()
                .is_some()
        }) {
            failures.push("zero targets must emit no draw events".into());
        }
    }
    failures
}

fn check_counter_draw_then_sacrifice(definition: &CardDefinition) -> Vec<String> {
    let Some(triggered) = definition.abilities.iter().find_map(|ability| {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            Some(triggered)
        } else {
            None
        }
    }) else {
        return vec!["counter/draw/sacrifice trigger missing".into()];
    };
    let mut failures = Vec::new();
    for starting_counters in [7u32, 8] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        game.add_counters(source, ironsmith::CounterType::Time, starting_counters);
        let card = CardDefinitionBuilder::new(CardId::new(), "Draw fixture")
            .card_types(vec![CardType::Land])
            .build();
        for _ in 0..12 {
            game.create_object_from_definition(&card, alice, Zone::Library);
        }
        let mut ctx = EffectContext::new_default(source, alice);
        for effect in &triggered.effects {
            if let Err(error) = execute_effect(&mut game, effect, &mut ctx) {
                failures.push(format!(
                    "{starting_counters} counters: execution failed {error:?}"
                ));
                break;
            }
        }
        if game.player(alice).unwrap().hand.len() != (starting_counters + 1) as usize {
            failures.push(format!(
                "{starting_counters} counters: draw must use post-increment source counter count"
            ));
        }
        let expected = if starting_counters == 8 {
            Zone::Graveyard
        } else {
            Zone::Battlefield
        };
        if game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id))
            .is_none_or(|object| object.zone != expected)
        {
            failures.push(format!(
                "{starting_counters} counters: source must finish in {expected:?}"
            ));
        }
    }
    failures
}

fn mutate_owner_program(
    effect: ironsmith::Effect,
    mutation: u8,
    changed: &mut bool,
) -> ironsmith::Effect {
    use ironsmith::effects as fx;
    if let Some(with_id) = effect.downcast_ref::<fx::WithIdEffect>() {
        let mut copy = with_id.clone();
        copy.effect = Box::new(mutate_owner_program(*copy.effect, mutation, changed));
        return ironsmith::Effect::new(copy);
    }
    if let Some(tagged) = effect.downcast_ref::<fx::ForEachTaggedEffect>() {
        let mut copy = tagged.clone();
        if mutation == 0
            && format!("{:?}", copy.effects).contains("ShuffleObjectsIntoLibraryEffect")
        {
            copy.effects.reverse();
            *changed = true;
        } else {
            copy.effects = copy
                .effects
                .into_iter()
                .map(|child| mutate_owner_program(child, mutation, changed))
                .collect();
        }
        return ironsmith::Effect::new(copy);
    }
    if mutation == 1 {
        if let Some(draw) = effect.downcast_ref::<fx::DrawCardsEffect>() {
            let mut copy = draw.clone();
            copy.player = ironsmith::target::PlayerFilter::You;
            *changed = true;
            return ironsmith::Effect::new(copy);
        }
    }
    if mutation == 2 {
        if let Some(shuffle) = effect.downcast_ref::<fx::ShuffleObjectsIntoLibraryEffect>() {
            let mut copy = shuffle.clone();
            copy.target =
                ironsmith::target::ChooseSpec::Tagged("different_destination_object".into());
            *changed = true;
            return ironsmith::Effect::new(copy);
        }
    }
    effect
}

fn check_owner_program_audit_mutations(definition: &CardDefinition) -> Vec<String> {
    let original = ironsmith::compiled_text::semantic_audit_lines(definition);
    let mut failures = Vec::new();
    for (mutation, label) in [
        (0, "reversed execution order"),
        (1, "changed drawing player"),
        (2, "changed shuffle target tag"),
    ] {
        let mut modified = definition.clone();
        let mut changed = false;
        for ability in &mut modified.abilities {
            if let AbilityKind::Triggered(triggered) = &mut ability.kind {
                if !format!("{:?}", triggered.effects).contains("ShuffleObjectsIntoLibraryEffect") {
                    continue;
                }
                triggered.effects = std::mem::take(&mut triggered.effects)
                    .try_map_effects(|effect| {
                        Ok::<_, std::convert::Infallible>(mutate_owner_program(
                            effect,
                            mutation,
                            &mut changed,
                        ))
                    })
                    .unwrap();
            }
        }
        if !changed {
            failures.push(format!("audit mutation did not locate {label}"));
        } else if ironsmith::compiled_text::semantic_audit_lines(&modified) == original {
            failures.push(format!("audit concealed {label}"));
        }
    }
    failures
}
