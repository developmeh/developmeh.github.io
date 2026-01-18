#!/bin/bash -e

if [ -d "./.test/bats" ]; then
  echo "Deleting folder $FOLDER"
  rm -rf "./.test/bats/"
fi