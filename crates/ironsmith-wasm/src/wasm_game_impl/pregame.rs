#[derive(Debug, Clone, PartialEq, Eq)]
enum CommanderPairAbility {
    Partner,
    Variant(String),
    PartnerWith(String),
    ChooseBackground,
    DoctorsCompanion,
}

#[cfg(test)]
mod free_for_all_setup_tests {
    use super::*;

    fn config(options: Option<FreeForAllOptionsInput>) -> MatchSetupInput {
        let deck = vec!["Plains".to_string(); 60];
        MatchSetupInput {
            player_names: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            starting_life: 20,
            seed: 806,
            format: MatchFormatInput::FreeForAll,
            decks: Some(vec![deck.clone(), deck.clone(), deck.clone(), deck]),
            sideboards: None,
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(0),
            hidden_deck_manifests: None,
            free_for_all: options,
            teams: None,
        }
    }

    #[test]
    fn free_for_all_setup_defaults_to_multiple_players_and_unlimited_range() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.apply_match_setup(config(None))
            .expect("default Free-for-All setup");

        assert_eq!(game.match_format, MatchFormatInput::FreeForAll);
        let state = game.game.free_for_all().expect("runtime profile");
        assert_eq!(
            state.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(state.range_of_influence(), None);
        assert_eq!(state.seats(), game.game.turn_store.turn_order);
        assert_eq!(
            game.pregame.as_ref().expect("pregame").player_order,
            game.game.turn_store.turn_order
        );
        assert!(!game.game.deploy_creatures_enabled());
        assert!(game.game.team_state().is_none());
    }

    #[test]
    fn free_for_all_setup_applies_direction_and_one_common_range() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.apply_match_setup(config(Some(FreeForAllOptionsInput {
            attack: FreeForAllAttackInput::Right,
            range_of_influence: Some(1),
            deploy_creatures: false,
        })))
        .expect("configured Free-for-All setup");

        let state = game.game.free_for_all().expect("runtime profile");
        assert_eq!(
            state.attack_option(),
            ironsmith::FreeForAllAttackOption::Right
        );
        assert_eq!(state.range_of_influence(), Some(1));
        assert_eq!(
            game.game.attack_direction(),
            Some(ironsmith::AttackDirection::Right)
        );
        let range = game
            .game
            .limited_range_of_influence()
            .expect("limited range");
        assert!(
            state
                .seats()
                .iter()
                .all(|player| range.configured_range(*player) == Some(1))
        );
    }

    #[test]
    fn free_for_all_setup_rejects_invalid_profile_input_transactionally() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Existing".to_string()], 17, 1);

        let mut wrong_format = config(Some(FreeForAllOptionsInput::default()));
        wrong_format.format = MatchFormatInput::Normal;
        assert!(wrong_format.validate_multiplayer_profile().is_err());
        assert_eq!(game.game.players.len(), 1);
        assert_eq!(game.game.players[0].name, "Existing");
        assert_eq!(game.game.players[0].life, 17);

        let mut too_few = config(None);
        too_few.player_names.truncate(2);
        too_few.decks.as_mut().unwrap().truncate(2);
        assert!(too_few.validate_multiplayer_profile().is_err());
        too_few.format = MatchFormatInput::ConspiracyDraft;
        assert!(too_few.validate_multiplayer_profile().is_err());
        assert_eq!(game.game.players.len(), 1);
        assert!(game.game.free_for_all().is_none());
    }

    #[test]
    fn grand_melee_setup_requires_ten_and_builds_numbered_runtime_lanes() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.format = MatchFormatInput::GrandMelee;
        setup.player_names = (0..10).map(|index| format!("Player {index}")).collect();
        setup.decks = Some(vec![vec!["Plains".to_string(); 60]; 10]);

        let mut too_few = setup.clone();
        too_few.player_names.pop();
        too_few.decks.as_mut().unwrap().pop();
        assert!(too_few.validate_multiplayer_profile().is_err());

        let mut game = WasmGame::new();
        game.apply_match_setup(setup)
            .expect("ten-player Grand Melee setup");
        assert_eq!(game.match_format, MatchFormatInput::GrandMelee);
        let state = game.game.grand_melee().expect("runtime profile");
        assert_eq!(state.starting_player_count(), 10);
        assert_eq!(state.marker_count(), 2);
        assert_eq!(game.game.active_players().len(), 2);
        assert_eq!(
            game.game.attack_direction(),
            Some(ironsmith::AttackDirection::Left)
        );
        assert_eq!(
            game.game.free_for_all().unwrap().range_of_influence(),
            Some(1)
        );
    }

    #[test]
    fn team_vs_team_setup_builds_fixed_team_blocks_and_individual_turns() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.format = MatchFormatInput::TeamVsTeam;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1], vec![2, 3]]);

        let mut game = WasmGame::new();
        game.apply_match_setup(setup)
            .expect("valid Team vs. Team setup");

        assert_eq!(game.match_format, MatchFormatInput::TeamVsTeam);
        let profile = game.game.team_vs_team().expect("runtime profile");
        assert_eq!(
            profile.teams(),
            &[
                vec![PlayerId::from_index(0), PlayerId::from_index(1)],
                vec![PlayerId::from_index(2), PlayerId::from_index(3)],
            ]
        );
        assert_eq!(
            profile.seats(),
            &[
                PlayerId::from_index(0),
                PlayerId::from_index(1),
                PlayerId::from_index(2),
                PlayerId::from_index(3),
            ]
        );
        assert_eq!(
            profile.starting_player(),
            profile.teams()[profile.starting_team()][0],
            "a two-player team's left midpoint is its first seat"
        );
        assert_eq!(
            game.game.turn_store.turn_order[0],
            profile.starting_player()
        );
        assert_eq!(
            game.pregame.as_ref().expect("pregame").player_order,
            game.game.turn_store.turn_order
        );
        assert!(game.game.free_for_all().is_none());
        assert!(game.game.limited_range_of_influence().is_none());
        assert!(game.game.attack_direction().is_none());
        assert!(!game.game.deploy_creatures_enabled());
        assert!(!game.game.shared_team_turns_enabled());
    }

    #[test]
    fn team_vs_team_setup_rejects_duplicate_or_missing_members_transactionally() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Existing".to_string()], 17, 1);

        let mut setup = config(None);
        setup.format = MatchFormatInput::TeamVsTeam;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1], vec![1, 2]]);
        assert!(setup.validate_multiplayer_profile().is_err());

        assert_eq!(game.game.players.len(), 1);
        assert_eq!(game.game.players[0].name, "Existing");
        assert_eq!(game.game.players[0].life, 17);
        assert!(game.game.team_vs_team().is_none());
    }

    #[test]
    fn emperor_setup_derives_roles_ranges_deploy_and_starting_order() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.player_names = (0..6).map(|index| format!("Player {index}")).collect();
        setup.decks = Some(vec![vec!["Plains".to_string(); 60]; 6]);
        setup.format = MatchFormatInput::Emperor;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1, 2], vec![3, 4, 5]]);

        let mut game = WasmGame::new();
        game.apply_match_setup(setup).expect("valid Emperor setup");

        assert_eq!(game.match_format, MatchFormatInput::Emperor);
        let profile = game.game.emperor().expect("runtime profile");
        assert_eq!(
            profile.emperors(),
            &[PlayerId::from_index(1), PlayerId::from_index(4),]
        );
        assert_eq!(profile.ranges(), &[1, 2, 1, 1, 2, 1]);
        assert_eq!(game.game.turn.active_player, profile.starting_emperor());
        assert_eq!(
            game.pregame.as_ref().expect("pregame").player_order[0],
            profile.starting_emperor()
        );
        assert!(game.game.deploy_creatures_enabled());
        assert!(!game.game.shared_team_turns_enabled());
    }

    #[test]
    fn emperor_setup_rejects_unequal_team_sizes_before_mutating_a_match() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.player_names = (0..7).map(|index| format!("Player {index}")).collect();
        setup.decks = Some(vec![vec!["Plains".to_string(); 60]; 7]);
        setup.format = MatchFormatInput::Emperor;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1, 2], vec![3, 4, 5, 6]]);
        assert!(setup.validate_multiplayer_profile().is_err());
    }

    #[test]
    fn two_headed_giant_setup_builds_shared_pools_turns_and_starting_team() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.format = MatchFormatInput::TwoHeadedGiant;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1], vec![2, 3]]);

        let mut game = WasmGame::new();
        game.apply_match_setup(setup)
            .expect("valid Two-Headed Giant setup");

        assert_eq!(game.match_format, MatchFormatInput::TwoHeadedGiant);
        let profile = game.game.two_headed_giant().expect("runtime profile");
        assert_eq!(profile.starting_life(), 30);
        assert_eq!(profile.poison_threshold(), 15);
        assert_eq!(game.game.turn.active_player, profile.starting_player());
        let pregame_order = &game.pregame.as_ref().expect("pregame").player_order;
        assert_eq!(pregame_order[0], profile.starting_player());
        assert!(
            pregame_order[0..2]
                .iter()
                .all(|player| profile.team_index(*player) == Some(profile.starting_team()))
        );
        assert!(game.game.shared_team_turns_enabled());
        assert!(game.game.players.iter().all(|player| player.life == 30));

        let mut invalid = config(None);
        invalid.format = MatchFormatInput::TwoHeadedGiant;
        invalid.free_for_all = None;
        invalid.teams = Some(vec![vec![0], vec![1, 2, 3]]);
        assert!(invalid.validate_multiplayer_profile().is_err());
    }

    #[test]
    fn alternating_teams_setup_uses_round_robin_seats_and_recommended_defaults() {
        let _id_guard = crate::test_id_counter_guard();
        let mut setup = config(None);
        setup.format = MatchFormatInput::AlternatingTeams;
        setup.free_for_all = None;
        setup.teams = Some(vec![vec![0, 1], vec![2, 3]]);

        let mut game = WasmGame::new();
        game.apply_match_setup(setup)
            .expect("valid Alternating Teams setup");

        assert_eq!(game.match_format, MatchFormatInput::AlternatingTeams);
        let profile = game.game.alternating_teams().expect("runtime profile");
        assert_eq!(
            profile.teams(),
            &[
                vec![PlayerId::from_index(0), PlayerId::from_index(1)],
                vec![PlayerId::from_index(2), PlayerId::from_index(3)]
            ]
        );
        assert_eq!(
            profile.seats(),
            &[
                PlayerId::from_index(0),
                PlayerId::from_index(2),
                PlayerId::from_index(1),
                PlayerId::from_index(3),
            ]
        );
        assert_eq!(
            profile.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(profile.range_of_influence(), Some(2));
        assert!(!profile.deploy_creatures());
        assert_eq!(
            game.pregame.as_ref().expect("pregame").player_order,
            game.game.turn_store.turn_order
        );
        assert!(!game.game.shared_team_turns_enabled());

        let mut invalid = config(Some(FreeForAllOptionsInput {
            attack: FreeForAllAttackInput::Left,
            range_of_influence: None,
            deploy_creatures: false,
        }));
        invalid.format = MatchFormatInput::AlternatingTeams;
        invalid.teams = Some(vec![vec![0], vec![1, 2, 3]]);
        assert!(invalid.validate_multiplayer_profile().is_err());

        let mut configured = config(Some(FreeForAllOptionsInput {
            attack: FreeForAllAttackInput::Left,
            range_of_influence: None,
            deploy_creatures: true,
        }));
        configured.format = MatchFormatInput::AlternatingTeams;
        configured.teams = Some(vec![vec![0, 1], vec![2, 3]]);
        let mut configured_game = WasmGame::new();
        configured_game
            .apply_match_setup(configured)
            .expect("explicit Alternating Teams options");
        let configured_profile = configured_game.game.alternating_teams().unwrap();
        assert_eq!(
            configured_profile.attack_option(),
            ironsmith::FreeForAllAttackOption::Left
        );
        assert_eq!(configured_profile.range_of_influence(), None);
        assert!(configured_profile.deploy_creatures());
    }
}

#[derive(Debug, Clone)]
struct ConstructedCopyIdentity {
    key: String,
    display_name: String,
    limit: usize,
}

impl WasmGame {
    pub(super) fn initialize_empty_match(
        &mut self,
        player_names: Vec<String>,
        starting_life: i32,
        seed: u64,
    ) {
        let player_count = player_names.len();
        self.game = GameState::new_with_runtime_id_reset(player_names, starting_life);
        // Card definitions are a session-level catalog, not match state. In the
        // lean WASM build the browser loads per-card sources on demand before
        // validation; resetting the registry here would drop those definitions
        // before startMatch/reveals use them.
        self.game.set_random_seed(seed);
        self.manabrew_game_id = format!("ironsmith-{seed:016x}");
        self.manabrew_human_players = vec![true; player_count];
        self.manabrew_next_prompt_id = 1;
        self.manabrew_open_prompt = None;
        self.match_format = MatchFormatInput::Normal;
        self.pregame = None;
        self.suspended_subgame_hosts.clear();
        self.grand_melee_host_lanes.clear();
        self.loaded_decks = Vec::new();
    }

