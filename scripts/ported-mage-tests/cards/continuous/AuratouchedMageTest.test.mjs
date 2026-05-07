import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/AuratouchedMageTest.java",
  "tests": [
    {
      "name": "testSearch",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Auratouched Mage"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "White Ward",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "White Ward"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
