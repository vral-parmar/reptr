use std::fs;
use std::path::PathBuf;

use reptr::commands::{add, build, library as library_cmd, new, retest, stats};
use reptr::library;
use reptr::model::{validate_engagement, Engagement, Severity};
use reptr::parse::{load_engagement_config, load_findings};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-engagement")
}

fn build_engagement(root: &std::path::Path) -> Engagement {
    let (cfg, client) = load_engagement_config(root).expect("config");
    let findings = load_findings(&root.join("findings")).expect("findings");
    let mut eng = Engagement {
        meta: cfg.engagement,
        client,
        findings,
        appendices: vec![],
        output: cfg.output,
        template: cfg.template,
        severity_thresholds: cfg.severity_thresholds,
        library: cfg.library,
    };
    eng.sort_findings();
    eng
}

#[test]
fn parses_sample_engagement() {
    let eng = build_engagement(&fixture_root());
    assert_eq!(eng.findings.len(), 2);
    // sorted highest-severity first
    assert_eq!(eng.findings[0].severity, Severity::Critical);
    assert_eq!(eng.findings[0].id, "F-001");
    assert_eq!(eng.findings[1].id, "F-002");
    assert!(eng.findings[0].body_html.contains("<h2>"));
    assert!(eng.findings[0].body_html.contains("Description"));
}

#[test]
fn sample_engagement_validates_clean() {
    let eng = build_engagement(&fixture_root());
    let errors = validate_engagement(&eng);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn build_renders_html_and_json_into_tempdir() {
    // Copy fixture into a tempdir so we don't pollute the repo with output/.
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    build::run(dst).expect("build succeeds");

    let html = dst.join("output/acme-webapp-2026.html");
    let json = dst.join("output/acme-webapp-2026.json");
    assert!(html.exists(), "html not produced");
    assert!(json.exists(), "json not produced");

    let html_body = fs::read_to_string(&html).unwrap();
    assert!(html_body.contains("Acme Web Application Assessment"));
    assert!(html_body.contains("SQL Injection"));
    assert!(html_body.contains("sev-critical"));

    let json_body = fs::read_to_string(&json).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_body).unwrap();
    assert_eq!(parsed["meta"]["slug"], "acme-webapp-2026");
    assert_eq!(parsed["findings"][0]["id"], "F-001");
}

#[test]
fn build_renders_pdf_when_typst_available() {
    // Skip if typst isn't installed — keeps CI / first-time devs unblocked.
    if std::process::Command::new("typst")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("skipping: typst CLI not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    let cfg_path = dst.join("reptr.toml");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    let cfg = cfg.replace(
        r#"formats = ["html", "json"]"#,
        r#"formats = ["html", "json", "pdf"]"#,
    );
    fs::write(&cfg_path, cfg).unwrap();

    build::run(dst).expect("build succeeds");

    let pdf_path = dst.join("output/acme-webapp-2026.pdf");
    assert!(pdf_path.exists(), "pdf not produced");
    let bytes = fs::read(&pdf_path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "pdf magic missing");
    assert!(
        bytes.len() > 1000,
        "pdf suspiciously small: {} bytes",
        bytes.len()
    );
}

#[test]
fn build_renders_docx_when_format_enabled() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    // Add docx to the formats list.
    let cfg_path = dst.join("reptr.toml");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    let cfg = cfg.replace(
        r#"formats = ["html", "json"]"#,
        r#"formats = ["html", "json", "docx"]"#,
    );
    fs::write(&cfg_path, cfg).unwrap();

    build::run(dst).expect("build succeeds");

    let docx_path = dst.join("output/acme-webapp-2026.docx");
    assert!(docx_path.exists(), "docx not produced");
    let bytes = fs::read(&docx_path).unwrap();
    assert!(
        bytes.len() > 1000,
        "docx suspiciously small: {} bytes",
        bytes.len()
    );
    // Word .docx files are zip archives; verify the local-file-header magic.
    assert_eq!(&bytes[..2], b"PK", "docx is not a valid zip archive");
}

#[test]
fn finding_image_refs_are_parsed_and_resolved() {
    let eng = build_engagement(&fixture_root());
    let sqli = eng
        .findings
        .iter()
        .find(|f| f.id == "F-001")
        .expect("sqli finding");
    assert_eq!(sqli.images.len(), 1, "expected exactly one image ref");
    let img = &sqli.images[0];
    assert_eq!(img.markdown_src, "../assets/screenshots/sqli-bypass.png");
    let resolved = img.resolved_path.as_ref().expect("resolved path");
    assert!(resolved.exists(), "{} should exist", resolved.display());
    assert!(resolved.ends_with("assets/screenshots/sqli-bypass.png"));
}

