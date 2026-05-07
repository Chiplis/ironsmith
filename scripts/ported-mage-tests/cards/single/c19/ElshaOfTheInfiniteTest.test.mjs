import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c19/ElshaOfTheInfiniteTest.java",
  "tests": [
    {
      "name": "test_MustApplyToTopCardOnly",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elsha of the Infinite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bolt of Keranos",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Birgi, God of Storytelling",
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
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Bolt of Keranos",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Birgi, God of Storytelling",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Harnfel, Horn of Bounty",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Bolt of Keranos",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Birgi, God of Storytelling",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Harnfel, Horn of Bounty",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"before cast\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Birgi, God of Storytelling\", 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Harnfel, Horn of Bounty"
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
          "name": "Harnfel, Horn of Bounty",
          "count": 1
        }
      ]
    }
  ]
});
