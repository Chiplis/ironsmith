//! Regression scenarios for the November 6, 2026 Comprehensive Rules audit.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::condition_eval::evaluate_condition_resolution;
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effect::Value;
use ironsmith::effects::{ChooseCreatureTypeEffect, FlipCoinEffect, RollDiceChooseResultEffect};
use ironsmith::types::SubtypeFamily;
use ironsmith::{
    AbilityKind, CardId, CardType, CounterType, DecisionMaker, EffectContext, EffectExecutor,
    GameState, ObjectId, Phase, PlayerFilter, PlayerId, PowerToughness, Zone,
};

const A: PlayerId = PlayerId(0);
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.turn_number = 2;
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}
fn creature(g: &mut GameState, name: &str, p: i32, t: i32) -> ObjectId {
    let d = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build();
    g.create_object_from_definition(&d, A, Zone::Battlefield)
}

fn card(route: &str) -> ironsmith::CardDefinition {
    let (name, block) = match route {
        "mountain" => ("Mountain", "Type: Basic Land — Mountain"),
        "bringer-of-the-last-gift" => (
            "Bringer of the Last Gift",
            "Mana cost: {6}{B}{B}\nType: Creature — Vampire Demon\nPower/Toughness: 6/6\nFlying\nWhen this creature enters, if you cast it, each player sacrifices all other creatures they control. Then each player returns all creature cards from their graveyard that weren't put there this way to the battlefield.",
        ),
        "dryad-militant" => (
            "Dryad Militant",
            "Mana cost: {G/W}\nType: Creature — Dryad Soldier\nPower/Toughness: 2/1\nIf an instant or sorcery card would be put into a graveyard from anywhere, exile it instead.",
        ),
        "buried-alive" => (
            "Buried Alive",
            "Mana cost: {2}{B}\nType: Sorcery\nSearch your library for up to three creature cards, put them into your graveyard, then shuffle.",
        ),
        "intuition" => (
            "Intuition",
            "Mana cost: {2}{U}\nType: Instant\nSearch your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.",
        ),
        "gifts-ungiven" => (
            "Gifts Ungiven",
            "Mana cost: {3}{U}\nType: Instant\nSearch your library for up to four cards with different names and reveal them. Target opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest into your hand. Then shuffle.",
        ),
        "realms-uncharted" => (
            "Realms Uncharted",
            "Mana cost: {2}{G}\nType: Instant\nSearch your library for up to four land cards with different names and reveal them. An opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest into your hand. Then shuffle.",
        ),
        "capital-punishment" => (
            "Capital Punishment",
            "Mana cost: {4}{B}{B}\nType: Sorcery\nCouncil's dilemma — Starting with you, each player votes for death or taxes. Each opponent sacrifices a creature of their choice for each death vote and discards a card for each taxes vote.",
        ),
        "horrifying-revelation" => (
            "Horrifying Revelation",
            "Mana cost: {B}\nType: Sorcery\nTarget player discards a card, then mills a card.",
        ),
        "tome-scour" => (
            "Tome Scour",
            "Mana cost: {U}\nType: Sorcery\nTarget player mills five cards.",
        ),
        "thought-scour" => (
            "Thought Scour",
            "Mana cost: {U}\nType: Instant\nTarget player mills two cards.\nDraw a card.",
        ),
        "notion-rain" => (
            "Notion Rain",
            "Mana cost: {1}{U}{B}\nType: Sorcery\nSurveil 2, then draw two cards. Notion Rain deals 2 damage to you. (To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
        ),
        "mind-rot" => (
            "Mind Rot",
            "Mana cost: {2}{B}\nType: Sorcery\nTarget player discards two cards.",
        ),
        "barter-in-blood" => (
            "Barter in Blood",
            "Mana cost: {2}{B}{B}\nType: Sorcery\nEach player sacrifices two creatures of their choice.",
        ),
        "wheel-of-fortune" => (
            "Wheel of Fortune",
            "Mana cost: {2}{R}\nType: Sorcery\nEach player discards their hand, then draws seven cards.",
        ),
        "shallow-grave" => (
            "Shallow Grave",
            "Mana cost: {1}{B}\nType: Instant\nReturn the top creature card of your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.",
        ),
        "walking-corpse" => (
            "Walking Corpse",
            "Mana cost: {1}{B}\nType: Creature — Zombie\nPower/Toughness: 2/2",
        ),
        "capricopian" => (
            "Capricopian",
            "Mana cost: {X}{G}\nType: Creature — Goat Hydra\nPower/Toughness: 0/0\nThis creature enters with X +1/+1 counters on it.\n{2}: Put a +1/+1 counter on this creature, then you may reselect which player this creature is attacking. Only the player this creature is attacking may activate this ability and only during the declare attackers step. (It can't attack its controller.)",
        ),
        "adanto-vanguard" => (
            "Adanto Vanguard",
            "Mana cost: {1}{W}\nType: Creature — Vampire Soldier\nPower/Toughness: 1/1\nAs long as this creature is attacking, it gets +2/+0.\nPay 4 life: This creature gains indestructible until end of turn. (Damage and effects that say \"destroy\" don't destroy it.)",
        ),
        "alarum" => (
            "Alarum",
            "Mana cost: {1}{W}\nType: Instant\nUntap target nonattacking creature. It gets +1/+3 until end of turn.",
        ),
        "leonardo-s-technique" => (
            "Leonardo's Technique",
            "Mana cost: {3}{W}\nType: Sorcery\nSneak {1}{W} (You may cast this spell for {1}{W} if you also return an unblocked attacker you control to hand during the declare blockers step.)\nReturn one or two target creature cards each with mana value 3 or less from your graveyard to the battlefield.",
        ),
        "portal-mage" => (
            "Portal Mage",
            "Mana cost: {2}{U}\nType: Creature — Human Wizard\nPower/Toughness: 2/2\nFlash\nWhen this creature enters during the declare attackers step, you may reselect which player or permanent target attacking creature is attacking. (It can't attack its controller or their permanents.)",
        ),
        "donatello-gadget-master" => (
            "Donatello, Gadget Master",
            "Mana cost: {2}{U}\nType: Legendary Creature — Mutant Ninja Turtle\nPower/Toughness: 3/2\nSneak {1}{U} (You may cast this spell for {1}{U} if you also return an unblocked attacker you control to hand during the declare blockers step. He enters tapped and attacking.)\nWhenever Donatello deals combat damage to a player, create a token that's a copy of target artifact you control.",
        ),
        "heroic-return" => (
            "Heroic Return",
            "Mana cost: {5}{W}\nType: Instant\nThis spell costs {2} less to cast if a creature is attacking you.\nReturn target creature card from your graveyard to the battlefield. If a Hero enters this way, it enters with two additional +1/+1 counters on it.",
        ),
        "swat-away" => (
            "Swat Away",
            "Mana cost: {2}{U}{U}\nType: Instant\nThis spell costs {2} less to cast if a creature is attacking you.\nThe owner of target spell or creature puts it on their choice of the top or bottom of their library.",
        ),
        "red-alert" => (
            "Red Alert!",
            "Mana cost: {2}{R}\nType: Instant\nThis spell costs {2} less to cast if a creature is attacking you.\nCreatures you control get +2/+0 until end of turn.",
        ),
        "qasali-ambusher" => (
            "Qasali Ambusher",
            "Mana cost: {1}{G}{W}\nType: Creature — Cat Warrior\nPower/Toughness: 2/3\nReach\nIf a creature is attacking you and you control a Forest and a Plains, you may cast this creature without paying its mana cost and as though it had flash.",
        ),
        "plains" => (
            "Plains",
            "Type: Basic Land — Plains",
        ),
        "veteran-bodyguard" => (
            "Veteran Bodyguard",
            "Mana cost: {3}{W}{W}\nType: Creature — Human\nPower/Toughness: 2/5\nAs long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead.",
        ),
        "mogg-fanatic" => (
            "Mogg Fanatic",
            "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1\nSacrifice this creature: It deals 1 damage to any target.",
        ),
        "elturel-survivors" => (
            "Elturel Survivors",
            "Mana cost: {3}{R}\nType: Creature — Tiefling Peasant\nPower/Toughness: 0/4\nTrample, myriad (Whenever this creature attacks, for each opponent other than defending player, you may create a token copy that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.)\nAs long as this creature is attacking, it gets +X/+0, where X is the number of lands defending player controls.",
        ),
        "kill-shot" => (
            "Kill Shot",
            "Mana cost: {2}{W}\nType: Instant\nDestroy target attacking creature.",
        ),
        "righteousness" => (
            "Righteousness",
            "Mana cost: {W}\nType: Instant\nTarget blocking creature gets +7/+7 until end of turn.",
        ),
        "balduvian-warlord" => (
            "Balduvian Warlord",
            "Mana cost: {3}{R}\nType: Creature — Human Barbarian\nPower/Toughness: 3/2\n{T}: Remove target blocking creature from combat. Creatures it was blocking that hadn't become blocked by another creature this combat become unblocked, then it blocks an attacking creature of your choice. Activate only during the declare blockers step.",
        ),
        "curtain-of-light" => (
            "Curtain of Light",
            "Mana cost: {1}{W}\nType: Instant\nCast this spell only during combat after blockers are declared.\nTarget unblocked attacking creature becomes blocked. (This spell works on creatures that can't be blocked.)\nDraw a card.",
        ),
        "dazzling-beauty" => (
            "Dazzling Beauty",
            "Mana cost: {2}{W}\nType: Instant\nCast this spell only during the declare blockers step.\nTarget unblocked attacking creature becomes blocked. (This spell works on creatures that can't be blocked.)\nDraw a card at the beginning of the next turn's upkeep.",
        ),
        "false-orders" => (
            "False Orders",
            "Mana cost: {R}\nType: Instant\nCast this spell only during the declare blockers step.\nRemove target creature defending player controls from combat. Creatures it was blocking that had become blocked by only that creature this combat become unblocked. You may have it block an attacking creature of your choice.",
        ),
        "general-jarkeld" => (
            "General Jarkeld",
            "Mana cost: {3}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 1/2\n{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step.",
        ),
        "ninja-of-the-deep-hours" => (
            "Ninja of the Deep Hours",
            "Mana cost: {3}{U}\nType: Creature — Human Ninja\nPower/Toughness: 2/2\nNinjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever this creature deals combat damage to a player, you may draw a card.",
        ),
        "sorrow-s-path" => (
            "Sorrow's Path",
            "Type: Land\n{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking.\nWhenever this land becomes tapped, it deals 2 damage to you and each creature you control.",
        ),
        "timetwister" => (
            "Timetwister",
            "Mana cost: {2}{U}\nType: Sorcery\nEach player shuffles their hand and graveyard into their library, then draws seven cards. (Then put Timetwister into its owner's graveyard.)",
        ),
        "step-between-worlds" => (
            "Step Between Worlds",
            "Mana cost: {3}{U}{U}\nType: Sorcery\nEach player may shuffle their hand and graveyard into their library. Each player who does draws seven cards. Exile Step Between Worlds.\nPlot {4}{U}{U} (You may pay {4}{U}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
        "sway-of-the-stars" => (
            "Sway of the Stars",
            "Mana cost: {8}{U}{U}\nType: Sorcery\nEach player shuffles their hand, graveyard, and all permanents they own into their library, then draws seven cards. Each player's life total becomes 7.",
        ),
        "turtles-in-time" => (
            "Turtles in Time",
            "Mana cost: {5}{U}{U}\nType: Sorcery\nReturn all creatures to their owners' hands. Each player may shuffle their hand and graveyard into their library, then each player who does draws seven cards.\nExile Turtles in Time.",
        ),
        "isamaru-hound-of-konda" => (
            "Isamaru, Hound of Konda",
            "Mana cost: {W}\nType: Legendary Creature — Dog\nPower/Toughness: 2/2",
        ),
        "void-stalker" => (
            "Void Stalker",
            "Mana cost: {1}{U}\nType: Creature — Elemental\nPower/Toughness: 2/1\n{2}{U}, {T}: Put this creature and target creature on top of their owners' libraries, then those players shuffle their libraries.",
        ),
        "vortex-elemental" => (
            "Vortex Elemental",
            "Mana cost: {U}\nType: Creature — Elemental\nPower/Toughness: 0/1\n{U}: Put this creature and each creature blocking or blocked by it on top of their owners' libraries, then those players shuffle.\n{3}{U}{U}: Target creature blocks this creature this turn if able.",
        ),
        "gomazoa" => (
            "Gomazoa",
            "Mana cost: {2}{U}\nType: Creature — Jellyfish\nPower/Toughness: 0/3\nDefender, flying\n{T}: Put this creature and each creature it's blocking on top of their owners' libraries, then those players shuffle.",
        ),
        "turn-the-earth" => (
            "Turn the Earth",
            "Mana cost: {G}\nType: Instant\nChoose up to three target cards in graveyards. The owners of those cards shuffle them into their libraries. You gain 2 life.\nFlashback {1}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "those-old-scientists" => (
            "Those Old Scientists",
            "Mana cost: {2}{U}\nType: Enchantment — Saga\n(As this Saga enters and after your draw\nstep, add a lore counter. Sacrifice after III.)\nI — Draw two cards, then discard two cards.\nII — Return up to one target artifact, instant, or sorcery card from your graveyard to your hand.\nIII — Choose up to two target cards in graveyards. The owners of those cards shuffle them into their libraries.",
        ),
        "guff-rewrites-history" => (
            "Guff Rewrites History",
            "Mana cost: {2}{R}\nType: Instant\nFor each player, choose target nonenchantment, nonland permanent that player controls. Those permanents' owners shuffle them into their libraries. Each player who controlled one of those permanents exiles cards from the top of their library until they exile a nonland card, then puts the rest on the bottom of their library in a random order. Each player may cast the nonland card they exiled without paying its mana cost.",
        ),
        "loaming-shaman" => (
            "Loaming Shaman",
            "Mana cost: {2}{G}\nType: Creature — Centaur Shaman\nPower/Toughness: 3/2\nWhen this creature enters, target player shuffles any number of target cards from their graveyard into their library.",
        ),
        "guile" => (
            "Guile",
            "Mana cost: {3}{U}{U}{U}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nThis creature can't be blocked except by three or more creatures.\nIf a spell or ability you control would counter a spell, instead exile that spell and you may play that card without paying its mana cost.\nWhen Guile is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "dread" => (
            "Dread",
            "Mana cost: {3}{B}{B}{B}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nFear (This creature can't be blocked except by artifact creatures and/or black creatures.)\nWhenever a creature deals damage to you, destroy it.\nWhen Dread is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "hostility" => (
            "Hostility",
            "Mana cost: {3}{R}{R}{R}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nHaste\nIf a spell you control would deal damage to an opponent, prevent that damage. Create a 3/1 red Elemental Shaman creature token with haste for each 1 damage prevented this way.\nWhen Hostility is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "purity" => (
            "Purity",
            "Mana cost: {3}{W}{W}{W}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nFlying\nIf noncombat damage would be dealt to you, prevent that damage. You gain life equal to the damage prevented this way.\nWhen Purity is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "vigor" => (
            "Vigor",
            "Mana cost: {3}{G}{G}{G}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nTrample\nIf damage would be dealt to another creature you control, prevent that damage. Put a +1/+1 counter on that creature for each 1 damage prevented this way.\nWhen Vigor is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "black-sun-s-zenith" => (
            "Black Sun's Zenith",
            "Mana cost: {X}{B}{B}\nType: Sorcery\nPut X -1/-1 counters on each creature. Shuffle Black Sun's Zenith into its owner's library.",
        ),
        "elixir-of-immortality" => (
            "Elixir of Immortality",
            "Mana cost: {1}\nType: Artifact\n{2}, {T}: You gain 5 life. Shuffle this artifact and your graveyard into their owner's library.",
        ),
        "cosi-s-trickster" => (
            "Cosi's Trickster",
            "Mana cost: {U}\nType: Creature — Merfolk Wizard\nPower/Toughness: 1/1\nWhenever an opponent shuffles their library, you may put a +1/+1 counter on this creature.",
        ),
        "floodpits-drowner" => (
            "Floodpits Drowner",
            "Mana cost: {1}{U}\nType: Creature — Merfolk\nPower/Toughness: 2/1\nFlash\nVigilance\nWhen this creature enters, tap target creature an opponent controls and put a stun counter on it.\n{1}{U}, {T}: Shuffle this creature and target creature with a stun counter on it into their owners' libraries.",
        ),
        "serene-remembrance" => (
            "Serene Remembrance",
            "Mana cost: {G}\nType: Sorcery\nShuffle Serene Remembrance and up to three target cards from a single graveyard into their owners' libraries.",
        ),
        "rishadan-pawnshop" => (
            "Rishadan Pawnshop",
            "Mana cost: {2}\nType: Artifact\n{2}, {T}: Shuffle target nontoken permanent you control into its owner's library.",
        ),
        "chaos-warp" => (
            "Chaos Warp",
            "Mana cost: {2}{R}\nType: Instant\nThe owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield.",
        ),
        "sin-unending-cataclysm" => (
            "Sin, Unending Cataclysm",
            "Mana cost: {5}{G}{U}\nType: Legendary Creature — Leviathan Avatar\nPower/Toughness: 5/5\nFlying, trample\nAs Sin enters, remove all counters from any number of artifacts, creatures, and enchantments. Sin enters with X +1/+1 counters on it, where X is twice the number of counters removed this way.\nWhen Sin dies, put its counters on target creature you control, then shuffle this card into its owner's library.",
        ),
        "teferi-temporal-pilgrim" => (
            "Teferi, Temporal Pilgrim",
            "Mana cost: {3}{U}{U}\nType: Legendary Planeswalker — Teferi\nLoyalty: 4\nWhenever you draw a card, put a loyalty counter on Teferi.\n0: Draw a card.\n−2: Create a 2/2 blue Spirit creature token with vigilance and \"Whenever you draw a card, put a +1/+1 counter on this token.\"\n−12: Target opponent chooses a permanent they control and returns it to its owner's hand. Then they shuffle each nonland permanent they control into its owner's library.",
        ),
        "cavalier-of-gales" => (
            "Cavalier of Gales",
            "Mana cost: {2}{U}{U}{U}\nType: Creature — Elemental Knight\nPower/Toughness: 5/5\nFlying\nWhen this creature enters, draw three cards, then put two cards from your hand on top of your library in any order.\nWhen this creature dies, shuffle it into its owner's library, then scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)",
        ),
        "angel-of-fury" => (
            "Angel of Fury",
            "Mana cost: {4}{W}{W}\nType: Creature — Angel\nPower/Toughness: 3/5\nFlying\nWhen this creature dies, you may shuffle it into its owner's library.",
        ),
        "alabaster-dragon" => (
            "Alabaster Dragon",
            "Mana cost: {4}{W}{W}\nType: Creature — Dragon\nPower/Toughness: 4/4\nFlying\nWhen this creature dies, shuffle it into its owner's library.",
        ),
        "serra-avatar" => (
            "Serra Avatar",
            "Mana cost: {4}{W}{W}{W}\nType: Creature — Avatar\nPower/Toughness: */*\nSerra Avatar's power and toughness are each equal to your life total.\nWhen Serra Avatar is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "worldspine-wurm" => (
            "Worldspine Wurm",
            "Mana cost: {8}{G}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 15/15\nTrample\nWhen this creature dies, create three 5/5 green Wurm creature tokens with trample.\nWhen Worldspine Wurm is put into a graveyard from anywhere, shuffle it into its owner's library.",
        ),
        "archivist-of-oghma" => ("Archivist of Oghma", "Mana cost: {1}{W}\nType: Creature — Halfling Cleric\nPower/Toughness: 2/2\nFlash\nWhenever an opponent searches their library, you gain 1 life and draw a card."),
        "tempt-with-discovery" => (
            "Tempt with Discovery",
            "Mana cost: {3}{G}\nType: Sorcery\nTempting offer — Search your library for a land card and put it onto the battlefield. Each opponent may search their library for a land card and put it onto the battlefield. For each opponent who searches a library this way, search your library for a land card and put it onto the battlefield. Then each player who searched a library this way shuffles.",
        ),
        "collective-voyage" => (
            "Collective Voyage",
            "Mana cost: {G}\nType: Sorcery\nJoin forces — Starting with you, each player may pay any amount of mana. Each player searches their library for up to X basic land cards, where X is the total amount of mana paid this way, puts them onto the battlefield tapped, then shuffles.",
        ),
        "pir-s-whim" => (
            "Pir's Whim",
            "Mana cost: {3}{G}\nType: Sorcery\nFor each player, choose friend or foe. Each friend searches their library for a land card, puts it onto the battlefield tapped, then shuffles. Each foe sacrifices an artifact or enchantment of their choice.",
        ),
        "veteran-explorer" => (
            "Veteran Explorer",
            "Mana cost: {G}\nType: Creature — Human Soldier Scout\nPower/Toughness: 1/1\nWhen this creature dies, each player may search their library for up to two basic land cards, put them onto the battlefield, then shuffle.",
        ),
        "avatar-of-growth" => (
            "Avatar of Growth",
            "Mana cost: {4}{G}{G}\nType: Creature — Elemental Avatar\nPower/Toughness: 4/4\nThis spell costs {1} less to cast for each opponent you have.\nTrample\nWhen this creature enters, each player searches their library for up to two basic land cards, puts them onto the battlefield, then shuffles.",
        ),
        "jungle-wayfinder" => (
            "Jungle Wayfinder",
            "Mana cost: {2}{G}\nType: Creature — Elf Warrior\nPower/Toughness: 3/3\nWhen this creature enters, each player may search their library for a basic land card, reveal it, put it into their hand, then shuffle.",
        ),
        "hired-giant" => (
            "Hired Giant",
            "Mana cost: {3}{R}\nType: Creature — Giant\nPower/Toughness: 4/4\nWhen this creature enters, each other player may search their library for a land card and put that card onto the battlefield. Then each player who searched their library this way shuffles.",
        ),
        "old-growth-dryads" => (
            "Old-Growth Dryads",
            "Mana cost: {G}\nType: Creature — Dryad\nPower/Toughness: 3/3\nWhen this creature enters, each opponent may search their library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
        "boldwyr-heavyweights" => (
            "Boldwyr Heavyweights",
            "Mana cost: {2}{R}{R}\nType: Creature — Giant Warrior\nPower/Toughness: 8/8\nTrample\nWhen this creature enters, each opponent may search their library for a creature card and put it onto the battlefield. Then each player who searched their library this way shuffles.",
        ),
        "noble-benefactor" => (
            "Noble Benefactor",
            "Mana cost: {2}{U}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nWhen this creature dies, each player may search their library for a card and put that card into their hand. Then each player who searched their library this way shuffles.",
        ),
        "rootweaver-druid" => (
            "Rootweaver Druid",
            "Mana cost: {2}{G}\nType: Creature — Elf Druid\nPower/Toughness: 2/1\nWhen this creature enters, each opponent may search their library for up to three basic land cards. They each put one of those cards onto the battlefield tapped under your control and the rest onto the battlefield tapped under their control. Then each player who searched their library this way shuffles.",
        ),
        "scheming-symmetry" => (
            "Scheming Symmetry",
            "Mana cost: {B}\nType: Sorcery\nChoose two target players. Each of them searches their library for a card, then shuffles and puts that card on top.",
        ),
        "field-of-ruin" => (
            "Field of Ruin",
            "Type: Land\n{T}: Add {C}.\n{2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. Each player searches their library for a basic land card, puts it onto the battlefield, then shuffles.",
        ),
        "weird-harvest" => (
            "Weird Harvest",
            "Mana cost: {X}{G}{G}\nType: Sorcery\nEach player may search their library for up to X creature cards, reveal those cards, put them into their hand, then shuffle.",
        ),
        "etali-primal-storm" => (
            "Etali, Primal Storm",
            "Mana cost: {4}{R}{R}\nType: Legendary Creature — Elder Dinosaur\nPower/Toughness: 6/6\nWhenever Etali attacks, exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs.",
        ),
        "etali-primal-conqueror" => (
            "Etali, Primal Conqueror",
            "Mana cost: {5}{R}{R}\nType: Legendary Creature — Elder Dinosaur\nPower/Toughness: 7/7\nTrample\nWhen Etali enters, each player exiles cards from the top of their library until they exile a nonland card. You may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.\n{9}{G/P}: Transform Etali. Activate only as a sorcery.",
        ),
        "primeval-spawn" => (
            "Primeval Spawn",
            "Mana cost: {5}{W}{U}{B}{R}{G}\nType: Creature — Avatar\nPower/Toughness: 10/10\nIf this creature would enter and it wasn't cast or no mana was spent to cast it, exile it instead.\nVigilance, trample, lifelink\nWhen this creature leaves the battlefield, exile the top ten cards of your library. You may cast any number of spells with total mana value 10 or less from among them without paying their mana costs.",
        ),
        "fevered-suspicion" => (
            "Fevered Suspicion",
            "Mana cost: {6}{B}{R}\nType: Sorcery\nEach opponent exiles cards from the top of their library until they exile a nonland card. You may cast any number of spells from among those nonland cards without paying their mana costs.\nRebound (If you cast this spell from your hand, exile it as it resolves. At the beginning of your next upkeep, you may cast this card from exile without paying its mana cost.)",
        ),
        "villainous-wealth" => (
            "Villainous Wealth",
            "Mana cost: {X}{B}{G}{U}\nType: Sorcery\nTarget opponent exiles the top X cards of their library. You may cast any number of spells with mana value X or less from among them without paying their mana costs.",
        ),
        "command-tower" => (
            "Command Tower",
            "Type: Land\n{T}: Add one mana of any color in your commander's color identity.",
        ),
        "darksteel-citadel" => (
            "Darksteel Citadel",
            "Type: Artifact Land\nIndestructible\n{T}: Add {C}.",
        ),
        "from-the-ashes" => (
            "From the Ashes",
            "Mana cost: {3}{R}\nType: Sorcery\nDestroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles.",
        ),
        "haunting-echoes" => (
            "Haunting Echoes",
            "Mana cost: {3}{B}{B}\nType: Sorcery\nExile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles.",
        ),
        "clarion-ultimatum" => (
            "Clarion Ultimatum",
            "Mana cost: {G}{G}{W}{W}{W}{U}{U}\nType: Sorcery\nChoose five permanents you control. For each of those permanents, you may search your library for a card with the same name as that permanent. Put those cards onto the battlefield tapped, then shuffle.",
        ),
        "dichotomancy" => (
            "Dichotomancy",
            "Mana cost: {7}{U}{U}\nType: Sorcery\nFor each tapped nonland permanent target opponent controls, search that player's library for a card with the same name as that permanent. Put those cards onto the battlefield under your control, then that player shuffles.\nSuspend 3—{1}{U}{U} (Rather than cast this card from your hand, you may pay {1}{U}{U} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
        ),
        "reap-intellect" => (
            "Reap Intellect",
            "Mana cost: {X}{2}{U}{B}\nType: Sorcery\nTarget opponent reveals their hand. You choose up to X nonland cards from it and exile them. For each card exiled this way, search that player's graveyard, hand, and library for any number of cards with the same name as that card and exile them. Then that player shuffles.",
        ),
        "doubling-chant" => (
            "Doubling Chant",
            "Mana cost: {5}{G}\nType: Sorcery\nFor each creature you control, you may search your library for a creature card with the same name as that creature. Put those cards onto the battlefield, then shuffle.",
        ),
        "krenko-s-buzzcrusher" => (
            "Krenko's Buzzcrusher",
            "Mana cost: {2}{R}{R}\nType: Artifact Creature — Insect Thopter\nPower/Toughness: 4/4\nFlying, trample\nWhen this creature enters, for each player, destroy up to one nonbasic land that player controls. For each land destroyed this way, its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
        "wave-of-vitriol" => (
            "Wave of Vitriol",
            "Mana cost: {5}{G}{G}\nType: Sorcery\nEach player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles.",
        ),
        "winds-of-abandon" => (
            "Winds of Abandon",
            "Mana cost: {1}{W}\nType: Sorcery\nExile target creature you don't control. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle.\nOverload {4}{W}{W} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        ),
        "jace-architect-of-thought" => (
            "Jace, Architect of Thought",
            "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 4\n+1: Until your next turn, whenever a creature an opponent controls attacks, it gets -1/-0 until end of turn.\n−2: Reveal the top three cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other on the bottom of your library in any order.\n−8: For each player, search that player's library for a nonland card and exile it, then that player shuffles. You may cast those cards without paying their mana costs.",
        ),
        "natural-balance" => (
            "Natural Balance",
            "Mana cost: {2}{G}{G}\nType: Sorcery\nEach player who controls six or more lands chooses five lands they control and sacrifices the rest. Each player who controls four or fewer lands may search their library for up to X basic land cards and put them onto the battlefield, where X is five minus the number of lands they control. Then each player who searched their library this way shuffles.",
        ),
        "new-frontiers" => (
            "New Frontiers",
            "Mana cost: {X}{G}\nType: Sorcery\nEach player may search their library for up to X basic land cards and put them onto the battlefield tapped. Then each player who searched their library this way shuffles.",
        ),
        "scholarship-sponsor" => (
            "Scholarship Sponsor",
            "Mana cost: {3}{W}\nType: Creature — Human Advisor\nPower/Toughness: 3/3\nWhen this creature enters, each player who controls fewer lands than the player who controls the most lands searches their library for a number of basic land cards less than or equal to the difference, puts those cards onto the battlefield tapped, then shuffles.",
        ),
        "borderland-explorer" => (
            "Borderland Explorer",
            "Mana cost: {1}{G}\nType: Creature — Elf Scout\nPower/Toughness: 3/1\nWhen this creature enters, each player may discard a card. Each player who discarded a card this way may search their library for a basic land card, reveal it, put it into their hand, then shuffle.",
        ),
        "turtle-tracks" => (
            "Turtle Tracks",
            "Mana cost: {2}{G}\nType: Sorcery\nAny number of target players may each search their library for a basic land card, put it onto the battlefield under their control, then shuffle.",
        ),
        "aven-mindcensor" => (
            "Aven Mindcensor",
            "Mana cost: {2}{W}\nType: Creature — Bird Wizard\nPower/Toughness: 2/1\nFlash\nFlying\nIf an opponent would search a library, that player searches the top four cards of that library instead.",
        ),
        "stranglehold" => (
            "Stranglehold",
            "Mana cost: {3}{R}\nType: Enchantment\nYour opponents can't search libraries.\nIf an opponent would begin an extra turn, that player skips that turn instead.",
        ),
        "opposition-agent" => (
            "Opposition Agent",
            "Mana cost: {2}{B}\nType: Creature — Human Rogue\nPower/Toughness: 3/2\nFlash\nYou control your opponents while they're searching their libraries.\nWhile an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.",
        ),
        "panglacial-wurm" => (
            "Panglacial Wurm",
            "Mana cost: {5}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 9/5\nTrample\nWhile you're searching your library, you may cast this card from your library.",
        ),
        "kaya-intangible-slayer" => (
            "Kaya, Intangible Slayer",
            "Mana cost: {3}{W}{W}{B}{B}\nType: Legendary Planeswalker — Kaya\nLoyalty: 6\nHexproof\n+2: Each opponent loses 3 life and you gain 3 life.\n0: You draw two cards. Then each opponent may scry 1.\n−3: Exile target creature or enchantment. If it wasn't an Aura, create a token that's a copy of it, except it's a 1/1 white Spirit creature with flying in addition to its other types.",
        ),
        "eager-construct" => (
            "Eager Construct",
            "Mana cost: {2}\nType: Artifact Creature — Construct\nPower/Toughness: 2/2\nWhen this creature enters, each player may scry 1. (To scry 1, look at the top card of your library, then you may put that card on the bottom.)",
        ),
        "kenessos-priest-of-thassa" => (
            "Kenessos, Priest of Thassa",
            "Mana cost: {1}{U}\nType: Legendary Creature — Merfolk Cleric\nPower/Toughness: 1/3\nIf you would scry a number of cards, scry that many cards plus one instead.\n{3}{G/U}: Look at the top card of your library. If it's a Kraken, Leviathan, Octopus, or Serpent creature card, you may put it onto the battlefield. If you don't put the card onto the battlefield, you may put it on the bottom of your library.",
        ),
        "eligeth-crossroads-augur" => (
            "Eligeth, Crossroads Augur",
            "Mana cost: {4}{U}{U}\nType: Legendary Creature — Sphinx\nPower/Toughness: 5/6\nFlying\nIf you would scry a number of cards, draw that many cards instead.\nPartner (You can have two commanders if both have partner.)",
        ),
        "the-temporal-anchor" => (
            "The Temporal Anchor",
            "Mana cost: {3}{U}{U}{U}\nType: Legendary Artifact\nAt the beginning of your upkeep, scry 2.\nWhenever you choose to put one or more cards on the bottom of your library while scrying, exile that many cards from the bottom of your library.\nDuring your turn, you may play cards exiled with The Temporal Anchor.",
        ),
        "model-of-unity" => (
            "Model of Unity",
            "Mana cost: {3}\nType: Artifact\nWhenever players finish voting, you and each opponent who voted for a choice you voted for may scry 2.\n{T}: Add one mana of any color.",
        ),
        "myr-custodian" => (
            "Myr Custodian",
            "Mana cost: {3}\nType: Artifact Creature — Myr\nPower/Toughness: 2/3\nWhen this creature enters, scry 2. Then each opponent may scry 1. (To scry X, a player looks at the top X cards of their library, then puts any number of them on the bottom and the rest on top in any order.)",
        ),
        "spire-of-industry" => (
            "Spire of Industry",
            "Type: Land\nFirst printed set: Aether Revolt\n{T}: Add {C}.\n{T}, Pay 1 life: Add one mana of any color. Activate only if you control an artifact.",
        ),
        "crumbling-vestige" => (
            "Crumbling Vestige",
            "Type: Land\nFirst printed set: Friday Night Magic 2016\nThis land enters tapped.\nWhen this land enters, add one mana of any color.\n{T}: Add {C}. ({C} represents colorless mana.)",
        ),
        "gaea-s-cradle" => (
            "Gaea's Cradle",
            "Type: Legendary Land\nFirst printed set: Judge Gift Cards 1998\n{T}: Add {G} for each creature you control.",
        ),
        "cannibalize" => (
            "Cannibalize",
            "Mana cost: {1}{B}\nType: Sorcery\nChoose two target creatures controlled by the same player. Exile one of those creatures and put two +1/+1 counters on the other.",
        ),
        "break-the-ice" => (
            "Break the Ice",
            "Mana cost: {B}{B}\nType: Sorcery\nFirst printed set: Modern Horizons 2\nDestroy target land that is snow or could produce {C}.\nOverload {4}{B}{B} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        ),
        "drannith-magistrate" => (
            "Drannith Magistrate",
            "Mana cost: {1}{W}\nType: Creature — Human Wizard\nFirst printed set: Ikoria: Lair of Behemoths\nPower/Toughness: 1/3\nYour opponents can't cast spells from anywhere other than their hands.",
        ),
        "the-eldest-reborn" => (
            "The Eldest Reborn",
            "Mana cost: {4}{B}\nType: Enchantment — Saga\nFirst printed set: Dominaria\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Each opponent sacrifices a creature or planeswalker of their choice.\nII — Each opponent discards a card.\nIII — Put target creature or planeswalker card from a graveyard onto the battlefield under your control.",
        ),
        "doubling-season" => (
            "Doubling Season",
            "Mana cost: {4}{G}\nType: Enchantment\nFirst printed set: Ravnica: City of Guilds\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.\nIf an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.",
        ),
        "urza-s-saga" => (
            "Urza's Saga",
            "Type: Enchantment Land — Urza's Saga\nFirst printed set: Modern Horizons 2 Promos\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — This Saga gains \"{T}: Add {C}.\"\nII — This Saga gains \"{2}, {T}: Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'\"\nIII — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.",
        ),
        "blood-moon" => (
            "Blood Moon",
            "Mana cost: {2}{R}\nType: Enchantment\nFirst printed set: The Dark\nNonbasic lands are Mountains.",
        ),
        "kilnmouth-dragon" => (
            "Kilnmouth Dragon",
            "Mana cost: {5}{R}{R}\nType: Creature — Dragon\nFirst printed set: Legions\nPower/Toughness: 5/5\nAmplify 3 (As this creature enters, put three +1/+1 counters on it for each Dragon card you reveal in your hand.)\nFlying\n{T}: This creature deals damage equal to the number of +1/+1 counters on it to any target.",
        ),
        "nameless-inversion" => (
            "Nameless Inversion",
            "Mana cost: {1}{B}\nType: Kindred Instant — Shapeshifter\nFirst printed set: Lorwyn\nChangeling (This card is every creature type.)\nTarget creature gets +3/-3 and loses all creature types until end of turn.",
        ),
        "arcane-adaptation" => (
            "Arcane Adaptation",
            "Mana cost: {2}{U}\nType: Enchantment\nFirst printed set: Ixalan Promos\nAs this enchantment enters, choose a creature type.\nCreatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.",
        ),
        "damping-sphere" => (
            "Damping Sphere",
            "Mana cost: {2}\nType: Artifact\nFirst printed set: Dominaria\nIf a land is tapped for two or more mana, it produces {C} instead of any other type and amount.\nEach spell a player casts costs {1} more to cast for each other spell that player has cast this turn.",
        ),
        "azorius-chancery" => (
            "Azorius Chancery",
            "Type: Land\nFirst printed set: Dissension\nThis land enters tapped.\nWhen this land enters, return a land you control to its owner's hand.\n{T}: Add {W}{U}.",
        ),
        "contamination" => (
            "Contamination",
            "Mana cost: {2}{B}\nType: Enchantment\nFirst printed set: Urza's Saga\nAt the beginning of your upkeep, sacrifice this enchantment unless you sacrifice a creature.\nIf a land is tapped for mana, it produces {B} instead of any other type and amount.",
        ),
        "wastes" => (
            "Wastes",
            "Type: Basic Land\nFirst printed set: Oath of the Gatewatch\n{T}: Add {C}.",
        ),
        "reflecting-pool" => (
            "Reflecting Pool",
            "Type: Land\nFirst printed set: Tempest\n{T}: Add one mana of any type that a land you control could produce.",
        ),
        "mystic-gate" => (
            "Mystic Gate",
            "Type: Land\nFirst printed set: Shadowmoor\n{T}: Add {C}.\n{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.",
        ),
        "eldritch-immunity" => (
            "Eldritch Immunity",
            "Mana cost: {C}\nType: Kindred Instant — Eldrazi\nFirst printed set: Modern Horizons 3 Commander\nTarget creature you control gains protection from each color until end of turn.\nOverload {4}{C} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        ),
        "thalia-heretic-cathar" => (
            "Thalia, Heretic Cathar",
            "Mana cost: {2}{W}\nType: Legendary Creature — Human Soldier\nFirst printed set: Eldritch Moon\nPower/Toughness: 3/2\nFirst strike\nCreatures and nonbasic lands your opponents control enter tapped.",
        ),
        "living-plane" => (
            "Living Plane",
            "Mana cost: {2}{G}{G}\nType: World Enchantment\nFirst printed set: Legends\nAll lands are 1/1 creatures that are still lands.",
        ),
        "arctic-foxes" => (
            "Arctic Foxes",
            "Mana cost: {1}{W}\nType: Creature — Fox\nPower/Toughness: 1/1\nThis creature can't be blocked by creatures with power 2 or greater as long as defending player controls a snow land.",
        ),
        "armory-veteran" => (
            "Armory Veteran",
            "Mana cost: {1}{R}\nType: Creature — Orc Warrior\nPower/Toughness: 2/2\nAs long as this creature is equipped, it has menace. (It can't be blocked except by two or more creatures.)",
        ),
        "bouncing-beebles" => (
            "Bouncing Beebles",
            "Mana cost: {2}{U}\nType: Creature — Beeble\nPower/Toughness: 2/2\nThis creature can't be blocked as long as defending player controls an artifact.",
        ),
        "chill" => (
            "Chill",
            "Mana cost: {1}{U}\nType: Enchantment\nRed spells cost {2} more to cast.",
        ),
        "foundry-inspector" => (
            "Foundry Inspector",
            "Mana cost: {3}\nType: Artifact Creature — Construct\nPower/Toughness: 3/2\nArtifact spells you cast cost {1} less to cast.",
        ),
        "ghostfire" => (
            "Ghostfire",
            "Mana cost: {2}{R}\nType: Instant\nGhostfire is colorless.\nGhostfire deals 3 damage to any target.",
        ),
        "grist-the-hunger-tide" => (
            "Grist, the Hunger Tide",
            "Mana cost: {1}{B}{G}\nType: Legendary Planeswalker — Grist\nLoyalty: 3\nAs long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.\n+1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.\n−2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.\n−5: Each opponent loses life equal to the number of creature cards in your graveyard.",
        ),
        "hazy-homunculus" => (
            "Hazy Homunculus",
            "Mana cost: {1}{U}\nType: Creature — Homunculus Illusion\nPower/Toughness: 1/1\nThis creature can't be blocked as long as defending player controls an untapped land.",
        ),
        "iona-shield-of-emeria" => (
            "Iona, Shield of Emeria",
            "Mana cost: {6}{W}{W}{W}\nType: Legendary Creature — Angel\nPower/Toughness: 7/7\nFlying\nAs Iona enters, choose a color.\nYour opponents can't cast spells of the chosen color.",
        ),
        "jegantha-the-wellspring" => (
            "Jegantha, the Wellspring",
            "Mana cost: {4}{R/G}\nType: Legendary Creature — Elemental Elk\nPower/Toughness: 5/5\nCompanion — No card in your starting deck has more than one of the same mana symbol in its mana cost. (If this card is your chosen companion, you may put it into your hand from outside the game for {3} as a sorcery.)\n{T}: Add {W}{U}{B}{R}{G}. This mana can't be spent to pay generic mana costs.",
        ),
        "kaheera-the-orphanguard" => (
            "Kaheera, the Orphanguard",
            "Mana cost: {1}{G/W}{G/W}\nType: Legendary Creature — Cat Beast\nPower/Toughness: 3/2\nCompanion — Each creature card in your starting deck is a Cat, Elemental, Nightmare, Dinosaur, or Beast card. (If this card is your chosen companion, you may put it into your hand from outside the game for {3} as a sorcery.)\nVigilance\nEach other creature you control that's a Cat, Elemental, Nightmare, Dinosaur, or Beast gets +1/+1 and has vigilance.",
        ),
        "keruga-the-macrosage" => (
            "Keruga, the Macrosage",
            "Mana cost: {3}{G/U}{G/U}\nType: Legendary Creature — Dinosaur Hippo\nPower/Toughness: 5/4\nCompanion — Your starting deck contains only cards with mana value 3 or greater and land cards. (If this card is your chosen companion, you may put it into your hand from outside the game for {3} as a sorcery.)\nWhen Keruga enters, draw a card for each other permanent you control with mana value 3 or greater.",
        ),
        "maelstrom-pulse" => (
            "Maelstrom Pulse",
            "Mana cost: {1}{B}{G}\nType: Sorcery\nDestroy target nonland permanent and all other permanents with the same name as that permanent.",
        ),
        "mistform-ultimus" => (
            "Mistform Ultimus",
            "Mana cost: {3}{U}\nType: Legendary Creature — Illusion\nPower/Toughness: 3/3\nMistform Ultimus is every creature type (even if this card isn't on the battlefield).",
        ),
        "pacifism" => (
            "Pacifism",
            "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack or block.",
        ),
        "paradise-mantle" => (
            "Paradise Mantle",
            "Mana cost: {0}\nType: Artifact — Equipment\nEquipped creature has \"{T}: Add one mana of any color.\"\nEquip {1}",
        ),
        "river-boa" => (
            "River Boa",
            "Mana cost: {1}{G}\nType: Creature — Snake\nPower/Toughness: 2/1\nIslandwalk (This creature can't be blocked as long as defending player controls an Island.)\n{G}: Regenerate this creature.",
        ),
        "snow-covered-forest" => (
            "Snow-Covered Forest",
            "Mana cost: \nType: Basic Snow Land — Forest\n({T}: Add {G}.)",
        ),
        "sol-ring" => (
            "Sol Ring",
            "Mana cost: {1}\nType: Artifact\n{T}: Add {C}{C}.",
        ),
        "sphinx-of-the-guildpact" => (
            "Sphinx of the Guildpact",
            "Mana cost: {7}\nType: Artifact Creature — Sphinx\nPower/Toughness: 5/5\nSphinx of the Guildpact is all colors.\nFlying\nHexproof from monocolored (This creature can't be the target of monocolored spells or abilities your opponents control.)",
        ),
        "spy-kit" => (
            "Spy Kit",
            "Mana cost: {2}\nType: Artifact — Equipment\nEquipped creature gets +1/+1 and has all names of nonlegendary creature cards in addition to its name.\nEquip {2}",
        ),
        "thran-golem" => (
            "Thran Golem",
            "Mana cost: {5}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3\nAs long as this creature is enchanted, it gets +2/+2 and has flying, first strike, and trample.",
        ),
        "umori-the-collector" => (
            "Umori, the Collector",
            "Mana cost: {2}{B/G}{B/G}\nType: Legendary Creature — Ooze\nPower/Toughness: 4/5\nCompanion — Each nonland card in your starting deck shares a card type. (If this card is your chosen companion, you may put it into your hand from outside the game for {3} as a sorcery.)\nAs Umori enters, choose a card type.\nSpells you cast of the chosen type cost {1} less to cast.",
        ),
        "willbender" => (
            "Willbender",
            "Mana cost: {1}{U}\nType: Creature — Human Wizard\nPower/Toughness: 1/2\nMorph {1}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)\nWhen this creature is turned face up, change the target of target spell or ability with a single target.",
        ),
        "wind-drake" => (
            "Wind Drake",
            "Mana cost: {2}{U}\nType: Creature — Drake\nPower/Toughness: 2/2\nFlying",
        ),
        "abyssal-harvester" => (
            "Abyssal Harvester",
            "Mana cost: {1}{B}{B}\nType: Creature — Demon Warlock\nFirst printed set: Foundations\nPower/Toughness: 3/2\n{T}: Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens you control.",
        ),
        "fiery-bombardment" => (
            "Fiery Bombardment",
            "Mana cost: {1}{R}\nType: Enchantment\nFirst printed set: Eventide\nChroma — {2}, Sacrifice a creature: This enchantment deals damage to any target equal to the number of red mana symbols in the sacrificed creature's mana cost.",
        ),
        "graf-rats" => (
            "Graf Rats",
            "Mana cost: {1}{B}\nType: Creature — Rat\nFirst printed set: Eldritch Moon\nPower/Toughness: 2/1\nAt the beginning of combat on your turn, if you both own and control this creature and a creature named Midnight Scavengers, exile them, then meld them into Chittering Host.",
        ),
        "grizzly-bears" => (
            "Grizzly Bears",
            "Mana cost: {1}{G}\nType: Creature — Bear\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 2/2",
        ),
        "island" => (
            "Island",
            "Type: Basic Land — Island\nFirst printed set: Limited Edition Alpha\n({T}: Add {U}.)",
        ),
        "ixidron" => (
            "Ixidron",
            "Mana cost: {3}{U}{U}\nType: Creature — Illusion\nFirst printed set: Time Spiral\nPower/Toughness: */*\nAs this creature enters, turn all other nontoken creatures face down. (They're 2/2 creatures.)\nIxidron's power and toughness are each equal to the number of face-down creatures on the battlefield.",
        ),
        "kira-nerys-firebrand" => (
            "Kira Nerys, Firebrand",
            "Mana cost: {1}{R}{W}\nType: Legendary Creature — Bajoran Soldier\nFirst printed set: Star Trek\nPower/Toughness: 4/3\nTrample\nIf a creature you control would enter while you control an attacking creature, instead it enters tapped and attacking.\nWhenever another creature you control enters, it gets +1/+1 until end of turn.",
        ),
        "krark-s-thumb" => (
            "Krark's Thumb",
            "Mana cost: {2}\nType: Legendary Artifact\nFirst printed set: Mirrodin\nIf you would flip a coin, instead flip two coins and ignore one.",
        ),
        "martial-coup" => (
            "Martial Coup",
            "Mana cost: {X}{W}{W}\nType: Sorcery\nFirst printed set: Conflux\nCreate X 1/1 white Soldier creature tokens. If X is 5 or more, destroy all other creatures.",
        ),
        "mirror-box" => (
            "Mirror Box",
            "Mana cost: {3}\nType: Artifact\nFirst printed set: Kamigawa: Neon Dynasty\nThe \"legend rule\" doesn't apply to permanents you control.\nEach legendary creature you control gets +1/+1.\nEach nontoken creature you control gets +1/+1 for each other creature you control with the same name as that creature.",
        ),
        "ral-zarek" => (
            "Ral Zarek",
            "Mana cost: {2}{U}{R}\nType: Legendary Planeswalker — Ral\nFirst printed set: Dragon's Maze\nLoyalty: 4\n+1: Tap target permanent, then untap another target permanent.\n−2: Ral Zarek deals 3 damage to any target.\n−7: Flip five coins. Take an extra turn after this one for each coin that comes up heads.",
        ),
        "stampeding-elk-herd" => (
            "Stampeding Elk Herd",
            "Mana cost: {3}{G}{G}\nType: Creature — Elk\nFirst printed set: Dragons of Tarkir\nPower/Toughness: 5/5\nFormidable — Whenever this creature attacks, if creatures you control have total power 8 or greater, creatures you control gain trample until end of turn.",
        ),
        "white-sun-s-twilight" => (
            "White Sun's Twilight",
            "Mana cost: {X}{W}{W}\nType: Sorcery\nFirst printed set: Phyrexia: All Will Be One\nYou gain X life. Create X 1/1 colorless Phyrexian Mite artifact creature tokens with toxic 1 and \"This token can't block.\" If X is 5 or more, destroy all other creatures. (Players dealt combat damage by a creature with toxic 1 also get a poison counter.)",
        ),
        "zethi-arcane-blademaster" => (
            "Zethi, Arcane Blademaster",
            "Mana cost: {1}{W}{U}\nType: Legendary Creature — Human Soldier\nFirst printed set: Secret Lair Drop\nPower/Toughness: 3/3\nMultikicker {W/U}\nWhen Zethi, Arcane Blademaster enters, exile up to X target instant cards from your graveyard, where X is the number of times Zethi was kicked. Put a kick counter on each of them.\nWhenever Zethi attacks, copy each exiled card you own with a kick counter on it. You may cast the copies.",
        ),
        "altar-golem" => (
            "Altar Golem",
            "Mana cost: {7}\nType: Artifact Creature — Golem\nFirst printed set: Eventide\nPower/Toughness: */*\nTrample\nAltar Golem's power and toughness are each equal to the number of creatures on the battlefield.\nThis creature doesn't untap during your untap step.\nTap five untapped creatures you control: Untap this creature.",
        ),
        "bayou-groff" => (
            "Bayou Groff",
            "Mana cost: {1}{G}\nType: Creature — Plant Dog\nFirst printed set: Strixhaven: School of Mages\nPower/Toughness: 5/4\nAs an additional cost to cast this spell, sacrifice a creature or pay {3}.",
        ),
        "borg-cube" => (
            "Borg Cube",
            "Mana cost: {5}{W}{B}\nType: Artifact — Spacecraft\nFirst printed set: Star Trek\nPower/Toughness: 6/6\nAffinity for artifact creatures\nWhenever this Spacecraft enters or attacks, assimilate a creature card exiled with it. If you can't, exile up to one target creature. (To assimilate a card, put it onto the battlefield under your control with a +1/+1 counter. It's a Borg artifact creature and loses all other creature types.)\nStation\n12+ | Artifact creatures you control have flying.",
        ),
        "contentious-plan" => (
            "Contentious Plan",
            "Mana cost: {1}{U}\nType: Sorcery\nFirst printed set: War of the Spark\nProliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)\nDraw a card.",
        ),
        "discerning-financier" => (
            "Discerning Financier",
            "Mana cost: {2}{W}\nType: Creature — Human Noble\nFirst printed set: Wilds of Eldraine\nPower/Toughness: 2/3\nAt the beginning of your upkeep, if an opponent controls more lands than you, create a Treasure token. (It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")\n{2}{W}: Choose another player. That player gains control of target Treasure you control. You draw a card.",
        ),
        "divination" => (
            "Divination",
            "Mana cost: {2}{U}\nType: Sorcery\nFirst printed set: Magic 2010\nDraw two cards.",
        ),
        "frankenstein-s-monster" => (
            "Frankenstein's Monster",
            "Mana cost: {X}{B}{B}\nType: Creature — Zombie\nFirst printed set: The Dark\nPower/Toughness: 0/1\nAs this creature enters, exile X creature cards from your graveyard. If you can't, put this creature into its owner's graveyard instead of onto the battlefield. For each creature card exiled this way, this creature enters with a +2/+0, +1/+1, or +0/+2 counter on it.",
        ),
        "humility" => (
            "Humility",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nFirst printed set: Tempest\nAll creatures lose all abilities and have base power and toughness 1/1.",
        ),
        "muraganda-petroglyphs" => (
            "Muraganda Petroglyphs",
            "Mana cost: {3}{G}\nType: Enchantment\nFirst printed set: Future Sight\nCreatures with no abilities get +2/+2.",
        ),
        "pain-s-reward" => (
            "Pain's Reward",
            "Mana cost: {2}{B}\nType: Sorcery\nFirst printed set: Saviors of Kamigawa\nEach player may bid life. You start the bidding with a bid of any number. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and draws four cards.",
        ),
        "past-in-flames" => (
            "Past in Flames",
            "Mana cost: {3}{R}\nType: Sorcery\nFirst printed set: Innistrad\nEach instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "phyrexian-revoker" => (
            "Phyrexian Revoker",
            "Mana cost: {2}\nType: Artifact Creature — Phyrexian Horror\nFirst printed set: Mirrodin Besieged\nPower/Toughness: 2/1\nAs this creature enters, choose a nonland card name.\nActivated abilities of sources with the chosen name can't be activated.",
        ),
        "pithing-needle" => (
            "Pithing Needle",
            "Mana cost: {1}\nType: Artifact\nFirst printed set: Saviors of Kamigawa\nAs this artifact enters, choose a card name.\nActivated abilities of sources with the chosen name can't be activated unless they're mana abilities.",
        ),
        "plea-for-power" => (
            "Plea for Power",
            "Mana cost: {3}{U}\nType: Sorcery\nFirst printed set: Conspiracy\nWill of the council — Starting with you, each player votes for time or knowledge. If time gets more votes, take an extra turn after this one. If knowledge gets more votes or the vote is tied, draw three cards.",
        ),
        "reanimate" => (
            "Reanimate",
            "Mana cost: {B}\nType: Sorcery\nFirst printed set: Tempest\nPut target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value.",
        ),
        "elvish-spirit-guide" => (
            "Elvish Spirit Guide",
            "Mana cost: {2}{G}\nType: Creature — Elf Spirit\nPower/Toughness: 2/2\nExile this card from your hand: Add {G}.",
        ),
        "llanowar-elves" => (
            "Llanowar Elves",
            "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}.",
        ),
        "maskwood-nexus" => (
            "Maskwood Nexus",
            "Mana cost: {4}\nType: Artifact\nCreatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\n{3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling.",
        ),
        "phyrexian-ingester" => (
            "Phyrexian Ingester",
            "Mana cost: {6}{U}\nType: Creature — Phyrexian Beast\nPower/Toughness: 3/3\nImprint — When this creature enters, you may exile target nontoken creature.\nThis creature gets +X/+Y, where X is the exiled creature card's power, and Y is the exiled creature card's toughness.",
        ),
        "duplicant" => (
            "Duplicant",
            "Mana cost: {6}\nType: Artifact Creature — Shapeshifter\nPower/Toughness: 2/4\nImprint — When this creature enters, you may exile target nontoken creature.\nAs long as a card exiled with this creature is a creature card, this creature has the power, toughness, and creature types of the last creature card exiled with it. It's still a Shapeshifter.",
        ),
        "battle-squadron" => (
            "Battle Squadron",
            "Mana cost: {3}{R}{R}\nType: Creature — Goblin\nPower/Toughness: */*\nFlying\nBattle Squadron's power and toughness are each equal to the number of creatures you control.",
        ),
        "simian-spirit-guide" => (
            "Simian Spirit Guide",
            "Mana cost: {2}{R}\nType: Creature — Ape Spirit\nFirst printed set: Planar Chaos\nPower/Toughness: 2/2\nExile this card from your hand: Add {R}.",
        ),
        "street-wraith" => (
            "Street Wraith",
            "Mana cost: {3}{B}{B}\nType: Creature — Wraith\nFirst printed set: Future Sight\nPower/Toughness: 3/4\nSwampwalk (This creature can't be blocked as long as defending player controls a Swamp.)\nCycling—Pay 2 life. (Pay 2 life, Discard this card: Draw a card.)",
        ),
        "think-twice" => (
            "Think Twice",
            "Mana cost: {1}{U}\nType: Instant\nFirst printed set: Time Spiral\nDraw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "underworld-breach" => (
            "Underworld Breach",
            "Mana cost: {1}{R}\nType: Enchantment\nFirst printed set: Theros Beyond Death Promos\nEach nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)\nAt the beginning of the end step, sacrifice this enchantment.",
        ),
        "yixlid-jailer" => (
            "Yixlid Jailer",
            "Mana cost: {1}{B}\nType: Creature — Zombie Wizard\nFirst printed set: Future Sight\nPower/Toughness: 2/1\nCards in graveyards lose all abilities.",
        ),
        "biotransference" => (
            "Biotransference",
            "Mana cost: {2}{B}{B}\nType: Enchantment\nFirst printed set: Warhammer 40,000 Commander\nCreatures you control are artifacts in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\nWhenever you cast an artifact spell, you lose 1 life and create a 2/2 black Necron Warrior artifact creature token.",
        ),
        "chromatic-orrery" => (
            "Chromatic Orrery",
            "Mana cost: {7}\nType: Legendary Artifact\nFirst printed set: Core Set 2021\nYou may spend mana as though it were mana of any color.\n{T}: Add {C}{C}{C}{C}{C}.\n{5}, {T}: Draw a card for each color among permanents you control.",
        ),
        "chrome-mox" => (
            "Chrome Mox",
            "Mana cost: {0}\nType: Artifact\nFirst printed set: Mirrodin\nImprint — When this artifact enters, you may exile a nonartifact, nonland card from your hand.\n{T}: Add one mana of any of the exiled card's colors.",
        ),
        "dusk-feaster" => (
            "Dusk Feaster",
            "Mana cost: {5}{B}{B}\nType: Creature — Vampire\nFirst printed set: Eldritch Moon\nPower/Toughness: 4/5\nDelirium — This spell costs {2} less to cast if there are four or more card types among cards in your graveyard.\nFlying",
        ),
        "elemental-resonance" => (
            "Elemental Resonance",
            "Mana cost: {2}{G}{G}\nType: Enchantment — Aura\nFirst printed set: Dissension\nEnchant permanent\nAt the beginning of your first main phase, add mana equal to enchanted permanent's mana cost. (Mana cost includes color. If a mana symbol has multiple colors, choose one.)",
        ),
        "faeburrow-elder" => (
            "Faeburrow Elder",
            "Mana cost: {1}{G}{W}\nType: Creature — Treefolk Druid\nFirst printed set: Throne of Eldraine\nPower/Toughness: 0/0\nVigilance\nThis creature gets +1/+1 for each color among permanents you control.\n{T}: For each color among permanents you control, add one mana of that color.",
        ),
        "forest" => (
            "Forest",
            "Type: Basic Land — Forest\nFirst printed set: Limited Edition Alpha\n({T}: Add {G}.)",
        ),
        "hill-giant" => (
            "Hill Giant",
            "Mana cost: {3}{R}\nType: Creature — Giant\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 3/3",
        ),
        "lunar-insight" => (
            "Lunar Insight",
            "Mana cost: {2}{U}\nType: Sorcery\nFirst printed set: Foundations\nDraw a card for each different mana value among nonland permanents you control.",
        ),
        "mirrorweave" => (
            "Mirrorweave",
            "Mana cost: {2}{W/U}{W/U}\nType: Instant\nFirst printed set: Shadowmoor\nEach other creature becomes a copy of target nonlegendary creature until end of turn.",
        ),
        "mycosynth-lattice" => (
            "Mycosynth Lattice",
            "Mana cost: {6}\nType: Artifact\nFirst printed set: Darksteel\nAll permanents are artifacts in addition to their other types.\nAll cards that aren't on the battlefield, spells, and permanents are colorless.\nPlayers may spend mana as though it were mana of any color.",
        ),
        "nighthawk-scavenger" => (
            "Nighthawk Scavenger",
            "Mana cost: {1}{B}{B}\nType: Creature — Vampire Rogue\nFirst printed set: Zendikar Rising Promos\nPower/Toughness: 1+*/3\nFlying, deathtouch, lifelink\nNighthawk Scavenger's power is equal to 1 plus the number of card types among cards in your opponents' graveyards.",
        ),
        "painter-s-servant" => (
            "Painter's Servant",
            "Mana cost: {2}\nType: Artifact Creature — Scarecrow\nFirst printed set: Shadowmoor\nPower/Toughness: 1/3\nAs this creature enters, choose a color.\nAll cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors.",
        ),
        "pit-of-offerings" => (
            "Pit of Offerings",
            "Type: Land — Cave\nFirst printed set: The Lost Caverns of Ixalan\nThis land enters tapped.\nWhen this land enters, exile up to three target cards from graveyards.\n{T}: Add {C}.\n{T}: Add one mana of any of the exiled cards' colors.",
        ),
        "reverse-the-sands" => (
            "Reverse the Sands",
            "Mana cost: {6}{W}{W}\nType: Sorcery\nFirst printed set: Champions of Kamigawa\nRedistribute any number of players' life totals. (Each of those players gets one life total back.)",
        ),
        "savannah-lions" => (
            "Savannah Lions",
            "Mana cost: {W}\nType: Creature — Cat\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 2/1",
        ),
        "tarmogoyf" => (
            "Tarmogoyf",
            "Mana cost: {1}{G}\nType: Creature — Lhurgoyf\nFirst printed set: Future Sight\nPower/Toughness: */1+*\nTarmogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.",
        ),
        "dark-ritual" => (
            "Dark Ritual",
            "Mana cost: {B}\nType: Instant\nFirst printed set: Limited Edition Alpha\nAdd {B}{B}{B}.",
        ),
        "opt" => (
            "Opt",
            "Mana cost: {U}\nType: Instant\nFirst printed set: Invasion\nScry 1. (Look at the top card of your library. You may put that card on the bottom.)\nDraw a card.",
        ),
        "abomination-of-llanowar" => (
            "Abomination of Llanowar",
            "Mana cost: {1}{B}{G}\nType: Legendary Creature — Elf Horror\nFirst printed set: Commander Legends\nPower/Toughness: */*\nVigilance; menace (This creature can't be blocked except by two or more creatures.)\nAbomination of Llanowar's power and toughness are each equal to the number of Elves you control plus the number of Elf cards in your graveyard.",
        ),
        "amoeboid-changeling" => (
            "Amoeboid Changeling",
            "Mana cost: {1}{U}\nType: Creature — Shapeshifter\nFirst printed set: Lorwyn\nPower/Toughness: 1/1\nChangeling (This card is every creature type.)\n{T}: Target creature gains all creature types until end of turn.\n{T}: Target creature loses all creature types until end of turn.",
        ),
        "ash-barrens" => (
            "Ash Barrens",
            "Type: Land\nFirst printed set: Commander 2016\n{T}: Add {C}.\nBasic landcycling {1} ({1}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)",
        ),
        "beacon-of-immortality" => (
            "Beacon of Immortality",
            "Mana cost: {5}{W}\nType: Instant\nFirst printed set: Fifth Dawn\nDouble target player's life total. Shuffle Beacon of Immortality into its owner's library.",
        ),
        "blessed-wind" => (
            "Blessed Wind",
            "Mana cost: {7}{W}{W}\nType: Sorcery\nFirst printed set: Prophecy\nTarget player's life total becomes 20.",
        ),
        "conspiracy" => (
            "Conspiracy",
            "Mana cost: {3}{B}{B}\nType: Enchantment\nFirst printed set: Mercadian Masques\nAs this enchantment enters, choose a creature type.\nCreatures you control are the chosen type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.",
        ),
        "gavi-nest-warden" => (
            "Gavi, Nest Warden",
            "Mana cost: {2}{U}{R}{W}\nType: Legendary Creature — Human Shaman\nFirst printed set: Commander 2020\nPower/Toughness: 2/5\nYou may pay {0} rather than pay the cycling cost of the first card you cycle each turn.\nWhenever you draw your second card each turn, create a 2/2 red and white Dinosaur Cat creature token.",
        ),
        "impact-tremors" => (
            "Impact Tremors",
            "Mana cost: {1}{R}\nType: Enchantment\nFirst printed set: Dragons of Tarkir\nWhenever a creature you control enters, this enchantment deals 1 damage to each opponent.",
        ),
        "lightning-bolt" => (
            "Lightning Bolt",
            "Mana cost: {R}\nType: Instant\nFirst printed set: Limited Edition Alpha\nLightning Bolt deals 3 damage to any target.",
        ),
        "new-perspectives" => (
            "New Perspectives",
            "Mana cost: {5}{U}\nType: Enchantment\nFirst printed set: Amonkhet\nWhen this enchantment enters, draw three cards.\nAs long as you have seven or more cards in hand, you may pay {0} rather than pay cycling costs.",
        ),
        "psychic-transfer" => (
            "Psychic Transfer",
            "Mana cost: {4}{U}\nType: Sorcery\nFirst printed set: Mirage\nIf the difference between your life total and target player's life total is 5 or less, exchange life totals with that player.",
        ),
        "rain-of-gore" => (
            "Rain of Gore",
            "Mana cost: {B}{R}\nType: Enchantment\nFirst printed set: Dissension\nIf a spell or ability would cause its controller to gain life, that player loses that much life instead.",
        ),
        "sacred-nectar" => (
            "Sacred Nectar",
            "Mana cost: {1}{W}\nType: Sorcery\nFirst printed set: Portal\nYou gain 4 life.",
        ),
        "sage-owl" => (
            "Sage Owl",
            "Mana cost: {1}{U}\nType: Creature — Bird\nFirst printed set: Weatherlight\nPower/Toughness: 1/1\nFlying\nWhen this creature enters, look at the top four cards of your library, then put them back in any order.",
        ),
        "show-and-tell" => (
            "Show and Tell",
            "Mana cost: {2}{U}\nType: Sorcery\nFirst printed set: Urza's Saga\nEach player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.",
        ),
        "soulless-one" => (
            "Soulless One",
            "Mana cost: {3}{B}\nType: Creature — Zombie Avatar\nFirst printed set: Onslaught\nPower/Toughness: */*\nSoulless One's power and toughness are each equal to the number of Zombies on the battlefield plus the number of Zombie cards in all graveyards.",
        ),
        "sulfuric-vortex" => (
            "Sulfuric Vortex",
            "Mana cost: {1}{R}{R}\nType: Enchantment\nFirst printed set: Scourge\nAt the beginning of each player's upkeep, this enchantment deals 2 damage to that player.\nIf a player would gain life, that player gains no life instead.",
        ),
        "suppression-field" => (
            "Suppression Field",
            "Mana cost: {1}{W}\nType: Enchantment\nFirst printed set: Ravnica: City of Guilds\nActivated abilities cost {2} more to activate unless they're mana abilities.",
        ),
        "tainted-remedy" => (
            "Tainted Remedy",
            "Mana cost: {2}{B}\nType: Enchantment\nFirst printed set: Magic Origins\nIf an opponent would gain life, that player loses that much life instead.",
        ),
        "the-wanderer" => (
            "The Wanderer",
            "Mana cost: {3}{W}\nType: Legendary Planeswalker\nFirst printed set: War of the Spark Promos\nLoyalty: 5\nPrevent all noncombat damage that would be dealt to you and other permanents you control.\n−2: Exile target creature with power 4 or greater.",
        ),
        "tireless-tracker" => (
            "Tireless Tracker",
            "Mana cost: {2}{G}\nType: Creature — Human Scout\nFirst printed set: Shadows over Innistrad Promos\nPower/Toughness: 3/2\nLandfall — Whenever a land you control enters, investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")\nWhenever you sacrifice a Clue, put a +1/+1 counter on this creature.",
        ),
        "traveling-chocobo" => (
            "Traveling Chocobo",
            "Mana cost: {2}{G}\nType: Creature — Bird\nFirst printed set: Final Fantasy\nPower/Toughness: 3/2\nYou may look at the top card of your library any time.\nYou may play lands and cast Bird spells from the top of your library.\nIf a land or Bird you control entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        ),
        "sephara-sky-s-blade" => (
            "Sephara, Sky's Blade",
            "Mana cost: {4}{W}{W}{W}\nType: Legendary Creature — Angel\nPower/Toughness: 7/7\nYou may pay {W} and tap four untapped creatures you control with flying rather than pay this spell's mana cost.\nFlying, lifelink\nOther creatures you control with flying have indestructible. (Damage and effects that say \"destroy\" don't destroy them.)",
        ),
        "wrath-of-god" => (
            "Wrath of God",
            "Mana cost: {2}{W}{W}\nType: Sorcery\nDestroy all creatures. They can't be regenerated.",
        ),
        "darksteel-myr" => (
            "Darksteel Myr",
            "Mana cost: {3}\nType: Artifact Creature — Myr\nPower/Toughness: 0/1\nIndestructible (Damage and effects that say \"destroy\" don't destroy this creature. If its toughness is 0 or less, it still dies.)",
        ),
        "march-of-souls" => (
            "March of Souls",
            "Mana cost: {4}{W}\nType: Sorcery\nDestroy all creatures. They can't be regenerated. For each creature destroyed this way, its controller creates a 1/1 white Spirit creature token with flying.",
        ),
        "leyline-of-singularity" => (
            "Leyline of Singularity",
            "Mana cost: {2}{U}{U}\nType: Enchantment\nIf this card is in your opening hand, you may begin the game with it on the battlefield.\nAll nonland permanents are legendary.",
        ),
        "echoing-truth" => (
            "Echoing Truth",
            "Mana cost: {1}{U}\nType: Instant\nReturn target nonland permanent and all other permanents with the same name as that permanent to their owners' hands.",
        ),
        "witness-protection" => (
            "Witness Protection",
            "Mana cost: {U}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson. (It loses all other colors, card types, creature types, and names.)",
        ),
        "sever-the-bloodline" => (
            "Sever the Bloodline",
            "Mana cost: {3}{B}\nType: Sorcery\nExile target creature and all other creatures with the same name as that creature.\nFlashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "winnow" => (
            "Winnow",
            "Mana cost: {1}{W}\nType: Instant\nDestroy target nonland permanent if another permanent with the same name is on the battlefield.\nDraw a card.",
        ),
        "eradicate" => (
            "Eradicate",
            "Mana cost: {2}{B}{B}\nType: Sorcery\nExile target nonblack creature. Search its controller's graveyard, hand, and library for all cards with the same name as that creature and exile them. Then that player shuffles.",
        ),
        "hallowed-moonlight" => (
            "Hallowed Moonlight",
            "Mana cost: {1}{W}\nType: Instant\nUntil end of turn, if a creature would enter and it wasn't cast, exile it instead.\nDraw a card.",
        ),
        "march-of-the-machines" => (
            "March of the Machines",
            "Mana cost: {3}{U}\nType: Enchantment\nEach noncreature artifact is an artifact creature with power and toughness each equal to its mana value. (Equipment that's a creature can't equip a creature.)",
        ),
        "refurbish" => (
            "Refurbish",
            "Mana cost: {3}{W}\nType: Sorcery\nReturn target artifact card from your graveyard to the battlefield.",
        ),
        "heliod-god-of-the-sun" => (
            "Heliod, God of the Sun",
            "Mana cost: {3}{W}\nType: Legendary Enchantment Creature — God\nPower/Toughness: 5/6\nIndestructible\nAs long as your devotion to white is less than five, Heliod isn't a creature. (Each {W} in the mana costs of permanents you control counts toward your devotion to white.)\nOther creatures you control have vigilance.\n{2}{W}{W}: Create a 2/1 white Cleric enchantment creature token.",
        ),
        "zombify" => (
            "Zombify",
            "Mana cost: {3}{B}\nType: Sorcery\nReturn target creature card from your graveyard to the battlefield.",
        ),
        "dryad-arbor" => (
            "Dryad Arbor",
            "Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1\n(This land isn't a spell, it's affected by summoning sickness, and it has \"{T}: Add {G}.\")",
        ),
        "awakened-amalgam" => (
            "Awakened Amalgam",
            "Mana cost: {4}\nType: Artifact Creature — Golem\nPower/Toughness: */*\nAwakened Amalgam's power and toughness are each equal to the number of differently named lands you control.",
        ),
        "graveyard-shift" => (
            "Graveyard Shift",
            "Mana cost: {4}{B}\nType: Sorcery\nThis spell has flash as long as there are five or more mana values among cards in your graveyard.\nReturn target creature card from your graveyard to the battlefield.",
        ),
        "ornithopter" => (
            "Ornithopter",
            "Mana cost: {0}\nType: Artifact Creature — Thopter\nPower/Toughness: 0/2\nFlying",
        ),
        "centaur-courser" => (
            "Centaur Courser",
            "Mana cost: {2}{G}\nType: Creature — Centaur Warrior\nPower/Toughness: 3/3",
        ),
        "shapeshifter" => (
            "Shapeshifter",
            "Mana cost: {6}\nType: Artifact Creature — Shapeshifter\nFirst printed set: Antiquities\nPower/Toughness: */7-*\nAs this creature enters, choose a number between 0 and 7.\nAt the beginning of your upkeep, you may choose a number between 0 and 7.\nShapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number.",
        ),
        "planar-nexus" => (
            "Planar Nexus",
            "Type: Land\nFirst printed set: Modern Horizons 3 Commander\nThis land is every nonbasic land type. (Nonbasic land types include Cave, Desert, Gate, Lair, Locus, Mine, Power-Plant, Sphere, Tower, and Urza's.)\n{T}: Add {C}.\n{1}, {T}: Add one mana of any color.",
        ),
        "dress-down" => (
            "Dress Down",
            "Mana cost: {1}{U}\nType: Enchantment\nFirst printed set: Modern Horizons 2 Promos\nFlash\nWhen this enchantment enters, draw a card.\nCreatures lose all abilities.\nAt the beginning of the end step, sacrifice this enchantment.",
        ),
        "invasion-of-zendikar" => (
            "Invasion of Zendikar",
            "Mana cost: {3}{G}\nType: Battle — Siege\nDefense: 3\n(As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)\nWhen this Siege enters, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
        ),
        "gavony-dawnguard" => (
            "Gavony Dawnguard",
            "Mana cost: {1}{W}{W}\nType: Creature — Human Soldier\nFirst printed set: Innistrad: Midnight Hunt\nPower/Toughness: 3/3\nWard {1}\nIf it's neither day nor night, it becomes day as this creature enters.\nWhenever day becomes night or night becomes day, look at the top four cards of your library. You may reveal a creature card with mana value 3 or less from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        ),
        "worldfire" => (
            "Worldfire",
            "Mana cost: {6}{R}{R}{R}\nType: Sorcery\nFirst printed set: Magic 2013\nExile all permanents. Exile all cards from all hands and graveyards. Each player's life total becomes 1.",
        ),
        "knight-of-grace" => (
            "Knight of Grace",
            "Mana cost: {1}{W}\nType: Creature — Human Knight\nFirst printed set: Dominaria\nPower/Toughness: 2/2\nFirst strike (This creature deals combat damage before creatures without first strike.)\nHexproof from black (This creature can't be the target of black spells or abilities your opponents control.)\nThis creature gets +1/+0 as long as any player controls a black permanent.",
        ),
        "knight-of-malice" => (
            "Knight of Malice",
            "Mana cost: {1}{B}\nType: Creature — Human Knight\nFirst printed set: Dominaria\nPower/Toughness: 2/2\nFirst strike (This creature deals combat damage before creatures without first strike.)\nHexproof from white (This creature can't be the target of white spells or abilities your opponents control.)\nThis creature gets +1/+0 as long as any player controls a white permanent.",
        ),
        "majestic-myriarch" => (
            "Majestic Myriarch",
            "Mana cost: {4}{G}\nType: Creature — Chimera\nFirst printed set: Hour of Devastation\nPower/Toughness: */*\nMajestic Myriarch's power and toughness are each equal to twice the number of creatures you control.\nAt the beginning of each combat, this creature gains flying until end of turn if you control a creature with flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance.",
        ),
        "urborg-scavengers" => (
            "Urborg Scavengers",
            "Mana cost: {2}{B}\nType: Creature — Spirit\nFirst printed set: March of the Machine: The Aftermath\nPower/Toughness: 2/2\nWhenever this creature enters or attacks, exile target card from a graveyard. Put a +1/+1 counter on this creature.\nThis creature has flying as long as a card exiled with it has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance.",
        ),
        "kathril-aspect-warper" => (
            "Kathril, Aspect Warper",
            "Mana cost: {2}{W}{B}{G}\nType: Legendary Creature — Nightmare Insect\nFirst printed set: Commander 2020\nPower/Toughness: 3/3\nWhen Kathril enters, put a flying counter on any creature you control if a creature card in your graveyard has flying. Repeat this process for first strike, double strike, deathtouch, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance. Then put a +1/+1 counter on Kathril for each counter put on a creature this way.",
        ),
        "agatha-s-soul-cauldron" => (
            "Agatha's Soul Cauldron",
            "Mana cost: {2}\nType: Legendary Artifact\nFirst printed set: Wilds of Eldraine Promos\nYou may spend mana as though it were mana of any color to activate abilities of creatures you control.\nCreatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with Agatha's Soul Cauldron.\n{T}: Exile target card from a graveyard. When a creature card is exiled this way, put a +1/+1 counter on target creature you control.",
        ),
        "leaden-myr" => (
            "Leaden Myr",
            "Mana cost: {2}\nType: Artifact Creature — Myr\nFirst printed set: Mirrodin\nPower/Toughness: 1/1\n{T}: Add {B}.",
        ),
        "confiscate" => (
            "Confiscate",
            "Mana cost: {4}{U}{U}\nType: Enchantment — Aura\nFirst printed set: Urza's Saga\nEnchant permanent\nYou control enchanted permanent.",
        ),
        "marvin-murderous-mimic" => (
            "Marvin, Murderous Mimic",
            "Mana cost: {2}\nType: Legendary Artifact Creature — Toy\nFirst printed set: Duskmourn: House of Horror\nPower/Toughness: 2/2\nMarvin has all activated abilities of creatures you control that don't have the same name as this creature.",
        ),
        "psychic-paper" => (
            "Psychic Paper",
            "Mana cost: {2}\nType: Artifact — Equipment\nFirst printed set: Doctor Who\nAs this Equipment becomes attached to a creature, choose a creature card name and a creature type.\nEquipped creature has ward {1}, it can't be blocked, and its name and creature type are the last chosen name and creature type.\nEquip {2}",
        ),
        "magnetic-theft" => (
            "Magnetic Theft",
            "Mana cost: {R}\nType: Instant\nFirst printed set: Fifth Dawn\nAttach target Equipment to target creature. (Control of the Equipment doesn't change.)",
        ),
        "storm-of-souls" => (
            "Storm of Souls",
            "Mana cost: {4}{W}{W}\nType: Sorcery\nFirst printed set: Crimson Vow Commander\nReturn all creature cards from your graveyard to the battlefield. Each of them is a 1/1 Spirit with flying in addition to its other types. Exile Storm of Souls.",
        ),
        "containment-priest" => (
            "Containment Priest",
            "Mana cost: {1}{W}\nType: Creature — Human Cleric\nFirst printed set: Commander 2014\nPower/Toughness: 2/2\nFlash\nIf a nontoken creature would enter and it wasn't cast, exile it instead.",
        ),
        "garruk-s-harbinger" => (
            "Garruk's Harbinger",
            "Mana cost: {1}{G}{G}\nType: Creature — Beast\nFirst printed set: Core Set 2021\nPower/Toughness: 4/3\nHexproof from black\nWhenever this creature deals combat damage to a player or planeswalker, look at that many cards from the top of your library. You may reveal a creature card or Garruk planeswalker card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
        "black-knight" => (
            "Black Knight",
            "Mana cost: {B}{B}\nType: Creature — Human Knight\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 2/2\nFirst strike (This creature deals combat damage before creatures without first strike.)\nProtection from white (This creature can't be blocked, targeted, dealt damage, or enchanted by anything white.)",
        ),
        "escaped-shapeshifter" => (
            "Escaped Shapeshifter",
            "Mana cost: {3}{U}{U}\nType: Creature — Shapeshifter\nFirst printed set: Tempest\nPower/Toughness: 3/4\nAs long as an opponent controls a creature with flying not named Escaped Shapeshifter, this creature has flying. The same is true for first strike, trample, and protection from any color.",
        ),
        "mind-bend" => (
            "Mind Bend",
            "Mana cost: {U}\nType: Instant\nFirst printed set: Mirage\nChange the text of target permanent by replacing all instances of one color word with another or one basic land type with another. (For example, you may change \"nonblack creature\" to \"nongreen creature\" or \"forestwalk\" to \"islandwalk.\" This effect lasts indefinitely.)",
        ),
        "swords-to-plowshares" => (
            "Swords to Plowshares",
            "Mana cost: {W}\nType: Instant\nFirst printed set: Limited Edition Alpha\nExile target creature. Its controller gains life equal to its power.",
        ),
        "murder" => (
            "Murder",
            "Mana cost: {1}{B}{B}\nType: Instant\nFirst printed set: Magic 2013\nDestroy target creature.",
        ),
        "indominus-rex-alpha" => (
            "Indominus Rex, Alpha",
            "Mana cost: {1}{U/B}{U/B}{G}{G}\nType: Legendary Creature — Dinosaur Mutant\nPower/Toughness: 6/6\nAs Indominus Rex enters, discard any number of creature cards. It enters with a flying counter on it if a card discarded this way has flying. The same is true for first strike, double strike, deathtouch, hexproof, haste, indestructible, lifelink, menace, reach, trample, and vigilance.\nWhen Indominus Rex enters, draw a card for each counter on it.",
        ),
        "breaker-of-creation" => (
            "Breaker of Creation",
            "Mana cost: {6}{C}{C}\nType: Creature — Eldrazi\nFirst printed set: Modern Horizons 3\nPower/Toughness: 8/4\nWhen you cast this spell, you gain 1 life for each colorless permanent you control.\nHexproof from each color\nAnnihilator 2 (Whenever this creature attacks, defending player sacrifices two permanents of their choice.)",
        ),
        "tam-mindful-first-year" => (
            "Tam, Mindful First-Year",
            "Mana cost: {1}{G/U}\nType: Legendary Creature — Gorgon Wizard\nFirst printed set: Lorwyn Eclipsed\nPower/Toughness: 2/2\nEach other creature you control has hexproof from each of its colors.\n{T}: Target creature you control becomes all colors until end of turn.",
        ),
        "baleful-strix" => (
            "Baleful Strix",
            "Mana cost: {U}{B}\nType: Artifact Creature — Bird\nFirst printed set: Planechase 2012\nPower/Toughness: 1/1\nFlying, deathtouch\nWhen this creature enters, draw a card.",
        ),
        "pongify" => (
            "Pongify",
            "Mana cost: {U}\nType: Instant\nFirst printed set: Planar Chaos\nDestroy target creature. It can't be regenerated. Its controller creates a 3/3 green Ape creature token.",
        ),
        "iridescent-angel" => (
            "Iridescent Angel",
            "Mana cost: {5}{W}{U}\nType: Creature — Angel\nFirst printed set: Odyssey\nPower/Toughness: 4/4\nFlying, protection from each color",
        ),
        "pledge-of-loyalty" => (
            "Pledge of Loyalty",
            "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has protection from each color among permanents you control. This effect doesn't remove this Aura.",
        ),
        "concerted-effort" => (
            "Concerted Effort",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nFirst printed set: Ravnica: City of Guilds\nAt the beginning of each upkeep, creatures you control gain flying until end of turn if a creature you control has flying. The same is true for fear, first strike, double strike, landwalk, protection, trample, and vigilance.",
        ),
        "war-room" => (
            "War Room",
            "Type: Land\n{T}: Add {C}.\n{3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card.",
        ),
        "commander-s-plate" => (
            "Commander's Plate",
            "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature gets +3/+3 and has protection from each color that's not in your commander's color identity.\nEquip commander {3}\nEquip {5}",
        ),
        "path-of-ancestry" => (
            "Path of Ancestry",
            "Type: Land\nThis land enters tapped.\n{T}: Add one mana of any color in your commander's color identity. When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1. (Look at the top card of your library. You may put that card on the bottom.)",
        ),
        "alpine-watchdog" => (
            "Alpine Watchdog",
            "Mana cost: {1}{W}\nType: Creature — Dog\nPower/Toughness: 2/2\nVigilance (Attacking doesn't cause this creature to tap.)",
        ),
        "go-shintai-of-life-s-origin" => (
            "Go-Shintai of Life's Origin",
            "Mana cost: {3}{G}\nType: Legendary Enchantment Creature — Shrine\nPower/Toughness: 3/4\n{W}{U}{B}{R}{G}, {T}: Return target enchantment card from your graveyard to the battlefield.\nWhenever Go-Shintai of Life's Origin or another nontoken Shrine you control enters, create a 1/1 colorless Shrine enchantment creature token.",
        ),
        "go-shintai-of-ancient-wars" => (
            "Go-Shintai of Ancient Wars",
            "Mana cost: {2}{R}\nType: Legendary Enchantment Creature — Shrine\nPower/Toughness: 2/2\nFirst strike\nAt the beginning of your end step, you may pay {1}. When you do, Go-Shintai of Ancient Wars deals X damage to target player or planeswalker, where X is the number of Shrines you control.",
        ),
        "traxos-scourge-of-kroog" => (
            "Traxos, Scourge of Kroog",
            "Mana cost: {4}\nType: Legendary Artifact Creature — Dragon Construct\nPower/Toughness: 7/7\nTrample\nTraxos enters tapped and doesn't untap during your untap step.\nWhenever you cast a historic spell, untap Traxos. (Artifacts, legendaries, and Sagas are historic.)",
        ),
        "bruna-the-fading-light" => (
            "Bruna, the Fading Light",
            "Mana cost: {5}{W}{W}\nType: Legendary Creature — Angel Horror\nPower/Toughness: 5/7\nWhen you cast this spell, you may return target Angel or Human creature card from your graveyard to the battlefield.\nFlying, vigilance\n(Melds with Gisela, the Broken Blade.)",
        ),
        "gisela-the-broken-blade" => (
            "Gisela, the Broken Blade",
            "Mana cost: {2}{W}{W}\nType: Legendary Creature — Angel Horror\nPower/Toughness: 4/3\nFlying, first strike, lifelink\nAt the beginning of your end step, if you both own and control Gisela and a creature named Bruna, the Fading Light, exile them, then meld them into Brisela, Voice of Nightmares.",
        ),
        "brisela-voice-of-nightmares" => (
            "Brisela, Voice of Nightmares",
            "Type: Legendary Creature — Eldrazi Angel\nPower/Toughness: 9/10\nFlying, first strike, vigilance, lifelink\nYour opponents can't cast spells with mana value 3 or less.",
        ),
        "brokkos-apex-of-forever" => (
            "Brokkos, Apex of Forever",
            "Mana cost: {2}{B}{G}{U}\nType: Legendary Creature — Nightmare Beast Elemental\nPower/Toughness: 6/6\nMutate {2}{U/B}{G}{G} (If you cast this spell for its mutate cost, put it over or under target non-Human creature you own. They mutate into the creature on top plus all abilities from under it.)\nTrample\nYou may cast this card from your graveyard using its mutate ability.",
        ),
        "tymna-the-weaver" => (
            "Tymna the Weaver",
            "Mana cost: {1}{W}{B}\nType: Legendary Creature — Human Cleric\nPower/Toughness: 2/2\nLifelink\nAt the beginning of each of your postcombat main phases, you may pay X life, where X is the number of opponents that were dealt combat damage this turn. If you do, draw X cards.\nPartner (You can have two commanders if both have partner.)",
        ),
        "thrasios-triton-hero" => (
            "Thrasios, Triton Hero",
            "Mana cost: {G}{U}\nType: Legendary Creature — Merfolk Wizard\nPower/Toughness: 1/3\n{4}: Scry 1, then reveal the top card of your library. If it's a land card, put it onto the battlefield tapped. Otherwise, draw a card.\nPartner (You can have two commanders if both have partner.)",
        ),
        "cactus-preserve" => (
            "Cactus Preserve",
            "Type: Land — Desert\nThis land enters tapped.\n{T}: Add one mana of any type that a land you control could produce.\n{3}: Until end of turn, this land becomes an X/X green Plant creature with reach, where X is the greatest mana value among your commanders. It's still a land.",
        ),
        "tangleweave-armor" => (
            "Tangleweave Armor",
            "Mana cost: {2}{G}{G}\nType: Artifact — Equipment\nLiving weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)\nEquipped creature gets +X/+X, where X is the greatest mana value among your commanders.\nEquip {4}",
        ),
        "cloudkill" => (
            "Cloudkill",
            "Mana cost: {4}{B}{B}\nType: Sorcery\nAll creatures get -X/-X until end of turn, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.",
        ),
        "colossal-dreadmaw" => (
            "Colossal Dreadmaw",
            "Mana cost: {4}{G}{G}\nType: Creature — Dinosaur\nPower/Toughness: 6/6\nTrample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)",
        ),
        "ral-s-outburst" => (
            "Ral's Outburst",
            "Mana cost: {2}{U}{R}\nType: Instant\nRal's Outburst deals 3 damage to any target. Look at the top two cards of your library. Put one of them into your hand and the other into your graveyard.",
        ),
        "embodiment-of-agonies" => (
            "Embodiment of Agonies",
            "Mana cost: {1}{B}{B}\nType: Creature — Demon\nPower/Toughness: 0/0\nFlying, deathtouch\nThis creature enters with a +1/+1 counter on it for each different mana cost among nonland cards in your graveyard.",
        ),
        "endless-one" => (
            "Endless One",
            "Mana cost: {X}\nType: Creature — Eldrazi\nPower/Toughness: 0/0\nThis creature enters with X +1/+1 counters on it.",
        ),
        "phyrexian-metamorph" => (
            "Phyrexian Metamorph",
            "Mana cost: {3}{U/P}\nType: Artifact Creature — Phyrexian Shapeshifter\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any artifact or creature on the battlefield, except it's an artifact in addition to its other types.",
        ),
        "opalescence" => (
            "Opalescence",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nEach other non-Aura enchantment is a creature in addition to its other types and has base power and base toughness each equal to its mana value.",
        ),
        "light-from-within" => (
            "Light from Within",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nChroma — Each creature you control gets +1/+1 for each white mana symbol in its mana cost.",
        ),
        "divinity-of-pride" => (
            "Divinity of Pride",
            "Mana cost: {W/B}{W/B}{W/B}{W/B}{W/B}\nType: Creature — Spirit Avatar\nPower/Toughness: 4/4\nFlying, lifelink\nThis creature gets +4/+4 as long as you have 25 or more life.",
        ),
        "sleight-of-mind" => (
            "Sleight of Mind",
            "Mana cost: {U}\nType: Instant\nChange the text of target spell or permanent by replacing all instances of one color word with another.",
        ),
        "abrupt-decay" => (
            "Abrupt Decay",
            "Mana cost: {B}{G}\nType: Instant\nThis spell can't be countered.\nDestroy target nonland permanent with mana value 3 or less.",
        ),
        "ritual-of-soot" => (
            "Ritual of Soot",
            "Mana cost: {2}{B}{B}\nType: Sorcery\nDestroy all creatures with mana value 3 or less.",
        ),
        "favor-of-the-mighty" => (
            "Favor of the Mighty",
            "Mana cost: {1}{W}\nType: Kindred Enchantment — Giant\nEach creature with the greatest mana value has protection from each color.",
        ),
        "soul-shatter" => (
            "Soul Shatter",
            "Mana cost: {2}{B}\nType: Instant\nEach opponent sacrifices a creature or planeswalker with the greatest mana value among creatures and planeswalkers they control.",
        ),
        "visions-of-glory" => (
            "Visions of Glory",
            "Mana cost: {4}{W}\nType: Sorcery\nCreate a 1/1 white Human creature token for each creature you control.\nFlashback {8}{W}{W}. This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone. (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "rush-of-knowledge" => (
            "Rush of Knowledge",
            "Mana cost: {4}{U}\nType: Sorcery\nDraw cards equal to the greatest mana value among permanents you control.",
        ),
        "glint-raker" => (
            "Glint Raker",
            "Mana cost: {3}{U}\nType: Creature — Drake\nPower/Toughness: 1/3\nFlying\nThis creature gets +X/+0, where X is the greatest mana value among artifacts you control.\nWhenever this creature deals combat damage to a player, you may reveal that many cards from the top of your library. Put an artifact card revealed this way into your hand and the rest into your graveyard.",
        ),
        "myr-battlesphere" => (
            "Myr Battlesphere",
            "Mana cost: {7}\nType: Artifact Creature — Myr Construct\nPower/Toughness: 4/7\nWhen this creature enters, create four 1/1 colorless Myr artifact creature tokens.\nWhenever this creature attacks, you may tap X untapped Myr you control. If you do, this creature gets +X/+0 until end of turn and deals X damage to the player or planeswalker it's attacking.",
        ),
        _ => panic!("unknown regression fixture {route}"),
    };
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, block, false).unwrap()
}

#[derive(Default)]
struct Menu(Vec<String>);
impl DecisionMaker for Menu {
    fn decide_options(&mut self, _: &GameState, options: &SelectOptionsContext) -> Vec<usize> {
        self.0.extend(
            options
                .options
                .iter()
                .filter(|o| o.legal)
                .map(|o| o.description.clone()),
        );
        vec![options.options[0].index]
    }
}
#[test]
fn formidable_counts_negative_power() {
    let mut g = game();
    let herd = g.create_object_from_definition(&card("stampeding-elk-herd"), A, Zone::Battlefield);
    creature(&mut g, "Another attacker", 3, 3);
    let weak = creature(&mut g, "Weakened creature", -2, 3);
    assert_eq!(g.current_power(weak), Some(-2));
    let ctx = EffectContext::new_default(herd, A);
    let abilities = g.current_abilities(herd).unwrap();
    let condition = abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => t.intervening_if.as_ref(),
            _ => None,
        })
        .unwrap();
    assert!(
        !evaluate_condition_resolution(&g, condition, &ctx).unwrap(),
        "5 + 3 - 2 = 6, below eight"
    );
}

#[test]
fn berserkers_frenzy_ignores_lower_die() {
    let mut g = game();
    let s = g.new_object_id();
    g.force_next_die_roll(3);
    g.force_next_die_roll(18);
    let out = RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 20)
        .with_ignore_lower(true)
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(out.as_count(), Some(18));
    assert!(
        !out.execution_facts
            .iter()
            .any(|fact| matches!(fact, ironsmith::effect::ExecutionFact::OtherNumber(_)))
    );
    assert_eq!(
        g.turn_store.turn_history.completed_die_roll_count(A),
        1,
        "only the 18 counts"
    );
}

#[test]
fn berserkers_frenzy_roller_breaks_tie() {
    let mut g = game();
    let s = g.new_object_id();
    g.force_next_die_roll(8);
    g.force_next_die_roll(8);
    let mut dm = Menu::default();
    RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 20)
        .with_ignore_lower(true)
        .execute(&mut g, &mut EffectContext::new(s, A, &mut dm))
        .unwrap();
    assert!(
        !dm.0.is_empty(),
        "two eights require the roller to select which to ignore"
    );
}

#[test]
fn neverwinter_hydra_rolls_the_chosen_number_of_dice() {
    let mut g = game();
    let s = g.new_object_id();
    RollDiceChooseResultEffect::new(PlayerFilter::You, 4097, 6)
        .with_total(None)
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 4097);
}

#[test]
fn all_requested_coin_flips_take_place() {
    let mut g = game();
    let s = g.new_object_id();
    let mut e = FlipCoinEffect::face_only(PlayerFilter::You);
    e.count = 4097;
    e.execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 4097);
}

