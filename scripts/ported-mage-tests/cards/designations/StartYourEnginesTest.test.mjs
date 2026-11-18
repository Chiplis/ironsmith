import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/designations/StartYourEnginesTest.java",
  "tests": [
    {
      "name": "testRegular",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertSpeed",
          "player": 0,
          "speed": 0
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 1
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 2
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed1OppTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 1
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 3
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed4",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 4
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed5",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Walking Sarcophagus",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 4
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeed4Graveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Loxodon Surveyor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Onyx Goblet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Walking Sarcophagus",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Walking Sarcophagus",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 1
        },
        {
          "op": "activateAbility",
          "source": "Onyx Goblet",
          "ability": "Target player loses 1 life",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Onyx Goblet",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 2
        },
        {
          "op": "activateAbility",
          "source": "Onyx Goblet",
          "ability": "Target player loses 1 life",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Onyx Goblet",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 3
        },
        {
          "op": "activateAbility",
          "source": "Onyx Goblet",
          "ability": "Target player loses 1 life",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Onyx Goblet",
          "count": 1,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 4
        },
        {
          "op": "activateAbility",
          "source": "Loxodon Surveyor",
          "ability": "Draw a card",
          "player": 0,
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Loxodon Surveyor",
          "count": 1,
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Loxodon Surveyor",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Loxodon Surveyor",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 4
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 0
        }
      ]
    },
    {
      "name": "testSpeedChangeControl",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Walking Sarcophagus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mind Control",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Walking Sarcophagus"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mind Control",
          "target": "Walking Sarcophagus"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Walking Sarcophagus",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertSpeed",
          "player": 0,
          "speed": 1
        },
        {
          "op": "assertSpeed",
          "player": 1,
          "speed": 1
        }
      ]
    }
  ]
});
