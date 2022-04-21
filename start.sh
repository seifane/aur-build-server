#!/bin/bash
gpg --import config/key.asc
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman -Syy
./aur-build-server $@