    fn populate_demo_libraries(&mut self) -> Result<(), String> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        let mut generated_decks = Vec::with_capacity(player_ids.len());
        for _ in &player_ids {
            generated_decks.push(
                self.build_random_demo_deck_names(60, 24)
                    ?,
            );
        }
        for (&player_id, deck) in player_ids.iter().zip(&generated_decks) {
            self.populate_player_library(player_id, deck)?;
        }
        self.loaded_decks = generated_decks;
        Ok(())
    }

    fn populate_explicit_libraries(&mut self, decks: &[Vec<String>]) -> Result<(), String> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, deck) in player_ids.iter().zip(decks.iter()) {
            self.populate_player_library(player_id, deck)?;
        }
        self.loaded_decks = decks.to_vec();
        Ok(())
    }

    fn populate_libraries_with_hidden_manifests(
        &mut self,
        decks: &[Vec<String>],
        hidden_manifests: &[HiddenDeckManifestInput],
    ) -> Result<(), String> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (player_index, (&player_id, deck)) in player_ids.iter().zip(decks.iter()).enumerate() {
            let manifest = hidden_manifests
                .iter()
                .find(|manifest| usize::from(manifest.owner) == player_index);

            if let Some(manifest) = manifest
                && manifest.slot_commitments.is_empty()
                && manifest.deck_count > 0
            {
                return Err(format!(
                    "hidden deck manifest for player {} has no slot commitments",
                    player_index + 1
                ));
            }

            if !deck.is_empty() {
                self.registry
                    .ensure_cards_loaded(deck.iter().map(|name| name.as_str()));
                let mut slots = manifest
                    .map(|manifest| manifest.slot_commitments.clone())
                    .unwrap_or_default();
                slots.sort_by_key(|slot| slot.slot);
                for (slot_index, name) in deck.iter().enumerate() {
                    let Some(definition) = self.find_card_definition(name).cloned() else {
                        return Err(format!("unknown card name: {name}"));
                    };
                    let object_id = self.game.create_object_from_catalog_definition(
                        &definition,
                        &self.registry,
                        player_id,
                        ironsmith::zone::Zone::Library,
                    );
                    if let Some(slot) = slots.get(slot_index) {
                        self.game.set_hidden_card_info(
                            object_id,
                            ironsmith::game_state::HiddenCardInfo {
                                owner: player_id,
                                zone: ironsmith::zone::Zone::Library,
                                slot: slot.slot,
                                commitment: slot.commitment.clone(),
                                public_slot: None,
                                public_commitment: None,
                            },
                        );
                    }
                }
                self.game.shuffle_player_library(player_id);
                continue;
            }

            if let Some(manifest) = manifest {
                let mut slots = manifest.slot_commitments.clone();
                slots.sort_by_key(|slot| slot.slot);
                for slot in slots.into_iter().take(manifest.deck_count) {
                    self.game.create_hidden_card_placeholder(
                        player_id,
                        ironsmith::zone::Zone::Library,
                        slot.slot,
                        slot.commitment,
                    );
                }
            }
        }
        self.loaded_decks = decks.to_vec();
        Ok(())
    }

    fn populate_hidden_manifest_sideboards(
        &mut self,
        sideboards: &[Vec<String>],
        hidden_manifests: &[HiddenDeckManifestInput],
    ) {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|player| player.id).collect();
        for (player_index, &player_id) in player_ids.iter().enumerate() {
            if sideboards
                .get(player_index)
                .is_some_and(|sideboard| !sideboard.is_empty())
            {
                continue;
            }
            let Some(manifest) = hidden_manifests
                .iter()
                .find(|manifest| usize::from(manifest.owner) == player_index)
            else {
                continue;
            };
            let mut slots = manifest.slot_commitments.clone();
            slots.sort_by_key(|slot| slot.slot);
            for slot in slots
                .into_iter()
                .skip(manifest.deck_count)
                .take(manifest.sideboard_count)
            {
                self.game.create_hidden_card_placeholder(
                    player_id,
                    ironsmith::zone::Zone::OutsideGame,
                    slot.slot,
                    slot.commitment,
                );
            }
        }
    }

    fn populate_explicit_sideboards(&mut self, sideboards: &[Vec<String>]) -> Result<(), String> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, sideboard) in player_ids.iter().zip(sideboards.iter()) {
            self.registry
                .ensure_cards_loaded(sideboard.iter().map(|name| name.as_str()));

            for name in sideboard {
                let Some(definition) = self.find_card_definition(name).cloned() else {
                    return Err(format!("unknown card name: {name}"));
                };
                self.game.create_object_from_catalog_definition(
                    &definition,
                    &self.registry,
                    player_id,
                    ironsmith::zone::Zone::OutsideGame,
                );
            }
        }
        Ok(())
    }

    fn populate_explicit_commanders(&mut self, commanders: &[Vec<String>]) -> Result<(), String> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, commander_names) in player_ids.iter().zip(commanders.iter()) {
            self.registry
                .ensure_cards_loaded(commander_names.iter().map(|name| name.as_str()));

            for name in commander_names {
                let Some(definition) = self.find_card_definition(name).cloned() else {
                    return Err(format!("unknown card name: {name}"));
                };
                let object_id = self.game.create_object_from_catalog_definition(
                    &definition,
                    &self.registry,
                    player_id,
                    ironsmith::zone::Zone::Command,
                );
                self.game.set_as_commander(object_id, player_id);
            }
        }
        Ok(())
    }

    fn companion_minimum_deck_size(format: MatchFormatInput) -> usize {
        match format {
            MatchFormatInput::Brawl | MatchFormatInput::CommanderDraft => 60,
            MatchFormatInput::Commander | MatchFormatInput::ArchenemyCommander => 100,
            MatchFormatInput::ConspiracyDraft => 40,
            MatchFormatInput::Normal
            | MatchFormatInput::FreeForAll
            | MatchFormatInput::GrandMelee
            | MatchFormatInput::TeamVsTeam
            | MatchFormatInput::Emperor
            | MatchFormatInput::TwoHeadedGiant
            | MatchFormatInput::AlternatingTeams
            | MatchFormatInput::Ante
            | MatchFormatInput::Planechase
            | MatchFormatInput::Vanguard
            | MatchFormatInput::Archenemy
            | MatchFormatInput::SupervillainRumble => 60,
        }
    }

    /// Validate every companion before `apply_match_setup` replaces the live
    /// game, preserving the setup transaction on any illegal designation.
    fn validate_companion_setup(
        &mut self,
        config: &MatchSetupInput,
        selections: Option<&[Option<String>]>,
    ) -> Result<Vec<Option<CardDefinition>>, String> {
        let player_count = config.player_names.len();
        let Some(selections) = selections else {
            return Ok(vec![None; player_count]);
        };
        if selections.len() != player_count {
            return Err("companion selection count must match the players".to_string());
        }

        let decks = config.decks.as_deref().ok_or_else(|| {
            "companion designation requires explicit starting decklists".to_string()
        })?;
        if decks.len() != player_count {
            return Err("deck count must match number of players in game".to_string());
        }
        let sideboards = config.sideboards.as_deref();
        let commanders = config.commanders.as_deref();
        let hidden_manifests = config.hidden_deck_manifests.as_deref().unwrap_or(&[]);
        let mut prepared = Vec::with_capacity(player_count);

        for (player_index, selection) in selections.iter().enumerate() {
            let Some(name) = selection.as_deref().map(str::trim).filter(|name| !name.is_empty())
            else {
                prepared.push(None);
                continue;
            };
            if decks[player_index].is_empty()
                && hidden_manifests
                    .iter()
                    .any(|manifest| usize::from(manifest.owner) == player_index)
            {
                return Err(format!(
                    "player {} cannot validate a companion against an opaque starting deck",
                    player_index + 1
                ));
            }

            let ordinary_sideboard_format = !config.format.uses_commander_setup();
            if ordinary_sideboard_format
                && !sideboards
                    .and_then(|lists| lists.get(player_index))
                    .is_some_and(|sideboard| {
                        sideboard
                            .iter()
                            .any(|entry| entry.trim().eq_ignore_ascii_case(name))
                    })
            {
                return Err(format!(
                    "{} must be owned in player {}'s outside-game sideboard",
                    name,
                    player_index + 1
                ));
            }

            self.registry.ensure_cards_loaded(
                std::iter::once(name)
                    .chain(decks[player_index].iter().map(String::as_str))
                    .chain(
                        commanders
                            .and_then(|lists| lists.get(player_index))
                            .into_iter()
                            .flatten()
                            .map(String::as_str),
                    ),
            );
            let companion = self.load_compilable_card_definition_result(name)?;
            let mut starting_deck = decks[player_index]
                .iter()
                .map(|card_name| self.load_compilable_card_definition_result(card_name))
                .collect::<Result<Vec<_>, _>>()?;
            if config.format.uses_commander_setup() {
                let commander_names = commanders
                    .and_then(|lists| lists.get(player_index))
                    .ok_or_else(|| "commander companion setup needs commanders".to_string())?;
                for commander_name in commander_names {
                    starting_deck
                        .push(self.load_compilable_card_definition_result(commander_name)?);
                }

                if starting_deck.iter().any(|definition| {
                    definition
                        .card
                        .name
                        .trim()
                        .eq_ignore_ascii_case(companion.card.name.trim())
                }) {
                    return Err(format!(
                        "{} has the same name as a card in player {}'s starting deck",
                        companion.card.name,
                        player_index + 1
                    ));
                }
                let mut commander_identity = ColorSet::COLORLESS;
                for commander_name in commander_names {
                    let definition = self.load_compilable_card_definition_result(commander_name)?;
                    commander_identity = commander_identity
                        .union(self.commander_definition_color_identity(&definition)?);
                }
                self.validate_commander_card_identity(&companion, commander_identity)?;

                if config.format == MatchFormatInput::CommanderDraft
                    && !config
                        .commander_draft
                        .as_ref()
                        .and_then(|draft| draft.card_pools.get(player_index))
                        .is_some_and(|pool| {
                            pool.iter().any(|entry| entry.trim().eq_ignore_ascii_case(name))
                        })
                {
                    return Err(format!(
                        "{} is not in player {}'s completed Commander Draft pool",
                        name,
                        player_index + 1
                    ));
                }
            }

            ironsmith::validate_companion_definition(
                &companion,
                &starting_deck,
                Self::companion_minimum_deck_size(config.format),
            )
            .map_err(|error| format!("{}: {error}", companion.card.name))?;
            prepared.push(Some(companion));
        }

        Ok(prepared)
    }

    fn populate_companion_designations(
        &mut self,
        companions: &[Option<CardDefinition>],
    ) -> Result<(), String> {
        let players = self
            .game
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        for (&player, companion) in players.iter().zip(companions.iter()) {
            let Some(companion) = companion else {
                continue;
            };
            let existing = self.game.player(player).and_then(|state| {
                state.sideboard.iter().copied().find(|object_id| {
                    self.game.object(*object_id).is_some_and(|object| {
                        object
                            .name
                            .trim()
                            .eq_ignore_ascii_case(companion.card.name.trim())
                    })
                })
            });
            let companion_id = existing.unwrap_or_else(|| {
                self.game.create_object_from_catalog_definition(
                    companion,
                    &self.registry,
                    player,
                    ironsmith::zone::Zone::OutsideGame,
                )
            });
            let mut starting_deck = self
                .game
                .player(player)
                .map(|state| state.library.clone())
                .unwrap_or_default();
            if let Some(state) = self.game.player(player) {
                starting_deck.extend(state.commanders.iter().copied());
            }
            self.game
                .designate_companion(
                    player,
                    companion_id,
                    &starting_deck,
                    Self::companion_minimum_deck_size(self.match_format),
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn load_planar_decks_for_setup(
        &mut self,
        planar_decks: &[Vec<PlanarCardInput>],
        player_count: usize,
    ) -> Result<Vec<Vec<(CardDefinition, ironsmith::game_state::PlanarCardKind)>>, String> {
        if planar_decks.len() != 1 && planar_decks.len() != player_count {
            return Err(
                "Planechase requires one planar deck per player or one communal planar deck"
                    .to_string(),
            );
        }
        let communal = planar_decks.len() == 1;
        let minimum = if communal {
            40usize.min(player_count.saturating_mul(10))
        } else {
            10
        };
        let maximum_phenomena = if communal {
            player_count.saturating_mul(2)
        } else {
            2
        };

        let mut prepared = Vec::with_capacity(planar_decks.len());
        for (deck_index, deck) in planar_decks.iter().enumerate() {
            if deck.len() < minimum {
                return Err(format!(
                    "planar deck {} must contain at least {minimum} cards",
                    deck_index + 1
                ));
            }
            let mut names = HashSet::new();
            let mut cards = Vec::with_capacity(deck.len());
            let mut phenomena = 0usize;
            for card in deck {
                let normalized = card.name.trim().to_ascii_lowercase();
                if normalized.is_empty() {
                    return Err(format!(
                        "planar deck {} contains an empty card name",
                        deck_index + 1
                    ));
                }
                if !names.insert(normalized) {
                    return Err(format!(
                        "planar deck {} contains more than one card named {}",
                        deck_index + 1,
                        card.name
                    ));
                }
                let definition = self.load_compilable_card_definition_result(&card.name)?;
                let is_plane = definition.card.card_types.contains(&CardType::Plane);
                let is_phenomenon = definition.card.card_types.contains(&CardType::Phenomenon);
                let kind = match (is_plane, is_phenomenon) {
                    (true, false) => ironsmith::game_state::PlanarCardKind::Plane,
                    (false, true) => ironsmith::game_state::PlanarCardKind::Phenomenon,
                    _ => {
                        return Err(format!(
                            "{} must have exactly one of the Plane or Phenomenon card types",
                            definition.card.name
                        ));
                    }
                };
                if card.kind.is_some_and(|declared| {
                    matches!(
                        (declared, kind),
                        (
                            PlanarCardKindInput::Plane,
                            ironsmith::game_state::PlanarCardKind::Phenomenon
                        ) | (
                            PlanarCardKindInput::Phenomenon,
                            ironsmith::game_state::PlanarCardKind::Plane
                        )
                    )
                }) {
                    return Err(format!(
                        "the declared planar kind for {} does not match its card type",
                        definition.card.name
                    ));
                }
                phenomena += usize::from(kind == ironsmith::game_state::PlanarCardKind::Phenomenon);
                cards.push((definition, kind));
            }
            if phenomena > maximum_phenomena {
                return Err(format!(
                    "planar deck {} may contain no more than {maximum_phenomena} phenomenon cards",
                    deck_index + 1
                ));
            }
            prepared.push(cards);
        }
        Ok(prepared)
    }

    fn load_vanguards_for_setup(
        &mut self,
        vanguards: &[VanguardCardInput],
        player_count: usize,
    ) -> Result<Vec<CardDefinition>, String> {
        if vanguards.len() != player_count {
            return Err("Vanguard requires exactly one vanguard card per player".to_string());
        }
        let mut prepared = Vec::with_capacity(vanguards.len());
        for card in vanguards {
            if card.name.trim().is_empty() {
                return Err("Vanguard card names may not be empty".to_string());
            }
            let mut definition = self.load_compilable_card_definition_result(&card.name)?;
            if !definition.card.card_types.contains(&CardType::Vanguard) {
                return Err(format!("{} is not a Vanguard card", definition.card.name));
            }
            if !definition.card.subtypes.is_empty() {
                return Err(format!(
                    "Vanguard card {} may not have subtypes",
                    definition.card.name
                ));
            }
            7_i32
                .checked_add(card.hand_modifier)
                .ok_or_else(|| format!("{} has an invalid hand modifier", definition.card.name))?;
            20_i32
                .checked_add(card.life_modifier)
                .ok_or_else(|| format!("{} has an invalid life modifier", definition.card.name))?;
            definition.card.hand_modifier = card.hand_modifier;
            definition.card.life_modifier = card.life_modifier;
            prepared.push(definition);
        }
        Ok(prepared)
    }

    fn load_scheme_decks_for_setup(
        &mut self,
        scheme_decks: &[Vec<String>],
        player_count: usize,
        variant: ironsmith::game_state::ArchenemyVariant,
    ) -> Result<Vec<(PlayerId, Vec<CardDefinition>)>, String> {
        if scheme_decks.len() != player_count {
            return Err("scheme deck count must match number of players in game".to_string());
        }
        let expected_nonempty =
            if variant == ironsmith::game_state::ArchenemyVariant::SupervillainRumble {
                player_count
            } else {
                1
            };
        if scheme_decks.iter().filter(|deck| !deck.is_empty()).count() != expected_nonempty {
            return Err(if expected_nonempty == 1 {
                "Archenemy requires exactly one nonempty scheme deck".to_string()
            } else {
                "Supervillain Rumble requires one nonempty scheme deck per player".to_string()
            });
        }

        let mut prepared = Vec::with_capacity(expected_nonempty);
        for (index, deck) in scheme_decks.iter().enumerate() {
            if deck.is_empty() {
                continue;
            }
            let mut definitions = Vec::with_capacity(deck.len());
            for name in deck {
                if name.trim().is_empty() {
                    return Err(format!(
                        "scheme deck {} contains an empty card name",
                        index + 1
                    ));
                }
                let definition = self.load_compilable_card_definition_result(name)?;
                if !definition.card.card_types.contains(&CardType::Scheme) {
                    return Err(format!("{} is not a Scheme card", definition.card.name));
                }
                definitions.push(definition);
            }
            prepared.push((PlayerId::from_index(index as u8), definitions));
        }
        Ok(prepared)
    }

    pub(super) fn load_conspiracies_for_setup(
        &mut self,
        selections: &[Vec<ConspiracyCardInput>],
        sideboards: &[Vec<String>],
        player_count: usize,
    ) -> Result<Vec<(PlayerId, Vec<ironsmith::ConspiracySetupCard>)>, String> {
        if selections.len() != player_count || sideboards.len() != player_count {
            return Err("conspiracy selections must provide one list per player".to_string());
        }
        let mut prepared = Vec::with_capacity(player_count);
        for player_index in 0..player_count {
            let mut available = sideboards[player_index]
                .iter()
                .map(|name| name.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            let mut cards = Vec::with_capacity(selections[player_index].len());
            for input in &selections[player_index] {
                let normalized = input.name.trim().to_ascii_lowercase();
                let Some(position) = available.iter().position(|name| *name == normalized) else {
                    return Err(format!(
                        "{} is not available in player {}'s drafted sideboard",
                        input.name,
                        player_index + 1
                    ));
                };
                available.remove(position);
                let definition = self.load_compilable_card_definition_result(&input.name)?;
                if !definition.card.card_types.contains(&CardType::Conspiracy) {
                    return Err(format!("{} is not a Conspiracy card", definition.card.name));
                }
                if !definition.card.subtypes.is_empty() {
                    return Err(format!(
                        "Conspiracy card {} may not have subtypes",
                        definition.card.name
                    ));
                }
                cards.push(ironsmith::ConspiracySetupCard {
                    definition,
                    agenda_names: input.agenda_names.clone(),
                });
            }
            prepared.push((PlayerId::from_index(player_index as u8), cards));
        }
        Ok(prepared)
    }

    fn commander_definition_color_identity(
        &mut self,
        definition: &CardDefinition,
    ) -> Result<ColorSet, String> {
        let mut identity = definition.card.color_identity();
        if let Some(other_face_name) = definition.card.other_face_name.as_deref() {
            let other_face = self.load_compilable_card_definition_result(other_face_name)?;
            identity = identity.union(other_face.card.color_identity());
        }
        Ok(identity)
    }

    fn commander_basic_land_type_identity(card: &ironsmith::card::Card) -> ColorSet {
        let mut identity = ColorSet::COLORLESS;
        for (subtype, color) in [
            (Subtype::Plains, Color::White),
            (Subtype::Island, Color::Blue),
            (Subtype::Swamp, Color::Black),
            (Subtype::Mountain, Color::Red),
            (Subtype::Forest, Color::Green),
        ] {
            if card.has_subtype(subtype) {
                identity = identity.with(color);
            }
        }
        identity
    }

    fn commander_static_ability_labels(
        definition: &CardDefinition,
        id: StaticAbilityId,
    ) -> Vec<String> {
        definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(ability) if ability.id() == id => {
                    Some(ability.display())
                }
                _ => None,
            })
            .collect()
    }

    fn deck_construction_copy_limit(labels: impl IntoIterator<Item = String>) -> Option<usize> {
        fn number_word(value: &str) -> Option<usize> {
            value.parse().ok().or_else(|| {
                Some(match value {
                    "one" => 1,
                    "two" => 2,
                    "three" => 3,
                    "four" => 4,
                    "five" => 5,
                    "six" => 6,
                    "seven" => 7,
                    "eight" => 8,
                    "nine" => 9,
                    "ten" => 10,
                    _ => return None,
                })
            })
        }

        for label in labels {
            let normalized = label.trim().trim_end_matches('.').to_ascii_lowercase();
            if normalized.starts_with("a deck can have any number of cards named ") {
                return Some(usize::MAX);
            }
            if let Some(remainder) = normalized.strip_prefix("a deck can have up to ")
                && let Some(limit) = remainder.split_whitespace().next().and_then(number_word)
            {
                return Some(limit);
            }
        }
        None
    }

    fn normal_constructed_copy_identity(
        &mut self,
        definition: &CardDefinition,
    ) -> Result<ConstructedCopyIdentity, String> {
        let mut family = vec![definition.clone()];
        if let Some(other_face_name) = definition.card.other_face_name.as_deref() {
            family.push(self.load_compilable_card_definition_result(other_face_name)?);
        }

        let is_basic_land = family.iter().any(|face| {
            face.card.has_supertype(Supertype::Basic) && face.card.has_card_type(CardType::Land)
        });
        let explicit_limit = family.iter().find_map(|face| {
            Self::deck_construction_copy_limit(Self::commander_static_ability_labels(
                face,
                StaticAbilityId::DeckConstructionRuleText,
            ))
        });

        family.sort_by_key(|face| face.card.id.0);
        let display_name = family
            .first()
            .map(|face| face.card.name.clone())
            .unwrap_or_else(|| definition.card.name.clone());
        let mut names = family
            .iter()
            .map(|face| face.card.name.trim().to_ascii_lowercase())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();

        Ok(ConstructedCopyIdentity {
            key: names.join("\u{0}"),
            display_name,
            limit: if is_basic_land {
                usize::MAX
            } else {
                explicit_limit.unwrap_or(4)
            },
        })
    }

    fn validate_normal_constructed_definitions<'a>(
        &mut self,
        definitions: impl IntoIterator<Item = &'a CardDefinition>,
    ) -> Result<(), String> {
        let mut counts = HashMap::<String, (usize, usize, String)>::new();
        for definition in definitions {
            let identity = self.normal_constructed_copy_identity(definition)?;
            let entry = counts
                .entry(identity.key)
                .or_insert_with(|| (0, identity.limit, identity.display_name.clone()));
            entry.0 += 1;
            entry.1 = entry.1.max(identity.limit);
            if entry.0 > entry.1 {
                return Err(format!(
                    "normal constructed decks and sideboards may contain no more than {} cards named {}",
                    entry.1, entry.2
                ));
            }
        }
        Ok(())
    }

    fn definition_refers_to_ante(definition: &CardDefinition) -> bool {
        definition.refers_to_ante
    }

    /// CR 407.3: every card that refers to ante is illegal in a game that is
    /// not using the ante variation, including the sideboard.
    pub(super) fn validate_ante_card_legality_for_setup(
        &mut self,
        decks: Option<&[Vec<String>]>,
        sideboards: Option<&[Vec<String>]>,
        playing_for_ante: bool,
    ) -> Result<(), String> {
        if playing_for_ante {
            return Ok(());
        }
        for card_name in decks
            .into_iter()
            .flatten()
            .flatten()
            .chain(sideboards.into_iter().flatten().flatten())
        {
            let definition = self.load_compilable_card_definition_result(card_name)?;
            if Self::definition_refers_to_ante(&definition) {
                return Err(format!(
                    "{} is illegal unless the match is played for ante",
                    definition.card.name
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_ante_manifest_visibility(
        format: MatchFormatInput,
        hidden_manifests: &[HiddenDeckManifestInput],
    ) -> Result<(), String> {
        if format == MatchFormatInput::Ante && !hidden_manifests.is_empty() {
            return Err(
                "ante setup requires public decklists so each randomly selected ante card can be examined"
                    .to_string(),
            );
        }
        Ok(())
    }

    pub(super) fn validate_normal_constructed_card_names(
        &mut self,
        deck: &[String],
        sideboard: &[String],
    ) -> Result<(), String> {
        self.registry.ensure_cards_loaded(
            deck.iter()
                .chain(sideboard.iter())
                .map(|name| name.as_str()),
        );
        let mut definitions = Vec::with_capacity(deck.len() + sideboard.len());
        for card_name in deck.iter().chain(sideboard.iter()) {
            definitions.push(self.load_compilable_card_definition_result(card_name)?);
        }
        self.validate_normal_constructed_definitions(&definitions)
    }

    fn validate_normal_constructed_setup(
        &mut self,
        player_count: usize,
        decks: Option<&[Vec<String>]>,
        sideboards: Option<&[Vec<String>]>,
        hidden_manifests: &[HiddenDeckManifestInput],
    ) -> Result<(), String> {
        let Some(decks) = decks else {
            if sideboards
                .is_some_and(|sideboards| sideboards.iter().any(|sideboard| !sideboard.is_empty()))
                || !hidden_manifests.is_empty()
            {
                return Err(
                    "normal constructed sideboards and hidden manifests require explicit decklists"
                        .to_string(),
                );
            }
            return Ok(());
        };
        if decks.len() != player_count {
            return Err("deck count must match number of players in game".to_string());
        }
        if let Some(sideboards) = sideboards
            && sideboards.len() != player_count
        {
            return Err("sideboard count must match number of players in game".to_string());
        }

        let mut manifest_by_player = vec![None; player_count];
        for manifest in hidden_manifests {
            let owner = usize::from(manifest.owner);
            if owner >= player_count {
                return Err("hidden deck manifest has an invalid owner".to_string());
            }
            if manifest_by_player[owner].replace(manifest).is_some() {
                return Err("hidden deck manifests must have unique owners".to_string());
            }
        }

        for player_index in 0..player_count {
            let deck = &decks[player_index];
            let sideboard = sideboards
                .and_then(|sideboards| sideboards.get(player_index))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let manifest = manifest_by_player[player_index];
            let deck_count = if deck.is_empty() {
                manifest.map_or(0, |manifest| manifest.deck_count)
            } else {
                deck.len()
            };
            let sideboard_count = if sideboard.is_empty() {
                manifest.map_or(0, |manifest| manifest.sideboard_count)
            } else {
                sideboard.len()
            };

            if deck_count < 60 {
                return Err(format!(
                    "normal constructed player {} must have at least 60 main-deck cards",
                    player_index + 1
                ));
            }
            if sideboard_count > 15 {
                return Err(format!(
                    "normal constructed player {} may have no more than 15 sideboard cards",
                    player_index + 1
                ));
            }

            if let Some(manifest) = manifest {
                if manifest.commander_count != 0 {
                    return Err(format!(
                        "normal constructed player {} hidden manifest cannot designate commanders",
                        player_index + 1
                    ));
                }
                if (!deck.is_empty() && manifest.deck_count != deck.len())
                    || (!sideboard.is_empty() && manifest.sideboard_count != sideboard.len())
                {
                    return Err(format!(
                        "normal constructed player {} manifest counts do not match the explicit setup",
                        player_index + 1
                    ));
                }
                let expected_slots = manifest
                    .deck_count
                    .checked_add(manifest.sideboard_count)
                    .ok_or_else(|| "hidden manifest card count overflowed".to_string())?;
                if manifest.slot_commitments.len() != expected_slots {
                    return Err(format!(
                        "normal constructed player {} hidden manifest must commit every main-deck and sideboard slot",
                        player_index + 1
                    ));
                }
                let mut slots = HashSet::new();
                for slot in &manifest.slot_commitments {
                    if slot.commitment.trim().is_empty() || !slots.insert(slot.slot) {
                        return Err(format!(
                            "normal constructed player {} hidden manifest has an empty or duplicate slot commitment",
                            player_index + 1
                        ));
                    }
                }
            } else if deck.is_empty() {
                return Err(format!(
                    "normal constructed player {} needs an explicit deck or hidden manifest",
                    player_index + 1
                ));
            }

            self.validate_normal_constructed_card_names(deck, sideboard)?;
        }
        Ok(())
    }

    pub(super) fn validate_conspiracy_limited_setup(
        &mut self,
        player_count: usize,
        decks: Option<&[Vec<String>]>,
        sideboards: Option<&[Vec<String>]>,
        hidden_manifests: &[HiddenDeckManifestInput],
    ) -> Result<(), String> {
        if !hidden_manifests.is_empty() {
            return Err(
                "Conspiracy Draft setup requires explicit post-draft card pools".to_string(),
            );
        }
        let decks = decks.ok_or_else(|| {
            "Conspiracy Draft games require one explicit limited deck per player".to_string()
        })?;
        let sideboards = sideboards.ok_or_else(|| {
            "Conspiracy Draft games require one explicit drafted sideboard per player".to_string()
        })?;
        if decks.len() != player_count || sideboards.len() != player_count {
            return Err(
                "Conspiracy Draft deck and sideboard counts must match the players".to_string(),
            );
        }
        for (index, deck) in decks.iter().enumerate() {
            if deck.len() < 40 {
                return Err(format!(
                    "Conspiracy Draft player {} must have at least 40 main-deck cards",
                    index + 1
                ));
            }
            for name in deck {
                let definition = self.load_compilable_card_definition_result(name)?;
                if definition.card.card_types.contains(&CardType::Conspiracy) {
                    return Err(format!(
                        "{} cannot be included in a deck",
                        definition.card.name
                    ));
                }
            }
            for name in &sideboards[index] {
                self.load_compilable_card_definition_result(name)?;
            }
        }
        Ok(())
    }

    fn is_ordinary_commander_eligible(definition: &CardDefinition) -> bool {
        let card = &definition.card;
        if Self::commander_static_ability_labels(definition, StaticAbilityId::CanBeCommander)
            .is_empty()
        {
            card.has_supertype(Supertype::Legendary)
                && (card.has_card_type(CardType::Creature)
                    || card.has_subtype(Subtype::Vehicle)
                    || (card.has_subtype(Subtype::Spacecraft) && card.power_toughness.is_some()))
        } else {
            true
        }
    }

    fn commander_pair_abilities(definition: &CardDefinition) -> Vec<CommanderPairAbility> {
        let mut abilities = Vec::new();
        for label in Self::commander_static_ability_labels(definition, StaticAbilityId::Partner) {
            let normalized = label.trim().to_ascii_lowercase();
            if normalized == "partner" {
                abilities.push(CommanderPairAbility::Partner);
            } else if normalized == "choose a background" {
                abilities.push(CommanderPairAbility::ChooseBackground);
            } else {
                abilities.push(CommanderPairAbility::Variant(normalized));
            }
        }
        for label in Self::commander_static_ability_labels(definition, StaticAbilityId::PartnerWith)
        {
            let normalized = label.trim().to_ascii_lowercase();
            let partner_name = normalized
                .strip_prefix("partner with ")
                .unwrap_or(&normalized)
                .trim()
                .to_string();
            abilities.push(CommanderPairAbility::PartnerWith(partner_name));
        }
        if !Self::commander_static_ability_labels(definition, StaticAbilityId::DoctorsCompanion)
            .is_empty()
        {
            abilities.push(CommanderPairAbility::DoctorsCompanion);
        }
        abilities
    }

    fn is_legendary_background(definition: &CardDefinition) -> bool {
        definition.card.has_supertype(Supertype::Legendary)
            && definition.card.has_card_type(CardType::Enchantment)
            && definition.card.has_subtype(Subtype::Background)
    }

    fn is_time_lord_doctor(definition: &CardDefinition) -> bool {
        let card = &definition.card;
        // The runtime subtype catalog represents the multiword Time Lord type
        // by retaining Doctor and no unrelated creature subtype.
        card.has_supertype(Supertype::Legendary)
            && card.has_card_type(CardType::Creature)
            && card.has_subtype(Subtype::Doctor)
            && card.subtypes.len() == 1
    }

    fn is_legendary_creature(definition: &CardDefinition) -> bool {
        definition.card.has_supertype(Supertype::Legendary)
            && definition.card.has_card_type(CardType::Creature)
    }

    fn commander_pair_is_legal(first: &CardDefinition, second: &CardDefinition) -> bool {
        let first_name = first.card.name.trim().to_ascii_lowercase();
        let second_name = second.card.name.trim().to_ascii_lowercase();
        let first_abilities = Self::commander_pair_abilities(first);
        let second_abilities = Self::commander_pair_abilities(second);

        if (first_abilities.contains(&CommanderPairAbility::ChooseBackground)
            && Self::is_ordinary_commander_eligible(first)
            && Self::is_legendary_background(second))
            || (second_abilities.contains(&CommanderPairAbility::ChooseBackground)
                && Self::is_legendary_background(first)
                && Self::is_ordinary_commander_eligible(second))
            || (first_abilities.contains(&CommanderPairAbility::DoctorsCompanion)
                && Self::is_legendary_creature(first)
                && Self::is_time_lord_doctor(second))
            || (second_abilities.contains(&CommanderPairAbility::DoctorsCompanion)
                && Self::is_legendary_creature(second)
                && Self::is_time_lord_doctor(first))
        {
            return true;
        }

        if !first.card.has_supertype(Supertype::Legendary)
            || !second.card.has_supertype(Supertype::Legendary)
            || !Self::is_ordinary_commander_eligible(first)
            || !Self::is_ordinary_commander_eligible(second)
        {
            return false;
        }

        first_abilities.iter().any(|first_ability| {
            second_abilities
                .iter()
                .any(|second_ability| match (first_ability, second_ability) {
                    (CommanderPairAbility::Partner, CommanderPairAbility::Partner) => true,
                    (
                        CommanderPairAbility::Variant(first_variant),
                        CommanderPairAbility::Variant(second_variant),
                    ) => first_variant == second_variant,
                    (
                        CommanderPairAbility::PartnerWith(first_partner),
                        CommanderPairAbility::PartnerWith(second_partner),
                    ) => first_partner == &second_name && second_partner == &first_name,
                    _ => false,
                })
        })
    }

    fn validate_commander_deck_card(
        &mut self,
        definition: &CardDefinition,
        commander_identity: ColorSet,
        seen_names: &mut HashSet<String>,
    ) -> Result<(), String> {
        let card = &definition.card;
        let is_basic_land =
            card.has_supertype(Supertype::Basic) && card.has_card_type(CardType::Land);
        let normalized_name = card.name.trim().to_ascii_lowercase();
        if !seen_names.insert(normalized_name) && !is_basic_land {
            return Err(format!(
                "Commander decks may contain only one card named {}",
                card.name
            ));
        }

        self.validate_commander_card_identity(definition, commander_identity)
    }

    fn validate_commander_card_identity(
        &mut self,
        definition: &CardDefinition,
        commander_identity: ColorSet,
    ) -> Result<(), String> {
        let card = &definition.card;

        let basic_land_identity = Self::commander_basic_land_type_identity(card);
        if !commander_identity.contains_all(basic_land_identity) {
            return Err(format!(
                "{} has a basic land type outside the commander's color identity",
                card.name
            ));
        }

        let identity = self.commander_definition_color_identity(definition)?;
        if !commander_identity.contains_all(identity) {
            return Err(format!(
                "{} is outside the commander's color identity",
                card.name
            ));
        }
        Ok(())
    }

    fn commander_draft_products(
        setup: &CommanderDraftSetupInput,
    ) -> Vec<ironsmith::CommanderDraftProduct> {
        let inputs = if setup.products.is_empty() {
            vec![CommanderDraftProductInput::CommanderLegends]
        } else {
            setup.products.clone()
        };
        inputs
            .into_iter()
            .map(|product| match product {
                CommanderDraftProductInput::CommanderLegends => {
                    ironsmith::CommanderDraftProduct::CommanderLegends
                }
                CommanderDraftProductInput::CommanderMasters => {
                    ironsmith::CommanderDraftProduct::CommanderMasters
                }
                CommanderDraftProductInput::BattleForBaldursGate => {
                    ironsmith::CommanderDraftProduct::BattleForBaldursGate
                }
                CommanderDraftProductInput::Other => ironsmith::CommanderDraftProduct::Other,
            })
            .collect()
    }

    fn validate_commander_draft_setup(
        &mut self,
        player_count: usize,
        decks: &[Vec<String>],
        commanders: &[Vec<String>],
        sideboards: Option<&[Vec<String>]>,
        hidden_manifests: &[HiddenDeckManifestInput],
        setup: &CommanderDraftSetupInput,
    ) -> Result<(), String> {
        if player_count < 3 {
            return Err("Commander Draft requires at least three players".to_string());
        }
        if decks.len() != player_count
            || commanders.len() != player_count
            || setup.card_pools.len() != player_count
        {
            return Err(
                "Commander Draft deck, commander, and card-pool counts must match the players"
                    .to_string(),
            );
        }
        if sideboards.is_some_and(|sideboards| {
            sideboards.len() != player_count
                || sideboards.iter().any(|sideboard| !sideboard.is_empty())
        }) {
            return Err("Commander Draft games do not use sideboards".to_string());
        }
        if !hidden_manifests.is_empty() {
            return Err("Commander Draft setup requires explicit completed card pools".to_string());
        }

        let products = Self::commander_draft_products(setup);
        let commander_masters =
            products.contains(&ironsmith::CommanderDraftProduct::CommanderMasters);
        for player in 0..player_count {
            self.registry.ensure_cards_loaded(
                setup.card_pools[player]
                    .iter()
                    .chain(decks[player].iter())
                    .chain(commanders[player].iter())
                    .map(String::as_str),
            );
            let pool = setup.card_pools[player]
                .iter()
                .map(|name| self.load_compilable_card_definition_result(name))
                .collect::<Result<Vec<_>, _>>()?;
            let main_deck = decks[player]
                .iter()
                .map(|name| self.load_compilable_card_definition_result(name))
                .collect::<Result<Vec<_>, _>>()?;
            let commander_definitions = commanders[player]
                .iter()
                .map(|name| self.load_compilable_card_definition_result(name))
                .collect::<Result<Vec<_>, _>>()?;

            ironsmith::CommanderDraftState::validate_completed_pool_and_size(
                &products,
                &pool,
                &main_deck,
                &commander_definitions,
            )?;
            if commander_definitions.len() == 1 {
                if !Self::is_ordinary_commander_eligible(&commander_definitions[0]) {
                    return Err(format!(
                        "{} is not an eligible Commander Draft commander",
                        commander_definitions[0].card.name
                    ));
                }
            } else {
                let masters_partner = commander_masters
                    && commander_definitions.iter().all(|definition| {
                        Self::is_ordinary_commander_eligible(definition)
                            && self
                                .commander_definition_color_identity(definition)
                                .is_ok_and(|identity| identity.count() <= 1)
                    });
                if !Self::commander_pair_is_legal(
                    &commander_definitions[0],
                    &commander_definitions[1],
                ) && !masters_partner
                {
                    return Err(format!(
                        "{} and {} do not have a legal shared Commander Draft partner ability",
                        commander_definitions[0].card.name, commander_definitions[1].card.name
                    ));
                }
            }

            let mut commander_identity = ColorSet::COLORLESS;
            for definition in &commander_definitions {
                commander_identity =
                    commander_identity.union(self.commander_definition_color_identity(definition)?);
            }
            for definition in &main_deck {
                self.validate_commander_card_identity(definition, commander_identity)?;
            }
        }
        Ok(())
    }

    fn validate_commander_setup(
        &mut self,
        player_count: usize,
        decks: &[Vec<String>],
        commanders: &[Vec<String>],
        sideboards: Option<&[Vec<String>]>,
        hidden_manifests: &[HiddenDeckManifestInput],
    ) -> Result<(), String> {
        if decks.len() != player_count {
            return Err("deck count must match number of players in game".to_string());
        }
        if commanders.len() != player_count {
            return Err("commander count must match number of players in game".to_string());
        }
        if let Some(sideboards) = sideboards {
            if sideboards.len() != player_count {
                return Err("sideboard count must match number of players in game".to_string());
            }
            if sideboards.iter().any(|sideboard| !sideboard.is_empty()) {
                return Err("Commander games do not use sideboards".to_string());
            }
        }

        let mut manifest_by_player = vec![None; player_count];
        for manifest in hidden_manifests {
            let owner = usize::from(manifest.owner);
            if owner >= player_count {
                return Err("hidden deck manifest has an invalid owner".to_string());
            }
            if manifest_by_player[owner].replace(manifest).is_some() {
                return Err("hidden deck manifests must have unique owners".to_string());
            }
            if manifest.sideboard_count != 0 {
                return Err("Commander hidden manifests cannot contain a sideboard".to_string());
            }
        }

        for (player_index, (deck, commander_list)) in
            decks.iter().zip(commanders.iter()).enumerate()
        {
            if !(commander_list.len() == 1 || commander_list.len() == 2) {
                return Err(
                    "commander matches require exactly 1 or 2 commanders per player".to_string(),
                );
            }

            let expected_deck_size = if commander_list.len() == 2 { 98 } else { 99 };
            let manifest = manifest_by_player[player_index];
            if deck.is_empty() {
                let Some(manifest) = manifest else {
                    return Err(format!(
                        "Commander player {} needs an explicit deck or hidden manifest",
                        player_index + 1
                    ));
                };
                if manifest.deck_count != expected_deck_size
                    || manifest.commander_count != commander_list.len()
                {
                    return Err(format!(
                        "Commander player {} hidden setup must contain {expected_deck_size} main-deck cards and {} commander(s)",
                        player_index + 1,
                        commander_list.len()
                    ));
                }
                if manifest.slot_commitments.len() != expected_deck_size {
                    return Err(format!(
                        "Commander player {} hidden manifest must commit every main-deck slot",
                        player_index + 1
                    ));
                }
                let mut slots = HashSet::new();
                for slot in &manifest.slot_commitments {
                    if slot.commitment.trim().is_empty() || !slots.insert(slot.slot) {
                        return Err(format!(
                            "Commander player {} hidden manifest has an empty or duplicate slot commitment",
                            player_index + 1
                        ));
                    }
                }
            } else if deck.len() != expected_deck_size {
                return Err(format!(
                    "commander main decks must contain {expected_deck_size} cards for {count} commander(s)",
                    count = commander_list.len()
                ));
            } else if let Some(manifest) = manifest
                && ((manifest.deck_count != 0 && manifest.deck_count != expected_deck_size)
                    || (manifest.commander_count != 0
                        && manifest.commander_count != commander_list.len()))
            {
                    return Err(format!(
                        "Commander player {} manifest counts do not match the explicit setup",
                        player_index + 1
                    ));
            }

            self.registry
                .ensure_cards_loaded(commander_list.iter().map(String::as_str));
            let mut commander_definitions = Vec::with_capacity(commander_list.len());
            for commander_name in commander_list {
                commander_definitions
                    .push(self.load_compilable_card_definition_result(commander_name)?);
            }
            if commander_definitions.len() == 1 {
                if !Self::is_ordinary_commander_eligible(&commander_definitions[0]) {
                    return Err(format!(
                        "{} is not an eligible Commander commander",
                        commander_definitions[0].card.name
                    ));
                }
            } else if !Self::commander_pair_is_legal(
                &commander_definitions[0],
                &commander_definitions[1],
            ) {
                return Err(format!(
                    "{} and {} do not have a legal shared partner ability",
                    commander_definitions[0].card.name, commander_definitions[1].card.name
                ));
            }

            let mut commander_identity = ColorSet::COLORLESS;
            let mut seen_names = HashSet::new();
            for definition in &commander_definitions {
                let normalized_name = definition.card.name.trim().to_ascii_lowercase();
                if !seen_names.insert(normalized_name) {
                    return Err("a player's commanders must have different names".to_string());
                }
                commander_identity =
                    commander_identity.union(self.commander_definition_color_identity(definition)?);
            }

            if !deck.is_empty() {
                self.registry
                    .ensure_cards_loaded(deck.iter().map(String::as_str));
                for card_name in deck {
                    let definition = self.load_compilable_card_definition_result(card_name)?;
                    self.validate_commander_deck_card(
                        &definition,
                        commander_identity,
                        &mut seen_names,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn validate_hidden_commander_reveal(
        &mut self,
        owner: PlayerId,
        revealed_object: ObjectId,
        definition: &CardDefinition,
    ) -> Result<(), String> {
        if self.match_format != MatchFormatInput::Commander {
            return Ok(());
        }

        let mut seen_names = HashSet::new();
        if let Some(player) = self.game.player(owner) {
            for commander in &player.commanders {
                if let Some(object) = self.game.object(*commander) {
                    seen_names.insert(object.name.trim().to_ascii_lowercase());
                }
            }
        }
        for (object_id, info) in self.game.hidden_card_entries() {
            if info.owner != owner || *object_id == revealed_object {
                continue;
            }
            if let Some(object) = self.game.object(*object_id)
                && object.card.is_some()
            {
                seen_names.insert(object.name.trim().to_ascii_lowercase());
            }
        }

        let commander_identity = self.game.get_commander_color_identity(owner);
        self.validate_commander_deck_card(definition, commander_identity, &mut seen_names)
    }

    fn validate_hidden_normal_reveals(
        &mut self,
        reveals: &[(PlayerId, ObjectId, CardDefinition)],
    ) -> Result<(), String> {
        if self.match_format != MatchFormatInput::Normal || reveals.is_empty() {
            return Ok(());
        }
        if let Some(definition) = reveals
            .iter()
            .map(|(_, _, definition)| definition)
            .find(|definition| Self::definition_refers_to_ante(definition))
        {
            return Err(format!(
                "{} is illegal unless the match is played for ante",
                definition.card.name
            ));
        }

        let excluded = reveals
            .iter()
            .map(|(_, object_id, _)| *object_id)
            .collect::<HashSet<_>>();
        let owners = reveals
            .iter()
            .map(|(owner, _, _)| *owner)
            .collect::<HashSet<_>>();
        let mut known_cards = HashMap::<PlayerId, Vec<(Option<CardId>, String)>>::new();
        for (object_id, info) in self.game.hidden_card_entries() {
            if !owners.contains(&info.owner) || excluded.contains(object_id) {
                continue;
            }
            if let Some(object) = self.game.object(*object_id)
                && object.card.is_some()
            {
                known_cards
                    .entry(info.owner)
                    .or_default()
                    .push((object.card, object.name.to_string()));
            }
        }
        for owner in &owners {
            if let Some(player) = self.game.player(*owner) {
                for object_id in &player.sideboard {
                    if excluded.contains(object_id)
                        || self.game.hidden_card_info(*object_id).is_some()
                    {
                        continue;
                    }
                    if let Some(object) = self.game.object(*object_id) {
                        known_cards
                            .entry(*owner)
                            .or_default()
                            .push((object.card, object.name.to_string()));
                    }
                }
            }
        }

        let mut definitions_by_owner = HashMap::<PlayerId, Vec<CardDefinition>>::new();
        for (owner, cards) in known_cards {
            for (card_id, current_name) in cards {
                let definition = if let Some(definition) =
                    card_id.and_then(|card_id| self.registry.get_by_id(card_id).cloned())
                {
                    definition
                } else {
                    self.load_compilable_card_definition_result(&current_name)?
                };
                definitions_by_owner
                    .entry(owner)
                    .or_default()
                    .push(definition);
            }
        }
        for (owner, _, definition) in reveals {
            definitions_by_owner
                .entry(*owner)
                .or_default()
                .push(definition.clone());
        }
        for definitions in definitions_by_owner.values() {
            if self
                .validate_normal_constructed_definitions(definitions)
                .is_err()
            {
                return Err(
                    "hidden constructed reveal would violate the committed deck and sideboard copy limit"
                        .to_string(),
                );
            }
        }
        Ok(())
    }

    fn validate_hidden_normal_reveal(
        &mut self,
        owner: PlayerId,
        revealed_object: ObjectId,
        definition: &CardDefinition,
    ) -> Result<(), String> {
        self.validate_hidden_normal_reveals(&[(owner, revealed_object, definition.clone())])
    }

    fn validate_hidden_commander_position_reveals(
        &mut self,
        reveals: &[ValidatedHiddenPositionReveal],
    ) -> Result<(), String> {
        if self.match_format != MatchFormatInput::Commander {
            return Ok(());
        }

        let mut pending_nonbasic_names = HashSet::new();
        for reveal in reveals {
            self.validate_hidden_commander_reveal(
                reveal.owner,
                reveal.object_id,
                &reveal.definition,
            )?;
            let card = &reveal.definition.card;
            let is_basic_land =
                card.has_supertype(Supertype::Basic) && card.has_card_type(CardType::Land);
            if !is_basic_land
                && !pending_nonbasic_names
                    .insert((reveal.owner, card.name.trim().to_ascii_lowercase()))
            {
                return Err(format!(
                    "Commander decks may contain only one card named {}",
                    card.name
                ));
            }
        }
        Ok(())
    }

    fn validate_hidden_normal_position_reveals(
        &mut self,
        reveals: &[ValidatedHiddenPositionReveal],
    ) -> Result<(), String> {
        let reveals = reveals
            .iter()
            .map(|reveal| (reveal.owner, reveal.object_id, reveal.definition.clone()))
            .collect::<Vec<_>>();
        self.validate_hidden_normal_reveals(&reveals)
    }

    fn validate_commander_manual_zone_addition(&self, zone: Zone) -> Result<(), String> {
        if self.match_format == MatchFormatInput::Commander
            && matches!(zone, Zone::Command | Zone::OutsideGame)
        {
            return Err(
                "Commander setup and outside-game cards cannot be changed by manual card injection"
                    .to_string(),
            );
        }
        if self.match_format == MatchFormatInput::Normal
            && self.pregame.is_some()
            && matches!(zone, Zone::Library | Zone::Command | Zone::OutsideGame)
        {
            return Err(
                "normal constructed starting decks and sideboards cannot be changed by manual card injection"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn brawl_basic_land_type(card: &ironsmith::card::Card) -> Option<Subtype> {
        [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ]
        .into_iter()
        .find(|subtype| card.has_subtype(*subtype))
    }

    fn validate_brawl_setup(
        &mut self,
        decks: &[Vec<String>],
        commanders: &[Vec<String>],
    ) -> Result<(), String> {
        if decks.len() != commanders.len() {
            return Err("Brawl deck and commander list counts must match".to_string());
        }

        for (player_index, (deck, commander_list)) in
            decks.iter().zip(commanders.iter()).enumerate()
        {
            if commander_list.len() != 1 {
                return Err(format!(
                    "Brawl player {} must designate exactly one commander",
                    player_index + 1
                ));
            }
            if deck.len() != 59 {
                return Err(format!(
                    "Brawl player {} must have exactly 59 main-deck cards plus its commander",
                    player_index + 1
                ));
            }

            let commander_name = commander_list[0].trim();
            let commander = self.load_compilable_card_definition_result(commander_name)?;
            let commander_card = &commander.card;
            let eligible_kind = commander_card.has_card_type(CardType::Creature)
                || commander_card.has_card_type(CardType::Planeswalker)
                || ((commander_card.has_subtype(Subtype::Vehicle)
                    || commander_card.has_subtype(Subtype::Spacecraft))
                    && commander_card.power_toughness.is_some());
            if !commander_card.has_supertype(Supertype::Legendary) || !eligible_kind {
                return Err(format!(
                    "{commander_name} is not an eligible Brawl commander"
                ));
            }

            let commander_identity = self.commander_definition_color_identity(&commander)?;
            let mut seen_names = HashSet::from([commander.card.name.trim().to_ascii_lowercase()]);
            let mut colorless_basic_land_types = HashSet::new();
            for card_name in deck {
                let definition = self.load_compilable_card_definition_result(card_name)?;
                let card = &definition.card;
                let is_basic_land =
                    card.has_supertype(Supertype::Basic) && card.has_card_type(CardType::Land);
                let normalized_name = card.name.trim().to_ascii_lowercase();
                if !seen_names.insert(normalized_name) && !is_basic_land {
                    return Err(format!(
                        "Brawl decks may contain only one copy of {}",
                        card.name
                    ));
                }

                let card_identity = self.commander_definition_color_identity(&definition)?;
                if commander_identity.contains_all(card_identity) {
                    continue;
                }
                if commander_identity.is_empty() && is_basic_land {
                    let Some(basic_type) = Self::brawl_basic_land_type(card) else {
                        return Err(format!(
                            "{} is outside the commander's color identity",
                            card.name
                        ));
                    };
                    colorless_basic_land_types.insert(basic_type);
                    if colorless_basic_land_types.len() > 1 {
                        return Err("a colorless Brawl commander's deck may use colored basic lands of only one basic land type".to_string());
                    }
                    continue;
                }
                return Err(format!(
                    "{} is outside the Brawl commander's color identity",
                    card.name
                ));
            }
        }

        Ok(())
    }

    fn player_hand_ids(&self, player: PlayerId) -> Vec<ObjectId> {
        self.game
            .player(player)
            .map(|player| player.hand.clone())
            .unwrap_or_default()
    }

    fn build_hand_selectable_objects(
        &self,
        player: PlayerId,
    ) -> Vec<ironsmith::decisions::context::SelectableObject> {
        self.player_hand_ids(player)
            .into_iter()
            .map(|id| {
                let name = self
                    .game
                    .object(id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| format!("Card {}", id.0));
                ironsmith::decisions::context::SelectableObject::new(id, name)
            })
            .collect()
    }

    fn parsed_pregame_action_kind(
        &self,
        card_id: ObjectId,
        ability_index: usize,
    ) -> Option<ironsmith::static_abilities::PregameActionKind> {
        let ability = self.game.object(card_id)?.abilities.get(ability_index)?;
        let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        static_ability.pregame_action_kind()
    }

    fn parsed_pregame_action_effects(
        &self,
        card_id: ObjectId,
        ability_index: usize,
    ) -> Option<Vec<ironsmith::effect::Effect>> {
        let ability = self.game.object(card_id)?.abilities.get(ability_index)?;
        let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        static_ability
            .pregame_action_effects()
            .map(|effects| effects.to_vec())
    }

    fn opening_pregame_action_was_used(&self, card_id: ObjectId, ability_index: usize) -> bool {
        self.pregame.as_ref().is_some_and(|pregame| {
            pregame
                .used_opening_actions
                .contains(&(card_id, ability_index))
        })
    }

    fn is_mulligan_redraw_pregame_action(&self, card_id: ObjectId, ability_index: usize) -> bool {
        matches!(
            self.parsed_pregame_action_kind(card_id, ability_index),
            Some(ironsmith::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount)
        )
    }

    fn available_mulligan_pregame_actions(&self, player: PlayerId) -> Vec<LegalAction> {
        self.player_hand_ids(player)
            .into_iter()
            .flat_map(|card_id| {
                let ability_len = self
                    .game
                    .object(card_id)
                    .map(|object| object.abilities.len())
                    .unwrap_or(0);
                (0..ability_len).filter_map(move |ability_index| {
                    self.is_mulligan_redraw_pregame_action(card_id, ability_index)
                        .then_some(LegalAction::UsePregameAction {
                            card_id,
                            ability_index,
                        })
                })
            })
            .collect()
    }

    fn available_pregame_actions(&self, player: PlayerId) -> Vec<LegalAction> {
        let hand_ids = self.player_hand_ids(player);
        let other_cards_in_hand = hand_ids.len().saturating_sub(1);
        let mut actions = Vec::new();
        for card_id in hand_ids {
            let Some(object) = self.game.object(card_id) else {
                continue;
            };
            for (ability_index, ability) in object.abilities.iter().enumerate() {
                if self.opening_pregame_action_was_used(card_id, ability_index) {
                    continue;
                }
                let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    continue;
                };
                let Some(kind) = static_ability.pregame_action_kind() else {
                    continue;
                };
                match kind {
                    ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(spec) => {
                        if spec.require_not_starting_player && self.game.is_active_player(player) {
                            continue;
                        }
                        if other_cards_in_hand < spec.exile_cards_from_hand {
                            continue;
                        }
                    }
                    ironsmith::static_abilities::PregameActionKind::RevealFromOpeningHand(_) => {}
                    ironsmith::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount
                    | ironsmith::static_abilities::PregameActionKind::ChooseColor => continue,
                }
                actions.push(LegalAction::UsePregameAction {
                    card_id,
                    ability_index,
                });
            }
        }
        actions
    }

    fn reveal_opening_hand_card_publicly(&mut self, player: PlayerId, card_id: ObjectId) {
        let description = self
            .game
            .object(card_id)
            .map(|object| format!("Reveal {} from opening hand", object.name))
            .unwrap_or_else(|| "Reveal a card from opening hand".to_string());
        for viewer_index in 0..self.game.players.len() {
            let viewer = PlayerId::from_index(viewer_index as u8);
            let view_ctx = ViewCardsContext::new(
                viewer,
                player,
                Some(card_id),
                Zone::Hand,
                description.clone(),
            )
            .with_public(true);
            merge_active_viewed_cards(
                &self.game,
                &mut self.active_viewed_cards,
                viewer,
                &[card_id],
                &view_ctx,
            );
            merge_audit_viewed_cards(
                &self.game,
                &mut self.active_audit_viewed_cards,
                viewer,
                &[card_id],
                &view_ctx,
            );
        }
    }

    fn execute_opening_hand_reveal_action(
        &mut self,
        player: PlayerId,
        card_id: ObjectId,
        ability_index: usize,
    ) -> Result<(), JsValue> {
        if self.opening_pregame_action_was_used(card_id, ability_index) {
            return Err(JsValue::from_str(
                "that opening-hand pregame action was already used",
            ));
        }
        let effects = self
            .parsed_pregame_action_effects(card_id, ability_index)
            .filter(|effects| !effects.is_empty())
            .ok_or_else(|| {
                JsValue::from_str("opening-hand reveal action has no typed consequence")
            })?;

        let mut decision_maker = WasmReplayDecisionMaker::new(&[]);
        for effect in &effects {
            let mut ctx =
                ironsmith::effects::EffectContext::new(card_id, player, &mut decision_maker);
            ironsmith::effects::execute_effect(&mut self.game, effect, &mut ctx).map_err(
                |err| {
                    JsValue::from_str(&format!(
                        "failed to register opening-hand reveal consequence: {err}"
                    ))
                },
            )?;
        }
        let (pending_context, viewed_cards, audit_viewed_cards) = decision_maker.finish();
        if pending_context.is_some() {
            return Err(JsValue::from_str(
                "opening-hand reveal consequence unexpectedly requested a decision",
            ));
        }
        self.active_viewed_cards =
            merge_carried_active_viewed_cards(self.active_viewed_cards.take(), viewed_cards);
        self.active_audit_viewed_cards.extend(audit_viewed_cards);

        self.reveal_opening_hand_card_publicly(player, card_id);
        let Some(pregame) = self.pregame.as_mut() else {
            return Err(JsValue::from_str(
                "pregame state disappeared while registering an opening-hand reveal",
            ));
        };
        pregame
            .used_opening_actions
            .insert((card_id, ability_index));
        Ok(())
    }

    fn shuffle_hand_into_library_and_draw(&mut self, player: PlayerId, opening_hand_size: usize) {
        let hand_ids = self.player_hand_ids(player);
        for id in hand_ids {
            let _ = self.game.move_object_by_effect(id, Zone::Library);
        }
        self.game.shuffle_player_library(player);
        let _ = self.game.draw_cards(player, opening_hand_size);
    }

    fn move_cards_to_library_bottom(&mut self, ordered_cards_bottom_first: &[ObjectId]) {
        for card_id in ordered_cards_bottom_first.iter().rev().copied() {
            let Some(owner) = self.game.object(card_id).map(|object| object.owner) else {
                continue;
            };
            let Some(new_id) = self.game.move_object_by_effect(card_id, Zone::Library) else {
                continue;
            };
            self.game
                .move_library_card_to_bottom(owner, new_id, "mulligan card put on bottom");
        }
    }

    pub(super) fn initialize_subgame_pregame_if_pending(&mut self) -> bool {
        if !self.game.subgame_starting_procedure_pending() || self.pregame.is_some() {
            return false;
        }

        let turn_order = self.game.team_apnap_player_order();
        let opening_hand_sizes = turn_order
            .iter()
            .copied()
            .map(|player| (player, self.game.vanguard_starting_hand_size(player)))
            .collect::<HashMap<_, _>>();
        let child_priority = PriorityLoopState::new(self.game.players.len());
        let parent_priority = std::mem::replace(&mut self.priority_state, child_priority);
        let parent_trigger_queue = std::mem::replace(&mut self.trigger_queue, TriggerQueue::new());
        self.suspended_subgame_hosts.push((
            self.runner.take(),
            self.runner_awaiting_priority,
            parent_trigger_queue,
            parent_priority,
            std::mem::take(&mut self.grand_melee_host_lanes),
        ));
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        self.pregame = Some(PregameState::new_with_hand_sizes(
            &turn_order,
            7,
            opening_hand_sizes,
            self.match_format,
        ));
        true
    }

    pub(super) fn restore_subgame_host_if_resumed(&mut self) -> bool {
        if !self.game.take_subgame_just_resumed() {
            return false;
        }
        let Some((
            runner,
            awaiting_priority,
            trigger_queue,
            priority_state,
            grand_melee_host_lanes,
        )) = self.suspended_subgame_hosts.pop()
        else {
            return false;
        };
        self.runner = runner;
        self.runner_awaiting_priority = awaiting_priority;
        self.trigger_queue = trigger_queue;
        self.priority_state = priority_state;
        self.grand_melee_host_lanes = grand_melee_host_lanes;
        self.pregame = None;
        true
    }

    fn normalize_pregame_state(&mut self) -> Result<(), JsValue> {
        loop {
            let Some(pregame) = self.pregame.as_ref() else {
                return Ok(());
            };

            match &pregame.stage {
                PregameStage::MulliganDecision {
                    undecided_players,
                    round_mulliganers,
                } if undecided_players.is_empty() => {
                    if round_mulliganers.is_empty() {
                        let queue = pregame
                            .player_order
                            .iter()
                            .copied()
                            .filter(|player| pregame.cards_to_bottom(*player) > 0)
                            .collect();
                        if let Some(pregame) = self.pregame.as_mut() {
                            pregame.stage = PregameStage::BottomCards {
                                queue,
                                pending_order: None,
                            };
                        }
                        continue;
                    }

                    let mulliganers = round_mulliganers.clone();
                    if let Some(pregame) = self.pregame.as_mut() {
                        for player in &mulliganers {
                            *pregame.mulligans_taken.entry(*player).or_insert(0) += 1;
                        }
                    }
                    for player in mulliganers.iter().copied() {
                        let opening_hand_size = self
                            .pregame
                            .as_ref()
                            .map(|pregame| pregame.opening_hand_size_for(player))
                            .unwrap_or(7);
                        self.shuffle_hand_into_library_and_draw(player, opening_hand_size);
                    }
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::MulliganDecision {
                            undecided_players: mulliganers,
                            round_mulliganers: Vec::new(),
                        };
                    }
                    continue;
                }
                PregameStage::BottomCards {
                    queue,
                    pending_order,
                } if queue.is_empty() && pending_order.is_none() => {
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::OpeningActions {
                            current_index: 0,
                            pending_hand_exile: None,
                        };
                    }
                    continue;
                }
                PregameStage::OpeningActions {
                    current_index,
                    pending_hand_exile,
                } if pending_hand_exile.is_none()
                    && *current_index >= pregame.player_order.len() =>
                {
                    if self.game.planechase.is_some()
                        && self.game.face_up_planar_objects().is_empty()
                    {
                        if self.match_format == MatchFormatInput::GrandMelee {
                            self.game
                                .reveal_grand_melee_starting_planes()
                                .map_err(|error| JsValue::from_str(&error))?;
                        } else {
                            self.game
                                .reveal_starting_plane()
                                .map_err(|error| JsValue::from_str(&error))?;
                        }
                    }
                    if self.game.is_subgame() {
                        self.game.complete_subgame_starting_procedure();
                    }
                    self.pregame = None;
                    continue;
                }
                _ => return Ok(()),
            }
        }
    }

    fn build_pregame_decision(&self) -> Result<Option<DecisionContext>, JsValue> {
        let Some(pregame) = self.pregame.as_ref() else {
            return Ok(None);
        };

        let ctx = match &pregame.stage {
            PregameStage::MulliganDecision {
                undecided_players, ..
            } => {
                let Some(player) = undecided_players.first().copied() else {
                    return Ok(None);
                };
                let mut actions = vec![LegalAction::KeepOpeningHand];
                if pregame.can_take_mulligan(player) {
                    actions.push(LegalAction::TakeMulligan);
                }
                actions.extend(self.available_mulligan_pregame_actions(player));
                DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
                    player, actions,
                ))
            }
            PregameStage::BottomCards {
                queue,
                pending_order,
            } => {
                if let Some((player, selected_cards)) = pending_order {
                    let items = selected_cards
                        .iter()
                        .filter_map(|id| {
                            self.game
                                .object(*id)
                                .map(|object| (*id, object.name.to_string()))
                        })
                        .collect();
                    DecisionContext::Order(ironsmith::decisions::context::OrderContext::new(
                        *player,
                        None,
                        "Order the selected cards for the bottom of your library. The first option becomes the bottom-most card.",
                        items,
                    ))
                } else {
                    let Some(player) = queue.first().copied() else {
                        return Ok(None);
                    };
                    let amount = pregame.cards_to_bottom(player);
                    DecisionContext::SelectObjects(
                        ironsmith::decisions::context::SelectObjectsContext::new(
                            player,
                            None,
                            format!("Choose {amount} card(s) to put on the bottom of your library"),
                            self.build_hand_selectable_objects(player),
                            amount,
                            Some(amount),
                        ),
                    )
                }
            }
            PregameStage::OpeningActions {
                current_index,
                pending_hand_exile,
            } => {
                let Some(player) = pregame.player_order.get(*current_index).copied() else {
                    return Ok(None);
                };
                if let Some(pending_exile) = pending_hand_exile {
                    if pending_exile.player != player {
                        return Err(JsValue::from_str(
                            "pregame hand exile prompt is out of sync with turn order",
                        ));
                    }
                    let source_name = self
                        .game
                        .object(pending_exile.source)
                        .map(|object| object.name.as_str())
                        .unwrap_or("this card");
                    DecisionContext::SelectObjects(
                        ironsmith::decisions::context::SelectObjectsContext::new(
                            player,
                            Some(pending_exile.source),
                            format!(
                                "Choose {} card(s) from your hand to exile for {}",
                                pending_exile.amount, source_name
                            ),
                            self.build_hand_selectable_objects(player),
                            pending_exile.amount,
                            Some(pending_exile.amount),
                        )
                        .with_reveal_policy(
                            ironsmith::decisions::context::SelectionRevealPolicy::Public,
                        ),
                    )
                } else {
                    let is_last_player = *current_index + 1 >= pregame.player_order.len();
                    let mut actions = vec![if is_last_player {
                        LegalAction::BeginGame
                    } else {
                        LegalAction::ContinuePregame
                    }];
                    actions.extend(self.available_pregame_actions(player));
                    DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
                        player, actions,
                    ))
                }
            }
        };

        Ok(Some(ctx))
    }

    fn apply_pregame_priority_action(&mut self, action: LegalAction) -> Result<(), JsValue> {
        match action {
            LegalAction::KeepOpeningHand => {
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players, ..
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "keep hand is only legal during mulligan decisions",
                    ));
                };
                if undecided_players.is_empty() {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                }
                undecided_players.remove(0);
            }
            LegalAction::TakeMulligan => {
                let Some((player, can_take_mulligan)) = self.pregame.as_ref().and_then(|pregame| {
                    let PregameStage::MulliganDecision {
                        undecided_players, ..
                    } = &pregame.stage
                    else {
                        return None;
                    };
                    let player = undecided_players.first().copied()?;
                    Some((player, pregame.can_take_mulligan(player)))
                }) else {
                    return Err(JsValue::from_str(
                        "mulligan is only legal during mulligan decisions",
                    ));
                };
                if !can_take_mulligan {
                    return Err(JsValue::from_str(
                        "a player whose opening hand is zero cards cannot take another mulligan",
                    ));
                }
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players,
                            round_mulliganers,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "mulligan is only legal during mulligan decisions",
                    ));
                };
                let Some(waiting_player) = undecided_players.first().copied() else {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                };
                debug_assert_eq!(waiting_player, player);
                undecided_players.remove(0);
                round_mulliganers.push(player);
            }
            LegalAction::ContinuePregame | LegalAction::BeginGame => {
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            current_index,
                            pending_hand_exile,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "continue is only legal during pregame opening actions",
                    ));
                };
                if pending_hand_exile.is_some() {
                    return Err(JsValue::from_str(
                        "a pregame action requires exiling cards before continuing",
                    ));
                }
                *current_index += 1;
                self.active_viewed_cards = None;
            }
            LegalAction::UsePregameAction {
                card_id,
                ability_index,
            } => {
                if self.is_mulligan_redraw_pregame_action(card_id, ability_index) {
                    let player = match self.pregame.as_ref() {
                        Some(PregameState {
                            stage:
                                PregameStage::MulliganDecision {
                                    undecided_players, ..
                                },
                            ..
                        }) => undecided_players.first().copied(),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        JsValue::from_str(
                            "mulligan redraw pregame actions can only be used while mulliganing",
                        )
                    })?;
                    let hand_ids = self.player_hand_ids(player);
                    if !hand_ids.contains(&card_id) {
                        return Err(JsValue::from_str(
                            "mulligan redraw source must be in the current player's hand",
                        ));
                    }
                    let draw_count = hand_ids.len();
                    for id in hand_ids {
                        let _ = self.game.move_object_by_effect(id, Zone::Exile);
                    }
                    let _ = self.game.draw_cards(player, draw_count);
                    return Ok(());
                }

                let player = match self.pregame.as_ref() {
                    Some(PregameState {
                        player_order,
                        stage:
                            PregameStage::OpeningActions {
                                current_index,
                                pending_hand_exile: None,
                            },
                        ..
                    }) => player_order.get(*current_index).copied(),
                    _ => None,
                }
                .ok_or_else(|| {
                    JsValue::from_str(
                        "pregame actions can only be used during pregame opening actions",
                    )
                })?;
                let hand_ids = self.player_hand_ids(player);
                if !hand_ids.contains(&card_id) {
                    return Err(JsValue::from_str(
                        "pregame action source must be in the current player's hand",
                    ));
                }
                if self.opening_pregame_action_was_used(card_id, ability_index) {
                    return Err(JsValue::from_str(
                        "that opening-hand pregame action was already used",
                    ));
                }
                let Some(kind) = self.parsed_pregame_action_kind(card_id, ability_index) else {
                    return Err(JsValue::from_str(
                        "selected ability is not a supported pregame action",
                    ));
                };
                let spec = match kind {
                    ironsmith::static_abilities::PregameActionKind::RevealFromOpeningHand(_) => {
                        self.execute_opening_hand_reveal_action(
                            player,
                            card_id,
                            ability_index,
                        )?;
                        return Ok(());
                    }
                    ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(spec) => {
                        spec
                    }
                    ironsmith::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount
                    | ironsmith::static_abilities::PregameActionKind::ChooseColor => {
                        return Err(JsValue::from_str(
                            "selected ability is not a supported opening pregame action",
                        ));
                    }
                };
                if spec.require_not_starting_player && self.game.is_active_player(player) {
                    return Err(JsValue::from_str(
                        "the starting player can't use that pregame action",
                    ));
                }
                if hand_ids.len().saturating_sub(1) < spec.exile_cards_from_hand {
                    return Err(JsValue::from_str(
                        "that pregame action requires more cards in hand to exile",
                    ));
                }
                let exile_cards_from_hand = spec.exile_cards_from_hand;
                let Some(new_id) = self.game.move_object_by_effect(card_id, Zone::Battlefield)
                else {
                    return Err(JsValue::from_str(
                        "failed to move the pregame card to the battlefield",
                    ));
                };
                for (counter_type, count) in spec.counters.iter().cloned() {
                    let _ = self.game.add_counters(new_id, counter_type, count);
                }
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        },
                    used_opening_actions,
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "pregame opening actions disappeared while resolving a pregame action",
                    ));
                };
                *pending_hand_exile =
                    (exile_cards_from_hand > 0).then_some(PendingPregameHandExile {
                        player,
                        source: new_id,
                        amount: exile_cards_from_hand,
                    });
                used_opening_actions.insert((card_id, ability_index));
            }
            other => {
                return Err(JsValue::from_str(&format!(
                    "illegal pregame priority action: {other:?}"
                )));
            }
        }

        Ok(())
    }

    fn dispatch_pregame_decision(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let restore =
            |this: &mut Self, ctx: DecisionContext, err: JsValue| -> Result<JsValue, JsValue> {
                this.pending_decision = Some(ctx);
                Err(err)
            };

        match (&pending_ctx, command) {
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                        if let Some(action_ref) = action_ref.as_ref() {
                            JsValue::from_str(&format!(
                                "invalid priority action ref: {action_ref:?}"
                            ))
                        } else if let Some(action_index) = action_index {
                            JsValue::from_str(&format!(
                                "invalid priority action index: {action_index}"
                            ))
                        } else {
                            JsValue::from_str("missing priority action selector")
                        }
                    });
                let action = match action {
                    Ok(action) => action,
                    Err(err) => return restore(self, pending_ctx, err),
                };
                if let Err(err) = self.apply_pregame_priority_action(action) {
                    return restore(self, pending_ctx, err);
                }
            }
            (
                DecisionContext::SelectObjects(objects),
                UiCommand::SelectObjects {
                    object_ids,
                    object_stable_ids,
                    object_hidden_refs,
                },
            ) => {
                let object_ids = match normalize_select_object_choice_ids(
                    &self.game,
                    objects,
                    &object_ids,
                    &object_stable_ids,
                    &object_hidden_refs,
                ) {
                    Ok(object_ids) => object_ids,
                    Err(err) => return restore(self, pending_ctx, err),
                };
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id.0)
                    .collect();
                if let Err(err) = validate_object_selection(
                    objects.min,
                    objects.max,
                    objects.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                ) {
                    return restore(self, pending_ctx, err);
                }
                let selected: Vec<ObjectId> =
                    object_ids.into_iter().map(ObjectId::from_raw).collect();
                enum PregameSelectResolution {
                    BottomNow,
                    BottomNeedsOrdering(PlayerId),
                    HandExile(Vec<ObjectId>),
                }

                let resolution = match self.pregame.as_ref().map(|pregame| &pregame.stage) {
                    Some(PregameStage::BottomCards {
                        queue,
                        pending_order,
                    }) if pending_order.is_none() => {
                        let Some(player) = queue.first().copied() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no player is waiting to bottom cards"),
                            );
                        };
                        if selected.len() <= 1 {
                            PregameSelectResolution::BottomNow
                        } else {
                            PregameSelectResolution::BottomNeedsOrdering(player)
                        }
                    }
                    Some(PregameStage::OpeningActions {
                        pending_hand_exile, ..
                    }) if pending_hand_exile.is_some() => {
                        if selected.is_empty() {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("expected at least one card to exile"),
                            );
                        }
                        PregameSelectResolution::HandExile(selected.clone())
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected select_objects command during pregame"),
                        );
                    }
                };

                match resolution {
                    PregameSelectResolution::BottomNow => {
                        self.move_cards_to_library_bottom(&selected);
                        let Some(PregameStage::BottomCards { queue, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        if !queue.is_empty() {
                            queue.remove(0);
                        }
                    }
                    PregameSelectResolution::BottomNeedsOrdering(player) => {
                        let Some(PregameStage::BottomCards { pending_order, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        *pending_order = Some((player, selected));
                    }
                    PregameSelectResolution::HandExile(card_ids) => {
                        for card_id in card_ids {
                            let _ = self.game.move_object_by_effect(card_id, Zone::Exile);
                        }
                        let Some(PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        }) = self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame hand exile state disappeared"),
                            );
                        };
                        *pending_hand_exile = None;
                    }
                }
            }
            (DecisionContext::Order(order), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = (0..order.items.len()).collect();
                if let Err(err) = validate_option_selection(
                    order.items.len(),
                    Some(order.items.len()),
                    &option_indices,
                    &legal,
                ) {
                    return restore(self, pending_ctx, err);
                }
                if unique_indices(&option_indices).len() != order.items.len() {
                    return restore(
                        self,
                        pending_ctx,
                        JsValue::from_str("ordering requires each option index exactly once"),
                    );
                }
                let selected_cards = match self.pregame.as_mut().map(|pregame| &mut pregame.stage) {
                    Some(PregameStage::BottomCards { pending_order, .. }) => {
                        let Some((_, selected_cards)) = pending_order.take() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no selected cards are waiting to be ordered"),
                            );
                        };
                        selected_cards
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected ordering command during pregame"),
                        );
                    }
                };
                let ordered_cards: Vec<ObjectId> = option_indices
                    .into_iter()
                    .filter_map(|index| selected_cards.get(index).copied())
                    .collect();
                self.move_cards_to_library_bottom(&ordered_cards);
                if let Some(PregameStage::BottomCards { queue, .. }) =
                    self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                    && !queue.is_empty()
                {
                    queue.remove(0);
                }
            }
            _ => {
                return restore(
                    self,
                    pending_ctx,
                    JsValue::from_str("command type does not match pregame decision"),
                );
            }
        }

        self.pending_decision = None;
        self.manabrew_open_prompt = None;
        self.advance_until_decision()?;
        self.snapshot()
    }

    fn finish_match_setup(&mut self, opening_hand_size: usize) -> Result<(), String> {
        self.reset_runtime_state();
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        let opening_hand_sizes = player_ids
            .iter()
            .copied()
            .map(|player| {
                let size = if self.match_format == MatchFormatInput::Vanguard {
                    self.game.vanguard_starting_hand_size(player)
                } else {
                    opening_hand_size
                };
                (player, size)
            })
            .collect::<HashMap<_, _>>();
        for player_id in &player_ids {
            let size = opening_hand_sizes
                .get(player_id)
                .copied()
                .unwrap_or(opening_hand_size);
            let _ = self.game.draw_cards(*player_id, size);
        }
        let pregame_order = self.game.team_apnap_player_order();
        self.pregame = Some(PregameState::new_with_hand_sizes(
            &pregame_order,
            opening_hand_size,
            opening_hand_sizes,
            self.match_format,
        ));
        // recompute_ui_decision reports JsValue errors for the UI layer; on
        // the native path an error JsValue cannot exist (its construction
        // panics), so this conversion only runs in wasm builds.
        self.recompute_ui_decision()
            .map_err(|error| format!("{error:?}"))
    }

    fn reset_runtime_state(&mut self) {
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(self.game.players.len());
        self.pregame = None;
        self.suspended_subgame_hosts.clear();
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.active_audit_viewed_cards.clear();
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = None;
        self.clear_active_resolving_stack_object();
        self.game_over = None;
        self.last_snapshot_perf = None;
        self.last_replay_execution_perf = None;
        self.last_advance_until_decision_perf = None;
        self.last_dispatch_perf = None;
        self.runner = None;
        self.grand_melee_host_lanes.clear();
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        if self.game.player(self.perspective).is_none()
            && let Some(first) = self.game.players.first()
        {
            self.perspective = first.id;
        }
    }
}

