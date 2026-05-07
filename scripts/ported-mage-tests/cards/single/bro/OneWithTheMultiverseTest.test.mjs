import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bro/OneWithTheMultiverseTest.java",
  "tests": [
    {
      "name": "castFromTopForFree",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        }
      ]
    },
    {
      "name": "castFromHandForFree",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
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
      "name": "castFromTopForFreeThenNormalFromTop",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 11
        }
      ]
    },
    {
      "name": "castFromTopForFreeThenNormalFromHand",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 10
        }
      ]
    },
    {
      "name": "castFromHandForFreeThenNormalFromHand",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 10
        }
      ]
    },
    {
      "name": "castFromHandForFreeThenNormalFromTop",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 11
        }
      ]
    },
    {
      "name": "castNormalFromTopThenFreeFromHand",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 11
        }
      ]
    },
    {
      "name": "castNormalFromTopThenFreeFromTop",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 11
        }
      ]
    },
    {
      "name": "castNormalFromHandThenFreeFromHand",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 0
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 10
        }
      ]
    },
    {
      "name": "castNormalFromHandThenFreeFromTop",
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
          "zone": "HAND",
          "player": 0,
          "name": "One with the Multiverse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Multiverse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Piker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gray Ogre"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Without paying manacost"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gray Ogre",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Goblin Piker",
          "expected": false
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
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Volcanic Island",
          "tapped": true,
          "count": 10
        }
      ]
    }
  ]
});
