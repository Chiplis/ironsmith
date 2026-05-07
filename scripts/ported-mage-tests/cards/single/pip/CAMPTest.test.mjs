import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/CAMPTest.java",
  "tests": [
    {
      "name": "test_ShareColor",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "C.A.M.P.",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Coiling Oracle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Fortify {3}",
          "target": "Volcanic Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Coiling Oracle"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Coiling Oracle",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Junk Token",
          "count": 1
        }
      ]
    },
    {
      "name": "test_DontShareColor",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "C.A.M.P.",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Coiling Oracle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Fortify {3}",
          "target": "Volcanic Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Coiling Oracle"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Coiling Oracle",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Junk Token",
          "count": 0
        }
      ]
    }
  ]
});
