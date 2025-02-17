#!/bin/sh
# download_makefiles_sh
#
# SPDX-FileCopyrightText: 2024 Softbear, Inc.
# SPDX-License-Identifier: AGPL-3.0-or-later

MAIN=https://raw.githubusercontent.com/SoftbearStudios/kodiak/refs/heads/main

if [ ! -d makefiles ] ; then
    mkdir makefiles
fi

echo "*" > makefiles/.gitignore

for f in client game server ; do
    echo "Downloading $f.mk"
    wget -o makefiles/$f.log -O makefiles/$f.mk $MAIN/makefiles/$f.mk
done
