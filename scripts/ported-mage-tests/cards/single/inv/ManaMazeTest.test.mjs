import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/inv/ManaMazeTest.java",
  "tests": [
    {
      "name": "test_DeepCopy_WithSelfReference",
      "skip": "Requires a Java StackOverflowError when CardUtil deep-copies a cyclic ArrayList"
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
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Aven Reedstalker",
          "methods": ["normal"]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mana Maze"
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Aven Reedstalker",
          "methods": []
        },
        {
          "op": "saveRuntimeCheckpoint"
        },
        {
          "op": "assertCastMethods",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Aven Reedstalker",
          "methods": ["normal"]
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "assertCastMethods",
          "player": 1,
          "name": "Aven Reedstalker",
          "methods": []
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
