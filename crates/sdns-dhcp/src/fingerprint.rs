use sdns_common::FingerprintId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FingerprintInput {
    pub mac: String,
    pub hostname: Option<String>,
    pub option_60: Option<String>,
    pub option_55_order: Vec<u16>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FingerprintClassification {
    pub vendor: String,
    pub class: String,
    pub model_family: String,
    pub confidence: f32,
    pub protocols: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FingerprintRule {
    pub id: FingerprintId,
    pub name: String,
    pub mac_oui: Option<String>,
    pub option_60_glob: Option<String>,
    pub option_55_order: Option<Vec<u16>>,
    pub classification: FingerprintClassification,
}

pub fn match_rule(
    input: &FingerprintInput,
    rule: &FingerprintRule,
) -> Option<FingerprintClassification> {
    let mut score = 0.0_f32;

    if let Some(oui) = &rule.mac_oui {
        if input
            .mac
            .to_ascii_lowercase()
            .starts_with(&oui.to_ascii_lowercase())
        {
            score += 0.3;
        }
    }

    if let Some(pattern) = &rule.option_60_glob {
        if input
            .option_60
            .as_deref()
            .map(|value| glob_match(pattern, value))
            .unwrap_or(false)
        {
            score += 0.5;
        }
    }

    if let Some(order) = &rule.option_55_order {
        if &input.option_55_order == order {
            score += 0.2;
        }
    }

    if score >= 0.4 {
        let mut classification = rule.classification.clone();
        classification.confidence = score.min(1.0);
        Some(classification)
    } else {
        None
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let normalized_pattern = pattern.to_ascii_lowercase().replace('*', "");
    value
        .to_ascii_lowercase()
        .contains(normalized_pattern.trim())
}

#[cfg(test)]
mod tests {
    use sdns_common::FingerprintId;

    use super::{FingerprintClassification, FingerprintInput, FingerprintRule, match_rule};

    #[test]
    fn scores_on_vendor_class_and_parameter_order() {
        let input = FingerprintInput {
            mac: "00:00:BC:3A:47:12".to_string(),
            hostname: Some("PowerFlex525".to_string()),
            option_60: Some("Rockwell/Allen-Bradley".to_string()),
            option_55_order: vec![1, 3, 6, 15, 28, 42],
        };
        let rule = FingerprintRule {
            id: FingerprintId::new(),
            name: "rockwell-powerflex".to_string(),
            mac_oui: Some("00:00:BC".to_string()),
            option_60_glob: Some("Rockwell*".to_string()),
            option_55_order: Some(vec![1, 3, 6, 15, 28, 42]),
            classification: FingerprintClassification {
                vendor: "rockwell".to_string(),
                class: "vfd".to_string(),
                model_family: "PowerFlex500".to_string(),
                confidence: 0.0,
                protocols: vec!["ethernet-ip".to_string()],
            },
        };

        let matched = match_rule(&input, &rule).expect("should match");
        assert!(matched.confidence >= 0.9);
    }
}
