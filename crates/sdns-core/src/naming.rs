pub fn build_semantic_name(
    name_parts: &[&str],
    sibling_names: &[String],
    zone_suffix: &str,
) -> String {
    let labels = name_parts
        .iter()
        .map(|value| normalize_label(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let base = if labels.is_empty() {
        "Device".to_string()
    } else {
        labels.join(".")
    };

    if !sibling_names
        .iter()
        .any(|name| name == &format!("{base}.{zone_suffix}"))
    {
        return format!("{base}.{zone_suffix}");
    }

    for idx in 2..=999 {
        let candidate = format!("{base}-{idx:02}.{zone_suffix}");
        if !sibling_names.iter().any(|name| name == &candidate) {
            return candidate;
        }
    }

    format!("{base}-overflow.{zone_suffix}")
}

fn normalize_label(input: &str) -> String {
    let trimmed = input.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if matches!(ch, '-' | '_' | ' ') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::build_semantic_name;

    #[test]
    fn appends_collision_suffix() {
        let siblings = vec!["DriveVFD.Conveyor.Cell5.local".to_string()];
        let name = build_semantic_name(&["DriveVFD", "Conveyor", "Cell5"], &siblings, "local");
        assert_eq!(name, "DriveVFD.Conveyor.Cell5-02.local");
    }

    #[test]
    fn retains_isa95_path_segments() {
        let name = build_semantic_name(
            &["DriveVFD", "Conveyor", "Cell5", "Zone3", "Milwaukee"],
            &[],
            "local",
        );
        assert_eq!(name, "DriveVFD.Conveyor.Cell5.Zone3.Milwaukee.local");
    }
}
