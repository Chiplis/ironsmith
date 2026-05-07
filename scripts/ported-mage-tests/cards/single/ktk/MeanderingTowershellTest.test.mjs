import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ktk/MeanderingTowershellTest.java",
  "tests": [
    {
      "name": "test_Simple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Meandering Towershell",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Meandering Towershell",
          "defender": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": 0,
          "count": "Meandering Powershell"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"T3 First Main: playerB at 20 life\", 3, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Meandering Towershell",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "test_TimeStop",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Meandering Towershell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Time Stop",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Meandering Towershell",
          "defender": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": 0,
          "count": "Meandering Powershell"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"T3 First Main: playerB at 20 life\", 3, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Meandering Towershell",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Meandering Towershell",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
