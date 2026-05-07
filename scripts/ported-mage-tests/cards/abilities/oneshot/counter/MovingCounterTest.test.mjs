import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/counter/MovingCounterTest.java",
  "tests": [
    {
      "name": "testWeaponRack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fathom Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Weapon Rack",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Weapon Rack"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Move a",
          "target": "Fathom Mage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Weapon Rack",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Fathom Mage",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testArcboundFiend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Fiend",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hagra Constrictor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcbound Fiend"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Hagra Constrictor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hagra Constrictor"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Arcbound Fiend",
          "counter": "P1P1",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Hagra Constrictor",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testCantBeCounteredNormal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
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
          "zone": "HAND",
          "player": 0,
          "name": "Bioshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Protean Hydra",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Protean Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Bioshift",
          "target": "Protean Hydra^Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bioshift",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Protean Hydra",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Protean Hydra",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testFateTransfer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Noxious Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ruin Processor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Fate Transfer",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Noxious Hatchling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Fate Transfer",
          "target": "Noxious Hatchling^Ruin Processor"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fate Transfer",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Noxious Hatchling",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ruin Processor",
          "power": 3,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testLeechBonder",
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
          "zone": "HAND",
          "player": 0,
          "name": "Leech Bonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ley Druid",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Leech Bonder"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Leech Bonder",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{U},",
          "target": "Leech Bonder"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ley Druid",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Leech Bonder",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
