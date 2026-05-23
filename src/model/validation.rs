use std::collections::HashMap;
use std::path::PathBuf;

use thiserror::Error;

use super::engagement::Engagement;
use super::finding::{Severity, Status};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("duplicate finding id `{id}` in {first} and {second}")]
    DuplicateId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
    #[error("finding `{path}` is missing a title")]
    MissingTitle { path: PathBuf },
    #[error("finding `{path}` has an invalid CVSS string `{value}`")]
    InvalidCvss { path: PathBuf, value: String },
    #[error("finding `{path}` has an invalid CVSS vector `{value}`: {reason}")]
    InvalidCvssVector {
        path: PathBuf,
        value: String,
        reason: String,
    },
    #[error(
        "finding `{path}` CVSS score `{stated}` does not match {computed:.1} computed from \
         vector `{vector}` — update the score or correct the vector"
    )]
    CvssScoreMismatch {
        path: PathBuf,
        stated: String,
        vector: String,
        computed: f64,
    },
    #[error("engagement slug is empty")]
    EmptySlug,
    #[error(
        "{count} open {severity} finding(s) exceed the allowed limit of {limit} \
         — resolve them or raise [severity_thresholds].{severity} in reptr.toml"
    )]
    ThresholdExceeded {
        severity: String,
        count: usize,
        limit: u32,
    },
}

