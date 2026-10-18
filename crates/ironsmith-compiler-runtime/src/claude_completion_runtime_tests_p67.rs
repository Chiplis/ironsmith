//! Claude package 67 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p67_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

/// The rendered card text, lowercased for surface checks that must not
/// depend on sentence-initial capitalization inside ability bodies.
fn p67_text(definition: &ironsmith::cards::CardDefinition) -> String {
    definition.canonical_text.to_ascii_lowercase()
}

fn p67_assert_no_internal_markers(definition: &ironsmith::cards::CardDefinition) {
    let text = p67_text(definition);
    assert!(!text.contains("tagged"), "internal tag leaked: {}", definition.canonical_text);
    assert!(!text.contains("divvy_"), "internal tag leaked: {}", definition.canonical_text);
    assert!(
        !text.contains("unsupported effect"),
        "unsupported effect leaked: {}",
        definition.canonical_text
    );
}

fn p67_find_effects<T: Clone + 'static>(effect: &Effect, found: &mut Vec<T>) {
    if let Some(hit) = effect.downcast_ref::<T>() {
        found.push(hit.clone());
    }
    effect.visit_child_effects(&mut |child| p67_find_effects(child, found));
}

fn p67_triggered_effects<T: Clone + 'static>(
    definition: &ironsmith::cards::CardDefinition,
) -> Vec<T> {
    let mut found = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                p67_find_effects(effect, &mut found);
            }
        }
    }
    found
}

const FACT_OR_FICTION: &str = "Mana cost: {3}{U}\nType: Instant\nReveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.";

#[test]
fn p67_fact_or_fiction_pile_split_renders_without_internal_tags() {
    let definition = p67_compile("Fact or Fiction", FACT_OR_FICTION);
    // The pile-grouping turn-history record stays in the program...
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("GroupIntoPiles"), "{debug}");
    // ...but it no longer hides the pile bundle from the renderer.
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(
        text.contains("an opponent separates those cards into two piles"),
        "{}",
        definition.canonical_text
    );
    assert!(
        text.contains("put one pile into your hand and the other into your graveyard"),
        "{}",
        definition.canonical_text
    );
}

const BONEYARD_PARLEY: &str = "Mana cost: {5}{B}{B}\nType: Sorcery\nExile up to five target creature cards from graveyards. An opponent separates those cards into two piles. Put all cards from the pile of your choice onto the battlefield under your control and the rest into their owners' graveyards.";

#[test]
fn p67_boneyard_parley_pile_split_renders_without_internal_tags() {
    let definition = p67_compile("Boneyard Parley", BONEYARD_PARLEY);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(text.contains("two piles"), "{}", definition.canonical_text);
    assert!(text.contains("pile of your choice"), "{}", definition.canonical_text);
}

const JACE_ARCHITECT: &str = "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 4\n+1: Until your next turn, whenever a creature an opponent controls attacks, it gets -1/-0 until end of turn.\n−2: Reveal the top three cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other on the bottom of your library in any order.\n−8: For each player, search that player's library for a nonland card and exile it, then that player shuffles. You may cast those cards without paying their mana costs.";

