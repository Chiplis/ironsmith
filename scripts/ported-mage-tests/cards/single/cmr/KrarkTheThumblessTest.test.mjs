import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/KrarkTheThumblessTest.java",
  "tests": [
    {
      "name": "test_MustAbleToCastAgainAfterRemoveToHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Krark, the Thumbless",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
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
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": "2 * 2"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Krark, the Thumbless"
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
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after cast\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast L\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after cast\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Whenever you cast\", 2)"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after lose trigger\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast L\", 0)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after lose trigger\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Whenever you cast\", 1)"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after lose trigger\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast L\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"after win trigger\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Whenever you cast\", 0)"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
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
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 4 * 3"
        }
      ]
    }
  ]
});
