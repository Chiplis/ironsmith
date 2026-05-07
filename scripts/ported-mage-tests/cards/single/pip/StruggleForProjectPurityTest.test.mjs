import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/StruggleForProjectPurityTest.java",
  "tests": [
    {
      "name": "test_Brotherhood",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Struggle for Project Purity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Struggle for Project Purity"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Brotherhood"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertHandCount",
          "player": "no draws on turn 2",
          "name": 2,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "no draws on turn 3",
          "name": 3,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "no draws on turn 4",
          "name": 4,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "draws trigger",
          "name": 5,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "draws trigger",
          "name": 5,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "draws trigger",
          "name": 5,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": "draws trigger",
          "name": 5,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Struggle for Project Purity",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Enclave",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Struggle for Project Purity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Struggle for Project Purity"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Enclave"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Struggle for Project Purity",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "20 - 2 * 2 - 2 * 2"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 2 * 2"
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": "20 - 2 * 2 - 2 * 2"
        }
      ]
    }
  ]
});