#[test]
fn token_creation_uses_the_full_count_unless_a_simulation_cap_is_configured() {
    let cat = CardDefinitionBuilder::new(CardId::new(), "Cat")
        .token()
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    for (cap, expected) in [(None, 513), (Some(512), 512)] {
        let mut g = game();
        if let Some(cap) = cap {
            g.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits {
                max_tokens_per_name_per_batch: cap,
                ..Default::default()
            });
        }
        let source = g.new_object_id();
        let mut ctx = EffectContext::new_default(source, A);
        let outcome = ironsmith::effects::CreateTokenEffect::you(cat.clone(), 513)
            .execute(&mut g, &mut ctx).unwrap();
        assert_eq!(outcome.result_objects().unwrap().len(), expected);
        ironsmith::effects::CreateTokenEffect::you(cat.clone(), 1).execute(&mut g, &mut ctx).unwrap();
        assert_eq!(g.battlefield.len(), expected + 1);
    }
}

#[test]
fn each_exalted_counter_grants_an_ability() {
    let mut g = game();
    let s = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("exalted".into());
    g.add_counters(s, kind, 4097).unwrap();
    assert_eq!(g.counter_count(s, kind), 4097);
    let triggers = g
        .current_abilities(s)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
        .count();
    assert_eq!(triggers, 4097);
}

