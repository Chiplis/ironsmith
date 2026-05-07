import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/CathedralMembraneTest.java",
  "tests": [
    {
      "name": "testMembraneTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Autochthon Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cathedral Membrane",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Miraculous Recovery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cathedral Membrane"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Autochthon Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Cathedral Membrane",
          "attacker": "Autochthon Wurm"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 6)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "unsupported",
          "source": "assertTapped(wurm, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(gigantosaurus, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(moraug, false)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cathedral Membrane",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 6)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, gigantosaurus, 0)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, moraug, 0)"
        }
      ]
    },
    {
      "name": "testMembraneTriggerAgain",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Autochthon Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gigantosaurus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cathedral Membrane",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Miraculous Recovery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cathedral Membrane"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Autochthon Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Cathedral Membrane",
          "attacker": "Autochthon Wurm"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 6)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Miraculous Recovery",
          "target": "Cathedral Membrane"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Autochthon Wurm",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Gigantosaurus",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Cathedral Membrane",
          "attacker": "Gigantosaurus"
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
          "op": "unsupported",
          "source": "assertTapped(wurm, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(gigantosaurus, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(moraug, false)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cathedral Membrane",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 3
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 6)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, gigantosaurus, 1 + 6)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, moraug, 0)"
        }
      ]
    }
  ]
});
