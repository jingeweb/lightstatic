#! /usr/bin/env node
'use strict';

import { program } from 'commander';
import colors from 'colors';
import { LightStaticOptions } from './common';
import { bootstrap } from './server';
import { getBaseHref } from './util';
import { logger } from './logger';

const options: LightStaticOptions = {} as unknown as LightStaticOptions;

program
  .name('lightstatic')
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  .version(require('../package.json').version)
  .usage('[path] [options]')
  .arguments('[path]')
  .action((root) => (options.root = root || '.'))
  .option('-b, --bind <string>', 'ip address to bind', '0.0.0.0')
  .option(
    '-p, --port <number>',
    'port to listen. if the specified port is not avaiable, lightstatic will find a free port instead',
    '8080',
  )
  .option('-g, --gzip', 'gzip encode response content', false)
  .option('-o, --open', 'open browser window after starting the server', false)
  .option('-5, --html5', 'use html5 mode url route(history api fallback like webpack-dev-server)', false)
  .option('-i, --index <file>', 'index file to redirect under html5 mode', 'index.html')
  .option('-d, --delay <number>', 'delay in milliseconds for response', '0')
  .option(
    '-m, --middleware <file>',
    'middleware file to use. see https://github.com/jingeweb/lightstatic#middleware',
    '',
  )
  .option('-s, --store-in-memory', 'store(cache) static files into memory', false)
  .option('-r, --cache-forever-regexp <regexp>', 'cache file forever if match regexp', '')
  .option('-l, --log-dir <directory>', 'write logs to directory', '')
  .option('--base-href <path>', 'server base href, useful when under nginx subpath', '/')
  .option('--no-access-log', 'do not print access log', false)
  .option('--no-color', 'disable color log', false)
  .parse(process.argv);

Object.assign(options, program.opts());

options.port = Number(options.port);
options.delay = Number(options.delay);
options.baseHref = getBaseHref(options.baseHref);

if (options.logDir || options.noColor) {
  colors.disable();
}

if (options.cacheForeverRegexp && !options.storeInMemory) {
  console.error('--cache-forever-regexp only effect with --store-in-memory');
  process.exit(-1);
}

logger.initialize(options);

bootstrap(options);