#[test]
fn p67_jace_minus_two_renders_other_pile_on_bottom() {
    let definition = p67_compile("Jace, Architect of Thought", JACE_ARCHITECT);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(
        text.contains(
            "put one pile into your hand and the other on the bottom of your library in any order"
        ),
        "{}",
        definition.canonical_text
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PutTaggedRemainderOnLibraryBottomEffect"), "{debug}");
}

const CHAOS_DRAGON: &str = "Mana cost: {1}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 4/4\nFlying, haste\nThis creature attacks each combat if able.\nAt the beginning of combat on your turn, each player rolls a d20. If one or more opponents had the highest result, this creature can't attack those players or planeswalkers they control this combat.";

#[test]
fn p67_chaos_dragon_highest_roll_restriction_renders() {
    let definition = p67_compile("Chaos Dragon", CHAOS_DRAGON);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(text.contains("each player rolls a d20"), "{}", definition.canonical_text);
    assert!(
        text.contains(
            "if one or more opponents had the highest result, this creature can't attack those players or planeswalkers they control this combat"
        ),
        "{}",
        definition.canonical_text
    );
    let rolls =
        p67_triggered_effects::<ironsmith::effects::RollDieKeepHighestPlayersEffect>(&definition);
    assert_eq!(rolls.len(), 1, "{definition:#?}");
    assert_eq!(rolls[0].sides, 20);
    assert_eq!(rolls[0].keep, ironsmith::target::PlayerFilter::Opponent);
}

const COURT_OF_CUNNING: &str = "Mana cost: {1}{U}{U}\nType: Enchantment\nWhen this enchantment enters, you become the monarch.\nAt the beginning of your upkeep, any number of target players each mill two cards. If you're the monarch, each of those players mills ten cards instead.";

#[test]
fn p67_court_of_cunning_monarch_mill_renders_instead() {
    let definition = p67_compile("Court of Cunning", COURT_OF_CUNNING);
    let text = p67_text(&definition);
    assert!(text.contains("each mill two cards"), "{}", definition.canonical_text);
    assert!(
        text.contains("if you're the monarch, each of those players mills ten cards instead"),
        "{}",
        definition.canonical_text
    );
    assert!(!text.contains("otherwise"), "{}", definition.canonical_text);
}

const ERESTOR: &str = "Mana cost: {1}{G}{U}\nType: Legendary Creature — Elf Noble\nPower/Toughness: 2/4\nWhenever players finish voting, each opponent who voted for a choice you voted for creates a Treasure token. You scry X, where X is the number of opponents who voted for a choice you didn't vote for. Draw a card.";

#[test]
fn p67_erestor_treasure_goes_to_agreeing_opponents() {
    use ironsmith::target::PlayerFilter;
    let definition = p67_compile("Erestor of the Council", ERESTOR);
    let tokens = p67_triggered_effects::<ironsmith::effects::CreateTokenEffect>(&definition);
    assert_eq!(tokens.len(), 1, "{definition:#?}");
    // Each agreeing opponent creates their own Treasure, not you.
    assert_ne!(tokens[0].controller, PlayerFilter::You, "{:?}", tokens[0].controller);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(
        text.contains("each opponent who voted for a choice you voted for creates a treasure token"),
        "{}",
        definition.canonical_text
    );
    assert!(
        text.contains("the number of opponents who voted for a choice you didn't vote for"),
        "{}",
        definition.canonical_text
    );
}

const FORGERS_FOUNDRY: &str = "Mana cost: {2}{U}\nType: Artifact\n{T}: Add {U}. When you spend this mana to cast an instant or sorcery spell with mana value 3 or less, you may exile that spell instead of putting it into its owner's graveyard as it resolves.\n{3}{U}{U}, {T}: You may cast any number of spells from among cards exiled with this artifact without paying their mana costs. Activate only as a sorcery.";

#[test]
fn p67_forgers_foundry_mana_rider_renders_and_links_exile() {
    let definition = p67_compile("Forger's Foundry", FORGERS_FOUNDRY);
    let text = p67_text(&definition);
    assert!(
        text.contains(
            "you may exile that spell instead of putting it into its owner's graveyard as it resolves"
        ),
        "{}",
        definition.canonical_text
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("link_exiled_to_source: true"), "{debug}");
}

const FULL_BORE: &str = "Mana cost: {R}\nType: Instant\nTarget creature you control gets +3/+2 until end of turn. If that creature was cast for its warp cost, it also gains trample and haste until end of turn.";

#[test]
fn p67_full_bore_warp_condition_is_a_named_alternative_cast_query() {
    let definition = p67_compile("Full Bore", FULL_BORE);
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("TargetPaidLabel"), "{debug}");
    assert!(debug.contains("AlternativeCast"), "{debug}");
    assert!(debug.contains("\"Warp\""), "{debug}");
    assert!(!debug.contains("CustomUnsupported"), "{debug}");
}

const GOLLUM: &str = "Mana cost: {1}{B}\nType: Legendary Creature — Halfling Horror\nPower/Toughness: 2/1\nWhenever Gollum attacks, look at the top two cards of your library, put them back in any order, then choose land or nonland. An opponent guesses whether the top card of your library is the chosen kind. Reveal that card. If they guessed right, remove Gollum from combat. Otherwise, you draw a card and Gollum can't be blocked this turn.";

#[test]
fn p67_gollum_guess_program_renders_both_outcomes() {
    let definition = p67_compile("Gollum, Scheming Guide", GOLLUM);
    let text = p67_text(&definition);
    assert!(text.contains("choose land or nonland"), "{}", definition.canonical_text);
    assert!(
        text.contains("an opponent guesses whether the top card of your library is the chosen kind"),
        "{}",
        definition.canonical_text
    );
    assert!(text.contains("if they guessed right,"), "{}", definition.canonical_text);
    assert!(text.contains("otherwise,"), "{}", definition.canonical_text);
    assert!(text.contains("can't be blocked"), "{}", definition.canonical_text);
}

