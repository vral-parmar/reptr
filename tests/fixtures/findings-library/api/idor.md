---
title: Insecure Direct Object Reference
severity: high
cwe: "CWE-639"
owasp: "A01:2021"
status: open
tags: [authorization, idor]
---

## Description

An API endpoint accepts an object identifier (e.g. `user_id`) but does not
verify that the authenticated caller is authorized to access that object.
A logged-in user can substitute another user's identifier and read or modify
their data.

## Proof of Concept

Replace with the specific endpoint and the request that succeeded against
an object the caller should not be able to reach.

## Impact

Disclosure or modification of data belonging to other users; privilege
escalation if the API exposes admin-tier resources via the same shape.

## Remediation

Enforce ownership at the controller boundary. Pull the authorization check
into shared middleware so new endpoints inherit it. Prefer opaque, unguessable
identifiers as defence in depth.

## References

- https://owasp.org/Top10/A01_2021-Broken_Access_Control/
- https://cwe.mitre.org/data/definitions/639.html
