import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/otc/VihaanGoldwakerTest.java",
  "tests": [
    {
      "name": "test_Simple",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vihaan, Goldwaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mimic",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mimic",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertType",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mimic",
          "cardType": "ARTIFACT",
          "extra": "true"
        },
        {
          "op": "assertType",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mimic",
          "cardType": "CREATURE",
          "extra": "true"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"check mimic Treasure\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Mimic\", SubType.TREASURE, true)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"check mimic Construct\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Mimic\", SubType.CONSTRUCT, true)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"check mimic Assassin\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Mimic\", SubType.ASSASSIN, true)"
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mimic",
          "ability": "VigilanceAbility.class",
          "expected": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mimic",
          "ability": "HasteAbility.class",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mimic",
          "count": 1
        }
      ]
    }
  ]
});
