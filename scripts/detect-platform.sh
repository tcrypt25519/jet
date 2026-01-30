#!/bin/bash

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "darwin"
elif [[ -d "/data/data/com.termux" ]]; then
    echo "termux"
elif [[ -f "/etc/debian_version" ]] || [[ -f "/etc/lsb-release" ]]; then
    echo "debian"
else
    echo "unknown"
fi
