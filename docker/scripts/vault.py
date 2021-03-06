#!/usr/bin/env python3

import requests
from os import environ
from time import sleep
from sys import exit
from typing import Any, Dict, Optional

ADDRESS = environ.get("VAULT_ADDR", "http://172.96.0.3:8200")
HEADERS = {
    "X-Vault-Token": environ.get("VAULT_TOKEN", "dev-token"),
    "Accepts": "application/json",
}

AWS_ACCESS_KEY_ID = environ.get("AWS_ACCESS_KEY_ID")
AWS_SECRET_ACCESS_KEY = environ.get("AWS_SECRET_ACCESS_KEY")
AWS_REGION = environ.get("AWS_REGION")


def send_request(
    method: str,
    path: str,
    body: Optional[Dict[str, Any]] = None,
    error_message: str = "",
    should_error=True,
    deserialize=False,
):
    r = requests.request(method, f"{ADDRESS}{path}", json=body, headers=HEADERS)
    if r.status_code not in [200, 201, 202, 203, 204, 205, 206] and should_error:
        print(f"{error_message}: ({r.status_code}) {r.text}")
        exit(1)

    if deserialize:
        return r.json()


print("Ensuring Vault is initialized...")
success = False
for attempt in range(10):
    try:
        response = requests.get(f"{ADDRESS}/v1/sys/init")
        if response.json().get("initialized"):
            success = True
            break
    except ConnectionError:
        print("\tconnection timeout, retrying in 1s")
        pass

    sleep(1)

if not success:
    print(f"Failed to connect to Vault: ({response.status_code}) {response.text}")
    exit(1)

print("Initializing database engine...")
send_request(
    "POST",
    "/v1/sys/mounts/database",
    {"type": "database"},
    "Failed to initialize database engine",
)

print("Initializing AWS engine...")
send_request(
    "POST", "/v1/sys/mounts/aws", {"type": "aws"}, "Failed to initialize AWS engine"
)

print("Setting root credentials...")
send_request(
    "POST",
    "/v1/aws/config/root",
    {
        "access_key": AWS_ACCESS_KEY_ID,
        "secret_key": AWS_SECRET_ACCESS_KEY,
        "region": AWS_REGION,
    },
    "Failed to configure AWS engine",
)

print("Initializing services KV engine...")
send_request(
    "POST",
    "/v1/sys/mounts/services",
    {"type": "kv", "options": {"version": "2"}},
    "Failed to initialize services KV engine",
)

print("Creating `wafflemaker` role...")
role = open("./wafflemaker.hcl", "r")
send_request(
    "POST",
    "/v1/sys/policies/acl/wafflemaker",
    {"policy": role.read()},
    "Failed to create wafflemaker policy",
)

print("Getting token...")
token = send_request(
    "POST",
    "/v1/auth/token/create-orphan",
    {"policies": ["wafflemaker"]},
    "Failed to create wafflemaker token",
    deserialize=True,
)
print(token.get("auth").get("client_token"))

print("Successfully setup Vault")