#[test]
fn oversized_ability_counter_placement_fails_without_changing_the_recipient() {
    let mut g = game();
    let source = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("exalted".into());
    g.add_counters(source, kind, 1).unwrap();
    let effect = ironsmith::Effect::put_counters_on_source(kind, i32::MAX);
    let error = ironsmith::effects::execute_effect(
        &mut g,
        &effect,
        &mut EffectContext::new_default(source, A),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. }
    ));
    assert_eq!(g.counter_count(source, kind), 1);
}

#[test]
fn admitting_a_counter_ability_preflights_existing_counters() {
    let mut g = game();
    let source = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("fixture".into());
    g.add_counters(source, kind, 100_000).unwrap();
    let error = g
        .register_named_ability_counter(kind, vec![ironsmith::ability::flying()])
        .unwrap_err();
    assert!(error.contains("ability counter occurrences"));
    assert_eq!(g.counter_count(source, kind), 100_000);
    assert!(g.current_abilities(source).unwrap().is_empty());
}

#[test]
fn a_sacrificed_creature_with_no_mana_cost_has_zero_red_symbols() {
    let mut g = game();
    let source = g.create_object_from_definition(&card("fiery-bombardment"), A, Zone::Battlefield);
    let token = creature(&mut g, "Costless creature", 1, 1);
    let mut ctx = EffectContext::new_default(source, A);
    ctx.tag_object(
        "__original_sacrifice_cost_0",
        ironsmith::UnifiedObjectSnapshot::from_object_id(&g, token).unwrap(),
    );
    g.move_object_by_effect(token, Zone::Graveyard);
    let v = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ironsmith::ChooseSpec::tagged("__original_sacrifice_cost_0")),
        color: ironsmith::Color::Red,
    };
    assert_eq!(
        ironsmith::effects::resolve_value(&g, &v, &ctx).map_err(|e| format!("{e:?}")),
        Ok(0)
    );
}

