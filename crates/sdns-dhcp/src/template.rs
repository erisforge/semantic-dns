use sdns_common::TemplateId;
use sdns_core::Isa95WorkCenterKind;

use crate::fingerprint::FingerprintClassification;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleAssignment {
    pub role: String,
    pub address: String,
    pub class_match: Option<String>,
    pub vendor_match: Option<String>,
    pub function_match: Option<String>,
    pub work_unit_id: Option<String>,
    pub process_area: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleTemplate {
    pub id: TemplateId,
    pub name: String,
    pub site_id: Option<String>,
    pub area_id: Option<String>,
    pub work_center_id: Option<String>,
    pub work_center_kind: Option<Isa95WorkCenterKind>,
    pub cell_id: String,
    pub zone_suffix: String,
    pub assignments: Vec<RoleAssignment>,
    pub unassigned_range: Vec<String>,
    pub quarantine_range: Vec<String>,
}

impl RoleAssignment {
    pub fn effective_work_unit_id(&self) -> &str {
        self.work_unit_id
            .as_deref()
            .unwrap_or(self.process_area.as_str())
    }
}

impl RoleTemplate {
    pub fn effective_work_center_id(&self) -> &str {
        self.work_center_id
            .as_deref()
            .unwrap_or(self.cell_id.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleMatch {
    pub assignment: RoleAssignment,
    pub score: f32,
}

pub fn choose_assignment(
    classification: &FingerprintClassification,
    function_hint: Option<&str>,
    template: &RoleTemplate,
    reserved_addresses: &[String],
) -> Option<RoleMatch> {
    template
        .assignments
        .iter()
        .filter(|assignment| !reserved_addresses.contains(&assignment.address))
        .map(|assignment| {
            let mut score = 0.0_f32;
            if assignment
                .class_match
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&classification.class))
                .unwrap_or(false)
            {
                score += 0.6;
            }
            if assignment
                .vendor_match
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&classification.vendor))
                .unwrap_or(false)
            {
                score += 0.3;
            }
            if assignment
                .function_match
                .as_deref()
                .zip(function_hint)
                .map(|(pattern, function)| {
                    function
                        .to_ascii_lowercase()
                        .contains(&pattern.to_ascii_lowercase().replace('*', ""))
                })
                .unwrap_or(false)
            {
                score += 0.1;
            }
            RoleMatch {
                assignment: assignment.clone(),
                score,
            }
        })
        .max_by(|left, right| {
            left.score
                .partial_cmp(&right.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use sdns_common::TemplateId;
    use sdns_core::Isa95WorkCenterKind;

    use crate::fingerprint::FingerprintClassification;

    use super::{RoleAssignment, RoleTemplate, choose_assignment};

    #[test]
    fn picks_best_available_matching_role() {
        let template = RoleTemplate {
            id: TemplateId::new(),
            name: "cell".to_string(),
            site_id: Some("Milwaukee".to_string()),
            area_id: Some("Zone3".to_string()),
            work_center_id: Some("Cell5".to_string()),
            work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
            cell_id: "Cell5".to_string(),
            zone_suffix: "local".to_string(),
            assignments: vec![RoleAssignment {
                role: "conveyor-vfd".to_string(),
                address: "192.168.1.47".to_string(),
                class_match: Some("vfd".to_string()),
                vendor_match: Some("rockwell".to_string()),
                function_match: Some("conveyor".to_string()),
                work_unit_id: Some("Conveyor".to_string()),
                process_area: "Conveyor".to_string(),
            }],
            unassigned_range: vec![],
            quarantine_range: vec![],
        };

        let classification = FingerprintClassification {
            vendor: "rockwell".to_string(),
            class: "vfd".to_string(),
            model_family: "PowerFlex500".to_string(),
            confidence: 0.8,
            protocols: vec!["ethernet-ip".to_string()],
        };

        let result = choose_assignment(&classification, Some("conveyor-main"), &template, &[])
            .expect("assignment");
        assert_eq!(result.assignment.address, "192.168.1.47");
        assert_eq!(template.effective_work_center_id(), "Cell5");
        assert_eq!(result.assignment.effective_work_unit_id(), "Conveyor");
    }
}
