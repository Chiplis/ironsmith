import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/vis/DesertionTest.java",
  "tests": [
    {
      "name": "test_MultipleCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Desertion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": "2 * 5"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Desertion",
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Desertion",
          "target": "Grizzly Bears"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"before resolve\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast Grizzly Bears\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"before resolve\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast Desertion\", 2)"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Desertion",
          "count": 2
        }
      ]
    }
  ]
});
