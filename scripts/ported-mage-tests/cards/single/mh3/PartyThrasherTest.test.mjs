import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/PartyThrasherTest.java",
  "tests": [
    {
      "name": "testPlayFromExile",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Party Thrasher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnarch",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnarch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnarch",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        }
      ]
    },
    {
      "name": "testTryUnplayableCard",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Party Thrasher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnarch",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnarch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Accorder's Shield"
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
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"Should have failed to execute, as Accorder's Shield should not be castable\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Can't find ability to activate command: Cast Accorder's Shield\")) { Assert.fail(\"must throw error about missing ability:\\n\" + e.getMessage()); } }"
        }
      ]
    },
    {
      "name": "testTryPlayFromExileNextTurn",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Party Thrasher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnarch",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnarch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
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
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"Should have failed to execute, as Memnite should not be castable the turn after Party Thrasher Exiled it.\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Can't find ability to activate command: Cast Memnite\")) { Assert.fail(\"must throw error about missing ability:\\n\" + e.getMessage()); } }"
        }
      ]
    }
  ]
});