#[cfg(test)]
mod subgame_host_tests {
    use super::*;

    #[test]
    fn subgame_pregame_suspends_and_restores_host_runtime_state() {
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 7);
        let alice = PlayerId::from_index(0);
        game.runner_awaiting_priority = true;
        game.grand_melee_host_lanes.insert(
            7,
            GrandMeleeHostLane {
                runner: None,
                runner_awaiting_priority: false,
                trigger_queue: TriggerQueue::new(),
                priority_state: PriorityLoopState::new(2),
            },
        );

        game.game
            .begin_subgame(None, alice, Vec::new())
            .expect("begin child game");
        assert!(game.initialize_subgame_pregame_if_pending());
        assert!(game.pregame.is_some());
        assert_eq!(game.suspended_subgame_hosts.len(), 1);
        assert!(game.grand_melee_host_lanes.is_empty());
        assert!(!game.runner_awaiting_priority);

        game.game.complete_subgame_starting_procedure();
        game.game
            .finish_subgame_with(
                GameResult::Winner(alice),
                &mut ironsmith::decision::AutoPassDecisionMaker,
            )
            .expect("restore parent game");
        assert!(game.restore_subgame_host_if_resumed());
        assert!(game.pregame.is_none());
        assert!(game.suspended_subgame_hosts.is_empty());
        assert!(game.runner_awaiting_priority);
        assert!(game.grand_melee_host_lanes.contains_key(&7));
    }

    #[test]
    fn shared_team_pregame_groups_the_starting_team_before_other_teams() {
        let mut game = WasmGame::new();
        game.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            7,
        );
        let [alice, bob, charlie, diana] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ];
        game.game
            .set_teams(vec![vec![alice, bob], vec![charlie, diana]])
            .expect("valid teams");
        game.game
            .enable_shared_team_turns()
            .expect("adjacent teams share turns");
        game.game
            .set_shared_team_member_order(0, vec![bob, alice])
            .expect("starting team order selected");

        game.finish_match_setup(7).expect("pregame starts");
        let pregame = game.pregame.as_ref().expect("pregame state");
        assert_eq!(game.game.turn.active_player, bob);
        assert_eq!(pregame.player_order, vec![bob, alice, charlie, diana]);
        let PregameStage::MulliganDecision {
            undecided_players, ..
        } = &pregame.stage
        else {
            panic!("expected the mulligan declaration round");
        };
        assert_eq!(undecided_players, &pregame.player_order);
    }
}

