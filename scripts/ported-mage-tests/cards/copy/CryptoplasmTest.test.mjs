import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/CryptoplasmTest.java",
  "tests": [
    {
      "name": "testTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cryptoplasm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 2
        }
      ]
    },
    {
      "name": "testFollowedFootsteps",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sigiled Paladin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Followed Footsteps",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cryptoplasm",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Sigiled Paladin"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Followed Footsteps"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Sigiled Paladin[only copy]"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Followed Footsteps",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Cryptoplasm",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sigiled Paladin",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sigiled Paladin",
          "count": 1
        }
      ]
    },
    {
      "name": "testDamageLifelink",
      "operations": [
        {
          "op": "setLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Divinity of Pride",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cryptoplasm",
          "count": 2
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Divinity of Pride"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Divinity of Pride"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Divinity of Pride",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Divinity of Pride:0",
          "attacker": "Divinity of Pride"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Divinity of Pride:1",
          "attacker": "Divinity of Pride"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Cryptoplasm",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Divinity of Pride",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Divinity of Pride",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        }
      ]
    },
    {
      "name": "testTransformMultipleTime",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cryptoplasm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
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
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Craw Wurm"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
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
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        }
      ]
    }
  ]
});
