import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/prevention/PreventAllDamageTest.java",
  "tests": [
    {
      "name": "test_SafePassage",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Safe Passage",
          "count": 1
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
          "name": "Pillarfield Ox",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Safe Passage"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Silvercoat Lion",
          "attacker": "Pillarfield Ox"
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
          "name": "Safe Passage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
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
        }
      ]
    },
    {
      "name": "test_EtherealHaze",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Ethereal Haze",
          "count": 1
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
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Ethereal Haze"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Silvercoat Lion",
          "attacker": "Silvercoat Lion"
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
          "name": "Ethereal Haze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "test_EnergyStorm",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
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
          "name": "Energy Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Fire Ambush",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Energy Storm"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Abbey Griffin",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Abbey Griffin"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Fire Ambush",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Fire Ambush",
          "target": "Abbey Griffin"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Energy Storm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fire Ambush",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "test_ArmoredTransport",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Armored Transport",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Armored Transport",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Armored Transport",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Armored Transport",
          "attacker": "Armored Transport"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Armored Transport",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Armored Transport",
          "count": 1
        }
      ]
    },
    {
      "name": "test_HazeFrog",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glory Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jwari Scuttler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lagac Lizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Daggerback Basilisk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Haze Frog",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Glory Seeker",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Jwari Scuttler",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Lagac Lizard",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Haze Frog"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Daggerback Basilisk",
          "attacker": "Lagac Lizard"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Haze Frog",
          "attacker": "Glory Seeker"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Glory Seeker",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Jwari Scuttler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lagac Lizard",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Daggerback Basilisk",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Haze Frog",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
