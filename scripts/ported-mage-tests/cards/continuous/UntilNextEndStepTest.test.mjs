import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilNextEndStepTest.java",
  "tests": [
    {
      "name": "testSameTurnTrue",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
      "name": "testSameTurnFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": "false ? 2 : 1",
          "toughness": "false ? 2 : 1"
        }
      ]
    },
    {
      "name": "testNextTurnTrue",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
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
      "name": "testNextTurnFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
          "power": "false ? 2 : 1",
          "toughness": "false ? 2 : 1"
        }
      ]
    },
    {
      "name": "testTurnCycleTrue",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
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
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
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
      "name": "testOpponentTurnTrue",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
      "name": "testOpponentTurnFalse",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextEndStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
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
    }
  ]
});
