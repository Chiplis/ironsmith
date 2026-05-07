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
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
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
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker^Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().startsWith(\"Missing CHOICE def\")) { Assert.fail(\"Unexpected exception \" + e.getMessage()); } }"
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
          "value": "Goblin Piker^Savannah"
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
          "count": 0,
          "name": 3
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
          "value": "Dryad Arbor^Dryad Arbor"
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
          "op": "setChoice",
          "player": 0,
          "value": "Dryad Arbor^Dryad Arbor^Dryad Arbor"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().startsWith(\"Missing CHOICE def\")) { Assert.fail(\"Unexpected exception \" + e.getMessage()); } }"
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
