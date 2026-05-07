import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/PlayerUnderControlTest.java",
  "tests": [
    {
      "name": "test_ClientSideDataMustBeHidden",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Emrakul, the Promised End",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 13
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Emrakul, the Promised End"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkUnderControl(\"turn 1 - A, B normal\", 1, PhaseStep.PRECOMBAT_MAIN, false)"
        },
        {
          "op": "unsupported",
          "source": "checkUnderControl(\"turn 2 - B under A\", 2, PhaseStep.PRECOMBAT_MAIN, true)"
        },
        {
          "op": "unsupported",
          "source": "checkUnderControl(\"turn 3 - A, B normal\", 3, PhaseStep.PRECOMBAT_MAIN, false)"
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
    }
  ]
});
