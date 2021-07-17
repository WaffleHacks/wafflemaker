# Allow reading and writing to services configuration KV
path "services/data/+" {
  capabilities = ["create", "update", "read"]
}


# Allow configuring and reading credentials for the database
path "database/static-roles/+" {
  capabilities = ["list", "create", "delete"]
  allowed_parameters = {
    "db_name" = ["postgresql"],
    "rotation_statements" = []
    "username" = []
    "rotation_period" = []
  }
}

path "database/static-creds/+" {
  capabilities = ["read"]
}

path "database/rotate-role/+" {
  capabilities = ["update"]
}


# Generate AWS credentials
path "aws/creds/+" {
  capabilities = ["read"]
}
