import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/SupernaturalStaminaTest.java",
  "tests": [
    {
      "name": "testChanneler",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Channeler Initiate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Supernatural Stamina",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Channeler Initiate"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Supernatural Stamina",
          "target": "Channeler Initiate"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Shock",
          "target": "Channeler Initiate"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Supernatural Stamina",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Channeler Initiate",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Channeler Initiate",
          "power": 0,
          "toughness": 1
        }
      ]
    }
  ]
});
