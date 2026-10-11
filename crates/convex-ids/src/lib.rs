//! Shared, distinct UUID types for Convex entities.
//!
//! IDs retain the UUID representation in JSON and PostgreSQL, but cannot be
//! substituted for IDs of another entity in Rust.
//!
//! ```compile_fail
//! use convex_ids::{UserId, WorkspaceId};
//! fn workspace_name(id: WorkspaceId) {}
//! workspace_name(UserId::new());
//! ```

macro_rules! impl_typed_uuid_kinds {
    ($ident:ident) => {
        #[doc = concat!("A typed UUID for `", stringify!($ident), "`.")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        #[cfg_attr(feature = "postgres", derive(sqlx::Type))]
        #[cfg_attr(feature = "postgres", sqlx(transparent))]
        #[repr(transparent)]
        pub struct $ident(uuid::Uuid);

        impl $ident {
            /// Creates a random UUID, preserving Convex's UUIDv4 generation.
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }

            /// Creates a nil UUID.
            pub const fn empty() -> Self {
                Self(uuid::Uuid::nil())
            }

            /// Wraps an existing UUID, including in constant fixtures.
            pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the underlying UUID at an untyped boundary.
            pub const fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }
        }

        impl From<uuid::Uuid> for $ident {
            fn from(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$ident> for uuid::Uuid {
            fn from(id: $ident) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $ident {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.parse().map(Self)
            }
        }

        impl TryFrom<&str> for $ident {
            type Error = uuid::Error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl TryFrom<String> for $ident {
            type Error = uuid::Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        #[cfg(feature = "utoipa")]
        impl utoipa::PartialSchema for $ident {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::String)
                    .format(Some(utoipa::openapi::schema::SchemaFormat::KnownFormat(
                        utoipa::openapi::schema::KnownFormat::Uuid,
                    )))
                    .build()
                    .into()
            }
        }

        #[cfg(feature = "utoipa")]
        impl utoipa::ToSchema for $ident {}
    };
}

mod ids;
pub use ids::*;

#[cfg(test)]
mod tests;
