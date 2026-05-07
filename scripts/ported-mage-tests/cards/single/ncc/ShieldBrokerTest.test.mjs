import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/ShieldBrokerTest.java",
  "tests": [
    {
      "name": "testNonCommander",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shield Broker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shield Broker"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "counter": "SHIELD",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Rograkh, Son of Rohgahh"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "counter": "SHIELD",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        }
      ]
    },
    {
      "name": "testCommander",
      "operations": [
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": "playerD",
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shield Broker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Rograkh, Son of Rohgahh"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shield Broker"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "counter": "SHIELD",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rograkh, Son of Rohgahh",
          "count": 1
        }
      ]
    }
  ]
});