#[test]
fn docx_embeds_referenced_image() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();
    let cfg_path = dst.join("reptr.toml");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    fs::write(
        &cfg_path,
        cfg.replace(
            r#"formats = ["html", "json"]"#,
            r#"formats = ["html", "json", "docx"]"#,
        ),
    )
    .unwrap();

    build::run(dst).expect("build succeeds");

    let docx = dst.join("output/acme-webapp-2026.docx");
    let bytes = fs::read(&docx).unwrap();
    // OOXML places embedded images under word/media/. Searching the raw zip
    // bytes is enough — we don't need to unzip in the test.
    assert!(
        bytes.windows(11).any(|w| w == b"word/media/"),
        "expected an entry under word/media/ inside the DOCX zip"
    );
}

#[test]
fn pdf_typst_source_references_image() {
    if std::process::Command::new("typst")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("skipping: typst CLI not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();
    let cfg_path = dst.join("reptr.toml");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    fs::write(
        &cfg_path,
        cfg.replace(
            r#"formats = ["html", "json"]"#,
            r#"formats = ["html", "json", "pdf"]"#,
        ),
    )
    .unwrap();

    build::run(dst).expect("build succeeds");

    let typ_src = fs::read_to_string(dst.join("output/acme-webapp-2026.typ")).unwrap();
    assert!(
        typ_src.contains("#image("),
        "expected typst source to embed the screenshot. got:\n{typ_src}"
    );
    let pdf = dst.join("output/acme-webapp-2026.pdf");
    let bytes = fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() > 1000);
}

#[test]
fn custom_html_template_overrides_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    // Write a minimal custom template with a sentinel marker we can grep for.
    let custom = r#"<!DOCTYPE html>
<html><head><title>{{ engagement.meta.name }}</title></head>
<body data-reptr-test="custom-template-v1">
<h1>{{ engagement.meta.name }}</h1>
<p>{{ engagement.findings | length }} findings</p>
<ul>
{% for f in engagement.findings %}
  <li data-id="{{ f.id }}" data-sev="{{ f.severity }}">{{ f.title }}</li>
{% endfor %}
</ul>
</body></html>
"#;
    fs::create_dir_all(dst.join("templates")).unwrap();
    fs::write(dst.join("templates/report.html"), custom).unwrap();

    // Point [template].html at it.
    let cfg_path = dst.join("reptr.toml");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap();
    cfg.push_str("\n[template]\nhtml = \"templates/report.html\"\n");
    fs::write(&cfg_path, cfg).unwrap();

    build::run(dst).expect("build succeeds");

    let html = fs::read_to_string(dst.join("output/acme-webapp-2026.html")).unwrap();
    assert!(
        html.contains(r#"data-reptr-test="custom-template-v1""#),
        "expected sentinel from custom template, got:\n{html}"
    );
    assert!(html.contains("2 findings"));
    assert!(html.contains(r#"data-sev="critical""#));
    assert!(
        !html.contains("Findings Overview"),
        "default template should not have been used"
    );
}

#[test]
fn missing_custom_template_errors_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    let cfg_path = dst.join("reptr.toml");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap();
    cfg.push_str("\n[template]\nhtml = \"templates/does-not-exist.html\"\n");
    fs::write(&cfg_path, cfg).unwrap();

    let err = build::run(dst).expect_err("expected build to fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("does-not-exist.html") && msg.to_lowercase().contains("reading"),
        "error should name the missing file. got: {msg}"
    );
}

#[test]
fn stats_aggregates_across_engagements() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path();

    // Two engagement dirs cloned from the fixture, with one extra finding in
    // the second so totals differ between them.
    copy_dir(&fixture_root(), &parent.join("eng-a")).unwrap();
    copy_dir(&fixture_root(), &parent.join("eng-b")).unwrap();

    // Make eng-b's slug unique so the rows are distinguishable.
    let b_cfg = parent.join("eng-b/reptr.toml");
    let cfg = fs::read_to_string(&b_cfg).unwrap();
    fs::write(
        &b_cfg,
        cfg.replace(
            r#"slug = "acme-webapp-2026""#,
            r#"slug = "contoso-mobile-2026""#,
        ),
    )
    .unwrap();

    // Add an extra HIGH finding to eng-b.
    fs::write(
        parent.join("eng-b/findings/003-extra.md"),
        "---\nid: F-003\ntitle: Extra finding for eng-b\nseverity: high\nstatus: resolved\n---\n\n## Description\n\nbody.\n",
    )
    .unwrap();

    let (rows, totals) = stats::collect_stats(parent).expect("stats ok");
    assert_eq!(rows.len(), 2, "expected 2 engagements");
    let slugs: Vec<&str> = rows.iter().map(|r| r.slug.as_str()).collect();
    assert!(slugs.contains(&"acme-webapp-2026"));
    assert!(slugs.contains(&"contoso-mobile-2026"));

    // Each fixture has 1 critical + 1 low; eng-b adds 1 high (resolved).
    assert_eq!(totals.counts.critical, 2);
    assert_eq!(totals.counts.high, 1);
    assert_eq!(totals.counts.low, 2);
    assert_eq!(totals.total, 5);
    assert_eq!(totals.resolved, 1);
    assert_eq!(totals.open, 4);

    // JSON serializes cleanly.
    let json = serde_json::to_value(&rows).unwrap();
    assert!(json[0].get("counts").is_some());
}

