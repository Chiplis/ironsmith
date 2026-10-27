// CR 123.2 sticker sheets: the session catalog of registered sheets, the
// sheets each player brings to a match, and the pregame selection of which of
// them are accessible during the game (CR 103.2d).

/// Everything a player gains access to by choosing one sticker sheet.
#[derive(Debug, Clone)]
pub(super) struct StickerSheetSpec {
    pub(super) name: String,
    name_stickers: Vec<String>,
    art_stickers: u32,
    /// `(ticket cost, power, toughness)`.
    pt_stickers: Vec<(u32, i32, i32)>,
    /// `(ticket cost, granted abilities, printed text)`. Rows the compiler
    /// cannot express are left out, so those stickers are never offered.
    ability_stickers: Vec<(u32, Vec<ironsmith::ability::Ability>, String)>,
}

/// At most this many of a player's sheets are accessible in one game.
const MAX_CHOSEN_STICKER_SHEETS: usize = 3;
const MIN_CONSTRUCTED_STICKER_SHEETS: usize = 10;

impl MatchFormatInput {
    fn uses_limited_sticker_sheets(self) -> bool {
        matches!(self, Self::ConspiracyDraft | Self::CommanderDraft)
    }
}

/// Registry name of the compiled definition holding one ability row.
fn sticker_row_definition_name(sheet: &str, index: usize) -> String {
    format!("{sheet} — Sticker {index}")
}

