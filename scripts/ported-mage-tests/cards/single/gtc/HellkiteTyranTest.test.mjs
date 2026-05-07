import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/gtc/HellkiteTyranTest.java",
  "tests": [
    {
      "name": "test_BothTriggers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hellkite Tyrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mox Sapphire",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Mox Ruby",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Elite Vanguard",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Hellkite Tyrant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Mox Sapphire",
          "count": 10
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Memnite",
          "count": 10
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Mox Sapphire",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": "playerC",
          "name": "Plains",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": "playerD",
          "name": "Mox Ruby",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": "playerD",
          "name": "Elite Vanguard",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hellkite Tyrant",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertWonTheGame(playerA)"
        }
      ]
    }
  ]
});
