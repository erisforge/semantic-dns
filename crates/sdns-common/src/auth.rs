use crate::{PrincipalId, error::AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Reader,
    Operator,
    Admin,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    DnsAdmin,
    DhcpWrite,
    SystemIngest,
}

impl Role {
    pub fn has_permission(self, permission: Permission) -> bool {
        match self {
            Role::Reader => matches!(permission, Permission::Read),
            Role::Operator => matches!(
                permission,
                Permission::Read | Permission::Write | Permission::DhcpWrite
            ),
            Role::Admin => true,
            Role::System => true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Principal {
    pub id: PrincipalId,
    pub name: String,
    pub role: Role,
}

impl Principal {
    pub fn require(&self, permission: Permission) -> Result<(), AppError> {
        if self.role.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "{} lacks {:?}",
                self.name, permission
            )))
        }
    }
}
