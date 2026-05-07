import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/SixTest.java",
  "tests": [
    {
      "name": "test_Retrace_Simple",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Six",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Grizzly Bears with retrace",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears with retrace"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Taiga"
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Taiga",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Retrace_Flash_YourTurnOnly",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Six",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Ambush Viper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Ambush Viper with retrace",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Ambush Viper with retrace",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Cast Ambush Viper with retrace",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Ambush Viper with retrace"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Taiga"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ambush Viper",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Taiga",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Retrace_Adventure",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Six",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Bonecrusher Giant with retrace",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Stomp with retrace",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bonecrusher Giant with retrace"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Taiga"
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
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Taiga",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Retrace_MDFC",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Six",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Egon, God of Death",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Egon, God of Death with retrace",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Throne of Death with retrace",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Throne of Death with retrace"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Taiga"
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
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Throne of Death",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Taiga",
          "count": 1
        }
      ]
    }
  ]
});
