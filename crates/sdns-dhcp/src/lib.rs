pub mod fingerprint;
pub mod lease;
pub mod template;

pub use fingerprint::{FingerprintClassification, FingerprintInput, FingerprintRule, match_rule};
pub use lease::{
    AuthorizeQuarantineRequest, DhcpLease, LeaseDecision, QuarantineEntry, ReplacementOutcome,
    detect_replacement,
};
pub use template::{RoleAssignment, RoleMatch, RoleTemplate, choose_assignment};
