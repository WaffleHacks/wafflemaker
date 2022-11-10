CREATE TABLE deployments (
    commit TEXT PRIMARY KEY
);

CREATE TYPE change_action AS ENUM ('modified', 'deleted');

CREATE TABLE changes (
    commit TEXT NOT NULL REFERENCES deployments (commit),
    path TEXT NOT NULL,
    action change_action NOT NULL,
    PRIMARY KEY (commit, path)
);
