import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/TokenLimitTest.java",
  "tests": [
    {
      "name": "testOnePlayerHitsLimit",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Anointed Procession",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 34
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Secure the Wastes",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Secure the Wastes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=16"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Secure the Wastes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=16"
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
          "name": "Warrior Token",
          "count": 500
        }
      ]
    },
    {
      "name": "testOnePlayerHitsLimitWithOtherPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Secure the Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 33
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Anointed Procession",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Secure the Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Secure the Wastes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=32"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Secure the Wastes"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=2"
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
          "name": "Warrior Token",
          "count": 500
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Warrior Token",
          "count": 2
        }
      ]
    }
  ]
});
