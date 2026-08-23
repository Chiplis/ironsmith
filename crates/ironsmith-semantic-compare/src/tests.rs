use super::{
    EmbeddingConfig, compare_card_semantics_scored, compare_semantics_scored,
    compiled_comparison_tokens, conjunction_flips_between,
    normalize_card_self_references_for_compare, normalize_repeated_you_after_draw,
    normalize_trigger_subject_for_compare, reminder_clauses, semantic_clauses,
    semantic_clauses_for_compare, strip_reminder_text_for_comparison,
};

fn strict_embedding() -> Option<EmbeddingConfig> {
    Some(EmbeddingConfig {
        dims: 384,
        mismatch_threshold: 0.99,
    })
}

#[test]
fn exact_normalized_clause_match_beats_special_mismatch_penalties() {
    let oracle = "{1}{W}: Target nonattacking, nonblocking creature gets +0/+2 until end of turn.";
    let compiled = vec![oracle.to_string()];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Unlikely Alliance", oracle, &compiled, strict_embedding());

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);
}

#[test]
fn targets_only_is_not_equivalent_to_has_a_matching_target() {
    for (card_name, oracle, compiled) in [
        (
            "Not of This World",
            "Counter target spell or ability that targets a permanent you control.",
            "Counter target spell that targets only a permanent you control or ability that targets only a permanent you control.",
        ),
        (
            "Diplomatic Escort",
            "{U}, {T}, Discard a card: Counter target spell or ability that targets a creature.",
            "{U}, {T}, Discard a card: Counter target spell that targets only a creature or ability that targets only a creature.",
        ),
        (
            "Siren Stormtamer",
            "{U}, Sacrifice this creature: Counter target spell or ability that targets you or a creature you control.",
            "{U}, Sacrifice this creature: Counter target spell that targets only a creature you control or ability that targets only a creature you control.",
        ),
    ] {
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_card_semantics_scored(
                card_name,
                oracle,
                &[compiled.to_string()],
                strict_embedding(),
            );

        assert!(
            similarity < 0.99,
            "{card_name}: a behaviorally narrower target set must not clear the strict numeric score floor; similarity={similarity} mismatch={mismatch}"
        );
        assert!(
            mismatch,
            "{card_name}: a behaviorally narrower target set must be flagged as a semantic mismatch"
        );
    }
}

#[test]
fn distributed_ordinary_counter_target_scope_remains_equivalent() {
    let oracle = "Counter target spell or ability that targets a permanent you control.";
    let compiled = vec![
        "Counter target spell that targets a permanent you control or ability that targets a permanent you control."
            .to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Not of This World", oracle, &compiled, strict_embedding());

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);
}

