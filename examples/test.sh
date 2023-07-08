#!/usr/bin/env bash

# Check if both arguments were provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 server_url path"
    exit 1
fi

# Get current datetime in the required format
datetime=$(date -u +"%Y-%m-%dT%H:%M:%S+0000")

# The server url provided as first argument
server_url=$1

# The path provided as second argument
path=$2

# Run the cURL command
curl -X POST -H 'Content-Type: application/json' -d "{
    \"datetime\": \"$datetime\",
    \"path\": \"$path\",
    \"values\": {\"2\":5.4}
}" "$server_url/api/actors/actors/one"

# Capture the return code of the curl command
rc=$?

# Return the exit code of the cURL command
exit $rc
