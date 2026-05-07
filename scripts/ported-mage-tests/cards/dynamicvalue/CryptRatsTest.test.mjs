import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/dynamicvalue/CryptRatsTest.java",
  "tests": [
    {
      "name": "damageOnlyCreatureAndPlayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crypt Rats",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Shivan Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon, Battle-Forged",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
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
          "life": 16
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Crypt Rats",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Swamp\", 0)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Shivan Dragon\", 4)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Gideon, Battle-Forged\", 0)"
        }
      ]
    }
  ]
});
