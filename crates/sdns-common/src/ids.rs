use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn parse_str(value: &str) -> Result<Self, uuid::Error> {
                Uuid::parse_str(value).map(Self)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

define_id!(DeviceId, "Typed identifier for a semantic device.");
define_id!(ObservationId, "Typed identifier for an observation.");
define_id!(LeaseId, "Typed identifier for a DHCP lease.");
define_id!(TemplateId, "Typed identifier for a DHCP role template.");
define_id!(FingerprintId, "Typed identifier for a fingerprint rule.");
define_id!(PrincipalId, "Typed identifier for an API principal.");
define_id!(
    QuarantineEntryId,
    "Typed identifier for a quarantine entry."
);
