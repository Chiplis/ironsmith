use super::*;

pub(crate) fn parse_ability_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_on_comma_or_semicolon(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(&segment) {
            actions.extend(protection_actions);
            continue;
        }

        // Try the segment as-is first, then split on "and" for compound keywords
        if let Some(action) = parse_ability_phrase(&segment) {
            actions.push(action);
        } else {
            // Split on "and" to handle "menace and deathtouch", "trample and haste", etc.
            let and_parts = split_on_and(&segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in &and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn reject_unimplemented_keyword_actions(
    _actions: &[KeywordAction],
    _clause: &str,
) -> Result<(), CardTextError> {
    Ok(())
}

pub(crate) fn parse_protection_chain(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let mut words = words(tokens);
    if words.first().copied() == Some("and") {
        words.remove(0);
    }
    if words.len() < 3 {
        return None;
    }
    if words[0] != "protection" || words[1] != "from" {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        match value {
            "white" => Some(KeywordAction::ProtectionFrom(ColorSet::WHITE)),
            "blue" => Some(KeywordAction::ProtectionFrom(ColorSet::BLUE)),
            "black" => Some(KeywordAction::ProtectionFrom(ColorSet::BLACK)),
            "red" => Some(KeywordAction::ProtectionFrom(ColorSet::RED)),
            "green" => Some(KeywordAction::ProtectionFrom(ColorSet::GREEN)),
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "all" if matches!(words.get(idx + 2).copied(), Some("color") | Some("colors")) => {
                Some(KeywordAction::ProtectionFromAllColors)
            }
            _ => parse_card_type(value)
                .map(KeywordAction::ProtectionFromCardType)
                .or_else(|| {
                    parse_subtype_word(value)
                        .or_else(|| value.strip_suffix('s').and_then(parse_subtype_word))
                        .map(KeywordAction::ProtectionFromSubtype)
                }),
        }
    };

    let mut from_count = 0usize;
    let mut parsed_count = 0usize;
    for idx in 0..words.len().saturating_sub(1) {
        if words[idx] != "from" {
            continue;
        }
        from_count += 1;
        if let Some(action) = parse_from_target(&words, idx) {
            parsed_count += 1;
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }

    if actions.is_empty() || parsed_count < from_count {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn keyword_action_to_static_ability(action: KeywordAction) -> Option<StaticAbility> {
    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
        KeywordAction::Haste => Some(StaticAbility::haste()),
        KeywordAction::Improvise => Some(StaticAbility::improvise()),
        KeywordAction::Convoke => Some(StaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => Some(StaticAbility::affinity_for_artifacts()),
        KeywordAction::Delve => Some(StaticAbility::delve()),
        KeywordAction::FirstStrike => Some(StaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(StaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(StaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(StaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(StaticAbility::vigilance()),
        KeywordAction::Trample => Some(StaticAbility::trample()),
        KeywordAction::Reach => Some(StaticAbility::reach()),
        KeywordAction::Defender => Some(StaticAbility::defender()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Phasing => Some(StaticAbility::phasing()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            StaticAbility::ward(TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(generic),
            ])))
        }),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Afterlife(amount) => Some(StaticAbility::custom(
            "afterlife",
            format!("afterlife {amount}"),
        )),
        KeywordAction::Fabricate(amount) => Some(StaticAbility::custom(
            "fabricate",
            format!("fabricate {amount}"),
        )),
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => Some(StaticAbility::custom("undying", "undying".to_string())),
        KeywordAction::Persist => Some(StaticAbility::custom("persist", "persist".to_string())),
        KeywordAction::Prowess => Some(StaticAbility::custom("prowess", "prowess".to_string())),
        KeywordAction::Exalted => Some(StaticAbility::custom("exalted", "exalted".to_string())),
        KeywordAction::Cascade => Some(StaticAbility::cascade()),
        KeywordAction::Storm => Some(StaticAbility::custom("storm", "storm".to_string())),
        KeywordAction::Toxic(amount) => {
            Some(StaticAbility::custom("toxic", format!("toxic {amount}")))
        }
        KeywordAction::BattleCry => Some(StaticAbility::custom(
            "battle_cry",
            "battle cry".to_string(),
        )),
        KeywordAction::Dethrone => Some(StaticAbility::custom("dethrone", "dethrone".to_string())),
        KeywordAction::Evolve => Some(StaticAbility::custom("evolve", "evolve".to_string())),
        KeywordAction::Ingest => Some(StaticAbility::custom("ingest", "ingest".to_string())),
        KeywordAction::Mentor => Some(StaticAbility::custom("mentor", "mentor".to_string())),
        KeywordAction::Skulk => Some(StaticAbility::skulk()),
        KeywordAction::Training => Some(StaticAbility::custom("training", "training".to_string())),
        KeywordAction::Riot => Some(StaticAbility::custom("riot", "riot".to_string())),
        KeywordAction::Unleash => Some(StaticAbility::unleash()),
        KeywordAction::Renown(amount) => {
            Some(StaticAbility::custom("renown", format!("renown {amount}")))
        }
        KeywordAction::Modular(amount) => Some(StaticAbility::custom(
            "modular",
            format!("modular {amount}"),
        )),
        KeywordAction::Graft(amount) => {
            Some(StaticAbility::custom("graft", format!("graft {amount}")))
        }
        KeywordAction::Soulbond => Some(StaticAbility::custom("soulbond", "soulbond".to_string())),
        KeywordAction::Soulshift(amount) => Some(StaticAbility::custom(
            "soulshift",
            format!("soulshift {amount}"),
        )),
        KeywordAction::Outlast(cost) => Some(StaticAbility::custom(
            "outlast",
            format!("outlast {}", cost.to_oracle()),
        )),
        KeywordAction::Unearth(cost) => Some(StaticAbility::custom(
            "unearth",
            format!("unearth {}", cost.to_oracle()),
        )),
        KeywordAction::Ninjutsu(cost) => Some(StaticAbility::custom(
            "ninjutsu",
            format!("ninjutsu {}", cost.to_oracle()),
        )),
        KeywordAction::Echo { .. } => None,
        KeywordAction::Extort => Some(StaticAbility::custom("extort", "extort".to_string())),
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => Some(StaticAbility::custom("sunburst", "sunburst".to_string())),
        KeywordAction::Fading(amount) => {
            Some(StaticAbility::custom("fading", format!("fading {amount}")))
        }
        KeywordAction::Vanishing(amount) => Some(StaticAbility::custom(
            "vanishing",
            format!("vanishing {amount}"),
        )),
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::Landwalk(subtype) => Some(StaticAbility::landwalk(subtype)),
        KeywordAction::Bloodthirst(amount) => Some(StaticAbility::bloodthirst(amount)),
        KeywordAction::Rampage(amount) => Some(StaticAbility::custom(
            "rampage",
            format!("rampage {amount}"),
        )),
        KeywordAction::Bushido(amount) => Some(StaticAbility::custom(
            "bushido",
            format!("bushido {amount}"),
        )),
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => Some(StaticAbility::custom(
            "protection_from_subtype",
            format!("protection from {:?}", subtype),
        )),
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(amount) => Some(StaticAbility::custom(
            "annihilator",
            format!("annihilator {amount}"),
        )),
        KeywordAction::ForMirrodin => None,
        KeywordAction::LivingWeapon => None,
        KeywordAction::Crew { .. } => None,
        KeywordAction::Saddle { .. } => None,
        KeywordAction::Marker(name) => Some(StaticAbility::custom(name, name.to_string())),
        KeywordAction::MarkerText(text) => Some(StaticAbility::custom("keyword_marker", text)),
        KeywordAction::Casualty(_) => None,
        KeywordAction::Conspire => None,
        KeywordAction::Devour(_) => None,
        KeywordAction::Ravenous => None,
        KeywordAction::Ascend => None,
        KeywordAction::Haunt => None,
        KeywordAction::Provoke => None,
        KeywordAction::Undaunted => None,
        KeywordAction::Enlist => None,
    }
}

pub(crate) fn parse_static_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if let Some(ability) = parse_ward_static_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_static_text_marker_line(tokens) {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_enters_tapped_with_choose_color_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_damage_not_removed_cleanup_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_prevent_damage_to_source_remove_counter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_choose_color_as_enters_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_damage_redirect_to_source_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) =
        parse_no_more_than_creatures_can_attack_or_block_each_combat_line(tokens)?
    {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_characteristic_defining_pt_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_no_maximum_hand_size_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_reduced_maximum_hand_size_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_library_of_leng_discard_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_draw_replace_exile_top_face_down_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_toph_first_metalbender_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_discard_or_redirect_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_pay_life_or_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_copy_activated_abilities_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_source_activation_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enchanted_has_activated_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) =
        parse_has_base_power_toughness_and_granted_keywords_static_line(tokens)?
    {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_filter_has_granted_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_equipped_gets_and_has_activated_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_shuffle_into_library_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_permanents_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_cant_cycle_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_starting_life_bonus_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_buyback_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_flashback_cost_modifier_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_spells_cost_modifier_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_skip_upkeep_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_are_artifacts_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_cards_spells_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_all_are_color_and_type_addition_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_creatures_are_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_blood_moon_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_land_type_addition_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_lands_are_pt_creatures_still_lands_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_remove_snow_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_attached_is_legendary_gets_and_has_keywords_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_soulbond_shared_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_granted_keyword_static_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_lose_all_abilities_and_transform_base_pt_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_lose_all_abilities_and_base_pt_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_creatures_lose_flying_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_each_creature_cant_be_blocked_by_more_than_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) =
        parse_each_creature_can_block_additional_creature_each_combat_line(tokens)?
    {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_anthem_and_type_color_addition_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_anthem_and_keyword_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_have_indestructible_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) =
        parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(tokens)?
    {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_subject_cant_be_blocked_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_may_choose_not_to_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_untap_during_each_other_players_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_doesnt_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_equipped_creature_has_line(tokens)? {
        return Ok(Some(ability));
    }
    if let Some(ability) = parse_enchanted_creature_has_line(tokens)? {
        return Ok(Some(ability));
    }
    if let Some(ability) = parse_you_control_attached_creature_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_attached_cant_attack_or_block_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_attached_prevent_all_damage_dealt_by_attached_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_attached_gets_and_cant_block_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_attached_has_keywords_and_triggered_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_attached_gets_and_has_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_gets_and_attacks_each_combat_if_able_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_attacks_each_combat_if_able_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_composed_anthem_effects_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_has_base_power_toughness_static_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_anthem_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_flying_restriction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_can_block_only_flying_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_assign_damage_as_unblocked_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_grant_flash_to_noncreature_spells_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_prevent_all_damage_to_source_by_creatures_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_prevent_all_damage_dealt_to_creatures_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_creatures_cant_block_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_enters_tapped_with_counters_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_enters_with_counters_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_with_additional_counter_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_conditional_enters_tapped_unless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_additional_land_play_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_play_lands_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_can_block_additional_creature_each_combat_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_creatures_able_to_block_source_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_activated_abilities_cant_be_activated_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_cant_clauses(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = words(tokens);
    if normalized.len() < 6 || !normalized.starts_with(&["activated", "abilities", "of"]) {
        return Ok(None);
    }

    let Some(cant_idx) = normalized.iter().position(|word| *word == "cant") else {
        return Ok(None);
    };
    if cant_idx <= 3 {
        return Ok(None);
    }

    let tail = &normalized[cant_idx..];
    if !tail.starts_with(&["cant", "be", "activated"]) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cant_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words: Vec<&str> = words(&subject_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let filter = if subject_words.len() == 3 && subject_words[1] == "and" {
        let t1 = subject_words[0]
            .strip_suffix('s')
            .unwrap_or(subject_words[0]);
        let t2 = subject_words[2]
            .strip_suffix('s')
            .unwrap_or(subject_words[2]);
        if let (Some(ct1), Some(ct2)) = (parse_card_type(t1), parse_card_type(t2)) {
            let mut a = ObjectFilter::default();
            a.zone = Some(Zone::Battlefield);
            a.card_types = vec![ct1];

            let mut b = ObjectFilter::default();
            b.zone = Some(Zone::Battlefield);
            b.card_types = vec![ct2];

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![a, b];
            disjunction
        } else {
            parse_object_filter(&subject_tokens, false)?
        }
    } else {
        parse_object_filter(&subject_tokens, false)?
    };

    let non_mana_only = normalized
        .windows(4)
        .any(|window| window == ["unless", "theyre", "mana", "abilities"]);

    let restriction = if non_mana_only {
        Restriction::activate_non_mana_abilities_of(filter)
    } else {
        Restriction::activate_abilities_of(filter)
    };

    let display_subject = subject_words.join(" ");
    let display = if non_mana_only {
        format!(
            "Activated abilities of {display_subject} can't be activated unless they're mana abilities."
        )
    } else {
        format!("Activated abilities of {display_subject} can't be activated.")
    };

    Ok(Some(StaticAbility::restriction(restriction, display)))
}

pub(crate) fn parse_can_block_additional_creature_each_combat_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens);
    if normalized.as_slice()
        == [
            "this",
            "creature",
            "can",
            "block",
            "an",
            "additional",
            "creature",
            "each",
            "combat",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "can",
                "block",
                "an",
                "additional",
                "creature",
            ]
    {
        return Ok(Some(
            StaticAbility::can_block_additional_creature_each_combat(1),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_ward_static_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("ward") {
        return Ok(None);
    }

    let cost_tokens = trim_commas(&tokens[1..]);
    if cost_tokens.is_empty() {
        return Ok(Some(StaticAbility::custom("ward", "Ward".to_string())));
    }

    if let Some(cost) = parse_ward_discard_card_type_cost(&cost_tokens) {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    if let Ok((cost, _)) = parse_activation_cost(&cost_tokens)
        && !cost.is_free()
    {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    // Preserve ward lines as static marker text rather than lowering the
    // ward cost into spell effects when a cost variant is not yet modeled.
    let marker_tail = format_ward_marker_tail(&cost_tokens);
    let marker = if marker_tail.is_empty() {
        "Ward".to_string()
    } else {
        format!("Ward—{}", marker_tail)
    };
    Ok(Some(StaticAbility::custom("keyword_marker", marker)))
}

pub(crate) fn parse_ward_discard_card_type_cost(tokens: &[Token]) -> Option<TotalCost> {
    let cost_words = words(tokens);
    if cost_words.first().copied() != Some("discard") {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }

    let words_tail = &cost_words[idx..];
    if words_tail.starts_with(&["your", "hand"]) && words_tail.len() == 2 {
        return Some(TotalCost::from_cost(crate::costs::Cost::discard_hand()));
    }

    while cost_words
        .get(idx)
        .is_some_and(|word| *word == "a" || *word == "an")
    {
        idx += 1;
    }

    let mut card_types = Vec::<CardType>::new();
    while let Some(word) = cost_words.get(idx) {
        if *word == "card" || *word == "cards" {
            idx += 1;
            break;
        }
        if *word == "and" || *word == "or" || *word == "a" || *word == "an" {
            idx += 1;
            continue;
        }
        let parsed = parse_card_type(word)?;
        if !card_types.contains(&parsed) {
            card_types.push(parsed);
        }
        idx += 1;
    }

    if idx != cost_words.len() {
        return None;
    }

    let cost = if card_types.len() > 1 {
        crate::costs::Cost::discard_types(count, card_types)
    } else if let Some(card_type) = card_types.first().copied() {
        crate::costs::Cost::discard(count, Some(card_type))
    } else {
        crate::costs::Cost::discard(count, None)
    };
    Some(TotalCost::from_cost(cost))
}

pub(crate) fn format_ward_marker_tail(tokens: &[Token]) -> String {
    let mut parts = Vec::new();
    let mut previous_word: Option<String> = None;
    for word in words(tokens) {
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            let should_brace = matches!(previous_word.as_deref(), Some("waterbend"));
            if should_brace {
                parts.push(format!("{{{word}}}"));
            } else {
                parts.push(word.to_string());
            }
            previous_word = Some(word.to_string());
            continue;
        }
        if let Ok(symbol) = parse_mana_symbol(word) {
            parts.push(ManaCost::from_symbols(vec![symbol]).to_oracle());
            previous_word = Some(word.to_string());
            continue;
        }
        parts.push(word.to_string());
        previous_word = Some(word.to_string());
    }

    if let Some(first) = parts.first_mut()
        && !first.starts_with('{')
    {
        *first = keyword_title(first);
    }

    parts.join(" ")
}

pub(crate) fn parse_composed_anthem_effects_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let comma_segments = split_on_comma(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    let Some(first_action_idx) = tokens.iter().position(|token| {
        token.is_word("get")
            || token.is_word("gets")
            || token.is_word("have")
            || token.is_word("has")
    }) else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    if subject_tokens.is_empty() || parse_anthem_subject(&subject_tokens).is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let mut segment = trim_commas(&raw_segment).to_vec();
        if segment.is_empty() {
            continue;
        }

        if segment.first().is_some_and(|token| token.is_word("and")) {
            let trimmed = trim_commas(&segment[1..]);
            if trimmed.first().is_some_and(|token| {
                token.is_word("get")
                    || token.is_word("gets")
                    || token.is_word("have")
                    || token.is_word("has")
            }) {
                segment = trimmed.to_vec();
            }
        }

        let starts_with_action = segment.first().is_some_and(|token| {
            token.is_word("get")
                || token.is_word("gets")
                || token.is_word("have")
                || token.is_word("has")
        });
        if starts_with_action {
            if idx > 0 {
                saw_omitted_subject_clause = true;
            }
            let mut expanded = subject_tokens.clone();
            expanded.extend(segment);
            segment = expanded;
        }

        let parsed_segment =
            if let Some(abilities) = parse_anthem_and_type_color_addition_line(&segment)? {
                abilities
            } else if let Some(abilities) = parse_anthem_and_keyword_line(&segment)? {
                abilities
            } else if let Some(abilities) = parse_granted_keyword_static_line(&segment)? {
                abilities
            } else if let Some(ability) = parse_anthem_line(&segment)? {
                vec![ability]
            } else {
                return Ok(None);
            };
        compiled.extend(parsed_segment);
    }

    if !saw_omitted_subject_clause || compiled.len() < 2 {
        return Ok(None);
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_static_text_marker_line(tokens: &[Token]) -> Option<StaticAbility> {
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let is_once_each_turn_play_from_exile = words
        .starts_with(&["once", "each", "turn", "you", "may", "play"])
        && words.contains(&"from")
        && words.contains(&"exile")
        && words.contains(&"cast")
        && words.windows(2).any(|window| window == ["spend", "mana"])
        && words
            .windows(4)
            .any(|window| window == ["as", "though", "it", "were"])
        && words
            .windows(3)
            .any(|window| window == ["any", "color", "to"]);
    if is_once_each_turn_play_from_exile {
        let mut text = "Once each turn, you may play a card from exile".to_string();
        if words
            .windows(3)
            .any(|window| window == ["collection", "counter", "on"])
        {
            text.push_str(" with a collection counter on it");
        }
        if words
            .windows(4)
            .any(|window| window == ["if", "it", "was", "exiled"])
        {
            text.push_str(" if it was exiled by an ability you controlled");
        }
        text.push_str(", and you may spend mana as though it were mana of any color to cast it");
        return Some(StaticAbility::custom(
            "once_each_turn_play_from_exile",
            text,
        ));
    }

    if words == ["you", "have", "shroud"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
            "You have shroud".to_string(),
        ));
    }

    if words == ["creatures", "without", "flying", "cant", "attack"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack(
                ObjectFilter::creature()
                    .without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
            ),
            "Creatures without flying can't attack".to_string(),
        ));
    }

    if words == ["this", "creature", "cant", "attack", "alone"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "This creature can't attack alone".to_string(),
        ));
    }

    if words.len() == 4
        && words[0] == "ward"
        && words[1] == "pay"
        && words[3] == "life"
        && words[2].parse::<u32>().is_ok()
    {
        return Some(StaticAbility::custom(
            "ward_pay_life",
            format!("Ward—Pay {} life", words[2]),
        ));
    }

    if words.starts_with(&[
        "lands",
        "dont",
        "untap",
        "during",
        "their",
        "controllers",
        "untap",
    ]) && matches!(words.last(), Some(&"step") | Some(&"steps"))
    {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::untap(ObjectFilter::land()),
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

pub(crate) fn parse_subject_cant_be_blocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();
    if normalized.len() < 4 || !normalized.ends_with(&["cant", "be", "blocked"]) {
        return Ok(None);
    }

    let tail_start = token_index_for_word_index(tokens, normalized.len() - 3).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked tail (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..tail_start]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .iter()
        .any(|token| matches!(token, Token::Comma(_)) || token.is_word("and"))
    {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words
        .first()
        .is_some_and(|word| *word == "this" || *word == "it")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "as" | "long"
                | "if"
                | "when"
                | "whenever"
                | "get"
                | "gets"
                | "gain"
                | "gains"
                | "have"
                | "has"
        )
    }) {
        return Ok(None);
    }
    if subject_words.windows(3).any(|window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbility::unblockable(),
        AnthemSubjectAst::Filter(filter) => {
            StaticAbility::grant_ability(filter, StaticAbility::unblockable())
        }
    };
    Ok(Some(ability))
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let Some(cant_idx) = normalized
        .windows(3)
        .position(|window| window == ["cant", "be", "blocked"])
    else {
        return Ok(None);
    };

    let tail = &normalized[cant_idx + 3..];
    if tail.len() < 7 || !tail.starts_with(&["as", "long", "as", "defending", "player", "controls"])
    {
        return Ok(None);
    }

    let mut type_words = &tail[6..];
    if matches!(type_words.first(), Some(&"a" | &"an" | &"the")) {
        type_words = &type_words[1..];
    }
    if type_words.len() != 1 {
        return Ok(None);
    }
    let Some(card_type) = parse_card_type(type_words[0]) else {
        return Ok(None);
    };
    if !matches!(
        card_type,
        CardType::Artifact
            | CardType::Battle
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
    ) {
        return Ok(None);
    }

    let tail_start = token_index_for_word_index(tokens, cant_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked conditional tail (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..tail_start]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let unblockable =
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(card_type);
    let ability = match subject {
        AnthemSubjectAst::Source => unblockable,
        AnthemSubjectAst::Filter(filter) => StaticAbility::grant_ability(filter, unblockable),
    };
    Ok(Some(ability))
}

pub(crate) fn parse_enters_tapped_with_choose_color_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("this")
        || !clause_words.contains(&"enters")
        || !clause_words.contains(&"tapped")
    {
        return Ok(None);
    }
    let tapped_word_idx = clause_words
        .iter()
        .position(|word| *word == "tapped")
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let tapped_token_idx =
        token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let trailing = &tokens[tapped_token_idx + 1..];
    if trailing.is_empty() {
        return Ok(None);
    }
    let Some(color_choice) = parse_choose_color_as_enters_line(trailing)? else {
        return Ok(None);
    };
    Ok(Some(vec![
        StaticAbility::enters_tapped_ability(),
        color_choice,
    ]))
}

pub(crate) fn parse_damage_not_removed_cleanup_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() != 9 {
        return Ok(None);
    }
    if words[0] != "damage" || words[2] != "removed" {
        return Ok(None);
    }
    let is_not = words[1] == "isnt" || words[1] == "isn't";
    let matches = is_not
        && words[3] == "from"
        && words[4] == "this"
        && words[5] == "creature"
        && words[6] == "during"
        && words[7] == "cleanup"
        && words[8] == "steps";
    if matches {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

pub(crate) fn parse_choose_color_as_enters_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 || words[0] != "as" {
        return Ok(None);
    }

    let mut idx = 1;
    let subject = if words.get(idx) == Some(&"this") {
        idx += 1;
        if words.get(idx) == Some(&"land") {
            idx += 1;
            "this land"
        } else {
            "this"
        }
    } else if words.get(idx) == Some(&"it") {
        idx += 1;
        "it"
    } else {
        return Ok(None);
    };

    if words.get(idx) != Some(&"enters") {
        return Ok(None);
    }
    idx += 1;

    if words.get(idx) == Some(&"the") && words.get(idx + 1) == Some(&"battlefield") {
        idx += 2;
    }

    if words.get(idx) != Some(&"choose") {
        return Ok(None);
    }
    idx += 1;
    if words.get(idx) == Some(&"a") {
        idx += 1;
    }
    if words.get(idx) != Some(&"color") {
        return Ok(None);
    }
    idx += 1;

    let mut excluded = None;
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let Some(color_word) = words.get(idx + 2) else {
            return Ok(None);
        };
        let color_set = parse_color(color_word).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported color choice '{}' (clause: '{}')",
                color_word,
                words.join(" ")
            ))
        })?;
        let color = color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice '{}' (clause: '{}')",
                color_word,
                words.join(" ")
            ))
        })?;
        excluded = Some(color);
        idx += 3;
    }

    if idx != words.len() {
        return Ok(None);
    }

    let display = if subject == "this land" {
        match excluded {
            Some(color) => format!(
                "As this land enters, choose a color other than {}.",
                format!("{color:?}").to_ascii_lowercase()
            ),
            None => "As this land enters, choose a color.".to_string(),
        }
    } else {
        match excluded {
            Some(color) => format!(
                "As it enters, choose a color other than {}.",
                format!("{color:?}").to_ascii_lowercase()
            ),
            None => "As it enters, choose a color.".to_string(),
        }
    };

    Ok(Some(StaticAbility::choose_color_as_enters(
        excluded, display,
    )))
}

