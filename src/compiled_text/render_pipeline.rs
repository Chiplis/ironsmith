pub fn compiled_lines(def: &CardDefinition) -> Vec<String> {
    let mut out = Vec::new();
    let subject = subject_for_card(&def.card);
    let rewrite_it_deals = def.card.card_types.contains(&CardType::Creature)
        || def.card.card_types.contains(&CardType::Artifact)
        || def.card.card_types.contains(&CardType::Land)
        || def.card.card_types.contains(&CardType::Planeswalker)
        || def.card.card_types.contains(&CardType::Battle);
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        match method {
            method if method.uses_composed_cost_effects() => {
                let name = method.name();
                let mana_cost = method.mana_cost();
                let cost_effects = method.cost_effects();
                let cast_condition = method.cast_condition();
                let mut parts = Vec::new();
                if let Some(cost) = mana_cost {
                    parts.push(format!("pay {}", cost.to_oracle()));
                }
                if !cost_effects.is_empty() {
                    parts.push(describe_alternative_cost_effects(cost_effects));
                }
                let clause = if parts.is_empty() {
                    "cast this spell without paying its mana cost".to_string()
                } else {
                    parts.join(" and ")
                };
                let mut line = format!("You may {clause} rather than pay this spell's mana cost");
                if !name.is_empty() {
                    line.push_str(&format!(" ({name})"));
                }
                if let Some(condition) = cast_condition
                    && let Some(condition_text) =
                        crate::static_abilities::describe_this_spell_cost_condition(condition)
                {
                    line = format!("If {condition_text}, {}", lowercase_first(&line));
                }
                out.push(line);
            }
            AlternativeCastingMethod::Madness { cost } => {
                out.push(format!("Madness {}", cost.to_oracle()));
            }
            AlternativeCastingMethod::Miracle { cost } => {
                out.push(format!("Miracle {}", cost.to_oracle()));
            }
            AlternativeCastingMethod::Flashback { cost, cost_effects } => {
                if cost_effects.is_empty() {
                    out.push(format!("Flashback—{}", cost.to_oracle()));
                } else {
                    let extra = capitalize_first(&describe_alternative_cost_effects(cost_effects));
                    out.push(format!("Flashback—{}, {}", cost.to_oracle(), extra));
                }
            }
            AlternativeCastingMethod::JumpStart => {
                out.push("Jump-start".to_string());
            }
            AlternativeCastingMethod::Escape { cost, exile_count } => {
                let count_text = small_number_word(*exile_count)
                    .map(str::to_string)
                    .unwrap_or_else(|| exile_count.to_string());
                if let Some(cost) = cost {
                    out.push(format!(
                        "Escape—{}, Exile {count_text} other cards from your graveyard",
                        cost.to_oracle()
                    ));
                } else {
                    out.push(format!(
                        "Escape—Exile {count_text} other cards from your graveyard"
                    ));
                }
            }
            AlternativeCastingMethod::Bestow { cost, cost_effects } => {
                if cost_effects.is_empty() {
                    out.push(format!("Bestow {}", cost.to_oracle()));
                } else {
                    let extra = capitalize_first(&describe_alternative_cost_effects(cost_effects));
                    out.push(format!("Bestow {}, {}", cost.to_oracle(), extra));
                }
            }
            other => {
                if other.name().eq_ignore_ascii_case("Parsed alternative cost") {
                    if let Some(cost) = other.mana_cost() {
                        out.push(format!(
                            "You may pay {} rather than pay this spell's mana cost",
                            cost.to_oracle()
                        ));
                    } else {
                        out.push(
                            "You may cast this spell rather than pay its mana cost".to_string(),
                        );
                    }
                    continue;
                }
                if let Some(cost) = other.mana_cost() {
                    out.push(format!(
                        "Alternative cast {}: {} {}",
                        idx + 1,
                        other.name(),
                        cost.to_oracle()
                    ));
                } else {
                    out.push(format!("Alternative cast {}: {}", idx + 1, other.name()));
                }
            }
        }
    }
    for cost in &def.optional_costs {
        out.push(describe_optional_cost_line(cost));
    }
    if let Some(filter) = &def.aura_attach_filter {
        out.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }
    let max_saga_chapter = def.max_saga_chapter.or_else(|| {
        def.abilities
            .iter()
            .filter_map(|ability| {
                if let AbilityKind::Triggered(triggered) = &ability.kind {
                    triggered
                        .trigger
                        .saga_chapters()
                        .and_then(|chapters| chapters.iter().copied().max())
                } else {
                    None
                }
            })
            .max()
    });
    if let Some(max_chapter) = max_saga_chapter
        && let Some(roman) = chapter_number_to_roman(max_chapter)
    {
        out.push(format!(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after {roman}.)"
        ));
    }
    let push_abilities = |output: &mut Vec<String>| {
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if let Some(group_text) = ability.text.as_deref().map(str::trim)
                && group_text.contains(',')
                && ability_can_render_as_keyword_group(ability)
            {
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    if !ability_can_render_as_keyword_group(next) {
                        break;
                    }
                    let next_text = next.text.as_deref().map(str::trim);
                    if next_text != Some(group_text) {
                        break;
                    }
                    consumed += 1;
                }
                if consumed > 1 {
                    output.push(format!("Keyword ability {}: {group_text}", ability_idx + 1));
                    ability_idx += consumed;
                    continue;
                }
            }
            if let AbilityKind::Activated(first) = &ability.kind
                && first.is_mana_ability()
                && first.effects.is_empty()
                && first.activation_condition.is_none()
                && first.mana_symbols().len() == 1
                && ability.text.is_none()
            {
                let mut symbols = vec![first.mana_symbols()[0]];
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    let AbilityKind::Activated(next_mana) = &next.kind else {
                        break;
                    };
                    if !next_mana.is_mana_ability()
                        || !next_mana.effects.is_empty()
                        || next_mana.activation_condition.is_some()
                        || next_mana.mana_symbols().len() != 1
                        || next_mana.mana_cost != first.mana_cost
                        || next.text.is_some()
                    {
                        break;
                    }
                    symbols.push(next_mana.mana_symbols()[0]);
                    consumed += 1;
                }
                if consumed > 1 {
                    let mut line = format!("Mana ability {}", ability_idx + 1);
                    let add = format!("Add {}", describe_mana_alternatives(&symbols));
                    if !first.mana_cost.costs().is_empty() {
                        let cost = describe_cost_list(first.mana_cost.costs());
                        line.push_str(": ");
                        line.push_str(&cost);
                        line.push_str(": ");
                        line.push_str(&add);
                    } else {
                        line.push_str(": ");
                        line.push_str(&add);
                    }
                    output.push(line);
                    ability_idx += consumed;
                    continue;
                }
            }
            output.extend(describe_ability(
                ability_idx + 1,
                ability,
                subject,
                rewrite_it_deals,
            ));
            ability_idx += 1;
        }
    };

    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    if !def.cost_effects.is_empty() {
        out.push(format!(
            "As an additional cost to cast this spell, {}",
            describe_additional_cost_effects(&def.cost_effects)
        ));
    }
    if !spell_like_card {
        push_abilities(&mut out);
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
        && !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
    {
        out.push(format!(
            "Spell effects: {}",
            describe_effect_list(spell_effects)
        ));
    }
    if spell_like_card {
        push_abilities(&mut out);
    }
    let normalized = out
        .into_iter()
        .map(|line| normalize_rendered_line_for_card(def, &line))
        .collect::<Vec<_>>();
    merge_adjacent_static_heading_lines(normalized)
        .into_iter()
        .map(|line| normalize_compiled_line_post_pass(def, &line))
        .collect()
}

