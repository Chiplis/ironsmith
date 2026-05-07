import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/CommanderAsIndomitableCreativityCastTest.java",
  "tests": [
    {
      "name": "test_CastFromHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indomitable Creativity",
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
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indomitable Creativity"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balduvian Bears"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
      "name": "test_CastFromCommand",
      "operations": [
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Indomitable Creativity",
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
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indomitable Creativity"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
    }
  ]
});
