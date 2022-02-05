gpg --import config/key.asc
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman-key --refresh-keys
sudo pacman -Syy
sudo pacman -Sy archlinux-keyring
./aur-build-server --sign