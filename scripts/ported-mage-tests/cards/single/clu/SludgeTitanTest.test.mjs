import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clu/SludgeTitanTest.java",
  "tests": [
    {
      "name": "testNoValidChoice",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Divination",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Divination",
          "count": 5
        }
      ]
    },
    {
      "name": "testCreatureOnly_ChooseNone",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Goblin Piker",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 5
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        }
      ]
    },
    {
      "name": "testCreatureOnly_ChooseOne",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Goblin Piker",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        }
      ]
    },
    {
      "name": "testCreatureOnly_ChooseTwoInvalid",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Goblin Piker",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "assertChoiceMenu",
          "description": "Choose up to 1 creature card",
          "player": 0,
          "kind": "select_objects",
          "includes": [
            "Goblin Piker"
          ],
          "min": 0,
          "max": 1
        },
        {
          "op": "setObjectChoices",
          "description": "Choose up to 1 creature card",
          "choices": [
            "Goblin Piker"
          ],
          "player": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 0
        }
      ]
    },
    {
      "name": "testBoth_ChooseTwo",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Savannah",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plateau",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Savannah"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Savannah",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        }
      ]
    },
    {
      "name": "testBoth_ChooseTwo_DryadArbor",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dryad Arbor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dryad Arbor"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 2
        }
      ]
    },
    {
      "name": "testBoth_ChooseThree_DryadArbor_Invalid",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "assertChoiceMenu",
          "description": "Choose up to 1 creature card",
          "player": 0,
          "kind": "select_objects",
          "includes": [
            "Dryad Arbor"
          ],
          "min": 0,
          "max": 1
        },
        {
          "op": "setObjectChoices",
          "description": "Choose up to 1 creature card",
          "choices": [
            "Dryad Arbor"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "description": "Choose up to 1 land card",
          "player": 0,
          "kind": "select_objects",
          "includes": [
            "Dryad Arbor"
          ],
          "min": 0,
          "max": 1
        },
        {
          "op": "setObjectChoices",
          "description": "Choose up to 1 land card",
          "choices": [
            "Dryad Arbor"
          ],
          "player": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Brownscale_triggers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sludge Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Golgari Brownscale",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sludge Titan",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Golgari Brownscale"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Golgari Brownscale",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Golgari Brownscale",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    }
  ]
});
