//! Source regression contracts for the Sol procedures pass. These tests were
//! authored without executing Cargo, the compiler, or any status DB refresh.
use super::compile_to_runtime_definition;

fn debug_card(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false).unwrap_or_else(|error| panic!("{name}: {error:?}"));
    format!("{:?} {:?}", definition.spell_effect, definition.abilities)
}

const CASES: &[(&str, &str)] = &[
    ("Ice Cauldron", "Mana cost: {4}\nType: Artifact\n{X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.\n{T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact."),
    ("Wheel of Sun and Moon", "Mana cost: {G/W}{G/W}\nType: Enchantment — Aura\nEnchant player\nIf a card would be put into enchanted player's graveyard from anywhere, instead that card is revealed and put on the bottom of that player's library."),
    ("Magar of the Magic Strings", "Mana cost: {1}{B}{R}\nType: Legendary Creature — Minotaur Performer\nPower/Toughness: 3/3\n{1}{B}{R}: Note the name of target instant or sorcery card in your graveyard and put it onto the battlefield face down. It's a 3/3 creature with \"Whenever this creature deals combat damage to a player, you may create a copy of the card with the noted name. You may cast the copy without paying its mana cost\" and \"If this creature would leave the battlefield, exile it instead of putting it anywhere else.\""),
    ("Frankenstein's Monster", "Mana cost: {X}{B}{B}\nType: Creature — Zombie\nPower/Toughness: 0/1\nAs this creature enters, exile X creature cards from your graveyard. If you can't, put this creature into its owner's graveyard instead of onto the battlefield. For each creature card exiled this way, this creature enters with a +2/+0, +1/+1, or +0/+2 counter on it."),
    ("Guff Rewrites History", "Mana cost: {2}{R}\nType: Instant\nFor each player, choose target nonenchantment, nonland permanent that player controls. Those permanents' owners shuffle them into their libraries. Each player who controlled one of those permanents exiles cards from the top of their library until they exile a nonland card, then puts the rest on the bottom of their library in a random order. Each player may cast the nonland card they exiled without paying its mana cost."),
    ("Aminatou's Augury", "Mana cost: {6}{U}{U}\nType: Sorcery\nExile the top eight cards of your library. You may put a land card from among them onto the battlefield. Until end of turn, for each nonland card type, you may cast a spell of that type from among the exiled cards without paying its mana cost."),
    ("Brilliant Ultimatum", "Mana cost: {W}{W}{U}{U}{U}{B}{B}\nType: Sorcery\nExile the top five cards of your library. An opponent separates those cards into two piles. You may play lands and cast spells from one of those piles. If you cast a spell this way, you cast it without paying its mana cost."),
    ("Call to the Void", "Mana cost: {4}{B}\nType: Sorcery\nEach player secretly chooses a creature they control and a creature they don't control. Then those choices are revealed. Destroy each creature chosen this way."),
    ("Caller of the Untamed", "Mana cost: {3}{G}\nType: Creature — Elf Shaman\nPower/Toughness: 2/4\nBefore you shuffle your deck to start the game, you may reveal this card from your deck and exile a creature card you drafted that isn't in your deck.\n{X}, {T}: Create a token that's a copy of a card you exiled with cards named Caller of the Untamed. X is the mana value of that card."),
    ("Choice of Damnations", "Mana cost: {5}{B}\nType: Sorcery — Arcane\nTarget opponent chooses a number. You may have that player lose that much life. If you don't, that player sacrifices all but that many permanents of their choice."),
    ("Cruel Entertainment", "Mana cost: {6}{B}\nType: Sorcery\nChoose target player and another target player. The first player controls the second player during the second player's next turn, and the second player controls the first player during the first player's next turn."),
    ("Embiggen", "Mana cost: {G}\nType: Instant\nUntil end of turn, target non-Brushwagg creature gets +1/+1 for each supertype, card type, and subtype it has."),
    ("Face Yourself", "Mana cost: {5}{R}{R}\nType: Sorcery\nFor each creature target player controls, create a token that's a copy of that creature, except it has haste and \"At the beginning of the end step, if you don't control a planeswalker, sacrifice this creature.\""),
    ("Finale of Promise", "Mana cost: {X}{R}{R}\nType: Sorcery\nYou may cast up to one target instant card and/or up to one target sorcery card from your graveyard each with mana value X or less without paying their mana costs. If a spell cast this way would be put into your graveyard, exile it instead. If X is 10 or more, copy each of those spells twice. You may choose new targets for the copies."),
    ("Garth One-Eye", "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 5/5\n{T}: Choose a card name that hasn't been chosen from among Disenchant, Braingeyser, Terror, Shivan Dragon, Regrowth, and Black Lotus. Create a copy of the card with the chosen name. You may cast the copy. (You still pay its costs.)"),
    ("Hit the Mother Lode", "Mana cost: {4}{R}{R}{R}\nType: Sorcery\nDiscover 10. If the discovered card's mana value is less than 10, create a number of tapped Treasure tokens equal to the difference. (To discover 10, exile cards from the top of your library until you exile a nonland card with mana value 10 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)"),
    ("Kang the Conqueror", "Mana cost: {2}{U}{U}\nType: Legendary Creature — Human Villain\nPower/Toughness: 4/5\nFlying\nPower-up — {5}{U}{U}{U}: Put a +1/+1 counter on Kang. Take an extra turn after this one. During that turn, power-up abilities can't be activated. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)"),
    ("Lim-Dûl's Vault", "Mana cost: {U}{B}\nType: Instant\nLook at the top five cards of your library. As many times as you choose, you may pay 1 life, put those cards on the bottom of your library in any order, then look at the top five cards of your library. Then shuffle and put the last cards you looked at this way on top in any order."),
    ("Multiple Choice", "Mana cost: {X}{U}\nType: Sorcery\nIf X is 1, scry 1, then draw a card.\nIf X is 2, you may choose a player. They return a creature they control to its owner's hand.\nIf X is 3, create a 4/4 blue and red Elemental creature token.\nIf X is 4 or more, do all of the above."),
    ("Myra the Magnificent", "Mana cost: {2}{U}{R}\nType: Legendary Creature — Human Performer\nPower/Toughness: 2/4\nWhenever you cast an instant or sorcery spell from your hand, open an Attraction.\n{X}, {T}: Exile target instant or sorcery card with mana value X from your graveyard and choose an Attraction you control that doesn't have a midway counter on it. Put a midway counter on it. For as long as that Attraction is on the battlefield, whenever you visit it, copy the exiled card. You may cast the copy without paying its mana cost."),
    ("Rowan's Grim Search", "Mana cost: {2}{B}\nType: Instant\nBargain (You may sacrifice an artifact, enchantment, or token as you cast this spell.)\nIf this spell was bargained, look at the top four cards of your library, then put up to two of them back on top of your library in any order and the rest into your graveyard.\nYou draw two cards and you lose 2 life."),
    ("Starving Revenant", "Mana cost: {2}{B}{B}\nType: Creature — Spirit Horror\nPower/Toughness: 4/4\nWhen this creature enters, surveil 2. Then for each card you put on top of your library, you draw a card and you lose 3 life.\nDescend 8 — Whenever you draw a card, if there are eight or more permanent cards in your graveyard, target opponent loses 1 life and you gain 1 life."),
    ("Talent of the Telepath", "Mana cost: {2}{U}{U}\nType: Sorcery\nTarget opponent reveals the top seven cards of their library. You may cast an instant or sorcery spell from among them without paying its mana cost. Then that player puts the rest into their graveyard.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast up to two instant and/or sorcery spells from among the revealed cards instead of one."),
    ("Tariff", "Mana cost: {1}{W}\nType: Sorcery\nEach player sacrifices the creature they control with the greatest mana value unless they pay that creature's mana cost. If two or more creatures a player controls are tied for greatest, that player chooses one."),
    ("The Hunger Tide Rises", "Mana cost: {2}{G}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI, II, III — Create a 1/1 black and green Insect creature token.\nIV — Sacrifice any number of creatures. Search your library and/or graveyard for a creature card with mana value less than or equal to the number of creatures sacrificed this way and put it onto the battlefield. If you search your library this way, shuffle."),
    ("Three Dog, Galaxy News DJ", "Mana cost: {1}{R}{W}\nType: Legendary Creature — Human Bard\nPower/Toughness: 1/5\nWhenever you attack, you may pay {2} and sacrifice an Aura attached to Three Dog. When you sacrifice an Aura this way, for each other attacking creature you control, create a token that's a copy of that Aura attached to that creature."),
    ("Ultima, Origin of Oblivion", "Mana cost: {5}\nType: Legendary Creature — God\nPower/Toughness: 4/4\nFlying\nWhenever Ultima attacks, put a blight counter on target land. For as long as that land has a blight counter on it, it loses all land types and abilities and has \"{T}: Add {C}.\"\nWhenever you tap a land for {C}, add an additional {C}."),
    ("Uvilda, Dean of Perfection", "Mana cost: {2}{U}\nType: Legendary Creature — Djinn Wizard\nPower/Toughness: 2/2\n{T}: You may exile an instant or sorcery card from your hand and put three refine counters on it. It gains \"At the beginning of your upkeep, if this card is exiled, remove a refine counter from it\" and \"When the last refine counter is removed from this card, if it's exiled, you may cast it. It costs {4} less to cast this way.\""),
    ("Nassari, Dean of Expression", "Mana cost: {3}{R}{R}\nType: Legendary Creature — Efreet Shaman\nPower/Toughness: 4/4\nAt the beginning of your upkeep, exile the top card of each opponent's library. Until end of turn, you may cast spells from among those exiled cards, and you may spend mana as though it were mana of any color to cast those spells.\nWhenever you cast a spell from exile, put a +1/+1 counter on Nassari."),
    ("Vault 112: Sadistic Simulation", "Mana cost: {2}{U}{R}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI, II — Tap up to one target creature and put a stun counter on it. You get {E}{E} (two energy counters).\nIII — Pay any amount of {E}. If you paid one or more {E} this way, shuffle your library, then exile that many cards from the top. You may play one of those cards without paying its mana cost."),
    ("Wheel of Potential", "Mana cost: {2}{R}\nType: Sorcery\nYou get {E}{E}{E} (three energy counters), then you may pay any amount of {E}.\nEach player may exile their hand and draw a number of cards equal to the amount of {E} paid this way. If seven or more {E} was paid this way, you may play cards you own exiled this way until the end of your next turn."),
 ];