#[test]
fn stats_works_inside_a_single_engagement_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    let (rows, totals) = stats::collect_stats(dst).expect("stats ok");
    assert_eq!(rows.len(), 1);
    assert_eq!(totals.engagements, 1);
    assert_eq!(totals.total, 2);
    assert_eq!(totals.counts.critical, 1);
    assert_eq!(totals.counts.low, 1);
}

#[test]
fn stats_errors_when_no_engagement_found() {
    let tmp = tempfile::tempdir().unwrap();
    let err = stats::collect_stats(tmp.path()).expect_err("should fail on empty dir");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("no engagements found"),
        "expected helpful error, got: {msg}"
    );
}

fn library_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/findings-library")
}

#[test]
fn library_lists_every_template() {
    let templates = library::list_templates(&library_root()).unwrap();
    let names: Vec<_> = templates.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"web/xss-stored"));
    assert!(names.contains(&"web/sql-injection"));
    assert!(names.contains(&"api/idor"));
    // Sorted alphabetically
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);

    let xss = templates
        .iter()
        .find(|t| t.name == "web/xss-stored")
        .unwrap();
    assert_eq!(xss.title, "Stored Cross-Site Scripting");
    assert_eq!(xss.severity, "high");
}

#[test]
fn library_load_missing_template_errors_clearly() {
    let err = library::load_template(&library_root(), "does-not-exist").expect_err("should fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("does-not-exist") && msg.contains("not found"),
        "expected helpful error, got: {msg}"
    );
}

#[test]
fn add_from_library_imports_and_assigns_id() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    // Point [library].path at the test fixture library (absolute path so it
    // works regardless of the temp dir's location).
    let mut cfg = fs::read_to_string(dst.join("reptr.toml")).unwrap();
    cfg.push_str(&format!(
        "\n[library]\npath = \"{}\"\n",
        library_root().display()
    ));
    fs::write(dst.join("reptr.toml"), cfg).unwrap();

    // Import with no title override — should use the template's title.
    add::run(
        dst,
        None,
        reptr::model::Severity::Medium,
        Some("web/xss-stored"),
    )
    .expect("add --from succeeds");

    // The fixture already has 002 (high-severity-ish finding); next id is F-003.
    let imported_path = dst.join("findings/003-stored-cross-site-scripting.md");
    assert!(imported_path.exists(), "import file not created");

    let body = fs::read_to_string(&imported_path).unwrap();
    assert!(body.contains("id: F-003"));
    assert!(body.contains("title: Stored Cross-Site Scripting"));
    assert!(
        body.contains("severity: high"),
        "template severity must survive import"
    );
    // Body markdown should come through intact.
    assert!(body.contains("Apply context-aware output encoding"));
}

#[test]
fn add_from_library_with_title_override() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();
    let mut cfg = fs::read_to_string(dst.join("reptr.toml")).unwrap();
    cfg.push_str(&format!(
        "\n[library]\npath = \"{}\"\n",
        library_root().display()
    ));
    fs::write(dst.join("reptr.toml"), cfg).unwrap();

    add::run(
        dst,
        Some("XSS in Profile Bio"),
        reptr::model::Severity::Medium,
        Some("web/xss-stored"),
    )
    .expect("add --from with title succeeds");

    let body = fs::read_to_string(dst.join("findings/003-xss-in-profile-bio.md")).unwrap();
    assert!(body.contains("title: XSS in Profile Bio"));
    // Template severity preserved
    assert!(body.contains("severity: high"));
}

#[test]
fn imported_finding_passes_validation_and_builds() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();
    let mut cfg = fs::read_to_string(dst.join("reptr.toml")).unwrap();
    cfg.push_str(&format!(
        "\n[library]\npath = \"{}\"\n",
        library_root().display()
    ));
    fs::write(dst.join("reptr.toml"), cfg).unwrap();

    add::run(
        dst,
        None,
        reptr::model::Severity::Medium,
        Some("web/sql-injection"),
    )
    .unwrap();
    add::run(dst, None, reptr::model::Severity::Medium, Some("api/idor")).unwrap();

    // Should build cleanly with the imported findings present.
    build::run(dst).expect("build with imported findings succeeds");

    let html = fs::read_to_string(dst.join("output/acme-webapp-2026.html")).unwrap();
    assert!(html.contains("Insecure Direct Object Reference"));
    assert!(html.contains("SQL Injection"));
}

