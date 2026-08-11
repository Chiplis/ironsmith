use super::*;

fn linked_exile_top_play_effects(
    exile_player: PlayerFilter,
    grant_player: PlayerFilter,
    duration: crate::effects::GrantPlayTaggedDuration,
    allow_land: bool,
    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
) -> Vec<Effect> {
    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    vec![
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), exile_player)
                .tag_moved(tag.clone()),
        ),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            tag,
            grant_player,
            duration,
            allow_land,
            mana_spend_mode,
        )),
    ]
}

fn assert_clause_uses_typed_sentence_boundary(effects: &[Effect], permission: &str) {
    let clause = describe_effect_clause_list(effects)
        .expect("a linked exile-top permission should have a clause surface");
    assert!(
        clause.contains("library. "),
        "expected a typed sentence boundary after the exile action: {clause}"
    );
    assert!(
        clause.contains(permission),
        "expected permission `{permission}` in: {clause}"
    );
    assert!(
        !clause.contains(", then "),
        "linked exile and permission must not fall back to a comma-then list: {clause}"
    );
}

#[test]
fn linked_exile_top_play_accepts_duration_player_and_land_variants() {
    let controls = [
        (
            PlayerFilter::You,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "You may play that card this turn",
        ),
        (
            PlayerFilter::DamagedPlayer,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            ironsmith_core::value_model::ManaSpendMode::AnyColor,
            "you may cast that card",
        ),
        (
            PlayerFilter::You,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd,
            true,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "you may play that card",
        ),
        (
            PlayerFilter::You,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep,
            true,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "you may play that card",
        ),
        (
            PlayerFilter::You,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
            true,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "may play that card for as long as it remains exiled",
        ),
    ];

    for (exile_player, grant_player, duration, allow_land, mana_mode, permission) in controls {
        let effects = linked_exile_top_play_effects(
            exile_player,
            grant_player,
            duration,
            allow_land,
            mana_mode,
        );
        assert_clause_uses_typed_sentence_boundary(&effects, permission);
    }
}

#[test]
fn plural_exile_pool_with_one_shared_play_renders_one_of_those_cards() {
    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let exile = Effect::new(
        crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(2), PlayerFilter::You)
            .tag_moved(tag.clone()),
    );
    let mut grant = crate::effects::GrantPlayTaggedEffect::new(
        tag,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    grant.cast_pool_is_plural = true;
    grant.max_plays = Some(1);

    assert_eq!(
        describe_effect_clause_list(&[exile.clone(), Effect::new(grant.clone())]).as_deref(),
        Some(
            "exile the top two cards of your library. Until your next end step, you may play one of those cards"
        )
    );

    for max_plays in [None, Some(2)] {
        grant.max_plays = max_plays;
        assert!(
            describe_effect_clause_list(&[exile.clone(), Effect::new(grant.clone())]).is_some_and(
                |text| {
                    text.contains("may play those cards")
                        && !text.contains("play one of those cards")
                }
            ),
            "a non-singleton play budget must not inherit the one-card choice surface"
        );
    }

    grant.max_plays = Some(1);
    grant.duration = crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd;
    assert!(
        describe_effect_clause_list(&[exile.clone(), Effect::new(grant.clone())])
            .is_some_and(|text| text.contains("may play those cards")
                && !text.contains("play one of those cards")),
        "a different permission duration must not inherit the exact next-end-step surface"
    );

    grant.duration = crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep;
    grant.tag = TagKey::from("__different_exiled_collection");
    assert!(
        describe_effect_clause_list(&[exile, Effect::new(grant)])
            .is_none_or(|text| !text.contains("play one of those cards")),
        "unrelated exile and permission tags must not be rendered as one linked collection"
    );
}

#[test]
fn linked_exile_top_play_renders_source_exile_event_boundary() {
    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
        .with_until_source_exiles_another(
            crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ),
        );
    let effects = vec![
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), PlayerFilter::You)
                .tag_moved(tag.clone()),
        ),
        Effect::new(
            crate::effects::GrantPlayTaggedEffect::new(
                tag,
                PlayerFilter::You,
                crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother,
                true,
                ironsmith_core::value_model::ManaSpendMode::Normal,
            )
            .with_surface(surface),
        ),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "exile the top card of your library. You may play that card until you exile another card with this enchantment"
        )
    );
}

#[test]
fn linked_exile_top_play_accepts_structural_tag_and_id_wrappers() {
    let mut effects = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    let exile = effects.remove(0);
    let grant = effects.remove(0);
    let effects = vec![
        Effect::with_id(17, exile.tag("exile_result")),
        Effect::with_id(18, grant.tag("permission_result")),
    ];

    assert_clause_uses_typed_sentence_boundary(&effects, "may play that card this turn");
}

#[test]
fn create_token_prefix_stays_coordinated_before_linked_permission_boundary() {
    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let effects = vec![
        Effect::create_tokens(crate::cards::tokens::treasure_token_definition(), 1)
            .tag("created_0"),
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(
                Value::Fixed(1),
                PlayerFilter::DamagedPlayer,
            )
            .tag_moved(tag.clone()),
        ),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            tag,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            false,
        )),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card"
        )
    );
}

