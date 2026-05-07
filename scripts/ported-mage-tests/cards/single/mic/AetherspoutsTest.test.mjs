import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mic/AetherspoutsTest.java",
  "tests": [
    {
      "name": "testAllToTop",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Aetherspouts",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Attack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Beast Attack"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Beast Token",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Kalonian Behemoth",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Aetherspouts"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balduvian Bears"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Beast Token",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Beast Token",
          "count": 0
        }
      ]
    },
    {
      "name": "testSomeToBottom",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Aetherspouts",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Attack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Beast Attack"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Beast Token",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Kalonian Behemoth",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Aetherspouts"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Beast Token",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Beast Token",
          "count": 0
        }
      ]
    }
  ]
});