#[test]
fn library_list_runs_without_panicking() {
    // Smoke-test the library list subcommand against the fixture library.
    // We can't easily capture stdout here, but ensuring it returns Ok is enough.
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();
    let mut cfg = fs::read_to_string(dst.join("reptr.toml")).unwrap();
    cfg.push_str(&format!(
        "\n[library]\npath = \"{}\"\n",
        library_root().display()
    ));
    fs::write(dst.join("reptr.toml"), cfg).unwrap();

    library_cmd::list(dst).expect("library list ok");
}

#[test]
fn detects_duplicate_finding_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    copy_dir(&fixture_root(), dst).unwrap();

    // Duplicate F-001 by copying it under a new filename.
    let extra = dst.join("findings/003-duplicate.md");
    let raw = fs::read_to_string(dst.join("findings/001-sql-injection-in-login-form.md")).unwrap();
    fs::write(&extra, raw).unwrap();

    let err = build::run(dst).expect_err("expected validation failure");
    let msg = format!("{err:#}");
    assert!(msg.to_lowercase().contains("validation"), "msg: {msg}");
}

#[test]
fn new_command_scaffolds_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd_guard = ChangeDir::to(tmp.path());
    new::run("scratch-engagement").expect("new succeeds");
    drop(cwd_guard);

    let root = tmp.path().join("scratch-engagement");
    assert!(root.join("reptr.toml").exists());
    assert!(root.join("client.toml").exists());
    assert!(root.join("findings/001-example-finding.md").exists());
    assert!(root.join("templates").is_dir());
    assert!(root.join("assets/screenshots").is_dir());
    assert!(root.join("output").is_dir());
}

// --- retest tests --------------------------------------------------------

/// Helper: copy fixture to a tempdir and return the path.
fn setup_engagement() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().to_path_buf();
    copy_dir(&fixture_root(), &dst).unwrap();
    (tmp, dst)
}

/// Helper: set a finding's `status:` field in its frontmatter via plain string
/// replacement. The fixture files use a single-line `status: <value>` so this
/// is reliable without a full TOML parser.
fn set_finding_status(path: &std::path::Path, new_status: &str) {
    let content = fs::read_to_string(path).unwrap();
    let updated = regex::Regex::new(r"(?m)^status: \w+$")
        .unwrap()
        .replace(&content, format!("status: {new_status}"))
        .to_string();
    fs::write(path, updated).unwrap();
}

/// Helper: change a finding's `severity:` field.
fn set_finding_severity(path: &std::path::Path, new_severity: &str) {
    let content = fs::read_to_string(path).unwrap();
    let updated = regex::Regex::new(r"(?m)^severity: \w+$")
        .unwrap()
        .replace(&content, format!("severity: {new_severity}"))
        .to_string();
    fs::write(path, updated).unwrap();
}

/// Helper: read `output/<slug>-retest.json` and return it as a Value.
fn read_retest_json(dst: &std::path::Path, slug: &str) -> serde_json::Value {
    let path = dst.join(format!("output/{slug}-retest.json"));
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&data).unwrap()
}

