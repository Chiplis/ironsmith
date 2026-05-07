import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/YouCannotPassTest.java",
  "tests": [
    {
      "name": "testBlocked",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yargle, Glutton of Urborg",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "You Cannot Pass!",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Yargle, Glutton of Urborg",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Indomitable Ancients",
          "attacker": "Yargle, Glutton of Urborg"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "You Cannot Pass!",
          "target": "Indomitable Ancients"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "You Cannot Pass!",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, yargle, 2)"
        }
      ]
    },
    {
      "name": "testBlockedBy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yargle, Glutton of Urborg",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "You Cannot Pass!",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Indomitable Ancients",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Yargle, Glutton of Urborg",
          "attacker": "Indomitable Ancients"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "You Cannot Pass!",
          "target": "Indomitable Ancients"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "You Cannot Pass!",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, yargle, 2)"
        }
      ]
    }
  ]
});
