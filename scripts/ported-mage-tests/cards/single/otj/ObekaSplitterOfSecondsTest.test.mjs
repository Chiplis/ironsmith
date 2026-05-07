import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/otj/ObekaSplitterOfSecondsTest.java",
  "tests": [
    {
      "name": "test_ExtraUpkeep",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Obeka, Splitter of Seconds",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fountain of Renewal",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Obeka, Splitter of Seconds",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Extra upkeeps are in extra phases after combat\", 1, PhaseStep.END_COMBAT, playerA, 20 + 1)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "unsupported",
          "source": "assertTapped(obeka, true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        }
      ]
    }
  ]
});
