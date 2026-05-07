import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh2/AeveProgenitorOozeTest.java",
  "tests": [
    {
      "name": "testAeve",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aeve, Progenitor Ooze",
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
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aeve, Progenitor Ooze"
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
          "name": "Aeve, Progenitor Ooze",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertTokenCount(playerA, aeve, 1)"
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents(StaticFilters.FILTER_PERMANENT_CREATURE, playerA.getId(), currentGame)) { if (permanent.getName().equals(aeve)) { if (permanent instanceof PermanentToken) { Assert.assertEquals(0, permanent.getCounters(currentGame).getCount(CounterType.P1P1)); } else { Assert.assertEquals(1, permanent.getCounters(currentGame).getCount(CounterType.P1P1)); } } }"
        }
      ]
    }
  ]
});
