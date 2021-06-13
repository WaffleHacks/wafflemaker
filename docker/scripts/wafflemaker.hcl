# Allow reading and writing to services configuration KV
path "services/data/+" {
  capabilities = ["create", "update", "read"]
}


# Allow configuring and reading credentials for the database
path "database/static-roles/+" {
  capabilities = ["create", "delete", "update"]
  allowed_parameters = {
    "db_name" = ["postgresql"]
  }
}

path "database/static-creds/+" {
  capabilities = ["read"]
}


# Generate AWS credentials
path "aws/creds/+" {
  capabilities = ["read"]
}