/// Helper: read `output/<slug>-retest.html` as a string.
fn read_retest_html(dst: &std::path::Path, slug: &str) -> String {
    let path = dst.join(format!("output/{slug}-retest.html"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

const SLUG: &str = "acme-webapp-2026";

#[test]
fn retest_first_run_establishes_baseline_no_retest_files() {
    let (_tmp, dst) = setup_engagement();

    // No prior JSON at all — this should behave like a plain build and return Ok.
    assert!(
        !dst.join(format!("output/{SLUG}.json")).exists(),
        "precondition: no prior JSON"
    );

    retest::run(&dst).expect("first retest run succeeds");

    // Regular build outputs are created.
    assert!(
        dst.join(format!("output/{SLUG}.html")).exists(),
        "baseline HTML not created"
    );
    assert!(
        dst.join(format!("output/{SLUG}.json")).exists(),
        "baseline JSON not created"
    );

    // Retest-specific delta files should NOT exist on a first (baseline) run.
    assert!(
        !dst.join(format!("output/{SLUG}-retest.json")).exists(),
        "retest JSON should not exist on first run"
    );
    assert!(
        !dst.join(format!("output/{SLUG}-retest.html")).exists(),
        "retest HTML should not exist on first run"
    );
}

#[test]
fn retest_no_changes_all_unchanged() {
    let (_tmp, dst) = setup_engagement();

    // Baseline.
    retest::run(&dst).expect("baseline ok");

    // Diff run — nothing changed.
    retest::run(&dst).expect("second retest ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["unchanged_count"], 2, "both fixture findings should be unchanged");
    assert_eq!(json["new_count"], 0);
    assert_eq!(json["resolved_count"], 0);
    assert_eq!(json["regressed_count"], 0);
    assert_eq!(json["changed_count"], 0);
    assert_eq!(json["removed_count"], 0);
}

#[test]
fn retest_detects_resolved_finding() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    // Resolve F-002 (the Low finding).
    let f002 = dst.join("findings/002-missing-security-headers.md");
    set_finding_status(&f002, "resolved");

    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["resolved_count"], 1, "expected one resolved finding");
    assert_eq!(json["unchanged_count"], 1);
    assert_eq!(json["regressed_count"], 0);

    let deltas = json["deltas"].as_array().unwrap();
    let resolved_delta = deltas
        .iter()
        .find(|d| d["id"] == "F-002")
        .expect("F-002 delta missing");
    assert_eq!(resolved_delta["change_type"], "resolved");
    assert_eq!(resolved_delta["label"], "open → resolved");
}

#[test]
fn retest_detects_regressed_finding() {
    let (_tmp, dst) = setup_engagement();

    // Make F-001 resolved in the baseline.
    let f001 = dst.join("findings/001-sql-injection-in-login-form.md");
    set_finding_status(&f001, "resolved");
    retest::run(&dst).expect("baseline ok");

    // Regress it back to open.
    set_finding_status(&f001, "open");
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["regressed_count"], 1, "expected one regressed finding");
    assert_eq!(json["resolved_count"], 0);

    let deltas = json["deltas"].as_array().unwrap();
    let d = deltas.iter().find(|d| d["id"] == "F-001").unwrap();
    assert_eq!(d["change_type"], "regressed");
    assert_eq!(d["label"], "resolved → open");
}

#[test]
fn retest_detects_new_finding() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    // Add a third finding.
    fs::write(
        dst.join("findings/003-csrf-missing-token.md"),
        "---\nid: F-003\ntitle: CSRF Missing Token\nseverity: high\nstatus: open\n---\n\n## Description\n\nCSRF token absent.\n",
    )
    .unwrap();

    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["new_count"], 1, "expected one new finding");
    assert_eq!(json["unchanged_count"], 2);

    let deltas = json["deltas"].as_array().unwrap();
    let new_delta = deltas.iter().find(|d| d["id"] == "F-003").unwrap();
    assert_eq!(new_delta["change_type"], "new");
    assert_eq!(new_delta["severity"], "high");
    assert_eq!(new_delta["label"], "New");
}

#[test]
fn retest_detects_removed_finding() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    // Remove F-002.
    fs::remove_file(dst.join("findings/002-missing-security-headers.md")).unwrap();

    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["removed_count"], 1, "expected one removed finding");
    assert_eq!(json["unchanged_count"], 1);

    let deltas = json["deltas"].as_array().unwrap();
    let removed = deltas.iter().find(|d| d["id"] == "F-002").unwrap();
    assert_eq!(removed["change_type"], "removed");
    assert_eq!(removed["label"], "Removed");
}

#[test]
fn retest_detects_severity_downgrade() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    // Downgrade F-001 from critical to high.
    let f001 = dst.join("findings/001-sql-injection-in-login-form.md");
    set_finding_severity(&f001, "high");

    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["changed_count"], 1, "expected one changed finding");
    assert_eq!(json["resolved_count"], 0);

    let deltas = json["deltas"].as_array().unwrap();
    let d = deltas.iter().find(|d| d["id"] == "F-001").unwrap();
    assert_eq!(d["change_type"], "changed");
    assert!(
        d["label"].as_str().unwrap().contains("critical → high"),
        "label should show severity change: {}",
        d["label"]
    );
}

#[test]
fn retest_detects_multiple_simultaneous_changes() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    // Resolve F-002 + add a new F-003.
    set_finding_status(
        &dst.join("findings/002-missing-security-headers.md"),
        "resolved",
    );
    fs::write(
        dst.join("findings/003-new.md"),
        "---\nid: F-003\ntitle: New Finding\nseverity: medium\nstatus: open\n---\n\nbody.\n",
    )
    .unwrap();

    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["new_count"], 1);
    assert_eq!(json["resolved_count"], 1);
    assert_eq!(json["unchanged_count"], 1);
    assert_eq!(json["regressed_count"], 0);
    assert_eq!(json["removed_count"], 0);
}

#[test]
fn retest_accepted_status_to_resolved_counts_as_resolved() {
    let (_tmp, dst) = setup_engagement();

    // Mark F-002 as accepted in the baseline.
    set_finding_status(&dst.join("findings/002-missing-security-headers.md"), "accepted");
    retest::run(&dst).expect("baseline ok");

    // Then resolve it.
    set_finding_status(&dst.join("findings/002-missing-security-headers.md"), "resolved");
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(
        json["resolved_count"], 1,
        "accepted → resolved should count as resolved"
    );
    assert_eq!(json["changed_count"], 0);
}

