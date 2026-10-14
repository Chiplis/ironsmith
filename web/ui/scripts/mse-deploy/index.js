const {resolve, join} = require('node:path');
const {readFileSync} = require('node:fs');
const {pathToFileURL} = require('node:url');

module.exports = {
  async onPostBuild({constants, utils}) {
    const cacheRoot = resolve('.cache/ironsmith-mse');
    try {
      const lock = JSON.parse(readFileSync(join(__dirname, 'upstream.json'), 'utf8'));
      const {installCorpus} = await import(pathToFileURL(join(__dirname, 'install.mjs')).href);
      await utils.cache.restore(cacheRoot);
      await installCorpus({
        lock, cacheRoot,
        destination: join(constants.PUBLISH_DIR, 'ironsmith', 'mse', lock.revision),
      });
      await utils.cache.save(cacheRoot);
    } catch (error) {
      utils.build.failBuild('Could not install the pinned Ironsmith MSE assets', {error});
    }
  },
};
