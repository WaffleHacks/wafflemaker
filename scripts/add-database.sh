#!/bin/bash

if [[ "$1" == "--help" || "$1" == "-h" ]]; then
  echo "Usage: $0 <connection string> <database>"
  echo "Automatically creates a database and configures a role with the same name."
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

psql "$connection_url" <<EOF
CREATE DATABASE "$database";
\c "$database"

REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE "$database" FROM PUBLIC;

CREATE ROLE "$database";
GRANT CONNECT ON DATABASE "$database" TO "$database";
GRANT USAGE, CREATE ON SCHEMA public TO "$database";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO "$database";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO "$database";
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO "$database";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE ON SEQUENCES TO "$database";

CREATE OR REPLACE FUNCTION trg_create_set_owner()
  RETURNS event_trigger
  LANGUAGE plpgsql
AS \$\$
DECLARE
  obj record;
BEGIN
  FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands() WHERE command_tag='CREATE TABLE'
  LOOP
    EXECUTE format('ALTER TABLE %s OWNER TO "$database"', obj.object_identity);
  END LOOP;
END;
\$\$;
CREATE EVENT TRIGGER trg_create_set_owner
  ON ddl_command_end
  WHEN tag IN ('CREATE TABLE')
  EXECUTE PROCEDURE trg_create_set_owner();
EOF
