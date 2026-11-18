import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planes/UndercityReachesTest.java",
  "tests": [
    {
      "name": "test_CanTriggerByController",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPlanechase",
          "plane": "Undercity Reaches"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ],
      "seed": 137
    },
    {
      "name": "test_CanTriggerByOther",
      "operations": [
        {
          "op": "clearZone",
          "player": 1,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPlanechase",
          "plane": "Undercity Reaches"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        }
      ],
      "seed": 137
    }
  ],
  "planarDecks": [
    [
      "Llanowar",
      "Hedron Fields of Agadeem",
      "Panopticon",
      "Tazeem",
      "Gavony",
      "Krosa",
      "Naya",
      "Undercity Reaches",
      "Goldmeadow",
      "Turri Island"
    ],
    [
      "Undercity Reaches",
      "Hedron Fields of Agadeem",
      "Panopticon",
      "Tazeem",
      "Gavony",
      "Krosa",
      "Naya",
      "Llanowar",
      "Goldmeadow",
      "Turri Island"
    ]
  ]
});