#[cfg(test)]
mod mulligan_policy_tests {
    use super::*;

    fn game_at_counted_mulligans(count: u32) -> (WasmGame, PlayerId) {
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 7);
        let alice = PlayerId::from_index(0);
        let mut pregame = PregameState::new(
            &game.game.turn_store.turn_order,
            7,
            MatchFormatInput::Normal,
        );
        pregame.mulligans_taken.insert(alice, count);
        game.pregame = Some(pregame);
        (game, alice)
    }

    #[test]
    fn zero_card_opening_hand_keeps_only_the_keep_action() {
        let (game, alice) = game_at_counted_mulligans(7);

        let decision = game
            .build_pregame_decision()
            .expect("building the final mulligan decision should succeed")
            .expect("the player must still receive a keep decision");
        let DecisionContext::Priority(context) = decision else {
            panic!("expected a priority decision");
        };
        assert_eq!(context.player, alice);
        assert!(context.actions.contains(&LegalAction::KeepOpeningHand));
        assert!(!context.actions.contains(&LegalAction::TakeMulligan));
    }

    #[test]
    fn last_legal_mulligan_that_produces_zero_cards_is_still_offered() {
        let (game, alice) = game_at_counted_mulligans(6);

        let decision = game
            .build_pregame_decision()
            .expect("building the last legal mulligan decision should succeed")
            .expect("the player must receive a decision");
        let DecisionContext::Priority(context) = decision else {
            panic!("expected a priority decision");
        };
        assert_eq!(context.player, alice);
        assert!(context.actions.contains(&LegalAction::TakeMulligan));
    }
}

