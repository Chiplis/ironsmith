import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/brc/RootpathPurifierTest.java",
  "tests": [
    {
      "name": "testChangeSupertypeInLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tree of Tales",
          "count": 1
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
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_CONTROLLED_PERMANENT_LAND, playerA.getId(), currentGame )) { Assert.assertTrue(permanent.getName() + \" should be basic\", permanent.isBasic(currentGame)); }"
        }
      ]
    },
    {
      "name": "testSearchToBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rampant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tree of Tales",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Tree of Tales"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rampant Growth"
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
          "name": "Tree of Tales",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_CONTROLLED_PERMANENT_LAND, playerA.getId(), currentGame )) { Assert.assertTrue(permanent.getName() + \" should be basic\", permanent.isBasic(currentGame)); }"
        }
      ]
    },
    {
      "name": "testSearchToHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lay of the Land",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tree of Tales",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Tree of Tales"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lay of the Land"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Tree of Tales",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_CONTROLLED_PERMANENT_LAND, playerA.getId(), currentGame )) { Assert.assertTrue(permanent.getName() + \" should be basic\", permanent.isBasic(currentGame)); }"
        }
      ]
    },
    {
      "name": "testRemove",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tree of Tales",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Rootpath Purifier"
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
          "name": "Rootpath Purifier",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rootpath Purifier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Murder",
          "count": 1
        }
      ]
    }
  ]
});
