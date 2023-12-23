#!/bin/bash
gpg --import config/key.asc
./aur-build-server $@