#[test]
fn five_coin_flips_with_eleven_thumbs_keep_five_results() {
    let mut g = game();
    g.create_object_from_definition(&card("mirror-box"), A, Zone::Battlefield);
    let thumb = card("krark-s-thumb");
    for _ in 0..11 {
        g.create_object_from_definition(&thumb, A, Zone::Battlefield);
    }
    let s = g.create_object_from_definition(&card("ral-zarek"), A, Zone::Battlefield);
    let mut effect = FlipCoinEffect::face_only(PlayerFilter::You);
    effect.count = 5;
    effect
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 5);
}

#[test]
fn an_interchangeable_printing_must_meet_a_card_name_choices_type_restriction() {
    struct ChoosePrinting;
    impl DecisionMaker for ChoosePrinting {
        fn decide_text(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TextInputContext,
        ) -> String {
            "Chun-Li, Countless Kicks".into()
        }
    }
    for (filter, expected) in [
        (ironsmith::ObjectFilter::artifact(), false),
        (ironsmith::ObjectFilter::creature(), true),
    ] {
        let mut g = game();
        let source = creature(&mut g, "Chooser", 1, 1);
        let effect = ironsmith::effects::ChooseCardNameEffect::new(
            PlayerFilter::You,
            Some(filter),
            "chosen",
        );
        let result = effect.execute(
            &mut g,
            &mut EffectContext::new(source, A, &mut ChoosePrinting),
        );
        if expected {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(ironsmith::effects::ExecutionError::Impossible(reason))
                if reason.contains("invalid card name")));
        }
        assert_eq!(g.chosen_named_option(source).is_some(), expected);
    }
}

