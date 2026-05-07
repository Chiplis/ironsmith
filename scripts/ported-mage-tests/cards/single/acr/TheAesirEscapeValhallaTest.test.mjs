import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/TheAesirEscapeValhallaTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Aesir Escape Valhalla"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gigantosaurus"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after I, lifecount\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 + 5)"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after II, +1/+1 counters\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Memnite\", CounterType.P1P1, 5)"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "The Aesir Escape Valhalla",
          "count": 0
        }
      ]
    }
  ]
});
