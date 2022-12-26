CREATE TABLE services (
    id TEXT PRIMARY KEY,
    spec JSONB NOT NULL,
    domain TEXT DEFAULT NULL,
    path TEXT NOT NULL DEFAULT '/'
);

CREATE TYPE containers_status AS ENUM ('configuring', 'pulling', 'creating', 'starting', 'healthy', 'unhealthy', 'stopped');

CREATE TABLE containers (
    service TEXT PRIMARY KEY REFERENCES services (id),
    id TEXT NOT NULL,
    image TEXT NOT NULL,
    status containers_status NOT NULL DEFAULT 'configuring'
);

CREATE TABLE leases (
    service TEXT NOT NULL REFERENCES services (id),
    id TEXT NOT NULL,
    expiration TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (service, id)
);

CREATE INDEX ON leases (id);