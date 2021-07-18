# Allow reading and writing to services configuration KV
path "services/data/+" {
  capabilities = ["create", "update", "read"]
}


# Allow configuring and reading credentials for the database
path "database/roles/+" {
  capabilities = ["list", "create", "delete"]
  allowed_parameters = {
    "db_name" = ["postgresql"]
    "default_ttl" = []
    "creation_statements" = []
  }
}

path "database/creds/+" {
  capabilities = ["read"]
}


# Generate AWS credentials
path "aws/creds/+" {
  capabilities = ["read"]
}
