#!/bin/bash

# check if the binary exists
if [ -f /usr/local/bin/sigcrack ]; then
    # remove the binary
    sudo rm /usr/local/bin/sigcrack
    echo "sigcrack binary removed from /usr/local/bin"
else
    echo "sigcrack binary not found in /usr/local/bin"
fi

# check if the source directory exists
if [ -d "$HOME/sigcrack" ]; then
    # remove the source directory
    rm -rf "$HOME/sigcrack"
    echo "sigcrack source directory removed from $HOME"
else
    echo "sigcrack source directory not found in $HOME"
fi

echo "sigcrack has been uninstalled successfully!"