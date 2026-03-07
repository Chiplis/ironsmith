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
        rendered.contains("power"),
        "expected token count to stay tied to the dying creature's power, got {rendered}"
    );
    assert!(
        !rendered.contains("target permanent's power"),
        "dying-creature power should not degrade into an unrelated target reference, got {rendered}"
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
