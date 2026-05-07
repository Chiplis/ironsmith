import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/OathOfKayaTest.java",
  "tests": [
    {
      "name": "test_AttackingPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Oath of Kaya",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
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
          "op": "assertCounterCount",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "counter": "LOYALTY",
          "count": 6
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "test_AttackingPlaneswalker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Oath of Kaya",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Liliana, Dreadhorde General"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Liliana, Dreadhorde General"
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
          "op": "assertCounterCount",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "counter": "LOYALTY",
          "count": "6 - 2 * 2"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    },
    {
      "name": "test_AttackingTwoPlaneswalkers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Oath of Kaya",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vivien, Champion of the Wilds",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Liliana, Dreadhorde General"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Vivien, Champion of the Wilds"
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
          "op": "assertCounterCount",
          "player": 1,
          "name": "Liliana, Dreadhorde General",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Vivien, Champion of the Wilds",
          "counter": "LOYALTY",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    }
  ]
});
