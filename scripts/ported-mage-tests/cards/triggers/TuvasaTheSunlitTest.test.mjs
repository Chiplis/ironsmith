import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/TuvasaTheSunlitTest.java",
  "tests": [
    {
      "name": "testWithStriveSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tuvasa the Sunlit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Burgeoning",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ajani's Welcome",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Burgeoning"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ajani's Welcome"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        }
      ]
    }
  ]
});
