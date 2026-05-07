import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/ShiftingWoodlandTest.java",
  "tests": [
    {
      "name": "test_NoDelirium",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "<i>Delirium</i> &mdash; {2}{G}{G}:",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_Copy_StopEndOfTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yavimaya Coast",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}. {this} deals 1 damage to you.",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Delirium</i> &mdash; {2}{G}{G}:",
          "target": "Grizzly Bears"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
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
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(woodland, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(woodland, CardType.LAND, true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, woodland, \"Green\", false)"
        }
      ]
    },
    {
      "name": "test_Copy_StopOnLTB",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yavimaya Coast",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shoot the Sheriff",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}. {this} deals 1 damage to you.",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Delirium</i> &mdash; {2}{G}{G}:",
          "target": "Grizzly Bears"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Shoot the Sheriff",
          "target": "Grizzly Bears"
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
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Copy_StopOnLTB_DeathTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yavimaya Coast",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shoot the Sheriff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Anodet Lurker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}. {this} deals 1 damage to you.",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Delirium</i> &mdash; {2}{G}{G}:",
          "target": "Anodet Lurker"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Anodet Lurker",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Shoot the Sheriff",
          "target": "Anodet Lurker"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} dies"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Copy_MDFC",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yavimaya Coast",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shifting Woodland",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Drowner of Truth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}. {this} deals 1 damage to you.",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Delirium</i> &mdash; {2}{G}{G}:",
          "target": "Drowner of Truth"
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
          "name": "Drowner of Truth",
          "count": 1
        }
      ]
    }
  ]
});