const GRELL_PHILOSOPHER: &str = "Mana cost: {2}{U}\nType: Creature — Horror Wizard\nPower/Toughness: 1/4\nAberrant Tinkering — When this creature enters and at the beginning of your upkeep, each Horror you control gains all activated abilities of target artifact an opponent controls until end of turn. You may spend blue mana as though it were mana of any color to activate those abilities.";

#[test]
fn p67_grell_philosopher_renders_blue_as_any_color_rider() {
    let definition = p67_compile("Grell Philosopher", GRELL_PHILOSOPHER);
    let text = p67_text(&definition);
    assert!(
        text.contains(
            "you may spend blue mana as though it were mana of any color to activate those abilities"
        ),
        "{}",
        definition.canonical_text
    );
}

const HEIRLOOM_EPIC: &str = "Mana cost: {1}\nType: Artifact — Book\n{4}, {T}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana. Activate only as a sorcery.";

#[test]
fn p67_heirloom_epic_renders_creature_tap_payment_rider() {
    let definition = p67_compile("Heirloom Epic", HEIRLOOM_EPIC);
    let text = p67_text(&definition);
    assert!(
        text.contains(
            "for each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana"
        ),
        "{}",
        definition.canonical_text
    );
    assert!(text.contains("activate only as a sorcery"), "{}", definition.canonical_text);
}

const IGNITE_THE_FUTURE: &str = "Mana cost: {3}{R}\nType: Sorcery\nExile the top three cards of your library. Until the end of your next turn, you may play those cards. If this spell was cast from a graveyard, you may play cards this way without paying their mana costs.\nFlashback {7}{R}";

#[test]
fn p67_ignite_the_future_graveyard_cast_grants_free_play() {
    let definition = p67_compile("Ignite the Future", IGNITE_THE_FUTURE);
    let text = p67_text(&definition);
    assert!(text.contains("without paying their mana costs"), "{}", definition.canonical_text);
    assert!(!text.contains("pay free"), "{}", definition.canonical_text);
}

const KARPLUSAN_MINOTAUR: &str = "Mana cost: {2}{R}{R}\nType: Creature — Minotaur Warrior\nPower/Toughness: 3/3\nCumulative upkeep—Flip a coin.\nWhenever you win a coin flip, this creature deals 1 damage to any target.\nWhenever you lose a coin flip, this creature deals 1 damage to any target of an opponent's choice.";

#[test]
fn p67_karplusan_minotaur_cumulative_upkeep_flips_a_coin() {
    let definition = p67_compile("Karplusan Minotaur", KARPLUSAN_MINOTAUR);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(text.contains("cumulative upkeep—flip a coin"), "{}", definition.canonical_text);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("CumulativeUpkeepEffect"), "{debug}");
    assert!(debug.contains("FlipCoinEffect"), "the upkeep payment flips a coin: {debug}");
}

const CALL_TO_THE_VOID: &str = "Mana cost: {4}{B}\nType: Sorcery\nEach player secretly chooses a creature they control and a creature they don't control. Then those choices are revealed. Destroy each creature chosen this way.";

#[test]
fn p67_call_to_the_void_secret_choices_render_and_destroy_both_sets() {
    let definition = p67_compile("Call to the Void", CALL_TO_THE_VOID);
    p67_assert_no_internal_markers(&definition);
    let text = p67_text(&definition);
    assert!(
        text.contains(
            "each player secretly chooses a creature they control and a creature they don't control"
        ),
        "{}",
        definition.canonical_text
    );
    assert!(text.contains("then those choices are revealed"), "{}", definition.canonical_text);
    assert!(text.contains("destroy each creature chosen this way"), "{}", definition.canonical_text);
}

const CELESTIAL_REUNION: &str = "Mana cost: {X}{G}\nType: Sorcery\nAs an additional cost to cast this spell, you may choose a creature type and behold two creatures of that type.\nSearch your library for a creature card with mana value X or less, reveal it, put it into your hand, then shuffle. If this spell's additional cost was paid and the revealed card is the chosen type, put that card onto the battlefield instead of putting it into your hand.";

#[test]
fn p67_celestial_reunion_paid_behold_replaces_hand_destination() {
    let definition = p67_compile("Celestial Reunion", CELESTIAL_REUNION);
    let debug = format!("{:?}", definition.spell_effect);
    // One zone change: the search's Library -> Hand move is replaced, rather
    // than moving the card from the hand afterward.
    assert!(debug.contains("LocalRewriteEffect"), "{debug}");
    assert!(debug.contains("chosen_creature_type: true"), "{debug}");
    assert!(debug.contains("ThisSpellPaidLabel"), "{debug}");
    let text = p67_text(&definition);
    assert!(
        text.contains("onto the battlefield instead of putting it into your hand"),
        "{}",
        definition.canonical_text
    );
}