#[cfg(test)]
mod normal_constructed_setup_tests {
    use super::*;

    fn repeated(name: &str, count: usize) -> Vec<String> {
        vec![name.to_string(); count]
    }

    fn two_player_lists(list: Vec<String>) -> Vec<Vec<String>> {
        vec![list.clone(), list]
    }

    fn normal_config(deck: Vec<String>, sideboard: Vec<String>) -> MatchSetupInput {
        MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 20,
            seed: 1001,
            format: MatchFormatInput::Normal,
            decks: Some(two_player_lists(deck)),
            sideboards: Some(two_player_lists(sideboard)),
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(0),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    fn hidden_manifests(deck_count: usize, sideboard_count: usize) -> Vec<HiddenDeckManifestInput> {
        (0..2)
            .map(|owner| HiddenDeckManifestInput {
                owner,
                deck_count,
                sideboard_count,
                commander_count: 0,
                decklist_hash: format!("normal-deck-{owner}"),
                commitment_root: format!("normal-root-{owner}"),
                slot_commitments: (0..deck_count + sideboard_count)
                    .map(|slot| HiddenDeckSlotInput {
                        slot: slot as u16,
                        commitment: format!("normal-{owner}-slot-{slot}"),
                    })
                    .collect(),
            })
            .collect()
    }

    fn hidden_normal_config(deck_count: usize, sideboard_count: usize) -> MatchSetupInput {
        MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 20,
            seed: 1002,
            format: MatchFormatInput::Normal,
            decks: Some(two_player_lists(Vec::new())),
            sideboards: Some(two_player_lists(Vec::new())),
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(0),
            hidden_deck_manifests: Some(hidden_manifests(deck_count, sideboard_count)),
            free_for_all: None,
            teams: None,
        }
    }

