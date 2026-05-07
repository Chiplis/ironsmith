import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/GainControlOfOwnedCreaturesTest.java",
  "tests": [
    {
      "name": "TrostaniDiscordantTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Trostani Discordant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dubious Challenge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dubious Challenge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trostani Discordant^Void Winnower"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Trostani Discordant"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dubious Challenge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Void Winnower",
          "power": 11,
          "toughness": 9
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Trostani Discordant",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Trostani Discordant",
          "power": 1,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "TrostaniDiscordantTriggerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Trostani Discordant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dubious Challenge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dubious Challenge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trostani Discordant^Void Winnower"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Trostani Discordant"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 8,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dubious Challenge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Void Winnower",
          "power": 12,
          "toughness": 10
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Trostani Discordant",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Trostani Discordant",
          "power": 1,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
