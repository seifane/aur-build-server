#!/bin/bash

if [ -n $SIGN_KEY_PATH ]; then
  echo "Trying to import gpg key $SIGN_KEY_PATH"
  gpg --import $SIGN_KEY_PATH
fi

./aur-build-server $@