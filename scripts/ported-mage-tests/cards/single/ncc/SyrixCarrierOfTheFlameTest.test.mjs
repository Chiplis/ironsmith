import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/SyrixCarrierOfTheFlameTest.java",
  "tests": [
    {
      "name": "testDamageTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Firewing Phoenix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Illustrious Historian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{5}, Exile "
        },
        {
          "op": "assertExileCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Illustrious Historian",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": "token",
          "name": 2,
          "power": "POSTCOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before trigger\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before trigger\", 2, PhaseStep.POSTCOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Firewing Phoenix"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Firewing Phoenix",
          "power": 4,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testDamageTriggerOpponentSource",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Firewing Phoenix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Illustrious Historian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cremate",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cremate",
          "target": "Illustrious Historian"
        },
        {
          "op": "assertExileCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Illustrious Historian",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before trigger\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"before trigger\", 2, PhaseStep.POSTCOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Firewing Phoenix"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Firewing Phoenix",
          "power": 4,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Firewing Phoenix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": "Firewing Phoenix"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syrix, Carrier of the Flame",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Firewing Phoenix",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Badlands",
          "tapped": true,
          "count": 5
        }
      ]
    }
  ]
});