const MOGIS_MARAUDER: &str = "Mana cost: {2}{B}\nType: Creature — Human Berserker\nPower/Toughness: 2/2\nWhen this creature enters, up to X target creatures each gain intimidate and haste until end of turn, where X is your devotion to black.";

const AGILITY_BOBBLEHEAD: &str = "Mana cost: {3}\nType: Artifact — Bobblehead\n{T}: Add one mana of any color.\n{3}, {T}: Up to X target creatures you control each gain haste until end of turn and can't be blocked this turn except by creatures with haste, where X is the number of Bobbleheads you control as you activate this ability.";

#[test]
fn p67_where_x_binds_plain_x_target_count_of_grants() {
    // The dynamic target-count prefix reads "up to X target" as XTimes(1);
    // the where-X clause must still define that X (never left unbound).
    let mogis = p67_compile("Mogis's Marauder", MOGIS_MARAUDER);
    let debug = format!("{:?}", mogis.abilities);
    assert!(debug.contains("Devotion"), "{debug}");
    assert!(!debug.contains("XTimes"), "unbound X target count: {debug}");
    assert!(
        p67_text(&mogis).contains("where x is your devotion to black"),
        "{}",
        mogis.canonical_text
    );

    let agility = p67_compile("Agility Bobblehead", AGILITY_BOBBLEHEAD);
    let debug = format!("{:?}", agility.abilities);
    assert!(debug.contains("AsYouActivateThisAbility"), "{debug}");
    assert!(!debug.contains("XTimes"), "unbound X target count: {debug}");
    assert!(
        p67_text(&agility).contains(
            "where x is the number of bobbleheads you control as you activate this ability"
        ),
        "{}",
        agility.canonical_text
    );
}

const GANDALF: &str = "Mana cost: {1}{U}{R}{W}\nType: Legendary Creature — Avatar Wizard\nPower/Toughness: 3/4\nWhenever you cast an instant or sorcery spell from your hand during an opponent's turn, exile that card with three time counters on it instead of putting it into your graveyard as it resolves. Then if the exiled card doesn't have suspend, it gains suspend.";

#[test]
fn p67_gandalf_replaces_the_resolving_spells_destination() {
    let definition = p67_compile("Gandalf of the Secret Fire", GANDALF);
    let debug = format!("{:?}", definition.abilities);
    // A destination replacement on the resolving spell, never an immediate
    // exile of the spell on the stack.
    assert!(debug.contains("RegisterZoneReplacementEffect"), "{debug}");
    assert!(debug.contains("GainSuspendIfMissing"), "{debug}");
    assert!(!debug.contains("ExileEffect"), "{debug}");
    let text = p67_text(&definition);
    assert!(
        text.contains("instead of putting it into your graveyard as it resolves"),
        "{}",
        definition.canonical_text
    );
}

const CAPTAIN_REX: &str = "Mana cost: {1}{R}{W}\nType: Legendary Creature — Human Pilot Employee\nPower/Toughness: 2/2\nAt the beginning of combat on your turn, choose target nonland permanent you control. Until end of turn, it becomes a Vehicle artifact with base power and toughness each equal to its mana value, and it gains crew 2 and \"Crash Land — Whenever this Vehicle deals damage, roll a six-sided die. If the result is equal to this Vehicle's mana value, sacrifice this Vehicle, then it deals that much damage to any target.\"";

#[test]
fn p67_captain_rex_grants_crew_and_the_quoted_crash_land_trigger() {
    let definition = p67_compile("Captain Rex Nebula", CAPTAIN_REX);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RollDie"), "the quoted trigger must not be dropped: {debug}");
    let text = p67_text(&definition);
    assert!(text.contains("crew 2"), "{}", definition.canonical_text);
    assert!(text.contains("roll a six-sided die"), "{}", definition.canonical_text);
}

#[test]
fn p67_granted_list_never_drops_a_quoted_item() {
    // A keyword followed by an unreadable quoted ability is unsupported, not
    // a grant of the keyword alone.
    let result = compile_to_runtime_definition(
        "P67 Quoted Drop Probe",
        "Type: Artifact\n{T}: Target creature gains crew 2 and \"Frobnicate the zorblax.\" until end of turn.",
        false,
    );
    if let Ok(definition) = result {
        let text = p67_text(&definition);
        assert!(
            text.contains("frobnicate") || !text.contains("crew 2"),
            "crew 2 granted while the quoted item was dropped: {}",
            definition.canonical_text
        );
    }
}
