#!/bin/sh

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
echo "git config --add core.hookspath $SCRIPT_DIR"
git config --add core.hookspath $SCRIPT_DIR
echo "done initialize."