---
id: F-002
title: Missing Security Headers
severity: low
cvss: "3.1"
cwe: "CWE-693"
owasp: "A05:2021"
status: open
affected_assets:
  - https://app.example.com/
tags: [hardening]
---

## Description

The application is missing `Content-Security-Policy`, `Strict-Transport-Security`, and `X-Content-Type-Options` response headers.

## Impact

Defense-in-depth gaps; harder to mitigate other classes of bugs.

## Remediation

Configure the reverse proxy to add these headers on every response.
