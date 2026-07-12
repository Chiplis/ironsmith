use super::*;

const DETERMINISTIC_MATCH_SEED_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const DETERMINISTIC_MATCH_SEED_PRIME: u64 = 0x0000_0100_0000_01b3;

fn mix_match_seed_bytes(seed: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *seed ^= byte as u64;
        *seed = seed.wrapping_mul(DETERMINISTIC_MATCH_SEED_PRIME);
    }
    *seed ^= 0xff;
    *seed = seed.wrapping_mul(DETERMINISTIC_MATCH_SEED_PRIME);
}

fn mix_match_seed_str(seed: &mut u64, value: &str) {
    mix_match_seed_bytes(seed, value.as_bytes());
}

fn mix_match_seed_u64(seed: &mut u64, value: u64) {
    mix_match_seed_bytes(seed, &value.to_le_bytes());
}

pub(super) fn deterministic_match_seed(
    player_names: &[String],
    starting_life: i32,
    format: MatchFormatInput,
    decks: Option<&[Vec<String>]>,
    commanders: Option<&[Vec<String>]>,
    opening_hand_size: usize,
) -> u64 {
    let mut seed = DETERMINISTIC_MATCH_SEED_OFFSET;
    mix_match_seed_str(&mut seed, "ironsmith-match-seed-v1");
    mix_match_seed_str(
        &mut seed,
        match format {
            MatchFormatInput::Normal => "normal",
            MatchFormatInput::Commander => "commander",
        },
    );
    mix_match_seed_u64(&mut seed, starting_life as i64 as u64);
    mix_match_seed_u64(&mut seed, opening_hand_size as u64);
    mix_match_seed_u64(&mut seed, player_names.len() as u64);
    for name in player_names {
        mix_match_seed_str(&mut seed, name);
    }

    if let Some(decks) = decks {
        mix_match_seed_u64(&mut seed, decks.len() as u64);
        for deck in decks {
            mix_match_seed_u64(&mut seed, deck.len() as u64);
            for card_name in deck {
                mix_match_seed_str(&mut seed, card_name);
            }
        }
    }

    if let Some(commanders) = commanders {
        mix_match_seed_u64(&mut seed, commanders.len() as u64);
        for commander_list in commanders {
            mix_match_seed_u64(&mut seed, commander_list.len() as u64);
            for commander_name in commander_list {
                mix_match_seed_str(&mut seed, commander_name);
            }
        }
    }

    if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct StackObjectSnapshot {
    pub(super) id: u64,
    pub(super) inspect_object_id: Option<u64>,
    pub(super) stable_id: Option<u64>,
    pub(super) source_stable_id: Option<u64>,
    pub(super) controller: u8,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) effect_text: Option<String>,
    /// "Triggered", "Activated", or null for spells.
    pub(super) ability_kind: Option<String>,
    /// Compiled text of the specific ability effects (for inspector display).
    pub(super) ability_text: Option<String>,
    pub(super) targets: Vec<TargetChoiceView>,
}

pub(super) fn build_stack_object_snapshot(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    entry: &ironsmith::game_state::StackEntry,
) -> StackObjectSnapshot {
    let obj = game.object(entry.object_id);
    let source_obj = entry
        .source_stable_id
        .and_then(|stable_id| game.find_object_by_stable_id(stable_id))
        .and_then(|id| game.object(id));
    let id = if entry.is_ability {
        let provenance_id = entry.provenance.raw();
        if provenance_id != 0 {
            provenance_id.saturating_mul(2).saturating_add(1)
        } else {
            entry.object_id.0.saturating_mul(2).saturating_add(1)
        }
    } else {
        entry.object_id.0.saturating_mul(2)
    };
    let source_stable_id = entry.source_stable_id.map(|stable_id| stable_id.0.0);
    let inspect_object_id = if entry.is_ability {
        source_obj.or(obj).map(|object| object.id.0)
    } else {
        obj.or(source_obj).map(|object| object.id.0)
    };
    let stable_id = obj.or(source_obj).map(|o| o.stable_id.0.0);
    let name = obj
        .map(|o| o.name.to_string())
        .or_else(|| source_obj.map(|o| o.name.to_string()))
        .or_else(|| entry.source_name.clone())
        .unwrap_or_else(|| format!("Object#{}", entry.object_id.0));
    let targets = entry
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            target_choice_view(game, perspective, viewed_cards, None, index, target)
        })
        .collect();

    if entry.is_ability {
        let ability_kind = if entry.triggering_event.is_some() {
            "Triggered"
        } else {
            "Activated"
        };
        let ability_text = stack_entry_ability_text(entry, obj);
        StackObjectSnapshot {
            id,
            inspect_object_id,
            stable_id,
            source_stable_id,
            controller: entry.controller.0,
            name,
            mana_cost: None,
            effect_text: None,
            ability_kind: Some(ability_kind.to_string()),
            ability_text,
            targets,
        }
    } else {
        let effect_text = if let Some(o) = obj.or(source_obj) {
            let lines: Vec<_> = o
                .compiled_card_text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            if lines.is_empty() {
                None
            } else {
                Some(lines.join("; "))
            }
        } else {
            None
        };
        StackObjectSnapshot {
            id,
            inspect_object_id,
            stable_id,
            source_stable_id,
            controller: entry.controller.0,
            name,
            mana_cost: obj
                .or(source_obj)
                .and_then(|o| o.mana_cost.as_ref().map(|mc| mc.to_oracle())),
            effect_text,
            ability_kind: None,
            ability_text: None,
            targets,
        }
    }
}

pub(super) fn pending_stack_preview_id(index: usize) -> u64 {
    JS_SAFE_INTEGER_MAX
        .saturating_sub(100_000)
        .saturating_sub(index as u64)
}

pub(super) fn insert_pending_stack_object_snapshots(
    snapshot: &mut GameSnapshot,
    stack_objects: Vec<StackObjectSnapshot>,
) {
    if stack_objects.is_empty() {
        return;
    }

    let preview_names =
        stack_objects
            .iter()
            .map(|stack_object| match stack_object.ability_kind.as_deref() {
                Some(kind) => format!("{} ({kind})", stack_object.name),
                None => stack_object.name.clone(),
            });

    snapshot.stack_preview.splice(0..0, preview_names);
    let count = stack_objects.len();
    snapshot.stack_objects.splice(0..0, stack_objects);
    snapshot.stack_size += count;
}