pub(crate) fn parse_damage_redirect_to_source_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() != 19 {
        return Ok(None);
    }
    let matches = words[0] == "all"
        && words[1] == "damage"
        && words[2] == "that"
        && words[3] == "would"
        && words[4] == "be"
        && words[5] == "dealt"
        && words[6] == "to"
        && words[7] == "you"
        && words[8] == "and"
        && words[9] == "other"
        && (words[10] == "permanents" || words[10] == "permanent")
        && words[11] == "you"
        && words[12] == "control"
        && words[13] == "is"
        && words[14] == "dealt"
        && words[15] == "to"
        && words[16] == "this"
        && words[17] == "creature"
        && words[18] == "instead";
    if matches {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 8 || !line_words.starts_with(&["no", "more", "than"]) {
        return Ok(None);
    }

    let Some((maximum, used)) = parse_number(&tokens[3..]) else {
        return Ok(None);
    };

    let tail = words(&tokens[3 + used..]);
    if tail.len() != 5 {
        return Ok(None);
    }

    if !matches!(tail[0], "creature" | "creatures")
        || tail[1] != "can"
        || tail[3] != "each"
        || tail[4] != "combat"
    {
        return Ok(None);
    }

    let ability = match tail[2] {
        "attack" => StaticAbility::max_attackers_each_combat(maximum as usize),
        "block" => StaticAbility::max_blockers_each_combat(maximum as usize),
        _ => return Ok(None),
    };
    Ok(Some(ability))
}

pub(crate) fn parse_characteristic_defining_pt_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    let has_this_pt = line_words.windows(4).any(|window| {
        window == ["this", "power", "and", "toughness"]
            || window == ["thiss", "power", "and", "toughness"]
    });
    if !has_this_pt {
        return Ok(None);
    }
    if !(line_words.contains(&"equal")
        && line_words.contains(&"number")
        && line_words.contains(&"of"))
    {
        return Ok(None);
    }

    let mut tail_tokens = tokens;
    if let Some(equal_word_idx) = line_words
        .windows(2)
        .position(|window| window == ["equal", "to"])
    {
        let start_word_idx = equal_word_idx + 2;
        if let Some(start_token_idx) = token_index_for_word_index(tokens, start_word_idx) {
            tail_tokens = &tokens[start_token_idx..];
        }
    }
    while tail_tokens
        .last()
        .is_some_and(|token| token.is_word("respectively") || matches!(token, Token::Period(_)))
    {
        tail_tokens = &tail_tokens[..tail_tokens.len().saturating_sub(1)];
    }
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let value = parse_characteristic_defining_pt_value(tail_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported characteristic defining P/T value (value: '{}')",
            words(tail_tokens).join(" ")
        ))
    })?;
    Ok(Some(StaticAbility::characteristic_defining_pt(
        value.clone(),
        value,
    )))
}

pub(crate) fn parse_characteristic_defining_pt_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "plus").then_some(idx))
        .collect();
    if plus_positions.is_empty() {
        return parse_characteristic_defining_pt_term(tokens);
    }

    let mut values = Vec::new();
    let mut start_word_idx = 0usize;
    for plus_word_idx in plus_positions {
        let start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
        let end_token_idx = token_index_for_word_index(tokens, plus_word_idx)?;
        values.push(parse_characteristic_defining_pt_term(
            &tokens[start_token_idx..end_token_idx],
        )?);
        start_word_idx = plus_word_idx + 1;
    }
    let final_start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
    values.push(parse_characteristic_defining_pt_term(
        &tokens[final_start_token_idx..],
    )?);

    let mut iter = values.into_iter();
    let mut acc = iter.next()?;
    for value in iter {
        acc = Value::Add(Box::new(acc), Box::new(value));
    }
    Some(acc)
}

