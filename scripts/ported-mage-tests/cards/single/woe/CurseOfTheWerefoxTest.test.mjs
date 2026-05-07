import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woe/CurseOfTheWerefoxTest.java",
  "tests": [
    {
      "name": "noTokenCreated",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of the Werefox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azorius First-Wing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of the Werefox",
          "target": "Azorius First-Wing"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wardens, 0)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, azoriusFirstWing, 0)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wardens, 0)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Monster",
          "count": 0
        }
      ]
    },
    {
      "name": "usualBehavior",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of the Werefox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of the Werefox",
          "target": "Nexus Wardens"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "<i>Constellation</i>"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nexus Wardens"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wardens, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, wardens, 2)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Monster",
          "count": 1
        }
      ]
    }
  ]
});
