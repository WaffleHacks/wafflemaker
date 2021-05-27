# Waffle Maker

WaffleHack's application deployment service built on [Docker](https://docker.com), for running the applications, 
and [Vault](https://vaultproject.io), for storing application secrets. This supersedes [autodeploy](https://github.com/WaffleHacks/autodeploy), 
fixing many of its pain points.


## Introduction

As stated above, Waffle Maker is used to automatically deploy our applications so that development can move as fast as
possible. We use GitHub actions to build Docker images on each push. Waffle Maker receives webhook events from GitHub 
whenever there is a push to the []() repository, and from Docker Hub whenever an image is built.


### Inspiration

This project was inspired by HackGT's [Beekeeper](https://github.com/HackGT/beekeeper) and [Beehive](https://github.com/HackGT/beehive) 
system for managing their deployments. Since we are not running at the scale of HackGT, we have no need for Kubenetes, 
so we opted to use Docker instead. Configuration is managed similarly to Beehive, however we use the TOML format as it 
has fewer quirks and does not depend on indentation.


### Methodology

When () gets updated, the diff is inspected, and the deployments are updated accordingly. Database credentials and secrets
are provisioned automatically per container through Vault. Secrets can either be auto-generated for things like session
tokens, or specified directly in Vault. Should a service require web access, the DNS records are
set up through the Cloudflare API.

When an image gets updated, the deployment configs are checked to see if the image should be deployed. If it is deployable, 
then the secrets are pulled from Vault, and the environment variables are populated. The new container is then spun up, 
and once it is stable, the old container is shutdown. If the container needs web access, the appropriate labels are 
applied so [Traefik](https://traefik.io) can route traffic if needed.
