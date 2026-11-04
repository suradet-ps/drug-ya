# Drug-Ya

**ตรวจคำสั่งยา ก่อนถึงมือผู้ป่วย** — a pharmacist-facing medication-order review console for HOSxP.

Drug-Ya reads today's OPD orders for one patient, adds the context a pharmacist actually needs (allergies, recent labs, renal function, diagnoses, pregnancy status), and runs a **configurable, explainable rule pack** over the order set. It never writes to HOSxP, never changes an order, and never pretends an unknown drug or a missing lab was checked.

| Search ▣ | Today's orders ▣ | Findings ▣ | Rule settings ▣ | Pharmacist confirmation ▣ |
|---|---|---|---|---|

Built with **Tauri 2 + Leptos 0.8**, judged by `drug-ya-core`, read from HOSxP by `drug-ya-hosxp`.

---

## Why this exists

At the dispensing window, the risky questions are rarely "what is this drug?" They are:

- Does this patient have an allergy that crosses this order?
- Are two orders interacting, duplicated, or additive?
- Is the dose wrong for this kidney function, age, or pregnancy status?
- Is the patient on a drug that needs a second look (high-alert)?
- What **could not** be checked because a lab or a diagnosis is missing?

Drug-Ya puts those answers next to the orders, shows the patient facts that triggered each finding, and states its own coverage. The pharmacist confirms the review; the prescriber remains the decision-maker.

> The starter rule pack is **not** a complete interaction database. It must be reviewed, tested, and signed off by the site pharmacist before clinical use. See [`docs/CLINICAL-RULES.md`](docs/CLINICAL-RULES.md).

---

## What it does

### Search

One search box accepts:

| Input | Example | Behaviour |
|---|---|---|
| HN | `0012345` | exact HN (5–10 digits) |
| CID | `1101701234567` | exact 13-digit national ID |
| Name | `สมชาย` | prefix then contains, max 20 |
| Queue | `12`, `Q12`, `QH12`, `คิว 12` | today's queue (1–4 digits auto-detected) |
| Explicit mode | `HN`, `CID`, `ชื่อ`, `คิว` | overrides auto-detection |

### Review

- Patient identity, age, sex, CID, today's visits, queue, department, prescriber.
- **Allergies** with symptom and report date.
- **Recent labs**: creatinine, eGFR, CrCl, potassium, INR, ALT (SGPT), AST
  (SGOT), HbA1c.
- **Diagnoses** (ICD-10, last 365 days) and PMH free text.
- **Pregnancy status**, with a session-only pharmacist override.
- Today's orders grouped by visit, with sig, quantity, estimated days, high-alert flag, and the drug dictionary status.
- **Findings** by severity, each with patient-specific factors, recommendation, evidence, source, and the involved order lines.
- **Coverage report**: checked / partial / skipped per category, missing data, unresolved drugs.
- **Session-local acknowledgement** and pharmacist note; no patient data is written to disk.

### Configuration

- Enable/disable each rule.
- Change a rule's severity.
- Change a rule's threshold (e.g. eGFR, age, maximum mg/day).
- Enable/disable whole categories.
- Choose the renal metric used for review.
- Sign off the rule-pack version with pharmacist name, licence number, and date.
- Settings are plain JSON; connection credentials are encrypted with `encryptman` and protected by the OS keychain.

---

## Read-only by design

The app is a **read-only HOSxP client**:

1. Dedicated DB user with `GRANT SELECT` only (the real boundary).
2. `SET SESSION TRANSACTION READ ONLY` on every pooled connection.
3. A comment-aware, semicolon-aware SQL guard that accepts only a single `SELECT`/`WITH`.
4. Compile-time SQL constants and bound parameters only.
5. Tauri capabilities: `core:default` only; no filesystem or shell scope.

No `INSERT`, `UPDATE`, `DELETE`, DDL, or session/audit write is ever sent to HOSxP.

---

## Quick start

```text
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked
⟫ cargo test                  # from the repository root: DB-free tests
⟫ cd apps/drug-ya-app
⟫ cargo tauri dev             # desktop app
```

Release artifact (from `apps/drug-ya-app`):

```text
⟫ cargo tauri build
```

The app opens on a HOSxP `officer` login gate. Connection setup lives in the settings dialog. The session is memory-only.

> The connection account should be a dedicated `SELECT`-only user. Verify the schema assumptions in [`docs/database.md`](docs/database.md) before the first real use.

<details>
<summary>Verified on Sra Bos (2026-11-04)</summary>

