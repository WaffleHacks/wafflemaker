# Waffle Maker

WaffleHack's application deployment service built on [Docker](https://docker.com), for running the applications, 
and [Vault](https://vaultproject.io), for storing application secrets. This supersedes [autodeploy](https://github.com/WaffleHacks/autodeploy), 
fixing many of its pain points.

An example service defintion can be found [here](./example-service.toml).


## Introduction

As stated above, Waffle Maker is used to automatically deploy our applications so that development can move as fast as
possible. We use GitHub actions to build Docker images on each push. Waffle Maker receives webhook events from GitHub 
whenever there is a push to the [waffles](https://github.com/WaffleHacks/waffles) repository, and from Docker Hub whenever an image is built.


### Inspiration

This project was inspired by HackGT's [Beekeeper](https://github.com/HackGT/beekeeper) and [Beehive](https://github.com/HackGT/beehive) 
system for managing their deployments. Since we are not running at the scale of HackGT, we have no need for Kubenetes, 
so we opted to use Docker instead. Configuration is managed similarly to Beehive, however we use the TOML format as it 
has fewer quirks and does not depend on indentation.


### Methodology

When [waffles](https://github.com/WaffleHacks/waffles) gets updated, the diff gets inspected, and the deployments get updated accordingly. Database credentials and secrets
are provisioned automatically per container through Vault. Secrets can either be auto-generated for things like session
tokens, or specified directly in Vault. Should a service require web access, the DNS records are
set up through the Cloudflare API.

When an image gets updated, the deployment configs are checked to see if the image should be deployed. If it is deployable, 
then the secrets are pulled from Vault, and the environment variables are populated. The new container is then spun up, 
and once it is stable, the old container is shutdown. If the container needs web access, the appropriate labels are 
applied so [Traefik](https://traefik.io) can route traffic if needed.


## Development

You will need the latest version of [Rust](https://www.rust-lang.org/learn/get-started) installed, along with 
[Docker](https://docs.docker.com/engine/install/) and [Docker Compose](https://docs.docker.com/compose/install/).

You'll also need AWS credentials with at least the [recommended permissions](https://www.vaultproject.io/docs/secrets/aws#example-iam-policy-for-vault).
These credentials will be put in a `.dockerenv` file ([example](./.dockerenv.example)).

If you would like to setup GitHub and/or Discord deployment notifications, you need to get values for them as well. The 
GitHub notifications require a GitHub application id and private key, and the ID of the installation to connect to. Directions
to get these values can be found [here](https://docs.github.com/en/developers/apps/building-github-apps/authenticating-with-github-apps#authenticating-as-an-installation). 
Discord notifications simply require a webhook URL which can be generated in a server's Settings under "Integrations".

To setup your development environment, use the provided Docker compose file:
```shell
docker compose up --build -d
```

You'll then need to create a copy of [`wafflemaker.example.toml`](./wafflemaker.example.toml) and fill in your values:
- dependencies.postgres = `postgres://{{username}}:{{password}}@172.96.0.2:5432/{{database}}`
- dependencies.redis = `redis://172.96.0.4:6379`
- deployment.network = `wafflemaker_default`
- secrets.address = `http://172.96.0.3:8200`

As for `secrets.token`, make sure you have the [Vault CLI](https://www.vaultproject.io/docs/install) installed and run the following command:
```shell
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=dev-token
vault token create -policy=wafflemaker -period=168h
```
This will generate a Vault token with the pre-created [`wafflemaker`](./docker/scripts/wafflemaker.hcl) policy.

Finally, we will need to register the database with Vault. This will allow Vault to dynamically create and manage users
within the database.
```shell
vault write database/config/postgresql \
  plugin_name=postgresql-database-plugin \
  allowed_roles="*" \
  connection_url="postgresql://{{username}}:{{password}}@172.96.0.2:5432/postgres?sslmode=disable" \
  username="postgres" \
  password="postgres"
```
