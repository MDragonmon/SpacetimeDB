#!/bin/bash

set -euo pipefail

usage() {
	echo "Usage: upload.sh <ssh-key-file-path> <hostname>"
}

if [ "$#" != 2 ] ; then
	usage
	exit 1
fi

if [ ! -f "$1" ] ; then
	usage
	echo "File not found: $1"
	exit
fi

echo "Make sure you have used \"export.sh\" to regenerate the SpacetimeDBUnitySDK."
echo
echo "We will be uploading this SDK to $2"
echo "Your current branch is $(git rev-parse --abbrev-ref HEAD)"
echo
echo "If everything looks correct, press [Enter] now to continue."
read -rp ""

scp -oStrictHostKeyChecking=no -i "$1" "SpacetimeDBUnitySDK.unitypackage" "root@${2}:/var/www/sdk/SpacetimeDBUnitySDK.unitypackage"
ssh -oStrictHostKeyChecking=no -i "$1" "root@${2}" "chown -R jenkins:jenkins /var/www/sdk"
echo "Upload success."
