//! Ordinary rules definitions use behavioral primitives, never named-keyword
//! identity. Recognized official shorthand is expanded to its full rules;
//! arbitrary custom prose retains its literal meaning.
use super::*;

fn source(words: &[&str]) -> bool {
    crate::util::is_source_reference_words(words) || words == ["it"]
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<Vec<StaticAbilityAst>> {
    let lowered = tokens.iter().cloned().map(|mut token| {
        if token.kind == TokenKind::Word { token.replace_word(token.slice.to_ascii_lowercase()); }
        token
    }).collect::<Vec<_>>();
    let tokens = crate::util::trim_statement_separators(&lowered);
    let words = crate::lexer::token_word_refs(tokens);
    let rendered = crate::lexer::render_token_slice(tokens);
    if rendered.to_ascii_lowercase().trim_end_matches('.') == "as this creature attacks, you may tap up to one untapped creature you control that you didn't choose to attack with and that either has haste or has been under your control continuously since this turn began. when you do, this creature gets +x/+0 until end of turn, where x is the tapped creature's power" {
        let trigger = ironsmith_core::TriggeredAbility {
            trigger: crate::model::ast::TriggerSpec::ThisAttacks,
            effects: ironsmith_core::ResolutionProgram::from_effects(vec![
                EffectAst::subject_verb_pump(
                    crate::effect::Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(
                        ironsmith_compiler_semantic::tag::declared_key("enlisted_creature").into()))),
                    crate::effect::Value::Fixed(0), crate::TargetAst::Source(None),
                    crate::effect::Until::EndOfTurn, None),
            ]),
            choices: Vec::new(), intervening_if: None, presentation_label: None,
        };
        let mut ability = StaticAbility::enlist_attack(trigger, rendered.clone());
        ability.id = None;
        return Some(vec![StaticAbilityAst::Static(ability)]);
    }
    let lower = rendered.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("before the game begins, you may reveal this card from outside the game as your chosen companion if ")
        && let Some(condition) = rest.strip_suffix(". once during the game, during a main phase of your turn while you have priority and the stack is empty, you may pay {3} to put your chosen companion into your hand. this is a special action that doesn't use the stack") {
        let condition_tokens = crate::lexer::lex_line(condition, 0).ok()?;
        let condition = crate::grammar::keyword_static_lines::parse_companion_deck_condition_tokens(&condition_tokens)?;
        let mut ability = StaticAbility::companion(condition, rendered.clone());
        ability.id = Some(ironsmith_core::StaticAbilityId::KeywordMarker);
        return Some(vec![StaticAbilityAst::Static(ability)]);
    }
    if let Some(abilities) = parse_entry_counter_or_ability(tokens) { return Some(abilities); }
    let compact_reminder: String = rendered.to_ascii_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    for color in ["w", "u", "b", "r", "g"] {
        let expected = format!("{{{color}/p}}canbepaidwith{{{color}}}or2life.foreach{{{color}/p}}paidwithlife,thisplaneswalkerenterswithtwofewerloyaltycounters");
        if compact_reminder.trim_end_matches('.') == expected {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker("For each Phyrexian mana symbol paid with life, this planeswalker enters with two fewer loyalty counters."))]);
        }
    }
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    if sentences.len() == 2 {
        let first = crate::lexer::render_token_slice(sentences[0]).to_ascii_lowercase();
        let compact: String = first.chars().filter(|c| !c.is_whitespace() && *c != '.').collect();
        let ward = match compact.as_str() {
            "youmaycastthiscardfacedownasa2/2creaturefor{3}" | "youmaycastthiscardfacedownfor{3}asa2/2creature" => Some(false),
            "youmaycastthiscardfacedownfor{3}asa2/2creaturewithward{2}" | "youmaycastthiscardfacedownasa2/2creaturewithward{2}for{3}" => Some(true),
            _ => None,
        };
        if let Some(ward) = ward {
            let second = crate::lexer::render_token_slice(sentences[1]).to_ascii_lowercase();
            if let Some(price) = second.trim_end_matches('.').strip_prefix("turn it face up any time for ") {
                let counter_tail = " and put a +1/+1 counter on it";
                let counter = price.ends_with(counter_tail);
                let price = price.strip_suffix(counter_tail).unwrap_or(price);
                if !ward || !counter {
                    let cost_tokens = crate::lexer::lex_line(price, 0).ok()?;
                    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&cost_tokens).ok()?;
                    let mut ability = if ward { StaticAbility::disguise(cost) } else if counter { StaticAbility::megamorph(cost) } else { StaticAbility::morph(cost) };
                    ability.id = None; ability.label = rendered.clone();
                    return Some(vec![StaticAbilityAst::Static(ability)]);
                }
            }
        }
    }

    // CR 702.16: every printed DEBT enumeration describes the same protection
    // rules; the quality reuses the protection grammar so any quality works.
    if let Some(quality) = [
        &["this", "creature", "can't", "be", "blocked", "targeted", "dealt", "damage", "or", "enchanted", "by", "anything"][..],
        &["this", "creature", "can't", "be", "blocked", "targeted", "dealt", "damage", "enchanted", "or", "equipped", "by", "anything"],
        &["this", "creature", "can't", "be", "blocked", "targeted", "dealt", "damage", "enchanted", "equipped", "or", "fortified", "by", "anything"],
    ].iter().find_map(|prefix| words.strip_prefix(*prefix)) && !quality.is_empty() {
        let mut colors = crate::color::ColorSet::new();
        let color_quality = quality.iter().filter(|word| **word != "or")
            .try_for_each(|word| { colors = colors.union(crate::util::parse_color(word)?); Some(()) });
        let protection = if color_quality.is_some() && !colors.is_empty() {
            StaticAbility::protection(crate::ability::ProtectionFrom::Color(colors))
        } else {
            let tokens = crate::lexer::lex_line(&format!("protection from {}", quality.join(" ")), 0).ok()?;
            let [action] = <[_; 1]>::try_from(crate::clause_support::parse_protection_chain(&tokens)?).ok()?;
            crate::static_ability_helpers::static_ability_for_keyword_action(action)?
        };
        let mut ability = protection;
        ability.id = None;
        ability.label = format!("This creature can't be blocked, targeted, dealt damage, enchanted, equipped, or fortified by anything {}.", quality.join(" "));
        return Some(vec![StaticAbilityAst::Static(ability)]);
    }


    if ironsmith_core::keyword_marker_mechanic(&rendered) == Some(ironsmith_core::KeywordMarkerMechanic::EntryCountersFromSpentColors) {
        return Some(vec![
            StaticAbilityAst::Static(StaticAbility::keyword_marker(rendered)),
            StaticAbilityAst::Static(StaticAbility::enters_with_counters_value(
                ironsmith_core::CounterType::Charge, crate::effect::Value::ColorsOfManaSpentToCastThisSpell)),
        ]);
    }
    if matches!(ironsmith_core::keyword_marker_mechanic(&rendered), Some(ironsmith_core::KeywordMarkerMechanic::RevealSharedTypesForEntryCounters { .. } | ironsmith_core::KeywordMarkerMechanic::CastBothSplitHalvesFromHand | ironsmith_core::KeywordMarkerMechanic::LinkOnGraveyardArrival | ironsmith_core::KeywordMarkerMechanic::RecurringCopyOnFirstNameResolution | ironsmith_core::KeywordMarkerMechanic::EntryCountersAndFollowingAbilities { .. } | ironsmith_core::KeywordMarkerMechanic::CombatBandsAndDamageChoice | ironsmith_core::KeywordMarkerMechanic::AssignCreatureSectors | ironsmith_core::KeywordMarkerMechanic::InitializeControllerSpeed | ironsmith_core::KeywordMarkerMechanic::ChooseSagaEntryChapter | ironsmith_core::KeywordMarkerMechanic::AnotherPlayerPaysGeneric | ironsmith_core::KeywordMarkerMechanic::DayNightTransformDefinition { .. } | ironsmith_core::KeywordMarkerMechanic::EntryLoyaltyReductionPerLifePip | ironsmith_core::KeywordMarkerMechanic::ExileOnStackExitIfCastFrom { .. } | ironsmith_core::KeywordMarkerMechanic::PermanentCountGrantsDesignation { .. } | ironsmith_core::KeywordMarkerMechanic::AttachWhileCreature | ironsmith_core::KeywordMarkerMechanic::DestroyAttachmentInstead)) {
        return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(rendered))]);
    }

    if matches!(words.as_slice(), ["while", "attached", "this", "isn't" | "isnt", "a", "creature"]) {
        return Some(vec![StaticAbilityAst::Static(StaticAbility::remove_card_types(
            ObjectFilter::source(), vec![crate::types::CardType::Creature], Some(PredicateAst::AttachedToSourceMatches(ObjectFilter::default())),
        ))]);
    }

    if matches!(words.as_slice(),
        ["this", "creature", "can", "deal", "excess", "combat", "damage", "to", "the", "controller", "of", "the", "planeswalker", "it's" | "its", "attacking"]
        | ["this", "creature", "can", "deal", "excess", "combat", "damage", "to", "the", "player", "or", "planeswalker", "it's" | "its", "attacking"])
    {
        let through_planeswalkers = words.contains(&"controller");
        return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(
            if through_planeswalkers {
                "This creature can deal excess combat damage to the controller of the planeswalker it's attacking."
            } else {
                "This creature can deal excess combat damage to the player or planeswalker it's attacking."
            },
        ))]);
    }
    // A reciprocal blocking restriction is two ordinary relations. The
    // referenced ability belongs to the other creature's filter; it is not
    // granted to this source by parsing its definition.
    if let Some(can) = tokens.iter().position(|token| token.is_word("can"))
        && source(&crate::lexer::token_word_refs(&tokens[..can]))
        && crate::lexer::token_word_refs(&tokens[can..]).starts_with(&[
            "can", "block", "or", "be", "blocked", "by", "only", "creatures", "with",
        ])
    {
        let mut excluded = tokens[can + 7..].to_vec();
        excluded.get_mut(1)?.replace_word("without");
        let excluded = parse_object_filter_lexed(&excluded, false).ok()?;
        let restrictions = vec![
            crate::effect::Restriction::BlockSpecificAttacker {
                blockers: ObjectFilter::source(), attacker: excluded.clone(),
            },
            crate::effect::Restriction::BlockSpecificAttacker {
                blockers: excluded, attacker: ObjectFilter::source(),
            },
        ];
        return Some(vec![StaticAbilityAst::Static(StaticAbility::restrictions(
            restrictions, crate::lexer::render_token_slice(tokens),
        ))]);
    }
    if words.starts_with(&["during", "your", "turn"])
        && let Some(is) = words.iter().position(|word| *word == "is")
        && is > 3 && source(&words[3..is])
    {
        let mut descriptors = &words[is + 1..];
        if descriptors.first() == Some(&"also") { descriptors = &descriptors[1..]; }
        if matches!(descriptors.first(), Some(&"a" | &"an")) { descriptors = &descriptors[1..]; }
        if descriptors.ends_with(&["in", "addition", "to", "its", "other", "types"]) {
            descriptors = &descriptors[..descriptors.len() - 6];
        }
        if !descriptors.is_empty() && let Some(mut types) = descriptors.iter().map(|word| crate::util::parse_card_type(word)).collect::<Option<Vec<_>>>() {
            // CR 702.161a's printed shorthand omits "artifact". Expand the
            // recognized complete reminder to its official definition; this
            // still lowers to conditional type addition, not LivingMetal ID.
            if words[3..is] == ["this", "vehicle"] && descriptors == ["creature"] {
                types.insert(0, crate::types::CardType::Artifact);
            }
            return Some(vec![StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::add_card_types(ObjectFilter::source(), types))),
                condition: crate::cards::builders::PredicateAst::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn),
            }]);
        }
    }
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    // These definitions describe payment substitutions, not cost reductions:
    // the shared payment planner preserves colored pips and spends each object
    // once, after mana abilities have been activated.
    for (introduction, body) in [
        ("", "Each card you exile from your graveyard while casting this spell pays for {1}."),
        ("Your creatures can help cast this spell.", "Each creature you tap while casting this spell pays for {1} or one mana of that creature's color."),
        ("Your artifacts can help cast this spell.", "Each artifact you tap after you're done activating mana abilities pays for {1}."),
    ] {
        let normalized = |text: &str| text.to_ascii_lowercase().replace('’', "'").chars()
            .filter(|c| !c.is_whitespace()).collect::<String>();
        let rendered = crate::lexer::render_token_slice(tokens);
        let expected = format!("{introduction} {body}");
        if normalized(&rendered).trim_end_matches('.') == normalized(&expected).trim_end_matches('.')
            || normalized(&rendered).trim_end_matches('.') == normalized(body).trim_end_matches('.')
        {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(body))]);
        }
    }

    if let Some(first) = sentences.first() {
        let words = crate::lexer::token_word_refs(first);
        if words.starts_with(&["players", "dealt", "combat", "damage", "by"])
            && let Some(also) = words.iter().position(|word| *word == "also")
            && also > 5 && source(&words[5..also])
            && words.get(also + 1) == Some(&"get")
            && words.ends_with(&["poison", "counters"])
            && words.len() == also + 5
            && let Some(amount) = crate::util::parse_number_word_u32(words[also + 2])
            && (sentences.len() == 1 || (sentences.len() == 2
                && crate::lexer::token_word_refs(sentences[1]) == ["a", "player", "with", "ten", "or", "more", "poison", "counters", "loses", "the", "game"]))
        {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(
                format!("Players dealt combat damage by this creature also get {amount} poison counters."),
            ))]);
        }
    }
    if let Some(first) = sentences.first() {
        let words = crate::lexer::token_word_refs(first);
        if let Some(subject) = words.strip_suffix(&["phases", "in", "or", "out", "before", "you", "untap", "during", "each", "of", "your", "untap", "steps"])
            && (source(subject) || subject == ["this"])
            && (sentences.len() == 1 || (sentences.len() == 2
                && matches!(crate::lexer::token_word_refs(sentences[1]).as_slice(),
                    ["while", "it's" | "its", "phased", "out", "it's" | "its", "treated", "as", "though", "it", "doesn't" | "doesnt", "exist"])))
        {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(
                "This phases in or out before you untap during each of your untap steps.",
            ))]);
        }
    }
    if sentences.len() == 2 {
        let first = crate::util::trim_edge_punctuation_tokens(sentences[0]);
        let first_words = crate::lexer::token_word_refs(first);
        let second = crate::lexer::token_word_refs(sentences[1]);
        let end_words = if first_words.ends_with(&["as", "you", "cast", "this"]) { Some(4) }
            else if first_words.ends_with(&["as", "you", "cast", "this", "card"]) { Some(5) }
            else { None };
        if first_words.first() == Some(&"target")
            && let Some(end_words) = end_words
            && matches!(second.as_slice(), ["this", "card", "enters", "attached", "to", "that", "creature" | "land" | "permanent" | "artifact" | "enchantment"])
        {
            let referent = second[6];
            if referent != "permanent" && !first_words[1..first_words.len() - end_words].contains(&referent) {
                return None;
            }
            let view = crate::lexer::TokenWordView::new(first);
            let end = *view.token_start_indices().get(first_words.len() - end_words)?;
            let filter = parse_object_filter_lexed(&first[1..end], false).ok()?;
            // CR 702.5's casting/entry shorthand also defines the ongoing
            // attachment restriction. Preserve the entire authored filter.
            return Some(vec![StaticAbilityAst::AttachmentRestriction {
                filter: filter.into(), display: crate::lexer::render_token_slice(tokens),
            }]);
        }
    }
    if let Some(first) = sentences.first() {
        let first_words = crate::lexer::token_word_refs(first);
        let immunity = match first_words.as_slice() {
            ["damage", "and", "effects", "that", "say", "destroy", "don't" | "dont", "destroy", subject @ ..] => source(subject),
            [subject @ .., "cannot", "be", "destroyed"] => source(subject),
            [subject @ .., "can't" | "cant", "be", "destroyed"] => source(subject),
            _ => false,
        };
        // This explanatory rider describes the ordinary zero-toughness SBA;
        // it must not add a destruction replacement or a new trigger.
        let ordinary_zero_toughness = sentences.len() == 2 && {
            let rendered = crate::lexer::render_token_slice(crate::util::trim_edge_punctuation_tokens(&sentences[1])).to_ascii_lowercase();
            rendered == "if its toughness is 0 or less, it's still put into its owner's graveyard"
        };
        if immunity && (sentences.len() == 1 || ordinary_zero_toughness) {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker("This permanent cannot be destroyed."))]);
        }
    }
    if let [subject @ .., "can", "block", "creatures", "with", "flying"] = words.as_slice()
        && source(subject)
        && words.len() == tokens.len()
    {
        return Some(vec![StaticAbilityAst::Static(StaticAbility::new(
            crate::static_abilities::StaticAbilityId::CanBlockFlying,
        ))]);
    }
    // Counter-producing damage is an operation shared by definitions, not
    // a grant of either Wither or Infect. Parse the counter kind explicitly.
    if let Some(deals) = tokens.iter().position(|token| token.is_word("deals"))
        && (source(&crate::lexer::token_word_refs(&tokens[..deals])) || crate::lexer::token_word_refs(&tokens[..deals]) == ["this"])
    {
        let tail = crate::lexer::render_token_slice(&tokens[deals..]);
        let normalized = tail.to_ascii_lowercase();
        let base = "deals damage to creatures in the form of -1/-1 counters";
        if normalized == base || normalized == format!("{base} and to players in the form of poison counters") {
            return Some(vec![StaticAbilityAst::Static(StaticAbility::keyword_marker(&format!("This {normalized}.")))]);
        }
    }
    if words.len() == tokens.len() {
        if let [
            "any",
            "amount",
            "of",
            "damage",
            subject @ ..,
            "deals",
            "to",
            "a",
            "creature",
            "is",
            "enough",
            "to",
            "destroy",
            "it",
        ] = words.as_slice()
            && (source(subject) || subject == &["this"])
        {
            return Some(vec![StaticAbilityAst::Static(
                StaticAbility::keyword_marker(
                    "Any amount of damage this deals to a creature is enough to destroy it.",
                ),
            )]);
        }
        if let [
            "damage",
            "dealt",
            "by",
            subject @ ..,
            "also",
            "causes",
            "you",
            "to",
            "gain",
            "that",
            "much",
            "life",
        ] = words.as_slice()
            && source(subject)
        {
            return Some(vec![StaticAbilityAst::Static(
                StaticAbility::keyword_marker(
                    "Damage dealt by this creature also causes you to gain that much life.",
                ),
            )]);
        }
        if let [
            subject @ ..,
            "deals",
            "combat",
            "damage",
            "before",
            "creatures",
            "without",
            "first",
            "strike",
        ] = words.as_slice()
            && source(subject)
        {
            return Some(vec![StaticAbilityAst::Static(
                StaticAbility::keyword_marker(
                    "This creature deals combat damage before creatures without first strike.",
                ),
            )]);
        }
        if let [
            subject @ ..,
            "deals",
            "both",
            "first-strike",
            "and",
            "regular",
            "combat",
            "damage",
        ] = words.as_slice()
            && source(subject)
        {
            return Some(vec![StaticAbilityAst::Static(
                StaticAbility::keyword_marker(
                    "This creature deals both first-strike and regular combat damage.",
                ),
            )]);
        }
    }
    if words.len() == tokens.len() {
        if matches!(
            words.as_slice(),
            [
                "you",
                "may",
                "cast",
                "this",
                "spell" | "card",
                "any",
                "time",
                "you",
                "could",
                "cast",
                "an",
                "instant"
            ]
        ) {
            return Some(vec![StaticAbilityAst::Static(
                StaticAbility::keyword_marker(
                    "You may cast this spell any time you could cast an instant.",
                ),
            )]);
        }
    }
    if let [
        "attacking",
        "doesn't" | "doesnt",
        "cause",
        subject @ ..,
        "to",
        "tap",
    ] = words.as_slice()
        && source(subject)
        && words.len() == tokens.len()
    {
        return Some(vec![StaticAbilityAst::Static(
            StaticAbility::keyword_marker("Attacking doesn't cause this creature to tap."),
        )]);
    }

    let can = tokens.iter().position(|token| token.is_word("can"))?;
    if !source(&crate::lexer::token_word_refs(&tokens[..can])) {
        return None;
    }
    let rest = &tokens[can + 1..];
    if !rest.first().is_some_and(|token| token.is_word("attack")) {
        return None;
    }
    let rest = &rest[1..];
    let tap = rest.len() >= 2
        && rest[0].is_word("and")
        && rest[1].kind == TokenKind::ManaGroup
        && rest[1].slice.eq_ignore_ascii_case("{t}");
    let rest = if tap { &rest[2..] } else { rest };
    let tail = crate::lexer::token_word_refs(rest);
    if tail.len() != rest.len()
        || !matches!(
            tail.as_slice(),
            [
                "as",
                "soon",
                "as",
                "it" | "he" | "she",
                "comes",
                "under",
                "your",
                "control"
            ] | [
                "no",
                "matter",
                "when",
                "it" | "he" | "she",
                "came",
                "under",
                "your",
                "control"
            ] | ["as", "though", "it", "had", "haste"]
        )
    {
        return None;
    }
    let mut abilities = vec![StaticAbilityAst::Static(
        StaticAbility::can_attack_as_though_haste(),
    )];
    if tap {
        abilities.push(StaticAbilityAst::Static(
            StaticAbility::activate_abilities_as_though_haste(
                ObjectFilter::source(),
                "You may activate abilities of this creature as though it had haste.",
            ),
        ));
    }
    Some(abilities)
}

