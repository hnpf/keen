#!/bin/bash
# keen installer! direct and fast.

set -e

# check for cargo
command -v cargo >/dev/null 2>&1 || { echo >&2 "cargo not found. install rust: https://rustup.rs/"; exit 1; }

VERSION=$(curl -sSL https://raw.githubusercontent.com/hnpf/keen/main/Cargo.toml | grep '^version =' | cut -d '"' -f 2 || echo "latest")

echo "this script will install keen v$VERSION."
for i in {3..1}; do
    echo -ne "starting in $i seconds... (ctrl+c to cancel)\r"
    sleep 1
done

echo -ne "--- let's go ---\n"
sleep 1

TMP_DIR=$(mktemp -d)
echo "--- cloning to $TMP_DIR ---"
git clone https://github.com/hnpf/keen.git "$TMP_DIR" --depth 1 || { echo "clone failed."; exit 1; }

cd "$TMP_DIR" || exit 1

VERSION=$(grep '^version =' Cargo.toml | cut -d '"' -f 2)
echo "--- getting keen v$VERSION ---"

echo "--- building (release mode) ---"

cargo build --release || { echo "build failed."; exit 1; }

echo "--- installing to ~/.local/bin ---"
./target/release/keen --install

echo "--- installation finished ---"
echo "keen v$VERSION is now installed. if ~/.local/bin wasn't in your PATH, follow the instructions above!"

# cleanup
rm -rf "$TMP_DIR"
