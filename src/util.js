const fs = require('fs').promises;
const path = require('path');
const homeDir = require('os').homedir();

const CWD = process.cwd();

function resolvePath(filepath) {
  if (filepath.startsWith('~')) {
    return path.join(homeDir, filepath.substring(1));
  } else {
    return path.resolve(CWD, filepath);
  }
}

async function getStat(file) {
  try {
    return await fs.stat(file);
  } catch(ex) {
    return null;
  }
}

function getBaseHref(href) {
  href = href || '/';
  if (!href.startsWith('/')) {
    href = '/' + href;
  }
  if (!href.endsWith('/')) {
    href += '/';
  }
  return href;
}


module.exports = {
  getBaseHref,
  getStat,
  resolvePath
};

