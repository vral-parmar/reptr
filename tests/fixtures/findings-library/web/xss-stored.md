---
title: Stored Cross-Site Scripting
severity: high
cwe: "CWE-79"
owasp: "A03:2021"
status: open
affected_assets: []
tags: [xss, injection]
---

## Description

The application stores attacker-controlled input and reflects it without
output encoding when other users view the affected page. An attacker can
inject JavaScript that runs in victims' browsers in the application's origin.

## Proof of Concept

1. Authenticate as a low-privilege user.
2. Submit `<script>alert(document.domain)</script>` into the affected field.
3. Have a higher-privilege user view the affected page — the script executes.

## Impact

Session hijacking, account takeover, phishing in trusted UI, defacement.

## Remediation

Apply context-aware output encoding (HTML escape for tags, attribute encoding
for `href=`/`src=`, JSON-stringify for inline `<script>` data). Add a strict
`Content-Security-Policy` header as defence in depth.

## References

- https://owasp.org/Top10/A03_2021-Injection/
- https://cwe.mitre.org/data/definitions/79.html
