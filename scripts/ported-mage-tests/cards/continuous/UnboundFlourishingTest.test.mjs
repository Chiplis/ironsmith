import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UnboundFlourishingTest.java",
  "tests": [
    {
      "name": "test_OnCastPermanent_MustDoubleX",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Endless One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Endless One"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Endless One\", CounterType.P1P1, 3 * 2)"
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
      "name": "test_OnCastPermanent_MustDoubleX_MultipleTimes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Endless One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Endless One"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you cast a permanent spell"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Endless One\", CounterType.P1P1, 3 * 2 * 2)"
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
      "name": "test_OnCastInstantOrSorcery_MustCopy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banefire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Banefire",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, 20 - 3)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3)"
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
      "name": "test_OnCastPermanent_MustIgnoreAdditionCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond of Agony",
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
          "name": "Bond of Agony"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, 20 - 3)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3 * 2)"
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
      "name": "test_OnActivatedAbility_MustCopy1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Helix Pinnacle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Helix Pinnacle\", CounterType.TOWER, 0)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Helix Pinnacle\", CounterType.TOWER, 3 + 3)"
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
      "name": "test_OnActivatedAbility_MustCopy1_MultipleTimes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Helix Pinnacle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Helix Pinnacle\", CounterType.TOWER, 0)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you cast an instant or sorcery spell"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Helix Pinnacle\", CounterType.TOWER, 3 + 3 * 2)"
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
      "name": "test_OnActivatedAbility_MustCopy2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cinder Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}{R}",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, 20 - 3)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3)"
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
      "name": "test_OnActivatedAbility_MustCopy2Counter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cinder Elemental",
          "count": 1
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
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stifle",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}{R}",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Stifle"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "stack ability ({X}{R}, {T}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerB, 20 - 3)"
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
      "name": "test_VariableManaCost",
      "operations": []
    },
    {
      "name": "test_MultipleXInstances",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chalice of the Void",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chalice of the Void"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Chalice of the Void\", CounterType.CHARGE, 1)"
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
        }
      ]
    }
  ]
});
