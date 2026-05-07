import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/csp/RimescaleDragonTest.java",
  "tests": [
    {
      "name": "testActivatedAbility",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
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
          "op": "unsupported",
          "source": "this.assertTapped(thopter, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ornithopter",
          "counter": "ICE",
          "count": 1
        }
      ]
    },
    {
      "name": "testStaticAbility",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "this.assertTapped(thopter, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ornithopter",
          "counter": "ICE",
          "count": 1
        }
      ]
    },
    {
      "name": "testStaticAbilityEnded",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Terror",
          "target": "Rimescale Dragon"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "this.assertTapped(thopter, false)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ornithopter",
          "counter": "ICE",
          "count": 1
        }
      ]
    }
  ]
});