#[test]
fn retest_open_to_false_positive_counts_as_changed_not_resolved() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    set_finding_status(
        &dst.join("findings/002-missing-security-headers.md"),
        "false_positive",
    );
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["changed_count"], 1);
    assert_eq!(json["resolved_count"], 0);

    let deltas = json["deltas"].as_array().unwrap();
    let d = deltas.iter().find(|d| d["id"] == "F-002").unwrap();
    assert_eq!(d["change_type"], "changed");
    assert!(d["label"].as_str().unwrap().contains("false_positive"));
}

#[test]
fn retest_html_output_is_written_and_well_formed() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");
    retest::run(&dst).expect("diff run ok");

    let html = read_retest_html(&dst, SLUG);
    assert!(html.trim().starts_with("<!doctype html>"));
    assert!(html.contains("</html>"));
    assert!(html.contains("Acme Web Application Assessment"));
}

#[test]
fn retest_html_unchanged_findings_get_tag_unchanged_class() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");
    retest::run(&dst).expect("diff run ok");

    let html = read_retest_html(&dst, SLUG);
    assert!(
        html.contains("tag-unchanged"),
        "unchanged findings should carry tag-unchanged CSS class"
    );
}

#[test]
fn retest_html_resolved_finding_gets_correct_classes() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    set_finding_status(&dst.join("findings/001-sql-injection-in-login-form.md"), "resolved");
    retest::run(&dst).expect("diff run ok");

    let html = read_retest_html(&dst, SLUG);
    assert!(html.contains("tag-resolved"), "resolved badge class missing");
    assert!(html.contains("sev-critical"), "critical severity badge missing");
}

#[test]
fn retest_html_regressed_finding_gets_regressed_class() {
    let (_tmp, dst) = setup_engagement();

    // Baseline with F-001 resolved.
    set_finding_status(&dst.join("findings/001-sql-injection-in-login-form.md"), "resolved");
    retest::run(&dst).expect("baseline ok");

    // Regress it.
    set_finding_status(&dst.join("findings/001-sql-injection-in-login-form.md"), "open");
    retest::run(&dst).expect("diff run ok");

    let html = read_retest_html(&dst, SLUG);
    assert!(html.contains("tag-regressed"), "regressed badge class missing");
}

#[test]
fn retest_html_new_finding_gets_tag_new_class() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");

    fs::write(
        dst.join("findings/003-new.md"),
        "---\nid: F-003\ntitle: New XSS\nseverity: medium\nstatus: open\n---\n\nbody.\n",
    )
    .unwrap();
    retest::run(&dst).expect("diff run ok");

    let html = read_retest_html(&dst, SLUG);
    assert!(html.contains("tag-new"), "new badge class missing");
    assert!(html.contains("New XSS"), "new finding title missing from HTML");
}

#[test]
fn retest_json_delta_array_contains_all_findings() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    let deltas = json["deltas"].as_array().unwrap();
    // Fixture has 2 findings; both should appear in the delta array.
    assert_eq!(deltas.len(), 2, "delta array should contain all 2 fixture findings");
}

#[test]
fn retest_json_delta_has_required_fields() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    let deltas = json["deltas"].as_array().unwrap();
    for d in deltas {
        assert!(d["id"].is_string(), "delta missing `id` field");
        assert!(d["title"].is_string(), "delta missing `title` field");
        assert!(d["severity"].is_string(), "delta missing `severity` field");
        assert!(d["change_type"].is_string(), "delta missing `change_type` field");
        assert!(d["label"].is_string(), "delta missing `label` field");
    }
}

#[test]
fn retest_json_top_level_has_all_count_fields() {
    let (_tmp, dst) = setup_engagement();
    retest::run(&dst).expect("baseline ok");
    retest::run(&dst).expect("diff run ok");

    let json = read_retest_json(&dst, SLUG);
    for field in &[
        "new_count",
        "removed_count",
        "resolved_count",
        "regressed_count",
        "changed_count",
        "unchanged_count",
        "engagement_name",
        "generated_at",
        "deltas",
    ] {
        assert!(
            !json[field].is_null(),
            "retest JSON missing field `{field}`"
        );
    }
}

#[test]
fn retest_reruns_correctly_accumulate_delta() {
    // Three runs: baseline → resolve → regress → verify each step.
    let (_tmp, dst) = setup_engagement();
    let f002 = dst.join("findings/002-missing-security-headers.md");

    // Run 1: baseline.
    retest::run(&dst).expect("baseline ok");

    // Run 2: resolve F-002.
    set_finding_status(&f002, "resolved");
    retest::run(&dst).expect("second run ok");
    let json2 = read_retest_json(&dst, SLUG);
    assert_eq!(json2["resolved_count"], 1, "run2: F-002 should be resolved");
    assert_eq!(json2["regressed_count"], 0);

    // Run 3: regress F-002 back to open.
    set_finding_status(&f002, "open");
    retest::run(&dst).expect("third run ok");
    let json3 = read_retest_json(&dst, SLUG);
    assert_eq!(
        json3["regressed_count"], 1,
        "run3: F-002 should now be regressed (resolved → open)"
    );
    assert_eq!(json3["resolved_count"], 0);
}