#[test]
fn named_multi_zone_search_tense_and_sequence_surface_compare_equally() {
    let oracle = "When this creature enters, you may search your library and/or graveyard for a card named Huatli, Dinosaur Knight, reveal it, then put it into your hand. If you searched your library this way, shuffle.";
    let compiled = vec![
        "When this creature enters, you may search your library and/or graveyard for a card named huatli dinosaur knight, reveal it, and put it into your hand. If you search your library this way, shuffle.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Sun-Blessed Mount", oracle, &compiled, strict_embedding());

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn revealed_set_shared_card_type_surface_compares_to_prior_result_rendering() {
    let oracle = "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card.";
    let compiled = vec![
        "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) = compare_card_semantics_scored(
        "Gandalf, Westward Voyager",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn d20_numeric_ranges_ignore_typographic_dash_choice() {
    let oracle = "Roll a d20.\n1-9 | Each player draws a card.\n10-19 | You draw a card.";
    let compiled = vec![
        "Roll a d20.".to_string(),
        "1—9 | Each player draws a card.".to_string(),
        "10—19 | draw a card.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) = compare_card_semantics_scored(
        "Mathise, Surge Channeler",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn repeated_you_subject_in_draw_life_clause_compares_to_implied_subject() {
    for (card_name, oracle, compiled) in [
        (
            "Phyrexian Gargantua",
            "When this creature enters, you draw two cards and you lose 2 life.",
            "When this creature enters, draw two cards and lose 2 life.",
        ),
        (
            "Infernal Idol",
            "{1}{B}{B}, {T}, Sacrifice this artifact: You draw two cards and you lose 2 life.",
            "{1}{B}{B}, {T}, Sacrifice this artifact: Draw two cards and lose 2 life.",
        ),
        (
            "Cut of the Profits",
            "You draw X cards and you lose X life.",
            "Draw X cards and you lose X life.",
        ),
    ] {
        let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
            compare_card_semantics_scored(
                card_name,
                oracle,
                &[compiled.to_string()],
                strict_embedding(),
            );

        assert_eq!(delta, 0, "{card_name}");
        assert!(
            similarity >= 0.99 && !mismatch,
            "{card_name}: similarity={similarity} mismatch={mismatch}"
        );
    }
}

#[test]
fn draw_subject_normalization_does_not_merge_a_serial_effect_list() {
    let oracle = "At the beginning of your upkeep, sacrifice a creature.\nWhen you sacrifice this creature, each opponent discards a card, you draw a card, and you gain 2 life.";
    let compiled = vec![
        "At the beginning of your upkeep, sacrifice a creature.".to_string(),
        "When you sacrifice this creature, draw a card, then gain 2 life.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Daemogoth Woe-Eater", oracle, &compiled, strict_embedding());

    assert!(mismatch, "the missing opponent discard must remain visible");
    assert!(
        similarity >= 0.97,
        "unexpected score regression: {similarity}"
    );
}

#[test]
fn draw_subject_normalization_handles_direct_and_triggered_draw_heads() {
    assert_eq!(
        normalize_repeated_you_after_draw(
            "Draw two cards, then discard a card and you lose 2 life."
        ),
        "Draw two cards, then discard a card and lose 2 life."
    );
    assert_eq!(
        normalize_repeated_you_after_draw(
            "Whenever you draw a card, put a +1/+1 counter on this and you gain 1 life."
        ),
        "Whenever you draw a card, put a +1/+1 counter on this and gain 1 life."
    );
    assert_eq!(
        normalize_repeated_you_after_draw(
            "When you sacrifice this creature, each opponent discards a card, you draw a card, and you gain 2 life."
        ),
        "When you sacrifice this creature, each opponent discards a card, you draw a card, and you gain 2 life."
    );
}

#[test]
fn carried_player_coreferences_compare_without_hiding_missing_effects() {
    for (card_name, oracle, compiled, prior_score) in [
        (
            "Cloak and Dagger, Entwined",
            "Deathtouch, lifelink\nWhen Cloak and Dagger enter, choose target opponent and up to one target creature they control. They reveal their hand. You may exile a nonland card from their hand or the chosen creature until Cloak and Dagger leave the battlefield.",
            "Deathtouch, lifelink\nWhen Cloak and Dagger enters, target opponent reveals their hand. You may exile a nonland creature card from that player's hand.",
            0.5455,
        ),
        (
            "Covetous Urge",
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            "Target opponent reveals their hand, choose a nonland permanent that player owns in a graveyard or in a hand, exile it, then you may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0.8392,
        ),
        (
            "Psychic Intrusion",
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            "Target opponent reveals their hand, choose a nonland permanent that player owns in a graveyard or in a hand, exile it, then you may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0.8392,
        ),
    ] {
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_card_semantics_scored(
                card_name,
                oracle,
                &[compiled.to_string()],
                strict_embedding(),
            );
        assert!(
            similarity + f32::EPSILON >= prior_score,
            "{card_name}: score regressed to {similarity}"
        );
        assert!(
            mismatch,
            "{card_name}: genuine missing semantics were hidden"
        );
    }

    let oracle = "Target player reveals their hand. You gain life equal to the number of cards in that player's hand.";
    let compiled = vec![
        "Target player reveals their hand, then gain life equal to the number of cards in their hand."
            .to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Search Warrant", oracle, &compiled, strict_embedding());
    assert_eq!(delta, 0);
    assert!(similarity >= 0.99 && !mismatch, "similarity={similarity}");
}

#[test]
fn antecedent_normalization_preserves_identical_text_score_baselines() {
    let cases: [(&str, &str, &[&str], f32); 6] = [
        (
            "Breathkeeper Seraph",
            "Flying, soulbond\nAs long as Breathkeeper Seraph is paired with another creature, each of those creatures has \"When this creature dies, you may return it to the battlefield under its owner's control at the beginning of your next upkeep.\"",
            &[
                "Flying",
                "Soulbond",
                "As long as this creature is paired with another creature, each of those creatures has \"When this creature dies, may At the beginning of your next upkeep, return it to the battlefield under its owner's control.\"",
            ],
            // Re-pinned after the as-long-as/if connective canonicalization.
            0.9022,
        ),
        (
            "Cabal Interrogator",
            "{X}{B}, {T}: Target player reveals X cards from their hand and you choose one of them. That player discards that card. Activate only as a sorcery.",
            &[
                "{X}{B}, {T}: Reveal it, choose a permanent revealed this way on the battlefield, in a hand, in a graveyard, in a library, or in exile, then discard that card. Activate only as a sorcery.",
            ],
            0.9222,
        ),
        (
            "Conqueror's Flail",
            "Equipped creature gets +1/+1 for each color among permanents you control.\nAs long as this Equipment is attached to a creature, your opponents can't cast spells during your turn.\nEquip {2}",
            &[
                "Equipped creature gets +1/+1 for each permanent you control.",
                "Your opponents can't cast spells as long as this creature is equipped and during your turn.",
                "Equip {2}",
            ],
            // Re-pinned after the as-long-as/if connective canonicalization.
            0.9691,
        ),
        (
            "Raksha Golden Cub",
            "Vigilance\nAs long as Raksha Golden Cub is equipped, Cat creatures you control get +2/+2 and have double strike.",
            &[
                "Vigilance",
                "Cat creatures you control get +2/+2 as long as this creature is equipped.",
                "As long as Raksha Golden Cub is equipped, Cat creatures you control have double strike.",
            ],
            // Re-pinned after the as-long-as/if connective canonicalization.
            0.9776,
        ),
        (
            "Rhystic Syphon",
            "Unless target player pays {3}, that player loses 5 life and you gain 5 life.",
            &["That player loses 5 life. You gain 5 life unless target player pays {3}."],
            0.8622,
        ),
        (
            "Thieving Sprite",
            "Flying\nWhen this creature enters, target player reveals X cards from their hand, where X is the number of Faeries you control. You choose one of those cards. That player discards that card.",
            &[
                "Flying",
                "When this creature enters, reveal it. You choose a card. You discard that card.",
            ],
            0.8225,
        ),
    ];

    for (card_name, oracle, compiled, baseline) in cases {
        let compiled = compiled
            .iter()
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>();
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_card_semantics_scored(card_name, oracle, &compiled, strict_embedding());
        assert!(
            similarity + 0.00005 >= baseline,
            "{card_name}: {similarity} fell below {baseline}"
        );
        assert!(mismatch, "{card_name}: known missing semantics were hidden");
    }
}

#[test]
fn unpaired_self_copula_surface_is_not_rewritten_during_clause_normalization() {
    for oracle_line in [
        "Enchanted creature gets +3/+3 as long as it's a Zombie. Otherwise, it gets -3/-3.",
        "Enchanted creature gets +2/+2 as long as it's a Vampire. Otherwise, it gets -2/-2.",
        "Enchanted creature gets +2/+2 as long as it's an enchantment. Otherwise, it gets -2/-2.",
        "Enchanted creature gets +2/+1 as long as it's black. Otherwise, it gets -1/-2.",
        "Enchanted creature gets +0/+2 as long as it's a Pirate. Otherwise, it gets -2/-0.",
        "Enchanted creature gets +1/+2 as long as it's white. Otherwise, it gets -2/-1.",
        "Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets -2/-1.",
        "As long as it's your turn, this creature has first strike.",
        "As long as it's your turn and you control an Army, this creature is an artifact creature.",
    ] {
        let clauses = semantic_clauses_for_compare(oracle_line);
        assert!(
            clauses.iter().any(|clause| clause.contains("it's")),
            "unpaired contraction was rewritten and would perturb unchanged-card scores: {clauses:?}"
        );
        assert!(
            clauses.iter().all(|clause| !clause.contains("this is")),
            "unpaired contraction was expanded during independent normalization: {clauses:?}"
        );
    }
}

#[test]
fn repeated_filtered_set_and_self_source_coreferences_compare_structurally() {
    let oracle = "Creatures you control get +1/+1 until end of turn. If you have the city's blessing, those creatures get +2/+2 until end of turn instead.";
    let compiled = vec!["Each creature you control gets +1/+1 until end of turn. If you have the city's blessing, each creature you control gets +2/+2 until end of turn instead.".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Pride of Conquerors", oracle, &compiled, strict_embedding());
    assert_eq!(delta, 0);
    assert!(similarity >= 0.99 && !mismatch, "similarity={similarity}");

    let under_filtered = vec!["Creatures you control get +1/+1 until end of turn. If you have the city's blessing, creatures get +2/+2 until end of turn instead.".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) = compare_card_semantics_scored(
        "Pride of Conquerors",
        oracle,
        &under_filtered,
        strict_embedding(),
    );
    assert!(
        mismatch && similarity < 1.0,
        "missing controller was hidden"
    );

    let oracle = "As long as this creature is attacking, it gets +X/+0.";
    let compiled = vec!["This creature gets +X/+0 as long as it's attacking.".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Elturel Survivors", oracle, &compiled, strict_embedding());
    assert_eq!(delta, 0);
    assert!(similarity >= 0.99 && !mismatch, "similarity={similarity}");
}

#[test]
fn coreference_normalization_does_not_mask_pact_weapon_regressions() {
    let oracle = "As long as this Equipment is attached to a creature, you don't lose the game for having 0 or less life.\nWhenever equipped creature attacks, draw a card and reveal it. The creature gets +X/+X until end of turn and you lose X life, where X is that card's mana value.\nEquip—Discard a card.";
    let compiled = vec![
        "You don't lose the game for having 0 or less life as long as this creature is equipped."
            .to_string(),
        "Whenever equipped creature attacks, draw a card, reveal it, creatures get +X/+X until end of turn, where X is its mana value, then lose X life, where X is that creature's mana value."
            .to_string(),
        "Equip Discard a card".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Pact Weapon", oracle, &compiled, strict_embedding());
    assert!(mismatch, "Pact Weapon's overbroad pump was hidden");
    // Floor re-pinned after the as-long-as/if and until-end-of-turn/this-turn
    // canonicalizations shifted clause pairing; the guard is the mismatch
    // flag above.
    assert!(similarity >= 0.9229, "score regressed to {similarity}");
}

#[test]
fn keyword_only_lines_and_lists_compare_equally() {
    let oracle = "Flying\nDeathtouch\nLifelink";
    let compiled = vec!["Flying, deathtouch, lifelink".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);

    let oracle = "Flying, first strike, lifelink";
    let compiled = vec![
        "Flying".to_string(),
        "First strike".to_string(),
        "Lifelink".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);
}

#[test]
fn veiled_apparition_animation_granted_trigger_wording_compares() {
    let oracle = "When an opponent casts a spell, if this permanent is an enchantment, it becomes a 3/3 Illusion creature with flying and \"At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}.\"";
    let compiled = vec!["Whenever an opponent casts a spell, if this enchantment is an enchantment, this enchantment becomes a 3/3 illusion creature with flying and at the beginning of your upkeep, sacrifice this creature unless you pay {1}{u}.".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Veiled Apparition", oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected no mismatch for Veiled Apparition animation grant wording"
    );
}

#[test]
fn fixed_pt_animation_shorthand_compares_to_explicit_base_pt_land_surface() {
    let oracle = "Target land becomes a 3/3 Elemental creature with flying until end of turn. It's still a land.";
    let compiled = vec![
            "Target land becomes an elemental creature with base power and toughness 3/3 and flying until end of turn. It's still a land.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Hydroform", oracle, &compiled, strict_embedding());

    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn fixed_pt_animation_shorthand_compares_leading_until_still_land_surface() {
    let oracle = "Until end of turn, target land becomes a 3/3 creature that's still a land.";
    let compiled = vec![
            "Target land becomes a creature with base power and toughness 3/3 until end of turn. It's still a land.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Animate Land", oracle, &compiled, strict_embedding());

    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn fixed_pt_animation_shorthand_compares_to_explicit_base_pt_artifact_surface() {
    let oracle = "{1}: This land becomes a 1/1 Phyrexian Blinkmoth artifact creature with flying and infect until end of turn. It's still a land.";
    let compiled = vec![
            "{1}: This land becomes a phyrexian blinkmoth artifact creature with base power and toughness 1/1 and flying and infect until end of turn. It's still a land".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Inkmoth Nexus", oracle, &compiled, strict_embedding());

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn animated_land_attack_trigger_and_each_become_surfaces_compare() {
    let oracle = "Heroic — Whenever you cast a spell that targets Anthousa, up to three target lands you control each become 2/2 Warrior creatures until end of turn. They're still lands.\nWhenever this land attacks, other creatures you control get +1/+1 until end of turn.";
    let compiled = vec![
            "Heroic — Whenever you cast a spell that targets this creature, up to three target lands you control become warrior creatures with base power and toughness 2/2 until end of turn. They're still lands.".to_string(),
            "Whenever this creature attacks, other creatures you control get +1/+1 until end of turn.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) = compare_card_semantics_scored(
        "Anthousa, Setessan Hero",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn anthousa_animation_surface_stays_above_latest_sync_score() {
    let oracle = "Heroic — Whenever you cast a spell that targets Anthousa, up to three target lands you control each become 2/2 Warrior creatures until end of turn. They're still lands.";
    let compiled = vec![
            "Heroic — Whenever you cast a spell that targets this creature, up to three target lands you control become warrior creatures with base power and toughness 2/2 until end of turn. They're still lands.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) = compare_card_semantics_scored(
        "Anthousa, Setessan Hero",
        oracle,
        &compiled,
        strict_embedding(),
    );

    if similarity < 0.9981 || mismatch {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
    }
    assert!(
        similarity >= 0.9981 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn restless_cottage_attack_followup_surface_stays_above_latest_sync_score() {
    let oracle = "This land enters tapped.\n{T}: Add {B} or {G}.\n{2}{B}{G}: This land becomes a 4/4 black and green Horror creature until end of turn. It's still a land.\nWhenever this land attacks, create a Food token and exile up to one target card from a graveyard.";
    let compiled = vec![
            "This land enters tapped.".to_string(),
            "{T}: Add {B} or {G}.".to_string(),
            "{2}{B}{G}: This land becomes a black and green horror creature with base power and toughness 4/4 until end of turn. It's still a land.".to_string(),
            "Whenever this creature attacks, create a Food token, then exile up to one target card in a graveyard.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_card_semantics_scored("Restless Cottage", oracle, &compiled, strict_embedding());

    if similarity < 0.965 {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
    }
    // Re-pinned after literal number/pt comparison landed: the trigger
    // clause split ("create a Food token" / "exile up to one target card")
    // is real render drift the comparator no longer forgives.
    assert!(
        similarity >= 0.965,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn conjunction_flip_detected_between_filter_clauses() {
    let flips = conjunction_flips_between(
        "Destroy target artifact or enchantment.",
        "Destroy target artifact and enchantment.",
    );
    assert_eq!(flips.len(), 1, "expected exactly one flip: {flips:?}");
    assert_eq!(flips[0].oracle_conjunction, "or");
    assert_eq!(flips[0].compiled_conjunction, "and");
    assert_eq!(flips[0].left, "artifact");
    assert_eq!(flips[0].right, "enchantment");
}

#[test]
fn conjunction_flip_ignores_mass_quantified_clauses() {
    // Oracle idiom: "destroy all artifacts and enchantments" is the mass
    // rendering of the filter "artifact or enchantment" — not a flip.
    let flips = conjunction_flips_between(
        "Destroy all artifacts and enchantments.",
        "Destroy each artifact or enchantment.",
    );
    assert!(flips.is_empty(), "mass context must not flag: {flips:?}");
}

#[test]
fn conjunction_flip_ignores_and_or_surfaces_and_matching_conjunctions() {
    assert!(
        conjunction_flips_between(
            "Exile up to two target artifact and/or enchantment cards.",
            "Exile up to two target artifact or enchantment cards.",
        )
        .is_empty(),
        "and/or surfaces are excluded"
    );
    assert!(
        conjunction_flips_between(
            "Sacrifice an artifact or creature.",
            "Sacrifice an artifact or creature.",
        )
        .is_empty(),
        "matching conjunctions must not flag"
    );
}

#[test]
fn self_sacrifice_name_and_card_type_references_compare_equally() {
    let oracle = "At the beginning of your end step, sacrifice Sothera.";
    let compiled =
        vec!["At the beginning of your end step, sacrifice this enchantment.".to_string()];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) = compare_card_semantics_scored(
        "Sothera, the Supervoid",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);
}

#[test]
fn sothera_surface_compare_clears_strict_gate() {
    let oracle = "Whenever a creature you control dies, each opponent chooses a creature they control and exiles it.\nAt the beginning of your end step, if a player controls no creatures, sacrifice Sothera, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it.";
    let compiled = vec![
            "Whenever a creature you control dies, each opponent chooses a creature they control and exiles it.".to_string(),
            "At the beginning of your end step, if a player controls no creatures, sacrifice this enchantment, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) = compare_card_semantics_scored(
        "Sothera, the Supervoid",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} delta={delta} mismatch={mismatch}"
    );
}

#[test]
fn intrinsic_land_mana_ability_does_not_penalize_type_line_oracle() {
    let oracle = "This land enters tapped.";
    let compiled = vec![
        "{T}: Add {G} or {W}.".to_string(),
        "This land enters tapped.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_card_semantics_scored("Arctic Treeline", oracle, &compiled, strict_embedding());

    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(!mismatch);
}

#[test]
fn strip_reminder_text_removes_parenthetical_mana_lines() {
    let text = "({T}: Add {W} or {B}.)\nThis land enters tapped.";
    assert_eq!(
        strip_reminder_text_for_comparison(text),
        "This land enters tapped."
    );
}

#[test]
fn strip_reminder_text_removes_standard_token_reminder_quotes() {
    let text = "Create a Treasure token. It has \"{T}, Sacrifice this artifact: Add one mana of any color.\"";
    assert_eq!(
        strip_reminder_text_for_comparison(text),
        "Create a Treasure token."
    );
}

#[test]
fn strip_reminder_text_preserves_semantic_token_abilities() {
    let text = "Create a Snake token with \"Whenever this creature deals damage to a player, that player gets a poison counter.\"";
    assert_eq!(strip_reminder_text_for_comparison(text), text);
}

#[test]
fn compare_semantics_normalizes_squad_and_junk_token_scaffolding() {
    let oracle = "Squad—{1}, Discard a card.\nWhen this creature dies, create a Junk token.";
    let compiled = vec![
            "Squad—{1}, Discard a card".to_string(),
            "When this creature dies, create a Junk token with \"{T}, Sacrifice this token, exile the top card of your library. You may play that card this turn. Activate only as a sorcery\"".to_string(),
            "When this creature enters, create how many times optional cost 'Squad' was paid tokens that are copies of this creature.".to_string(),
        ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) = compare_card_semantics_scored(
        "Thrill-Kill Disciple",
        oracle,
        &compiled,
        strict_embedding(),
    );

    assert_eq!(delta, 0);
    assert!(
        similarity >= 0.99 && !mismatch,
        "similarity={similarity} mismatch={mismatch}"
    );
}

#[test]
fn reminder_clauses_split_activation_restriction_text() {
    let clauses = reminder_clauses(
        "(Activate only if this creature attacked this turn and only once each turn.)",
    );
    assert_eq!(
        clauses,
        vec![
            "Activate only if this permanent attacked this turn".to_string(),
            "Activate only once each turn".to_string(),
        ]
    );
}

#[test]
fn compiled_comparison_tokens_drop_effect_scaffolding() {
    let tokens = compiled_comparison_tokens("If effect #0 happened, you draw a card.");
    assert!(!tokens.iter().any(|token| token == "if"));
    assert!(!tokens.iter().any(|token| token == "effect"));
    assert!(!tokens.iter().any(|token| token == "happen"));
    assert!(tokens.iter().any(|token| token == "draw"));
}

#[test]
fn compare_semantics_keeps_boast_costed_prefix() {
    let oracle = "Boast — {1}{R}: This creature deals 1 damage to any target. (Activate only if this creature attacked this turn and only once each turn.)";
    let compiled = vec![String::from(
        "Activated ability 1: Boast {1}{R}: This creature deals 1 damage to any target.",
    )];
    let (oracle_coverage, compiled_coverage, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(!mismatch);
    assert_eq!(oracle_coverage, 1.0);
    assert_eq!(compiled_coverage, 1.0);
    assert_eq!(similarity, 1.0);
}

#[test]
fn compare_semantics_splits_keyword_only_comma_bundles() {
    let oracle = "Trample; haste; shroud";
    let compiled = vec![String::from("Trample, haste, shroud")];
    let (oracle_coverage, compiled_coverage, similarity, delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());

    assert!(!mismatch);
    assert_eq!(oracle_coverage, 1.0);
    assert_eq!(compiled_coverage, 1.0);
    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
}

#[test]
fn compare_semantics_ignores_only_once_reminder_only() {
    let oracle = "Sacrifice a Food: Draw a card. (Activate only once each turn.)";
    let compiled = vec![String::from(
        "Activated ability 1: Sacrifice a Food: Draw a card.",
    )];
    let (oracle_coverage, compiled_coverage, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(!mismatch);
    assert_eq!(oracle_coverage, 1.0);
    assert_eq!(compiled_coverage, 1.0);
    assert_eq!(similarity, 1.0);
}

#[test]
fn compare_semantics_ignores_choose_scaffolding_clause() {
    let oracle = "When this land enters, sacrifice it.";
    let compiled = vec![String::from(
        "Triggered ability 1: When this land enters, you choose a permanent you control in the battlefield. you sacrifice a permanent.",
    )];
    let (oracle_cov, compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        oracle_cov >= 0.25,
        "expected reasonable oracle coverage for scaffolding drift, got {oracle_cov}"
    );
    assert!(
        compiled_cov >= 0.25,
        "expected reasonable compiled coverage for scaffolding drift, got {compiled_cov}"
    );
    assert!(
        similarity >= 0.25,
        "expected reasonable similarity for scaffolding drift, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for scaffolding-only drift");
}

#[test]
fn compare_semantics_ignores_tagging_scaffolding_clause() {
    let oracle = "Whenever a creature you control dies, put a +1/+1 counter on equipped creature.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever a creature you control dies, tag the object attached to this artifact as 'equipped'. Put a +1/+1 counter on the tagged object 'equipped'.",
    )];
    let (_oracle_cov, compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        compiled_cov >= 0.25,
        "expected reasonable compiled coverage for tagging scaffolding, got {compiled_cov}"
    );
    assert!(
        similarity >= 0.25,
        "expected reasonable similarity for tagging scaffolding, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for tagging scaffolding");
}

#[test]
fn compare_semantics_ignores_choose_background_scaffolding_clause() {
    let oracle = "Whenever Skanos Dragonheart attacks, it gets +X/+X until end of turn, where X is the greatest power among other Dragons you control and Dragon cards in your graveyard.\nChoose a Background (You can have a Background as a second commander.)";
    let compiled = vec![
        String::from(
            "Triggered ability 1: Whenever Skanos Dragonheart attacks, it gets +X/+X until end of turn, where X is the greatest power among other Dragons you control and Dragon cards in your graveyard.",
        ),
        String::from(
            "Spell effects: You choose exactly 1 a Background you control in the battlefield and tags it as '__it__'.",
        ),
    ];
    let (_oracle_cov, compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        compiled_cov >= 0.75,
        "expected choose-background scaffolding to be ignored, got {compiled_cov}"
    );
    assert!(
        similarity >= 0.75,
        "expected choose-background scaffolding to preserve strong similarity, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for choose-background scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_object_controller_wording() {
    let oracle = "Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.";
    let compiled = vec![String::from(
        "Spell effects: Deal 4 damage to target creature. Deal 2 damage to that object's controller.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.70,
        "expected controller wording normalization to keep similarity high, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for object/controller wording"
    );
}

#[test]
fn compare_semantics_normalizes_possessive_object_anaphors() {
    let oracle = "You gain life equal to that creature's toughness.";
    let compiled = vec![String::from("You gain life equal to its toughness.")];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected possessive object anaphors to compare identically, got {similarity}"
    );
    assert!(!mismatch);
}

#[test]
fn compare_semantics_normalizes_end_of_turn_play_permission_surfaces() {
    let oracle = "Exile the top card of your library. Until end of turn, you may play it.";
    let compiled = vec![String::from(
        "Exile the top card of your library, then you may play them this turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected end-of-turn permission surfaces to compare identically, got {similarity}"
    );
    assert!(!mismatch);
}

#[test]
fn compare_semantics_normalizes_remainder_sentence_split() {
    let oracle = "Reveal the top eight cards of your library. Put up to two artifact cards from among them onto the battlefield and the rest on the bottom of your library in a random order.";
    let compiled = vec![String::from(
        "Reveal the top eight cards of your library. Put up to two artifact cards from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected remainder sentence split to compare identically, got {similarity}"
    );
    assert!(!mismatch);
}

#[test]
fn compare_semantics_normalizes_not_named_and_exiled_return_phrasing() {
    let oracle = "When this enchantment enters, you may exile target nonland permanent not named Detention Sphere and all other permanents with the same name as that permanent. When this enchantment leaves the battlefield, return the exiled cards to the battlefield under their owner's control.";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When Detention Sphere enters, you may Exile target nonland permanent. Exile all other permanent with the same name as that object.",
        ),
        String::from(
            "Triggered ability 2: This enchantment leaves the battlefield: Return all card in exile to the battlefield.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, _mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.50,
        "expected normalization to preserve baseline similarity, got {similarity}"
    );
}

#[test]
fn compare_semantics_ignores_gendered_self_pronouns_for_source_library_move() {
    let oracle = "When this creature dies, put him on the bottom of his owner's library. If you do, return the exiled cards to their owners' hands.";
    let compiled = vec![String::from(
        "When this creature dies, put this creature on the bottom of its owner's library. If you do, return the exiled cards to their owners' hands.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.99,
        "expected gendered self-pronouns to compare as source references, got {similarity}"
    );
    assert!(!mismatch);
}

#[test]
fn compare_semantics_normalizes_target_opponent_exile_creature_and_graveyard_phrasing() {
    let oracle = "Target opponent exiles a creature they control and their graveyard.";
    let compiled = vec![String::from(
        "Spell effects: Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.95,
        "expected normalized phrasing to keep similarity high, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for opponent creature+graveyard exile phrasing"
    );
}

#[test]
fn compare_semantics_normalizes_target_player_exile_graveyard_phrasing() {
    let oracle = "Exile target player's graveyard.";
    let compiled = vec![String::from(
        "Spell effects: Exile all cards from target player's graveyard.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.99,
        "expected target-player graveyard exile normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for target-player graveyard exile phrasing"
    );
}

#[test]
fn compare_semantics_normalizes_each_creature_you_control_gets_anthem_wording() {
    let oracle = "Creatures you control get +2/+2.";
    let compiled = vec![String::from(
        "Static ability 1: Each creature you control gets +2/+2.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.99,
        "expected anthem singular/plural normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for each-creature vs creatures anthem wording"
    );
}

#[test]
fn compare_semantics_normalizes_target_player_gain_then_draw_sentence_split() {
    let oracle = "Target player gains 7 life and draws two cards.";
    let compiled = vec![String::from(
        "Spell effects: Target player gains 7 life. Target player draws two cards.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.99,
        "expected gain-then-draw sentence split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for gain-then-draw sentence split"
    );
}

#[test]
fn compare_semantics_normalizes_target_player_mill_draw_lose_sentence_split() {
    let oracle = "Target player mills two cards, draws two cards, and loses 2 life.";
    let compiled = vec![String::from(
        "Spell effects: Target player mills 2 cards. Target player draws two cards. target player loses 2 life.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    assert!(
        similarity >= 0.99,
        "expected mill/draw/lose sentence split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for mill/draw/lose sentence split"
    );
}

#[test]
fn compare_semantics_normalizes_control_no_permanents_other_than_this_self_reference() {
    let oracle = "At the beginning of your upkeep, if you control no permanents other than this enchantment and have no cards in hand, you win the game.";
    let compiled = vec![String::from(
        "Triggered ability 1: At the beginning of your upkeep, if you control no other permanents and you have no cards in hand, you win the game.",
    )];
    let (oracle_cov, compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, None);
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!(
            "oracle_cov={oracle_cov:.4} compiled_cov={compiled_cov:.4} similarity={similarity:.4} mismatch={mismatch}"
        );
    }
    if similarity < 0.99 || mismatch {
        let oracle_clauses = super::semantic_clauses(oracle);
        let compiled_clauses = super::semantic_clauses(&compiled[0]);
        let oracle_tokens = oracle_clauses
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        let compiled_tokens = compiled_clauses
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        eprintln!("oracle_clauses: {:?}", oracle_clauses);
        eprintln!("compiled_clauses: {:?}", compiled_clauses);
        eprintln!("oracle_tokens: {:?}", oracle_tokens);
        eprintln!("compiled_tokens: {:?}", compiled_tokens);
    }
    assert!(
        similarity >= 0.99,
        "expected self-reference normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for self-reference clause");
}

#[test]
fn compare_semantics_penalizes_unless_pay_role_inversion() {
    let oracle =
        "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever an opponent casts a spell, you may draw a card unless you pay {1}.",
    )];
    let (_oracle_coverage, _compiled_coverage, similarity, _line_delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "payer-role inversion must count as semantic mismatch"
    );
    assert!(
        similarity < 0.99,
        "payer-role inversion should not remain above strict 0.99 score floor (score={similarity})"
    );
}

#[test]
fn compare_semantics_normalizes_any_combination_of_colors_wording() {
    let oracle = "Add two mana in any combination of colors.\nDraw a card.";
    let compiled = vec![String::from(
        "Spell effects: Add 2 mana in any combination of {W} and/or {U} and/or {B} and/or {R} and/or {G}. Draw a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected any-combination mana wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for any-combination mana wording"
    );
}

#[test]
fn compare_semantics_keeps_side_effect_on_second_mana_ability() {
    let oracle = "{T}: Add {C}.\n{T}: Add one mana of any color. This land deals 3 damage to you.";
    let compiled = vec![
        String::from("Mana ability 1: {T}: Add {C}."),
        String::from("Mana ability 2: {T}: Add one mana of any color. Deal 3 damage to you."),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected side-effect mana ability normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for side-effect mana ability"
    );
}

#[test]
fn compare_semantics_keeps_separate_same_cost_mana_abilities() {
    let oracle = "{T}: Add {C}.\n{T}: Add {B} or {R}.";
    let compiled = vec![
        String::from("Mana ability 1: {T}: Add {C}."),
        String::from("Mana ability 2: {T}: Add {B} or {R}."),
    ];
    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(
        !mismatch,
        "separate mana abilities must not become one choice"
    );
}

#[test]
fn compare_semantics_normalizes_reveal_land_then_enters_tapped_dual_land_wording() {
    let oracle = "As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped.\n{T}: Add {G} or {W}.";
    let compiled = vec![
        String::from(
            "Static ability 1: As this land enters you may reveal a forest or plains card from your hand if you dont this land enters tapped.",
        ),
        String::from("Mana ability 2: {T}: Add {G}."),
        String::from("Mana ability 3: {T}: Add {W}."),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected reveal-dual normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for reveal-dual wording");
}

#[test]
fn compare_semantics_normalizes_tapped_for_mana_enchantment_wording() {
    let oracle = "Enchant land\nWhenever enchanted land is tapped for mana, its controller adds an additional {G}.";
    let compiled = vec![
        String::from("Enchant land"),
        String::from(
            "Triggered ability 1: Whenever a player taps a enchanted land for mana: that object's controller adds {G}.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected tapped-for-mana enchantment normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for tapped-for-mana enchantment wording"
    );
}

#[test]
fn compare_semantics_normalizes_copy_spell_wording() {
    let oracle = "Copy target instant or sorcery spell. You may choose new targets for the copy.";
    let compiled = vec![String::from(
        "Spell effects: Copy target instant and sorcery spell 1 time(s). you may choose new targets for this spell.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    // The "1 time(s)" scaffolding in this surface now costs score by
    // design (numbers compare literally and scaffolding tokens are no
    // longer dropped); the remaining wording variance still normalizes.
    assert!(
        similarity >= 0.985,
        "expected copy-spell wording normalization to stay near strict threshold, got {similarity}"
    );
    // The scaffolding-laden surface is now honestly flagged as a
    // mismatch; only the score-level tolerance is retained above.
    assert!(
        mismatch,
        "expected scaffolding-laden copy-spell wording to flag a mismatch"
    );
}

#[test]
fn compare_semantics_normalizes_split_gets_and_gains_clause() {
    let oracle = "{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.";
    let compiled = vec![String::from(
        "Activated ability 2: {X}{R}{G}, {T}: target creature gets +X/+0 until end of turn. it gains Trample until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected split gets/gains normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for split gets/gains wording"
    );
}

#[test]
fn compare_semantics_normalizes_you_and_target_opponent_each_draw_wording() {
    let oracle = "{T}: You and target opponent each draw a card.";
    let compiled = vec![String::from(
        "Activated ability 3: {T}: you draw a card. target opponent draws a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected each-draw wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for each-draw wording");
}

#[test]
fn normalize_trigger_subject_for_compare_skips_multi_subject_triggers() {
    let line = "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.";
    assert_eq!(normalize_trigger_subject_for_compare(line), line);
}

#[test]
fn compare_semantics_normalizes_sacrifice_damage_source_wording() {
    let oracle = "Sacrifice this artifact: It deals 2 damage to any target.";
    let compiled = vec![String::from(
        "Activated ability 1: Sacrifice this artifact: this artifact deals 2 damage to any target.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected sacrifice damage-source wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for sacrifice damage-source wording"
    );
}

#[test]
fn compare_semantics_normalizes_that_player_pronouns_globally() {
    let oracle = "When this creature enters, each opponent discards a card and you gain 3 life.";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, for each opponent, that player discards a card and you gain 3 life.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected global that-player normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for global that-player normalization"
    );
}

#[test]
fn compare_semantics_normalizes_each_other_attacking_creature_plurality() {
    let oracle =
        "Whenever this creature attacks, other attacking creatures get +1/+0 until end of turn.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected other-attacking-creatures plurality normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for other-attacking-creatures plurality normalization"
    );
}

#[test]
fn compare_semantics_normalizes_lose_then_add_split() {
    let oracle = "Whenever a creature enters, you lose 1 life and add {B}.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever a creature enters, you lose 1 life. Add {B}.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected lose-then-add split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for lose-then-add split normalization"
    );
}

#[test]
fn compare_semantics_normalizes_draw_then_target_opponent_gains_split() {
    let oracle = "{3}{W}{W}: You draw a card and target opponent gains 3 life.";
    let compiled = vec![String::from(
        "Activated ability 1: {3}{W}{W}: you draw a card. target opponent gains 3 life.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected draw-then-gain split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for draw-then-gain split normalization"
    );
}

#[test]
fn compare_semantics_normalizes_tap_then_put_split() {
    let oracle = "Tap target creature and put three stun counters on it. Scry 1.";
    let compiled = vec![String::from(
        "Spell effects: Tap target creature. Put three stun counters on it. Scry 1.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected tap-then-put split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for tap-then-put split normalization"
    );
}

#[test]
fn compare_semantics_normalizes_draw_then_put_counter_split() {
    let oracle = "{6}: Draw a card and put a +1/+1 counter on this creature.";
    let compiled = vec![String::from(
        "Activated ability 2: {6}: you draw a card. Put a +1/+1 counter on this creature.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected draw-then-put split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for draw-then-put split normalization"
    );
}

#[test]
fn compare_semantics_normalizes_choose_target_followup_scaffolding() {
    let oracle = "Target creature you control deals damage equal to its power to target creature an opponent controls.";
    let compiled = vec![String::from(
        "Spell effects: Choose target creature you control. that creature deals damage equal to its power to target creature an opponent controls.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected choose-target followup normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for choose-target followup normalization"
    );
}

#[test]
fn compare_semantics_normalizes_pay_life_cost_wording() {
    let oracle = "{2}, Pay 2 life: Draw a card.";
    let compiled = vec![String::from(
        "Activated ability 1: {2}, Lose 2 life: you draw a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected pay-life cost normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for pay-life cost normalization"
    );
}

#[test]
fn compare_semantics_normalizes_untap_then_gets_and_gains_split() {
    let oracle = "Untap target creature. It gets +2/+2 and gains reach until end of turn.";
    let compiled = vec![String::from(
        "Spell effects: Untap target creature. it gets +2/+2 until end of turn. it gains Reach until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected untap-then-buff split normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for untap-then-buff split normalization"
    );

    let compiled = vec![String::from(
        "Untap target creature, it gets +2/+2 until end of turn, then it gains Reach until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected comma/then untap-then-buff normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for comma/then untap-then-buff normalization"
    );
}

#[test]
fn compare_semantics_normalizes_target_spell_or_nonland_permanent_wording() {
    let oracle =
        "Return target spell or nonland permanent an opponent controls to its owner's hand.";
    let compiled = vec![String::from(
        "Spell effects: Return target opponent's nonland spell or an opponent's nonland permanent to its owner's hand.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected opponent-controlled spell/permanent wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for opponent-controlled spell/permanent wording"
    );
}

#[test]
fn compare_semantics_normalizes_each_land_basic_type_wording() {
    let oracle = "Each land is a Swamp in addition to its other land types.";
    let compiled = vec![String::from(
        "Static ability 1: Lands are Swamps in addition to their other types.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected each-land-type wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for each-land-type wording");
}

#[test]
fn compare_semantics_flags_reflexive_when_you_do_vs_if_you_do_mismatch() {
    let oracle = "Whenever Felothar enters or attacks, you may sacrifice a nonland permanent. When you do, put a +1/+1 counter on each creature you control.";
    let compiled = vec![String::from(
        "Triggered ability 2: When Felothar enters or this creature attacks, you may sacrifice a nonland permanent you control. If you do, Put a +1/+1 counter on each creature you control.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity < 0.99,
        "expected reflexive-trigger vs conditional wording to stay below strict threshold, got {similarity}"
    );
    assert!(
        mismatch,
        "expected mismatch for reflexive-trigger vs conditional wording"
    );
}

#[test]
fn compare_semantics_normalizes_soulbond_keyword_scaffolding() {
    let oracle = "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)
As long as this creature is paired with another creature, each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"";
    let compiled = vec![
        String::from(
            "Triggered ability 1: Whenever a creature you control enters, effect(SoulbondPairEffect)",
        ),
        String::from(
            "Static ability 2: As long as this is paired with another creature each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected soulbond keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for soulbond keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_named_soulbond_pairing_surface() {
    let oracle = "As long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"";
    let compiled = vec![String::from(
        "Static ability 3: As long as this creature is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected named soulbond pairing wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for named soulbond pairing wording"
    );
}

#[test]
fn compare_semantics_normalizes_dethrone_keyword_scaffolding() {
    let oracle = "Dethrone (Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on it.)
Pay 3 life: Add {R}.";
    let compiled = vec![
        String::from(
            "Triggered ability 1: Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on this creature.",
        ),
        String::from("Mana ability 2: Pay 3 life: Add {R}."),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected dethrone keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for dethrone keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_accorder_paladin_battle_cry_keyword_scaffolding() {
    let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for battle cry keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_blade_instructor_mentor_keyword_scaffolding() {
    let oracle = "Mentor (Whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less than this creature's power.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected mentor keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for mentor keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_dead_reveler_unleash_keyword_scaffolding() {
    let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
        ),
        String::from(
            "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for unleash keyword scaffolding"
    );
}

#[test]
fn compare_card_semantics_normalizes_titled_legend_short_self_reference() {
    let oracle = "At the beginning of each of your postcombat main phases, if you gained 3 or more life this turn, exile Sorin, then return him to the battlefield transformed under his owner's control.";
    let normalized = normalize_card_self_references_for_compare(oracle, "Sorin of House Markov");
    assert!(
        normalized.contains("exile this"),
        "expected titled-legend short self-reference normalization to replace the lead name, got {normalized}"
    );
    assert!(
        !normalized.contains("Sorin"),
        "expected titled-legend short self-reference normalization to remove the lead name, got {normalized}"
    );
}

#[test]
fn compare_semantics_normalizes_debtors_transport_afterlife_keyword_scaffolding() {
    let oracle = "Afterlife 2 (When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected afterlife keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for afterlife keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_dokuchi_shadow_walker_ninjutsu_keyword_scaffolding() {
    let oracle = "Ninjutsu {3}{B} ({3}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)";
    let compiled = vec![String::from(
        "Activated ability 1: {3}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield tapped and attacking. Activate only during combat.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected ninjutsu keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for ninjutsu keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_goblin_grappler_provoke_keyword_scaffolding() {
    let oracle = "Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, untap target defending player's creature. target defending player's creature gains Blocks each combat if able until end of combat.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected provoke keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for provoke keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_goblin_wardriver_battle_cry_keyword_scaffolding() {
    let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for battle cry keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_gore_house_chainwalker_unleash_keyword_scaffolding() {
    let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
        ),
        String::from(
            "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for unleash keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_hammer_dropper_mentor_keyword_scaffolding() {
    let oracle = "Mentor (Whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less than this creature's power.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected mentor keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for mentor keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_hookhand_mariner_daybound_keyword_scaffolding() {
    let oracle =
        "Daybound (If a player casts no spells during their own turn, it becomes night next turn.)";
    let compiled = vec![String::from(
        "Triggered ability 1: At the beginning of each player's upkeep, if this creature is transformed, if two or more spells were cast last turn, transform this creature. Otherwise, if no spells were cast last turn, transform this creature.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected daybound/nightbound keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for daybound/nightbound keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_knight_of_the_pilgrims_road_renown_keyword_scaffolding() {
    let oracle = "Renown 1 (When this creature deals combat damage to a player, if it isn't renowned, put a +1/+1 counter on it and it becomes renowned.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature deals combat damage to a player, if this creature isn't renowned, put 1 +1/+1 counter on it and it becomes renowned.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected renown keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for renown keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_loxodon_partisan_battle_cry_keyword_scaffolding() {
    let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for battle cry keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_ministrant_of_obligation_afterlife_keyword_scaffolding() {
    let oracle = "Afterlife 2 (When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected afterlife keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for afterlife keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_ninja_of_the_new_moon_ninjutsu_keyword_scaffolding() {
    let oracle = "Ninjutsu {3}{B} ({3}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)";
    let compiled = vec![String::from(
        "Activated ability 1: {3}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield tapped and attacking. Activate only during combat.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected ninjutsu keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for ninjutsu keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_rakdos_cackler_unleash_keyword_scaffolding() {
    let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
        ),
        String::from(
            "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for unleash keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_rampaging_rendhorn_riot_keyword_scaffolding() {
    let oracle = "Riot (This creature enters with your choice of a +1/+1 counter or haste.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, choose one — • This creature enters with a +1/+1 counter on it. • This creature gains haste until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected riot keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for riot keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_spawn_of_rix_maadi_unleash_keyword_scaffolding() {
    let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
        ),
        String::from(
            "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for unleash keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_tavern_ruffian_daybound_keyword_scaffolding() {
    let oracle =
        "Daybound (If a player casts no spells during their own turn, it becomes night next turn.)";
    let compiled = vec![String::from(
        "Triggered ability 1: At the beginning of each player's upkeep, if this creature is transformed, if two or more spells were cast last turn, transform this creature. Otherwise, if no spells were cast last turn, transform this creature.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected daybound/nightbound keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for daybound/nightbound keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_visionary_augmenter_fabricate_keyword_scaffolding() {
    let oracle = "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, choose one — • Put two +1/+1 counters on this creature. • Create two 1/1 colorless Servo artifact creature tokens.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected fabricate keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for fabricate keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_weaponcraft_enthusiast_fabricate_keyword_scaffolding() {
    let oracle = "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, choose one — • Put two +1/+1 counters on this creature. • Create two 1/1 colorless Servo artifact creature tokens.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected fabricate keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for fabricate keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_scavenge_keyword_scaffolding() {
    let oracle = "Scavenge {5}{G} ({5}{G}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Scavenge only as a sorcery.)";
    let compiled = vec![String::from(
        "Activated ability 1: {5}{G}, Exile this card from your graveyard: Put this creature's power +1/+1 counter(s) on target creature. Activate only as a sorcery.",
    )];
    let compiled_clauses = super::semantic_clauses_for_compare(&compiled.join("\n"));
    assert_eq!(
        compiled_clauses,
        vec![String::from("Scavenge {5}{G}")],
        "expected compiled scavenge scaffolding to normalize to keyword form"
    );
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected scavenge keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for scavenge keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_myriad_keyword_scaffolding() {
    let oracle = "Myriad (Whenever this creature attacks, for each opponent other than defending player, you may create a token copy that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.)";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever this creature attacks, for each opponent other than defending player, you may Create a token that's a copy of this creature, tapped, attacking that player or a planeswalker they control, and exile at end of combat.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected myriad keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for myriad keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_zhur_taa_goblin_riot_keyword_scaffolding() {
    let oracle = "Riot (This creature enters with your choice of a +1/+1 counter or haste.)";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, choose one — • This creature enters with a +1/+1 counter on it. • This creature gains haste until end of turn.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected riot keyword normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for riot keyword scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_echo_counter_scaffolding() {
    let oracle = "Flying, protection from black
Echo {3}{W}{W}
When this creature enters, return target creature card from your graveyard to the battlefield.";
    let compiled = vec![
        String::from("Keyword ability 1: Flying, Protection from black"),
        String::from("Static ability 3: This creature enters with an echo counter on it."),
        String::from(
            "Triggered ability 4: At the beginning of your upkeep, remove an echo counter from this creature. If effect #0 happened, Sacrifice this creature unless you pay {3}{W}{W}.",
        ),
        String::from(
            "Triggered ability 5: When this creature enters, return target creature card from your graveyard to the battlefield.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected echo scaffolding normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for echo counter scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_echo_counter_scaffolding_without_other_keywords() {
    let oracle = "Echo {3}
{1}, {T}: Tap target artifact, creature, or land.";
    let compiled = vec![
        String::from("Static ability 1: This artifact enters with an echo counter on it."),
        String::from(
            "Triggered ability 2: At the beginning of your upkeep, remove an echo counter from this artifact. If you do, sacrifice this artifact unless you pay {3}.",
        ),
        String::from("{1}, {T}: Tap target artifact or creature or land."),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected echo-only scaffolding normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for echo-only scaffolding");
}

#[test]
fn compare_semantics_normalizes_reveal_choice_sentence_helper_scaffolding() {
    let oracle = "Reveal the top six cards of your library. You may put up to one land card from among them onto the battlefield tapped and up to one Elf card from among them into your hand. Put the rest on the bottom of your library in a random order.";
    let compiled = vec![String::from(
        "Look at the top six cards of your library. Reveal it. you choose up to one land card in library and tags it as '__sentence_helper_chosen_l0_s0_e1'. Put it onto the battlefield tapped. you choose up to one other Elf card in library and tags it as '__sentence_helper_chosen_l0_s0_e3'. Return it to its owner's hand. Put the remaining tagged cards on the bottom of your library in a random order.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected reveal-choice helper normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for reveal-choice helper scaffolding"
    );
}

#[test]
fn compare_semantics_normalizes_nonmana_echo_counter_scaffolding() {
    let oracle = "Haste
Echo—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)";
    let compiled = vec![
        String::from("Static ability 1: Haste."),
        String::from("Static ability 2: This creature enters with an echo counter on it."),
        String::from(
            "Triggered ability 3: At the beginning of your upkeep, remove an echo counter from this creature. If you do, Sacrifice this creature unless you discard a card.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected non-mana echo scaffolding normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for non-mana echo counter scaffolding"
    );
}

#[test]
fn compare_semantics_flags_missing_esper_sentinel_where_x_power_clause() {
    let oracle = "Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever an opponent casts noncreature spell as that player's first spell this turn, you draw a card unless they pay {X}.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity < 0.99,
        "expected missing where-X power clause to stay below strict threshold, got {similarity}"
    );
    assert!(
        mismatch,
        "expected mismatch when where-X power clause is missing"
    );
}

#[test]
fn compare_semantics_flags_first_noncreature_scope_mismatch() {
    let oracle = "Whenever an opponent casts their first noncreature spell each turn, draw a card.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever an opponent casts noncreature spell as that player's first spell this turn, draw a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "expected mismatch when first-noncreature scope is rewritten to first-spell scope"
    );
    assert!(
        similarity < 0.99,
        "first-noncreature scope mismatch should stay below strict 0.99 threshold (score={similarity})"
    );
}

#[test]
fn compare_semantics_flags_opponent_controls_vs_you_dont_control_mismatch() {
    let oracle = "Destroy target creature an opponent controls.";
    let compiled = vec![String::from(
        "Spell effects: Destroy target creature you don't control.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "expected mismatch when opponent-controls scope is rewritten to you-don't-control scope"
    );
    assert!(
        similarity < 0.99,
        "opponent-controls scope mismatch should stay below strict 0.99 threshold (score={similarity})"
    );
}

#[test]
fn compare_semantics_flags_instant_and_or_target_mismatch_outside_copy_context() {
    let oracle = "Counter target instant or sorcery spell.";
    let compiled = vec![String::from(
        "Spell effects: Counter target instant and sorcery spell.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "expected mismatch when instant-or-sorcery target is rewritten as instant-and-sorcery"
    );
    assert!(
        similarity < 0.99,
        "instant-and/or target mismatch should stay below strict 0.99 threshold (score={similarity})"
    );
}

#[test]
fn compare_semantics_flags_missing_activated_ability_cost_floor_clause() {
    let oracle = "Activated abilities of creatures you control cost {2} less to activate.
This effect can't reduce the mana in that cost to less than one mana.";
    let compiled = vec![String::from(
        "Static ability 1: Activated abilities of creatures you control cost {2} less to activate.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity < 0.99,
        "expected missing minimum-cost clause to stay below strict threshold, got {similarity}"
    );
    assert!(
        mismatch,
        "expected mismatch when activated-ability cost floor clause is missing"
    );
}

#[test]
fn compare_semantics_flags_counter_type_erasure_in_remove_cost() {
    let oracle = "{T}, Remove a +1/+1 counter from this creature: Draw a card.";
    let compiled = vec![String::from(
        "Activated ability 1: {T}, Remove a counter from this creature: Draw a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity < 0.99,
        "expected counter-type erasure to stay below strict threshold, got {similarity}"
    );
    assert!(
        mismatch,
        "expected mismatch when specific counter type is erased"
    );
}

#[test]
fn compare_semantics_flags_enchanted_type_erasure_from_tagged_object_scaffolding() {
    let oracle = "Destroy enchanted creature.";
    let compiled = vec![String::from(
        "Spell effects: Tag the object attached to this Aura as 'enchanted'. Destroy target tagged object 'enchanted'.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity < 0.99,
        "expected enchanted-type erasure to stay below strict threshold, got {similarity}"
    );
    assert!(
        mismatch,
        "expected mismatch when enchanted target type is reduced to generic tagged object"
    );
}

#[test]
fn compare_semantics_normalizes_grant_play_tagged_scaffolding() {
    let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
    let compiled = vec![String::from(
        "Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(GrantPlayTaggedEffect { tag: TagKey(\"exiled_0\"), player: You, duration: UntilEndOfTurn })",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected grant-play-tagged normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for grant-play-tagged scaffolding"
    );
}

#[test]
fn compare_semantics_flags_generic_effect_scaffolding_not_as_play_permission() {
    let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
    let compiled = vec![String::from(
        "Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(SomeOtherEffect { player: You })",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "generic Effect(...) scaffolding should not be normalized as play permission (score={similarity})"
    );
}

#[test]
fn compare_semantics_normalizes_named_wish_counter_wording() {
    let oracle = "This artifact enters with three wish counters on it.
{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.";
    let compiled = vec![
        String::from("Static ability 1: This artifact enters with three wish counters on it."),
        String::from(
            "Activated ability 2: {1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected named-counter normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for named-counter wording");
}

#[test]
fn compare_semantics_normalizes_pact_upkeep_payment_clause() {
    let oracle = "Counter target spell.
At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.";
    let compiled = vec![
        String::from("Spell effects: Counter target spell."),
        String::from(
            "Triggered ability 1: At the beginning of your upkeep, you pay {3}{U}{U}. If that doesn't happen, you lose the game.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected pact upkeep normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for pact upkeep wording");
}

#[test]
fn compare_semantics_flags_homeward_path_owned_creatures_quantifier_loss() {
    let oracle = "{T}: Add {C}.
{T}: Each player gains control of all creatures they own.";
    let compiled = vec![
        String::from("Mana ability 1: {T}: Add {C}."),
        String::from(
            "Activated ability 2: {T}: For each player, that player gains control of a creature that player owns.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        mismatch,
        "quantifier loss from 'all creatures' to singular should be a mismatch (score={similarity})"
    );
}

#[test]
fn compare_semantics_normalizes_heat_shimmer_temporary_copy_clause() {
    let oracle = "Create a token that's a copy of target creature, except it has haste and \"At the beginning of the end step, exile this token.\"";
    let compiled = vec![String::from(
        "Spell effects: Create a token that's a copy of target creature, with haste, and exile it at the beginning of the next end step.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected temporary-copy normalization to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected no mismatch for temporary-copy wording");
}

#[test]
fn compare_semantics_keeps_visible_conspire_keyword_from_dragging_compiled_coverage_down() {
    let oracle = "Burn Trail deals 3 damage to any target.\nConspire (As you cast this spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose a new target for the copy.)";
    let compiled = vec![
        String::from("Spell effects: Deal 3 damage to any target."),
        String::from(
            "Conspire (As you cast this spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose a new target for the copy.)",
        ),
    ];
    let (oracle_coverage, compiled_coverage, _similarity, _delta, _mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    let (_oracle_coverage, _compiled_coverage, similarity, _delta, _mismatch) =
        compare_card_semantics_scored("Burn Trail", oracle, &compiled, strict_embedding());
    assert!(
        oracle_coverage >= 0.70,
        "expected damage clause to stay aligned even with the named self-reference, got {oracle_coverage}"
    );
    assert!(
        compiled_coverage >= 0.70,
        "expected visible Conspire keyword line to avoid counting as an unmatched extra clause, got {compiled_coverage}"
    );
    assert!(
        similarity >= 0.95,
        "expected Burn Trail reminder-text normalization to clear the 0.95 floor, got {similarity}"
    );
}

#[test]
fn compare_semantics_normalizes_boggart_trawler_graveyard_exile_clause() {
    let oracle = "When this creature enters, exile target player's graveyard.";
    let compiled = vec![String::from(
        "Triggered ability 1: When this creature enters, exile all cards from target player's graveyard.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected graveyard-exile normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for graveyard-exile wording"
    );
}

#[test]
fn compare_semantics_normalizes_static_prison_sentence_split_and_pay_typo() {
    let oracle = "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield. You get {E}{E}.
At the beginning of your first main phase, sacrifice this enchantment unless you pay {E}.";
    let compiled = vec![
        String::from(
            "Triggered ability 1: When this enchantment enters, exile target opponent's nonland permanent until this enchantment leaves the battlefield and you get {E}{E}.",
        ),
        String::from(
            "Triggered ability 2: At the beginning of your first main phase, sacrifice this enchantment unless you Pay {E}.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected static-prison normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for static-prison sentence/typo wording"
    );
}

#[test]
fn compare_semantics_normalizes_saw_in_half_death_copy_wording() {
    let oracle = "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time.";
    let compiled = vec![String::from(
        "Spell effects: Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under its controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected saw-in-half normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for saw-in-half death-copy wording"
    );
}

#[test]
fn compare_semantics_normalizes_hullbreaker_horror_modal_bullet_formatting() {
    let oracle = "Whenever you cast a spell, choose up to one —
• Return target spell you don't control to its owner's hand.
• Return target nonland permanent to its owner's hand.";
    let compiled = vec![String::from(
        "Triggered ability 3: Whenever you cast a spell, choose up to one - Return target spell you don't control to its owner's hand. • Return target nonland permanent to its owner's hand.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        let oracle_tokens = semantic_clauses(oracle)
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        let compiled_tokens = semantic_clauses(&compiled.join("\n"))
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        eprintln!("oracle_tokens={:?}", oracle_tokens);
        eprintln!("compiled_tokens={:?}", compiled_tokens);
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected hullbreaker modal formatting normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for hullbreaker modal formatting wording"
    );
}

#[test]
fn compare_semantics_normalizes_ertai_modal_bullet_formatting() {
    let oracle = "When this creature enters, choose up to one —
• Counter target spell, activated ability, or triggered ability. Its controller draws a card.
• Destroy another target creature or planeswalker. Its controller draws a card.";
    let compiled = vec![String::from(
        "Triggered ability 2: When this creature enters, choose up to one - Counter target spell, activated ability, or triggered ability. Its controller draws a card. • Destroy another target creature or planeswalker. Its controller draws a card.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        let oracle_tokens = semantic_clauses(oracle)
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        let compiled_tokens = semantic_clauses(&compiled.join("\n"))
            .iter()
            .map(|clause| super::comparison_tokens(clause))
            .collect::<Vec<_>>();
        eprintln!("oracle_tokens={:?}", oracle_tokens);
        eprintln!("compiled_tokens={:?}", compiled_tokens);
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected ertai modal formatting normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for ertai modal formatting wording"
    );
}

#[test]
fn compare_semantics_normalizes_choose_one_or_both_modal_bullet_formatting() {
    let oracle = "Flash
When Flash Thompson enters, choose one or both —
• Heckle — Tap target creature.
• Hero Worship — Untap target creature.";
    let compiled = vec![
        String::from("Static ability 1: Flash."),
        String::from(
            "Triggered ability 2: When Flash Thompson enters, choose up to two - Tap target creature. • Untap target creature.",
        ),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected choose-one-or-both modal formatting normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for choose-one-or-both modal formatting wording"
    );
}

#[test]
fn compare_semantics_accepts_exact_choose_one_modal_bullets() {
    let oracle = "When this creature enters, choose one —
• Put a shield counter on target permanent.
• Proliferate.";
    let compiled = vec![oracle.to_string()];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
        eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
        eprintln!(
            "compiled_clauses={:?}",
            semantic_clauses(&compiled.join("\n"))
        );
        eprintln!("similarity={similarity} mismatch={mismatch}");
    }
    assert!(
        similarity >= 0.99,
        "expected exact choose-one bullet text to stay above strict threshold, got {similarity}"
    );
    assert!(!mismatch, "expected exact choose-one bullet text to match");
}

#[test]
fn compare_semantics_normalizes_attack_trigger_one_or_more_creatures_wording() {
    let oracle = "Whenever you attack, for each opponent, create a 1/1 black Ninja creature token that's tapped and attacking that player.";
    let compiled = vec![String::from(
        "Triggered ability 1: Whenever one or more creature you control attack, for each opponent, Create a 1/1 black Ninja creature token that's tapped and attacking.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected attack-trigger wording normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for attack-trigger wording normalization"
    );
}

#[test]
fn compare_semantics_normalizes_urzas_saga_zero_or_one_mana_cost_wording() {
    let oracle = "III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.";
    let compiled = vec![String::from(
        "Triggered ability 3: III — Search your library for an artifact card with mana value 1 or less, put it onto the battlefield, then shuffle.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(
        similarity >= 0.99,
        "expected urza-saga mana-cost normalization to stay above strict threshold, got {similarity}"
    );
    assert!(
        !mismatch,
        "expected no mismatch for urza-saga mana-cost wording"
    );
}

#[test]
fn compare_semantics_normalizes_zombie_ogre_morbid_condition_surface() {
    let oracle = "At the beginning of your end step, if a creature died this turn, venture into the dungeon.";
    let compiled = vec![String::from(
        "At the beginning of your end step, if one or more creatures died this turn, venture into the dungeon.",
    )];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(similarity >= 0.99, "score={similarity}");
    assert!(!mismatch);
}

#[test]
fn compare_semantics_normalizes_arrest_split_prison_surface() {
    let oracle = "Enchant creature\nEnchanted creature can't attack or block, and its activated abilities can't be activated.";
    let compiled = vec![
        "Enchant creature".to_string(),
        "Enchanted creature can't attack or block.".to_string(),
        "Enchanted creature activated abilities can't be activated.".to_string(),
    ];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        compare_semantics_scored(oracle, &compiled, strict_embedding());
    assert!(similarity >= 0.99, "score={similarity}");
    assert!(!mismatch);
}
