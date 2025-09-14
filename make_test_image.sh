#!/bin/bash

# This script just makes pictures super tiny so I can upload them
# without accidentally doing a privacy
magick "$1" -resize 16x16 "$1.new"
if [[ -f "$1.new" ]]
then
    exiftool -overwrite_original_in_place -tagsFromFile "$1" "$1.new"
    touch -r "$1" "$1.new"
    mv "$1.new" "$1"
else
    echo "FAILED: $1"
fi