    fn validate_normal_config(game: &mut WasmGame, config: &MatchSetupInput) -> Result<(), String> {
        game.validate_normal_constructed_setup(
            config.player_names.len(),
            config.decks.as_deref(),
            config.sideboards.as_deref(),
            config.hidden_deck_manifests.as_deref().unwrap_or(&[]),
        )
    }

    #[test]
    fn normal_constructed_public_setup_enforces_minimum_main_deck_size() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();

        assert!(
            validate_normal_config(
                &mut game,
                &normal_config(repeated("Plains", 59), Vec::new()),
            )
            .unwrap_err()
            .contains("at least 60"),
            "59-card constructed decks must be rejected before setup"
        );
        game.apply_match_setup(normal_config(repeated("Plains", 60), Vec::new()))
            .expect("60-card constructed decks should be accepted");
        game.apply_match_setup(normal_config(repeated("Plains", 61), Vec::new()))
            .expect("normal constructed decks have no maximum size");
    }

    #[test]
    fn normal_constructed_no_deck_payload_cannot_create_an_empty_match() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let config = MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 20,
            seed: 1003,
            format: MatchFormatInput::Normal,
            decks: None,
            sideboards: None,
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(7),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        };

        validate_normal_config(&mut game, &config)
            .expect("the no-deck payload is reserved for demo generation");
        match game.build_random_demo_deck_names(60, 24) {
            Ok(deck) => {
                assert_eq!(deck.len(), 60);
                game.validate_normal_constructed_card_names(&deck, &[])
                    .expect("generated demo cards must satisfy constructed copy limits");
                game.apply_match_setup(config)
                    .expect("an available legal demo deck should start");
                assert_eq!(game.loaded_decks.len(), 2);
                assert!(game.loaded_decks.iter().all(|deck| deck.len() == 60));
                assert!(game.game.players.iter().all(|player| {
                    player.library.len() + player.hand.len() == 60 && player.hand.len() == 7
                }));
            }
            Err(error) => {
                assert!(
                    error.contains("eligible")
                        || error.contains("insufficient copy-limit capacity")
                );
                assert!(game.loaded_decks.is_empty());
                assert!(
                    game.game
                        .players
                        .iter()
                        .all(|player| player.library.is_empty() && player.hand.is_empty())
                );
            }
        }
    }

    #[test]
    fn normal_constructed_demo_generation_rejects_reduced_pool_copy_limit_bypass() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.set_semantic_threshold(100.0);
        for card_name in [
            "Lightning Bolt",
            "Counterspell",
            "Giant Growth",
            "Opt",
            "Divination",
            "Llanowar Elves",
            "Grizzly Bears",
            "Ornithopter",
            "Serra Angel",
            "Doom Blade",
            "Raise Dead",
            "Unsummon",
        ] {
            game.external_semantic_scores.insert(
                card_name.to_ascii_lowercase(),
                if card_name == "Lightning Bolt" {
                    1.0
                } else {
                    0.0
                },
            );
        }
        let players_before = game.game.players.clone();
        let error = game
            .build_random_demo_deck_names(60, 24)
            .expect_err("one ordinary spell cannot legally fill 36 demo spell slots");

        assert!(error.contains("insufficient copy-limit capacity"));
        assert_eq!(game.game.players, players_before);
        assert!(game.loaded_decks.is_empty());
        assert!(
            game.game
                .players
                .iter()
                .all(|player| player.library.is_empty() && player.hand.is_empty())
        );
    }

    #[test]
    fn normal_constructed_sideboard_and_combined_copy_boundaries_are_enforced() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();

        game.apply_match_setup(normal_config(
            repeated("Plains", 60),
            repeated("Island", 15),
        ))
        .expect("a 15-card constructed sideboard should be accepted");
        assert!(
            validate_normal_config(
                &mut game,
                &normal_config(repeated("Plains", 60), repeated("Island", 16)),
            )
            .is_err(),
            "a 16-card constructed sideboard must be rejected"
        );

        let mut four_bolts = repeated("Plains", 57);
        four_bolts.extend(repeated("Lightning Bolt", 3));
        game.apply_match_setup(normal_config(
            four_bolts.clone(),
            repeated("Lightning Bolt", 1),
        ))
        .expect("four copies combined across main deck and sideboard should be legal");
        assert!(
            validate_normal_config(
                &mut game,
                &normal_config(four_bolts, repeated("Lightning Bolt", 2)),
            )
            .is_err(),
            "a fifth nonbasic copy in the sideboard must be rejected"
        );
    }

    #[test]
    fn normal_constructed_basic_and_oracle_copy_exceptions_are_authoritative() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let unlimited = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::from_raw(91_001),
            "Unlimited Copy Probe",
        )
        .card_types(vec![CardType::Creature])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::deck_construction_rule_text(
                "A deck can have any number of cards named Unlimited Copy Probe.",
            ),
        ))
        .build();
        game.registry.register(unlimited);

        game.apply_match_setup(normal_config(
            repeated("Plains", 60),
            repeated("Plains", 15),
        ))
        .expect("basic lands should be unlimited across deck and sideboard");
        game.apply_match_setup(normal_config(
            repeated("Unlimited Copy Probe", 60),
            repeated("Unlimited Copy Probe", 15),
        ))
        .expect("an explicit any-number Oracle rule should override the four-copy limit");
    }

    #[test]
    fn normal_constructed_linked_faces_share_one_stable_copy_identity() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let mut deck = repeated("Plains", 56);
        deck.extend([
            "Breaking".to_string(),
            "Entering".to_string(),
            "Breaking".to_string(),
            "Entering".to_string(),
        ]);
        let legal_config = normal_config(deck.clone(), Vec::new());
        validate_normal_config(&mut game, &legal_config)
            .expect("four linked-face aliases should count as four copies of one card");
        game.apply_match_setup(legal_config)
            .expect("the public setup path should accept that linked-face identity");
        deck.push("Breaking".to_string());
        assert!(
            validate_normal_config(&mut game, &normal_config(deck, Vec::new())).is_err(),
            "front and back face names must not create separate copy-limit buckets"
        );
    }

    #[test]
    fn normal_constructed_rejection_is_transactional_across_public_import_paths() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let players_before = game.game.players.clone();

        assert!(
            validate_normal_config(
                &mut game,
                &normal_config(repeated("Plains", 59), Vec::new()),
            )
            .is_err()
        );
        assert_eq!(game.game.players, players_before);
        assert!(game.pregame.is_none());
        assert!(game.loaded_decks.is_empty());

        game.apply_match_setup(normal_config(repeated("Plains", 60), Vec::new()))
            .expect("legal setup should enter pregame");
        assert!(
            game.validate_commander_manual_zone_addition(Zone::Library)
                .is_err(),
            "manual pregame library injection must not bypass constructed validation"
        );
        assert!(
            game.validate_commander_manual_zone_addition(Zone::OutsideGame)
                .is_err(),
            "manual pregame sideboard injection must not bypass constructed validation"
        );
        assert!(
            game.validate_commander_manual_zone_addition(Zone::Command)
                .is_err(),
            "normal constructed pregame injection must not create a commander zone"
        );
    }

    #[test]
    fn normal_constructed_hidden_manifests_enforce_counts_and_copy_limits_without_leaks() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();

        assert!(validate_normal_config(&mut game, &hidden_normal_config(59, 0)).is_err());
        assert!(validate_normal_config(&mut game, &hidden_normal_config(60, 16)).is_err());

        let mut incomplete = hidden_normal_config(60, 15);
        incomplete
            .hidden_deck_manifests
            .as_mut()
            .expect("hidden manifests")
            .first_mut()
            .expect("alice manifest")
            .slot_commitments
            .pop();
        assert!(validate_normal_config(&mut game, &incomplete).is_err());

        game.apply_match_setup(hidden_normal_config(60, 15))
            .expect("complete hidden normal-constructed setup should start");
        let alice = PlayerId::from_index(0);
        for slot in 0..4 {
            game.reveal_hidden_slot_input(RevealHiddenSlotInput {
                owner: 0,
                slot,
                card_name: "Lightning Bolt".to_string(),
                commitment: Some(format!("normal-0-slot-{slot}")),
                recompute_decision: false,
            })
            .expect("the first four committed copies should reveal");
        }
        let sideboard_slot = game
            .game
            .hidden_card_entries()
            .find(|(_, info)| info.owner == alice && info.slot == 60)
            .map(|(object_id, _)| *object_id)
            .expect("hidden sideboard slot should exist");
        let lightning_bolt = game
            .load_compilable_card_definition_result("Lightning Bolt")
            .expect("Lightning Bolt should compile");
        let error = game
            .validate_hidden_normal_reveal(alice, sideboard_slot, &lightning_bolt)
            .expect_err("a fifth combined deck-and-sideboard copy must be rejected");
        assert!(error.contains("copy limit"));
        assert!(
            !error.contains("Lightning Bolt"),
            "hidden-identity rejection must not disclose the committed card name"
        );
        assert!(game.game.is_hidden_card_placeholder(sideboard_slot));
    }

    #[test]
    fn ante_setup_selects_one_public_card_before_opening_hands_and_transfers_ownership() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let mut config = normal_config(repeated("Plains", 60), Vec::new());
        config.format = MatchFormatInput::Ante;
        config.opening_hand_size = Some(7);

        game.apply_match_setup(config)
            .expect("the public ante variation should be selectable");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        assert_eq!(game.game.ante.len(), 2);
        for player in [alice, bob] {
            assert_eq!(game.game.player(player).unwrap().library.len(), 52);
            assert_eq!(game.game.player(player).unwrap().hand.len(), 7);
            assert_eq!(
                game.game
                    .ante
                    .iter()
                    .filter_map(|id| game.game.object(*id))
                    .filter(|object| object.owner == player)
                    .count(),
                1
            );
        }
        for card in game.game.ante.iter().copied() {
            assert!(crate::object_visible_to_perspective(
                &game.game, alice, None, card
            ));
            assert!(crate::object_visible_to_perspective(
                &game.game, bob, None, card
            ));
        }

        game.record_game_result(GameResult::Winner(bob));
        assert!(
            game.game
                .ante
                .iter()
                .filter_map(|id| game.game.object(*id))
                .all(|object| object.owner == bob)
        );
    }

    #[test]
    fn ante_cards_are_illegal_off_variation_and_hidden_ante_setup_is_rejected() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let ante_probe = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::from_raw(407_003),
            "Ante Legality Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .oracle_text(
            "Remove Ante Legality Probe from your deck before playing if you're not playing for ante.",
        )
        .build();
        assert!(ante_probe.refers_to_ante);
        game.registry.register(ante_probe);

        let mut deck = repeated("Plains", 59);
        deck.push("Ante Legality Probe".to_string());
        let decks = two_player_lists(deck);
        assert!(
            game.validate_ante_card_legality_for_setup(Some(&decks), None, false)
                .expect_err("ante cards must be illegal in ordinary games")
                .contains("unless the match is played for ante")
        );
        game.validate_ante_card_legality_for_setup(Some(&decks), None, true)
            .expect("the same cards are legal when ante is selected");

        let hidden = hidden_manifests(60, 0);
        assert!(
            WasmGame::validate_ante_manifest_visibility(MatchFormatInput::Ante, &hidden).is_err(),
            "opaque committed cards cannot satisfy ante's public-examination rule"
        );
    }

    #[test]
    fn companion_setup_designates_owned_legal_sideboard_card_transactionally() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let selections = Some(vec![Some("Gyruda, Doom of Depths".to_string()), None]);
        game.apply_match_setup_with_companions_native(
            normal_config(
                repeated("Plains", 60),
                vec!["Gyruda, Doom of Depths".to_string()],
            ),
            selections,
        )
        .expect("an all-even starting deck may reveal Gyruda");

        let alice = PlayerId::from_index(0);
        let chosen = game.game.player(alice).unwrap().companion.expect("chosen companion");
        assert_eq!(game.game.object(chosen).unwrap().name, "Gyruda, Doom of Depths");
        assert_eq!(game.game.object(chosen).unwrap().zone, Zone::OutsideGame);

        let mut invalid_deck = repeated("Plains", 59);
        invalid_deck.push("Lightning Bolt".to_string());
        let players_before = game.game.players.clone();
        let error = game
            .apply_match_setup_with_companions_native(
                normal_config(
                    invalid_deck,
                    vec!["Gyruda, Doom of Depths".to_string()],
                ),
                Some(vec![Some("Gyruda, Doom of Depths".to_string()), None]),
            )
            .expect_err("an odd-mana-value card violates Gyruda's condition");
        assert!(error.contains("does not fulfill"), "{error}");
        assert_eq!(game.game.players, players_before, "rejection must not replace live setup");
    }
}

