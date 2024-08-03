#!/bin/bash

# check if cargo is installed
if ! command -v cargo &> /dev/null
then
    echo "cargo could not be found, please install Rust and Cargo first."
    exit
fi

# clone the repository
git clone https://github.com/yourusername/sigcrack.git
cd sigcrack

# build the project
cargo build --release

# copy the binary to /usr/local/bin
sudo cp target/release/sigcrack /usr/local/bin/

# cleanup
cd ..
rm -rf sigcrack

echo "sigcrack has been installed successfully!"