import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pc2/FracturedPowerstoneTest.java",
  "tests": [
    {
      "name": "test_FracturedPowerstone_Single",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fractured Powerstone",
          "count": 1
        },
        {
          "op": "assertPlanechase",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 0
        },
        {
          "op": "rollPlanarDie",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "planarFace": "chaos"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlanechase",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Roll the planar",
          "planarFace": "chaos"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlanechase",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 1
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
          "name": "Eldrazi Token",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Fractured Powerstone",
          "tapped": true,
          "count": 1
        }
      ],
      "seed": 137
    },
    {
      "name": "test_FracturedPowerstone_NoCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fractured Powerstone",
          "count": 1
        },
        {
          "op": "assertPlanechase",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 0
        },
        {
          "op": "rollPlanarDie",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "planarFace": "chaos"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlanechase",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Roll the planar",
          "planarFace": "chaos"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlanechase",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 1
        },
        {
          "op": "rollPlanarDie",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "planarFace": "chaos"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlanechase",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "plane": "Hedron Fields of Agadeem",
          "rollCost": 2
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
          "name": "Eldrazi Token",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 1
        }
      ],
      "seed": 137
    }
  ],
  "planarDecks": [
    [
      "Llanowar",
      "Undercity Reaches",
      "Panopticon",
      "Tazeem",
      "Gavony",
      "Krosa",
      "Naya",
      "Hedron Fields of Agadeem",
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