pub(crate) fn parse_characteristic_defining_pt_term(tokens: &[Token]) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    if let Some((number, used)) = parse_number(tokens) {
        if tokens.len() == used {
            return Some(Value::Fixed(number as i32));
        }
    }

    let mut start = tokens;
    while start
        .first()
        .is_some_and(|token| token.as_word().is_some_and(is_article))
    {
        start = &start[1..];
    }
    if start.is_empty() {
        return None;
    }

    if start.first().is_some_and(|token| token.is_word("number"))
        && start.get(1).is_some_and(|token| token.is_word("of"))
    {
        start = &start[2..];
    }
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    let start_words = words(start);
    if start_words.as_slice()
        == [
            "cards", "in", "the", "hand", "of", "the", "opponent", "with", "the", "most", "cards",
            "in", "hand",
        ]
        || start_words.as_slice()
            == [
                "cards", "in", "the", "hand", "of", "an", "opponent", "with", "the", "most",
                "cards", "in", "hand",
            ]
    {
        return Some(Value::MaxCardsInHand(PlayerFilter::Opponent));
    }
    if start_words.as_slice()
        == [
            "cards", "in", "the", "hand", "of", "the", "player", "with", "the", "most", "cards",
            "in", "hand",
        ]
    {
        return Some(Value::MaxCardsInHand(PlayerFilter::Any));
    }

    let filter = parse_object_filter(start, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_would_be_put = words
        .windows(3)
        .any(|window| window == ["would", "be", "put"]);
    let has_graveyard = words.contains(&"graveyard");
    let has_anywhere = words.contains(&"anywhere");
    let has_shuffle = words.contains(&"shuffle");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");

    if has_would_be_put
        && has_graveyard
        && has_anywhere
        && has_shuffle
        && has_library
        && has_instead
    {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

pub(crate) fn parse_permanents_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["permanents", "enter", "tapped"]
        || words.as_slice() == ["permanents", "enters", "tapped"]
    {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

pub(crate) fn parse_players_cant_cycle_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "cant", "cycle", "cards"] {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

pub(crate) fn parse_starting_life_bonus_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 || !words.starts_with(&["you", "start", "the", "game"]) {
        return Ok(None);
    }
    if !words.contains(&"additional") || !words.contains(&"life") {
        return Ok(None);
    }
    let mut amount = None;
    for (idx, _token) in tokens.iter().enumerate() {
        if let Some((value, _)) = parse_number(&tokens[idx..]) {
            amount = Some(value);
            break;
        }
    }
    let amount = amount
        .ok_or_else(|| CardTextError::ParseError("missing starting life amount".to_string()))?;
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

pub(crate) fn parse_buyback_cost_reduction_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 5 || !words.starts_with(&["buyback", "costs", "cost"]) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !words.contains(&"less") {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub(crate) fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }
    if !words.contains(&"more") || !words.contains(&"target") || !words.contains(&"beyond") {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let (amount_value, _) =
        parse_cost_modifier_amount(amount_tokens).unwrap_or((Value::Fixed(1), 0));
    let amount = if let Value::Fixed(v) = amount_value {
        v.max(0) as u32
    } else {
        1
    };

    Ok(Some(StaticAbility::cost_increase_per_target_beyond_first(
        amount,
    )))
}

pub(crate) fn parse_if_this_spell_costs_less_to_cast_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["if"]) {
        return Ok(None);
    }

    // Expected shape:
    // "If <condition>, this spell costs {N} less to cast."
    let Some(comma_idx) = tokens.iter().position(|t| matches!(t, Token::Comma(_))) else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let condition = parse_this_spell_cost_condition(&condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let costs_idx = tail_tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .unwrap_or_else(|| (Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_words = words(amount_tokens.get(used..).unwrap_or_default());
    if !remaining_words.contains(&"less") || !remaining_words.contains(&"cast") {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

pub(crate) fn parse_this_spell_target_condition(
    tokens: &[Token],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = words(tokens);
    let target_start = if w.starts_with(&["it", "targets"]) {
        2
    } else if w.starts_with(&["this", "spell", "targets"]) {
        3
    } else {
        return None;
    };
    let target_tokens = trim_commas(tokens.get(target_start..).unwrap_or_default());
    if target_tokens.is_empty() {
        return None;
    }
    let target_words = words(&target_tokens);
    if target_words.starts_with(&["you"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You));
    }
    if target_words.starts_with(&["an", "opponent"]) || target_words.starts_with(&["opponent"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Opponent));
    }
    if target_words.starts_with(&["a", "player"]) || target_words.starts_with(&["player"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Any));
    }
    parse_object_filter(&target_tokens, false)
        .ok()
        .map(ThisSpellCostCondition::TargetsObject)
}

pub(crate) fn parse_this_spell_cost_condition(
    tokens: &[Token],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = words(tokens);
    if w.is_empty() {
        return None;
    }

    // you have 3 or less life
    if w.len() >= 6 && w[0] == "you" && w[1] == "have" && w.contains(&"life") {
        if let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default()) {
            if w[3] == "or" && w[4] == "less" && w[5] == "life" {
                return Some(ThisSpellCostCondition::YouLifeTotalOrLess(n as i32));
            }
        }
    }
    // your life total is 5 or less
    if w.len() >= 7
        && w[0] == "your"
        && w[1] == "life"
        && w[2] == "total"
        && w[3] == "is"
        && w[w.len().saturating_sub(2)..] == ["or", "less"]
        && let Some((n, _)) = parse_number(tokens.get(4..).unwrap_or_default())
    {
        return Some(ThisSpellCostCondition::YouLifeTotalOrLess(n as i32));
    }
    if w.as_slice()
        == [
            "your", "life", "total", "is", "less", "than", "your", "starting", "life", "total",
        ]
    {
        return Some(ThisSpellCostCondition::LifeTotalLessThanStarting);
    }

    if w.as_slice() == ["you", "attacked", "this", "turn"]
        || w.as_slice() == ["youve", "attacked", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::AttackedThisTurn,
            display: w.join(" "),
        });
    }
    if w.as_slice() == ["a", "creature", "died", "this", "turn"]
        || w.as_slice() == ["creature", "died", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::CreatureDiedThisTurn,
            display: w.join(" "),
        });
    }
    if w.as_slice() == ["you", "gained", "life", "this", "turn"]
        || w.as_slice() == ["youve", "gained", "life", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(1));
    }
    if (w.starts_with(&["youve", "gained"]) || w.starts_with(&["you", "gained"]))
        && w.len() >= 7
        && w[w.len() - 3..] == ["life", "this", "turn"]
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
        && w.get(3) == Some(&"or")
        && w.get(4) == Some(&"more")
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n));
    }
    if w.as_slice() == ["its", "night"] || w.as_slice() == ["it", "is", "night"] {
        return Some(ThisSpellCostCondition::IsNight);
    }
    if w.as_slice() == ["youve", "sacrificed", "an", "artifact", "this", "turn"]
        || w.as_slice() == ["you", "sacrificed", "an", "artifact", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::YouSacrificedArtifactThisTurn);
    }
    if w.as_slice() == ["youve", "committed", "a", "crime", "this", "turn"]
        || w.as_slice() == ["you", "committed", "a", "crime", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::YouCommittedCrimeThisTurn);
    }
    if w.as_slice()
        == [
            "a",
            "creature",
            "left",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
    {
        return Some(ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn);
    }
    if (w.starts_with(&["youve", "cast", "another"])
        || w.starts_with(&["you", "cast", "another"]))
        && w.ends_with(&["this", "turn"])
    {
        if w.contains(&"instant") || w.contains(&"sorcery") {
            let mut types = Vec::new();
            if w.contains(&"instant") {
                types.push(CardType::Instant);
            }
            if w.contains(&"sorcery") {
                types.push(CardType::Sorcery);
            }
            return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: types,
            });
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: Vec::new(),
        });
    }
    if (w.starts_with(&["youve", "cast"]) || w.starts_with(&["you", "cast"]))
        && w.ends_with(&["this", "turn"])
        && (w.contains(&"instant") || w.contains(&"sorcery"))
    {
        let mut types = Vec::new();
        if w.contains(&"instant") {
            types.push(CardType::Instant);
        }
        if w.contains(&"sorcery") {
            types.push(CardType::Sorcery);
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: types,
        });
    }

    if w.as_slice() == ["you", "werent", "the", "starting", "player"] {
        return Some(ThisSpellCostCondition::NotStartingPlayer);
    }
    if w.as_slice() == ["a", "creature", "is", "attacking", "you"] {
        return Some(ThisSpellCostCondition::CreatureIsAttackingYou);
    }
    if w.as_slice()
        == [
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn",
        ]
    {
        return Some(ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn);
    }
    if w.len() >= 11
        && w[0] == "there"
        && w[1] == "are"
        && w.contains(&"card")
        && w.contains(&"types")
        && w.contains(&"graveyard")
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        return Some(ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(
            n,
        ));
    }
    if w.starts_with(&["you", "have"])
        && w.ends_with(&["in", "your", "graveyard"])
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        if w.contains(&"instant") || w.contains(&"sorcery") {
            let mut types = Vec::new();
            if w.contains(&"instant") {
                types.push(CardType::Instant);
            }
            if w.contains(&"sorcery") {
                types.push(CardType::Sorcery);
            }
            return Some(ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore {
                count: n,
                card_types: types,
            });
        }
        return Some(ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(n));
    }
    if w.len() >= 7
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "has")
            || (w[0] == "opponent" && w[1] == "has"))
    {
        let count_start = if w[0] == "an" { 3 } else { 2 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if tail == ["or", "more", "poison", "counters"]
                || tail == ["or", "more", "poison", "counter"]
            {
                return Some(ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n));
            }
            if tail == ["or", "more", "cards", "in", "their", "graveyard"]
                || tail == ["or", "more", "cards", "in", "his", "graveyard"]
                || tail == ["or", "more", "cards", "in", "her", "graveyard"]
                || tail == ["or", "more", "card", "in", "their", "graveyard"]
            {
                return Some(ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n));
            }
        }
    }

    if w.starts_with(&["there", "are", "no"]) && w.ends_with(&["in", "your", "hand"]) {
        let filter_tokens = trim_commas(tokens.get(3..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: w.join(" "),
            });
        }
    }
    if ((w.starts_with(&["you", "have", "no", "other", "creature", "cards"])
        && w.windows(2).any(|window| window == ["or", "if"]))
        || w.starts_with(&[
            "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
        ]))
        && let Some(named_idx) = w.iter().position(|word| *word == "named")
        && named_idx + 1 < w.len()
    {
        let name = w[named_idx + 1..].join(" ");
        if !name.is_empty() {
            return Some(ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name));
        }
    }

    if w.starts_with(&["there", "is"]) && w.ends_with(&["in", "your", "graveyard"]) {
        let filter_tokens = trim_commas(tokens.get(2..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::CardInYourGraveyardMatching {
                filter,
                display: w.join(" "),
            });
        }
    }

    if w.as_slice()
        == [
            "it",
            "targets",
            "a",
            "spell",
            "or",
            "ability",
            "that",
            "targets",
            "a",
            "creature",
            "you",
            "control",
            "with",
            "power",
            "7",
            "or",
            "greater",
        ]
    {
        let mut protected = ObjectFilter::creature().you_control();
        protected.power = Some(crate::filter::Comparison::GreaterThanOrEqual(7));
        let mut stack_target = ObjectFilter::default();
        stack_target.zone = Some(Zone::Stack);
        stack_target.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        stack_target.targets_object = Some(Box::new(protected));
        return Some(ThisSpellCostCondition::TargetsObject(stack_target));
    }

    if let Some(target_condition) = parse_this_spell_target_condition(tokens) {
        return Some(target_condition);
    }

    // an opponent has no cards in hand
    if w.as_slice() == ["an", "opponent", "has", "no", "cards", "in", "hand"]
        || w.as_slice() == ["opponent", "has", "no", "cards", "in", "hand"]
    {
        return Some(ThisSpellCostCondition::OpponentHasNoCardsInHand);
    }

    // an opponent controls seven or more lands
    if w.len() >= 7 && w[0] == "an" && w[1] == "opponent" && w[2] == "controls" {
        if let Some((n, _)) = parse_number(tokens.get(3..).unwrap_or_default()) {
            let tail = &w[4..];
            if tail == ["or", "more", "lands"] || tail == ["or", "more", "land"] {
                return Some(ThisSpellCostCondition::OpponentControlsLandsOrMore(n));
            }
        }
    }

    // an opponent controls at least four more creatures than you
    if w.len() >= 10
        && w[0] == "an"
        && w[1] == "opponent"
        && w[2] == "controls"
        && w[3] == "at"
        && w[4] == "least"
    {
        if let Some((n, _)) = parse_number(tokens.get(5..).unwrap_or_default()) {
            let tail = &w[6..];
            if tail == ["more", "creatures", "than", "you"]
                || tail == ["more", "creature", "than", "you"]
            {
                return Some(
                    ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n),
                );
            }
        }
    }

    // there are ten or more creature cards total in all graveyards
    if w.len() >= 12 && w[0] == "there" && w[1] == "are" {
        if let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default()) {
            let tail = &w[3..];
            if tail
                == [
                    "or",
                    "more",
                    "creature",
                    "cards",
                    "total",
                    "in",
                    "all",
                    "graveyards",
                ]
            {
                return Some(ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n));
            }
        }
    }

    // an opponent cast two or more spells this turn
    if w.len() >= 9
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "cast")
            || (w[0] == "opponent" && w[1] == "cast"))
    {
        let count_start = if w[0] == "an" { 3 } else { 2 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if tail == ["or", "more", "spells", "this", "turn"]
                || tail == ["or", "more", "spell", "this", "turn"]
            {
                return Some(ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n));
            }
        }
    }

    // an opponent has drawn four or more cards this turn
    if w.len() >= 10
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "has" && w[3] == "drawn")
            || (w[0] == "opponent" && w[1] == "has" && w[2] == "drawn"))
    {
        let count_start = if w[0] == "an" { 4 } else { 3 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if tail == ["or", "more", "cards", "this", "turn"]
                || tail == ["or", "more", "card", "this", "turn"]
            {
                return Some(ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n));
            }
        }
    }

    if let Ok(condition_expr) = parse_static_condition_clause(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: w.join(" "),
        });
    }

    None
}

pub(crate) fn parse_trailing_this_spell_cost_condition(
    remaining_tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<crate::static_abilities::ThisSpellCostCondition>, CardTextError> {
    let remaining_words = words(remaining_tokens);
    let Some(if_idx) = remaining_words.iter().position(|word| *word == "if") else {
        return Ok(None);
    };
    let condition_token_idx = token_index_for_word_index(remaining_tokens, if_idx + 1).ok_or_else(
        || {
            CardTextError::ParseError(format!(
                "unable to map this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            ))
        },
    )?;
    let condition_tokens = trim_commas(&remaining_tokens[condition_token_idx..]);
    if condition_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    Ok(Some(condition))
}

pub(crate) fn parse_cost_modifier_prefix_condition(
    tokens: &[Token],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let subject_end = spells_token_idx.min(tokens.len());
    let head_tokens = &tokens[..subject_end];

    if words_start_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }

    if words_start_with(tokens, &["during", "your", "turn"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = match parse_static_condition_clause(&condition_tokens) {
            Ok(condition) => condition,
            Err(_) => {
                let condition_words = words(&condition_tokens);
                match condition_words.as_slice() {
                    ["this", "creature", "is", "tapped"]
                    | ["this", "permanent", "is", "tapped"]
                    | ["it", "is", "tapped"] => crate::ConditionExpr::SourceIsTapped,
                    ["this", "creature", "is", "untapped"]
                    | ["this", "permanent", "is", "untapped"]
                    | ["it", "is", "untapped"] => crate::ConditionExpr::SourceIsUntapped,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported static condition clause (clause: '{}')",
                            condition_words.join(" ")
                        )));
                    }
                }
            }
        };
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

pub(crate) fn parse_spells_cost_modifier_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }
    if clause_words.contains(&"first")
        && clause_words.contains(&"each")
        && clause_words.contains(&"turn")
    {
        return Ok(Some(StaticAbility::custom(
            "first_spell_cost_modifier",
            clause_words.join(" "),
        )));
    }

    let Some(spells_token_idx) = tokens
        .iter()
        .position(|token| token.is_word("spell") || token.is_word("spells"))
    else {
        return Ok(None);
    };

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let subject_words = words(&subject_tokens);
    let is_this_spell =
        subject_words.as_slice() == ["this"] || subject_words.as_slice() == ["thiss"];

    let mut cost_token_idx = None;
    for idx in spells_token_idx + 1..tokens.len() {
        if !tokens[idx].is_word("cost") && !tokens[idx].is_word("costs") {
            continue;
        }
        let amount_tokens = &tokens[idx + 1..];
        let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
        if parsed_amount.is_some() || parsed_mana_cost.is_some() {
            cost_token_idx = Some(idx);
            break;
        }
    }
    let Some(cost_token_idx) = cost_token_idx else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        crate::ability::SpellFilter::default()
    } else {
        parse_spell_filter(&subject_tokens)
    };

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    let between_words = words(between_tokens);
    if !is_this_spell {
        for (idx, token) in between_tokens.iter().enumerate() {
            if !token.is_word("spell") && !token.is_word("spells") {
                continue;
            }
            let mut start = idx;
            while start > 0 {
                if between_tokens[start - 1].is_word("and")
                    || between_tokens[start - 1].is_word("or")
                    || matches!(between_tokens[start - 1], Token::Comma(_))
                {
                    break;
                }
                start -= 1;
            }
            let descriptor_tokens = trim_commas(&between_tokens[start..idx]);
            if descriptor_tokens.is_empty() {
                continue;
            }
            let extra_filter = parse_spell_filter(&descriptor_tokens);
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        let between_filter = parse_spell_filter(between_tokens);
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        if between_words.windows(2).any(|window| window == ["you", "cast"]) {
            filter.controller = Some(PlayerFilter::You);
        }
        if between_words
            .iter()
            .any(|word| *word == "opponent" || *word == "opponents")
            && between_words
                .iter()
                .any(|word| *word == "cast" || *word == "casts")
        {
            filter.controller = Some(PlayerFilter::Opponent);
        }
        let mut target_player: Option<PlayerFilter> = None;
        let mut target_object: Option<ObjectFilter> = None;
        let mut targets_idx = None;
        for (idx, token) in between_tokens.iter().enumerate() {
            if token.is_word("target") || token.is_word("targets") {
                if idx > 0 && between_tokens[idx - 1].is_word("that") {
                    targets_idx = Some(idx);
                    break;
                }
            }
        }
        if let Some(targets_idx) = targets_idx {
            let target_tokens = &between_tokens[targets_idx + 1..];
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target in spells-cost modifier clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let target_words = words(target_tokens);
            if target_words.starts_with(&["you"]) {
                target_player = Some(PlayerFilter::You);
            } else if target_words.starts_with(&["opponent"])
                || target_words.starts_with(&["opponents"])
            {
                target_player = Some(PlayerFilter::Opponent);
            } else if target_words.starts_with(&["player"])
                || target_words.starts_with(&["players"])
            {
                target_player = Some(PlayerFilter::Any);
            } else {
                target_object = Some(parse_object_filter(target_tokens, false)?);
            }
            filter.targets_player = target_player;
            filter.targets_object = target_object;
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or_else(|| {
            if let Some((_, used)) = &parsed_mana_cost {
                (Value::Fixed(1), *used)
            } else {
                (Value::Fixed(1), 0)
            }
        });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }

    if let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        if parsed_mana_cost.is_some() {
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic mana-symbol cost modifier (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        amount_value = dynamic_value;
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if remaining_words
        .windows(3)
        .any(|window| window == ["where", "x", "is"])
    {
        let clause = clause_words.join(" ");
        let where_word_idx = remaining_words
            .windows(3)
            .position(|window| window == ["where", "x", "is"])
            .unwrap_or(0);
        let where_token_idx = token_index_for_word_index(remaining_tokens, where_word_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in spells-cost modifier (clause: '{clause}')"
                ))
            })?;
        let where_tokens = trim_commas(&remaining_tokens[where_token_idx..]);
        let x_value = parse_where_x_value_clause(&where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                crate::ConditionExpr::YourTurn => {
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                }
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
                {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };

    if is_less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell
            && let Some((cost, _)) = parsed_mana_cost.clone()
        {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReductionManaCost::new(
                    cost,
                    this_spell_condition,
                ),
            )));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost);
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(
                ability,
            )));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(
            ability,
        )));
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(
            ability,
        )));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if let Some(condition) = non_this_condition.clone() {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(
        ability,
    )))
}

pub(crate) fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut crate::ability::SpellFilter,
    remaining_tokens: &[Token],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = words(remaining_tokens);
    let Some(if_word_idx) = remaining_words.iter().position(|word| *word == "if") else {
        return Ok(());
    };
    let condition_words = &remaining_words[if_word_idx..];
    if condition_words.len() < 4
        || condition_words[0] != "if"
        || condition_words[1] != "it"
        || (condition_words[2] != "targets" && condition_words[2] != "target")
    {
        return Ok(());
    }

    let target_word_idx = if_word_idx + 3;
    let target_token_idx = token_index_for_word_index(remaining_tokens, target_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map trailing target condition in spells-cost modifier (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = &remaining_tokens[target_token_idx..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(target_tokens);
    if target_words.starts_with(&["you"]) {
        filter.targets_player = Some(PlayerFilter::You);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["opponent"]) || target_words.starts_with(&["opponents"]) {
        filter.targets_player = Some(PlayerFilter::Opponent);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
        filter.targets_player = Some(PlayerFilter::Any);
        filter.targets_object = None;
        return Ok(());
    }

    filter.targets_object = Some(parse_object_filter(target_tokens, false)?);
    filter.targets_player = None;
    Ok(())
}

pub(crate) fn parse_flashback_cost_modifier_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 || clause_words.first().copied() != Some("flashback") {
        return Ok(None);
    }
    let cost_idx = tokens
        .iter()
        .rposition(|token| token.is_word("cost") || token.is_word("costs"));
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = crate::ability::SpellFilter::default();
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);
    if clause_words
        .windows(2)
        .any(|window| window == ["you", "pay"])
    {
        filter.controller = Some(PlayerFilter::You);
    } else if clause_words
        .windows(3)
        .any(|window| window == ["your", "opponents", "pay"])
        || clause_words
            .windows(2)
            .any(|window| window == ["opponents", "pay"] || window == ["opponent", "pays"])
    {
        filter.controller = Some(PlayerFilter::Opponent);
    }

    if is_less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub(crate) fn parse_cost_modifier_amount(tokens: &[Token]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let word = tokens.first().and_then(Token::as_word)?;
    let symbol = parse_mana_symbol(word).ok()?;
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub(crate) fn parse_cost_modifier_mana_cost(tokens: &[Token]) -> Option<(crate::mana::ManaCost, usize)> {
    use crate::mana::{ManaCost, ManaSymbol};

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut used = 0usize;
    while let Some(word) = tokens.get(used).and_then(Token::as_word) {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        match symbol {
            ManaSymbol::X | ManaSymbol::Snow | ManaSymbol::Life(_) => {
                break;
            }
            _ => {
                pips.push(vec![symbol]);
                used += 1;
            }
        }
    }
    if used == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips), used))
}

