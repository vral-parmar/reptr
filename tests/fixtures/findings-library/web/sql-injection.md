---
title: SQL Injection
severity: critical
cvss: "9.8"
cwe: "CWE-89"
owasp: "A03:2021"
status: open
tags: [injection, database]
---

## Description

User input is concatenated into a SQL statement without parameterization,
letting an attacker break out of the query context and execute arbitrary
SQL against the database.

## Proof of Concept

Replace with the specific endpoint, parameter, and exploit payload observed.

## Impact

Authentication bypass, data exfiltration of every row the database role can
read, possible code execution on the DB host depending on engine and privileges.

## Remediation

Use parameterized queries (prepared statements) everywhere user input
reaches the database. Treat ORMs' raw-string escape hatches as untrusted.

## References

- https://owasp.org/Top10/A03_2021-Injection/
- https://cwe.mitre.org/data/definitions/89.html