#[test]
fn interchangeable_legendary_printings_invoke_the_legend_rule() {
    let mut g = game();
    let mut d = card("zethi-arcane-blademaster");
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    d.card.name = "Chun-Li, Countless Kicks".to_string();
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    assert_eq!(
        ironsmith::rules::state_based::get_legend_rule_specs(&g).len(),
        1
    );
}

#[test]
fn five_token_board_wipes_spare_the_new_creatures() {
    let mut wrong = vec![];
    for (route, subtype) in [
        ("martial-coup", ironsmith::Subtype::Soldier),
        ("white-sun-s-twilight", ironsmith::Subtype::Mite),
    ] {
        for x in [4, 5] {
            let mut g = game();
            let old = creature(&mut g, "Old creature", 2, 2);
            let id = g.create_object_from_definition(&card(route), A, Zone::Stack);
            let mut entry = ironsmith::StackEntry::new(id, A);
            g.object_mut(id).unwrap().x_value = Some(x as u32);
            entry.x_value = Some(x);
            g.push_to_stack(entry);
            ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
            assert_eq!(g.battlefield.contains(&old), x < 5);
            let soldiers = g
                .battlefield
                .iter()
                .filter(|&&s| g.current_has_subtype(s, subtype))
                .count();
            if soldiers != x as usize {
                wrong.push(format!("{route} at X={x}: {soldiers} surviving tokens"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join(", "));
}

#[test]
fn abyssal_harvester_keeps_the_new_nightmare() {
    let mut g = game();
    let d = card("abyssal-harvester");
    let harvester = g.create_object_from_definition(&d, A, Zone::Battlefield);
    let program = d
        .abilities
        .into_iter()
        .find_map(|a| match a.kind {
            AbilityKind::Activated(a) => Some(a.effects),
            _ => None,
        })
        .unwrap();
    let mut previous_nightmare = None;
    for name in ["First Bear", "Second Bear"] {
        let bear = creature(&mut g, name, 2, 2);
        let grave = g.move_object_by_effect(bear, Zone::Graveyard).unwrap();
        let mut entry = ironsmith::StackEntry::ability(harvester, A, program.clone());
        entry.targets.push(ironsmith::Target::Object(grave));
        g.push_to_stack(entry);
        ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
        assert!(
            g.exile
                .iter()
                .any(|&id| g.object(id).is_some_and(|o| o.name == name))
        );
        let copies: Vec<_> = g
            .battlefield
            .iter()
            .copied()
            .filter(|&id| g.current_has_subtype(id, ironsmith::Subtype::Nightmare))
            .collect();
        assert_eq!(
            copies.len(),
            1,
            "each activation leaves exactly its own Nightmare"
        );
        assert_eq!(g.object(copies[0]).unwrap().name, name);
        assert_ne!(previous_nightmare.replace(copies[0]), Some(copies[0]));
    }
}

#[test]
fn ixidron_cannot_turn_graf_rats_face_down() {
    let mut g = game();
    let rats = g.create_object_from_definition(&card("graf-rats"), A, Zone::Battlefield);
    let bear = g.create_object_from_definition(&card("grizzly-bears"), A, Zone::Battlefield);
    let id = g.create_object_from_definition(&card("ixidron"), A, Zone::Stack);
    g.push_to_stack(ironsmith::StackEntry::new(id, A));
    ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
    assert!(g.is_face_down(bear), "ordinary creature turns face down");
    assert!(
        !g.is_face_down(rats),
        "face-up Graf Rats must survive Ixidron's instruction face up"
    );
}

#[test]
fn creature_type_choices_are_independent_of_an_opponents_library() {
    fn menu(g: &mut GameState, id: ObjectId) -> Vec<String> {
        let mut dm = Menu::default();
        ChooseCreatureTypeEffect::for_family(PlayerFilter::You, SubtypeFamily::Creature)
            .execute(g, &mut EffectContext::new(id, A, &mut dm))
            .unwrap();
        dm.0
    }
    let mut a = game();
    let id = creature(&mut a, "Source", 1, 1);
    let mut b = a.clone();
    a.create_object_from_definition(&card("island"), PlayerId(1), Zone::Library);
    b.create_object_from_definition(&card("kira-nerys-firebrand"), PlayerId(1), Zone::Library);
    let first = menu(&mut a, id);
    let second = menu(&mut b, id);
    assert_eq!(
        first.contains(&"Bajoran".to_string()),
        second.contains(&"Bajoran".to_string()),
        "substituting an unrevealed Kira for an Island changes Alice's menu"
    );
}

fn put(game: &mut GameState, route: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_definition(&card(route), owner, zone)
}

fn resolve_spell(
    game: &mut GameState,
    route: &str,
    targets: Vec<ironsmith::Target>,
    dm: &mut impl DecisionMaker,
) {
    let spell = put(game, route, A, Zone::Stack);
    let mut entry = ironsmith::StackEntry::new(spell, A);
    entry.targets = targets;
    game.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

fn can_cast(game: &GameState, card: ObjectId) -> bool {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    compute_legal_actions(game, A).unwrap().iter().any(
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card),
    )
}

#[test]
fn graveyard_casting_abilities_can_be_lost_and_regained_in_timestamp_order() {
    let opponent = PlayerId(1);
    for donor in ["underworld-breach", "past-in-flames"] {
        for grant_is_newer in [false, true] {
            let mut g = game();
            let printed = put(&mut g, "think-twice", A, Zone::Graveyard);
            let granted = put(&mut g, "divination", A, Zone::Graveyard);
            for _ in 0..4 {
                put(&mut g, "bayou-groff", A, Zone::Graveyard);
            }
            g.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Blue, 5);
            assert!(can_cast(&g, printed));
            assert!(!can_cast(&g, granted));
            let apply_grant = |g: &mut GameState| {
                if donor == "past-in-flames" {
                    resolve_spell(
                        g,
                        donor,
                        vec![],
                        &mut ironsmith::decision::SelectFirstDecisionMaker,
                    );
                } else {
                    put(g, donor, A, Zone::Battlefield);
                }
            };
            if !grant_is_newer {
                apply_grant(&mut g);
            }
            let jailer = put(&mut g, "yixlid-jailer", opponent, Zone::Battlefield);
            assert!(!can_cast(&g, printed), "printed flashback under Jailer");
            if grant_is_newer {
                apply_grant(&mut g);
            }
            assert_eq!(can_cast(&g, granted), grant_is_newer, "{donor}");
            g.phase_out(jailer);
            assert!(can_cast(&g, printed));
            assert!(can_cast(&g, granted));
        }
    }
}

#[test]
fn named_sources_in_hand_obey_mana_and_nonmana_prohibitions() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let mut g = game();
    let street = put(&mut g, "street-wraith", A, Zone::Hand);
    let guide = put(&mut g, "simian-spirit-guide", A, Zone::Hand);
    let needle = put(&mut g, "pithing-needle", PlayerId(1), Zone::Battlefield);
    g.set_chosen_named_option(needle, "Street Wraith".into());
    let actions = compute_legal_actions(&g, A).unwrap();
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == street))
    );
    g.set_chosen_named_option(needle, "Simian Spirit Guide".into());
    let actions = compute_legal_actions(&g, A).unwrap();
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == guide)
        )
    );
    let revoker = put(&mut g, "phyrexian-revoker", PlayerId(1), Zone::Battlefield);
    g.set_chosen_named_option(revoker, "Simian Spirit Guide".into());
    assert!(
        !compute_legal_actions(&g, A).unwrap().iter().any(
            |a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == guide)
        )
    );
}

