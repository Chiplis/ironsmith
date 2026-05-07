import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/GainControlDiedCastAgainTest.java",
  "tests": [
    {
      "name": "testBoostEffectsWorksForControllerOfElesh",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Volrath's Stronghold",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kalonian Tusker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Keiga, the Tide Star",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kalonian Tusker",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Elesh Norn, Grand Cenobite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Keiga, the Tide Star",
          "attacker": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elesh Norn, Grand Cenobite",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kalonian Tusker",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Kalonian Tusker",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testBoostEffectsWorkForController",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Akroma's Vengeance",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Volrath's Stronghold",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Darksteel Gargoyle",
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
          "name": "Keiga, the Tide Star",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Gargoyle",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Keiga, the Tide Star"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Elesh Norn, Grand Cenobite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Keiga, the Tide Star",
          "attacker": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Akroma's Vengeance"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "END_TURN",
          "player": 1,
          "ability": "{1}{B}, {T}: Put target creature card",
          "target": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "castSpell",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Keiga, the Tide Star",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Keiga, the Tide Star",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Elesh Norn, Grand Cenobite",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Akroma's Vengeance",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Gargoyle",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Darksteel Gargoyle",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Darksteel Gargoyle",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Darksteel Gargoyle",
          "power": 5,
          "toughness": 5
        }
      ]
    }
  ]
});
