import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/RaulTroubleShooterTest.java",
  "tests": [
    {
      "name": "test_Cast_Simple",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{T}: Each player"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
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
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 2
        }
      ]
    },
    {
      "name": "test_Cast_Split",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Cast_Simple_Instant_YourTurn",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{T}: Each player"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Lightning Bolt",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Lightning Bolt",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Lightning Bolt",
          "expected": false
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
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "test_CannotCast_Instant_NotYourTurn",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
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
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{T}: Each player"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Lightning Bolt",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Lightning Bolt",
          "expected": false
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_CannotCast_OtherTurn",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{T}: Each player"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Grizzly Bears",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Grizzly Bears",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Cast_MilledBefore",
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
          "name": "Raul, Trouble Shooter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Bump in the Night",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thought Scour",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Thought Scour",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raul, Trouble Shooter"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Bump in the Night",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Memnite",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Thought Scour",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bump in the Night",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Bump in the Night",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Memnite",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Thought Scour",
          "expected": false
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