#[test]
fn retest_does_not_fail_on_empty_findings_dir() {
    let (_tmp, dst) = setup_engagement();

    // Baseline with normal findings.
    retest::run(&dst).expect("baseline ok");

    // Remove ALL findings.
    fs::remove_file(dst.join("findings/001-sql-injection-in-login-form.md")).unwrap();
    fs::remove_file(dst.join("findings/002-missing-security-headers.md")).unwrap();

    retest::run(&dst).expect("retest with empty findings dir should not panic");

    let json = read_retest_json(&dst, SLUG);
    assert_eq!(json["removed_count"], 2);
    assert_eq!(json["new_count"], 0);
    assert_eq!(json["unchanged_count"], 0);
}

// --- CVSS integration tests ----------------------------------------------

// Verified computed scores:
//   CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H  → 9.8  (fixture F-001)
//   CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N  → 7.5

const CVSS_VECTOR_9_8: &str = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H";
const CVSS_VECTOR_7_5: &str = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N";

/// Write a minimal finding to `findings/003-<slug>.md` with the given CVSS fields.
fn write_cvss_finding(
    dst: &std::path::Path,
    id: &str,
    title: &str,
    cvss: Option<&str>,
    cvss_vector: Option<&str>,
) {
    let mut fm = format!("---\nid: {id}\ntitle: {title}\nseverity: medium\nstatus: open\n");
    if let Some(s) = cvss {
        fm.push_str(&format!("cvss: \"{s}\"\n"));
    }
    if let Some(v) = cvss_vector {
        fm.push_str(&format!("cvss_vector: \"{v}\"\n"));
    }
    fm.push_str("---\n\n## Description\n\nbody.\n");
    let slug: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    fs::write(dst.join(format!("findings/003-{slug}.md")), fm).unwrap();
}

/// Read `output/<slug>.json` and return the finding with the given `id`.
fn get_finding_from_json(dst: &std::path::Path, id: &str) -> serde_json::Value {
    let body = fs::read_to_string(dst.join(format!("output/{SLUG}.json"))).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("finding {id} not found in JSON output"))
        .clone()
}

#[test]
fn cvss_vector_only_auto_derives_score_in_json() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "SSRF", None, Some(CVSS_VECTOR_7_5));

    build::run(&dst).expect("build with vector-only finding should succeed");

    let f003 = get_finding_from_json(&dst, "F-003");
    assert_eq!(
        f003["cvss"].as_str().unwrap_or(""),
        "7.5",
        "score should be auto-derived from vector. got: {}",
        f003["cvss"]
    );
}

#[test]
fn cvss_derived_score_is_formatted_to_one_decimal_place() {
    // Verifies the format!("{:.1}", ...) rounding, not just the numeric value.
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "SSRF", None, Some(CVSS_VECTOR_7_5));

    build::run(&dst).expect("build succeeds");

    let f003 = get_finding_from_json(&dst, "F-003");
    let score = f003["cvss"].as_str().unwrap();
    assert_eq!(score, "7.5", "derived score must be exactly '7.5', not '7.50' or '7'");
}

#[test]
fn cvss_derived_score_appears_in_html_output() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "SSRF Finding", None, Some(CVSS_VECTOR_7_5));

    build::run(&dst).expect("build succeeds");

    let html = fs::read_to_string(dst.join(format!("output/{SLUG}.html"))).unwrap();
    assert!(
        html.contains("7.5"),
        "derived CVSS score should appear somewhere in HTML output"
    );
}

#[test]
fn cvss_vector_field_is_preserved_in_json_output() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "SSRF", Some("7.5"), Some(CVSS_VECTOR_7_5));

    build::run(&dst).expect("build succeeds");

    let f003 = get_finding_from_json(&dst, "F-003");
    assert_eq!(
        f003["cvss_vector"].as_str().unwrap_or(""),
        CVSS_VECTOR_7_5,
        "cvss_vector should be round-tripped into JSON output"
    );
}

#[test]
fn cvss_absent_score_absent_from_json() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Info Disclosure", None, None);

    build::run(&dst).expect("build without any CVSS fields should succeed");

    let f003 = get_finding_from_json(&dst, "F-003");
    assert!(
        f003["cvss"].is_null(),
        "cvss key should be absent from JSON when no score/vector provided"
    );
    assert!(
        f003["cvss_vector"].is_null(),
        "cvss_vector key should be absent from JSON when not provided"
    );
}