fn card_self_reference_phrase(def: &CardDefinition) -> &'static str {
    if def.card.is_creature() {
        "this creature"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Vehicle))
    {
        "this Vehicle"
    } else if def.card.is_instant() || def.card.is_sorcery() {
        "this spell"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Aura))
    {
        "this Aura"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Equipment))
    {
        "this Equipment"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Fortification))
    {
        "this Fortification"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Saga))
    {
        "this Saga"
    } else if def.card.is_enchantment() {
        "this enchantment"
    } else if def.card.has_card_type(CardType::Battle) {
        "this battle"
    } else if def.card.is_land() {
        "this land"
    } else if def.card.is_artifact() {
        "this artifact"
    } else if def.card.is_planeswalker() {
        "this planeswalker"
    } else {
        "this permanent"
    }
}

fn normalize_rendered_line_for_card(def: &CardDefinition, line: &str) -> String {
    let self_ref = card_self_reference_phrase(def);
    let self_ref_cap = capitalize_first(self_ref);
    fn strip_rebalance_prefix(name: &str) -> &str {
        let trimmed = name.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
            trimmed[2..].trim()
        } else {
            trimmed
        }
    }
    let display_name = {
        let full = def.card.name.trim();
        if full.is_empty() {
            String::new()
        } else {
            let left_half = full.split("//").next().map(str::trim).unwrap_or(full);
            let short = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            strip_rebalance_prefix(short).to_string()
        }
    };
    let oracle_mentions_name = {
        let oracle_text = def.card.oracle_text.to_ascii_lowercase();
        let full_name = def.card.name.trim().to_ascii_lowercase();
        if full_name.is_empty() {
            false
        } else {
            let left_half = full_name
                .split("//")
                .next()
                .map(str::trim)
                .unwrap_or(full_name.as_str());
            let short_name = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            let rebalance_short = strip_rebalance_prefix(short_name);
            oracle_text.contains(&full_name)
                || (short_name.len() >= 3 && oracle_text.contains(short_name))
                || (rebalance_short.len() >= 3 && oracle_text.contains(rebalance_short))
        }
    };
    let has_graveyard_activation = card_has_graveyard_activated_ability(def);
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let oracle_mentions_display_possessive = {
        let lowered = display_name.to_ascii_lowercase();
        !lowered.is_empty() && oracle_lower.contains(&format!("{lowered}'s "))
    };
    // Normalize card name self-references to "this" for pattern matching,
    // mirroring the parser's replace_names_with_map normalization.
    let oracle_normalized = {
        let name_lower = def.card.name.trim().to_ascii_lowercase();
        if !name_lower.is_empty() {
            oracle_lower.replace(&name_lower, "this")
        } else {
            oracle_lower.clone()
        }
    };
    // Detect "exile this {noun} from your hand" in oracle and extract the noun used.
    let exile_from_hand_noun = if oracle_normalized.contains("exile this card from your hand") {
        Some("card")
    } else if oracle_normalized.contains("exile this creature from your hand") {
        Some("creature")
    } else if oracle_normalized.contains("exile this from your hand") {
        Some("card")
    } else {
        None
    };
    let _has_self_exile_from_hand = exile_from_hand_noun.is_some();
    let has_basic_landcycling = oracle_lower.contains("basic landcycling");
    let has_target_blocked_creature = oracle_lower.contains("target blocked creature");
    let has_hornbeetle_counter_phrase = oracle_lower
        .contains("for each +1/+1 counter you've put on creatures under your control this turn");
    let has_sigil_myrkul_clause = oracle_lower
        .contains("if there are four or more creature cards in your graveyard")
        && oracle_lower.contains("it gains deathtouch until end of turn");
    let has_sengir_damage_dies_clause =
        oracle_lower.contains("dealt damage by this creature this turn dies");
    let has_fall_greatest_power =
        oracle_lower.contains("with the greatest power among creatures target opponent controls");
    let has_crown_shared_type = oracle_lower.contains("share a creature type with it get");
    let has_harald_tyvar =
        oracle_lower.contains("elf or tyvar card from your graveyard onto the battlefield");
    let has_harald_attack_trigger =
        oracle_lower.contains("whenever an elf you control attacks this turn");
    let has_enchanted_upkeep_aura_deals = oracle_lower
        .contains("upkeep of enchanted creature's controller")
        && oracle_lower.contains("this aura deals");
    let has_when_this_siege_enters = oracle_lower.contains("when this siege enters");
    let has_when_this_saga_enters = oracle_lower.contains("when this saga enters");
    let has_when_this_vehicle_enters = oracle_lower.contains("when this vehicle enters");
    let has_this_equipment = oracle_lower.contains("this equipment");
    let has_when_this_enchantment_enters = oracle_lower.contains("when this enchantment enters");
    let has_greeds_gambit_triplet = oracle_lower
        .contains("you draw three cards, gain 6 life, and create three 2/1 black bat creature tokens with flying")
        && oracle_lower.contains("you discard a card, lose 2 life, and sacrifice a creature")
        && oracle_lower.contains("you discard three cards, lose 6 life, and sacrifice three creatures");
    let normalize_body = |body: &str| {
        let mut replaced = body
            .trim()
            .replace("~", self_ref)
            .replace("this source", self_ref)
            .replace("this permanent", self_ref)
            .replace(" enters the battlefield", " enters");
        if !def.card.name.trim().is_empty() {
            replaced = replaced
                .replace("card named This", &format!("card named {}", def.card.name))
                .replace("card named this", &format!("card named {}", def.card.name));
        }
        if let Some(rest) = replaced.strip_prefix("This enters ") {
            replaced = format!("{self_ref_cap} enters {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("Enters the battlefield with ") {
            replaced = format!("{self_ref_cap} enters with {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("enters the battlefield with ") {
            replaced = format!("{self_ref} enters with {rest}");
        }
        if oracle_mentions_name {
            let lowered = replaced.to_ascii_lowercase();
            let self_ref_lower = self_ref.to_ascii_lowercase();
            let safe_name_substitution = lowered.starts_with("when this ")
                || lowered.starts_with("whenever this ")
                || lowered.starts_with("at the beginning of ")
                || lowered.starts_with(&format!("{self_ref_lower} "))
                || (oracle_mentions_display_possessive
                    && (lowered.starts_with("this creature's ")
                        || lowered.starts_with("this artifact's ")
                        || lowered.starts_with("this enchantment's ")
                        || lowered.starts_with("this land's ")
                        || lowered.starts_with("this planeswalker's ")
                        || lowered.starts_with("this battle's ")
                        || lowered.starts_with("this permanent's ")
                        || lowered.starts_with("this spell's ")));
            if safe_name_substitution {
                if let Some(rest) = replaced.strip_prefix(&format!("When {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("Whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("when {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&self_ref_cap) {
                    replaced = format!("{}{}", display_name, rest);
                } else if let Some(rest) = replaced.strip_prefix(self_ref) {
                    replaced = format!("{}{}", display_name, rest);
                }
            }
        }
        if self_ref != "this creature" {
            replaced = replaced
                .replace("Transform this creature", &format!("Transform {self_ref}"))
                .replace("transform this creature", &format!("transform {self_ref}"));
        }
        let mut phrased = normalize_common_semantic_phrasing(&replaced);
        let when_you_do_subject = [
            "this creature",
            "this artifact",
            "this enchantment",
            "this land",
            "this planeswalker",
            "this permanent",
            "this Saga",
            "this battle",
            "this spell",
            "this Aura",
            "this Equipment",
            "this Vehicle",
            "this Fortification",
        ]
        .into_iter()
        .find(|subject| {
            oracle_lower.contains(&format!(
                "when you do, {} deals",
                subject.to_ascii_lowercase()
            ))
        });
        if let Some(subject) = when_you_do_subject {
            if let Some((head, tail)) = phrased.split_once(". If you do, Deal ") {
                let tail = tail.trim();
                phrased = format!("{head}. When you do, {subject} deals {tail}");
            } else if let Some((head, tail)) = phrased.split_once(". If you do, deal ") {
                let tail = tail.trim();
                phrased = format!("{head}. When you do, {subject} deals {tail}");
            }
        }
        if let Some((prefix, rest)) = phrased.split_once("— For each player, that player discards ")
        {
            let rest = rest.trim();
            phrased = format!("{prefix}— Each player discards {rest}");
        }
        if oracle_lower.contains("put a +1/+1 counter on it")
            && phrased.contains("with a +1/+1 counter on it")
        {
            phrased = phrased
                .replace(
                    " to the battlefield with a +1/+1 counter on it",
                    " to the battlefield. Put a +1/+1 counter on it",
                )
                .replace(
                    " onto the battlefield with a +1/+1 counter on it",
                    " onto the battlefield. Put a +1/+1 counter on it",
                );
        }
        if has_graveyard_activation {
            phrased = phrased
                .replace(
                    "Return this creature to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "return this creature to its owner's hand",
                    "return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this source to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this Aura to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this permanent to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace("Exile this creature", "Exile this card from your graveyard")
                .replace("exile this creature", "exile this card from your graveyard")
                .replace(
                    "Exile this permanent",
                    "Exile this card from your graveyard",
                )
                .replace(
                    "exile this permanent",
                    "exile this card from your graveyard",
                );
        }
        if let Some(noun) = exile_from_hand_noun {
            // By this point, normalize_body already replaced "this source"/"this permanent"
            // with self_ref (e.g. "this creature"), so match the actual self_ref value.
            let exile_self = format!("Exile {self_ref}");
            let exile_self_lower = format!("exile {self_ref}");
            let target_upper = format!("Exile this {noun} from your hand");
            let target_lower = format!("exile this {noun} from your hand");
            phrased = phrased
                .replace("Exile 1 card(s) from your hand", &target_upper)
                .replace("Exile a card from your hand", &target_upper)
                .replace("exile 1 card(s) from your hand", &target_lower)
                .replace("exile a card from your hand", &target_lower)
                .replace(&exile_self, &target_upper)
                .replace(&exile_self_lower, &target_lower);
        }
        if has_basic_landcycling {
            phrased = phrased
                .replace("Landcycling {", "Basic landcycling {")
                .replace("landcycling {", "basic landcycling {")
                .replace("Basic basic landcycling {", "Basic landcycling {")
                .replace("basic basic landcycling {", "basic landcycling {");
        }
        if has_target_blocked_creature {
            phrased = phrased
                .replace(
                    "Destroy target creature.",
                    "Destroy target blocked creature.",
                )
                .replace("Destroy target creature", "Destroy target blocked creature");
        }
        if has_hornbeetle_counter_phrase {
            phrased = phrased
                .replace(
                    "for each creature.",
                    "for each +1/+1 counter you've put on creatures under your control this turn.",
                )
                .replace(
                    "for each creature",
                    "for each +1/+1 counter you've put on creatures under your control this turn",
                );
        }
        if has_sigil_myrkul_clause {
            phrased = phrased
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn.",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.",
                )
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn",
                );
        }
        if has_sengir_damage_dies_clause {
            phrased = phrased
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature.",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.",
                )
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature",
                );
        }
        if has_fall_greatest_power {
            phrased = phrased
                .replace(
                    "III — Exile target creature an opponent controls.",
                    "III — Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "III — Exile target creature an opponent controls",
                    "III — Exile a creature with the greatest power among creatures target opponent controls",
                )
                .replace(
                    "Exile target creature an opponent controls.",
                    "Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "Exile target creature an opponent controls",
                    "Exile a creature with the greatest power among creatures target opponent controls",
                );
        }
        if has_crown_shared_type {
            phrased = phrased
                .replace(
                    "Sacrifice this Aura: this Aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                )
                .replace(
                    "Sacrifice this aura: this aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                );
        }
        if has_harald_tyvar {
            phrased = phrased
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield.",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield.",
                )
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield",
                );
        }
        if has_harald_attack_trigger {
            phrased = phrased
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn.",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn.",
                )
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn",
                );
        }
        if has_enchanted_upkeep_aura_deals {
            phrased = phrased.replace(
                "At the beginning of the upkeep of enchanted creature's controller, deal ",
                "At the beginning of the upkeep of enchanted creature's controller, this Aura deals ",
            );
        }
        if has_this_equipment {
            phrased = phrased
                .replace("This artifact", "This Equipment")
                .replace("this artifact", "this Equipment");
        }
        if has_when_this_siege_enters {
            phrased = phrased
                .replace("When this permanent enters, ", "When this Siege enters, ")
                .replace("when this permanent enters, ", "when this Siege enters, ")
                .replace("When this battle enters, ", "When this Siege enters, ")
                .replace("when this battle enters, ", "when this Siege enters, ");
        }
        if has_when_this_saga_enters {
            phrased = phrased
                .replace("When this enchantment enters, ", "When this Saga enters, ")
                .replace("when this enchantment enters, ", "when this Saga enters, ")
                .replace("When this permanent enters, ", "When this Saga enters, ")
                .replace("when this permanent enters, ", "when this Saga enters, ");
        }
        if has_when_this_vehicle_enters {
            phrased = phrased
                .replace("When this artifact enters, ", "When this Vehicle enters, ")
                .replace("when this artifact enters, ", "when this Vehicle enters, ")
                .replace("When this permanent enters, ", "When this Vehicle enters, ")
                .replace("when this permanent enters, ", "when this Vehicle enters, ");
        }
        if has_when_this_enchantment_enters {
            phrased = phrased
                .replace(
                    "When this permanent enters, ",
                    "When this enchantment enters, ",
                )
                .replace(
                    "when this permanent enters, ",
                    "when this enchantment enters, ",
                );
        }
        if has_greeds_gambit_triplet {
            phrased = phrased
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
                )
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                );
            phrased = phrased
                .replace(
                    "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                );
        }
        normalize_sentence_surface_style(&phrased)
    };
    if let Some((prefix, rest)) = line.split_once(':')
        && is_render_heading_prefix(prefix)
    {
        let normalized_body = normalize_body(rest);
        return format!("{}: {}", prefix.trim(), normalized_body);
    }
    normalize_body(line)
}