#[test]
fn all_source_ready_procedures_compile_strictly() {
    for (name, text) in CASES { debug_card(name, text); }
}

fn case(name: &str) -> &'static str {
    CASES.iter().find(|(candidate, _)| *candidate == name).unwrap().1
}

#[test]
fn exiled_binary_piles_choose_an_opponent_then_offer_repeated_ordered_free_plays() {
    let debug = debug_card("Brilliant Ultimatum", case("Brilliant Ultimatum"));
    assert!(debug.contains("ChoosePlayerEffect"), "the opponent is chosen during resolution: {debug}");
    assert!(debug.contains("RepeatProcessEffect"), "the controller chooses the next play each round: {debug}");
    assert!(debug.contains("allow_land: true") && debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(debug.contains("SameObjectId"), "pile membership cannot follow a blinked card: {debug}");
}

#[test]
fn named_printed_copies_keep_the_finite_menu_history_and_normal_casting_price() {
    let debug = debug_card("Garth One-Eye", case("Garth One-Eye"));
    assert!(debug.contains("exclude_source_history: true"), "{debug}");
    assert!(debug.to_lowercase().contains("black lotus") && debug.to_lowercase().contains("shivan dragon"), "{debug}");
    assert!(debug.contains("copy_named_definition: true") && debug.contains("as_copy: true"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: false"), "copies retain their printed mana cost: {debug}");
}

#[test]
fn every_nonland_card_type_has_a_separate_single_cast_permission() {
    let debug = debug_card("Aminatou's Augury", case("Aminatou's Augury"));
    assert_eq!(debug.matches("max_plays: Some(1)").count(), 14, "each grant owns its usage budget: {debug}");
    for card_type in ["Creature", "Artifact", "Enchantment", "Planeswalker", "Instant", "Sorcery", "Battle", "Kindred"] {
        assert!(debug.contains(&format!("card_types: [{card_type}]")), "{card_type}: {debug}");
    }
    assert!(debug.contains("UntilEndOfTurn"), "{debug}");
}

#[test]
fn counter_linked_land_changes_keep_the_activated_mana_rule_and_duration() {
    let debug = debug_card("Ultima, Origin of Oblivion", case("Ultima, Origin of Oblivion"));
    assert!(debug.contains("ForAsLongAs"), "the land's counter owns the duration: {debug}");
    assert!(debug.to_lowercase().contains("blight"), "{debug}");
    assert!(debug.contains("RemoveAllSubtypesOfFamily") || debug.contains("RemoveSubtypes"), "{debug}");
    assert!(debug.contains("AddMana"), "the land receives its mana ability: {debug}");
}

#[test]
fn finite_card_name_menus_exclude_each_previous_choice_and_exhaust_cleanly() {
    use ironsmith::effects::{EffectExecutor, ChooseCardNameEffect};
    use ironsmith::target::PlayerFilter;
    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.new_object_id();
    let mut choice = ChooseCardNameEffect::new(PlayerFilter::You, None, "named_choice");
    choice.options = vec!["Forest".into(), "Island".into()];
    choice.exclude_source_history = true;
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for expected in ["Forest", "Island"] {
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        choice.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.get_tagged("named_choice").unwrap().name, expected);
    }
    let before = game.chosen_named_option(source).unwrap().to_string();
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    choice.execute(&mut game, &mut ctx).unwrap();
    assert!(ctx.get_tagged("named_choice").is_none());
    assert_eq!(game.chosen_named_option(source), Some(before.as_str()));
}

#[test]
fn per_player_targets_keep_announcement_bindings_and_phase_consults_before_casts() {
    let debug = debug_card("Guff Rewrites History", case("Guff Rewrites History"));
    assert!(debug.contains("target_controller_fixed_on_announcement: true"), "{debug}");
    assert!(debug.contains("CountPlayers(Any)"), "one mandatory target per player: {debug}");
    assert!(debug.contains("accumulated_match_tag: Some"), "{debug}");
    assert!(debug.find("ConsultTopOfLibraryEffect").unwrap() < debug.find("CastTaggedEffect").unwrap(), "{debug}");
}

#[test]
fn failed_entry_exile_redirects_and_each_actual_card_has_an_independent_counter_choice() {
    let debug = debug_card("Frankenstein's Monster", case("Frankenstein's Monster"));
    assert!(debug.contains("RedirectEnteringSourceEffect"), "failure changes the entry destination: {debug}");
    assert!(debug.contains("TaggedSnapshotCount"), "failure compares the actual exile arrivals to X: {debug}");
    assert!(debug.contains("ForEachTagged"), "every exiled card owns its own choice: {debug}");
    for counter in ["PlusTwoPlusZero", "PlusOnePlusOne", "PlusZeroPlusTwo"] {
        assert!(debug.contains(counter), "{debug}");
    }
}

#[test]
fn face_down_arrival_retains_the_original_name_for_its_own_granted_trigger() {
    let debug = debug_card("Magar of the Magic Strings", case("Magar of the Magic Strings"));
    assert!(debug.contains("retained_reference_tags: [TagKey(\"__noted_card_name__\")]"), "{debug}");
    assert!(debug.contains("copy_named_definition: true"), "the trigger copies the original printed card: {debug}");
    assert!(debug.contains("enters_face_down: true"), "{debug}");
}

#[test]
fn arbitrary_origin_graveyard_replacement_reveals_then_bottoms_only_physical_cards() {
    let debug = debug_card("Wheel of Sun and Moon", case("Wheel of Sun and Moon"));
    assert!(debug.contains("physical_card: true"), "tokens are not cards: {debug}");
    assert!(debug.contains("from: None"), "the card may come from any zone: {debug}");
    assert!(debug.contains("Graveyard") && debug.contains("Library"), "{debug}");
    assert!(debug.to_lowercase().contains("reveal"), "{debug}");
}

#[test]
fn enchanted_players_cards_are_redirected_from_each_origin_but_other_players_are_not() {
    use ironsmith::cards::CardDefinition;
    use ironsmith::card::CardBuilder;
    use ironsmith::effect::Effect;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition: CardDefinition = compile_to_runtime_definition("Wheel of Sun and Moon", case("Wheel of Sun and Moon"), false).unwrap();
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.object_mut(aura).unwrap().attached_to = Some(ironsmith::object::AttachmentTarget::Player(alice));
    for (index, origin) in [Zone::Hand, Zone::Library, Zone::Stack, Zone::Exile, Zone::Battlefield].into_iter().enumerate() {
        let card = CardBuilder::new(ironsmith::ids::CardId::new(), format!("Origin witness {index}"))
            .card_types(vec![CardType::Artifact]).build();
        let object = game.create_object_from_card(&card, alice, origin);
        let mut ctx = ironsmith::effects::EffectContext::new_default(aura, alice);
        ironsmith::effects::execute_effect(&mut game, &Effect::move_to_zone(ChooseSpec::SpecificObject(object), Zone::Graveyard, false), &mut ctx).unwrap();
        assert!(!game.player(alice).unwrap().graveyard.iter().any(|id| game.object(*id).is_some_and(|card| card.name == format!("Origin witness {index}"))));
        assert!(game.player(alice).unwrap().library.iter().any(|id| game.object(*id).is_some_and(|card| card.name == format!("Origin witness {index}"))));
    }
    let card = CardBuilder::new(ironsmith::ids::CardId::new(), "Other player's witness")
        .card_types(vec![CardType::Artifact]).build();
    let object = game.create_object_from_card(&card, bob, Zone::Hand);
    let mut ctx = ironsmith::effects::EffectContext::new_default(aura, alice);
    ironsmith::effects::execute_effect(&mut game, &Effect::move_to_zone(ChooseSpec::SpecificObject(object), Zone::Graveyard, false), &mut ctx).unwrap();
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
}

#[test]
fn exact_activation_payment_preserves_mixed_mana_types_amounts_and_overwrites_the_prior_note() {
    use ironsmith::effects::{EffectExecutor, NoteActivationManaPaymentEffect, AddManaOfNotedTypeEffect};
    use ironsmith::target::PlayerFilter;
    use ironsmith::mana::ManaSymbol;
    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source_card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Noted mana witness")
        .card_types(vec![ironsmith::types::CardType::Artifact]).build();
    let source = game.create_object_from_card(&source_card, alice, ironsmith::Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.mana.activation_payment.add(ManaSymbol::White, 2);
    ctx.mana.activation_payment.add(ManaSymbol::Blue, 1);
    ctx.mana.activation_payment.add(ManaSymbol::Colorless, 3);
    NoteActivationManaPaymentEffect::default().execute(&mut game, &mut ctx).unwrap();
    let mut add = AddManaOfNotedTypeEffect::new(0, PlayerFilter::You);
    add.use_noted_amount = true;
    add.execute(&mut game, &mut ctx).unwrap();
    for (symbol, amount) in [(ManaSymbol::White, 2), (ManaSymbol::Blue, 1), (ManaSymbol::Colorless, 3)] {
        assert_eq!(game.player(alice).unwrap().mana_pool.amount(symbol), amount);
    }
    ctx.mana.activation_payment = ironsmith::player::ManaPool::default();
    ctx.mana.activation_payment.add(ManaSymbol::Green, 1);
    NoteActivationManaPaymentEffect::default().execute(&mut game, &mut ctx).unwrap();
    add.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.amount(ManaSymbol::White), 2);
    assert_eq!(game.player(alice).unwrap().mana_pool.amount(ManaSymbol::Green), 1);
}

#[test]
fn noted_mana_payment_and_last_exiled_cast_restriction_are_structural() {
    let debug = debug_card("Ice Cauldron", case("Ice Cauldron"));
    assert!(debug.contains("NoteActivationManaPaymentEffect"), "{debug}");
    assert!(debug.contains("use_noted_amount: true"), "{debug}");
    assert!(debug.contains("__last_source_exiled_at_production__"), "the spending subject is bound when mana is made: {debug}");
    assert!(debug.contains("restrict_to_matching_spell: true"), "{debug}");
}
