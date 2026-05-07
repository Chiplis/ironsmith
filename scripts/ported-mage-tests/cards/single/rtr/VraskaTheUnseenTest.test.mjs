import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rtr/VraskaTheUnseenTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vraska the Unseen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1:"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 3\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1 - 2 * 2)"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_OnlyCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vraska the Unseen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cinder Pyromancer",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1:"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Cinder Pyromancer",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: {this} deals",
          "target": "Vraska the Unseen"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 2\", 2, PhaseStep.END_TURN, playerA, \"Vraska the Unseen\", CounterType.LOYALTY, 5 + 1 - 2 - 1)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Cinder Pyromancer",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
