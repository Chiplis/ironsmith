import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/CascadeTest.java",
  "tests": [
    {
      "name": "testGainsCascade",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maelstrom Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Predatory Advantage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Predatory Advantage"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Predatory Advantage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lizard Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testLosesCascade",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maelstrom Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Predatory Advantage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nacatl Outlander",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Arbor Elf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Predatory Advantage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Nacatl Outlander"
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
          "name": "Predatory Advantage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nacatl Outlander",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arbor Elf",
          "count": 0
        }
      ]
    },
    {
      "name": "testEmptyLibraryCascasde",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ardent Plea",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ardent Plea"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ardent Plea",
          "count": 1
        }
      ]
    },
    {
      "name": "testEmptyLibraryCascasdeNexus",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maelstrom Nexus",
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
          "name": "Aven Skirmisher",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aven Skirmisher"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Maelstrom Nexus",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Aven Skirmisher",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Aven Skirmisher",
          "count": 1
        }
      ]
    },
    {
      "name": "testRecastCascadeCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
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
          "name": "Enlisted Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enlisted Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Unsummon",
          "target": "Enlisted Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Enlisted Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enlisted Wurm",
          "count": 1
        }
      ]
    },
    {
      "name": "testHaveToPayAdditionalCosts",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Promise of Power",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Enlisted Wurm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enlisted Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enlisted Wurm",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithSplitSpell",
      "operations": []
    },
    {
      "name": "testWithXCost",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lotus Petal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fireball",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maelstrom Nexus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fireball"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lotus Petal",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fireball",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testRemovedFromStack",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsubstantiate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maelstrom Nexus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Unsubstantiate"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        }
      ]
    }
  ]
});
