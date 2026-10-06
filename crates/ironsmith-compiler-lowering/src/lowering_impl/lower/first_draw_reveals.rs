//! Finalize front-end-authored reveal links without inferring a link from names,
//! labels, source-object identity or proximity of independently parsed abilities.
use crate::ability::AbilityKind;
use crate::cards::{CardDefinition, builders::CardTextError};
use sha2::{Digest, Sha256};

pub(super) fn stamp_first_draw_pairs(definition: &mut CardDefinition) -> Result<(), CardTextError> {
    let mut producers = std::collections::BTreeMap::<u32, usize>::new();
    let mut consumers = std::collections::BTreeMap::<u32, usize>::new();
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Static(ability) => if let ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn { linked_reveal_pair: Some(pair), .. } = &ability.payload {
                *producers.entry(pair.pair).or_default() += 1;
            },
            AbilityKind::Triggered(ability) => if let ironsmith_core::TriggerKind::PlayerRevealsCard { first_draw_pair: Some(pair), .. } = &ability.trigger.kind {
                *consumers.entry(pair.pair).or_default() += 1;
            },
            _ => {}
        }
    }
    if producers.is_empty() && consumers.is_empty() { return Ok(()); }
    if producers.keys().ne(consumers.keys()) || producers.values().any(|count| *count != 1) {
        return Err(CardTextError::InvariantViolation("first-draw authored group lost its producer or linked triggers".into()));
    }
    let bytes = serde_json::to_vec(&definition.abilities).map_err(|error|
        CardTextError::InvariantViolation(format!("cannot stamp first-draw definition: {error}")))?;
    let stamp = ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into());
    for ability in &mut definition.abilities {
        match &mut ability.kind {
            AbilityKind::Static(ability) => if let ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn { linked_reveal_pair: Some(pair), .. } = &mut ability.payload {
                pair.definition = stamp;
            },
            AbilityKind::Triggered(ability) => if let ironsmith_core::TriggerKind::PlayerRevealsCard { first_draw_pair: Some(pair), .. } = &mut ability.trigger.kind {
                pair.definition = stamp;
            },
            _ => {}
        }
    }
    Ok(())
}