/// An entry replacement with a player choice, independent of any keyword name.
fn parse_entry_counter_or_ability(tokens: &[OwnedLexToken]) -> Option<Vec<StaticAbilityAst>> {
    use crate::cards::builders::{EffectAst, ChooseOneModeAst, ObjectChoiceEffectAst, TargetAst, GrantedAbilityAst};
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let enters = words.iter().position(|w| *w == "enters")?;
    if !source(&words[..enters]) || !matches!(words.get(enters + 1..enters + 6)?, ["with", "your", "choice", "of", "a" | "an"]) { return None; }
    let start = enters + 6;
    let start = if words.get(start) == Some(&"additional") { start + 1 } else { start };
    let counter = words.iter().position(|w| *w == "counter")?;
    if counter <= start || words.get(counter + 1) != Some(&"or") { return None; }
    let counter_type = crate::grammar::filters::parse_counter_type_words(&words[start..=counter])?;
    let ability_tokens = &tokens[*view.token_start_indices().get(counter + 2)?..];
    let granted = if words.len() == counter + 3 {
        crate::activation_and_restrictions::parse_single_word_keyword_action(words[counter + 2])
            .map(|keyword| vec![GrantedAbilityAst::KeywordAction(Box::new(keyword))])
    } else { None }.or_else(|| {
        super::parse_static_ability_ast_line_lexed(ability_tokens).ok().flatten()
            .map(|abilities| abilities.into_iter().map(|ability| GrantedAbilityAst::StaticAbility(Box::new(ability))).collect())
    })?;
    let modes = vec![
        ChooseOneModeAst { description: format!("Put a {} counter on this permanent", counter_type.description()), effects: vec![EffectAst::subject_verb_put_counters(counter_type, crate::effect::Value::Fixed(1), TargetAst::Source(None), None, false)] },
        ChooseOneModeAst { description: format!("Gain {}", crate::lexer::render_token_slice(ability_tokens)), effects: vec![EffectAst::subject_verb_grant_abilities_to_target(TargetAst::Source(None), granted, crate::effect::Until::Forever)] },
    ];
    Some(vec![StaticAbilityAst::Static(StaticAbility::as_enters_effect_program(
        vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { chooser: PlayerFilter::You, modes })].into(),
        crate::lexer::render_token_slice(&tokens[..view.token_start_indices()[enters]]), false, false, None,
    ))])
}
