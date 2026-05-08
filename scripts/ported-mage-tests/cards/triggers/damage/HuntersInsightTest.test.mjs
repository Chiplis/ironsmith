import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/HuntersInsightTest.java",
  "tests": [
    {
      "name": "testDrawingCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hunter's Insight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Stampeding Rhino",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Stampeding Rhino",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Hunter's Insight",
          "target": "Stampeding Rhino"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 5
        }
      ]
    }
  ]
});
