import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tdc/BetorAncestorsVoiceTest.java",
  "tests": [
    {
      "name": "test_RhowFaithmender",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Betor, Ancestor's Voice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rhox Faithmender",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Betor, Ancestor's Voice",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rhox Faithmender",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Rhox Faithmender"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 28
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rhox Faithmender",
          "counter": "P1P1",
          "count": 8
        }
      ]
    }
  ]
});
