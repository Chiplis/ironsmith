import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/ScarletSpiderBenReillyTest.java",
  "tests": [
    {
      "name": "testScarletSpiderBenReilly",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"tap all creatures\", playerA, new SimpleActivatedAbility( new TapAllEffect(new FilterCreaturePermanent(SubType.BEAR, \"bears\")), new ManaCostsImpl<>(\"\") ))"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Scarlet Spider, Ben Reilly",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "tap all"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Scarlet Spider, Ben Reilly with Web-slinging"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bear Cub"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Scarlet Spider, Ben Reilly",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        }
      ]
    }
  ]
});
