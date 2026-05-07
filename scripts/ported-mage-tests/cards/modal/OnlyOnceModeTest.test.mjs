import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/modal/OnlyOnceModeTest.java",
  "tests": [
    {
      "name": "test_OncePerGame",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Three Bowls of Porridge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spectral Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"before mode 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Spectral Bears\", 0)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Spectral Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"before mode 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Spectral Bears\", 2)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"before mode 2\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Spectral Bears\", false, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Spectral Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"after mode 2\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Spectral Bears\", true, 1)"
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
      "name": "test_OncePerTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galadriel, Light of Valinor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": "2 * triggerCalls"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"before mode 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"G\", 0)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"after mode 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"G\", 3)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"before mode 2\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", CounterType.P1P1, 0)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after mode 2\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", CounterType.P1P1, 1)"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"before mode 1 on turn 3\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"G\", 0)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"after mode 1 on turn 3\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"G\", 3)"
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
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 5
        }
      ]
    }
  ]
});
