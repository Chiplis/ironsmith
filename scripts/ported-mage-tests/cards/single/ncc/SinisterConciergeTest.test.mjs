import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/SinisterConciergeTest.java",
  "tests": [
    {
      "name": "testWorking",
      "operations": [
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
          "name": "Sinister Concierge",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bonded Construct",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Sinister Concierge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bonded Construct"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Sinister Concierge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(sinisterConcierge, CounterType.TIME, 3)"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(bondedConstruct, CounterType.TIME, 3)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Sinister Concierge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(sinisterConcierge, CounterType.TIME, 1)"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(bondedConstruct, CounterType.TIME, 1)"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Sinister Concierge",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Sinister Concierge",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinister Concierge",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bonded Construct",
          "count": 1
        }
      ]
    }
  ]
});