pub(crate) fn parse_cost_modifier_components(
    amount_tokens: &[Token],
) -> (Option<(Value, usize)>, Option<(crate::mana::ManaCost, usize)>) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

pub(crate) fn parse_dynamic_cost_modifier_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let words_all = words(tokens);
    let Some(each_idx) = words_all.iter().position(|word| *word == "each") else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = words(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }
    if filter_words.starts_with(&["creature", "that", "died", "this", "turn"])
        || filter_words.starts_with(&["creatures", "that", "died", "this", "turn"])
    {
        return Ok(Some(Value::CreaturesDiedThisTurn));
    }
    if filter_words.starts_with(&["creature", "that", "died", "under", "your", "control"])
        || filter_words.starts_with(&["creatures", "that", "died", "under", "your", "control"])
    {
        if filter_words.contains(&"this") && filter_words.contains(&"turn") {
            return Ok(Some(Value::CreaturesDiedThisTurnControlledBy(
                PlayerFilter::You,
            )));
        }
    }
    // "for each spell you've cast this turn" (and limited variants like "instant and sorcery spell")
    let has_spell_cast_turn = (filter_words.contains(&"spell") || filter_words.contains(&"spells"))
        && (filter_words.contains(&"cast") || filter_words.contains(&"casts"))
        && filter_words.contains(&"this")
        && filter_words.contains(&"turn");
    if has_spell_cast_turn {
        let player = if filter_words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
        {
            PlayerFilter::You
        } else if filter_words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let other_than_first = filter_words
            .windows(4)
            .any(|window| window == ["other", "than", "the", "first"]);
        if other_than_first {
            return Ok(Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            )));
        }

        let exclude_source = filter_words.contains(&"other");
        let has_instant = filter_words.contains(&"instant");
        let has_sorcery = filter_words.contains(&"sorcery");
        if has_instant || has_sorcery {
            let mut filter = ObjectFilter::spell();
            filter.card_types = if has_instant && has_sorcery {
                vec![CardType::Instant, CardType::Sorcery]
            } else if has_instant {
                vec![CardType::Instant]
            } else {
                vec![CardType::Sorcery]
            };
            return Ok(Some(Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            }));
        }

        let simple = matches!(
            filter_words.as_slice(),
            ["spell", "youve", "cast", "this", "turn"]
                | ["spells", "youve", "cast", "this", "turn"]
                | ["spell", "you", "cast", "this", "turn"]
                | ["spells", "you", "cast", "this", "turn"]
                | ["spell", "your", "cast", "this", "turn"]
                | ["spells", "your", "cast", "this", "turn"]
        );
        if simple {
            return Ok(Some(Value::SpellsCastThisTurn(player)));
        }
    }

    if filter_words.windows(2).any(|pair| pair == ["card", "type"])
        && filter_words.contains(&"graveyard")
    {
        let player = if filter_words
            .windows(2)
            .any(|pair| pair == ["your", "graveyard"])
        {
            PlayerFilter::You
        } else if filter_words
            .windows(2)
            .any(|pair| pair == ["opponents", "graveyard"] || pair == ["opponent", "graveyard"])
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if filter_words.starts_with(&[
        "color", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || filter_words.starts_with(&[
        "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || filter_words
        .starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || filter_words.starts_with(&[
            "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ])
    {
        return Ok(Some(Value::ColorsOfManaSpentToCastThisSpell));
    }
    if filter_words.starts_with(&["creature", "in", "your", "party"])
        || filter_words.starts_with(&["creatures", "in", "your", "party"])
    {
        return Ok(Some(Value::PartySize(PlayerFilter::You)));
    }
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let lands_tokens = &filter_tokens[4..];
        if let Ok(filter) = parse_object_filter(lands_tokens, false) {
            return Ok(Some(Value::BasicLandTypesAmong(filter)));
        }
    }

    // "for each <counter> counter removed this way" (storage lands, mana batteries, etc.)
    // The remove-counters cost plumbs the removed total through `CostContext.x_value`,
    // so model the dynamic amount as `X`.
    if (filter_words.contains(&"counter") || filter_words.contains(&"counters"))
        && filter_words.contains(&"removed")
        && filter_words.windows(2).any(|pair| pair == ["this", "way"])
    {
        return Ok(Some(Value::X));
    }

    let mut source_counter_words = filter_words.as_slice();
    if source_counter_words
        .first()
        .is_some_and(|word| is_article(word) || *word == "one" || *word == "another")
    {
        source_counter_words = &source_counter_words[1..];
    }
    let source_counter_match = if source_counter_words.len() >= 3
        && (source_counter_words[0] == "counter" || source_counter_words[0] == "counters")
        && source_counter_words[1] == "on"
    {
        Some((None, 1usize))
    } else if source_counter_words.len() >= 4
        && parse_counter_type_word(source_counter_words[0]).is_some()
        && (source_counter_words[1] == "counter" || source_counter_words[1] == "counters")
        && source_counter_words[2] == "on"
    {
        Some((parse_counter_type_word(source_counter_words[0]), 2usize))
    } else {
        None
    };
    if let Some((counter_type, on_idx)) = source_counter_match {
        let tail = &source_counter_words[on_idx + 1..];
        let on_source = tail.starts_with(&["it"])
            || tail.starts_with(&["this"])
            || tail.starts_with(&["that", "object"])
            || tail.starts_with(&["that", "permanent"]);
        if on_source {
            return Ok(Some(match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            }));
        }
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_add_mana_equal_amount_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    let equal_idx = words_all
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let tail = &words_all[equal_idx + 2..];
    if tail.is_empty() {
        return None;
    }

    let parse_power_or_toughness_segment = |segment: &[&str]| -> Option<Value> {
        let tagged_it_power = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
        let tagged_it_toughness =
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));

        if segment == ["this", "creature", "power"]
            || segment == ["this", "creatures", "power"]
            || segment == ["its", "power"]
        {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Source)));
        }
        if segment == ["this", "creature", "toughness"]
            || segment == ["this", "creatures", "toughness"]
            || segment == ["its", "toughness"]
        {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Source)));
        }
        if segment == ["that", "creature", "power"]
            || segment == ["that", "creatures", "power"]
            || segment == ["that", "objects", "power"]
        {
            return Some(tagged_it_power.clone());
        }
        if segment == ["that", "creature", "toughness"]
            || segment == ["that", "creatures", "toughness"]
            || segment == ["that", "objects", "toughness"]
        {
            return Some(tagged_it_toughness.clone());
        }
        if segment == ["the", "sacrificed", "creature", "power"]
            || segment == ["the", "sacrificed", "creatures", "power"]
            || segment == ["sacrificed", "creature", "power"]
            || segment == ["sacrificed", "creatures", "power"]
        {
            return Some(tagged_it_power);
        }
        if segment == ["the", "sacrificed", "creature", "toughness"]
            || segment == ["the", "sacrificed", "creatures", "toughness"]
            || segment == ["sacrificed", "creature", "toughness"]
            || segment == ["sacrificed", "creatures", "toughness"]
        {
            return Some(tagged_it_toughness);
        }
        None
    };

    if let Some(plus_idx) = tail.iter().position(|word| *word == "plus")
        && plus_idx > 0
        && plus_idx + 1 < tail.len()
        && let Some(left) = parse_power_or_toughness_segment(&tail[..plus_idx])
        && let Some(right) = parse_power_or_toughness_segment(&tail[plus_idx + 1..])
    {
        return Some(Value::Add(Box::new(left), Box::new(right)));
    }

    if tail.starts_with(&["this", "creature", "power"])
        || tail.starts_with(&["this", "creatures", "power"])
        || tail.starts_with(&["its", "power"])
        || tail.starts_with(&["that", "creature", "power"])
        || tail.starts_with(&["that", "creatures", "power"])
        || tail.starts_with(&["that", "objects", "power"])
        || tail.starts_with(&["the", "sacrificed", "creature", "power"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "power"])
        || tail.starts_with(&["sacrificed", "creature", "power"])
        || tail.starts_with(&["sacrificed", "creatures", "power"])
    {
        let source = if tail[0] == "that" || tail.contains(&"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if tail.starts_with(&["this", "creature", "toughness"])
        || tail.starts_with(&["this", "creatures", "toughness"])
        || tail.starts_with(&["its", "toughness"])
        || tail.starts_with(&["that", "creature", "toughness"])
        || tail.starts_with(&["that", "creatures", "toughness"])
        || tail.starts_with(&["that", "objects", "toughness"])
        || tail.starts_with(&["the", "sacrificed", "creature", "toughness"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "toughness"])
        || tail.starts_with(&["sacrificed", "creature", "toughness"])
        || tail.starts_with(&["sacrificed", "creatures", "toughness"])
    {
        let source = if tail[0] == "that" || tail.contains(&"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if tail.starts_with(&["that", "spell", "mana", "value"])
        || tail.starts_with(&["that", "spells", "mana", "value"])
        || tail.starts_with(&["that", "card", "mana", "value"])
        || tail.starts_with(&["that", "cards", "mana", "value"])
        || tail.starts_with(&["the", "mana", "value", "of", "the", "sacrificed", "creature"])
        || tail.starts_with(&["the", "mana", "value", "of", "the", "sacrificed", "artifact"])
        || tail.starts_with(&["the", "mana", "value", "of", "the", "sacrificed", "permanent"])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "creature"])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "artifact"])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "permanent"])
        || tail.starts_with(&["the", "sacrificed", "creature", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "artifact", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "permanent", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "artifacts", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "permanents", "mana", "value"])
        || tail.starts_with(&["sacrificed", "creature", "mana", "value"])
        || tail.starts_with(&["sacrificed", "artifact", "mana", "value"])
        || tail.starts_with(&["sacrificed", "permanent", "mana", "value"])
        || tail.starts_with(&["sacrificed", "creatures", "mana", "value"])
        || tail.starts_with(&["sacrificed", "artifacts", "mana", "value"])
        || tail.starts_with(&["sacrificed", "permanents", "mana", "value"])
        || tail.starts_with(&["its", "mana", "value"])
    {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if matches!(
        tail,
        ["this", "spell", "mana", "value"]
            | ["this", "creature", "mana", "value"]
            | ["this", "permanent", "mana", "value"]
            | ["this", "card", "mana", "value"]
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
    }

    None
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    if words_all.starts_with(&["that", "much"]) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "skip", "their", "upkeep", "steps"] {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub(crate) fn parse_legend_rule_doesnt_apply_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"legend") && words.contains(&"rule") && words.contains(&"doesnt") {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "permanents", "are", "colorless"] {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_are_artifacts_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&["all", "permanents", "are", "artifacts"]) {
        return Ok(Some(StaticAbility::add_card_types(
            ObjectFilter::permanent(),
            vec![CardType::Artifact],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"colorless")
        && words.contains(&"cards")
        && words.contains(&"spells")
        && words.contains(&"permanents")
    {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = words.iter().position(|word| *word == "are") else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..are_idx];
    let filter = parse_object_filter(subject_tokens, false)?;

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    if words.get(are_idx + 2) != Some(&"and") || words.get(are_idx + 3) != Some(&"are") {
        return Ok(None);
    }

    let tail = &words[are_idx + 4..];
    let Some(addition_idx) = tail
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "their", "other"])
    else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    let scope = &tail[addition_idx + 5..];
    if !matches!(scope, ["creature", "type"] | ["creature", "types"]) {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if is_article(descriptor) || matches!(*descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), base_color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_creatures_are_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let are_idx = words.iter().position(|word| *word == "are");
    let Some(are_idx) = are_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..are_idx];
    let filter = parse_object_filter(subject_tokens, false)?;

    let color_word = words.get(are_idx + 1).copied();
    let Some(color_word) = color_word else {
        return Ok(None);
    };
    let Some(color) = parse_color(color_word) else {
        return Ok(None);
    };
    if words.len() != are_idx + 2 {
        return Ok(None);
    }

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

pub(crate) fn parse_blood_moon_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["nonbasic", "lands", "are", "mountains"] {
        return Ok(Some(StaticAbility::blood_moon()));
    }
    Ok(None)
}

pub(crate) fn parse_remove_snow_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "lands", "are", "no", "longer", "snow"] {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_land_type_addition_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..be_idx], false)?;

    let mut subtype_word_idx = be_idx + 1;
    if words
        .get(subtype_word_idx)
        .is_some_and(|word| is_article(word))
    {
        subtype_word_idx += 1;
    }
    let Some(subtype_word) = words.get(subtype_word_idx).copied() else {
        return Ok(None);
    };
    let Some(subtype) = parse_subtype_word(subtype_word)
        .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
    else {
        return Ok(None);
    };
    if !is_land_subtype(subtype) {
        return Ok(None);
    }

    let tail = &words[subtype_word_idx + 1..];
    let valid_tail = matches!(
        tail,
        ["in", "addition", "to", "its", "other", "land", "type"]
            | ["in", "addition", "to", "its", "other", "land", "types"]
            | ["in", "addition", "to", "their", "other", "land", "type"]
            | ["in", "addition", "to", "their", "other", "land", "types"]
    );
    if !valid_tail {
        return Ok(None);
    }

    Ok(Some(StaticAbility::add_subtypes(filter, vec![subtype])))
}

pub(crate) fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..be_idx], false)?;
    let (power, toughness) = match parse_pt_modifier(words[be_idx + 1]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    if !matches!(words[be_idx + 2], "creature" | "creatures") {
        return Ok(None);
    }

    let tail = &words[be_idx + 3..];
    let valid_tail = matches!(
        tail,
        ["that", "are", "still", "land"]
            | ["that", "are", "still", "lands"]
            | ["that", "is", "still", "land"]
            | ["that", "is", "still", "a", "land"]
    );
    if !valid_tail {
        return Ok(None);
    }

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, power, toughness),
    ]))
}

