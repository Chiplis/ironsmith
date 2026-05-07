import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/sld/LaraCroftTombRaiderTest.java",
  "tests": [
    {
      "name": "test_Lara_Permission_Over_Two_Turns",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lara Croft, Tomb Raider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Gerrard's Hourglass Pendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Lara Croft, Tomb Raider",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Gerrard's Hourglass Pendant"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Lara Croft, Tomb Raider",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerA.TARGET_SKIP"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "label": "Cast Gerrard's Hourglass Pendant",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gerrard's Hourglass Pendant"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gerrard's Hourglass Pendant",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Wastes",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        }
      ]
    }
  ]
});