#[test]
fn a_creatures_additional_casting_cost_counts_as_an_ability_until_removed() {
    let mut g = game();
    let groff = put(&mut g, "bayou-groff", A, Zone::Battlefield);
    put(&mut g, "muraganda-petroglyphs", A, Zone::Battlefield);
    assert_eq!(g.current_power(groff), Some(5));
    put(&mut g, "humility", A, Zone::Battlefield);
    assert_eq!(g.current_power(groff), Some(3));
}

#[test]
fn station_power_and_toughness_are_available_only_while_stationed() {
    let mut g = game();
    for zone in [
        Zone::Library,
        Zone::Hand,
        Zone::Stack,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Battlefield,
    ] {
        let id = put(&mut g, "borg-cube", A, zone);
        assert_eq!(
            (g.current_power(id), g.current_toughness(id)),
            (None, None),
            "{zone:?}"
        );
        if zone == Zone::Battlefield {
            g.add_counters(id, CounterType::Charge, 12);
            assert_eq!(
                (g.current_power(id), g.current_toughness(id)),
                (Some(6), Some(6))
            );
        }
    }
}

#[test]
fn reanimate_uses_zero_for_an_uncast_creatures_entry_x() {
    let mut g = game();
    let target = put(&mut g, "frankenstein-s-monster", A, Zone::Graveyard);
    resolve_spell(
        &mut g,
        "reanimate",
        vec![ironsmith::Target::Object(target)],
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    assert_eq!(g.player(A).unwrap().life, 18);
    let permanent = g
        .battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).unwrap().name.as_ref() == "Frankenstein's Monster")
        .unwrap();
    assert_eq!(
        (g.current_power(permanent), g.current_toughness(permanent)),
        (Some(0), Some(1))
    );
}

