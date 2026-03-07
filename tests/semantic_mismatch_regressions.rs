use ironsmith::effects::{
    ChooseObjectsEffect, ExileUntilMatchCastEffect, GrantPlayTaggedEffect, ReflexiveTriggerEffect,
    ScheduleDelayedTriggerEffect,
};
use ironsmith::{
    cards::CardDefinitionBuilder, compiled_text::compiled_lines, ids::CardId, types::CardType,
};

fn rendered_lines(text: &str, name: &str, card_types: &[CardType]) -> String {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);

    if !card_types.is_empty() {
        builder = builder.card_types(card_types.to_vec());
    }

    let def = builder
        .parse_text(text)
        .expect("high-priority semantic mismatch oracle text should parse");
    compiled_lines(&def).join(" ").to_ascii_lowercase()
}

#[test]
fn regression_semantic_mismatch_flowstone_sculpture_choice_clause() {
    let rendered = rendered_lines(
        "{2}, Discard a card: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample. (This effect lasts indefinitely.)",
        "Flowstone Sculpture",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("put a +1/+1 counter on a creature"),
        "expected at least one preserved mode from the choice clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_mountain_titan_casting_condition() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Mountain Titan")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.",
        )
        .expect_err("Mountain Titan clause is currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();

    assert!(
        rendered.contains("unsupported until-end-of-turn permission clause"),
        "expected explicit unsupported error for Mountain Titan clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_root_greevil_choice_qualifier() {
    let rendered = rendered_lines(
        "{2}{G}, {T}, Sacrifice this creature: Destroy all enchantments of the color of your choice.",
        "Root Greevil",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("choose one")
            && rendered.contains("white enchantment")
            && rendered.contains("blue enchantment")
            && rendered.contains("black enchantment")
            && rendered.contains("red enchantment")
            && rendered.contains("green enchantment"),
        "expected color-choice semantics to be preserved as explicit modes, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_vesuva_copy_and_enter_tapped() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Vesuva")
        .card_types(vec![CardType::Land])
        .parse_text("You may have this land enter tapped as a copy of any land on the battlefield.")
        .expect_err("Vesuva enters-as-copy replacement is currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();

    assert!(
        rendered.contains("unsupported enters-as-copy replacement clause"),
        "expected explicit unsupported error for Vesuva clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_reckless_blaze_triggered_dies_clause() {
    let rendered = rendered_lines(
        "Reckless Blaze deals 5 damage to each creature. Whenever a creature you control dealt damage this way dies this turn, add {R}.",
        "Reckless Blaze",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("whenever"),
        "expected triggered death clause to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("dies this turn"),
        "expected duration condition on death trigger to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("add {r}"),
        "expected colorless mana gain clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_admonition_angel_nonland_permanent_target() {
    let rendered = rendered_lines(
        "Flying\nLandfall — Whenever a land you control enters, you may exile target nonland permanent other than this creature.\nWhen this creature leaves the battlefield, return all cards exiled with it to the battlefield under their owners' control.",
        "Admonition Angel",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("nonland permanent"),
        "expected landfall target to remain a nonland permanent, got {rendered}"
    );
    assert!(
        !rendered.contains("nonland creature"),
        "landfall target should not narrow to creatures, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_asmira_graveyard_battlefield_clause() {
    let rendered = rendered_lines(
        "Flying\nAt the beginning of each end step, put a +1/+1 counter on Asmira for each creature put into your graveyard from the battlefield this turn.",
        "Asmira, Holy Avenger",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("from the battlefield this turn"),
        "expected battlefield graveyard timing clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_entered_battlefield_under_control_this_turn() {
    let rendered = rendered_lines(
        "Flying, vigilance\n{T}: Put a +1/+1 counter on each creature that entered the battlefield under your control this turn.",
        "Shaile, Dean of Radiance // Embrose, Dean of Shadow",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("entered the battlefield under your control this turn"),
        "expected entry-timing clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_sky_tether_attached_keyword_mix() {
    let rendered = rendered_lines(
        "Enchant creature\nEnchanted creature has defender and loses flying.",
        "Sky Tether",
        &[CardType::Enchantment],
    );

    assert!(
        rendered.contains("enchanted creature has defender"),
        "expected defender grant to remain on the enchanted creature, got {rendered}"
    );
    assert!(
        rendered.contains("enchanted creature loses flying"),
        "expected flying removal to remain on the enchanted creature, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_beast_hunt_reveal_all_creatures() {
    let rendered = rendered_lines(
        "Reveal the top three cards of your library. Put all creature cards revealed this way into your hand and the rest into your graveyard.",
        "Beast Hunt",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("reveal the top three cards of your library"),
        "expected multi-card reveal to remain, got {rendered}"
    );
    assert!(
        rendered.contains("put all creature cards revealed this way into your hand"),
        "expected creature-card selection to remain, got {rendered}"
    );
    assert!(
        rendered.contains("rest into your graveyard"),
        "expected remainder destination to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_uurg_power_only_cda() {
    let rendered = rendered_lines(
        "Uurg's power is equal to the number of land cards in your graveyard.\nAt the beginning of your upkeep, surveil 1.\n{B}{G}, Sacrifice a land: You gain 2 life.",
        "Uurg, Spawn of Turg",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("power is the number of land cards in your graveyard"),
        "expected the power-defining clause to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("its toughness is"),
        "power-only characteristic-defining ability should not invent a toughness clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_dwarven_thaumaturgist_switch_pt() {
    let rendered = rendered_lines(
        "{T}: Switch target creature's power and toughness until end of turn.",
        "Dwarven Thaumaturgist",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("switches power and toughness until end of turn"),
        "expected power/toughness switch effect to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_rhonass_last_stand_plural_untap_lock() {
    let rendered = rendered_lines(
        "Create a 5/4 green Snake creature token. Lands you control don't untap during your next untap step.",
        "Rhonas's Last Stand",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("lands you control don't untap during your next untap step"),
        "expected plural untap lock to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("a land you control can't untap"),
        "plural untap lock should not collapse to a singular land, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_eight_and_a_half_tails_set_white() {
    let rendered = rendered_lines(
        "{1}{W}: Target permanent you control gains protection from white until end of turn.\n{1}: Target spell or permanent becomes white until end of turn.",
        "Eight-and-a-Half-Tails",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("target permanent you control gains protection from white until end of turn"),
        "expected protection ability to remain, got {rendered}"
    );
    assert!(
        rendered.contains("target spell or permanent becomes white until end of turn"),
        "expected color-setting ability to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_vraskas_scorn_library_and_or_graveyard() {
    let rendered = rendered_lines(
        "Target opponent loses 4 life. You may search your library and/or graveyard for a card named Vraska, Scheming Gorgon, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        "Vraska's Scorn",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("search your library and/or graveyard for a card named vraska scheming gorgon"),
        "expected combined library/graveyard search clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("if you search your library this way, shuffle"),
        "expected conditional shuffle clause to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("effect #"),
        "renderer should not leak internal effect ids, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_phyrexian_dragon_engine_from_graveyard_trigger() {
    let rendered = rendered_lines(
        "Double strike\nWhen this creature enters from your graveyard, you may discard your hand. If you do, draw three cards.\nUnearth {3}{R}{R}",
        "Phyrexian Dragon Engine",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("when this creature enters from your graveyard"),
        "expected trigger origin zone to remain in compiled text, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_corpse_augur_graveyard_owner_kept() {
    let rendered = rendered_lines(
        "When this creature dies, you draw X cards and you lose X life, where X is the number of creature cards in target player's graveyard.",
        "Corpse Augur",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("draw a card for each creature card in target player's graveyard"),
        "expected target graveyard qualifier on draw clause, got {rendered}"
    );
    assert!(
        rendered.contains("lose 1 life for each creature card in target player's graveyard"),
        "expected target graveyard qualifier on life-loss clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_end_blaze_epiphany_delayed_exile_choice_permission() {
    let text = "End-Blaze Epiphany deals X damage to target creature. When that creature dies this turn, exile a number of cards from the top of your library equal to its power, then choose a card exiled this way. Until the end of your next turn, you may play that card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "End-Blaze Epiphany")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("End-Blaze Epiphany should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("when that creature dies this turn"),
        "expected delayed death trigger to remain, got {rendered}"
    );
    assert!(
        rendered
            .contains("exile a number of cards from the top of your library equal to its power"),
        "expected top-of-library exile count tied to that creature's power, got {rendered}"
    );
    assert!(
        rendered.contains("until the end of your next turn"),
        "expected next-turn play permission to remain, got {rendered}"
    );

    let effects = def
        .spell_effect
        .expect("spell should lower to spell effects");
    assert_eq!(effects.len(), 2, "expected damage plus one delayed trigger");
    assert!(
        effects
            .iter()
            .all(|effect| effect.downcast_ref::<GrantPlayTaggedEffect>().is_none()),
        "play permission must not resolve immediately at top level"
    );

    let delayed = effects[1]
        .downcast_ref::<ScheduleDelayedTriggerEffect>()
        .expect("second spell effect should be the delayed trigger");
    assert!(
        delayed
            .effects
            .iter()
            .any(|effect| effect.downcast_ref::<ChooseObjectsEffect>().is_some()),
        "delayed trigger should still choose one of the exiled cards"
    );
    assert!(
        delayed
            .effects
            .iter()
            .any(|effect| effect.downcast_ref::<GrantPlayTaggedEffect>().is_some()),
        "play permission should be nested inside the delayed trigger"
    );
}

#[test]
fn regression_semantic_mismatch_dazzling_sphinx_exile_until_instant_or_sorcery() {
    let text = "Flying\nWhenever this creature deals combat damage to a player, that player exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then that player puts the exiled cards that weren't cast this way on the bottom of their library in a random order.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Dazzling Sphinx")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("Dazzling Sphinx should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("until they exile an instant or sorcery card"),
        "expected exile-until match clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("without paying its mana cost"),
        "expected free-cast permission to remain, got {rendered}"
    );
    assert!(
        rendered.contains("weren't cast this way") && rendered.contains("random order"),
        "expected bottom-the-rest random-order clause to remain, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ExileUntilMatchCastEffect"),
        "expected generic exile-until-match effect, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("ChooseObjectsEffect"),
        "top-card choose/exile fallback should not be used anymore, got {abilities_debug}"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(&ability.kind, ironsmith::ability::AbilityKind::Triggered(triggered)
                if triggered.effects.iter().any(|effect| effect.downcast_ref::<ExileUntilMatchCastEffect>().is_some()))
        }),
        "triggered ability should carry the generic exile-until-match runtime effect"
    );
}

#[test]
fn regression_semantic_mismatch_courageous_outrider_look_at_top_reveal_choice() {
    let rendered = rendered_lines(
        "When this creature enters, look at the top four cards of your library. You may reveal a Human card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        "Courageous Outrider",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("look at the top four cards of your library"),
        "expected top-of-library look clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("human"),
        "expected Human-card qualifier to remain tied to the looked-at cards, got {rendered}"
    );
    assert!(
        rendered.contains("put") && rendered.contains("into") && rendered.contains("hand"),
        "expected hand-move clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("bottom of") && rendered.contains("library"),
        "expected rest-on-bottom clause to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("triggering"),
        "looked-at cards should not resolve to the triggering-object tag, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_brawn_graveyard_and_forest_condition() {
    let rendered = rendered_lines(
        "Trample\nAs long as this card is in your graveyard and you control a Forest, creatures you control have trample.",
        "Brawn",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("this card is in your graveyard and you control a forest"),
        "expected both graveyard and Forest conditions to render together, got {rendered}"
    );
    assert!(
        !rendered.contains("and("),
        "static condition renderer should not leak debug formatting, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_harald_mixed_filter_reveal_choice() {
    let rendered = rendered_lines(
        "Menace\nWhen Harald enters, look at the top five cards of your library. You may reveal an Elf, Warrior, or Tyvar card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        "Harald, King of Skemfar",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("elf") && rendered.contains("warrior") && rendered.contains("tyvar"),
        "expected mixed subtype/name filter to remain in the reveal choice, got {rendered}"
    );
    assert!(
        !rendered.contains("named tyvar"),
        "Tyvar should stay a subtype match, not a named-card filter, got {rendered}"
    );
    assert!(
        rendered.contains("put the rest on the bottom of your library"),
        "expected rest-on-bottom clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_harald_from_among_them_compacts() {
    let rendered = rendered_lines(
        "Menace\nWhen Harald enters, look at the top five cards of your library. You may reveal an Elf, Warrior, or Tyvar card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        "Harald, King of Skemfar",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("reveal an elf or warrior or tyvar card from among them"),
        "expected looked-at selection to stay limited to the top cards, got {rendered}"
    );
    assert!(
        !rendered.contains("choose up to one elf or warrior or tyvar in a library"),
        "looked-at selection should not degrade into an unrestricted library choice, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_errand_rider_then_if_negative_control() {
    let rendered = rendered_lines(
        "When this creature enters, draw a card. Then if you don't control a legendary creature, put a card from your hand on the bottom of your library.",
        "Errand-Rider of Gondor",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("draw a card"),
        "expected draw clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("if you don't control a legendary creature")
            || rendered.contains("if you control no legendary creature"),
        "expected negative-control predicate to remain explicit, got {rendered}"
    );
    assert!(
        !rendered.contains("if that doesn't happen"),
        "negative-control condition should not collapse into a result predicate, got {rendered}"
    );
    assert!(
        rendered.contains("put a card from your hand on the bottom of your library"),
        "expected follow-up move clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_formidable_speaker_if_you_do_search() {
    let rendered = rendered_lines(
        "When this creature enters, you may discard a card. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.\n{1}, {T}: Untap another target permanent.",
        "Formidable Speaker",
        &[CardType::Creature],
    );

    assert!(
        rendered
            .contains("you may discard a card. if you do, search your library for a creature card"),
        "expected the discard and search clauses to stay linked by the if-you-do gate, got {rendered}"
    );
    assert!(
        rendered.contains("put it into your hand") && rendered.contains("shuffle"),
        "expected the full search tail to remain after the if-you-do gate, got {rendered}"
    );
    assert!(
        !rendered.contains("you may discard a card. search your library"),
        "search clause should not become unconditional after the optional discard, got {rendered}"
    );
    assert!(
        rendered.contains("untap another target permanent"),
        "expected the activated ability to remain unchanged, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_unscrupulous_contractor_when_you_do_reflexive_trigger() {
    let text = "When this creature enters, you may sacrifice a creature. When you do, target player draws two cards and loses 2 life.\nPlot {2}{B}";
    let def = CardDefinitionBuilder::new(CardId::new(), "Unscrupulous Contractor")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("Unscrupulous Contractor should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("when you do, target player draws two cards")
            && rendered.contains("target player loses 2 life"),
        "expected reflexive trigger wording to remain explicit, got {rendered}"
    );
    assert!(
        !rendered.contains("if you do, target player draws two cards"),
        "reflexive trigger must not collapse into an immediate if-you-do clause, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ReflexiveTriggerEffect"),
        "expected lowered reflexive trigger runtime effect, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("IfEffect"),
        "reflexive followup should not lower to IfEffect anymore, got {abilities_debug}"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(&ability.kind, ironsmith::ability::AbilityKind::Triggered(triggered)
                if triggered.choices.is_empty()
                    && triggered
                        .effects
                        .iter()
                        .any(|effect| effect.downcast_ref::<ReflexiveTriggerEffect>().is_some()))
        }),
        "outer ETB trigger should keep target selection inside the reflexive trigger"
    );
}

#[test]
fn regression_semantic_mismatch_deny_the_divine_countered_spell_exiled() {
    let rendered = rendered_lines(
        "Counter target creature or enchantment spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.",
        "Deny the Divine",
        &[CardType::Instant],
    );

    assert!(
        rendered.contains("counter target creature or enchantment spell"),
        "expected counter clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("if you do, exile it") || rendered.contains("if it happened, exile it"),
        "expected the follow-up to exile the countered spell itself, got {rendered}"
    );
    assert!(
        !rendered.contains("exile a card in a graveyard"),
        "countered spell should not degrade into a generic graveyard card, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_arcbond_delayed_damage_fanout() {
    let rendered = rendered_lines(
        "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
        "Arcbond",
        &[CardType::Instant],
    );

    assert!(
        rendered.contains("choose target creature"),
        "expected chosen-target setup to remain, got {rendered}"
    );
    assert!(
        rendered.contains("whenever that creature is dealt damage this turn"),
        "expected delayed trigger to stay tied to the chosen creature, got {rendered}"
    );
    assert!(
        rendered.contains("each other creature and each player"),
        "expected damage fanout to cover both each other creature and each player, got {rendered}"
    );
    assert!(
        !rendered.contains("that player"),
        "arcbond fanout should not collapse into a per-player controller clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_heal_next_turns_upkeep() {
    let rendered = rendered_lines(
        "Prevent the next 1 damage that would be dealt to any target this turn.\nDraw a card at the beginning of the next turn's upkeep.",
        "Heal",
        &[CardType::Instant],
    );

    assert!(
        rendered.contains("prevent the next 1 damage that would be dealt to any target this turn"),
        "expected prevention clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("at the beginning of the next turn's upkeep")
            || rendered.contains("at the beginning of the next turns upkeep"),
        "expected the delayed draw to stay on the next turn's upkeep, got {rendered}"
    );
    assert!(
        !rendered.contains("at the beginning of the next end step"),
        "next-turn upkeep trigger should not degrade into next end step, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_spider_ham_animal_may_ham_subtypes() {
    let rendered = rendered_lines(
        "When Spider-Ham enters, create a Food token.\nAnimal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1.",
        "Spider-Ham, Peter Porker",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("jackal"),
        "expected Jackal subtype to remain in the anthem list, got {rendered}"
    );
    assert!(
        rendered.contains("wolf"),
        "expected Wolf subtype to remain in the anthem list, got {rendered}"
    );
    assert!(
        rendered.contains("food token"),
        "expected ETB Food token clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_one_with_the_kami_trigger_disjunction() {
    let rendered = rendered_lines(
        "Flash\nEnchant creature you control\nWhenever enchanted creature or another modified creature you control dies, create X 1/1 colorless Spirit creature tokens, where X is that creature's power.",
        "One with the Kami",
        &[CardType::Enchantment],
    );

    assert!(
        rendered.contains("enchanted creature"),
        "expected enchanted-creature trigger branch to remain, got {rendered}"
    );
    assert!(
        rendered.contains("another modified creature you control"),
        "expected modified-creature trigger branch to remain, got {rendered}"
    );
    assert!(
        rendered.contains("create x 1/1 colorless spirit creature tokens"),
        "expected variable token count to stay explicit, got {rendered}"
    );
    assert!(
        rendered.contains("where x is") && rendered.contains("power"),
        "expected token count to stay tied to the dying creature's power, got {rendered}"
    );
    assert!(
        !rendered.contains("target permanent's power"),
        "dying-creature power should not degrade into an unrelated target reference, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_fear_of_falling_shared_duration() {
    let rendered = rendered_lines(
        "Flying\nWhenever this creature attacks, target creature defending player controls gets -2/-0 and loses flying until your next turn.",
        "Fear of Falling",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("gets -2/-0 until your next turn"),
        "expected the power reduction to keep the shared next-turn duration, got {rendered}"
    );
    assert!(
        rendered.contains("loses flying until your next turn"),
        "expected the flying-loss clause to keep the shared next-turn duration, got {rendered}"
    );
    assert!(
        !rendered.contains("gets -2/-0 until end of turn"),
        "shared duration should not collapse back to end of turn, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_campfire_commanders_from_public_zones() {
    let rendered = rendered_lines(
        "{1}, {T}: You gain 2 life.\n{2}, {T}, Exile this artifact: Put all commanders you own from the command zone and from your graveyard into your hand. Then shuffle your graveyard into your library.",
        "Campfire",
        &[CardType::Artifact],
    );

    assert!(
        rendered.contains("commander"),
        "expected commander selection to remain in the activated ability, got {rendered}"
    );
    assert!(
        rendered.contains("command zone") && rendered.contains("graveyard"),
        "expected both public zones to remain in the commander-return effect, got {rendered}"
    );
    assert!(
        rendered.contains("hand") && rendered.contains("shuffle your graveyard into your library"),
        "expected hand return plus graveyard shuffle to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("reveal it. return it to its owner's hand"),
        "campfire should not collapse to returning the exiled source, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_gixs_caress_reveal_choose_discard_chain() {
    let rendered = rendered_lines(
        "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card.\nCreate a tapped Powerstone token.",
        "Gix's Caress",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("target opponent reveals their hand"),
        "expected the targeted hand-reveal clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("nonland card") && rendered.contains("opponent's hand"),
        "expected card selection to stay tied to the revealed opponent hand, got {rendered}"
    );
    assert!(
        rendered.contains("discards that card")
            && (rendered.contains("that player") || rendered.contains("target opponent")),
        "expected discard to stay tied to the chosen revealed card, got {rendered}"
    );
    assert!(
        rendered.contains("tapped powerstone token"),
        "expected tapped Powerstone token creation to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("in your hand") && !rendered.contains("you discard a card"),
        "gix's caress should not fall back to choosing or discarding from your hand, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_contagious_vorrac_if_not_into_hand_followup() {
    let rendered = rendered_lines(
        "When this creature enters, look at the top four cards of your library. You may reveal a land card from among them and put it into your hand. Put the rest on the bottom of your library in a random order. If you didn't put a card into your hand this way, proliferate.",
        "Contagious Vorrac",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("look at the top four cards of your library"),
        "expected top-of-library look clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("land"),
        "expected land-card selection to remain, got {rendered}"
    );
    assert!(
        rendered.contains("put the rest on the bottom of your library"),
        "expected rest-on-bottom clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("proliferate"),
        "expected fallback proliferate clause to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("if that doesn't happen"),
        "fallback clause should track whether a card was put into hand, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_strongarm_tactics_discarded_creature_followup() {
    let rendered = rendered_lines(
        "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.",
        "Strongarm Tactics",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("each player discards a card"),
        "expected discard clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("didn't discard a creature card this way")
            || rendered.contains("did not discard a creature card this way"),
        "expected discarded-creature qualifier to remain explicit, got {rendered}"
    );
    assert!(
        rendered.contains("loses 4 life"),
        "expected life-loss follow-up to remain, got {rendered}"
    );
    assert!(
        !rendered.contains("if that doesn't happen"),
        "discard qualifier should not collapse into a generic result predicate, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_westvale_abbey_transform_then_untap() {
    let rendered = rendered_lines(
        "{T}: Add {C}.\n{5}, {T}, Pay 1 life: Create a 1/1 white and black Human Cleric creature token.\n{5}, {T}, Sacrifice five creatures: Transform this land, then untap it.",
        "Westvale Abbey // Ormendahl, Profane Prince",
        &[CardType::Land],
    );

    assert!(
        rendered.contains("sacrifice five creatures"),
        "expected sacrifice activation cost to remain, got {rendered}"
    );
    assert!(
        rendered.contains("transform this land")
            || rendered.contains("transform this permanent")
            || rendered.contains("transform it"),
        "expected transform self-reference to remain explicit, got {rendered}"
    );
    assert!(
        rendered.contains("untap it"),
        "expected untap follow-up to remain attached to the transformed land, got {rendered}"
    );
}