pub(crate) fn parse_granted_keyword_static_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "have" || *word == "has")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .position(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    if words(&tokens[..have_token_idx])
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, have_token_idx)?;
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"equipped")
        || subject_words.contains(&"enchanted")
        || subject_words.contains(&"mana")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "can"
                | "cant"
                | "cannot"
                | "attack"
                | "attacks"
                | "block"
                | "blocks"
                | "blocked"
                | "blocking"
                | "cast"
                | "spell"
                | "spells"
                | "during"
                | "until"
                | "unless"
                | "when"
                | "whenever"
                | "if"
                | "though"
        )
    }) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    if let Some(period_idx) = tail_tokens
        .iter()
        .position(|token| matches!(token, Token::Period(_)))
    {
        let leading = trim_commas(&tail_tokens[..period_idx]);
        let trailing = trim_commas(&tail_tokens[period_idx + 1..]);
        let trailing_words = words(&trailing);
        let is_supported_flashback_tail = trailing_words
            == [
                "its",
                "flashback",
                "cost",
                "is",
                "equal",
                "to",
                "its",
                "mana",
                "cost",
            ];
        if !trailing_words.is_empty() && !is_supported_flashback_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(idx) = words(&tail_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        if idx + 3 >= tail_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        keyword_tokens = trim_commas(&tail_tokens[..idx]);
        suffix_condition = Some(parse_static_condition_clause(&tail_tokens[idx + 3..])?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    let keyword_words = words(&keyword_tokens);
    if let Some(and_idx) = keyword_words
        .windows(6)
        .position(|window| window == ["and", "attack", "each", "combat", "if", "able"])
        .or_else(|| {
            keyword_words
                .windows(6)
                .position(|window| window == ["and", "attacks", "each", "combat", "if", "able"])
        })
    {
        keyword_tokens = trim_commas(&keyword_tokens[..and_idx]);
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    let mapped = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect::<Vec<_>>();
    if mapped.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let mut compiled = Vec::new();
    let mut granted_abilities = mapped;
    if grants_must_attack {
        granted_abilities.push(StaticAbility::must_attack());
    }
    for ability in granted_abilities {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    let granted = GrantAbility::source(ability).with_condition(condition.clone());
                    compiled.push(StaticAbility::new(granted));
                } else {
                    compiled.push(ability);
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                let granted = if let Some(condition) = &condition {
                    GrantAbility::new(filter.clone(), ability).with_condition(condition.clone())
                } else {
                    GrantAbility::new(filter.clone(), ability)
                };
                compiled.push(StaticAbility::new(granted));
            }
        }
    }
    Ok(Some(compiled))
}

pub(crate) fn parse_all_creatures_lose_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "creatures", "lose", "flying"] {
        return Ok(Some(StaticAbility::remove_ability(
            ObjectFilter::creature(),
            StaticAbility::flying(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_each_creature_cant_be_blocked_by_more_than_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    // Familiar Ground: "Each creature can't be blocked by more than one creature."
    let clause_words = words(tokens);
    if clause_words.len() < 10 {
        return Ok(None);
    }
    let (subject_len, you_control) = if clause_words.starts_with(&[
        "each", "creature", "you", "control", "cant", "be", "blocked", "by", "more", "than",
    ]) {
        (4usize, true)
    } else if clause_words.starts_with(&[
        "each", "creature", "cant", "be", "blocked", "by", "more", "than",
    ]) {
        (2usize, false)
    } else {
        return Ok(None);
    };

    // "Each creature (you control) can't be blocked by more than <N> creature(s)"
    let amount_word_idx = subject_len + 6;
    let Some(amount_token_idx) = token_index_for_word_index(tokens, amount_word_idx) else {
        return Ok(None);
    };
    let Some((amount, used)) = parse_number(&tokens[amount_token_idx..]) else {
        return Ok(None);
    };

    // Expect "... creature(s)" after the number.
    let rest_tokens = &tokens[amount_token_idx + used..];
    let rest_words = words(rest_tokens);
    if rest_words
        .first()
        .is_some_and(|w| *w == "creature" || *w == "creatures")
    {
        let filter = if you_control {
            ObjectFilter::creature().you_control()
        } else {
            ObjectFilter::creature()
        };
        let granted = StaticAbility::cant_be_blocked_by_more_than(amount as usize);
        return Ok(Some(StaticAbility::grant_ability(filter, granted)));
    }

    Ok(None)
}

pub(crate) fn parse_each_creature_can_block_additional_creature_each_combat_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    // High Ground: "Each creature can block an additional creature each combat."
    let clause_words = words(tokens);
    if clause_words.len() < 9 {
        return Ok(None);
    }
    let (_subject_len, you_control) =
        if clause_words.starts_with(&["each", "creature", "you", "control", "can", "block"]) {
            (4usize, true)
        } else if clause_words.starts_with(&["each", "creature", "can", "block"]) {
            (2usize, false)
        } else {
            return Ok(None);
        };
    if !clause_words.ends_with(&["each", "combat"]) {
        return Ok(None);
    }
    let Some(additional_word_idx) = clause_words.iter().position(|w| *w == "additional") else {
        return Ok(None);
    };
    if additional_word_idx == 0 {
        return Ok(None);
    }

    let mut additional = 1usize;
    let prev = clause_words[additional_word_idx - 1];
    if prev != "an" {
        if let Some(prev_token_idx) = token_index_for_word_index(tokens, additional_word_idx - 1)
            && let Some((count, used)) = parse_number(&tokens[prev_token_idx..])
            && used > 0
        {
            additional = count as usize;
        }
    }

    let filter = if you_control {
        ObjectFilter::creature().you_control()
    } else {
        ObjectFilter::creature()
    };
    let granted = StaticAbility::can_block_additional_creature_each_combat(additional);
    Ok(Some(StaticAbility::grant_ability(filter, granted)))
}

pub(crate) fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(is_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
        return Ok(None);
    };
    let Some(with_idx) = words
        .windows(5)
        .position(|window| window == ["with", "base", "power", "and", "toughness"])
    else {
        return Ok(None);
    };
    if with_idx <= is_idx {
        return Ok(None);
    }

    let Some(pt_word) = words.get(with_idx + 5) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_lose_all = words.windows(3).any(|window| {
        matches!(
            window,
            ["lose", "all", "abilities"] | ["loses", "all", "abilities"]
        )
    });
    if !has_lose_all {
        return Ok(None);
    }

    let subject_end = is_idx.min(
        words
            .iter()
            .position(|word| *word == "lose" || *word == "loses")
            .unwrap_or(is_idx),
    );
    if subject_end == 0 {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut descriptor_words = words[is_idx + 1..with_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if descriptor_words.first().copied() == Some("all") {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.contains(&card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !creature_subtypes.contains(&subtype) {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty() && !set_card_types.contains(&CardType::Creature) {
        set_card_types.push(CardType::Creature);
    }

    let mut set_name: Option<String> = None;
    let tail_words = &words[with_idx + 6..];
    if let Some(named_idx) = tail_words.iter().position(|word| *word == "named") {
        let end_idx = (named_idx + 1..tail_words.len())
            .find(|idx| {
                matches!(
                    tail_words[*idx],
                    "and" | "lose" | "loses" | "with" | "it" | "that" | "those" | "this"
                )
            })
            .unwrap_or(tail_words.len());
        if end_idx > named_idx + 1 {
            set_name = Some(title_case_words(&tail_words[named_idx + 1..end_idx]));
        }
    }

    let has_except_mana = words
        .windows(3)
        .any(|window| window == ["except", "mana", "abilities"]);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));

    Ok(Some(abilities))
}

pub(crate) fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let lose_idx = words
        .iter()
        .position(|word| *word == "lose" || *word == "loses");
    let Some(lose_idx) = lose_idx else {
        return Ok(None);
    };

    if !words[lose_idx + 1..]
        .windows(2)
        .any(|window| window == ["all", "abilities"])
    {
        return Ok(None);
    }
    if words.contains(&"becomes") {
        return Err(CardTextError::ParseError(format!(
            "unsupported lose-all-abilities static becomes clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..lose_idx];
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_except_mana = words
        .windows(3)
        .any(|window| window == ["except", "mana", "abilities"]);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    if let Some(have_idx) = have_idx {
        let after_have = &words[have_idx + 1..];
        if after_have.starts_with(&["base", "power", "and", "toughness"])
            && let Some(modifier_token) = after_have.iter().find(|word| word.contains('/'))
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_all_have_indestructible_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };
    if words[..have_idx]
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .position(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    let tail = trim_commas(&tokens[have_token_idx + 1..]);
    let Some(actions) = parse_ability_line(&tail) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..have_token_idx], false)?;
    Ok(Some(StaticAbility::grant_ability(
        filter,
        StaticAbility::indestructible(),
    )))
}

#[derive(Debug, Clone)]
pub(crate) enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAnthemClause {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) power: AnthemValue,
    pub(crate) toughness: AnthemValue,
    pub(crate) condition: Option<crate::ConditionExpr>,
}

pub(crate) fn words_start_with(tokens: &[Token], expected: &[&str]) -> bool {
    words(tokens).starts_with(expected)
}

pub(crate) fn find_source_reference_start(tokens: &[Token]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

pub(crate) fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(
        filter.attacking
            || filter.nonattacking
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.unblocked,
    ) * 2;
    score += usize::from(filter.is_commander || filter.noncommander) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(filter.entered_battlefield_this_turn) * 2;
    score += usize::from(filter.entered_battlefield_controller.is_some()) * 2;
    score += usize::from(filter.was_dealt_damage_this_turn) * 2;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_supertypes.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_custom_static_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

pub(crate) fn parse_best_object_filter_suffix(tokens: &[Token]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        if candidate
            .first()
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            candidate = &candidate[1..];
        }
        if candidate.is_empty() {
            continue;
        }
        let candidate_words = words(candidate);
        if matches!(candidate_words.as_slice(), ["it"] | ["them"]) {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, filter));
        }
    }
    best.map(|(_, filter)| filter)
}

pub(crate) fn parse_anthem_subject(tokens: &[Token]) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = words(tokens);
    if subject_words.as_slice() == ["it"] {
        return Ok(AnthemSubjectAst::Source);
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    parse_best_object_filter_suffix(tokens)
        .map(AnthemSubjectAst::Filter)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported anthem subject (clause: '{}')",
                words(tokens).join(" ")
            ))
        })
}

pub(crate) fn parse_static_quantity_prefix(
    tokens: &[Token],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing quantity in static condition".to_string(),
        ));
    }

    if tokens[0].is_word("no") {
        return Ok((crate::effect::Comparison::LessThanOrEqual(0), 1));
    }

    if let Some((value, used)) = parse_number(tokens) {
        let value = value as i32;
        let first_word = tokens.first().and_then(Token::as_word);
        if matches!(first_word, Some("a" | "an")) {
            return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), used));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("more") || token.is_word("greater"))
        {
            return Ok((
                crate::effect::Comparison::GreaterThanOrEqual(value),
                used + 2,
            ));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("less") || token.is_word("fewer"))
        {
            return Ok((crate::effect::Comparison::LessThanOrEqual(value), used + 2));
        }
        return Ok((crate::effect::Comparison::Equal(value), used));
    }

    if allow_default_one {
        return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), 0));
    }

    Err(CardTextError::ParseError(
        "missing quantity in static condition".to_string(),
    ))
}

pub(crate) fn parse_permanent_card_count_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["permanent", "card"])
        && !token_words.starts_with(&["permanent", "cards"])
    {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    for (idx, word) in token_words.iter().enumerate() {
        if let Some(zone) = parse_zone_word(word) {
            filter.zone = Some(zone);
            if idx > 0 {
                match token_words[idx - 1] {
                    "your" => filter.owner = Some(PlayerFilter::You),
                    "opponent" | "opponents" => filter.owner = Some(PlayerFilter::Opponent),
                    _ => {}
                }
            }
        }
    }

    filter.zone.map(|_| filter)
}

pub(crate) fn parse_static_condition_clause(tokens: &[Token]) -> Result<crate::ConditionExpr, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_words = words(&tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }

    if clause_words == ["this", "creature", "is", "equipped"]
        || clause_words == ["it", "is", "equipped"]
    {
        return Ok(crate::ConditionExpr::SourceIsEquipped);
    }
    if clause_words == ["this", "creature", "is", "enchanted"]
        || clause_words == ["it", "is", "enchanted"]
    {
        return Ok(crate::ConditionExpr::SourceIsEnchanted);
    }
    if clause_words == ["equipped", "creature", "is", "tapped"] {
        return Ok(crate::ConditionExpr::EquippedCreatureTapped);
    }
    if clause_words == ["equipped", "creature", "is", "untapped"] {
        return Ok(crate::ConditionExpr::EquippedCreatureUntapped);
    }
    if clause_words == ["it", "is", "attacking"]
        || clause_words == ["its", "attacking"]
        || clause_words == ["this", "creature", "is", "attacking"]
        || clause_words == ["this", "permanent", "is", "attacking"]
    {
        return Ok(crate::ConditionExpr::SourceIsAttacking);
    }
    if clause_words == ["it", "is", "your", "turn"] || clause_words == ["its", "your", "turn"] {
        return Ok(crate::ConditionExpr::YourTurn);
    }
    if clause_words == ["it", "is", "not", "your", "turn"]
        || clause_words == ["its", "not", "your", "turn"]
    {
        return Ok(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )));
    }

    if clause_words.starts_with(&["there", "are"]) || clause_words.starts_with(&["there", "is"])
    {
        let quantified = &tokens[2..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, false)?;
        let mut filter_tokens = &quantified[used..];
        if filter_tokens
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            filter_tokens = &filter_tokens[1..];
        }
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in static condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let filter = parse_permanent_card_count_filter(filter_tokens)
            .or_else(|| parse_object_filter(filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counted object phrase in static condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let control_prefix_len = if clause_words.starts_with(&["you", "control"])
        || clause_words.starts_with(&["you", "controls"])
        || clause_words.starts_with(&["opponent", "control"])
        || clause_words.starts_with(&["opponent", "controls"])
        || clause_words.starts_with(&["opponents", "control"])
        || clause_words.starts_with(&["opponents", "controls"])
    {
        2
    } else if clause_words.starts_with(&["an", "opponent", "control"])
        || clause_words.starts_with(&["an", "opponent", "controls"])
        || clause_words.starts_with(&["your", "opponents", "control"])
        || clause_words.starts_with(&["your", "opponents", "controls"])
    {
        3
    } else {
        0
    };
    if control_prefix_len > 0 {
        let quantified = &tokens[control_prefix_len..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
        let mut filter_tokens: Vec<Token> = tokens[..control_prefix_len].to_vec();
        filter_tokens.extend_from_slice(&quantified[used..]);
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported control condition filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let has_counter_on_source = clause_words
        .windows(2)
        .any(|window| window == ["on", "it"] || window == ["on", "this"]);
    let starts_with_source_ref = clause_words.starts_with(&["it", "has"])
        || clause_words.starts_with(&["this", "has"])
        || clause_words.starts_with(&["this", "land", "has"])
        || clause_words.starts_with(&["this", "creature", "has"])
        || clause_words.starts_with(&["this", "permanent", "has"]);
    if starts_with_source_ref && has_counter_on_source {
        let counter_idx = clause_words.iter().position(|word| *word == "counter");
        if let Some(counter_idx) = counter_idx
            && counter_idx > 0
            && let Some(counter_type) = parse_counter_type_word(clause_words[counter_idx - 1])
        {
            let mut filter = ObjectFilter::source();
            filter.with_counter = Some(crate::filter::CounterConstraint::Typed(counter_type));
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(clause_words.join(" ")),
            });
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_anthem_for_each_expression(
    tokens: &[Token],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !words_start_with(&tokens, &["for", "each"]) {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    }
    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    if words_start_with(rest, &["basic", "land", "type", "among"]) {
        let filter_tokens = &rest[4..];
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported domain count filter (clause: '{}')",
                words(&tokens).join(" ")
            ))
        })?;
        return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
    }

    if let Some(attached_idx) = rest.iter().position(|token| token.is_word("attached")) {
        let filter_tokens = &rest[..attached_idx];
        let tail_words = words(&rest[attached_idx + 1..]);
        let attached_to_source = tail_words == ["to", "it"]
            || tail_words == ["to", "this", "creature"]
            || tail_words == ["to", "this", "permanent"];
        if attached_to_source {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                    words(&tokens).join(" ")
                ))
            })?;
            return Ok(AnthemCountExpression::AttachedToSource(filter));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            words(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

pub(crate) fn parse_anthem_prefix_condition(
    tokens: &[Token],
    get_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    if words_start_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }
    if words_start_with(tokens, &["during", "your", "turn"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

pub(crate) fn parse_anthem_clause(
    tokens: &[Token],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;

    let modifier_token = tokens
        .get(get_idx + 1)
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in anthem clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier(modifier_token).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in anthem clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let tail_tokens = trim_edge_punctuation(&tokens[get_idx + 2..tail_end]);
    let mut scale: Option<AnthemCountExpression> = None;
    let mut suffix_condition: Option<crate::ConditionExpr> = None;
    if !tail_tokens.is_empty() {
        if words_start_with(&tail_tokens, &["for", "each"]) {
            scale = Some(parse_anthem_for_each_expression(&tail_tokens)?);
        } else if words_start_with(&tail_tokens, &["as", "long", "as"]) {
            suffix_condition = Some(parse_static_condition_clause(&tail_tokens[3..])?);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing anthem clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    };

    let (power, toughness) = if let Some(scale_expr) = scale {
        (
            AnthemValue::scaled(base_power, scale_expr.clone()),
            AnthemValue::scaled(base_toughness, scale_expr),
        )
    } else {
        (
            AnthemValue::Fixed(base_power),
            AnthemValue::Fixed(base_toughness),
        )
    };

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
    })
}

pub(crate) fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone());

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    StaticAbility::new(anthem)
}

