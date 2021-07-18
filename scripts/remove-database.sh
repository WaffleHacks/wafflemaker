#!/bin/bash

if [[ "$1" == "--help" || "$1" == "-h" ]]; then
  echo "Usage: $0 <connection string> <database>"
  echo "Removes a database and its corresponding a role."
  exit 0
elif [[ "$#" -gt 2 ]]; then
  echo "Too many arguments, run $0 --help for usage information"
  exit 1
elif [[ "$#" -lt 2 ]]; then
  echo "Missing required argument(s), run $0 --help for usage information"
  exit 1
fi

connection_url="$1";
database="$2";

echo "THIS IS AN IRREVERSIBLE DESTRUCTIVE ACTION!"
read -p "Are you sure you want to continue? [N/y] " -n 1 -r
echo
if [[ ! "$REPLY" =~ ^[Yy]$ ]]; then
  echo "Cancelled"
  exit 1
fi

psql "$connection_url" <<EOF
DROP OWNED BY "$database" CASCADE;
DROP DATABASE "$database";
DROP ROLE "$database";
EOF