#[test]
fn another_player_excludes_the_chooser_before_transferring_control() {
    let mut g = game();
    let financier = put(&mut g, "discerning-financier", A, Zone::Battlefield);
    let treasure = CardDefinitionBuilder::new(CardId::new(), "Treasure")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::Subtype::Treasure])
        .build();
    let t = g.create_object_from_definition(&treasure, A, Zone::Battlefield);
    put(&mut g, "divination", A, Zone::Library);
    let program = g
        .current_abilities(financier)
        .unwrap()
        .into_iter()
        .find_map(|a| match a.kind {
            AbilityKind::Activated(a) => Some(a.effects),
            _ => None,
        })
        .unwrap();
    let mut entry = ironsmith::StackEntry::ability(financier, A, program);
    entry.targets = vec![ironsmith::Target::Object(t)];
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(
        &mut g,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(g.current_controller(t), Some(PlayerId(1)));
    assert_eq!(g.player(A).unwrap().hand.len(), 1);
}

#[derive(Default)]
struct MultiplayerDecisions {
    voters: Vec<PlayerId>,
    bidders: Vec<PlayerId>,
}

impl DecisionMaker for MultiplayerDecisions {
    fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.voters.push(c.player);
        vec![c.options.iter().find(|o| o.legal).unwrap().index]
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.bidders.push(c.player);
        false
    }
    fn decide_proliferate(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        ironsmith::decisions::specs::ProliferateResponse {
            permanents: c.eligible_permanents.iter().map(|p| p.0).collect(),
            players: c.eligible_players.iter().map(|p| p.0).collect(),
        }
    }
}

