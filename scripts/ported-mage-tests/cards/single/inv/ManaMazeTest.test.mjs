import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/inv/ManaMazeTest.java",
  "tests": [
    {
      "name": "test_DeepCopy_WithSelfReference",
      "operations": []
    },
    {
      "name": "test_DeepCopy_WatcherWithSelfReference",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mana Maze",
          "count": 1
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
          "zone": "HAND",
          "player": 1,
          "name": "Aven Reedstalker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Aven Reedstalker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mana Maze"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "label": "Cast Aven Reedstalker",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mana Maze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Aven Reedstalker",
          "count": 0
        }
      ]
    }
  ]
});
