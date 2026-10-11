use super::*;

macro_rules! check_ids {
    ($($kind:ident),+ $(,)?) => {
        $(
            let raw = uuid::Uuid::new_v4();
            let id = $kind::from(raw);
            assert_eq!(id.as_uuid(), &raw);
            assert_eq!(uuid::Uuid::from(id), raw);
            assert_eq!(id.to_string(), raw.to_string());
            assert_eq!(raw.to_string().parse::<$kind>().unwrap(), id);
            assert_eq!($kind::try_from(raw.to_string()).unwrap(), id);
            assert_eq!($kind::try_from(raw.to_string().as_str()).unwrap(), id);
            assert!("invalid-id".parse::<$kind>().is_err());
            assert_eq!($kind::default(), $kind::empty());
            assert!($kind::empty().as_uuid().is_nil());
            let generated = $kind::new();
            assert_eq!(generated.as_uuid().get_version_num(), 4);
            assert_ne!(generated, $kind::new());

            #[cfg(feature = "serde")]
            {
                let json = serde_json::to_value(id).unwrap();
                assert_eq!(json, serde_json::to_value(raw).unwrap());
                assert_eq!(serde_json::from_value::<$kind>(json).unwrap(), id);
                assert!(serde_json::from_str::<$kind>("\"invalid-id\"").is_err());
            }

            #[cfg(feature = "utoipa")]
            {
                let schema = serde_json::to_value(<$kind as utoipa::PartialSchema>::schema()).unwrap();
                assert_eq!(schema, serde_json::json!({"type": "string", "format": "uuid"}));
            }

            #[cfg(feature = "postgres")]
            {
                use sqlx::{Postgres, Type};
                assert_eq!(<$kind as Type<Postgres>>::type_info(), <uuid::Uuid as Type<Postgres>>::type_info());
                assert!(<$kind as Type<Postgres>>::compatible(&<uuid::Uuid as Type<Postgres>>::type_info()));
            }
        )+
    };
}

#[test]
fn ids_preserve_uuid_boundaries() {
    check_ids!(
        UserId,
        SessionId,
        OidcIdentityId,
        OidcFlowId,
        MethodProofId,
        RecoveryTokenId,
        WorkspaceId,
        MembershipId,
        TeamId,
        WorkflowStatusId,
        InvitationId,
        IssueId,
        IssueActivityId,
    );
}
