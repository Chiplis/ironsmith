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
          "op": "unsupported",
          "source": "checkMonarch(\"no monarch before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, null)"
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
          "op": "unsupported",
          "source": "checkMonarch(\"monarch 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, playerA)"
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
          "op": "unsupported",
          "source": "checkMonarch(\"monarch 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerD)"
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
          "op": "unsupported",
          "source": "checkMonarch(\"monarch to A\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, playerA)"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "unsupported",
          "source": "checkMonarch(\"monarch to D\", 2, PhaseStep.POSTCOMBAT_MAIN, playerD, playerD)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "unsupported",
          "source": "checkMonarch(\"nothing to steal (keep on D)\", 3, PhaseStep.POSTCOMBAT_MAIN, playerC, playerD)"
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "unsupported",
          "source": "checkMonarch(\"monarch to C\", 3 + 4, PhaseStep.POSTCOMBAT_MAIN, playerC, playerC)"
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
          "op": "unsupported",
          "source": "addCustomEffect_TargetDamage(playerA, 100)"
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
          "op": "unsupported",
          "source": "checkMonarch(\"monarch to A\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, playerA)"
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
          "op": "unsupported",
          "source": "checkMonarch(\"monarch to D\", 2, PhaseStep.POSTCOMBAT_MAIN, playerD, playerD)"
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
          "op": "unsupported",
          "source": "assertLostTheGame(playerA)"
        }
      ]
    }
  ]
});
