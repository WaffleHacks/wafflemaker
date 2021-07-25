# Allow reading and writing to services configuration KV
path "services/data/+" {
  capabilities = ["create", "update", "read"]
}


# Allow configuring and reading credentials for the database
path "database/roles/+" {
  capabilities = ["list", "create", "delete"]
  allowed_parameters = {
    "db_name" = ["postgresql"]
    "*" = []
  }
}

path "database/creds/+" {
  capabilities = ["read"]
}


# Generate AWS credentials
path "aws/creds/+" {
  capabilities = ["read"]
}


# Allow renewing and revoking leases
path "sys/leases/renew" {
  capabilities = ["update"]
}

path "sys/leases/renew" {
  capabilities = ["update"]
}
