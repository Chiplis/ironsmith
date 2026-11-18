import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mkc/NellyBorcaImpulsiveAccuserTest.java",
  "tests": [
    {
      "name": "testOneCreatureOneOpponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Bear Cub",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ]
    },
    {
      "name": "testTwoCreaturesOneOpponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Bear Cub",
          "defender": "playerC"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ]
    },
    {
      "name": "testTwoCreaturesTwoOpponents",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Bear Cub",
          "defender": "playerC"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": "playerD"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ]
    },
    {
      "name": "testOneCreatureAttackNelly",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Bear Cub",
          "defender": 0
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackSize",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 0
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ]
    },
    {
      "name": "testTwoCreaturesAttackNellyAndOpponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Bear Cub",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ]
    },
    {
      "name": "testNellyAttacks",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Nelly Borca, Impulsive Accuser",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "text": "Whenever Nelly Borca attacks, suspect target creature. Then goad all suspected creatures.",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 0
        }
      ]
    },
    {
      "name": "testNellyPlayerAttacks",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "count": 0
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 0
        }
      ]
    },
    {
      "name": "testOneCreatureDoubleStrikeOneOpponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nelly Borca, Impulsive Accuser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Adorned Pouncer",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "text": "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents",
          "count": 1
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
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