- 15 core tables present.
- OPD queue is `ovst.oqueue` (int); there is no `ovst.queue`.
- `officer_authentication_type_id` is NULL on all 104 accounts; the query
  coalesces NULL to local DB, so login is not blocked.
- Labs use `SGOT` (not an AST-named item); the classifier avoids a raw
  `%ast%` substring.
- `opd_allergy` has `icode` (empty), `seriousness_id`, `no_alert`, and
  `force_no_order`; the engine uses them.
- `opitemrece` lacks a composite `(hn, vstdate)` index; ask the DBA to add it.

See [`docs/database.md`](docs/database.md) for the full verification.

</details>

---

## Anatomy

```
drug-ya/
├── crates/
│   ├── drug-ya-core/       pure domain: rules, engine, dictionary, dose, renal, Thai labels
│   ├── drug-ya-hosxp/      the only crate allowed to talk to HOSxP (read-only)
│   ├── drug-ya-config/     encrypted connection + plain review settings (no PHI)
│   └── drug-ya-bridge/     WASM -> Tauri invoke wrapper
├── apps/drug-ya-app/       Tauri 2 shell + Leptos 0.8 frontend
├── docs/
│   ├── DESIGN.md           product + visual/UX design
│   ├── CLINICAL-RULES.md   rule governance, evidence, limitations
│   ├── database.md         HOSxP schema verification package
│   ├── verify-schema.sql   copy/paste read-only verification queries
│   └── auth.md             officer login + session design
├── icon-master.svg         the supplied mark, outer shadow removed
└── Cargo.toml
```

Dependency direction:

```text
frontend -> drug-ya-core
drug-ya-hosxp -> drug-ya-core
drug-ya-app -> core + hosxp + config
```

`drug-ya-core` has no I/O. The default test suite never needs a database.

---

## Rule engine in one paragraph

Each rule declares a category, severity, Thai title/detail/recommendation, evidence, source, and a trigger. Triggers cover drug pairs/triples, duplicate classes, class burden, single-drug patient conditions (age, eGFR/CrCl, ICD-10, pregnancy), lab thresholds, maximum daily dose, and allergy cross-reactivity. The engine evaluates every enabled rule against **today's orders only**, deduplicates overlapping threshold bands, sorts by severity, and produces a coverage report that names the missing lab, diagnosis, weight, sig, or drug mapping that limited the review. Findings are advice for a pharmacist, not automatic order changes.

---

## Design

The visual system is olive, sage, bone, and ink — derived from the supplied mark, with the outer shadow removed. The layout is a dark search rail, a warm clinical canvas, a patient header, a compact clinical strip, an orders table, and a findings column. The UI is Thai-first; code and docs are English.

See [`docs/DESIGN.md`](docs/DESIGN.md) for tokens, layout, states, and component rules.

---

## Status

| Milestone | Scope |
|---|---|
| **D0** | Study sibling apps, HOSxP patterns, clinical review framing |
| **D1** | Visual system, logo/no-shadow mark, rule-pack design |
| **D2** | Read-only schema package, queue/lab/vitals verification SQL |
| **M0** | Tauri 2 + Leptos 0.8 scaffold, login gate, encrypted config |
| **M1** | Search by HN/CID/name/queue, patient context |
| **M2** | Today's orders + clinical context + dose/renal derivation |
| **M3** | Configurable rule engine + findings + coverage |
| **M4** | Settings, rule thresholds, pharmacist sign-off, session confirmation |
| **M5** | Site schema verification, rule review, pilot feedback |

Current status: M0–M4 are implemented. The D2 schema verification was completed
against Sra Bos on 2026-11-04 (see `docs/database.md`). The remaining M5 work is
the clinical rule review/sign-off, pilot feedback, and the optional
`opitemrece(hn, vstdate)` index.

---

## Limitations (read before use)

- Reviews **today's OPD orders only**; it is not a full medication reconciliation.
- The starter rule pack is incomplete and must be reviewed and signed off.
- Drug mapping is by local `drugitems.name`, not a national product code.
- Lab items are matched by name; site naming varies.
- Allergy records are free text; cross-reactivity requires pharmacist judgment.
- Dose estimates depend on a parseable sig and strength. If they cannot be parsed, the app says so.
- No pharmacogenomics, no full pediatric weight-based dosing, no IV compatibility, no TPN, no chemotherapy protocols.
- No patient data is persisted; the acknowledgement/note are session-only.

---

## Licence

MIT. Not affiliated with HOSxP, BMS, or any hospital system vendor.

```text
  ─────────────────────────────────────────
   A missed interaction is not a statistic.
   It is the next patient at the window.
  ─────────────────────────────────────────
```
