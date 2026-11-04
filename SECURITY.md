# Security Policy

Drug-Ya handles HOSxP credentials and patient data. Security and privacy issues
are taken seriously.

## Reporting a vulnerability

Do not open a public issue for a security or privacy vulnerability. Use GitHub
private vulnerability reporting:

1. Open <https://github.com/suradet-ps/drug-ya/security/advisories/new>.
2. Describe the issue in the private advisory.
3. Do not include real patient data, credentials, or hospital secrets.

If the private advisory form is unavailable, contact the maintainer through the
GitHub profile linked from the repository.

Include:

- a description of the issue;
- the affected version or commit;
- steps to reproduce;
- the impact;
- any suggested fix.

Do not include real patient data, credentials, or hospital secrets in the
report. Use fictional data or redact the values.

## Scope

In scope:

- HOSxP write access or a bypass of the read-only guard
- credential leakage
- patient data leakage in logs, settings, errors, or exports
- authentication or session bypass
- Tauri capability escalation
- dependency vulnerabilities

Out of scope:

- issues in HOSxP itself
- issues in the hospital network
- physical access to an unlocked workstation
- social engineering

## Security rules

1. HOSxP is read-only. No write statement is allowed.
2. The database user must have `GRANT SELECT` only.
3. Every pooled connection must set a read-only session.
4. Every SQL statement must pass the read-only guard.
5. Every value must be bound. No SQL string concatenation.
6. No PHI or secrets in logs, errors, or settings exports.
7. Connection credentials are encrypted at rest with `encryptman`; the master
   key lives in the OS keychain.
8. The login session is memory-only.
9. No patient data is persisted locally. Acknowledgements and notes are
   session-only.
10. The UI never changes a HOSxP order.

## PHI handling

Patient names, HN, CID, addresses, phone numbers, visit data, drug lines, labs,
diagnoses, and allergies are PHI.

- Do not log them.
- Do not put them in the plain settings file.
- Do not include them in crash reports.
- Mask CID in search results.
- Show the full CID only on the selected patient card.
- Clear the review, acknowledgement, and note on logout or patient change.
- Follow hospital policy for printed copies and screenshots saved by the OS.

## Secret scanning

The repository uses several layers: GitHub secret scanning and push
protection, GitGuardian monitoring, an optional `ggshield` pre-commit hook, and
a `.gitguardian.yaml` file for local scans. See
[`docs/security-scanning.md`](docs/security-scanning.md) for setup and for the
false-positive process.

A secret that reaches the public repository must be rotated, not just deleted.
The value remains in the git history and may already have been copied.

## Dependency policy

- Prefer well-maintained crates.
- Run `cargo deny check all` before release.
- Do not add a dependency for a small function that can be written safely.
- Review licenses before adding a dependency.
