import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseTemplateContext, loadMseTemplate } from '../src/lib/mse/template.mjs';
import { text } from '../src/lib/mse/script-values.mjs';

async function context() {
  const files = {
    'choices.mse-style/style': 'game: choices\ncard width: 375\ncard height: 523\n',
    'choices.mse-game/game': `card field:
\ttype: boolean
\tname: flag
card field:
\ttype: choice
\tname: group
\tchoice:
\t\tname: outer
\t\tchoice: first
\t\tchoice:
\t\t\tname: nested
\t\t\tdefault name: Standard
\t\t\tchoice: child
\tchoice: other
card field:
\ttype: choice
\tname: group default
\tchoice:
\t\tname: group
\t\tdefault name: Standard
\t\tchoice: child
card field:
\ttype: choice
\tname: implicit
card field:
\ttype: multiple choice
\tname: colors
\tchoice: white
\tchoice: blue
\tchoice: black
\tempty choice: colorless
card field:
\ttype: multiple choice
\tname: exclusive
\tinitial: white, blue
\tchoice: white
\tchoice: blue
\tscript: exclusive_choice(input, choices:"white, blue")
card field:
\ttype: text
\tname: primary
\tscript: primary_choice(card.group)
`,
  };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => files[path.key] }), 'choices.mse-style');
  return new MseTemplateContext(template).initialize();
}

test('choice defaults follow native group IDs and primary_choice finds the top-level group', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'group')), 'outer first');
  assert.equal(text(c.field('card', 'group_default')), 'group');
  assert.equal(text(c.field('card', 'implicit')), 'implicit');
  assert.equal(text(c.field('card', 'flag')), 'yes');
  assert.equal(c.runtime.run('primary_choice(card.flag)'), 'yes');
  assert.equal(text(c.field('card', 'primary')), 'outer');
  c.setField('card', 'group', 'outer nested child');
  assert.equal(text(c.field('card', 'primary')), 'outer');
  c.setField('card', 'group', 'other');
  assert.equal(text(c.field('card', 'primary')), 'other');
});

test('multiple choices normalize duplicates, unknown choices, prefixes and declaration order', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'colors')), 'colorless');
  c.setField('card', 'colors', ' black, white, black, unknown, bluetail');
  assert.equal(text(c.field('card', 'colors')), 'white, blue, black');
  c.setField('card', 'colors', 'unknown');
  assert.equal(text(c.field('card', 'colors')), 'colorless');
});

test('last_change is empty initially and explicitly selects the preferred option after an edit', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'exclusive')), 'white');
  c.setField('card', 'exclusive', 'white, blue', { lastChange: 'blue' });
  assert.equal(text(c.field('card', 'exclusive')), 'blue');
  c.invalidate();
  assert.equal(text(c.field('card', 'exclusive')), 'blue');
});
