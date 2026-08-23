use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

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

#[test]
fn named_consult_result_surfaces_preserve_typed_alternatives_and_remainders() {
    let gamekeeper = compiled_card_text("Gamekeeper");
    assert!(
        gamekeeper.contains(
            "If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and put all other cards revealed this way into your graveyard"
        ),
        "{gamekeeper}"
    );

    let illuna = compiled_card_text("Illuna, Apex of Wishes");
    assert!(
        illuna.contains(
            "exile cards from the top of your library until you exile a nonland permanent card. You may put it onto the battlefield. If you don't, put it into its owner's hand"
        ),
        "{illuna}"
    );

    let ryan = compiled_card_text("Ryan Sinclair");
    assert!(
        ryan.contains(
            "exile cards from the top of your library until you exile a nonland card. If that card's mana value is less than or equal to Ryan's power, you may cast it without paying its mana cost. Put the exiled cards on the bottom of your library in a random order"
        ),
        "{ryan}"
    );

    let solstice = compiled_card_text("Solstice Revelations");
    assert!(
        solstice.contains(
            "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than the number of Mountains you control. If you don't cast that card this way, put it into your hand"
        ),
        "{solstice}"
    );

    let songbirds = compiled_card_text("Songbirds' Blessing");
    assert!(
        songbirds.contains(
            "reveal cards from the top of your library until you reveal an Aura card. You may put that card onto the battlefield. If you don't, put it into your hand. Put the rest on the bottom of your library in a random order"
        ),
        "{songbirds}"
    );
}

#[test]
fn leading_then_looked_partition_keeps_runtime_pool_provenance() {
    let definition = parse_oracle_card_definition("Wilfred Mott");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wilfred should have a triggered ability");
    let effects = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();

    let looked = effects
        .iter()
        .find_map(|effect| find_nested_effect::<crate::effects::LookAtTopCardsEffect>(effect))
        .expect("looked-card pool");
    let chosen = effects
        .iter()
        .find_map(|effect| find_nested_effect::<ChooseObjectsEffect>(effect))
        .expect("selection from looked-card pool");
    let moved = effects
        .iter()
        .find_map(|effect| {
            find_nested_effect::<crate::effects::ForEachTaggedEffect>(effect)
                .filter(|for_each| for_each.tag == chosen.tag)
        })
        .expect("chosen-card movement");
    let remainder = effects
        .iter()
        .find_map(|effect| {
            find_nested_effect::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(effect)
        })
        .expect("looked-card remainder movement");

    assert!(
        chosen
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == looked.tag),
        "selection must be restricted to the looked-card pool: {chosen:#?}"
    );
    assert_eq!(moved.tag, chosen.tag);
    assert_eq!(remainder.tag, looked.tag);
    assert_eq!(remainder.keep_tagged.as_ref(), Some(&chosen.tag));

    let rendered = canonical_compiled_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "nonland permanent card with mana value 3 or less from among them onto the battlefield"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("Put the rest on the bottom of your library in a random order"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("For each card chosen this way")
            && !rendered.contains("Unless it's a permanent"),
        "{rendered}"
    );
}