fn sticker_sheet_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl WasmGame {
    fn register_sticker_sheet(
        &mut self,
        name: &str,
        name_stickers: &[String],
        art_stickers: u32,
        rows: &[StickerRowSource],
        row_definitions: &[CardDefinition],
    ) -> usize {
        let mut pt_stickers = Vec::new();
        let mut ability_stickers = Vec::new();
        for row in rows {
            if let (Some(power), Some(toughness)) = (row.power, row.toughness) {
                pt_stickers.push((row.ticket_cost, power, toughness));
                continue;
            }
            let definition_name = sticker_row_definition_name(name, row.index);
            if let Some(definition) = row_definitions
                .iter()
                .find(|definition| definition.name().eq_ignore_ascii_case(&definition_name))
            {
                ability_stickers.push((
                    row.ticket_cost,
                    definition.abilities.clone(),
                    row.text.clone(),
                ));
            }
        }
        self.sticker_sheet_catalog.insert(
            sticker_sheet_key(name),
            StickerSheetSpec {
                name: name.trim().to_string(),
                name_stickers: name_stickers.to_vec(),
                art_stickers,
                pt_stickers,
                ability_stickers,
            },
        );
        1
    }

    /// Compile each ability row of a sheet source in-process. A row that
    /// does not compile simply has no sticker.
    fn compile_sticker_sheet_rows(&self, name: &str, rows: &[StickerRowSource]) -> Vec<CardDefinition> {
        rows.iter()
            .filter_map(|row| {
                let block = row.block.as_deref()?;
                self.compile_external_single_card(&sticker_row_definition_name(name, row.index), block)
                    .ok()
            })
            .collect()
    }

    fn sticker_sheet_spec(&self, name: &str) -> Option<&StickerSheetSpec> {
        self.sticker_sheet_catalog.get(&sticker_sheet_key(name))
    }

    /// CR 123.2a: each player's sheets are unique and must be real sheets.
    fn validate_sticker_sheet_pools(
        &self,
        pools: &[Vec<String>],
        player_count: usize,
        format: MatchFormatInput,
    ) -> Result<(), String> {
        if pools.len() > player_count {
            return Err(format!(
                "stickerSheets has {} entries for {player_count} players",
                pools.len()
            ));
        }
        for (seat, pool) in pools.iter().enumerate() {
            let mut seen = HashSet::new();
            for name in pool {
                if !seen.insert(sticker_sheet_key(name)) {
                    return Err(format!(
                        "player {} lists sticker sheet {name:?} more than once; each sticker sheet must be unique",
                        seat + 1
                    ));
                }
                if self.sticker_sheet_spec(name).is_none() {
                    return Err(format!(
                        "player {} lists {name:?}, which is not a known sticker sheet",
                        seat + 1
                    ));
                }
            }
            if !pool.is_empty() && !format.uses_limited_sticker_sheets()
                && pool.len() < MIN_CONSTRUCTED_STICKER_SHEETS
            {
                return Err(format!(
                    "player {} must bring at least {MIN_CONSTRUCTED_STICKER_SHEETS} unique sticker sheets in constructed play",
                    seat + 1
                ));
            }
        }
        Ok(())
    }

    /// Give `player` access to every sticker on the chosen sheets and keep
    /// the sheets revealed (CR 123.2c).
    fn grant_sticker_sheets(&mut self, player: PlayerId, sheets: &[String]) {
        let specs: Vec<StickerSheetSpec> = sheets
            .iter()
            .filter_map(|name| self.sticker_sheet_spec(name).cloned())
            .collect();
        for spec in &specs {
            for word in &spec.name_stickers {
                self.game.add_accessible_name_sticker(player, word.clone());
            }
            for _ in 0..spec.art_stickers {
                self.game.add_accessible_art_sticker(player);
            }
            for &(tickets, power, toughness) in &spec.pt_stickers {
                self.game.add_accessible_pt_sticker(player, tickets, power, toughness);
            }
            for (tickets, abilities, text) in &spec.ability_stickers {
                self.game
                    .add_accessible_ability_sticker(player, *tickets, abilities.clone(), text.clone());
            }
        }
        if !specs.is_empty() {
            self.revealed_sticker_sheets
                .push((player, specs.into_iter().map(|spec| spec.name).collect()));
        }
    }

    /// CR 103.2d/123.2a-b: constructed selects exactly three at random from
    /// each submitted pool. Limited leaves the choice (including none) to its
    /// player. Use the match RNG so peers and transcript replay agree.
    fn take_sticker_sheet_choices(&mut self, order: &[PlayerId]) -> Vec<(PlayerId, Vec<String>)> {
        let Some(pools) = self.pending_sticker_sheet_pools.take() else {
            return Vec::new();
        };
        let mut queue = Vec::new();
        for &player in order {
            let Some(pool) = pools.get(player.index()).filter(|pool| !pool.is_empty()) else {
                continue;
            };
            if self.match_format.uses_limited_sticker_sheets() {
                queue.push((player, pool.clone()));
            } else {
                let mut chosen = pool.clone();
                self.game.shuffle_slice(&mut chosen);
                chosen.truncate(MAX_CHOSEN_STICKER_SHEETS);
                self.grant_sticker_sheets(player, &chosen);
            }
        }
        queue
    }

    fn sticker_sheet_views(&self) -> Vec<PlayerStickerSheetsView> {
        self.revealed_sticker_sheets
            .iter()
            .map(|(player, sheets)| PlayerStickerSheetsView {
                player: player.0,
                sheets: sheets.clone(),
            })
            .collect()
    }

    fn sticker_sheet_choice_decision(player: PlayerId, offers: &[String]) -> DecisionContext {
        let options = offers
            .iter()
            .enumerate()
            .map(|(index, name)| ironsmith::decisions::context::SelectableOption::new(index, name.clone()))
            .collect();
        DecisionContext::SelectOptions(
            ironsmith::decisions::context::SelectOptionsContext::new(
                player,
                None,
                format!("Choose up to {MAX_CHOSEN_STICKER_SHEETS} sticker sheets to play with this game"),
                options,
                0,
                MAX_CHOSEN_STICKER_SHEETS.min(offers.len()),
            )
            .with_context_text("Only stickers on the chosen sheets are available, and the chosen sheets stay revealed."),
        )
    }

    fn apply_sticker_sheet_choice(&mut self, option_indices: &[usize]) -> Result<(), String> {
        let Some(PregameStage::StickerSheets { queue }) =
            self.pregame.as_ref().map(|pregame| &pregame.stage)
        else {
            return Err("no sticker sheet choice is pending".to_string());
        };
        let Some((player, offers)) = queue.first().cloned() else {
            return Err("no player is choosing sticker sheets".to_string());
        };
        if option_indices.len() > MAX_CHOSEN_STICKER_SHEETS {
            return Err(format!(
                "at most {MAX_CHOSEN_STICKER_SHEETS} sticker sheets can be chosen"
            ));
        }
        if option_indices.iter().any(|&index| index >= offers.len()) {
            return Err("sticker sheet choice is out of range".to_string());
        }
        if unique_indices(option_indices).len() != option_indices.len() {
            return Err("each sticker sheet can be chosen only once".to_string());
        }
        let chosen: Vec<String> = option_indices
            .iter()
            .map(|&index| offers[index].clone())
            .collect();
        self.grant_sticker_sheets(player, &chosen);
        if let Some(PregameStage::StickerSheets { queue }) =
            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
        {
            queue.remove(0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod sticker_sheet_tests {
    use super::*;

    fn row(index: usize, ticket_cost: u32, text: &str) -> StickerRowSource {
        let pt = text.split_once('/').and_then(|(power, toughness)| {
            Some((power.parse::<i32>().ok()?, toughness.parse::<i32>().ok()?))
        });
        StickerRowSource {
            index,
            ticket_cost,
            text: text.to_string(),
            power: pt.map(|(power, _)| power),
            toughness: pt.map(|(_, toughness)| toughness),
            block: pt
                .is_none()
                .then(|| format!("Type: Artifact Creature — Sticker\nPower/Toughness: 1/1\n{text}")),
        }
    }

    fn sheet_source(name: &str, words: [&str; 3]) -> ExternalCardSourceFile {
        serde_json::from_value(serde_json::json!({
            "canonicalName": name,
            "group": {
                "kind": "sticker_sheet",
                "name": name,
                "nameStickers": words,
                "artStickers": 1,
                "rows": [
                    {"index": 0, "ticketCost": 3, "text": "Flying",
                     "block": "Type: Artifact Creature — Sticker\nPower/Toughness: 1/1\nFlying"},
                    {"index": 1, "ticketCost": 2, "text": "1/4", "power": 1, "toughness": 4},
                ],
            },
        }))
        .expect("sticker sheet source")
    }

    fn game_with_sheets(count: usize) -> (WasmGame, Vec<String>) {
        let mut game = WasmGame::new();
        let names: Vec<String> = (0..count).map(|index| format!("Test Sheet {index}")).collect();
        for name in &names {
            game.register_external_card_source(sheet_source(name, ["Cool", "Fluffy", "Otter"]))
                .expect("sheet registers");
        }
        (game, names)
    }

    #[test]
    fn sheet_rows_compile_into_ability_and_pt_stickers() {
        let _guard = crate::test_id_counter_guard();
        let (game, names) = game_with_sheets(1);
        let spec = game.sticker_sheet_spec(&names[0]).expect("registered");
        assert_eq!(spec.name_stickers, vec!["Cool", "Fluffy", "Otter"]);
        assert_eq!(spec.art_stickers, 1);
        assert_eq!(spec.pt_stickers, vec![(2, 1, 4)]);
        assert_eq!(spec.ability_stickers.len(), 1);
        assert_eq!(spec.ability_stickers[0].0, 3);
        assert_eq!(spec.ability_stickers[0].2, "Flying");
        assert!(!spec.ability_stickers[0].1.is_empty());
        let _ = row(0, 1, "1/1");
    }

    #[test]
    fn pools_must_list_unique_known_sheets() {
        let _guard = crate::test_id_counter_guard();
        let (game, names) = game_with_sheets(10);
        assert!(game
            .validate_sticker_sheet_pools(&[names.clone(), Vec::new()], 2, MatchFormatInput::Normal)
            .is_ok());
        assert!(game
            .validate_sticker_sheet_pools(&[vec![names[0].clone(), names[0].to_uppercase()]], 2, MatchFormatInput::Normal)
            .unwrap_err()
            .contains("unique"));
        assert!(game
            .validate_sticker_sheet_pools(&[vec!["Not A Sheet".to_string()]], 2, MatchFormatInput::Normal)
            .unwrap_err()
            .contains("not a known sticker sheet"));
        for count in 1..10 {
            assert!(game.validate_sticker_sheet_pools(&[names[..count].to_vec()], 2, MatchFormatInput::Normal)
                .unwrap_err().contains("at least 10"));
        }
        assert!(game.validate_sticker_sheet_pools(&[Vec::new()], 2, MatchFormatInput::Normal).is_ok());
        assert!(game.validate_sticker_sheet_pools(&[names[..2].to_vec()], 2, MatchFormatInput::CommanderDraft).is_ok());
    }

    #[test]
    fn limited_players_can_choose_none_even_from_small_pools() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, names) = game_with_sheets(5);
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 7);
        game.match_format = MatchFormatInput::ConspiracyDraft;
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.pending_sticker_sheet_pools = Some(vec![names[..2].to_vec(), names.clone()]);
        let queue = game.take_sticker_sheet_choices(&[alice, bob]);
        assert_eq!(queue, vec![(alice, names[..2].to_vec()), (bob, names.clone())]);
        assert!(game.revealed_sticker_sheets.is_empty());
        assert!(game.game.available_name_stickers(alice).is_empty());
        assert!(game.game.available_name_stickers(bob).is_empty());
    }

    #[test]
    fn pregame_choice_grants_only_the_chosen_sheets() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, names) = game_with_sheets(5);
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 7);
        game.match_format = MatchFormatInput::ConspiracyDraft;
        let alice = PlayerId::from_index(0);
        game.pregame = Some(PregameState::new(&[alice], 7, MatchFormatInput::Normal));
        game.pregame.as_mut().unwrap().stage = PregameStage::StickerSheets {
            queue: vec![(alice, names.clone())],
        };
        assert!(game.apply_sticker_sheet_choice(&[0, 1, 2, 3]).is_err());
        game.apply_sticker_sheet_choice(&[4, 1]).expect("two sheets chosen");
        assert_eq!(
            game.revealed_sticker_sheets,
            vec![(alice, vec![names[4].clone(), names[1].clone()])]
        );
        assert_eq!(game.game.available_pt_stickers(alice).len(), 2);
        assert!(matches!(
            game.pregame.as_ref().unwrap().stage,
            PregameStage::StickerSheets { ref queue } if queue.is_empty()
        ));
    }

    #[test]
    fn constructed_sheet_randomness_is_seeded_per_player_and_cannot_reroll() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, names) = game_with_sheets(12);
        let order = [PlayerId::from_index(0), PlayerId::from_index(1)];
        let mut first = None;
        let mut outcomes = HashSet::new();
        for seed in [7, 7, 8, 9, 10, 11] {
            game.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, seed);
            game.pending_sticker_sheet_pools = Some(vec![names.clone(), names.clone()]);
            let before = game.game.irreversible_random_count();
            assert!(game.take_sticker_sheet_choices(&order).is_empty());
            assert_eq!(game.game.irreversible_random_count(), before + 2);
            let chosen = game.revealed_sticker_sheets.clone();
            assert_eq!(chosen.len(), 2);
            for (player, sheets) in &chosen {
                assert_eq!(sheets.len(), 3);
                assert_eq!(sheets.iter().collect::<HashSet<_>>().len(), 3);
                assert!(sheets.iter().all(|name| names.contains(name)));
                assert_eq!(game.game.available_name_stickers(*player).len(), 9);
                assert_eq!(game.game.available_art_stickers(*player).len(), 3);
                assert_eq!(game.game.available_pt_stickers(*player).len(), 3);
                assert_eq!(game.game.available_ability_stickers(*player).len(), 3);
            }
            if seed == 7 {
                if let Some(expected) = &first { assert_eq!(&chosen, expected); }
                else { first = Some(chosen.clone()); }
            }
            outcomes.insert(chosen[0].1.clone());
            assert!(game.take_sticker_sheet_choices(&order).is_empty());
            assert_eq!(game.revealed_sticker_sheets, chosen);
            assert_eq!(game.game.irreversible_random_count(), before + 2);
        }
        assert!(outcomes.len() > 1, "different match seeds must select different sheets");
    }

    #[test]
    fn limited_sheet_choices_precede_opening_hands_and_may_select_none() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, names) = game_with_sheets(2);
        game.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 7);
        game.match_format = MatchFormatInput::ConspiracyDraft;
        game.populate_explicit_libraries(&[vec!["Plains".into(); 40], vec!["Plains".into(); 40]])
            .expect("limited libraries");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.pending_sticker_sheet_pools = Some(vec![names.clone(), Vec::new()]);
        game.finish_match_setup(7).expect("limited setup");
        for player in [alice, bob] { assert!(game.game.player(player).unwrap().hand.is_empty()); }
        assert!(matches!(game.pending_decision.as_ref(), Some(DecisionContext::SelectOptions(options))
            if options.player == alice && options.min == 0 && options.max == 2));
        game.apply_sticker_sheet_choice(&[]).expect("choose no sheets");
        game.normalize_pregame_state().expect("advance to opening hands");
        assert!(game.revealed_sticker_sheets.is_empty());
        assert!(game.game.available_name_stickers(alice).is_empty());
        for player in [alice, bob] { assert_eq!(game.game.player(player).unwrap().hand.len(), 7); }
        game.normalize_pregame_state().expect("normalization is idempotent");
        for player in [alice, bob] { assert_eq!(game.game.player(player).unwrap().hand.len(), 7); }
    }

    #[test]
    fn constructed_match_randomly_selects_three_sheets_before_mulligans() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, names) = game_with_sheets(10);
        let deck = vec!["Plains".to_string(); 60];
        let config = MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 20,
            seed: 7,
            format: MatchFormatInput::Normal,
            decks: Some(vec![deck.clone(), deck]),
            sideboards: None,
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: None,
            hidden_deck_manifests: None,
            public_decklists: None,
            free_for_all: None,
            teams: None,
            starting_player: Some(0),
        };
        assert!(game
            .apply_match_setup_with_stickers(
                config.clone(),
                None,
                Some(vec![vec!["Unknown Sheet".to_string()], Vec::new()]),
            )
            .is_err());
        game.apply_match_setup_with_stickers(config, None, Some(vec![names.clone(), Vec::new()]))
            .expect("constructed match with sticker sheets");
        let alice = PlayerId::from_index(0);
        assert_eq!(game.revealed_sticker_sheets.len(), 1);
        assert_eq!(game.revealed_sticker_sheets[0].0, alice);
        let selected = &game.revealed_sticker_sheets[0].1;
        assert_eq!(selected.len(), 3);
        assert_eq!(selected.iter().collect::<HashSet<_>>().len(), 3);
        assert!(selected.iter().all(|name| names.contains(name)));
        assert_eq!(game.game.available_name_stickers(alice).len(), 9);
        match game.pending_decision.as_ref() {
            Some(DecisionContext::Priority(priority)) => {
                assert!(priority.actions.contains(&LegalAction::KeepOpeningHand));
            }
            other => panic!("expected the mulligan decision, got {other:?}"),
        }
    }
}