#[cfg(test)]
mod commander_setup_tests {
    use super::*;

    fn repeated(name: &str, count: usize) -> Vec<String> {
        vec![name.to_string(); count]
    }

    fn two_player_lists(list: Vec<String>) -> Vec<Vec<String>> {
        vec![list.clone(), list]
    }

    fn explicit_config(deck: Vec<String>, commanders: Vec<String>) -> MatchSetupInput {
        MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 5,
            seed: 41,
            format: MatchFormatInput::Commander,
            decks: Some(two_player_lists(deck)),
            sideboards: None,
            commanders: Some(two_player_lists(commanders)),
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(1),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    fn hidden_manifests(deck_count: usize, commander_count: usize) -> Vec<HiddenDeckManifestInput> {
        (0..2)
            .map(|owner| HiddenDeckManifestInput {
                owner,
                deck_count,
                sideboard_count: 0,
                commander_count,
                decklist_hash: format!("deck-{owner}"),
                commitment_root: format!("root-{owner}"),
                slot_commitments: (0..deck_count)
                    .map(|slot| HiddenDeckSlotInput {
                        slot: slot as u16,
                        commitment: format!("player-{owner}-slot-{slot}"),
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn commander_validation_accepts_single_partner_and_background_constructions() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();

        game.validate_commander_setup(
            2,
            &two_player_lists(repeated("Plains", 99)),
            &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
            None,
            &[],
        )
        .expect("a legendary creature and 99 in-identity basic lands should be legal");

        game.validate_commander_setup(
            2,
            &two_player_lists(repeated("Plains", 98)),
            &two_player_lists(vec![
                "Rograkh, Son of Rohgahh".to_string(),
                "Tymna the Weaver".to_string(),
            ]),
            None,
            &[],
        )
        .expect("two ordinary partner commanders should be legal");

        game.validate_commander_setup(
            2,
            &two_player_lists(repeated("Plains", 98)),
            &two_player_lists(vec![
                "Abdel Adrian, Gorion's Ward".to_string(),
                "Candlekeep Sage".to_string(),
            ]),
            None,
            &[],
        )
        .expect("choose a Background plus a legendary Background should be legal");
    }

    #[test]
    fn commander_validation_rejects_ineligible_pairs_duplicates_identity_and_sideboards() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let players_before_validation = game.game.players.clone();

        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(repeated("Plains", 99)),
                &two_player_lists(vec!["Sol Ring".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("not an eligible")
        );
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(repeated("Plains", 98)),
                &two_player_lists(vec![
                    "Rograkh, Son of Rohgahh".to_string(),
                    "Isamaru, Hound of Konda".to_string(),
                ]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("legal shared partner")
        );

        let mut duplicate = repeated("Plains", 97);
        duplicate.extend(["Sol Ring".to_string(), "Sol Ring".to_string()]);
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(duplicate),
                &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("only one card named Sol Ring")
        );

        let mut off_identity = repeated("Plains", 98);
        off_identity.push("Lightning Bolt".to_string());
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(off_identity),
                &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("outside the commander's color identity")
        );
        let mut linked_face_identity = repeated("Island", 98);
        linked_face_identity.push("Breaking".to_string());
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(linked_face_identity),
                &two_player_lists(vec!["Oona, Queen of the Fae".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("outside the commander's color identity")
        );
        let mut disallowed_basic_type = repeated("Plains", 98);
        disallowed_basic_type.push("Tundra".to_string());
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(disallowed_basic_type),
                &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("basic land type outside")
        );
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(repeated("Island", 99)),
                &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
                None,
                &[],
            )
            .unwrap_err()
            .contains("outside the commander's color identity")
        );
        assert!(
            game.validate_commander_setup(
                2,
                &two_player_lists(repeated("Plains", 99)),
                &two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]),
                Some(&two_player_lists(vec!["Lightning Bolt".to_string()])),
                &[],
            )
            .unwrap_err()
            .contains("do not use sideboards")
        );
        assert_eq!(
            game.game.players, players_before_validation,
            "format validation must finish before replacing live match state"
        );
    }

    #[test]
    fn commander_runtime_derives_life_and_opening_hand_from_the_format() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.apply_match_setup(explicit_config(
            repeated("Plains", 99),
            vec!["Isamaru, Hound of Konda".to_string()],
        ))
        .expect("valid Commander setup should start");

        assert!(game.game.players.iter().all(|player| player.life == 40));
        assert!(
            game.game
                .players
                .iter()
                .all(|player| player.hand.len() == 7)
        );
        assert_eq!(
            game.pregame
                .as_ref()
                .map(|pregame| pregame.opening_hand_size),
            Some(7)
        );
        assert!(
            game.game
                .players
                .iter()
                .all(|player| player.commanders.len() == 1)
        );
    }

    #[test]
    fn commander_hidden_setup_requires_complete_counts_and_validates_reveals() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let commanders = two_player_lists(vec!["Isamaru, Hound of Konda".to_string()]);
        let decks = two_player_lists(Vec::new());
        let manifests = hidden_manifests(99, 1);
        game.validate_commander_setup(2, &decks, &commanders, None, &manifests)
            .expect("complete committed Commander decks should pass setup validation");

        let mut incomplete = manifests.clone();
        incomplete[0].slot_commitments.pop();
        assert!(
            game.validate_commander_setup(2, &decks, &commanders, None, &incomplete)
                .unwrap_err()
                .contains("commit every main-deck slot")
        );

        let config = MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 20,
            seed: 9,
            format: MatchFormatInput::Commander,
            decks: Some(decks),
            sideboards: None,
            commanders: Some(commanders),
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(3),
            hidden_deck_manifests: Some(manifests),
            free_for_all: None,
            teams: None,
        };
        game.apply_match_setup(config)
            .expect("complete hidden Commander setup should start");
        let (&hidden_object, info) = game
            .game
            .hidden_card_entries()
            .next()
            .expect("hidden deck should create committed placeholders");
        let owner = info.owner;
        let lightning_bolt = game
            .load_compilable_card_definition_result("Lightning Bolt")
            .expect("Lightning Bolt should compile");
        assert!(
            game.validate_hidden_commander_reveal(owner, hidden_object, &lightning_bolt)
                .unwrap_err()
                .contains("outside the commander's color identity")
        );
        assert!(game.game.is_hidden_card_placeholder(hidden_object));

        let hidden_for_owner: Vec<ObjectId> = game
            .game
            .hidden_card_entries()
            .filter(|(_, info)| info.owner == owner)
            .map(|(object_id, _)| *object_id)
            .take(2)
            .collect();
        let sol_ring = game
            .load_compilable_card_definition_result("Sol Ring")
            .expect("Sol Ring should compile");
        game.game
            .reveal_hidden_card_with_definition(hidden_for_owner[0], &sol_ring)
            .expect("test setup should reveal the first committed Sol Ring");
        assert!(
            game.validate_hidden_commander_reveal(owner, hidden_for_owner[1], &sol_ring)
                .unwrap_err()
                .contains("only one card named Sol Ring")
        );
    }

    #[test]
    fn commander_manual_command_and_outside_game_injection_cannot_bypass_setup() {
        let mut game = WasmGame::new();
        game.match_format = MatchFormatInput::Commander;
        assert!(
            game.validate_commander_manual_zone_addition(Zone::Command)
                .is_err()
        );
        assert!(
            game.validate_commander_manual_zone_addition(Zone::OutsideGame)
                .is_err()
        );
        assert!(
            game.validate_commander_manual_zone_addition(Zone::Hand)
                .is_ok()
        );
    }

    #[test]
    fn commander_companion_uses_the_pre_command_zone_starting_deck() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.apply_match_setup_with_companions_native(
            explicit_config(repeated("Swamp", 99), vec!["Anje Falkenrath".to_string()]),
            Some(vec![Some("Obosh, the Preypiercer".to_string()), None]),
        )
        .expect("odd-valued commander plus lands fulfills Obosh in Commander");
        let alice = PlayerId::from_index(0);
        let chosen = game.game.player(alice).unwrap().companion.expect("Commander companion");
        assert_eq!(game.game.object(chosen).unwrap().zone, Zone::OutsideGame);
        assert_eq!(game.game.player(alice).unwrap().commanders.len(), 1);

        let mut invalid = WasmGame::new();
        let error = invalid
            .validate_companion_setup(
                &explicit_config(
                    repeated("Plains", 99),
                    vec!["Athreos, God of Passage".to_string()],
                ),
                Some(&[Some("Lurrus of the Dream-Den".to_string()), None]),
            )
            .expect_err("the mana-value-3 commander is part of Lurrus's starting deck check");
        assert!(error.contains("does not fulfill"), "{error}");
    }
}

#[cfg(test)]
mod commander_draft_setup_tests {
    use super::*;

    fn repeated(name: &str, count: usize) -> Vec<String> {
        vec![name.to_string(); count]
    }

    fn three_player_lists(list: Vec<String>) -> Vec<Vec<String>> {
        vec![list.clone(), list.clone(), list]
    }

