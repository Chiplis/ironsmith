import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/HarvesterDruidTest.java",
  "tests": [
    {
      "name": "testOneInstance",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Harvester Druid",
          "count": 1
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{R}{R}\", options)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{R}\", options)"
        }
      ]
    },
    {
      "name": "testTwoInstances",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Harvester Druid",
          "count": 2
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{R}{R}{R}\", options)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{R}{R}\", options)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{R}\", options)"
        }
      ]
    }
  ]
});
