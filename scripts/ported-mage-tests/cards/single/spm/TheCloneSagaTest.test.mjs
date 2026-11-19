import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/TheCloneSagaTest.java",
  "tests": [
    {
      "name": "testTheCloneSaga",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Clone Saga",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ragavan, Nimble Pilferer@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Clone Saga",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Clone Saga",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Clone Saga",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "choices": [],
          "description": "Surveil 3 — select cards to put on graveyard"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Clone Saga",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ragavan, Nimble Pilferer",
          "holdStack": true,
          "objectAlias": "@original"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Ragavan, Nimble Pilferer",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Ragavan, Nimble Pilferer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ragavan, Nimble Pilferer",
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Clone Saga",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ragavan, Nimble Pilferer"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Ragavan, Nimble Pilferer",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Ragavan, Nimble Pilferer",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "source": "Ragavan, Nimble Pilferer",
          "count": 2,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "assertStackSize",
          "turn": 5,
          "phase": "COMBAT_DAMAGE",
          "count": 4
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": null,
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 2
        }
      ]
    }
  ]
});
