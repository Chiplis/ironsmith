import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/NestOfScarabsTest.java",
  "tests": [
    {
      "name": "scarabs_SoulStinger_TwoCountersTwoTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soulstinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soulstinger"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Soulstinger"
        },
        {
          "op": "setStrictChooseMode",
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
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soulstinger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Soulstinger",
          "counter": "M1M1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soulstinger",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 2
        }
      ]
    },
    {
      "name": "scarabs_BlackSunsZenithTargettingSelfAndOpponent_4Counters4Tokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Black Sun's Zenith"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
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
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Grizzly Bears",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Hill Giant",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Hill Giant",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 4
        }
      ]
    },
    {
      "name": "scarabsForBothPlayers_BlackSunsZenithTargettingSelfAndOpponent_4Counters4Tokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Black Sun's Zenith"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
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
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Black Sun's Zenith",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Grizzly Bears",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Hill Giant",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Hill Giant",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Insect Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 4
        }
      ]
    },
    {
      "name": "scarab_infectDamageTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blight Mamba",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Omens",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Blight Mamba",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Omens",
          "attacker": "Blight Mamba"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wall of Omens",
          "power": -1,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Wall of Omens",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 1
        }
      ]
    },
    {
      "name": "scarab_witherDamageTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sickle Ripper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Omens",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Sickle Ripper",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Omens",
          "attacker": "Sickle Ripper"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wall of Omens",
          "power": -2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Wall of Omens",
          "counter": "M1M1",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 2
        }
      ]
    },
    {
      "name": "scarabs_ETBWithCountersTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Noxious Hatchling",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Noxious Hatchling"
        },
        {
          "op": "setStrictChooseMode",
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
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Noxious Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 4
        }
      ]
    },
    {
      "name": "scarabs_OpponentETBWithCountersNoTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Noxious Hatchling",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Noxious Hatchling"
        },
        {
          "op": "setStrictChooseMode",
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
          "player": 1,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Noxious Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Insect Token",
          "count": 0
        }
      ]
    },
    {
      "name": "noTriggerOpponentBlight",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "High Perfect Morcant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nest of Scarabs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "High Perfect Morcant"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Memnite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insect Token",
          "count": 0
        }
      ]
    }
  ]
});
