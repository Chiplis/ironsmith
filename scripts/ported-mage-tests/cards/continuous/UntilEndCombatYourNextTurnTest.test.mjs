import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilEndCombatYourNextTurnTest.java",
  "tests": [
    {
      "name": "testSameTurnPre",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testSameTurnPost",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testOppTurnPre",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testOppTurnPost",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTurnCyclePre",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTurnCycleFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "false ? 2 : 1",
          "toughness": "false ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTimeStopTurnCyclePre",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTimeStopTurnCycleFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTimeStop2TurnCyclePre",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTimeStop2TurnCycleFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilEndCombatOfYourNextTurn ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    }
  ]
});