#[test]
fn multiplayer_choices_and_characteristics_respect_range_and_turn_direction() {
    let mut g = GameState::new(
        ["A", "B", "C", "D", "E"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        20,
    );
    let seats: Vec<_> = (0..5).map(PlayerId).collect();
    g.enable_limited_range_of_influence(seats.clone(), vec![1; 5])
        .unwrap();
    g.turn.active_player = A;
    g.turn_store.turn_order = seats.iter().copied().rev().collect();
    let c = PlayerId(2);
    let golem = put(&mut g, "altar-golem", A, Zone::Battlefield);
    let power = g.current_power(golem);
    let distant = put(&mut g, "bayou-groff", c, Zone::Battlefield);
    assert_eq!(g.current_power(golem), power);
    g.add_counters(distant, CounterType::PlusOnePlusOne, 1);
    g.player_mut(c).unwrap().poison_counters = 1;
    g.player_mut(A).unwrap().poison_counters = 1;
    for _ in 0..9 {
        put(&mut g, "divination", A, Zone::Library);
    }
    let mut dm = MultiplayerDecisions::default();
    resolve_spell(&mut g, "contentious-plan", vec![], &mut dm);
    assert_eq!(g.player(A).unwrap().poison_counters, 2);
    assert_eq!(g.player(c).unwrap().poison_counters, 1);
    assert_eq!(g.current_power(distant), Some(6));
    resolve_spell(&mut g, "plea-for-power", vec![], &mut dm);
    assert_eq!(dm.voters, [A, PlayerId(4), PlayerId(1)]);
    resolve_spell(&mut g, "pain-s-reward", vec![], &mut dm);
    assert!(dm.bidders.contains(&PlayerId(1)));
    assert!(!dm.bidders.contains(&c) && !dm.bidders.contains(&PlayerId(3)));
}

#[path = "november_2026_rules/characteristic_interactions.rs"]
mod characteristic_interactions;

#[path = "november_2026_rules/expanded_interactions.rs"]
mod expanded_interactions;

#[path = "november_2026_rules/gameplay_audit.rs"]
mod gameplay_audit;

fn enter_from_hand(g: &mut GameState, route: &str, dm: &mut impl DecisionMaker) -> ObjectId {
    let id = put(g, route, A, Zone::Hand);
    let stable = g.object(id).unwrap().stable_id;
    let result = g
        .move_object_with_etb_processing_with_dm(id, Zone::Battlefield, dm)
        .unwrap();
    assert!(!result.pending && result.programs.is_empty());
    let id = g.find_object_by_stable_id(stable).unwrap();
    assert_eq!(g.object(id).unwrap().zone, Zone::Battlefield);
    id
}

#[test]
fn magistrate_allows_a_spell_cast_from_its_owners_hand() {
    let mut g = game();
    put(
        &mut g,
        "drannith-magistrate",
        PlayerId(1),
        Zone::Battlefield,
    );
    let spell = put(&mut g, "grizzly-bears", A, Zone::Hand);
    g.player_mut(A).unwrap().mana_pool.green = 2;
    assert!(can_cast(&g, spell));
}

#[test]
fn saga_entry_counters_follow_the_intrinsic_replacement() {
    for (modifier, saga, count) in [
        ("doubling-season", "the-eldest-reborn", 2),
        ("blood-moon", "urza-s-saga", 0),
    ] {
        let mut g = game();
        put(&mut g, modifier, A, Zone::Battlefield);
        let id = enter_from_hand(
            &mut g,
            saga,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        assert_eq!(g.counter_count(id, CounterType::Lore), count, "{modifier}");
    }
}

#[test]
fn amplify_uses_current_creature_types_for_both_cards() {
    struct Reveal;
    impl DecisionMaker for Reveal {
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(ctx.max.unwrap_or(usize::MAX))
                .map(|c| c.id)
                .collect()
        }
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }
    for (candidate, adapt) in [("nameless-inversion", false), ("grizzly-bears", true)] {
        let mut g = game();
        if adapt {
            let enchantment = put(&mut g, "arcane-adaptation", A, Zone::Battlefield);
            g.set_chosen_creature_type(enchantment, ironsmith::Subtype::Elf);
        }
        put(&mut g, candidate, A, Zone::Hand);
        let id = enter_from_hand(&mut g, "kilnmouth-dragon", &mut Reveal);
        assert_eq!(
            g.counter_count(id, CounterType::PlusOnePlusOne),
            3,
            "{candidate}"
        );
    }
}

#[test]
fn thalia_taps_an_animated_basic_land() {
    let mut g = game();
    put(
        &mut g,
        "thalia-heretic-cathar",
        PlayerId(1),
        Zone::Battlefield,
    );
    put(&mut g, "living-plane", A, Zone::Battlefield);
    let id = enter_from_hand(
        &mut g,
        "island",
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    assert!(g.is_tapped(id));
}

#[test]
fn a_land_that_can_add_a_color_or_colorless_can_pay_a_colorless_pip() {
    let mut g = game();
    let gate = put(&mut g, "mystic-gate", A, Zone::Battlefield);
    g.tap(gate);
    put(&mut g, "reflecting-pool", A, Zone::Battlefield);
    creature(&mut g, "Target", 2, 2);
    let spell = put(&mut g, "eldritch-immunity", A, Zone::Hand);
    assert!(can_cast(&g, spell));
}

#[test]
fn mana_type_filters_use_rewritten_production_bundles() {
    for (land, modifier, expected) in [
        ("azorius-chancery", "damping-sphere", true),
        ("wastes", "contamination", false),
    ] {
        let mut g = game();
        put(&mut g, modifier, A, Zone::Battlefield);
        let id = put(&mut g, land, A, Zone::Battlefield);
        let stable = g.object(id).unwrap().stable_id;
        resolve_spell(
            &mut g,
            "break-the-ice",
            vec![ironsmith::Target::Object(id)],
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        let after = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            g.object(after).unwrap().zone == Zone::Graveyard,
            expected,
            "{land}"
        );
    }
}

#[test]
fn casting_requires_a_complete_group_of_targets_with_the_same_controller() {
    let mut g = game();
    g.player_mut(A).unwrap().mana_pool.black = 2;
    put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    put(&mut g, "grizzly-bears", PlayerId(1), Zone::Battlefield);
    let spell = put(&mut g, "cannibalize", A, Zone::Hand);
    assert!(!can_cast(&g, spell));
    put(
        &mut g,
        "drannith-magistrate",
        PlayerId(1),
        Zone::Battlefield,
    );
    assert!(can_cast(&g, spell));
}

#[test]
fn potential_mana_includes_restricted_activations_and_triggered_production() {
    use ironsmith::ManaSymbol::{Black, Blue, Colorless, Green, Red, White};
    use ironsmith::ability::ActivatedAbilityRuntimeExt;

    for donor in ["spire-of-industry", "crumbling-vestige"] {
        let mut g = game();
        let land = put(&mut g, donor, A, Zone::Battlefield);
        g.tap(land);
        let pool = put(&mut g, "reflecting-pool", A, Zone::Battlefield);
        let ability = g
            .current_abilities(pool)
            .unwrap()
            .into_iter()
            .find_map(|ability| match ability.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let types = ability.inferred_mana_symbols(&g, pool, A);
        for symbol in [White, Blue, Black, Red, Green, Colorless] {
            assert!(types.contains(&symbol), "{donor} could produce {symbol:?}");
        }
    }
}

#[test]
fn sacrificed_land_mana_uses_its_last_abilities_with_current_output_quantities_and_replacements() {
    use ironsmith::effects::AddManaOfLandProducedTypesEffect;
    for (donor, contaminated, black) in [("gaea-s-cradle", false, 0), ("island", true, 1)] {
        let mut g = game();
        let source = put(&mut g, "sol-ring", A, Zone::Battlefield);
        if contaminated {
            put(&mut g, "contamination", A, Zone::Battlefield);
        }
        let land = put(&mut g, donor, A, Zone::Battlefield);
        let info = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            g.object(land).unwrap(),
            &g,
        );
        g.move_object_by_effect(land, Zone::Graveyard).unwrap();
        let mut ctx = EffectContext::new_default(source, A);
        ctx.tag_object("sacrificed", info);
        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ironsmith::ObjectFilter::tagged("sacrificed"),
            true,
            false,
        );
        effect.execute(&mut g, &mut ctx).unwrap();
        let pool = &g.player(A).unwrap().mana_pool;
        assert_eq!(u64::from(pool.total()), black, "{donor}");
        assert_eq!(pool.black, black, "{donor}");
    }
}

#[path = "november_2026_rules/damage_and_stack.rs"]
mod damage_and_stack;

#[path = "november_2026_rules/zone_and_name_rules.rs"]
mod zone_and_name_rules;

#[path = "november_2026_rules/zone_trigger_regressions.rs"]
mod zone_trigger_regressions;

#[path = "november_2026_rules/library_search.rs"]
mod library_search;

#[path = "november_2026_rules/copy_and_multiplayer.rs"]
mod copy_and_multiplayer;

#[path = "november_2026_rules/simultaneous_scry.rs"]
mod simultaneous_scry;

#[path = "november_2026_rules/additional_name_searches.rs"]
mod additional_name_searches;
#[path = "november_2026_rules/repeated_name_search.rs"]
mod repeated_name_search;
#[path = "november_2026_rules/searches_for_lost_permanents.rs"]
mod searches_for_lost_permanents;
#[path = "november_2026_rules/simultaneous_search.rs"]
mod simultaneous_search;

#[path = "november_2026_rules/owner_library_shuffles.rs"]
mod owner_library_shuffles;

#[path = "november_2026_rules/combat_roles.rs"]
mod combat_roles;

#[path = "november_2026_rules/graveyard_order.rs"]
mod graveyard_order;

#[path = "november_2026_rules/commander_designation.rs"]
mod commander_designation;

#[path = "november_2026_rules/mana_symbols.rs"]
mod mana_symbols;