    fn config(
        products: Vec<CommanderDraftProductInput>,
        deck: Vec<String>,
        commanders: Vec<String>,
        pool: Vec<String>,
    ) -> MatchSetupInput {
        MatchSetupInput {
            player_names: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            starting_life: 1,
            seed: 903_013,
            format: MatchFormatInput::CommanderDraft,
            decks: Some(three_player_lists(deck)),
            sideboards: None,
            commanders: Some(three_player_lists(commanders)),
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: Some(CommanderDraftSetupInput {
                products,
                card_pools: three_player_lists(pool),
            }),
            opening_hand_size: Some(1),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    fn validate(game: &mut WasmGame, config: &MatchSetupInput) -> Result<(), String> {
        game.validate_commander_draft_setup(
            config.player_names.len(),
            config.decks.as_deref().unwrap(),
            config.commanders.as_deref().unwrap(),
            config.sideboards.as_deref(),
            config.hidden_deck_manifests.as_deref().unwrap_or(&[]),
            config.commander_draft.as_ref().unwrap(),
        )
    }

    #[test]
    fn u077_commander_draft_handoff_uses_commander_multiplayer_rules_and_checkpoints() {
        let _id_guard = crate::test_id_counter_guard();
        let mut deck = repeated("Plains", 57);
        deck.extend(repeated("Sol Ring", 2));
        let setup = config(
            Vec::new(),
            deck,
            vec!["Isamaru, Hound of Konda".to_string()],
            vec![
                "Isamaru, Hound of Konda".to_string(),
                "Sol Ring".to_string(),
                "Sol Ring".to_string(),
            ],
        );
        let mut host = WasmGame::new();
        host.apply_match_setup(setup)
            .expect("a legal completed Commander Legends draft should start");

        assert_eq!(host.match_format, MatchFormatInput::CommanderDraft);
        assert!(host.game.commander_damage_loss_enabled());
        assert!(host.game.players.iter().all(|player| {
            player.life == 40
                && player.hand.len() == 7
                && player.sideboard.is_empty()
                && player.commanders.len() == 1
                && host
                    .game
                    .object(player.commanders[0])
                    .is_some_and(|object| object.zone == Zone::Command)
        }));
        let profile = host.game.free_for_all().expect("multiplayer profile");
        assert_eq!(
            profile.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(profile.range_of_influence(), None);

        let checkpoint = host.build_sync_checkpoint();
        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("Commander Draft checkpoint should import");
        assert_eq!(guest.match_format, MatchFormatInput::CommanderDraft);
        assert!(guest.game.commander_damage_loss_enabled());
        assert_eq!(
            guest.game.free_for_all().unwrap().attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
    }

    #[test]
    fn u077_commander_draft_construction_uses_physical_pool_copies_and_no_maximum() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let no_maximum = config(
            vec![CommanderDraftProductInput::CommanderLegends],
            repeated("Plains", 60),
            vec!["Isamaru, Hound of Konda".to_string()],
            vec!["Isamaru, Hound of Konda".to_string()],
        );
        validate(&mut game, &no_maximum)
            .expect("Commander Draft has a 60-card minimum and no maximum");

        let mut duplicate_deck = repeated("Plains", 57);
        duplicate_deck.extend(repeated("Sol Ring", 2));
        let mut duplicate_setup = config(
            vec![CommanderDraftProductInput::CommanderLegends],
            duplicate_deck,
            vec!["Isamaru, Hound of Konda".to_string()],
            vec![
                "Isamaru, Hound of Konda".to_string(),
                "Sol Ring".to_string(),
                "Sol Ring".to_string(),
            ],
        );
        validate(&mut game, &duplicate_setup)
            .expect("duplicate names are legal when both physical copies were drafted");

        duplicate_setup.decks.as_mut().unwrap()[0][0] = "Sol Ring".to_string();
        assert!(validate(&mut game, &duplicate_setup).is_err());

        game.initialize_empty_match(vec!["Existing".to_string()], 17, 77);
        assert!(validate(&mut game, &duplicate_setup).is_err());
        assert_eq!(game.game.players.len(), 1);
        assert_eq!(game.game.players[0].name, "Existing");
        assert_eq!(game.game.players[0].life, 17);
        assert_eq!(game.match_format, MatchFormatInput::Normal);
    }

    #[test]
    fn u077_commander_masters_and_special_product_exceptions_are_scoped() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let masters = config(
            vec![CommanderDraftProductInput::CommanderMasters],
            repeated("Plains", 58),
            vec![
                "Isamaru, Hound of Konda".to_string(),
                "Sram, Senior Edificer".to_string(),
            ],
            vec![
                "Isamaru, Hound of Konda".to_string(),
                "Sram, Senior Edificer".to_string(),
            ],
        );
        validate(&mut game, &masters)
            .expect("two independently eligible one-color commanders gain partner in Masters");

        let mut legends = masters.clone();
        legends.commander_draft.as_mut().unwrap().products =
            vec![CommanderDraftProductInput::CommanderLegends];
        assert!(
            validate(&mut game, &legends)
                .unwrap_err()
                .contains("legal shared Commander Draft partner")
        );

        let mut two_color = masters;
        for commanders in two_color.commanders.as_mut().unwrap() {
            commanders[1] = "Oona, Queen of the Fae".to_string();
        }
        for pool in &mut two_color.commander_draft.as_mut().unwrap().card_pools {
            pool[1] = "Oona, Queen of the Fae".to_string();
        }
        assert!(validate(&mut game, &two_color).is_err());

        let piper = config(
            vec![CommanderDraftProductInput::CommanderLegends],
            repeated("Wastes", 58),
            repeated("The Prismatic Piper", 2),
            Vec::new(),
        );
        validate(&mut game, &piper)
            .expect("Legends permits two Piper commander additions outside the pool");
        let mut wrong_product = piper;
        wrong_product.commander_draft.as_mut().unwrap().products =
            vec![CommanderDraftProductInput::Other];
        assert!(validate(&mut game, &wrong_product).is_err());
    }
}

#[cfg(test)]
mod planechase_setup_tests {
    use super::*;

    #[test]
    fn planechase_setup_requires_typed_planar_decks_and_reveals_after_opening_actions() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let planar_deck = (0..30)
            .map(|index| {
                let name = format!("Setup Plane {index}");
                game.registry.register(CardDefinition::new(
                    ironsmith::CardBuilder::new(CardId::new(), &name)
                        .card_types(vec![CardType::Plane])
                        .build(),
                ));
                PlanarCardInput { name, kind: None }
            })
            .collect::<Vec<_>>();
        let main_deck = vec!["Plains".to_string(); 60];
        let config = MatchSetupInput {
            player_names: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            starting_life: 20,
            seed: 901,
            format: MatchFormatInput::Planechase,
            decks: Some(vec![main_deck.clone(), main_deck.clone(), main_deck]),
            sideboards: None,
            commanders: None,
            planar_decks: Some(vec![planar_deck]),
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(0),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        };

        game.apply_match_setup(config)
            .expect("typed communal Planechase setup should start");
        assert_eq!(game.match_format, MatchFormatInput::Planechase);
        assert!(game.game.planechase.is_some());
        assert!(game.game.face_up_planar_objects().is_empty());
        let free_for_all = game
            .game
            .free_for_all()
            .expect("multiplayer Planechase should use the default Free-for-All profile");
        assert_eq!(
            free_for_all.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(free_for_all.range_of_influence(), None);

        let turn_order_len = game.game.turn_store.turn_order.len();
        game.pregame.as_mut().unwrap().stage = PregameStage::OpeningActions {
            current_index: turn_order_len,
            pending_hand_exile: None,
        };
        game.normalize_pregame_state()
            .expect("starting plane reveal should complete");
        assert!(game.pregame.is_none());
        assert_eq!(game.game.face_up_planar_objects().len(), 1);
    }
}

#[cfg(test)]
mod vanguard_setup_tests {
    use super::*;

    fn register_vanguard(game: &mut WasmGame, name: &str) {
        game.registry.register(CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Vanguard])
                .build(),
        ));
    }

    fn vanguard_config() -> MatchSetupInput {
        let main_deck = vec!["Plains".to_string(); 60];
        MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 99,
            seed: 902,
            format: MatchFormatInput::Vanguard,
            decks: Some(vec![main_deck.clone(), main_deck]),
            sideboards: None,
            commanders: None,
            planar_decks: None,
            vanguards: Some(vec![
                VanguardCardInput {
                    name: "Patient Avatar".to_string(),
                    hand_modifier: 2,
                    life_modifier: -3,
                },
                VanguardCardInput {
                    name: "Fierce Avatar".to_string(),
                    hand_modifier: -1,
                    life_modifier: 4,
                },
            ]),
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            // Vanguard always uses seven as its unmodified opening-hand basis.
            opening_hand_size: Some(1),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    #[test]
    fn vanguard_setup_applies_each_players_signed_modifiers_and_command_card() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        register_vanguard(&mut game, "Patient Avatar");
        register_vanguard(&mut game, "Fierce Avatar");

        game.apply_match_setup(vanguard_config())
            .expect("typed Vanguard setup should start");

        let alice = game.game.players[0].id;
        let bob = game.game.players[1].id;
        assert_eq!(game.match_format, MatchFormatInput::Vanguard);
        assert_eq!(game.game.player(alice).unwrap().life, 17);
        assert_eq!(game.game.player(bob).unwrap().life, 24);
        assert_eq!(game.game.player(alice).unwrap().hand.len(), 9);
        assert_eq!(game.game.player(bob).unwrap().hand.len(), 6);
        assert_eq!(game.game.player(alice).unwrap().max_hand_size, 9);
        assert_eq!(game.game.player(bob).unwrap().max_hand_size, 6);
        assert_eq!(
            game.pregame.as_ref().unwrap().opening_hand_size_for(alice),
            9
        );
        assert_eq!(game.pregame.as_ref().unwrap().opening_hand_size_for(bob), 6);

        let pregame = game.pregame.as_mut().unwrap();
        pregame.mulligans_taken.insert(alice, 8);
        pregame.mulligans_taken.insert(bob, 6);
        assert!(pregame.can_take_mulligan(alice));
        assert!(!pregame.can_take_mulligan(bob));
        pregame.mulligans_taken.insert(alice, 9);
        assert!(!pregame.can_take_mulligan(alice));

        for player in [alice, bob] {
            let object = game
                .game
                .vanguard_card(player)
                .expect("each player should own one vanguard");
            assert!(game.game.command_zone.contains(&object));
            assert_eq!(game.game.object(object).unwrap().zone, Zone::Command);
            assert_eq!(game.game.current_controller(object), Some(player));
        }
    }

    #[test]
    fn vanguard_loader_rejects_untyped_or_miscounted_payload() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        register_vanguard(&mut game, "Patient Avatar");

        let mut config = vanguard_config();
        let error = game
            .load_vanguards_for_setup(config.vanguards.as_deref().unwrap(), 2)
            .expect_err("unknown/untyped Vanguard payload should be rejected");
        assert!(error.contains("Fierce Avatar"));

        config.vanguards.as_mut().unwrap().pop();
        let error = game
            .load_vanguards_for_setup(config.vanguards.as_deref().unwrap(), 2)
            .expect_err("one Vanguard for two players should be rejected");
        assert!(error.contains("exactly one"));
    }
}

#[cfg(test)]
mod archenemy_setup_tests {
    use super::*;

    fn archenemy_config(game: &mut WasmGame) -> MatchSetupInput {
        let schemes = (0..20)
            .map(|index| {
                let name = format!("Setup Scheme {index}");
                game.registry.register(CardDefinition::new(
                    ironsmith::CardBuilder::new(CardId::new(), &name)
                        .card_types(vec![CardType::Scheme])
                        .build(),
                ));
                name
            })
            .collect::<Vec<_>>();
        let main_deck = vec!["Plains".to_string(); 60];
        MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 99,
            seed: 903,
            format: MatchFormatInput::Archenemy,
            decks: Some(vec![main_deck.clone(), main_deck]),
            sideboards: None,
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: Some(vec![schemes, Vec::new()]),
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(1),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    #[test]
    fn archenemy_setup_loads_typed_schemes_and_designates_nonempty_owner() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let config = archenemy_config(&mut game);
        game.apply_match_setup(config)
            .expect("typed Archenemy setup should start");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        assert_eq!(game.match_format, MatchFormatInput::Archenemy);
        assert!(game.game.is_archenemy(alice));
        assert!(!game.game.is_archenemy(bob));
        assert_eq!(game.game.player(alice).unwrap().life, 40);
        assert_eq!(game.game.player(bob).unwrap().life, 20);
        assert_eq!(game.game.scheme_deck(alice).unwrap().len(), 20);
        assert_eq!(game.game.player(alice).unwrap().hand.len(), 7);
    }

    #[test]
    fn archenemy_setup_rejects_multiple_designated_decks_transactionally() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let mut config = archenemy_config(&mut game);
        let second_deck = config.scheme_decks.as_ref().unwrap()[0].clone();
        config.scheme_decks.as_mut().unwrap()[1] = second_deck;
        assert!(
            game.load_scheme_decks_for_setup(
                config.scheme_decks.as_deref().unwrap(),
                2,
                ironsmith::game_state::ArchenemyVariant::Default,
            )
            .unwrap_err()
            .contains("exactly one nonempty")
        );
        assert!(game.game.archenemy.is_none());
    }

    #[test]
    fn supervillain_rumble_setup_is_free_for_all_and_requires_three_players() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Existing".to_string()], 17, 1);
        let mut config = archenemy_config(&mut game);
        config.format = MatchFormatInput::SupervillainRumble;
        let schemes = config.scheme_decks.as_ref().unwrap()[0].clone();
        config.scheme_decks = Some(vec![schemes.clone(), schemes]);
        assert!(config.validate_multiplayer_profile().is_err());
        assert_eq!(game.game.players.len(), 1);

        config.player_names.push("Charlie".to_string());
        config
            .decks
            .as_mut()
            .unwrap()
            .push(vec!["Plains".to_string(); 60]);
        let third_scheme_deck = config.scheme_decks.as_ref().unwrap()[0].clone();
        config
            .scheme_decks
            .as_mut()
            .unwrap()
            .push(third_scheme_deck);
        game.apply_match_setup(config)
            .expect("three-player Supervillain Rumble setup");

        let state = game.game.free_for_all().expect("Rumble profile");
        assert_eq!(
            state.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(state.range_of_influence(), None);
        assert_eq!(
            game.game
                .scheme_deck(PlayerId::from_index(0))
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            game.game
                .scheme_deck(PlayerId::from_index(1))
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            game.game
                .scheme_deck(PlayerId::from_index(2))
                .unwrap()
                .len(),
            20
        );
    }
}

#[cfg(test)]
mod conspiracy_setup_tests {
    use super::*;

    fn conspiracy_config(game: &mut WasmGame) -> MatchSetupInput {
        let definition =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Drafted Secret")
                .card_types(vec![CardType::Conspiracy])
                .parse_text("Hidden agenda")
                .expect("synthetic hidden-agenda conspiracy should compile");
        game.registry.register(definition);
        MatchSetupInput {
            player_names: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            starting_life: 99,
            seed: 905,
            format: MatchFormatInput::ConspiracyDraft,
            decks: Some(vec![
                vec!["Plains".to_string(); 40],
                vec!["Plains".to_string(); 40],
                vec!["Plains".to_string(); 40],
            ]),
            sideboards: Some(vec![
                vec!["Drafted Secret".to_string(), "Island".to_string()],
                Vec::new(),
                Vec::new(),
            ]),
            commanders: None,
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: Some(vec![
                vec![ConspiracyCardInput {
                    name: "Drafted Secret".to_string(),
                    agenda_names: vec!["Grizzly Bears".to_string()],
                }],
                Vec::new(),
                Vec::new(),
            ]),
            commander_draft: None,
            opening_hand_size: Some(1),
            hidden_deck_manifests: None,
            free_for_all: None,
            teams: None,
        }
    }

    #[test]
    fn conspiracy_setup_consumes_selected_sideboard_cards_and_keeps_agendas_hidden() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        let config = conspiracy_config(&mut game);
        game.apply_match_setup(config)
            .expect("legal post-draft Conspiracy setup should start");

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        assert_eq!(game.match_format, MatchFormatInput::ConspiracyDraft);
        assert_eq!(game.game.player(alice).unwrap().life, 20);
        assert_eq!(game.game.player(bob).unwrap().life, 20);
        assert_eq!(game.game.player(alice).unwrap().hand.len(), 7);
        assert_eq!(game.game.player(alice).unwrap().sideboard.len(), 1);
        let free_for_all = game.game.free_for_all().expect("Conspiracy game profile");
        assert_eq!(
            free_for_all.attack_option(),
            ironsmith::FreeForAllAttackOption::MultiplePlayers
        );
        assert_eq!(free_for_all.range_of_influence(), None);
        let conspiracy = game.game.conspiracy_cards()[0];
        assert!(game.game.is_face_down_conspiracy(conspiracy));
        assert_eq!(
            game.game.agenda_names_for(alice, conspiracy).unwrap(),
            ["Grizzly Bears"]
        );
        assert!(game.game.agenda_names_for(bob, conspiracy).is_none());
        assert_eq!(game.game.object(conspiracy).unwrap().zone, Zone::Command);
    }

    #[test]
    fn conspiracy_setup_rejects_a_selection_outside_the_drafted_sideboard_transactionally() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Existing".to_string()], 17, 1);
        let mut config = conspiracy_config(&mut game);
        config.sideboards.as_mut().unwrap()[0].clear();

        let error = game
            .load_conspiracies_for_setup(
                config.conspiracies.as_deref().unwrap(),
                config.sideboards.as_deref().unwrap(),
                3,
            )
            .unwrap_err();
        assert!(error.contains("not available"));
        assert_eq!(game.game.players.len(), 1);
        assert_eq!(game.game.player(PlayerId::from_index(0)).unwrap().life, 17);
        assert!(game.game.conspiracy.is_none());
    }
}
