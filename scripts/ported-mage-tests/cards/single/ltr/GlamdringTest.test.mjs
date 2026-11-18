import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/GlamdringTest.java",
  "tests": [
    {
      "name": "tooExpensiveToCastForFree",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Healing Salve",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "In Garruk's Wake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blue Sun's Zenith",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glamdring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blur Sliver",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}",
          "target": "Blur Sliver"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Blur Sliver",
          "defender": 1
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Glamdring",
          "description": "Choose one",
          "options": [
            "Cast Lightning Bolt",
            "Cast Healing Salve"
          ],
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "In Garruk's Wake",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "canCastForFree",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Healing Salve",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "In Garruk's Wake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blue Sun's Zenith",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glamdring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blur Sliver",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}",
          "target": "Blur Sliver"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Blur Sliver",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "enoughCardsInGraveyardToCastInGarruksWake",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Healing Salve",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "In Garruk's Wake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blue Sun's Zenith",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glamdring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blur Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Serum Visions",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Brainstorm",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elder Deep-Fiend",
          "count": 2,
          "tapped": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Elder Deep-Fiend",
          "tapped": true,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Glamdring",
          "ability": "Equip {3}",
          "target": "Blur Sliver"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Glamdring",
          "abilityKind": "Activated",
          "objectTarget": "Blur Sliver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "attachment": "Glamdring",
          "target": "Blur Sliver",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blur Sliver",
          "power": 14,
          "toughness": 2,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Serum Visions",
          "count": 7,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Brainstorm",
          "count": 5,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Blur Sliver",
          "defender": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "life": 6,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "count": 1,
          "source": "Glamdring",
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "In Garruk's Wake"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "count": 1,
          "source": "In Garruk's Wake"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "name": "Elder Deep-Fiend",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "name": "Elder Deep-Fiend",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "name": "In Garruk's Wake",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "life": 6,
          "seatIndependent": true
        }
      ]
    }
  ]
});
