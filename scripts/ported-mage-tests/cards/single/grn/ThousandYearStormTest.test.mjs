import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/grn/ThousandYearStormTest.java",
  "tests": [
    {
      "name": "test_CalcBeforeStorm",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerB, 20 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"copy\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CalcStackBefore",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"copy\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CalcStackAfter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"copy\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CalcAfterStormOnSameStep",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"copy\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CalcAfterStormOnDiffStep",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"copy\", 1, PhaseStep.END_TURN, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_OwnCounts",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 100
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 100
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"0x copy\", 1, PhaseStep.PRECOMBAT_MAIN, playerB, startLife - 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thousand-Year Storm"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"1x copy\", 1, PhaseStep.BEGIN_COMBAT, playerB, startLife - 3 - 3 * 2)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
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
          "op": "unsupported",
          "source": "checkLife(\"2x copy\", 1, PhaseStep.END_COMBAT, playerB, startLife - 3 - 3 * 2 - 3 * 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
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
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"3x copy\", 1, PhaseStep.END_TURN, playerB, startLife - 3 - 3 * 2 - 3 * 3 - 3 * 4)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"3. no copy\", 3, PhaseStep.BEGIN_COMBAT, playerB, startLife - 3)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"3. 1x copy\", 3, PhaseStep.END_COMBAT, playerB, startLife - 3 - 3 * 2)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
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
          "op": "unsupported",
          "source": "checkLife(\"3. 2x copy\", 3, PhaseStep.END_TURN, playerB, startLife - 3 - 3 * 2 - 3 * 3)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_OpponentDontCounts",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"0x copy\", 1, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"0x copy\", 1, PhaseStep.BEGIN_COMBAT, playerA, 20 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"1x copy\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"0x copy\", 1, PhaseStep.END_COMBAT, playerA, 20 - 3 - 3)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_WaitStackResolvedWithBolts",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"A must get damage\", 2, PhaseStep.END_TURN, playerA, 20 - 3 - 3)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"B must get damage\", 2, PhaseStep.END_TURN, playerB, 20 - 3)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_GetControlNotCounts",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thousand-Year Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lay Claim",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"a 0x copy\", 2, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"a 1x copy\", 2, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {U}",
          "count": 7
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lay Claim",
          "target": "Thousand-Year Storm"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"b 0x copy after control\", 3, PhaseStep.UPKEEP, playerA, 20 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"b 0x copy\", 4, PhaseStep.BEGIN_COMBAT, playerA, 20 - 3 - 3)"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"b 1x copy\", 4, PhaseStep.END_COMBAT, playerA, 20 - 3 - 3 - 3 * 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
