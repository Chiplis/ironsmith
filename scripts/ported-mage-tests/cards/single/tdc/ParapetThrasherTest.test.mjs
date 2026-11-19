import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tdc/ParapetThrasherTest.java",
  "tests": [
    {
      "name": "testParapetThrasherMode1",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Parapet Thrasher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fountain of Youth",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Parapet Thrasher",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Destroy target artifact that opponent controls."
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "Fountain of Youth"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "source": "Parapet Thrasher",
          "count": 1,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
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
          "player": 1,
          "name": "Fountain of Youth",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "testParapetThrasherMode2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Parapet Thrasher",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Parapet Thrasher",
          "defender": 1
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
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
          "life": 16
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 16
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 16
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "testParapetThrasherMode3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Parapet Thrasher",
          "count": 1
        },
        {
          "op": "clearZone",
          "zone": "library",
          "player": 0
        },
        {
          "op": "addCard",
          "name": "Island",
          "player": 0,
          "zone": "LIBRARY",
          "count": 1
        },
        {
          "op": "addCard",
          "name": "Mountain",
          "player": 0,
          "zone": "LIBRARY",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Parapet Thrasher",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Exile the top card of your library. You may play it this turn."
          ]
        },
        {
          "op": "assertStackObject",
          "source": "Parapet Thrasher",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "playLand",
          "name": "Island",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
