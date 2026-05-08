import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/HydroManFluidFelonTest.java",
  "tests": [
    {
      "name": "testHydroManFluidFelon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hydro-Man, Fluid Felon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fugitive Wizard"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hydro-Man, Fluid Felon",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hydro-Man, Fluid Felon",
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
          "op": "unsupported",
          "source": "assertType(hydroManFluidFelon, CardType.LAND, true)"
        },
        {
          "op": "unsupported",
          "source": "assertNotType(hydroManFluidFelon, CardType.CREATURE)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(hydroManFluidFelon, false)"
        }
      ]
    }
  ]
});
