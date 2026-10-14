import {readFile} from 'node:fs/promises';
import upstream from '../../src/lib/mse/upstream.mjs';

export function mseDeploymentPlugin() {
  return {
    name: 'ironsmith-mse-deployment',
    apply: 'build',
    async generateBundle() {
      const emit = (name, source) => this.emitFile({type: 'asset', fileName: `deploy/mse/${name}`, source});
      for (const name of ['index.js', 'install.mjs']) {
        emit(name, (await readFile(new URL(name, import.meta.url), 'utf8')).replaceAll('../../src/lib/mse/','./'));
      }
      for(const name of ['font-metadata.mjs','font-resources.mjs','document.mjs'])emit(name,await readFile(new URL(`../../src/lib/mse/${name}`,import.meta.url),'utf8'));
      emit('package.json', JSON.stringify({name: 'netlify-plugin-ironsmith-mse', version: '1.0.0', type: 'commonjs'}));
      emit('manifest.yml', 'name: netlify-plugin-ironsmith-mse\n');
      emit('upstream.json', JSON.stringify(upstream));
    },
  };
}
