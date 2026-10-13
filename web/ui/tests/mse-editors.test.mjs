import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { MseFieldValue, text } from '../src/lib/mse/script-values.mjs';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { loadMseTemplate, MseTemplateContext } from '../src/lib/mse/template.mjs';

test('combined editor emits native separator, prefix, suffix and soft tags', () => {
  const runtime = new MseRuntime({ builtins: coreBuiltins(), variables: { a: new MseFieldValue('A', false), empty: new MseFieldValue('', true), value: '' } });
  assert.equal(runtime.run('combined_editor(field1:a, field2:empty, separator:" / ", prefix:"[", suffix:"]")'), '<prefix>[</prefix>A<sep> / </sep><suffix>]</suffix>');
  assert.equal(runtime.run('combined_editor(field1:a, field2:empty, separator:" / ", suffix:"]", soft_before_empty:true)'), 'A<sep>]<soft> / </soft></sep>');
  assert.equal(runtime.run('combined_editor(field1:empty, field2:empty, separator:" / ", prefix:"[", suffix:"]", hide_when_empty:true)'), '');
  assert.equal(runtime.run('forward_editor(field:a, prefix:"#")'), '<prefix>#</prefix>A');
  assert.throws(() => runtime.run('combined_editor(field1:a, field2:empty)'), { code: 'editor-separators' });
  assert.throws(() => runtime.run('combined_editor(field:"A")'), { code: 'editor-field-type' });
});

async function context() {
  const files = {
    'editor.mse-style/style': 'game: editor\ncard width: 375\ncard height: 523\n',
    'editor.mse-game/game': `card field:
\ttype: text
\tname: first
\tscript: to_upper(input)
card field:
\ttype: text
\tname: second
card field:
\ttype: text
\tname: combined
\tscript: combined_editor(field1:card.first, field2:card.second, separator:" / ", prefix:"[", suffix:"]")
`,
  };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => files[path.key] }), 'editor.mse-style');
  return new MseTemplateContext(template, { card: { first: 'one', second: 'two' } }).initialize();
}

test('combined edits propagate into components and run component scripts', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'combined')), '<prefix>[</prefix>ONE<sep> / </sep>two<suffix>]</suffix>');
  c.setField('card', 'combined', '<prefix>[</prefix>three<sep> / </sep>four<suffix>]</suffix>');
  assert.equal(text(c.field('card', 'combined')), '<prefix>[</prefix>THREE<sep> / </sep>four<suffix>]</suffix>');
  assert.equal(text(c.field('card', 'first')), 'THREE');
  assert.equal(text(c.field('card', 'second')), 'four');
  assert.equal(c.field('card', 'first').isDefault, false);
  c.setField('card', 'first', 'five');
  assert.equal(text(c.field('card', 'combined')), '<prefix>[</prefix>FIVE<sep> / </sep>four<suffix>]</suffix>');
});

test('newer components survive an older combined edit and missing parts clear older components', async () => {
  const c = await context();
  c.setField('card', 'combined', 'older');
  c.setField('card', 'first', 'newer');
  assert.equal(text(c.field('card', 'combined')), '<prefix>[</prefix>NEWER<sep> / </sep><suffix>]</suffix>');
  assert.equal(text(c.field('card', 'second')), '');
});
