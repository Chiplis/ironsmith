import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/blb/SplashPortalTest.java",
  "tests": [
    {
      "name": "test_ExileOtterNonOtter",
      "operations": [
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
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alania's Pathmaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Splash Portal",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Splash Portal"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Splash Portal"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alania's Pathmaker"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
