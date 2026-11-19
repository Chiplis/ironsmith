import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bot/OptimusPrimeTest.java",
  "tests": [
    {
      "name": "testOptimusPrime",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Optimus Prime, Hero@optimus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gutless Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Omega Myr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Gutless Ghoul",
          "ability": "Sacrifice a creature"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Optimus Prime, Hero"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gutless Ghoul",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Optimus Prime, Autobot Leader",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Gutless Ghoul",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gutless Ghoul"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Optimus Prime, Autobot Leader",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Gutless Ghoul",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gutless Ghoul",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Centaur Courser",
          "attacker": "Gutless Ghoul"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "distribution",
          "description": "Assign 4 combat damage from Gutless Ghoul (assign exactly 4 total)",
          "choices": [
            "Centaur Courser",
            "Centaur Courser",
            "Centaur Courser",
            "Bob"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "source": "Optimus Prime, Autobot Leader",
          "count": 1,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
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
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Optimus Prime, Hero",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Optimus Prime, Hero",
          "power": 4,
          "toughness": 8
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Gutless Ghoul",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Omega Myr",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gutless Ghoul",
          "ability": "Trample",
          "expected": false
        }
      ]
    }
  ]
});