#[test]
fn noncoordinated_prefix_and_intervening_effects_keep_their_sentence_surfaces() {
    let mut prefixed = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    prefixed.insert(0, Effect::plus_one_counters(2, ChooseSpec::Source));
    assert_eq!(
        describe_effect_clause_list(&prefixed).as_deref(),
        Some(
            "put two +1/+1 counters on this source. Exile the top card of your library. Until the end of your next turn, you may play that card"
        )
    );

    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let daxos = vec![
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(
                Value::Fixed(1),
                PlayerFilter::DamagedPlayer,
            )
            .tag_moved(tag.clone()),
        ),
        Effect::new(crate::effects::GainLifeEffect::you(Value::ManaValueOf(
            Box::new(ChooseSpec::Tagged(tag.clone())),
        ))),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            tag,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            ironsmith_core::value_model::ManaSpendMode::AnyColor,
        )),
    ];
    assert_eq!(
        describe_effect_clause_list(&daxos).as_deref(),
        Some(
            "exile the top card of that player's library. You gain life equal to that card's mana value. Until end of turn, you may cast that card, and you may spend mana as though it were mana of any color to cast that spell"
        )
    );
}

#[test]
fn unrelated_or_qualified_permissions_do_not_certify_a_sentence_boundary() {
    let assert_falls_back = |effects: Vec<Effect>| {
        let clause = describe_effect_clause_list(&effects)
            .expect("generic clause rendering should still provide a fallback");
        assert!(
            clause.contains(", then "),
            "an unlinked or qualified grant must not certify the multi-sentence compact: {clause}"
        );
    };

    let mut mismatched = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    mismatched[1] = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("different_exile_result"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        false,
    ));
    assert_falls_back(mismatched);

    let mut filtered = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    filtered[1] = Effect::new(
        filtered[1]
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            .expect("control grant")
            .clone()
            .with_filter(ObjectFilter::creature()),
    );
    assert_falls_back(filtered);

    let mut top_only = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    top_only[1] = Effect::new(
        top_only[1]
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            .expect("control grant")
            .clone()
            .while_on_top_of_library(),
    );
    assert_falls_back(top_only);

    let source_control = linked_exile_top_play_effects(
        PlayerFilter::You,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
        true,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    assert_falls_back(source_control);
}

#[cfg(ironsmith_runtime_parser_tests)]
fn render_card(name: &str, card_type: CardType, text: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![card_type])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn representative_cards_keep_linked_permission_sentence_boundaries() {
    let ragavan = render_card(
        "Ragavan, Nimble Pilferer",
        CardType::Creature,
        "Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.\nDash {1}{R}",
    );
    assert_eq!(
        ragavan,
        "Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.\nDash {1}{R}"
    );

    let bard = render_card(
        "Bard Class",
        CardType::Enchantment,
        "Whenever you cast a legendary spell, exile the top two cards of your library. You may play them this turn.",
    );
    assert!(
        bard.contains(
            "exile the top two cards of your library. You may play those cards this turn"
        ),
        "{bard}"
    );
    assert!(!bard.contains(", then you may play"), "{bard}");

    let campus = render_card(
        "Campus Renovation",
        CardType::Sorcery,
        "Return up to one target artifact or enchantment card from your graveyard to the battlefield. Exile the top two cards of your library. You may play those cards until the end of your next turn.",
    );
    assert!(
        campus.contains("Exile the top two cards of your library. Until the end of your next turn, you may play those cards"),
        "{campus}"
    );
    assert!(!campus.contains(", then you may play"), "{campus}");

    let daxos = render_card(
        "Daxos of Meletis",
        CardType::Creature,
        "Whenever this creature deals combat damage to a player, exile the top card of that player's library. You gain life equal to that card's mana value. Until end of turn, you may cast that card and you may spend mana as though it were mana of any color to cast that spell.",
    );
    assert!(
        daxos.contains("library. You gain life equal to that card's mana value. Until end of turn, you may cast that card"),
        "{daxos}"
    );
    assert!(!daxos.contains(", then you may cast"), "{daxos}");

    let valakut = render_card(
        "Valakut Exploration",
        CardType::Enchantment,
        "Landfall — Whenever a land you control enters, exile the top card of your library. You may play that card for as long as it remains exiled.",
    );
    assert!(
        valakut.contains("library. You may play that card for as long as it remains exiled"),
        "{valakut}"
    );
    assert!(!valakut.contains(", then you may play"), "{valakut}");

    let soul_partition = render_card(
        "Soul Partition",
        CardType::Instant,
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
    );
    assert_eq!(
        soul_partition,
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast."
    );

    let pyxis = render_card(
        "Pyxis of Pandemonium",
        CardType::Artifact,
        "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield.",
    );
    assert_eq!(
        pyxis,
        "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield."
    );

    let semesters_end = render_card(
        "Semester's End",
        CardType::Instant,
        "Exile any number of target creatures and/or planeswalkers you control. At the beginning of the next end step, return each of them to the battlefield under its owner's control. Each of them enters with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if it's a planeswalker.",
    );
    assert_eq!(
        semesters_end,
        "Exile any number of target creatures and/or planeswalkers you control. At the beginning of the next end step, return each of them to the battlefield under its owner's control. Each of them enters with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if it's a planeswalker."
    );
}
