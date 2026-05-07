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
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each\", testTurn, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each\", testTurn, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each\", testTurn, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "turn": "Empty Stack",
          "phase": 4,
          "player": "COMBAT_DAMAGE",
          "count": 0
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each\", testTurn, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "op": "unsupported",
          "source": "checkStackObject(\"suspect creature\", 1, PhaseStep.DECLARE_ATTACKERS, playerA, \"Whenever {this} attacks, suspect target creature. Then goad all suspected creatures.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
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
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each first strike\", testTurn, PhaseStep.FIRST_COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"draw 1 each second strike\", testTurn, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents,\" + \" you and the controller of those creatures each draw a card.\", 1)"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
          "count": 1
        }
      ]
    }
  ]
});
