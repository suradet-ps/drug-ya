# Drug-Ya

[![CI](https://github.com/suradet-ps/drug-ya/actions/workflows/ci.yml/badge.svg)](https://github.com/suradet-ps/drug-ya/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: stable](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-24c8db.svg?logo=tauri&logoColor=white)](https://tauri.app/)
[![Leptos v0.8](https://img.shields.io/badge/Leptos-v0.8-blue.svg)](https://leptos.dev)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/suradet-ps/drug-ya/issues)

---

## ◆ PULSE

One patient, one day, one review screen. Drug-Ya reads today's OPD
orders for a patient from HOSxP, adds the context a pharmacist needs
(allergies, recent labs, renal function, diagnoses, pregnancy status),
and runs a site-configurable, explainable rule pack over the order set.
It never writes to HOSxP, never changes an order, and never pretends an
unknown drug or a missing lab was checked.

| Orders ▣ | Allergies ▣ | Renal / labs ▣ | Findings ▣ | Coverage ▣ | Sign-off ▣ |
|---|---|---|---|---|---|

*v0.1.0 - the first complete review loop: officer sign-in, today's orders,
findings, coverage, and pharmacist confirmation.*

> Built with Tauri 2 + Leptos 0.8, judged by `drug-ya-core`, read from
> HOSxP by `drug-ya-hosxp` - never a write, never a plaintext secret.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One toolchain, a few commands.

```
⟫ rustup target add wasm32-unknown-unknown   # via rust-toolchain.toml
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked
⟫ cargo test                                 # DB-free workspace tests
⟫ cd apps/drug-ya-app
⟫ cargo tauri dev
```

The release artifact, from `apps/drug-ya-app`:

```
⟫ cargo tauri build
```

<details>
<summary>Prerequisites</summary>

- Rust **stable** (edition 2024) with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) - installed above
- Tauri 2 system dependencies for your platform
- A HOSxP MySQL/MariaDB database, accessed with a `SELECT`-only account

</details>

On first launch the login gate asks for the HOSxP **`officer`** account.
Connection setup lives in the settings dialog; credentials are encrypted
before they touch disk, and the session never leaves the app process. A
failed login is throttled, and every patient command requires a live
session.

> Before the first real use, verify the site assumptions in
> [`docs/database.md`](docs/database.md) and review the rule pack in
> [`docs/CLINICAL-RULES.md`](docs/CLINICAL-RULES.md).

<details>
<summary>Verified on Sra Bos (2026-11-04)</summary>

- 15 core tables present.
- OPD queue is `ovst.oqueue` (int); there is no `ovst.queue`.
- `officer_authentication_type_id` is NULL on all 104 accounts; the query
  coalesces NULL to local DB, so login is not blocked.
- Labs use `SGOT`, not an AST-named item; the classifier avoids a raw
  `%ast%` substring.
- `opd_allergy` has `icode`, `seriousness_id`, `no_alert`, and
  `force_no_order`; the engine honors the structured columns.
- `drugusage` and `sp_use` sig names are joined, not collapsed to one field.
- `opitemrece` lacks a composite `(hn, vstdate)` index; ask the DBA to add it.

See [`docs/database.md`](docs/database.md) for the full verification.

</details>

---

## ◆ ANATOMY

Four crates and one app, around a boundary that never bends: HOSxP is
read, never written.

- **Judges** - `drug-ya-core` is the pure domain: dictionary resolution,
  dose and days-supply parsing, CKD-EPI 2021 and Cockcroft-Gault, Thai/BE
  formatting, the rule engine, coverage, and sig composition. It has no
  I/O and is testable without a database.
- **Reads** - `drug-ya-hosxp` is the only crate allowed to talk to MySQL.
  Every statement is a compile-time constant checked by a comment-aware,
  semicolon-aware guard that accepts only a single `SELECT` or `WITH`.
  Read-only is enforced in code, at the session, and at the DB grant.
- **Seals** - `drug-ya-config` stores connection credentials encrypted
  with `encryptman` (AES-256-GCM), the master key in the OS keychain.
  Review settings stay in a plain, non-secret JSON file. No patient data
  is persisted.
- **Speaks** - `drug-ya-bridge` carries typed commands to the Leptos UI,
  which is Thai-first by design.
- **Shows** - `apps/drug-ya-app` is the Tauri 2 shell and the review
  console: login gate, search rail, patient workspace, findings, and the
  settings and sign-off modal.

Dependency direction:

```text
frontend -> drug-ya-core
drug-ya-hosxp -> drug-ya-core
drug-ya-app -> core + hosxp + config
```

Every SQL constant has a guard test. The default test suite never needs a
HOSxP instance.

---

## ◆ RITUALS

**The core ceremony** - one patient, today's orders:

1. Open Drug-Ya, connect to HOSxP, and sign in with an officer account.
   The session is memory-only.
2. Search by HN, CID, name, or today's queue. Auto-detection is explicit
   and the mode can be overridden.
3. Read the patient: one clinical summary (eGFR/CrCl, creatinine,
   potassium, INR, weight/BMI), one context bar (allergies, diagnoses,
   pregnancy, PMH), and today's visit chips.
4. Review the orders. Directions for use are the complete HOSxP text,
   joined from every `drugusage` and `sp_use` name field. A generated
   shorthand never replaces the prescriber's wording.
5. Read the findings, then the coverage report. Missing labs, diagnoses,
   weight, sig, or drug mappings are shown, not hidden.
6. Acknowledge required findings, add a session-only note, and confirm the
   review. The prescriber remains the decision-maker.

**The ceremony of uncertainty** - an empty findings list is not proof that
everything was checked. Coverage is part of the result, and every skipped
or partial category says why.

**The ceremony of read-only** - five layers: a `GRANT SELECT` database
account, `SET SESSION TRANSACTION READ ONLY` on every pooled connection,
the SQL guard, bound parameters only, and Tauri `core:default` only. No
`INSERT`, `UPDATE`, `DELETE`, DDL, or session write is ever sent to HOSxP.

**The ceremony of privacy** - HN and CID are masked where they do not
need to be shown; names, orders, labs, and review notes are never logged.
No patient data is written to disk. Acknowledgements and notes are gone
when the session ends.

---

## ◆ ECHOES

**Where this artifact is heading**

```
D0-D2  ▸ sibling study, visual system, schema package ─────────────── ▸ sealed
M0     ▸ Tauri 2 + Leptos scaffold, officer login, encrypted config ─ ▸ sealed
M1     ▸ search by HN/CID/name/queue, patient context ─────────────── ▸ sealed
M2     ▸ today's orders + labs + dose and renal derivation ────────── ▸ sealed
M3     ▸ configurable rules, findings, coverage ───────────────────── ▸ sealed
M4     ▸ settings, thresholds, pharmacist sign-off, confirmation ──── ▸ sealed
M5     ▸ site schema verification, rule review, pilot ─────────────── ▸ in progress
```

The schema verification against Sra Bos is complete. The remaining M5
work is the pharmacist rule review and sign-off, pilot feedback, and the
optional `opitemrece(hn, vstdate)` index.

**Raising the artifact** - read `AGENTS.md` before touching a query or a
rule. It holds the confirmed HOSxP schema notes and the hard rules:
read-only without exceptions, no plaintext credentials, no patient data
on disk, and a sign-off before clinical use. The design language lives in
[`docs/DESIGN.md`](docs/DESIGN.md).

**Status** - CI gates every change: `cargo fmt --check`, native and wasm32
`clippy -- -D warnings`, DB-free host tests, `cargo deny`, and a release
WASM build of the Leptos frontend. A separate test-build job compiles the
Tauri app, and tagged releases are published for Windows, Linux, and macOS.
[Watch the gates](.github/workflows).

---

## ◆ LIMITS

- Reviews **today's OPD orders only**. It is not a full medication
  reconciliation and does not include prior visits or outside hospitals.
- The starter rule pack is incomplete. A site pharmacist must review the
  evidence, thresholds, and local practice, then sign off the pack before
  clinical use.
- Drug mapping is by local `drugitems.name`, not a national product code.
- Allergy records are free text. Cross-reactivity still needs pharmacist
  judgment.
- Lab items are matched by name; site naming varies.
- Dose estimates depend on a parseable sig and strength. When they cannot
  be parsed, the app says so instead of guessing.
- No pharmacogenomics, full pediatric weight-based dosing, IV
  compatibility, TPN, or chemotherapy protocols.
- No patient data is persisted. Acknowledgement and notes are session-only.

---

```
  ─────────────────────────────────────────
   A missed interaction is not a statistic.
   It is the next patient at the window.
  ─────────────────────────────────────────
```

Licensed under [MIT](LICENSE). Not affiliated with HOSxP, BMS, or any
hospital system vendor.
