import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/counter/AddingCountersToPermanentsTest.java",
  "tests": [
    {
      "name": "testBlackSunsZenith",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
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
          "player": 0,
          "name": "Witch's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Black Sun's Zenith",
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
          "name": "Witch's Familiar",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Black Sun's Zenith"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Witch's Familiar",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Witch's Familiar",
          "power": 0,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Witch's Familiar",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Witch's Familiar",
          "power": 0,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testFairgroundsTrumpeter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fairgrounds Trumpeter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Woodland Wanderer",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Woodland Wanderer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Fairgrounds Trumpeter"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Woodland Wanderer",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fairgrounds Trumpeter",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testSoulstingerNormal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soulstinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Turn to Slag",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soulstinger"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Turn to Slag",
          "target": "Soulstinger"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Soulstinger",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Turn to Slag",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Pillarfield Ox",
          "power": 0,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testSoulstinger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soulstinger",
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
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Turn to Slag",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soulstinger"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Turn to Slag",
          "target": "Soulstinger"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Soulstinger",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Turn to Slag",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "HapatraVizierOfPoisons",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hapatra, Vizier of Poisons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kitchen Finks",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Kitchen Finks"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When "
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kitchen Finks",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 1
        }
      ]
    }
  ]
});
