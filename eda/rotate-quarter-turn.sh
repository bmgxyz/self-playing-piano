#!/usr/bin/env bash

# TODO upstream rotation into gctk
sed 's/X/TEMP/g' | sed 's/Y/X-/g' | sed 's/TEMP/Y/g' | gctk translate -x 60.65002 <&0
