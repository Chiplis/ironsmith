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
            MatchFormatInput::FreeForAll => "free_for_all",
            MatchFormatInput::GrandMelee => "grand_melee",
            MatchFormatInput::TeamVsTeam => "team_vs_team",
            MatchFormatInput::Emperor => "emperor",
            MatchFormatInput::TwoHeadedGiant => "two_headed_giant",
            MatchFormatInput::AlternatingTeams => "alternating_teams",
            MatchFormatInput::Ante => "ante",
            MatchFormatInput::Planechase => "planechase",
            MatchFormatInput::Vanguard => "vanguard",
            MatchFormatInput::Archenemy => "archenemy",
            MatchFormatInput::SupervillainRumble => "supervillain_rumble",
            MatchFormatInput::ArchenemyCommander => "archenemy_commander",
            MatchFormatInput::ConspiracyDraft => "conspiracy_draft",
            MatchFormatInput::CommanderDraft => "commander_draft",
            MatchFormatInput::Commander => "commander",
            MatchFormatInput::Brawl => "brawl",
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
    /// Full source ability, including its activation cost or trigger condition.
    pub(super) source_ability_text: Option<String>,
    pub(super) targets: Vec<TargetChoiceView>,
}

// Resolve identity against the captured abilities first: the source may have
// transformed, lost abilities, or left play since this entry was created.
fn stack_source_ability_text(
    entry: &ironsmith::game_state::StackEntry,
    source: Option<&ironsmith::object::Object>,
) -> Option<String> {
    let abilities = entry
        .source_snapshot
        .as_ref()
        .map(|snapshot| snapshot.abilities.as_slice())
        .or_else(|| source.map(|object| object.abilities.as_slice()))?;
    let index = if let Some(identity) = entry.trigger_identity {
        abilities.iter().position(|ability| {
            matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Triggered(triggered)
                if ironsmith::triggers::compute_trigger_identity(triggered) == identity)
        })
    } else {
        entry.ability_index.filter(|&index| {
            abilities.get(index).is_some_and(|ability| {
                matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            })
        })
    }?;
    let compiled = entry
        .source_snapshot
        .as_ref()
        .map(|snapshot| snapshot.compiled_card_text.as_str())
        .or_else(|| source.map(|object| object.compiled_card_text.as_ref()))?;
    let lines: Vec<_> = compiled
        .lines()
        .filter_map(normalize_stack_display_text)
        .collect();
    // Only index card text when it maps one-to-one to executable abilities.
    if lines.len() == abilities.len() {
        return lines.get(index).cloned();
    }
    normalize_stack_display_text(&ironsmith::runtime_display::ability_surface_text(
        &abilities[index],
    ))
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
            source_ability_text: stack_source_ability_text(entry, source_obj.or(obj)),
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
            source_ability_text: None,
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

#[cfg(test)]
mod source_ability_tests {
    use super::*;
    use ironsmith::game_state::StackEntry;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith_registry_test::compile_to_runtime_definition;

    #[test]
    fn stack_source_ability_text_preserves_identity_and_captured_source() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = compile_to_runtime_definition(
        "Ability Identity Fixture",
        "Type: Artifact\n{1}: Draw a card.\n{2}: Draw a card.\nWhenever you gain life, draw a card.",
        false,
    ).expect("fixture should compile");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let captured = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let expected = ironsmith::runtime_display::compiled_text_lines(
            &game.object(source).unwrap().to_card_definition(),
        );
        assert_eq!(expected.len(), 3);
        let first = StackEntry::ability(source, alice, Vec::new())
            .with_ability_index(0)
            .with_source_snapshot(captured.clone());
        let second = StackEntry::ability(source, alice, Vec::new())
            .with_ability_index(1)
            .with_source_snapshot(captured.clone());
        // The source no longer has the abilities that were activated.
        game.object_mut(source).unwrap().abilities = Default::default();
        let first_view = build_stack_object_snapshot(&game, alice, None, &first);
        let second_view = build_stack_object_snapshot(&game, alice, None, &second);
        assert_eq!(
            first_view.source_ability_text.as_deref(),
            Some(expected[0].as_str())
        );
        assert_eq!(
            second_view.source_ability_text.as_deref(),
            Some(expected[1].as_str())
        );
        let ironsmith::ability::AbilityKind::Triggered(triggered) = &captured.abilities[2].kind
        else {
            panic!("expected third ability to be triggered");
        };
        let trigger = StackEntry::ability(source, alice, Vec::new())
            .with_trigger_identity(ironsmith::triggers::compute_trigger_identity(triggered))
            .with_source_snapshot(captured);
        let trigger_view = build_stack_object_snapshot(&game, alice, None, &trigger);
        assert_eq!(
            trigger_view.source_ability_text.as_deref(),
            Some(expected[2].as_str())
        );
        assert_eq!(
            stack_source_ability_text(&second, None).as_deref(),
            Some(expected[1].as_str())
        );
        assert_eq!(
            serde_json::to_value(&second_view).unwrap()["source_ability_text"],
            expected[1]
        );
        let unknown = StackEntry::ability(source, alice, Vec::new());
        assert!(
            build_stack_object_snapshot(&game, alice, None, &unknown)
                .source_ability_text
                .is_none()
        );
    }
}
