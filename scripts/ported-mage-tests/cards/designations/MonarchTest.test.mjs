import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/designations/MonarchTest.java",
  "tests": [
    {
      "name": "test_MonarchByCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thorn of the Black Rose",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerD",
          "name": "Thorn of the Black Rose",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "monarch": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thorn of the Black Rose"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "monarch": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Thorn of the Black Rose"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertMonarch",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "monarch": "playerD"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_MonarchByDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thorn of the Black Rose",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thorn of the Black Rose"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "monarch": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "assertMonarch",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "monarch": "playerD"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "assertMonarch",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerC",
          "monarch": "playerD"
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "assertMonarch",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerC",
          "monarch": "playerC"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_MonarchByDies",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "target damage 100 for playerA",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: target damage 100 for playerA deals 100 damage to any target."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thorn of the Black Rose",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thorn of the Black Rose"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "monarch": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target damage 100",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertMonarch",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "monarch": "playerD"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerState",
          "player": 0,
          "property": "hasLost",
          "expected": true
        }
      ]
    }
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ]
});
