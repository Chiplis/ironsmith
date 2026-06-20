import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilYourNextUpkeep.java",
  "tests": [
    {
      "name": "testSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
      "name": "testOppTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
      "name": "testTurnCycle",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
          "power": "false ? 2 : 1",
          "toughness": "false ? 2 : 1"
        }
      ]
    },
    {
      "name": "testParadoxHazeOppSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
      "name": "testParadoxHazeSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
          "power": "false ? 2 : 1",
          "toughness": "false ? 2 : 1"
        }
      ]
    },
    {
      "name": "testEonHubSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
      "name": "testEonHubCycleTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerA, new SimpleActivatedAbility(new BoostSourceEffect( 1, 1, Duration.UntilYourNextUpkeepStep ), new ManaCostsImpl<>(\"{0}\")), null, CardType.CREATURE, \"\", Zone.BATTLEFIELD )"
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
          "power": "true ? 2 : 1",
          "toughness": "true ? 2 : 1"
        }
      ]
    }
  ]
});
