#!/bin/bash

sudo pacman-key --init
./aur-build-worker $@
