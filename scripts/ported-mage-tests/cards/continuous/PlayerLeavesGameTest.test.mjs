import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/PlayerLeavesGameTest.java",
  "tests": [
    {
      "name": "test_PlayerLeaveGame",
      "operations": [
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndCustomEffect",
      "operations": [],
      "skip": "upstream body comments out the only test call"
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndWhileOnBattlefieldEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "effect",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "Creatures get +1/+1."
        },
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndEndOfGameEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "effect",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "Creatures get +1/+1."
        },
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndUntilSourceLeavesBattlefielEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "effect",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 for as long as this permanent remains on the battlefield."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "effect",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_EndOfTurnMultiLeave",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until end of turn."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "concede",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilYourNextTurnMultiLeave",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until your next turn."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "concede",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "boost",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "boost",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilEndOfYourNextTurnMultiLeave",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until the end of your next turn."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "concede",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "boost",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "name": "boost",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ]
});
