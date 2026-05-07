import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pc2/FracturedPowerstoneTest.java",
  "tests": [
    {
      "name": "test_FracturedPowerstone_Single",
      "operations": [
        {
          "op": "unsupported",
          "source": "addPlane(playerA, Planes.PLANE_HEDRON_FIELDS_OF_AGADEEM)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fractured Powerstone",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}: Roll the planar"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Roll the planar"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Eldrazi Token",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Fractured Powerstone",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "test_FracturedPowerstone_NoCost",
      "operations": [
        {
          "op": "unsupported",
          "source": "addPlane(playerA, Planes.PLANE_HEDRON_FIELDS_OF_AGADEEM)"
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
          "name": "Fractured Powerstone",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}: Roll the planar"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Roll the planar"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}: Roll the planar"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Eldrazi Token",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 1
        }
      ]
    }
  ]
});
