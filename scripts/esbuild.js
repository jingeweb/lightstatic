const path = require('path');
const { build } = require('esbuild');
const pkg = require('../package.json');

const externals = new Set(Object.keys(pkg.dependencies));
externals.delete('get-port');
externals.add('../package.json');

(async () => {
  const result = await build({
    bundle: true,
    watch: 'WATCH' in process.env,
    entryPoints: [path.resolve(__dirname, '../src/index.ts')],
    loader: {
      '.ts': 'ts',
      '.html': 'text',
    },
    target: 'node18',
    platform: 'node',
    format: 'iife',
    charset: 'utf8',
    external: [...externals],
    outfile: path.resolve(__dirname, '../lib/index.js'),
    sourcemap: true,
    sourcesContent: false,
  });
  if (result.errors?.length) {
    throw result.errors;
  }
  if (result.warnings?.length) {
    console.warn(result.warnings);
  }
})().catch((ex) => {
  console.error(ex);
  process.exit(-1);
});
