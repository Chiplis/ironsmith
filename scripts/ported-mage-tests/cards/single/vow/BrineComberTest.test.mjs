import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/vow/BrineComberTest.java",
  "tests": [
    {
      "name": "test_Aura",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brine Comber",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rancor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rancor",
          "target": "Brine Comber"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spirit Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rancor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Brine Comber",
          "power": 3,
          "toughness": 1
        }
      ]
    },
    {
      "name": "test_DisturbAura",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brine Comber",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lantern Bearer",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Lanterns' Lift using Disturb",
          "target": "Brine Comber"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spirit Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lanterns' Lift",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Brine Comber",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "test_BestowAura",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brine Comber",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Hopeful Eidolon using bestow",
          "target": "Brine Comber"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spirit Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Brine Comber",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testDisturbSideTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Brine Comber",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hyena Umbra",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Brinebound Gift using Disturb",
          "target": "Kraken Hatchling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Hyena Umbra",
          "target": "Kraken Hatchling"
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
          "name": "Spirit Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brinebound Gift",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kraken Hatchling",
          "power": 1,
          "toughness": 5
        }
      ]
    }
  ]
});
