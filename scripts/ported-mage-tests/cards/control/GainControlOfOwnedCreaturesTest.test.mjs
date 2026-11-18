import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/GainControlOfOwnedCreaturesTest.java",
  "tests": [
    {
      "name": "TrostaniDiscordantTest",
      "operations": [
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
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "LIBRARY",
          "name": "Forest",
          "count": 12
        },
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
          "op": "assertLibraryOrder",
          "player": 0,
          "cards": [
            "Void Winnower",
            "Trostani Discordant",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest"
          ]
        },
        {
          "op": "castSpell",
          "name": "Dubious Challenge",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dubious Challenge",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trostani Discordant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Void Winnower"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Trostani Discordant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Trostani Discordant",
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
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "LIBRARY",
          "name": "Forest",
          "count": 12
        },
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
          "op": "assertLibraryOrder",
          "player": 0,
          "cards": [
            "Void Winnower",
            "Trostani Discordant",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest"
          ]
        },
        {
          "op": "castSpell",
          "name": "Dubious Challenge",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dubious Challenge",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trostani Discordant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Void Winnower"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Trostani Discordant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Trostani Discordant",
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
        },
        {
          "op": "assertStackObject",
          "source": "Trostani Discordant",
          "count": 1,
          "turn": 2,
          "phase": "END_TURN",
          "abilityKind": "Triggered",
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Trostani Discordant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Trostani Discordant",
          "count": 0
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
          "name": "Void Winnower",
          "power": 12,
          "toughness": 10
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
