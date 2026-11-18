import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/PromiseOfTomorrowTest.java",
  "tests": [
    {
      "name": "test_NormalCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Promise of Tomorrow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears@bear",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "@bear.1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "@bear.2"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Promise of Tomorrow",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_Commander_LeaveInZone",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Promise of Tomorrow",
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
          "zone": "COMMAND",
          "player": 0,
          "name": "Balduvian Bears",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Forest",
          "ability": "{T}: Add {G}",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Lightning Bolt",
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Promise of Tomorrow",
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 1,
          "source": "Promise of Tomorrow",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Promise of Tomorrow",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 0,
          "checks": []
        }
      ]
    },
    {
      "name": "test_Commander_MoveToCommandZoneFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Promise of Tomorrow",
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
          "zone": "COMMAND",
          "player": 0,
          "name": "Balduvian Bears",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Forest",
          "ability": "{T}: Add {G}",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Lightning Bolt",
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Promise of Tomorrow",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 1,
          "source": "Promise of Tomorrow",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Promise of Tomorrow",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "command",
            "name": "Balduvian Bears",
            "player": 0
          },
          "count": 1,
          "checks": []
        }
      ]
    }
  ],
  "turnOrder": [0, 3, 2, 1]
});
