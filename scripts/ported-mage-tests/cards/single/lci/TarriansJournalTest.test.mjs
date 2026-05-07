import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/TarriansJournalTest.java",
  "tests": [
    {
      "name": "testFunctionality",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tarrian's Journal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Edgar Markov",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}, Discard your hand: Transform"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "The Tomb of Aclazotz",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You may cast a creature spell from your graveyard this turn. "
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kraken Hatchling"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vampire Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(kraken, SubType.VAMPIRE)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Kraken Hatchling",
          "counter": "FINALITY",
          "count": 1
        }
      ]
    }
  ]
});
