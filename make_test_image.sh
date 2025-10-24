#!/bin/bash

# This script just makes pictures super tiny so I can upload them
# without accidentally doing a privacy
if [[ $(exiftool -s3 -MIMEType "$1") =~ ^video/ ]]
then
  echo "video found, using ffmpeg"
  ffmpeg -i "$1"  -vf scale=16:16 "$1.new.mp4"
  mv "$1.new.mp4" "$1.new"
elif [[ $(exiftool -s3 -MIMEType "$1") =~ ^image/ ]]
then
  echo "image found, using magick"
  magick "$1" -resize 16x16 "$1.new"
else
  echo "Neither video nor image; bailing."
  exit 1
fi

if [[ -f "$1.new" ]]
then
  exiftool -all= "$1.new"
  exiftool -overwrite_original_in_place -tagsFromFile "$1" "$1.new"
  touch -r "$1" "$1.new"
  mv "$1.new" "$1"
else
  echo "FAILED: $1"
fi