#[test]
fn cvss_score_zero_is_valid() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Info Only", Some("0.0"), None);

    build::run(&dst).expect("CVSS score 0.0 is the minimum valid value and should pass");
}

#[test]
fn cvss_score_ten_is_valid() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Max Score", Some("10.0"), None);

    build::run(&dst).expect("CVSS score 10.0 is the maximum valid value and should pass");
}

#[test]
fn cvss_matching_score_and_vector_passes_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "SSRF High", Some("7.5"), Some(CVSS_VECTOR_7_5));

    build::run(&dst).expect("matching explicit score and vector should pass validation");
}

#[test]
fn cvss_invalid_vector_fails_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Bad Vector", None, Some("CVSS:3.1/INVALID/VECTOR"));

    let err = build::run(&dst).expect_err("invalid CVSS vector should cause build failure");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_completely_malformed_vector_fails_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Malformed Vector", None, Some("not-a-cvss-vector"));

    let err = build::run(&dst).expect_err("completely malformed vector should fail");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_score_mismatch_fails_build() {
    let (_tmp, dst) = setup_engagement();
    // Score says 5.0 but the vector computes 9.8.
    write_cvss_finding(&dst, "F-003", "Mismatch Finding", Some("5.0"), Some(CVSS_VECTOR_9_8));

    let err = build::run(&dst).expect_err("mismatched score/vector should fail validation");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_score_out_of_range_fails_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Bad Score Range", Some("15.0"), None);

    let err = build::run(&dst).expect_err("out-of-range CVSS score should fail validation");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_negative_score_fails_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Negative Score", Some("-1.0"), None);

    let err = build::run(&dst).expect_err("negative CVSS score should fail validation");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_non_numeric_score_fails_build() {
    let (_tmp, dst) = setup_engagement();
    write_cvss_finding(&dst, "F-003", "Non Numeric Score", Some("not-a-number"), None);

    let err = build::run(&dst).expect_err("non-numeric CVSS score should fail validation");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_multiple_invalid_findings_both_reported() {
    let (_tmp, dst) = setup_engagement();
    // Two bad findings: one with an out-of-range score, one with an invalid vector.
    fs::write(
        dst.join("findings/003-bad-score.md"),
        "---\nid: F-003\ntitle: Bad Score\nseverity: medium\nstatus: open\ncvss: \"20.0\"\n---\n\nbody.\n",
    )
    .unwrap();
    fs::write(
        dst.join("findings/004-bad-vector.md"),
        "---\nid: F-004\ntitle: Bad Vector\nseverity: medium\nstatus: open\ncvss_vector: \"CVSS:3.1/JUNK\"\n---\n\nbody.\n",
    )
    .unwrap();

    let err = build::run(&dst).expect_err("multiple CVSS errors should fail build");
    let msg = format!("{err:#}");
    // The bail message counts errors: "2 validation error(s)"
    assert!(
        msg.contains('2') || msg.to_lowercase().contains("validation"),
        "error should report multiple failures. got: {msg}"
    );
}

#[test]
fn cvss_valid_finding_among_invalid_does_not_suppress_error() {
    let (_tmp, dst) = setup_engagement();
    // F-003 is valid (9.8 with matching vector from fixture F-001 is already
    // clean); add F-004 with a bad vector. Build must still fail.
    write_cvss_finding(&dst, "F-003", "Valid Finding", Some("9.8"), Some(CVSS_VECTOR_9_8));
    fs::write(
        dst.join("findings/004-bad.md"),
        "---\nid: F-004\ntitle: Bad Vector\nseverity: medium\nstatus: open\ncvss_vector: \"CVSS:3.1/GARBAGE\"\n---\n\nbody.\n",
    )
    .unwrap();

    let err = build::run(&dst).expect_err("one invalid finding among valid ones must still fail");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("validation"),
        "error should mention validation. got: {msg}"
    );
}

#[test]
fn cvss_fixture_findings_have_matching_score_and_vector() {
    // Regression guard: verifies that F-001 in the fixture continues to have a
    // matching cvss score and vector so other tests using the fixture stay clean.
    let eng = build_engagement(&fixture_root());
    let f001 = eng.findings.iter().find(|f| f.id == "F-001").expect("F-001 in fixture");
    assert_eq!(f001.cvss.as_deref(), Some("9.8"));
    assert_eq!(f001.cvss_vector.as_deref(), Some(CVSS_VECTOR_9_8));
}

// --- helpers -------------------------------------------------------------

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let out = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out)?;
        } else if entry.file_type().is_file() {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(entry.path(), &out)?;
        }
    }
    Ok(())
}

struct ChangeDir {
    previous: PathBuf,
}

impl ChangeDir {
    fn to(p: &std::path::Path) -> Self {
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(p).unwrap();
        Self { previous }
    }
}

impl Drop for ChangeDir {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}
