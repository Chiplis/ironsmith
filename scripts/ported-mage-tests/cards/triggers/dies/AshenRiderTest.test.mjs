import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/AshenRiderTest.java",
  "tests": [
    {
      "name": "cartelAristrocraftInteractionOpponentDoesNotPayLife",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
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
          "name": "Island",
          "count": 2
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
          "name": "Ashen Rider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Volrath, the Shapestealer",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ashen Rider"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Putrefy",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Volrath, the Shapestealer"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Until your next turn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ashen Rider"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Putrefy",
          "target": "Ashen Rider[only copy]"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashen Rider",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Putrefy",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Volrath, the Shapestealer",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    }
  ]
});
