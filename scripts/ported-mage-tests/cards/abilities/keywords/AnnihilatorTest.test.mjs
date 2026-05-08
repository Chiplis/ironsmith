import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/AnnihilatorTest.java",
  "tests": [
    {
      "name": "testCardsSacrificedToAnnihilatorTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Emrakul, the Aeons Torn",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Emrakul, the Aeons Torn",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
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
          "op": "assertLife",
          "player": 0,
          "life": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testCardItThatBetrays",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cauldron Haze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Rector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "It That Betrays",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "It That Betrays",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Academy Rector"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Cauldron Haze",
          "target": "Academy Rector"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Whenever an opponent"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "persist"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cauldron Haze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Academy Rector",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Academy Rector",
          "power": 0,
          "toughness": 1
        }
      ]
    }
  ]
});