/// Run the validation rules from §7 of the build plan:
/// - unique ids
/// - non-empty title
/// - cvss (if present) parses as a number between 0.0 and 10.0
/// - engagement slug is non-empty
pub fn validate_engagement(eng: &Engagement) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if eng.meta.slug.trim().is_empty() {
        errors.push(ValidationError::EmptySlug);
    }

    let mut seen: HashMap<&str, &PathBuf> = HashMap::new();
    for f in &eng.findings {
        if f.title.trim().is_empty() {
            errors.push(ValidationError::MissingTitle {
                path: f.source_path.clone(),
            });
        }
        if let Some(cvss) = &f.cvss {
            match cvss.parse::<f32>() {
                Ok(n) if (0.0..=10.0).contains(&n) => {}
                _ => errors.push(ValidationError::InvalidCvss {
                    path: f.source_path.clone(),
                    value: cvss.clone(),
                }),
            }
        }
        // Validate CVSS vector format and cross-check with stated score.
        if let Some(vector) = &f.cvss_vector {
            match vector.parse::<cvss::v3::Base>() {
                Ok(base) => {
                    // If a numeric score is also stated, it must agree with the
                    // value the vector computes to (within rounding to 1 d.p.).
                    if let Some(score_str) = &f.cvss {
                        if let Ok(stated) = score_str.parse::<f64>() {
                            let computed = base.score().value();
                            if (stated - computed).abs() > 0.05 {
                                errors.push(ValidationError::CvssScoreMismatch {
                                    path: f.source_path.clone(),
                                    stated: score_str.clone(),
                                    vector: vector.clone(),
                                    computed,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    errors.push(ValidationError::InvalidCvssVector {
                        path: f.source_path.clone(),
                        value: vector.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        if let Some(prev) = seen.insert(f.id.as_str(), &f.source_path) {
            errors.push(ValidationError::DuplicateId {
                id: f.id.clone(),
                first: prev.clone(),
                second: f.source_path.clone(),
            });
        }
    }

    // Severity threshold gating — counts only Status::Open findings.
    let t = &eng.severity_thresholds;
    let check_threshold = |limit: Option<u32>, sev: Severity, label: &str| {
        if let Some(limit) = limit {
            let count = eng
                .findings
                .iter()
                .filter(|f| f.severity == sev && f.status == Status::Open)
                .count();
            if count as u32 > limit {
                Some(ValidationError::ThresholdExceeded {
                    severity: label.to_string(),
                    count,
                    limit,
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    errors.extend(
        [
            check_threshold(t.critical, Severity::Critical, "critical"),
            check_threshold(t.high, Severity::High, "high"),
            check_threshold(t.medium, Severity::Medium, "medium"),
            check_threshold(t.low, Severity::Low, "low"),
        ]
        .into_iter()
        .flatten(),
    );

    errors
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{
        Client, Engagement, EngagementMeta, Finding, LibraryConfig, OutputConfig, Severity,
        SeverityThresholds, Status, TemplateConfig,
    };

    fn make_finding(id: &str, cvss: Option<&str>, cvss_vector: Option<&str>) -> Finding {
        Finding {
            id: id.to_string(),
            title: format!("Finding {id}"),
            severity: Severity::High,
            cvss: cvss.map(String::from),
            cvss_vector: cvss_vector.map(String::from),
            cwe: None,
            owasp: None,
            status: Status::Open,
            affected_assets: vec![],
            tags: vec![],
            body_markdown: String::new(),
            body_html: String::new(),
            source_path: PathBuf::from(format!("findings/{id}.md")),
            images: vec![],
        }
    }

    fn make_engagement(findings: Vec<Finding>) -> Engagement {
        Engagement {
            meta: EngagementMeta {
                name: "Test".to_string(),
                slug: "test-2026".to_string(),
                kind: String::new(),
                start_date: None,
                end_date: None,
                report_version: "1.0".to_string(),
            },
            client: Client::default(),
            findings,
            appendices: vec![],
            output: OutputConfig::default(),
            template: TemplateConfig::default(),
            severity_thresholds: SeverityThresholds::default(),
            library: LibraryConfig::default(),
        }
    }

    // Vector that computes to 9.8 (AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H)
    const VECTOR_9_8: &str = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H";
    // Vector that computes to 7.5 (C:H only, I:N/A:N)
    const VECTOR_7_5: &str = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N";

    #[test]
    fn valid_score_and_matching_vector_passes() {
        let eng = make_engagement(vec![make_finding("F-001", Some("9.8"), Some(VECTOR_9_8))]);
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn valid_score_no_vector_passes() {
        let eng = make_engagement(vec![make_finding("F-001", Some("7.5"), None)]);
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn no_cvss_fields_at_all_passes() {
        let eng = make_engagement(vec![make_finding("F-001", None, None)]);
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn valid_vector_no_score_passes() {
        // Score was auto-derived from vector at parse time; validation should pass.
        let eng = make_engagement(vec![make_finding("F-001", Some("9.8"), Some(VECTOR_9_8))]);
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn invalid_cvss_score_string_fails() {
        let eng = make_engagement(vec![make_finding("F-001", Some("not-a-number"), None)]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::InvalidCvss { .. }));
        assert!(errors[0].to_string().contains("not-a-number"));
    }

    #[test]
    fn cvss_score_out_of_range_fails() {
        let eng = make_engagement(vec![make_finding("F-001", Some("12.0"), None)]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::InvalidCvss { .. }));
    }

    #[test]
    fn cvss_score_negative_fails() {
        let eng = make_engagement(vec![make_finding("F-001", Some("-1.0"), None)]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::InvalidCvss { .. }));
    }

    #[test]
    fn invalid_cvss_vector_format_fails() {
        let eng = make_engagement(vec![make_finding(
            "F-001",
            None,
            Some("CVSS:3.1/NOT_A_VECTOR"),
        )]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InvalidCvssVector { .. }
        ));
        let msg = errors[0].to_string();
        assert!(msg.contains("CVSS:3.1/NOT_A_VECTOR"));
    }

    #[test]
    fn completely_malformed_vector_fails() {
        let eng = make_engagement(vec![make_finding("F-001", None, Some("not-a-cvss-vector"))]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InvalidCvssVector { .. }
        ));
    }

    #[test]
    fn cvss_score_mismatch_with_vector_fails() {
        // Score says 5.0 but the vector computes 9.8 — should be caught.
        let eng = make_engagement(vec![make_finding("F-001", Some("5.0"), Some(VECTOR_9_8))]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::CvssScoreMismatch { .. }
        ));
        let msg = errors[0].to_string();
        assert!(msg.contains("5.0"));
        assert!(msg.contains("9.8"));
    }

    #[test]
    fn cvss_score_mismatch_error_names_the_vector() {
        let eng = make_engagement(vec![make_finding("F-001", Some("3.0"), Some(VECTOR_7_5))]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        let msg = errors[0].to_string();
        assert!(
            msg.contains(VECTOR_7_5),
            "error should quote the vector. got: {msg}"
        );
    }

    #[test]
    fn score_within_rounding_tolerance_passes() {
        // The vector computes 9.8; stating 9.8 exactly should pass.
        let eng = make_engagement(vec![make_finding("F-001", Some("9.8"), Some(VECTOR_9_8))]);
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn invalid_vector_error_message_includes_path() {
        let mut f = make_finding("F-001", None, Some("CVSS:3.1/BAD"));
        f.source_path = PathBuf::from("findings/001-sqli.md");
        let eng = make_engagement(vec![f]);
        let errors = validate_engagement(&eng);
        assert!(!errors.is_empty());
        let msg = errors[0].to_string();
        assert!(
            msg.contains("001-sqli.md"),
            "error should name the file. got: {msg}"
        );
    }

    #[test]
    fn mismatch_error_message_includes_path_and_computed_value() {
        let mut f = make_finding("F-001", Some("5.0"), Some(VECTOR_9_8));
        f.source_path = PathBuf::from("findings/001-sqli.md");
        let eng = make_engagement(vec![f]);
        let errors = validate_engagement(&eng);
        let msg = errors[0].to_string();
        assert!(msg.contains("001-sqli.md"));
        assert!(msg.contains("5.0"));
        assert!(msg.contains("9.8"));
    }

    #[test]
    fn multiple_findings_each_invalid_vector_reported() {
        let eng = make_engagement(vec![
            make_finding("F-001", None, Some("CVSS:3.1/BAD_ONE")),
            make_finding("F-002", None, Some("CVSS:3.1/BAD_TWO")),
        ]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 2, "expected one error per invalid vector");
    }

    #[test]
    fn valid_finding_among_invalid_does_not_suppress_errors() {
        let eng = make_engagement(vec![
            make_finding("F-001", Some("9.8"), Some(VECTOR_9_8)), // valid
            make_finding("F-002", None, Some("CVSS:3.1/BAD")),    // invalid
        ]);
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InvalidCvssVector { .. }
        ));
    }

    // --- severity threshold tests -----------------------------------------

    fn make_finding_with_severity_status(id: &str, severity: Severity, status: Status) -> Finding {
        Finding {
            id: id.to_string(),
            title: format!("Finding {id}"),
            severity,
            cvss: None,
            cvss_vector: None,
            cwe: None,
            owasp: None,
            status,
            affected_assets: vec![],
            tags: vec![],
            body_markdown: String::new(),
            body_html: String::new(),
            source_path: PathBuf::from(format!("findings/{id}.md")),
            images: vec![],
        }
    }

    fn with_thresholds(mut eng: Engagement, t: SeverityThresholds) -> Engagement {
        eng.severity_thresholds = t;
        eng
    }

    #[test]
    fn no_thresholds_set_always_passes() {
        let eng = make_engagement(vec![
            make_finding_with_severity_status("F-001", Severity::Critical, Status::Open),
            make_finding_with_severity_status("F-002", Severity::High, Status::Open),
        ]);
        // Default thresholds = all None → no gate
        assert!(validate_engagement(&eng).is_empty());
    }

    #[test]
    fn threshold_zero_fails_when_any_open_of_that_severity() {
        let eng = with_thresholds(
            make_engagement(vec![make_finding_with_severity_status(
                "F-001",
                Severity::Critical,
                Status::Open,
            )]),
            SeverityThresholds {
                critical: Some(0),
                ..Default::default()
            },
        );
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::ThresholdExceeded { .. }
        ));
    }

    #[test]
    fn threshold_passes_when_count_within_limit() {
        let eng = with_thresholds(
            make_engagement(vec![
                make_finding_with_severity_status("F-001", Severity::High, Status::Open),
                make_finding_with_severity_status("F-002", Severity::High, Status::Open),
            ]),
            SeverityThresholds {
                high: Some(2),
                ..Default::default()
            },
        );
        assert!(
            validate_engagement(&eng).is_empty(),
            "2 open highs with limit 2 should pass"
        );
    }

    #[test]
    fn threshold_fails_when_count_exceeds_limit() {
        let eng = with_thresholds(
            make_engagement(vec![
                make_finding_with_severity_status("F-001", Severity::High, Status::Open),
                make_finding_with_severity_status("F-002", Severity::High, Status::Open),
                make_finding_with_severity_status("F-003", Severity::High, Status::Open),
            ]),
            SeverityThresholds {
                high: Some(2),
                ..Default::default()
            },
        );
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::ThresholdExceeded { .. }
        ));
    }

    #[test]
    fn resolved_findings_do_not_count_against_threshold() {
        let eng = with_thresholds(
            make_engagement(vec![
                make_finding_with_severity_status("F-001", Severity::Critical, Status::Resolved),
                make_finding_with_severity_status("F-002", Severity::Critical, Status::Resolved),
            ]),
            SeverityThresholds {
                critical: Some(0),
                ..Default::default()
            },
        );
        assert!(
            validate_engagement(&eng).is_empty(),
            "resolved findings should not count against threshold"
        );
    }

    #[test]
    fn accepted_findings_do_not_count_against_threshold() {
        let eng = with_thresholds(
            make_engagement(vec![make_finding_with_severity_status(
                "F-001",
                Severity::Critical,
                Status::Accepted,
            )]),
            SeverityThresholds {
                critical: Some(0),
                ..Default::default()
            },
        );
        assert!(
            validate_engagement(&eng).is_empty(),
            "accepted findings should not count against threshold"
        );
    }

    #[test]
    fn false_positive_findings_do_not_count_against_threshold() {
        let eng = with_thresholds(
            make_engagement(vec![make_finding_with_severity_status(
                "F-001",
                Severity::Critical,
                Status::FalsePositive,
            )]),
            SeverityThresholds {
                critical: Some(0),
                ..Default::default()
            },
        );
        assert!(
            validate_engagement(&eng).is_empty(),
            "false_positive findings should not count against threshold"
        );
    }

    #[test]
    fn threshold_error_message_includes_count_and_limit() {
        let eng = with_thresholds(
            make_engagement(vec![
                make_finding_with_severity_status("F-001", Severity::Critical, Status::Open),
                make_finding_with_severity_status("F-002", Severity::Critical, Status::Open),
            ]),
            SeverityThresholds {
                critical: Some(1),
                ..Default::default()
            },
        );
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 1);
        let msg = errors[0].to_string();
        assert!(
            msg.contains('2'),
            "error should mention the count (2). got: {msg}"
        );
        assert!(
            msg.contains('1'),
            "error should mention the limit (1). got: {msg}"
        );
        assert!(
            msg.contains("critical"),
            "error should name the severity. got: {msg}"
        );
    }

    #[test]
    fn multiple_severity_thresholds_exceeded_all_reported() {
        let eng = with_thresholds(
            make_engagement(vec![
                make_finding_with_severity_status("F-001", Severity::Critical, Status::Open),
                make_finding_with_severity_status("F-002", Severity::High, Status::Open),
            ]),
            SeverityThresholds {
                critical: Some(0),
                high: Some(0),
                ..Default::default()
            },
        );
        let errors = validate_engagement(&eng);
        assert_eq!(errors.len(), 2, "both thresholds should be reported");
        assert!(errors
            .iter()
            .all(|e| matches!(e, ValidationError::ThresholdExceeded { .. })));
    }

    #[test]
    fn threshold_only_applies_to_matching_severity() {
        // Critical threshold = 0 but the open finding is High — should pass.
        let eng = with_thresholds(
            make_engagement(vec![make_finding_with_severity_status(
                "F-001",
                Severity::High,
                Status::Open,
            )]),
            SeverityThresholds {
                critical: Some(0),
                ..Default::default()
            },
        );
        assert!(
            validate_engagement(&eng).is_empty(),
            "critical threshold should not affect high findings"
        );
    }

    #[test]
    fn threshold_none_means_unlimited() {
        // 10 open criticals with None threshold → no error.
        let findings: Vec<Finding> = (1..=10)
            .map(|i| {
                make_finding_with_severity_status(
                    &format!("F-{i:03}"),
                    Severity::Critical,
                    Status::Open,
                )
            })
            .collect();
        let eng = make_engagement(findings);
        // threshold.critical is None by default
        assert!(
            validate_engagement(&eng).is_empty(),
            "None threshold should never fire regardless of count"
        );
    }
}