#[derive(Debug)]
pub(crate) struct TypeColorAdditionClause {
    pub(crate) added_colors: ColorSet,
    pub(crate) set_colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn parse_type_color_addition_clause(
    tokens: &[Token],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let words = words(tokens);
    if words.len() < 7 || words.first() != Some(&"is") {
        return Ok(None);
    }

    let Some(addition_idx) = words
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "its", "other"])
    else {
        return Ok(None);
    };
    if addition_idx <= 1 {
        return Ok(None);
    }

    let scope_words = &words[addition_idx + 5..];
    let mut allow_colors = false;
    let mut allow_types = false;
    let mut segment_start = 0usize;
    for idx in 0..=scope_words.len() {
        let is_boundary = idx == scope_words.len() || scope_words[idx] == "and";
        if !is_boundary {
            continue;
        }
        if segment_start == idx {
            segment_start = idx + 1;
            continue;
        }
        let segment = &scope_words[segment_start..idx];
        segment_start = idx + 1;
        if segment.len() == 1 && (segment[0] == "color" || segment[0] == "colors") {
            allow_colors = true;
            continue;
        }
        if matches!(segment.last().copied(), Some("type" | "types"))
            && segment[..segment.len() - 1]
                .iter()
                .all(|word| is_type_scope_qualifier_word(word))
        {
            allow_types = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported in-addition scope in type/color clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_words = words[1..addition_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.contains(&card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if allow_types {
                if !subtypes.contains(&subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

pub(crate) fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

pub(crate) fn parse_soulbond_shared_line(tokens: &[Token]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    let prefix = [
        "as",
        "long",
        "as",
        "this",
        "creature",
        "is",
        "paired",
        "with",
        "another",
        "creature",
    ];
    if !clause_words.starts_with(&prefix) {
        return Ok(None);
    }

    let rest = trim_commas(&tokens[prefix.len()..]);
    if rest.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing soulbond shared effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let rest_words = words(&rest);
    let pt_modifier_idx = if rest_words.starts_with(&["both", "creatures", "get"]) {
        Some(3usize)
    } else if rest_words.starts_with(&["each", "of", "those", "creatures", "gets"]) {
        Some(5usize)
    } else {
        None
    };
    if let Some(modifier_idx) = pt_modifier_idx {
        let modifier = *rest_words.get(modifier_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let (power, toughness) = parse_pt_modifier(modifier).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(vec![StaticAbility::soulbond_shared_power_toughness(
            power, toughness,
        )]));
    }

    let ability_start = if rest_words.starts_with(&["both", "creatures", "have"]) {
        Some(3usize)
    } else if rest_words.starts_with(&["each", "of", "those", "creatures", "has"]) {
        Some(5usize)
    } else {
        None
    };
    if let Some(ability_start) = ability_start {
        let mut ability_tokens = trim_commas(&rest[ability_start..]);
        let ability_end = ability_tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_) | Token::Comma(_)))
            .unwrap_or(ability_tokens.len());
        ability_tokens = trim_commas(&ability_tokens[..ability_end]);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing shared ability in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let Some(actions) = parse_ability_line(&ability_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported shared ability in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let abilities: Vec<StaticAbility> = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect();
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported shared ability in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let shared = abilities
            .into_iter()
            .map(StaticAbility::soulbond_shared_ability)
            .collect();
        return Ok(Some(shared));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported soulbond shared clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_anthem_and_type_color_addition_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };

    let and_idx = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx));
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };

    let addition_tokens = &tokens[and_idx + 1..];
    let Some(additions) = parse_type_color_addition_clause(addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_keyword_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let have_idx = clause_words
        .iter()
        .position(|word| *word == "have" || *word == "has");

    let (Some(get_idx), Some(have_idx)) = (get_idx, have_idx) else {
        return Ok(None);
    };

    if have_idx <= get_idx {
        return Ok(None);
    }

    let have_token_idx = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > get_idx && (token.is_word("have") || token.is_word("has"))).then_some(idx)
    });
    let Some(have_token_idx) = have_token_idx else {
        return Ok(None);
    };

    let pre_grant_words = words(&tokens[..have_token_idx]);
    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if pre_grant_words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let mut ability_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<crate::ConditionExpr> = None;
    if let Some(as_long_idx) = words(&ability_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        let as_token_idx =
            token_index_for_word_index(&ability_tokens, as_long_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map trailing 'as long as' keyword condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let condition_start_idx = token_index_for_word_index(&ability_tokens, as_long_idx + 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let ability_head = trim_commas(&ability_tokens[..as_token_idx]);
        if ability_head.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let condition_tokens = trim_commas(&ability_tokens[condition_start_idx..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        trailing_condition = Some(parse_static_condition_clause(&condition_tokens)?);
        ability_tokens = ability_head;
    }

    let mut abilities: Vec<StaticAbility> = Vec::new();
    let mut granted_activated_ability: Option<Ability> = None;
    let mut granted_activated_display: Option<String> = None;

    let and_has_idx = (0..ability_tokens.len().saturating_sub(1)).find(|idx| {
        ability_tokens[*idx].is_word("and")
            && (ability_tokens[*idx + 1].is_word("has") || ability_tokens[*idx + 1].is_word("have"))
    });
    if let Some(and_has_idx) = and_has_idx {
        let keyword_tokens = trim_commas(&ability_tokens[..and_has_idx]);
        if !keyword_tokens.is_empty() {
            if let Some(actions) = parse_ability_line(&keyword_tokens) {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                abilities.extend(
                    actions
                        .into_iter()
                        .filter_map(keyword_action_to_static_ability),
                );
            } else {
                return Ok(None);
            }
        }

        let ability_tail_tokens = trim_commas(&ability_tokens[and_has_idx + 2..]);
        if !ability_tail_tokens.is_empty() {
            let has_colon = ability_tail_tokens
                .iter()
                .any(|token| matches!(token, Token::Colon(_)));
            let Some(parsed) = parse_activated_line(&ability_tail_tokens)? else {
                if has_colon {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported granted activated ability in anthem clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                return Ok(None);
            };
            let mut ability = parsed.ability;
            let display = words(&ability_tail_tokens).join(" ");
            if ability.text.is_none() {
                ability.text = Some(display.clone());
            }
            granted_activated_display = Some(display);
            granted_activated_ability = Some(ability);
        }
    } else if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        abilities = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect();
    } else {
        return Ok(None);
    }

    if abilities.is_empty() && granted_activated_ability.is_none() {
        return Ok(None);
    }

    let clause_tail_end = if have_token_idx > get_idx + 2
        && tokens
            .get(have_token_idx - 1)
            .is_some_and(|token| token.is_word("and"))
    {
        have_token_idx - 1
    } else {
        have_token_idx
    };
    let mut clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause)];
    for ability in abilities {
        let granted = match &clause.subject {
            AnthemSubjectAst::Source => GrantAbility::source(ability),
            AnthemSubjectAst::Filter(filter) => GrantAbility::new(filter.clone(), ability),
        };
        let granted = if let Some(condition) = &clause.condition {
            granted.with_condition(condition.clone())
        } else {
            granted
        };
        result.push(StaticAbility::new(granted));
    }

    if let Some(ability) = granted_activated_ability {
        let filter = match &clause.subject {
            AnthemSubjectAst::Source => ObjectFilter::source(),
            AnthemSubjectAst::Filter(filter) => filter.clone(),
        };
        let mut granted = crate::static_abilities::GrantObjectAbilityForFilter::new(
            filter,
            ability,
            granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
        );
        if let Some(condition) = &clause.condition {
            granted = granted.with_condition(condition.clone());
        }
        result.push(StaticAbility::new(granted));
    }

    Ok(Some(result))
}

pub(crate) fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx))
    else {
        return Ok(None);
    };
    let Some(attack_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > and_idx && (token.is_word("attack") || token.is_word("attacks"))).then_some(idx)
    }) else {
        return Ok(None);
    };

    let attack_tail = words(&tokens[attack_idx..]);
    if attack_tail.as_slice() != ["attacks", "each", "combat", "if", "able"]
        && attack_tail.as_slice() != ["attack", "each", "combat", "if", "able"]
    {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause)];
    let granted = match &clause.subject {
        AnthemSubjectAst::Source => GrantAbility::source(StaticAbility::must_attack()),
        AnthemSubjectAst::Filter(filter) => {
            GrantAbility::new(filter.clone(), StaticAbility::must_attack())
        }
    };
    let granted = if let Some(condition) = &clause.condition {
        granted.with_condition(condition.clone())
    } else {
        granted
    };
    result.push(StaticAbility::new(granted));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    // Targeted "gets +N/+N" text is usually a one-shot spell/ability effect,
    // not a global/static anthem.
    if words.contains(&"target") {
        return Ok(None);
    }
    // "until end of turn" indicates a temporary effect, not a permanent anthem.
    if words
        .windows(4)
        .any(|w| w == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, get_idx, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

pub(crate) fn parse_has_base_power_toughness_static_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"target") {
        return Ok(None);
    }
    if subject_words.starts_with(&["until", "end", "of", "turn"])
        || subject_words.starts_with(&["until", "your", "next", "turn"])
    {
        return Ok(None);
    }

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let tail = &rest_words[5..];
    if !tail.is_empty() {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    Ok(Some(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    )))
}

pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let enters_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enters_idx) = enters_idx else {
        return Ok(None);
    };
    let with_idx = clause_words.iter().position(|word| *word == "with");
    let Some(with_idx) = with_idx else {
        return Ok(None);
    };
    if with_idx <= enters_idx {
        return Ok(None);
    }

    let tapped_between = clause_words[enters_idx + 1..with_idx]
        .iter()
        .any(|word| *word == "tapped");
    if !tapped_between {
        return Ok(None);
    }
    if !clause_words
        .iter()
        .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }
    if !is_source_reference_words(&clause_words[..enters_idx]) {
        return Ok(None);
    }

    let Some(counters) = parse_enters_with_counters_line(tokens)? else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::enters_tapped_ability(), counters]))
}

pub(crate) fn parse_enters_with_counters_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enters_idx = words
        .iter()
        .position(|word| *word == "enters")
        .unwrap_or(usize::MAX);
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enters_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }
    let subject_words = words.get(..enters_idx).unwrap_or_default();
    if !is_source_reference_words(subject_words) {
        return Ok(None);
    }
    if !words.contains(&"with")
        || !words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let with_idx = tokens
        .iter()
        .position(|token| token.is_word("with"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'with' in enters-with-counters clause".to_string())
        })?;
    let after_with = &tokens[with_idx + 1..];
    let (mut count, used) = parse_value(after_with).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter count in self ETB counters (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let counter_type = parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for self ETB counters (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let counter_idx = after_with
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword for self ETB counters (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let mut tail = &after_with[counter_idx + 1..];
    if tail.first().is_some_and(|token| token.is_word("on")) {
        tail = &tail[1..];
    }
    if tail.first().is_some_and(|token| token.is_word("it")) {
        tail = &tail[1..];
    } else if tail
        .first()
        .is_some_and(|token| token.is_word("this") || token.is_word("thiss"))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(Token::as_word)
            && (matches!(word, "source" | "spell" | "card")
                || word == "creature"
                || word == "permanent"
                || parse_card_type(word).is_some())
        {
            tail = &tail[1..];
        }
    }
    let tail = trim_commas(tail);
    let tail_has_words = tail.iter().any(|token| token.as_word().is_some());
    if tail_has_words {
        count = parse_where_x_value_clause(&tail).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing self ETB counter clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    }

    Ok(Some(StaticAbility::enters_with_counters_value(
        counter_type,
        count,
    )))
}

pub(crate) fn parse_where_x_value_clause(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    if let Some(value) = parse_where_x_source_stat_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_gained_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }

    // where X is your devotion to black
    if words.contains(&"devotion") {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"all")
        && words.contains(&"players")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects> plus/minus N
    if let Some(value) = parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens) {
        return Some(value);
    }

    // where X is the number of cards in your hand
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"your")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if words.contains(&"party")
        && words.contains(&"your")
        && (words.contains(&"creature") || words.contains(&"creatures"))
    {
        return Some(Value::PartySize(PlayerFilter::You));
    }

    // where X is the number of differently named <objects>
    if let Some(value) = parse_where_x_is_number_of_differently_named_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }

    None
}

pub(crate) fn parse_where_x_source_stat_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(["this", "creatures", "power"]) | Some(["its", "power"]) => Some(Value::SourcePower),
        Some(["this", "creatures", "toughness"]) | Some(["its", "toughness"]) => {
            Some(Value::SourceToughness)
        }
        Some(["this", "creatures", "mana", "value"]) | Some(["its", "mana", "value"]) => {
            Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(["the", "amount", "of", "life", "you", "gained", "this", "turn"])
        | Some(["amount", "of", "life", "you", "gained", "this", "turn"]) => {
            Some(Value::LifeGainedThisTurn(PlayerFilter::You))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some([
            "the",
            "total",
            "amount",
            "of",
            "noncombat",
            "damage",
            "dealt",
            "to",
            "your",
            "opponents",
            "this",
            "turn",
        ])
        | Some([
            "total",
            "amount",
            "of",
            "noncombat",
            "damage",
            "dealt",
            "to",
            "your",
            "opponents",
            "this",
            "turn",
        ]) => Some(Value::NoncombatDamageDealtToPlayersThisTurn(
            PlayerFilter::Opponent,
        )),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let mut idx = 3usize;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }
    let aggregate = match clause_words.get(idx).copied() {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.get(idx).copied() == Some("power") {
        idx += 1;
        "power"
    } else if clause_words.get(idx).copied() == Some("toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.get(idx).copied() == Some("mana")
        && clause_words.get(idx + 1).copied() == Some("value")
    {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !matches!(clause_words.get(idx).copied(), Some("of" | "among")) {
        return None;
    }
    idx += 1;

    if aggregate == "greatest" && value_kind == "mana_value" {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value);
        }
    }

    let object_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[Token],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let commander_start_token_idx = token_index_for_word_index(tokens, commander_start_word_idx)?;
    let commander_words = words(&tokens[commander_start_token_idx..]);
    let normalized: Vec<&str> = commander_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if normalized
        != [
            "commander",
            "you",
            "own",
            "on",
            "battlefield",
            "or",
            "in",
            "command",
            "zone",
        ]
    {
        return None;
    }

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(PlayerFilter::You);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

pub(crate) fn parse_where_x_is_number_of_differently_named_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let number_idx = clause_words.iter().position(|word| *word == "number")?;
    if clause_words.get(number_idx + 1).copied() != Some("of") {
        return None;
    }
    if clause_words.get(number_idx + 2).copied() != Some("differently") {
        return None;
    }
    if clause_words.get(number_idx + 3).copied() != Some("named") {
        return None;
    }

    let object_start_word_idx = number_idx + 4;
    let object_start_token_idx = token_index_for_word_index(tokens, object_start_word_idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::DistinctNames(filter))
}

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    if clause_words.contains(&"creature")
        && clause_words.contains(&"type")
        && clause_words.contains(&"common")
    {
        return None;
    }

    let number_idx = clause_words.iter().position(|word| *word == "number")?;
    if clause_words.get(number_idx + 1).copied() != Some("of") {
        return None;
    }

    let object_start_word_idx = number_idx + 2;
    let mut seen_words = 0usize;
    let mut object_start_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == object_start_word_idx {
            object_start_token_idx = Some(idx);
            break;
        }
        seen_words += 1;
    }
    let object_start_token_idx = object_start_token_idx?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter_words = words(filter_tokens);
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::BasicLandTypesAmong(scope_filter));
    }
    if filter_words.starts_with(&["color", "among"])
        || filter_words.starts_with(&["colors", "among"])
    {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::ColorsAmong(scope_filter));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let value_start_idx = token_index_for_word_index(tokens, 3)?;
    let (fixed_value, fixed_used) = parse_number(&tokens[value_start_idx..])?;
    let plus_word_idx = 3 + fixed_used;
    if words.get(plus_word_idx).copied() != Some("plus") {
        return None;
    }

    let mut number_word_idx = plus_word_idx + 1;
    if words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if words.get(number_word_idx).copied() != Some("number")
        || words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_idx = token_index_for_word_index(tokens, number_word_idx + 2)?;
    let filter_tokens = &tokens[filter_start_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value as i32)),
        Box::new(Value::Count(filter)),
    ))
}

pub(crate) fn parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let mut number_word_idx = 3usize;
    if clause_words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx).copied() != Some("number")
        || clause_words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = (filter_start_word_idx + 1..clause_words.len())
        .find(|idx| matches!(clause_words[*idx], "plus" | "minus"))?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    let filter_words = words(&filter_tokens);
    let count_value = if filter_words.contains(&"cards")
        && filter_words.contains(&"in")
        && filter_words.contains(&"your")
        && (filter_words.contains(&"hand") || filter_words.contains(&"hands"))
    {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        Value::Count(filter)
    };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = words(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(count_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn token_index_for_word_index(tokens: &[Token], word_index: usize) -> Option<usize> {
    let mut seen_words = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == word_index {
            return Some(idx);
        }
        seen_words += 1;
    }
    None
}

pub(crate) fn parse_enters_tapped_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped = clause_words.contains(&"enter") || clause_words.contains(&"enters");
        let has_tapped = clause_words.contains(&"tapped");
        if has_enters_tapped && has_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.contains(&"unless") {
        return Ok(None);
    }
    let enter_word_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !clause_words
        .iter()
        .skip(enter_word_idx + 1)
        .any(|word| *word == "tapped")
    {
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this") {
        return Ok(None);
    }
    if clause_words.contains(&"copy") {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let before_enter = &tokens[..enter_token_idx];
    let before_words = words(before_enter);
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize| {
        token_index_for_word_index(before_enter, before_words.len().saturating_sub(suffix_len))
            .unwrap_or(before_enter.len())
    };
    if before_words.ends_with(&["played", "by", "your", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "an", "opponent"])
        || before_words.ends_with(&["played", "by", "a", "opponent"])
    {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(3);
    }
    let mut filter = parse_object_filter(&before_enter[..filter_end], false)?;
    if let Some(controller) = controller_override {
        filter.controller = Some(controller);
    }
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

pub(crate) fn parse_conditional_enters_tapped_unless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"enters") && !clause_words.contains(&"enter") {
        return Ok(None);
    }
    if !clause_words.contains(&"tapped") || !clause_words.contains(&"unless") {
        return Ok(None);
    }

    let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    let condition_words = words(&tokens[unless_idx + 1..]);
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "fewer", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "basic", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands(),
        ));
    }
    if condition_words.starts_with(&["a", "player", "has", "13", "or", "less", "life"])
        || condition_words.starts_with(&["a", "player", "has", "thirteen", "or", "less", "life"])
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if condition_words.starts_with(&["you", "have", "two", "or", "more", "opponents"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if condition_words.starts_with(&["you", "control"])
        || condition_words.starts_with(&["you", "controls"])
    {
        let control_idx = tokens[unless_idx + 1..]
            .iter()
            .position(|token| token.is_word("control") || token.is_word("controls"))
            .map(|idx| unless_idx + 1 + idx)
            .unwrap_or(unless_idx + 1);
        let filter_tokens = trim_edge_punctuation(&tokens[control_idx + 1..]);
        if !filter_tokens.is_empty() {
            if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
                let condition = crate::ConditionExpr::YouControl(filter);
                return Ok(Some(StaticAbility::enters_tapped_unless_condition(
                    condition,
                    clause_words.join(" "),
                )));
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enter_word_idx = words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }

    if words.first().copied() == Some("this") {
        return Ok(None);
    }
    if !words.contains(&"battlefield")
        || !words.contains(&"with")
        || !words.contains(&"additional")
        || !words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..enter_token_idx], false)?;

    let additional_idx = tokens
        .iter()
        .position(|token| token.is_word("additional"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let mut count = 1u32;
    if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&tokens[additional_idx - 1..additional_idx])
    {
        count = parsed;
    } else if let Some((parsed, _)) = parse_number(&tokens[additional_idx + 1..]) {
        count = parsed;
    }

    let counter_type = parse_counter_type_from_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(StaticAbility::enters_with_counters_for_filter(
        filter,
        counter_type,
        count,
    )))
}

