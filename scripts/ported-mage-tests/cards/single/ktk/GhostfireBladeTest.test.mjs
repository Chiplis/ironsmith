import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ktk/GhostfireBladeTest.java",
  "tests": [
    {
      "name": "test_CanPlayWithCostReduce",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghostfire Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alpha Myr",
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
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Equip {3}",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Equip {3}",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Equip {3}",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}",
          "target": "Alpha Myr"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Alpha Myr",
          "power": 4,
          "toughness": 3
        }
      ]
    }
  ]
});
