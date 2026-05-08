import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/unf/CometStellarPupTest.java",
  "tests": [
    {
      "name": "testRoll1",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 7
        }
      ]
    },
    {
      "name": "testRoll2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 2)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Squirrel Token",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Squirrel Token",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 7
        }
      ]
    },
    {
      "name": "testRoll3",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 3)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 4
        }
      ]
    },
    {
      "name": "testRoll4",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 4)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "PlayerB"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 3
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testRoll5",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ancient Brontodon",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 5)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ancient Brontodon"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Ancient Brontodon\", 5)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 3
        }
      ]
    },
    {
      "name": "testRoll6",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ghalta, Primal Hunger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"6 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 6)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"7 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 7)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"9 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 9)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 2)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"9 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 11)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 4)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghalta, Primal Hunger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Ghalta, Primal Hunger\", 11)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 9
        }
      ]
    },
    {
      "name": "testRoll6WithCarthTheLion",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Carth the Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"7 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 7)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"10 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 10)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 2)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 13
        }
      ]
    },
    {
      "name": "testRoll6AgainstEidolonOfObstruction",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "count": 1
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
          "player": 1,
          "name": "Eidolon of Obstruction",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 6)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"7 loyalty\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, comet, CounterType.LOYALTY, 6)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Roll a six-sided die."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "0: Roll a six-sided die.",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Comet, Stellar Pup",
          "counter": "LOYALTY",
          "count": 8
        }
      ]
    }
  ]
});