pub(crate) fn parse_creatures_cant_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["creatures", "cant", "block"] {
        return Ok(Some(StaticAbility::grant_ability(
            ObjectFilter::creature(),
            StaticAbility::cant_block(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "creatures",
        ]
    {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let is_this_creature = words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "this",
            "creature",
            "by",
            "creatures",
        ];
    let is_this_permanent = words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "this",
            "permanent",
            "by",
            "creatures",
        ];

    if is_this_creature || is_this_permanent {
        return Ok(Some(StaticAbility::prevent_all_damage_to_self_by_creatures()));
    }
    Ok(None)
}

pub(crate) fn annihilator_granted_ability(amount: u32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_attacks(),
            effects: vec![Effect::sacrifice_player(
                ObjectFilter::permanent(),
                Value::Fixed(amount as i32),
                PlayerFilter::Defending,
            )],
            choices: vec![],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!("Annihilator {amount}")),
    }
}

pub(crate) fn parse_equipped_creature_has_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let clause_text = words.join(" ");
    if words.len() < 4 || words[0] != "equipped" || words[1] != "creature" || words[2] != "has" {
        return Ok(None);
    }

    let ability_tokens = &tokens[3..];
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    let mut extra_grants = Vec::new();
    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbility::attached_ability_grant(
                annihilator_granted_ability(amount),
                format!("equipped creature has annihilator {amount}"),
            ));
            continue;
        }
        if let Some(ability) = keyword_action_to_static_ability(action) {
            abilities.push(ability);
        }
    }

    if abilities.is_empty() && extra_grants.is_empty() {
        return Ok(None);
    }

    let mut out = Vec::new();
    if !abilities.is_empty() {
        out.push(StaticAbility::equipment_grant(abilities));
    }
    out.extend(extra_grants);
    Ok(Some(out))
}

pub(crate) fn parse_enchanted_creature_has_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let clause_text = words.join(" ");
    if words.len() < 4 || words.first().copied() != Some("enchanted") {
        return Ok(None);
    }
    let subject = match words.get(1).copied() {
        Some("creature") => "enchanted creature",
        Some("permanent") => "enchanted permanent",
        _ => return Ok(None),
    };
    if words.get(2).copied() != Some("has") {
        return Ok(None);
    }

    let ability_tokens = &tokens[3..];
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            out.push(StaticAbility::attached_ability_grant(
                annihilator_granted_ability(amount),
                format!("{subject} has annihilator {amount}"),
            ));
            continue;
        }

        let Some(static_ability) = keyword_action_to_static_ability(action) else {
            continue;
        };
        let ability_text = format!(
            "{subject} has {}",
            static_ability.display().to_ascii_lowercase()
        );
        let granted = Ability::static_ability(static_ability).with_text(&ability_text);
        out.push(StaticAbility::attached_ability_grant(granted, ability_text));
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_attached_cant_attack_or_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = normalize_cant_words(tokens);
    if normalized.len() < 4 {
        return Ok(None);
    }

    let is_enchanted = normalized.starts_with(&["enchanted", "creature"]);
    let is_equipped = normalized.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let subject_len = 2usize;
    let tail = &normalized[subject_len..];
    if !tail.starts_with(&["cant"]) {
        return Ok(None);
    }

    let subject = if is_equipped {
        "equipped creature"
    } else {
        "enchanted creature"
    };

    let (restriction, display) = if tail == ["cant", "attack"] {
        (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject} can't attack"),
        )
    } else if tail == ["cant", "block"] {
        (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject} can't block"),
        )
    } else if tail == ["cant", "attack", "or", "block"] {
        (
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format!("{subject} can't attack or block"),
        )
    } else {
        return Ok(None);
    };

    let granted = Ability {
        kind: AbilityKind::Static(StaticAbility::restriction(restriction, display.clone())),
        functional_zones: vec![Zone::Battlefield],
        text: Some(display.clone()),
    };

    Ok(Some(StaticAbility::attached_ability_grant(
        granted,
        normalized.join(" "),
    )))
}

pub(crate) fn parse_you_control_attached_creature_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 4 || !line_words.starts_with(&["you", "control"]) {
        return Ok(None);
    }

    let tail = &line_words[2..];
    let is_attached_subject = matches!(
        tail,
        ["enchanted", "creature"]
            | ["enchanted", "permanent"]
            | ["enchanted", "land"]
            | ["enchanted", "artifact"]
            | ["equipped", "creature"]
            | ["equipped", "permanent"]
    );
    if !is_attached_subject {
        return Ok(None);
    }

    Ok(Some(StaticAbility::control_attached_permanent(
        line_words.join(" "),
    )))
}

pub(crate) fn parse_attached_gets_and_cant_block_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if get_idx >= and_idx {
        return Ok(None);
    }

    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words = normalize_cant_words(&tail_tokens);
    let restriction = match tail_words.as_slice() {
        ["cant", "block"] => crate::effect::Restriction::block(ObjectFilter::source()),
        ["cant", "attack"] => crate::effect::Restriction::attack(ObjectFilter::source()),
        ["cant", "attack", "or", "block"] => {
            crate::effect::Restriction::attack_or_block(ObjectFilter::source())
        }
        _ => return Ok(None),
    };

    let subject = if is_enchanted {
        "enchanted creature"
    } else {
        "equipped creature"
    };
    let display = match tail_words.as_slice() {
        ["cant", "block"] => format!("{subject} can't block"),
        ["cant", "attack"] => format!("{subject} can't attack"),
        _ => format!("{subject} can't attack or block"),
    };
    let granted = Ability {
        kind: AbilityKind::Static(StaticAbility::restriction(restriction, display.clone())),
        functional_zones: vec![Zone::Battlefield],
        text: Some(display.clone()),
    };
    let grant = StaticAbility::attached_ability_grant(granted, display);
    let anthem = build_anthem_static_ability(&parse_anthem_clause(tokens, get_idx, and_idx)?);
    Ok(Some(vec![anthem, grant]))
}

pub(crate) fn parse_prevent_damage_to_source_remove_counter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 12 {
        return Ok(None);
    }

    if !line_words.starts_with(&["if", "damage", "would", "be", "dealt", "to"]) {
        return Ok(None);
    }
    if !(line_words[6..].starts_with(&["this", "creature"])
        || line_words[6..].starts_with(&["this", "permanent"]))
    {
        return Ok(None);
    }
    if !line_words
        .windows(3)
        .any(|window| window == ["prevent", "that", "damage"])
    {
        return Ok(None);
    }

    let Some(remove_word_idx) = line_words.iter().position(|word| *word == "remove") else {
        return Ok(None);
    };
    let Some(counter_word_idx) = line_words[remove_word_idx + 1..]
        .iter()
        .position(|word| *word == "counter" || *word == "counters")
        .map(|idx| remove_word_idx + 1 + idx)
    else {
        return Ok(None);
    };

    let remove_token_idx = token_index_for_word_index(tokens, remove_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map remove clause in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        ))
    })?;
    let counter_token_idx = token_index_for_word_index(tokens, counter_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map counter clause in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        ))
    })?;

    let mut descriptor_tokens = trim_commas(&tokens[remove_token_idx + 1..=counter_token_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        )));
    }

    let (amount, used) = parse_number(&descriptor_tokens).unwrap_or((1, 0));
    descriptor_tokens = descriptor_tokens[used..].to_vec();
    while descriptor_tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        descriptor_tokens.remove(0);
    }

    let counter_type = parse_counter_type_from_tokens(&descriptor_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        ))
    })?;

    let after_counter_words = line_words.get(counter_word_idx + 1..).unwrap_or_default();
    let valid_tail = after_counter_words.starts_with(&["from", "this", "creature"])
        || after_counter_words.starts_with(&["from", "this", "permanent"])
        || after_counter_words.starts_with(&["from", "it"]);
    if !valid_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-damage remove tail (clause: '{}')",
            line_words.join(" ")
        )));
    }

    Ok(Some(StaticAbility::prevent_damage_to_self_remove_counter(
        counter_type,
        amount,
    )))
}

pub(crate) fn parse_attached_prevent_all_damage_dealt_by_attached_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    // "Prevent all damage that would be dealt by enchanted creature."
    if !line_words.starts_with(&["prevent", "all", "damage"]) {
        return Ok(None);
    }
    if !line_words.ends_with(&["by", "enchanted", "creature"]) {
        return Ok(None);
    }

    let display = "prevent all damage that would be dealt by enchanted creature".to_string();
    let granted = Ability {
        kind: AbilityKind::Static(StaticAbility::new(
            crate::static_abilities::PreventAllDamageDealtByThisPermanent,
        )),
        functional_zones: vec![Zone::Battlefield],
        text: Some(display.clone()),
    };

    Ok(Some(StaticAbility::attached_ability_grant(
        granted, display,
    )))
}

pub(crate) fn parse_attached_has_keywords_and_triggered_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(and_idx) = ability_tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx == 0 || and_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_starts = ability_tokens
        .get(and_idx + 1)
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&ability_tokens, and_idx + 1);
    if !trigger_starts {
        return Ok(None);
    }

    let keyword_tokens = trim_edge_punctuation(&ability_tokens[..and_idx]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let clause_text = line_words.join(" ");
    let mut keyword_statics = Vec::new();
    let mut extra_grants = Vec::new();
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbility::attached_ability_grant(
                annihilator_granted_ability(amount),
                format!(
                    "{} has annihilator {amount}",
                    if is_equipped {
                        "equipped creature"
                    } else {
                        "enchanted creature"
                    }
                ),
            ));
        } else if let Some(static_ability) = keyword_action_to_static_ability(action) {
            keyword_statics.push(static_ability);
        }
    }
    if keyword_statics.is_empty() && extra_grants.is_empty() {
        return Ok(None);
    }

    let trigger_tokens = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    let triggered = match parse_triggered_line(&trigger_tokens)? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let (compiled_effects, choices) = compile_trigger_effects(Some(&trigger), &effects)?;
            Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: compile_trigger_spec(trigger),
                    effects: compiled_effects,
                    choices,
                    intervening_if: max_triggers_per_turn
                        .map(crate::ConditionExpr::MaxTimesEachTurn),
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(words(&trigger_tokens).join(" ")),
            }
        }
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached triggered grant clause (clause: '{}')",
                clause_text
            )));
        }
    };

    let subject = match parse_anthem_subject(&tokens[..has_idx]) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };

    let mut static_abilities = Vec::new();
    for ability in keyword_statics {
        static_abilities.push(StaticAbility::grant_ability(filter.clone(), ability));
    }
    static_abilities.extend(extra_grants);
    let subject_text = words(&tokens[..has_idx]).join(" ");
    let display = format!("{subject_text} has {}", words(&trigger_tokens).join(" "));
    static_abilities.push(StaticAbility::attached_ability_grant(triggered, display));

    Ok(Some(static_abilities))
}

pub(crate) fn parse_attached_is_legendary_gets_and_has_keywords_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 10 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(is_idx) = tokens.iter().position(|token| token.is_word("is")) else {
        return Ok(None);
    };
    if is_idx < 2
        || !tokens
            .get(is_idx + 1)
            .is_some_and(|token| token.is_word("legendary"))
    {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if !(is_idx < get_idx && get_idx + 1 < tokens.len() && get_idx < has_idx) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..is_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false)?;

    let modifier_token = tokens.get(get_idx + 1).and_then(Token::as_word);
    let Some(modifier_token) = modifier_token else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(modifier_token) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let keyword_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };

    let clause_text = line_words.join(" ");
    let mut out = Vec::new();
    out.push(StaticAbility::add_supertypes(
        filter.clone(),
        vec![Supertype::Legendary],
    ));

    let anthem_clause = ParsedAnthemClause {
        subject: AnthemSubjectAst::Filter(filter.clone()),
        power: AnthemValue::Fixed(power),
        toughness: AnthemValue::Fixed(toughness),
        condition: None,
    };
    out.push(build_anthem_static_ability(&anthem_clause));

    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let Some(static_ability) = keyword_action_to_static_ability(action) {
            out.push(StaticAbility::grant_ability(filter.clone(), static_ability));
        }
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_attached_gets_and_has_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }
    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if !(get_idx < and_idx && and_idx < has_idx) {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let anthem = build_anthem_static_ability(&clause);

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attached ability after 'has' (clause: '{}')",
            line_words.join(" ")
        )));
    }
    let ability_text = words(&ability_tokens).join(" ");
    let ability = Ability {
        kind: AbilityKind::Static(StaticAbility::custom(
            "attached_grant",
            ability_text.clone(),
        )),
        functional_zones: vec![Zone::Battlefield],
        text: Some(ability_text.clone()),
    };
    let subject = if is_enchanted {
        "enchanted creature"
    } else {
        "equipped creature"
    };
    let display = format!("{subject} has {ability_text}");
    let grant = StaticAbility::attached_ability_grant(ability, display);

    Ok(Some(vec![anthem, grant]))
}

pub(crate) fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 4 || line_words[0] != "equipped" || line_words[1] != "creature" {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = &tokens[has_idx + 1..];
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let Some(parsed) = parse_activated_line(ability_tokens)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_words.join(" ")
            )));
        }
        return Ok(None);
    };

    let mut static_abilities = Vec::new();
    if let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
        && get_idx < has_idx
    {
        let clause_tail_end = if has_idx > get_idx + 2
            && tokens
                .get(has_idx - 1)
                .is_some_and(|token| token.is_word("and"))
        {
            has_idx - 1
        } else {
            has_idx
        };
        let clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
        static_abilities.push(build_anthem_static_ability(&clause));
    }

    let mut ability = parsed.ability;
    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }
    static_abilities.push(StaticAbility::attached_ability_grant(
        ability,
        line_words.join(" "),
    ));

    Ok(Some(static_abilities))
}

pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["you", "may", "choose", "not", "to", "untap"]) {
        return Ok(None);
    }
    if !words.ends_with(&["during", "your", "untap", "step"]) {
        return Ok(None);
    }
    if words.len() <= 10 {
        return Ok(None);
    }

    let subject_words = &words[6..words.len() - 4];
    let subject_allowed = matches!(
        subject_words,
        ["this"]
            | ["it"]
            | ["this", "artifact"]
            | ["this", "creature"]
            | ["this", "land"]
            | ["this", "permanent"]
            | ["this", "card"]
    );
    if !subject_allowed {
        return Ok(None);
    }

    let subject = subject_words.join(" ");
    let text = format!("You may choose not to untap {subject} during your untap step");
    Ok(Some(StaticAbility::custom(
        "may_choose_not_to_untap_during_your_untap_step",
        text,
    )))
}

pub(crate) fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if !is_untap_during_each_other_players_untap_step_words(&line_words) {
        return Ok(None);
    }
    Ok(Some(StaticAbility::custom(
        "untap_during_each_other_players_untap_step",
        line_words.join(" "),
    )))
}

