import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/destroy/BaneOfProgessTest.java",
  "tests": [
    {
      "name": "testDestroy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crucible of Worlds",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Stolen Identity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Bane of Progress",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Stolen Identity",
          "target": "Crucible of Worlds"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Bane of Progress"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Stolen Identity",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Crucible of Worlds",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bane of Progress",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Bane of Progress",
          "power": 4,
          "toughness": 4
        }
      ]
    }
  ]
});
