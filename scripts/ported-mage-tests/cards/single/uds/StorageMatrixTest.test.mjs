import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/uds/StorageMatrixTest.java",
  "tests": [
    {
      "name": "testOnlyAffectsActivePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Storage Matrix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unwinding Clock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Marble Chalice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mine Worker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Storage Matrix"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "ability": "{T}: You gain 1 life"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"gain 1st turn\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 21)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"gain 1st turn\", 1, PhaseStep.POSTCOMBAT_MAIN, playerB, 21)"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "CardType.LAND.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"worker not land doesn't untap\", 2, PhaseStep.UPKEEP, playerB, worker, true, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"chalice untaps\", 2, PhaseStep.UPKEEP, playerA, chalice, false, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"gain 2nd turn\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 22)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CardType.CREATURE.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"Wastes didn't untap\", 3, PhaseStep.UPKEEP, playerA, \"Wastes\", true, 3)"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "CardType.ARTIFACT.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"worker artifact does untap\", 4, PhaseStep.UPKEEP, playerB, worker, false, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"chalice untaps\", 4, PhaseStep.UPKEEP, playerA, chalice, false, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    }
  ]
});
