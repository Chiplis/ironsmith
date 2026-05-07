import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/GristVoraciousLarvaTest.java",
  "tests": [
    {
      "name": "test_Unearth_Trigger_NoMana",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unearth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unearth",
          "target": "Grist, Voracious Larva"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Unearth_Trigger_Pay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unearth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unearth",
          "target": "Grist, Voracious Larva"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Unearth_Trigger_NoPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unearth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unearth",
          "target": "Grist, Voracious Larva"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Bloodghast_Trigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bloodghast",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        }
      ]
    },
    {
      "name": "test_CastBloodghast_NoTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloodghast",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloodghast"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bloodghast",
          "count": 1
        }
      ]
    },
    {
      "name": "test_CastGravecrawler_Trigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gravecrawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gravecrawler",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Cast_NonCreature_NoTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Muldrotha, the Gravetide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mox Jet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mox Jet"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mox Jet",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Play_DryadArbor_FromGraveyard_Trigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, Voracious Larva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Muldrotha, the Gravetide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dryad Arbor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Land"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        }
      ]
    },
    {
      "name": "test_PlusOne_NoLibrary",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Insect Token",
          "counter": "DEATHTOUCH",
          "count": 0
        }
      ]
    },
    {
      "name": "test_PlusOne_MillNonBlack",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Insect Token",
          "counter": "DEATHTOUCH",
          "count": 0
        }
      ]
    },
    {
      "name": "test_PlusOne_MillBlack",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Insect Token",
          "counter": "DEATHTOUCH",
          "count": 1
        }
      ]
    },
    {
      "name": "test_PlusOne_MillBlack_Chatterfang",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chatterfang, Squirrel General",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Insect Token",
          "counter": "DEATHTOUCH",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Squirrel Token",
          "counter": "DEATHTOUCH",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Minus6",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bitterblossom",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Keranos, God of Storms",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, gristPW, CounterType.LOYALTY, 4)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-6"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 5
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grist, the Plague Swarm",
          "counter": "LOYALTY",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bitterblossom",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Taiga",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Keranos, God of Storms",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.LAND, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Dryad Arbor\", SubType.INSECT)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Dryad Arbor\", SubType.DRYAD)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Dryad Arbor\", SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"{G}{B}\", true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Baneslayer Angel",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Baneslayer Angel\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Baneslayer Angel\", SubType.INSECT)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Baneslayer Angel\", SubType.ANGEL)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Baneslayer Angel\", \"{G}{B}\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Baneslayer Angel\", ObjectColor.WHITE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Keranos, God of Storms\", CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Keranos, God of Storms\", CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Keranos, God of Storms\", SubType.INSECT)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Keranos, God of Storms\", \"{G}{B}\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Baneslayer Angel\", \"{R}{U}\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Grist, the Hunger Tide\", CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Grist, the Hunger Tide\", CardType.PLANESWALKER, true)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Grist, the Hunger Tide\", SubType.INSECT)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Grist, the Hunger Tide\", SubType.GRIST)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Grist, the Hunger Tide\", \"{G}{B}\", true)"
        }
      ]
    }
  ]
});
