#! /usr/bin/env node
'use strict';

const program = require('commander');
const pkg = require('../package.json');
const {
  bootstrap
} = require('../src/server');

const options = {};

program
 .storeOptionsAsProperties(false)
 .name(pkg.name)
 .version(pkg.version)
 .usage('[path] [options]')
 .arguments('[path]').action(root => options.root = root || '.')
 .option('-b, --bind <string>', 'ip address to bind', '0.0.0.0')
 .option('-p, --port <number>', 'port to listen. if the specified port is not avaiable, sfserver will find a free port instead', 8080)
 .option('-g, --gzip', 'gzip encode response content', false)
 .option('-o, --open', 'open browser window after starting the server', false)
 .option('-5, --html5', 'use html5 mode url route(history api fallback like webpack-dev-server)', false)
 .option('-i, --index <file>', 'index file to redirect under html5 mode', 'index.html')
 .option('-d, --delay <number>', 'delay in milliseconds for response' , 0)
 .option('-m, --middleware <file>', 'middleware file to use')
//  .option('--cert <path>', 'certificate to enable https. NOT implement yet!')
 .option('--base-href <path>', 'server base href, useful when under nginx subpath')
//  .option('--proxy-prefix <string>', 'proxy request to proxy-remote server if request url starts with proxy-prefix', '/__api/')
//  .option('--proxy-remote <url>', 'remote server to proxy request. if not set, sfserver won\'t do proxy')
 .option('--no-access', 'do not print access log')
 .option('--no-log', 'do not print any log expect errors', false)
 .option('--no-color', 'disable color log')
 .parse(process.argv);


Object.assign(options, program.opts());
if (!options.log) {
  options.access = false;
}
options.port = Number(options.port);
options.delay = Number(options.delay);

bootstrap(options);