pub(crate) fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let prefix_len = [
        ["this", "doesnt", "untap", "during", "your", "untap", "step"].as_slice(),
        [
            "this", "land", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "artifact", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "creature", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "doesn't", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "land", "doesn't", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "artifact", "doesn't", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "creature", "doesn't", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "does", "not", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
    ]
    .iter()
    .find(|pattern| clause_words.starts_with(pattern))
    .map(|pattern| pattern.len());

    if let Some(prefix_len) = prefix_len {
        let tail_tokens = trim_commas(&tokens[prefix_len..]);
        if tail_tokens.is_empty() {
            return Ok(Some(StaticAbility::doesnt_untap()));
        }
        let tail_words = words(&tail_tokens);
        if tail_words.first().copied() == Some("if") {
            let condition_tokens = trim_commas(&tail_tokens[1..]);
            if condition_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing condition after untap-step if-clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let condition = parse_static_condition_clause(&condition_tokens)?;
            let granted =
                crate::static_abilities::GrantAbility::source(StaticAbility::doesnt_untap())
                    .with_condition(condition);
            return Ok(Some(StaticAbility::new(granted)));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing untap-step clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let attached_subject = clause_words.starts_with(&["enchanted", "creature"])
        || clause_words.starts_with(&["equipped", "creature"]);
    if attached_subject {
        let remainder = &clause_words[2..];
        let attached_matches = matches!(
            remainder,
            [
                "doesnt",
                "untap",
                "during",
                "its",
                "controller",
                "untap",
                "step"
            ] | [
                "doesnt",
                "untap",
                "during",
                "its",
                "controllers",
                "untap",
                "step"
            ] | [
                "does",
                "not",
                "untap",
                "during",
                "its",
                "controller",
                "untap",
                "step"
            ] | [
                "does",
                "not",
                "untap",
                "during",
                "its",
                "controllers",
                "untap",
                "step"
            ]
        );

        if attached_matches {
            let text = clause_words.join(" ");
            let granted = Ability::static_ability(StaticAbility::doesnt_untap()).with_text(&text);
            return Ok(Some(StaticAbility::attached_ability_grant(granted, text)));
        }
    }

    Ok(None)
}

pub(crate) fn parse_flying_restriction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let flying_only_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ];

    if flying_only_matches {
        return Ok(Some(StaticAbility::flying_only_restriction()));
    }

    let flying_or_reach_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
            "or",
            "reach",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ];

    if flying_or_reach_matches {
        return Ok(Some(StaticAbility::flying_restriction()));
    }

    Ok(None)
}

pub(crate) fn parse_can_block_only_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let matches = normalized.as_slice()
        == [
            "this",
            "can",
            "block",
            "only",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "can",
                "block",
                "only",
                "creatures",
                "with",
                "flying",
            ]
        || normalized.as_slice() == ["can", "block", "only", "creatures", "with", "flying"]
        || normalized.as_slice() == ["this", "can", "block", "only", "creature", "with", "flying"]
        || normalized.as_slice()
            == [
                "this", "creature", "can", "block", "only", "creature", "with", "flying",
            ];

    if matches {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub(crate) fn parse_assign_damage_as_unblocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    if normalized.first().copied() != Some("you") {
        return Ok(None);
    }

    let mut idx = 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"have") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) != Some(&"this") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"creature") {
        idx += 1;
    }

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "assign", "its", "combat", "damage", "as", "though", "it", "werent", "blocked",
        ] || tail
            == [
                "assign", "its", "combat", "damage", "as", "though", "it", "wasnt", "blocked",
            ];

    if matches {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

pub(crate) fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let mut idx = 0;
    if normalized.get(idx) != Some(&"you") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"cast") {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "noncreature",
            "spells",
            "as",
            "though",
            "they",
            "had",
            "flash",
        ] || tail
            == [
                "noncreature",
                "spells",
                "as",
                "though",
                "they",
                "have",
                "flash",
            ];

    if matches {
        return Ok(Some(StaticAbility::grants(
            crate::grant::GrantSpec::flash_to_noncreature_spells(),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some(attack_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
    else {
        return Ok(None);
    };
    if words[attack_idx..] != ["attacks", "each", "combat", "if", "able"]
        && words[attack_idx..] != ["attack", "each", "combat", "if", "able"]
    {
        return Ok(None);
    }

    if attack_idx == 0 {
        return Ok(Some(StaticAbility::must_attack()));
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbility::must_attack()));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => Ok(Some(StaticAbility::must_attack())),
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbility::grant_ability(
            filter,
            StaticAbility::must_attack(),
        ))),
    }
}

pub(crate) fn parse_additional_land_play_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["you", "may", "play"]) {
        return Ok(None);
    }

    let Some(count_token_idx) = token_index_for_word_index(tokens, 3) else {
        return Ok(None);
    };
    let Some((count, used)) = parse_number(&tokens[count_token_idx..]) else {
        return Ok(None);
    };
    let rest_word_idx = 3 + used;
    if rest_word_idx >= words.len() {
        return Ok(None);
    }
    let rest_words = &words[rest_word_idx..];
    let is_match = rest_words
        == ["additional", "land", "on", "each", "of", "your", "turns"]
        || rest_words
            == [
                "additional",
                "lands",
                "on",
                "each",
                "of",
                "your",
                "turns",
            ];
    if !is_match {
        return Ok(None);
    }
    if count == 0 {
        return Ok(None);
    }

    let abilities = (0..count)
        .map(|_| StaticAbility::additional_land_play())
        .collect::<Vec<_>>();
    if !abilities.is_empty() {
        return Ok(Some(abilities));
    }
    Ok(None)
}

pub(crate) fn parse_play_lands_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "may", "play", "lands", "from", "your", "graveyard"] {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            ObjectFilter::land(),
            Zone::Graveyard,
        );
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub(crate) fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }
    let power_str = parts[0].trim_start_matches('+');
    let toughness_str = parts[1].trim_start_matches('+');
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub(crate) fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = if let Some(rest) = trimmed.strip_prefix('+') {
        (1, rest)
    } else if let Some(rest) = trimmed.strip_prefix('-') {
        (-1, rest)
    } else {
        (1, trimmed)
    };

    if value_text.eq_ignore_ascii_case("x") {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub(crate) fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }

    let power = parse_signed_pt_component(parts[0])?;
    let toughness = parse_signed_pt_component(parts[1])?;
    Ok((power, toughness))
}

pub(crate) fn parse_no_maximum_hand_size_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "have", "no", "maximum", "hand", "size"] {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub(crate) fn parse_reduced_maximum_hand_size_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let (player, mut idx) = if words.starts_with(&["your"]) || words.starts_with(&["you"]) {
        (crate::target::PlayerFilter::You, 1usize)
    } else if words.starts_with(&["each", "opponent"]) || words.starts_with(&["each", "opponents"])
    {
        (crate::target::PlayerFilter::Opponent, 2usize)
    } else if words.starts_with(&["opponent"]) || words.starts_with(&["opponents"]) {
        (crate::target::PlayerFilter::Opponent, 1usize)
    } else if words.starts_with(&["each", "player"]) || words.starts_with(&["each", "players"]) {
        (crate::target::PlayerFilter::Any, 2usize)
    } else if words.starts_with(&["player"]) || words.starts_with(&["players"]) {
        (crate::target::PlayerFilter::Any, 1usize)
    } else {
        return Ok(None);
    };

    if words.get(idx..idx + 5) != Some(["maximum", "hand", "size", "is", "reduced"].as_slice()) {
        return Ok(None);
    }
    idx += 5;
    if words.get(idx) != Some(&"by") {
        return Ok(None);
    }
    idx += 1;

    let Some(amount_word) = words.get(idx) else {
        return Err(CardTextError::ParseError(format!(
            "missing maximum-hand-size reduction amount (clause: '{}')",
            words.join(" ")
        )));
    };
    let Some(amount) = parse_named_number(amount_word) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported maximum-hand-size reduction amount '{}' (clause: '{}')",
            amount_word,
            words.join(" ")
        )));
    };
    idx += 1;

    if idx != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::reduce_maximum_hand_size(
        player, amount,
    )))
}

pub(crate) fn parse_library_of_leng_discard_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_effect_causes = words.windows(3).any(|w| w == ["effect", "causes", "you"]);
    let has_discard = words.contains(&"discard");
    let has_top = words.contains(&"top");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_effect_causes && has_discard && has_top && has_library && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::library_of_leng_discard_replacement()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    if !words.starts_with(&["if", "you", "would", "draw", "a", "card"]) {
        return Ok(None);
    }

    let has_exile = words.contains(&"exile");
    let has_top_card = words.windows(2).any(|window| window == ["top", "card"]);
    let has_library = words.contains(&"library");
    let has_face_down = words.windows(2).any(|window| window == ["face", "down"]);
    let has_instead = words.contains(&"instead");

    if has_exile && has_top_card && has_library && has_face_down && has_instead {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub(crate) fn parse_toph_first_metalbender_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_nontoken = words.contains(&"nontoken");
    let has_artifact = words
        .iter()
        .any(|word| *word == "artifact" || *word == "artifacts");
    let has_you_control = words
        .windows(2)
        .any(|pair| pair == ["you", "control"] || pair == ["you", "controls"]);
    let has_land = words.iter().any(|word| *word == "land" || *word == "lands");
    let has_addition = words
        .windows(4)
        .any(|pair| pair == ["in", "addition", "to", "their"]);

    if has_nontoken && has_artifact && has_you_control && has_land && has_addition {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::TophFirstMetalbender,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_discard_or_redirect_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_enter = words
        .iter()
        .any(|word| *word == "enter" || *word == "enters");
    let has_battlefield = words.contains(&"battlefield");
    let has_discard = words.contains(&"discard");
    let has_land = words.contains(&"land");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_enter && has_battlefield && has_discard && has_land && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::discard_or_redirect_replacement(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_pay_life_or_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let starts_with_as_this = words.starts_with(&["as", "this"]);
    let has_pay = words.contains(&"pay");
    let has_life = words.contains(&"life");
    if !starts_with_as_this || !has_pay || !has_life {
        return Ok(None);
    }

    let Some(pay_idx) = tokens.iter().position(|token| token.is_word("pay")) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'pay' keyword in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };
    if !words[..pay_idx]
        .iter()
        .any(|word| *word == "enter" || *word == "enters")
    {
        return Ok(None);
    }
    if !words[..pay_idx].contains(&"may") {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB prefix (clause: '{}')",
            words.join(" ")
        )));
    }

    let Some((value, _)) = parse_number(&tokens[pay_idx + 1..]) else {
        return Err(CardTextError::ParseError(format!(
            "missing life payment amount in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let if_dont_idx = words
        .windows(3)
        .position(|window| window == ["if", "you", "dont"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
                words.join(" ")
            ))
        })?;

    let trailing = &words[if_dont_idx + 3..];
    let valid_trailing = trailing.starts_with(&["it", "enters", "tapped"])
        || trailing.starts_with(&["it", "enter", "tapped"])
        || trailing.starts_with(&["it", "enters", "the", "battlefield", "tapped"])
        || trailing.starts_with(&["it", "enter", "the", "battlefield", "tapped"]);
    if !valid_trailing {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(value)))
}

pub(crate) fn parse_copy_activated_abilities_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..words.len().saturating_sub(4) {
        if words[idx] == "has"
            && words[idx + 1] == "all"
            && words[idx + 2] == "activated"
            && words[idx + 3] == "abilities"
            && words[idx + 4] == "of"
        {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };

    let mut condition = None;
    let prefix = &words[..has_idx];
    if prefix.starts_with(&["as", "long", "as"])
        && prefix.contains(&"own")
        && prefix.contains(&"exiled")
        && prefix.contains(&"counter")
    {
        if let Some(counter_word) = prefix
            .iter()
            .zip(prefix.iter().skip(1))
            .find_map(|(word, next)| {
                if *next == "counter" {
                    Some(*word)
                } else {
                    None
                }
            })
            .and_then(parse_counter_type_word)
        {
            condition = Some(crate::ConditionExpr::OwnsCardExiledWithCounter(
                counter_word,
            ));
        }
    }

    let after_of = &words[(has_idx + 5)..];
    let mut filter = None;
    if after_of.contains(&"land") || after_of.contains(&"lands") {
        filter = Some(ObjectFilter::land());
    } else if after_of.contains(&"creature") || after_of.contains(&"creatures") {
        let mut base = ObjectFilter::creature();
        if after_of.contains(&"control") {
            base = base.you_control();
        }
        filter = Some(base);
    } else if after_of.contains(&"card") && after_of.contains(&"exiled") {
        filter = Some(ObjectFilter {
            zone: Some(Zone::Exile),
            ..Default::default()
        });
    }

    let Some(filter) = filter else {
        return Ok(None);
    };

    let counter = after_of
        .iter()
        .zip(after_of.iter().skip(1))
        .find_map(|(word, next)| {
            if *next == "counter" {
                parse_counter_type_word(word)
            } else {
                None
            }
        });

    let exclude_source_name = words.windows(5).any(|window| {
        window == ["same", "name", "as", "this", "creature"]
            || window == ["same", "name", "as", "thiss", "creature"]
    });

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(words.join(" "));
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }

    Ok(Some(StaticAbility::copy_activated_abilities(ability)))
}

pub(crate) fn parse_players_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&[
        "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color",
    ]) {
        return Ok(Some(StaticAbility::spend_mana_as_any_color_players()));
    }

    Ok(None)
}

pub(crate) fn parse_source_activation_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&[
        "you",
        "may",
        "spend",
        "mana",
        "as",
        "though",
        "it",
        "were",
        "mana",
        "of",
        "any",
        "color",
        "to",
        "pay",
        "the",
        "activation",
        "costs",
        "of",
    ]) {
        return Ok(None);
    }

    if words
        .iter()
        .any(|word| *word == "abilities" || *word == "ability")
    {
        return Ok(Some(
            StaticAbility::spend_mana_as_any_color_activation_costs(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_enchanted_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["enchanted"]) || !token_words.contains(&"has") {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let ability_tokens = &tokens[has_idx + 1..];
    let Some(parsed) = parse_activated_line(ability_tokens)? else {
        return Ok(None);
    };

    let mut ability = parsed.ability;
    if ability.text.is_none() {
        let text_words = words(ability_tokens).join(" ");
        ability.text = Some(text_words);
    }

    Ok(Some(StaticAbility::attached_ability_grant(
        ability,
        token_words.join(" "),
    )))
}

pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"target") {
        return Ok(None);
    }
    if subject_words.starts_with(&["until", "end", "of", "turn"])
        || subject_words.starts_with(&["until", "your", "next", "turn"])
    {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&tokens[has_idx + 1..]);
    let rest_words = words(&rest_tokens);
    if rest_words.len() < 8 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if rest_words[5] != "and" {
        return Ok(None);
    }
    if !matches!(rest_words[6], "has" | "have" | "gain" | "gains") {
        return Ok(None);
    }

    let Some(ability_start_idx) = token_index_for_word_index(&rest_tokens, 7) else {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let ability_tokens = trim_commas(&rest_tokens[ability_start_idx..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let granted = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect::<Vec<_>>();
    if granted.is_empty() {
        return Ok(None);
    }

    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut compiled = Vec::new();
    match subject {
        AnthemSubjectAst::Source => {
            compiled.push(StaticAbility::set_base_power_toughness(
                ObjectFilter::source(),
                power,
                toughness,
            ));
            compiled.extend(granted);
        }
        AnthemSubjectAst::Filter(filter) => {
            compiled.push(StaticAbility::set_base_power_toughness(
                filter.clone(),
                power,
                toughness,
            ));
            for ability in granted {
                compiled.push(StaticAbility::grant_ability(filter.clone(), ability));
            }
        }
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_filter_has_granted_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    if tokens[..has_idx]
        .iter()
        .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "deal"
                | "deals"
                | "create"
                | "creates"
                | "draw"
                | "draws"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "sacrifice"
                | "sacrifices"
                | "put"
                | "puts"
                | "gain"
                | "gains"
                | "lose"
                | "loses"
                | "discard"
                | "discards"
                | "counter"
                | "counters"
                | "search"
                | "reveals"
                | "investigate"
                | "investigates"
        )
    }) {
        return Ok(None);
    }
    if subject_words.contains(&"may") {
        return Ok(None);
    }
    let ability_tokens = &tokens[has_idx + 1..];
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    let mut granted_static: Option<StaticAbility> = None;
    let mut granted_ability: Option<Ability> = if has_colon {
        let Some(parsed) = parse_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Some(parsed.ability)
    } else if let Some(parsed) = parse_cycling_line(ability_tokens)? {
        Some(parsed.ability)
    } else if looks_like_trigger {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let (compiled_effects, choices) =
                    compile_trigger_effects(Some(&trigger), &effects)?;
                Some(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: compile_trigger_spec(trigger),
                        effects: compiled_effects,
                        choices,
                        intervening_if: max_triggers_per_turn
                            .map(crate::ConditionExpr::MaxTimesEachTurn),
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                })
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    } else if let Some(abilities) = parse_static_ability_line(ability_tokens)? {
        if abilities.len() != 1 {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted static ability chain (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        granted_static = Some(
            abilities
                .into_iter()
                .next()
                .expect("single granted static ability"),
        );
        None
    } else {
        return Ok(None);
    };
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };
    if let Some(static_ability) = granted_static {
        return Ok(Some(StaticAbility::grant_ability(filter, static_ability)));
    }
    let mut ability = granted_ability
        .take()
        .ok_or_else(|| CardTextError::ParseError("missing granted ability".to_string()))?;
    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }

    let attached_subject = subject_words
        .first()
        .is_some_and(|word| *word == "enchanted" || *word == "equipped");
    if attached_subject {
        return Ok(Some(StaticAbility::attached_ability_grant(
            ability,
            clause_words.join(" "),
        )));
    }

    Ok(Some(StaticAbility::grant_object_ability_for_filter(
        filter,
        ability,
        clause_words.join(" "),
    )))
}
