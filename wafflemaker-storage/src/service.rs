use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, PgPool};
use time::OffsetDateTime;
use wafflemaker_service::Service as ServiceSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Service {
    pub id: String,
    pub spec: ServiceSpec,
    pub domain: Option<String>,
    pub path: String,
}

macro_rules! service_from_record {
    () => {
        |r| {
            use serde::de::IntoDeserializer;

            // TODO: handle error properly
            let spec = ServiceSpec::deserialize(r.spec.into_deserializer()).unwrap();

            Ok(Service {
                id: r.id,
                spec,
                domain: r.domain,
                path: r.path,
            })
        }
    };
}

impl Service {
    /// Get all the services
    pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<Service>> {
        query!("SELECT * FROM services")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(service_from_record!())
            .collect()
    }

    /// Find a service by its id
    pub async fn find<S>(id: S, pool: &PgPool) -> sqlx::Result<Option<Service>>
    where
        S: AsRef<str>,
    {
        query!("SELECT * FROM services WHERE id = $1", id.as_ref())
            .fetch_optional(pool)
            .await?
            .map(service_from_record!())
            .transpose()
    }

    /// Create a new service from a specification
    pub async fn create_from_spec(
        id: String,
        spec: ServiceSpec,
        default_domain: String,
        pool: &PgPool,
    ) -> sqlx::Result<Service> {
        let domain = match spec.web.enabled {
            true => Some(spec.web.domain.as_ref().unwrap_or(&default_domain)),
            false => None,
        };
        // TODO: pull from spec
        let path = "/";

        // TODO: properly handle error
        let serialized = serde_json::to_value(&spec).unwrap();

        let record = query!(
            "INSERT INTO services (id, spec, domain, path) VALUES ($1, $2, $3, $4) RETURNING *",
            id,
            serialized,
            domain,
            path,
        )
        .fetch_one(pool)
        .await?;

        Ok(Service {
            id: record.id,
            spec,
            domain: record.domain,
            path: record.path,
        })
    }

    /// Add a lease to the service
    pub async fn add_lease(
        &self,
        id: String,
        expiration: OffsetDateTime,
        pool: &PgPool,
    ) -> sqlx::Result<Lease> {
        query_as!(
            Lease,
            "INSERT INTO leases VALUES ($1, $2, $3) RETURNING *",
            self.id,
            id,
            expiration
        )
        .fetch_one(pool)
        .await
    }

    /// Set the container associated with the service
    pub async fn set_container(
        &self,
        id: String,
        image: String,
        pool: &PgPool,
    ) -> sqlx::Result<Container> {
        query_as!(
            Container,
            "INSERT INTO containers VALUES ($1, $2, $3) RETURNING service, id, image, status AS \"status: _\"",
            &self.id,
            id,
            image
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a service by its ID
    pub async fn delete<S>(id: S, pool: &PgPool) -> sqlx::Result<()>
    where
        S: AsRef<str>,
    {
        query!("DELETE FROM services WHERE id = $1", id.as_ref())
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Returns whether the service can be accessed from the internet
    pub fn is_publicly_accessible(&self) -> bool {
        self.domain.is_some()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "containers_status")]
pub enum Status {
    Configuring,
    Pulling,
    Creating,
    Starting,
    Healthy,
    Unhealty,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Container {
    pub service: String,
    pub id: String,
    pub image: String,
    pub status: Status,
}

impl Container {
    /// The the container associated with a service
    pub async fn for_service<S>(service: S, pool: &PgPool) -> sqlx::Result<Option<Container>>
    where
        S: AsRef<str>,
    {
        query_as!(
            Container,
            "SELECT service, id, image, status AS \"status: _\" FROM containers WHERE service = $1",
            service.as_ref()
        )
        .fetch_optional(pool)
        .await
    }

    /// Update the status of the container
    pub async fn update_status(&mut self, status: Status, pool: &PgPool) -> sqlx::Result<()> {
        query!(
            "UPDATE containers SET status = $1 WHERE id = $2 AND service = $3",
            status as _,
            &self.id,
            &self.service
        )
        .execute(pool)
        .await?;

        self.status = status;

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lease {
    pub service: String,
    pub id: String,
    #[serde(with = "time::serde::iso8601")]
    pub expiration: OffsetDateTime,
}

impl Lease {
    /// Get all the leases for a given service
    pub async fn for_service<S>(service: S, pool: &PgPool) -> sqlx::Result<Vec<Lease>>
    where
        S: AsRef<str>,
    {
        query_as!(
            Lease,
            "SELECT * FROM leases WHERE service = $1",
            service.as_ref()
        )
        .fetch_all(pool)
        .await
    }

    /// Get all the leases
    pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<Lease>> {
        query_as!(Lease, "SELECT * FROM leases")
            .fetch_all(pool)
            .await
    }

    /// Update a lease's expiration
    pub async fn update<S>(id: S, expiration: OffsetDateTime, pool: &PgPool) -> sqlx::Result<Lease>
    where
        S: AsRef<str>,
    {
        query_as!(
            Lease,
            "UPDATE leases SET expiration = $1 WHERE id = $2 RETURNING *",
            expiration,
            id.as_ref()
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a lease by its ID
    pub async fn delete<S>(id: S, pool: &PgPool) -> sqlx::Result<()>
    where
        S: AsRef<str>,
    {
        query!("DELETE FROM leases WHERE id = $1", id.as_ref())
            .execute(pool)
            .await?;

        Ok(())
    }
}
