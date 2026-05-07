import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/GodEternalDiesTriggeredAbilityTest.java",
  "tests": [
    {
      "name": "dyingTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "God-Eternal Bontu"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        }
      ]
    },
    {
      "name": "exilingTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Anguished Unmaking",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Anguished Unmaking",
          "target": "God-Eternal Bontu"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        }
      ]
    },
    {
      "name": "bounceDoesNotTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absorb Identity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absorb Identity",
          "target": "God-Eternal Bontu"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "God-Eternal Bontu",
          "count": 0
        }
      ]
    }
  ]
});
