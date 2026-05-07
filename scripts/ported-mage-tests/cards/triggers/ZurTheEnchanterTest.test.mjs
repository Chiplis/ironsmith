import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/ZurTheEnchanterTest.java",
  "tests": [
    {
      "name": "testAuraToBattlefieldDoesNotTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Zur the Enchanter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Diplomatic Immunity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Empyrial Armor",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Diplomatic Immunity",
          "target": "Zur the Enchanter"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Zur the Enchanter",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Zur the Enchanter"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Diplomatic Immunity",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Empyrial Armor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Zur the Enchanter",
          "power": 2,
          "toughness": 5
        }
      ]
    }
  ]
});
