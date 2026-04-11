fn parse_graveyard_threshold_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let (count, tail_start, constrained_player) = if filtered.len() >= 5
        && filtered[0] == "there"
        && filtered[1] == "are"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, None)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "have"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, Some(PlayerAst::You))
    } else {
        return Ok(None);
    };

    let tail = &filtered[tail_start..];
    let Some(in_idx) = rfind_index(tail, |word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 1 >= tail.len() {
        return Ok(None);
    }

    let graveyard_owner_words = &tail[in_idx + 1..];
    let player = match graveyard_owner_words {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let raw_filter_words = &tail[..in_idx];
    if raw_filter_words.is_empty()
        || slice_contains(raw_filter_words, &"type")
        || slice_contains(raw_filter_words, &"types")
    {
        return Ok(None);
    }

    let mut normalized_filter_words = Vec::with_capacity(raw_filter_words.len());
    for (idx, word) in raw_filter_words.iter().enumerate() {
        if *word == "and"
            && raw_filter_words
                .get(idx + 1)
                .is_some_and(|next| *next == "or")
        {
            continue;
        }
        normalized_filter_words.push(*word);
    }
    if normalized_filter_words.is_empty() {
        return Ok(None);
    }

    let mut filter = if matches!(normalized_filter_words.as_slice(), ["card"] | ["cards"]) {
        ObjectFilter::default()
    } else {
        let filter_tokens = normalized_filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);

    Ok(Some(PredicateAst::PlayerControlsAtLeast {
        player,
        filter,
        count,
    }))
}

fn parse_mana_spent_to_cast_predicate(words: &[&str]) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 || words[0] != "at" || words[1] != "least" {
        return None;
    }

    let amount_tokens = vec![OwnedLexToken::word(
        words[2].to_string(),
        TextSpan::synthetic(),
    )];
    let (amount, _) = parse_number(&amount_tokens)?;

    let mut idx = 3;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    let tail = &words[idx..];
    let canonical_tail = ["mana", "was", "spent", "to", "cast", "this", "spell"];
    let plural_tail = ["mana", "were", "spent", "to", "cast", "this", "spell"];
    if tail == canonical_tail || tail == plural_tail {
        return Some((amount, symbol));
    }

    None
}

fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    parse_mana_symbol_word_flexible(word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::compiler::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn parse_object_filter_lexed_handles_with_keyword_disjunction() {
        let tokens = lex_line("creatures with flying or reach", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(
            filter.any_of[0].static_abilities,
            vec![StaticAbilityId::Flying]
        );
        assert_eq!(
            filter.any_of[1].static_abilities,
            vec![StaticAbilityId::Reach]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_without_keyword_clause() {
        let tokens = lex_line("creatures without flying", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_static_abilities,
            vec![StaticAbilityId::Flying]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_with_no_abilities_clause() {
        let tokens = lex_line("creatures with no abilities", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.no_abilities);
    }

    #[test]
    fn parse_object_filter_lexed_handles_joint_owner_controller_clause() {
        let tokens = lex_line("permanents you both own and control", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_handles_chosen_player_graveyard_clause() {
        let tokens = lex_line("artifact card in the chosen player's graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.owner, Some(PlayerFilter::ChosenPlayer));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_owner_or_controller_disjunction() {
        let tokens = lex_line("artifacts target opponent owns or controls", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Artifact]);
        assert_eq!(
            filter.any_of[0].owner,
            Some(PlayerFilter::target_opponent())
        );
        assert_eq!(filter.any_of[0].controller, None);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Artifact]);
        assert_eq!(filter.any_of[1].owner, None);
        assert_eq!(
            filter.any_of[1].controller,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_target_player_reference_clause() {
        let tokens = lex_line("spell that targets player", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.targets_player, Some(PlayerFilter::Any));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attacking_target_opponent_clause() {
        let tokens = lex_line("creature attacking target opponent", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn temporal_graveyard_from_battlefield_phrase_parser_matches() {
        assert_eq!(
            parse_graveyard_from_battlefield_this_turn_words(&[
                "graveyard",
                "from",
                "battlefield",
                "this",
                "turn",
            ]),
            Some(5)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_put_there_from_anywhere_this_turn_clause() {
        let tokens = lex_line(
            "creature cards in a graveyard that were put there from anywhere this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.entered_graveyard_this_turn);
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_battlefield_this_turn_clause() {
        let tokens = lex_line(
            "creatures that entered the battlefield under your control this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.entered_battlefield_this_turn);
        assert_eq!(
            filter.entered_battlefield_controller,
            Some(PlayerFilter::You)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card not named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.excluded_name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tagged_reference_prefix() {
        let tokens = lex_line("that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.tagged_constraints,
            vec![TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            }]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_since_your_last_turn_ended_clause() {
        let tokens = lex_line("creatures that entered since your last turn ended", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.entered_since_your_last_turn_ended);
    }

    #[test]
    fn parse_object_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("face up creature cards", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.face_down, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_single_graveyard_phrase() {
        let tokens = lex_line("creature cards in a single graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.single_graveyard);
    }

    #[test]
    fn parse_object_filter_lexed_handles_one_or_more_colors_phrase() {
        let tokens = lex_line("creatures of one or more colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.colors, Some(any_color));
    }

    #[test]
    fn parse_object_filter_lexed_handles_mana_value_eq_counters_on_source_clause() {
        let tokens = lex_line(
            "creature card with mana value equal to the number of charge counters on this artifact",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.mana_value_eq_counters_on_source,
            Some(crate::object::CounterType::Charge)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_exclusion_phrase() {
        let tokens = lex_line("creatures other than enchanted creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("enchanted"),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_different_one_of_prefix() {
        let tokens = lex_line("different one of those creatures", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_pt_literal_prefix() {
        let tokens = lex_line("2/2 creature token", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(2)));
        assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(2)));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_all_colors_clause() {
        let tokens = lex_line("creature that isnt all colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.all_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_exactly_two_colors_clause() {
        let tokens = lex_line("creature that isnt exactly two colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.exactly_two_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_to_tagged_reference() {
        let tokens = lex_line("creature attached to it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_its_attached_to_reference_alias() {
        let tokens = lex_line("creature its attached to", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_source_linked_exile_reference() {
        let tokens = lex_line("spell exiled with this", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_mana_value_as_sacrificed_reference() {
        let tokens = lex_line(
            "creature with same mana value as the sacrificed creature",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("sacrifice_cost_0"),
                    relation: TaggedOpbjectRelation::SameManaValueAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_name_as_tagged_reference() {
        let tokens = lex_line("creature with same name as that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tap_activated_ability_phrase() {
        let tokens = lex_line(
            "creature with activated abilities with {T} in their costs",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.has_tap_activated_ability);
    }

    #[test]
    fn parse_object_filter_lexed_handles_convoked_it_reference() {
        let tokens = lex_line("creature that convoked it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("convoked_this_spell"),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_spell_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("Face up noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.face_down, Some(false));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_raw_handles_hyphenated_face_state_words() {
        let tokens = lex_line("face-down noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint(&tokens);
        assert_eq!(filter.face_down, Some(true));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_lexed_builds_power_or_toughness_disjunction() {
        let tokens = lex_line("creature spells with power or toughness 2 or less", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[0].power.is_some());
        assert!(filter.any_of[0].toughness.is_none());
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].power.is_none());
        assert!(filter.any_of[1].toughness.is_some());
    }

    #[test]
    fn parse_spell_filter_lexed_handles_even_mana_value_phrase() {
        let tokens = lex_line("even mana value spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(
            filter.mana_value_parity,
            Some(crate::filter::ParityRequirement::Even)
        );
    }
}
