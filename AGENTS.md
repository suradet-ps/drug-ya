# AGENTS.md - Drug-Ya

This file tells coding agents and future contributors how to work on this
repository without breaking its clinical, privacy, or read-only invariants.
Read it before changing a query, a rule, or a finding severity.

## 1. What this project is

Drug-Ya is a **Tauri 2 desktop application for hospital pharmacists**. It
reads today's OPD orders from HOSxP (MySQL/MariaDB) in strict read-only mode,
adds patient context (allergies, recent labs, renal function, ICD-10 diagnoses,
pregnancy status), and runs a **site-configurable rule pack** over the order
set. Findings are explanations for a pharmacist, not automatic order changes.

The application never writes to HOSxP. It never signs for a prescriber. It
never claims that an unknown drug or a missing lab was reviewed.

### Design goals

- One patient, one day, one review screen.
- Every finding names the order lines and the patient facts that triggered it.
- Every finding cites a source and carries a severity that the site can adjust.
- The coverage report is first-class: missing data is shown, not hidden.
- The rule pack is data-driven and must be signed off by the site pharmacist.
- Read-only HOSxP access without exceptions.
- Patient data stays in memory; no local review history in v1.

### Non-goals

- Not a prescribing system or an autonomous clinical decision maker.
- Not a complete drug interaction database.
- Not a full medication reconciliation or BPMH product.
- Not an EHR, stock system, or billing tool.
- Not a system that writes to HOSxP.
- Not a system that stores patient data on disk in v1.

## 2. Hard rules (non-negotiable)

1. **Never write to HOSxP.** No `INSERT`, `UPDATE`, `DELETE`, `ALTER`, `CREATE`,
   `DROP`, or any DDL. Not in tests, not "just once". All DB access goes through
   `crates/drug-ya-hosxp`, which must remain SELECT-only end to end.
2. **Never log PHI or secrets.** HN, CID, names, addresses, drug lines, and
   connection credentials never appear in logs or error messages. Log timing,
   counts, and error classes only. `Patient`/`OrderLine` must never be formatted
   into a log line.
3. **Parameterized queries only.** No SQL string interpolation of user input.
   Dynamic `IN (?, ?, ...)` lists are built only by repeating `?`.
4. **No schema guessing without a flag.** Every table/column not verified
   against the target instance carries a `SCHEMA-UNVERIFIED` marker and is
   tracked in `docs/database.md`. Remove the marker only after sign-off.
5. **Login reads `officer` only.** Never query `opduser`, never select
   `officer_login_password` (plaintext), never log credentials, and never
   persist a login token or login log to HOSxP. See `docs/auth.md`.
6. **The rule pack is not a medical device claim.** The starter rules are a
   review aid. The site pharmacist must review evidence, thresholds, and local
   practice and sign off the pack before clinical use.
7. **Do not auto-change orders.** Drug-Ya may recommend "contact the prescriber"
   or "review the dose"; it must not cancel, substitute, or rewrite an order.
8. **No patient data on disk.** Settings may hold the site label, rule
   overrides, thresholds, and sign-off. They must never hold patient identity,
   orders, labs, diagnoses, allergies, or review notes. The acknowledgement and
   note are session-only.
9. **Every finding has a source.** No rule may ship without `source` and
   `evidence` text. A finding without provenance is not acceptable.
10. **Coverage is part of the result.** If a lab, diagnosis, weight, sig, or
    drug mapping is missing, the review must say so and mark the affected
    category `Partial` or `Skipped`. Never let "no findings" imply "everything
    was checked".
11. **Thai belongs to the edges.** Thai UI strings live in the frontend and in
    user-facing command errors. Core labels that are shared by the UI and tests
    (Thai dates, severity labels, rule titles) live in `drug-ya-core` and are
    deliberately user-facing. Code comments and docs are English.

## 3. Workspace layout

```
drug-ya/
├── crates/
│   ├── drug-ya-core/          pure domain; no I/O, no sqlx
│   │   ├── auth.rs            officer session + MD5 verification
│   │   ├── dictionary.rs      generic keys, aliases, classes, high-alert
│   │   ├── dose.rs            conservative sig/dose/days-supply parsing
│   │   ├── engine.rs          rule evaluation, findings, coverage
│   │   ├── model.rs           serializable domain + IPC types
│   │   ├── query_kind.rs      HN/CID/name/queue classification
│   │   ├── renal.rs           CKD-EPI 2021 + Cockcroft-Gault
│   │   ├── rules.rs           rule pack, triggers, settings, starter rules
│   │   └── thai.rs            Thai/BE formatting
│   ├── drug-ya-hosxp/         the only crate allowed to talk to MySQL
│   │   ├── client.rs          read-only repository + dataset assembly
│   │   ├── queries.rs         every SQL constant + guard tests
│   │   ├── readonly.rs        comment/semicolon-aware SQL guard
│   │   └── config.rs          Host/port/db/user/password
│   ├── drug-ya-config/        encrypted connection.json + plain settings.json
│   └── drug-ya-bridge/        WASM -> Tauri invoke wrapper
├── apps/drug-ya-app/
│   ├── src/commands.rs        thin Tauri command adapters (Thai errors here)
│   ├── src/state.rs           backend session, pool, login throttle
│   ├── frontend/src/          Leptos 0.8 CSR UI
│   ├── frontend/style.css     design tokens + all UI styles
│   └── frontend/fonts/        bundled Sarabun + Inter WOFF2
├── docs/
│   ├── DESIGN.md
│   ├── CLINICAL-RULES.md
│   ├── database.md
│   ├── verify-schema.sql
│   └── auth.md
├── icon-master.svg
└── Cargo.toml
```

Dependency direction:

```text
frontend -> drug-ya-core
drug-ya-hosxp -> drug-ya-core
drug-ya-app -> core + hosxp + config
drug-ya-config -> core (ReviewSettings only)
```

`drug-ya-core` must stay testable without a database. The default test suite
must not require a live HOSxP instance.

## 4. Domain model

### Inputs

- `Patient` - HN, CID, name, birth date, sex, age.
- `TodayVisit` - VN, date, time, queue, department, prescriber, order count.
- `OrderLine` - line id, icode, drug name, strength, units, sig, qty, item
  number, prescriber, order date/time, resolved generic key, classes,
  high-alert flag, known flag, estimated mg/day, estimated days supply, dose
  note.
- `AllergyRecord` - agent, symptom, reporter, report date, note.
- `DiagnosisRecord` - ICD-10, local label, date, diag type.
- `LabValue` - parsed value, original text, unit, measured date, item name, flag.
- `ClinicalContext` - age, sex, weight/height/BMI, creatinine, eGFR + source,
  Cockcroft-Gault CrCl, potassium, INR, ALT, AST, HbA1c, pregnancy status +
  source, diagnoses, PMH, allergies, data-availability flags, medication-scope
  note.

### Outputs

- `Finding` - id, rule id, category, effective severity, title, detail,
  recommendation, evidence, source, patient factors, order line ids, drug
  names, acknowledgement requirement, data note.
- `ReviewSummary` - counts, actionable count, Thai headline, risk level.
- `CategoryCoverage` - checked/partial/skipped, rule count, finding count, note.
- `CoverageReport` - category rows, missing data, unresolved drugs, known/total
  orders, checked/applied rule counts, notes.
- `ReviewResult` - patient, date, visits, orders, clinical context, summary,
  findings, coverage, warnings, rule-pack metadata, checked-at display string.

### Severity

`Critical` → `High` → `Moderate` → `Low` → `Info` (declaration order is also
the sort rank). Critical/High require acknowledgement in the UI. Info includes
the high-alert checklist, which is not a problem by itself.

### Categories

Allergy, Interaction, DuplicateTherapy, Renal, Dose, Geriatric, Disease, Lab,
Pregnancy, Pediatric, HighAlert. Coverage is reported for every category, even
when it is skipped.

## 5. Rule engine

`drug-ya-core::engine::analyze(patient, orders, clinical, settings, pack)`

- Applies per-rule overrides (enabled, severity, threshold).
- Evaluates drug pairs/triples, duplicate classes, class burden, single-drug
  patient conditions, lab thresholds, max daily dose, and allergy cross-reactivity.
- Uses only **today's orders** (all today's visits, one patient).
- Derives renal values from the latest creatinine + age/sex/weight.
- Deduplicates overlapping threshold bands, keeping the highest severity for the
  same category + order-line set.
- Filters by the site's minimum severity, sorts, and produces the coverage report.
- Never calls a database, never reads a file, never writes.

### Add or change a rule

1. Add a `Rule` to `default_rule_pack()` in `crates/drug-ya-core/src/rules.rs`.
2. Use a stable id, a category, a severity, Thai title/detail/recommendation,
   evidence, and a source.
3. Use `DrugSelector::keys` for specific generics and `classes` for classes.
4. Add or update a test in `engine.rs` if the trigger is non-trivial.
5. Run `cargo test -p drug-ya-core` and `cargo clippy`.
6. Update `docs/CLINICAL-RULES.md` if the rule changes the documented scope.
7. The site pharmacist re-signs the pack version when rules change.

### Drug dictionary

`dictionary.rs` maps local `drugitems.name` text to a stable generic key and
therapeutic classes. Longest alias wins. A drug that does not match is shown as
`ยังไม่รู้จัก` and included in `coverage.unresolved_drugs`; it must never be
silently treated as safe. Add aliases carefully; avoid short or ambiguous
strings. Every class used by a rule must have a Thai label in `CLASS_LABELS`;
the test `every_referenced_class_has_a_thai_label` enforces this.

## 6. HOSxP integration

All schema assumptions are in `docs/database.md` and must be verified against
the real instance. The connector falls back gracefully on schema drift:

- `ovst.oqueue` missing → try legacy `ovst.queue`, then visits load without
  queue; queue search returns a Thai message. Sra Bos uses `oqueue` (int), not
  `queue`.
- `drugusage`/`sp_use`/`doctor` missing → orders load without sig/doctor.
- `drugitems.strength`/`units` missing → orders load in minimal shape; dose
  review reports "cannot estimate".
- `opd_allergy` missing → allergy category skipped.
- `opd_allergy.icode`/`seriousness_id`/`no_alert`/`force_no_order` missing →
  minimal allergy query; text matching still works.
- Lab `AST` is named `SGOT` on Sra Bos; the classifier must not use
  `LIKE '%ast%'` (it matches Blast/Cast/Yeast).
- `ovstdiag`/`ovst` missing → disease category skipped.
- lab tables missing → renal/lab categories skipped.
- `opdscreen`/`pmh` missing → weight/PMH warnings; renal CrCl skipped.

### Core tables/columns used

| Purpose | Table | Columns |
|---|---|---|
| Login | `officer` | id, login name, password MD5, active, auth type, doctor code, name fields |
| Patient | `patient` | hn, cid, pname, fname, lname, birthday, sex |
| Visits | `ovst` | vn, hn, vstdate, vsttime, **oqueue** (int), main_dep, doctor |
| Department | `kskdepartment` | depcode, department |
| Prescriber | `doctor` | code, name, licenseno |
| Orders | `opitemrece` | vn, hn, an, icode, qty, item_no, drugusage, sp_use, doctor, rxdate, rxtime, vstdate, sub_type, item_type |
| Drug master | `drugitems` | icode, name, strength, units |
| Sig | `drugusage`, `sp_use` | code, name1, name2, name3 |
| Allergy | `opd_allergy` | hn, agent, symptom, reporter, report_date, note, icode, seriousness_id, no_alert, force_no_order |
| Diagnoses | `ovstdiag` | vn, hn, icd10, diagtype |
| Labs | `lab_head`, `lab_order`, `lab_items` | lab_order_number, hn, order_date, confirm_report, lab_items_code, lab_items_name, lab_items_unit, lab_order_result, confirm |
| Vitals | `opdscreen` | hn, vstdate, bw, height, bmi, pmh |

### Read-only enforcement

1. Dedicated `GRANT SELECT` DB user.
2. `SET SESSION TRANSACTION READ ONLY` on every pooled connection.
3. `readonly.rs` guard: comment-aware, semicolon-aware, only `SELECT`/`WITH`.
4. Compile-time constants and bound `?` parameters only.
5. Tauri capabilities: `core:default` only.

Every SQL constant gets a guard test.

## 7. Login and session

- Login gate is the first screen. Patient UI does not exist behind it.
- Verify `officer_login_name`, `officer_active = 'Y'`,
  `COALESCE(officer_authentication_type_id,1) = 1`, and
  `MD5(password) = officer_login_password_md5` case-insensitively. **On Sra Bos
  the auth-type column is NULL for all 104 accounts**; the COALESCE is
  mandatory and must not be changed to a literal `= 1` check.
- Display name: officer name fields → `doctor.name` via `officer_doctor_code` →
  login name.
- Never select `officer_login_password`; never query `opduser`.
- Throttle failed attempts: 5 failures, then growing lockout (30s base, up to
  4 doublings).
- Session is memory-only and cleared on logout, restart, or connection change.
- All patient, rule-settings, and review commands require a session.
- Every signed-in officer gets `role = "STAFF"`; HOSxP group-based roles are a
  later milestone.

## 8. Configuration and privacy

Two files under the platform config dir (`DrugYa`):

- `connection.json` - encrypted host/port/database/user/password with
  `encryptman` AES-256-GCM; master key in the OS keychain (`DrugYa` service).
- `settings.json` - plain JSON: site label, enabled categories, minimum
  severity, elderly age, renal metric, pregnancy/pediatric/high-alert toggles,
  finding sort, per-rule overrides, rule-pack sign-off. Never patient data.

The frontend never touches the database. It calls Tauri commands only. The
password is a `SecretString` in Rust and is never returned to the frontend.
No patient data is persisted; acknowledgement and notes are session-only.

## 9. Testing expectations

Default suite, no database:

- `drug-ya-core`: dictionary resolution, dose parsing, CKD-EPI/Cockcroft-Gault,
  Thai/BE formatting, query classification, MD5 verification, rule pack integrity
  (unique ids, every class labelled, thresholds/overrides), engine findings for
  interaction/allergy/renal/disease/duplicate/pregnancy, coverage gaps.
- `drug-ya-hosxp`: read-only guard tests for every SQL constant, query shape
  tests, lab-name classification, numeric parsing, pregnancy-code window.
- `drug-ya-config`: encrypted connection round-trip, no plaintext credentials on
  disk, settings round-trip, missing-field defaults, legacy migration.
- `apps/drug-ya-app/frontend`: native `cargo check` for view/type correctness;
  a wasm `trunk build` for the real target. No browser tests yet.

Live database tests (future): gated behind an integration feature; never run
against production; read the connection from an environment variable; do not
commit test patient data or credentials.

## 10. Style and conventions

- Rust 2024, rustfmt, clippy with warnings denied.
- One typed `thiserror` error per crate boundary. Translate to Thai only in
  `src-tauri/commands.rs` (here: `apps/drug-ya-app/src/commands.rs`).
- Prefer small explicit functions. Avoid `unwrap`/`expect` on production paths
  without a documented invariant.
- CSS is plain CSS with tokens in `:root`; no framework.
- No ORM or heavy query builder: `sqlx::query_as` with explicit row structs.
- Do not add write-capable Tauri capabilities.
- Do not add a second way to reach MySQL.
- Do not add patient-data persistence, print history, or local audit logs
  without a separate privacy and retention decision.
- The supplied `icon-master.svg` has its outer drop shadow removed by request;
  do not add another outer shadow to the in-app mark.

## 11. What not to do

- Do not infer a drug class from a trade name without dictionary evidence.
- Do not treat an unknown drug as safe or as covered.
- Do not hide missing data to make the review look clean.
- Do not silently change a rule threshold or severity without a visible override.
- Do not auto-cancel/change orders or write recommendations into HOSxP.
- Do not print a signature or generate a prescriber signature.
- Do not store patient data in `settings.json`, logs, error strings, or the
  webview's localStorage.
- Do not add PDF/print output in v1 without a privacy and record-retention
  decision.
- Do not claim the rule pack is complete or validated before the site sign-off.

## 12. Milestones

| Milestone | Scope |
|---|---|
| **D0** | Sibling-app study, HOSxP table patterns, review framing |
| **D1** | Visual system, mark without shadow, rule-pack design |
| **D2** | Schema verification package + SQL |
| **M0** | Tauri 2 + Leptos 0.8, login gate, encrypted connection |
| **M1** | Search by HN/CID/name/queue, patient context |
| **M2** | Today's orders + labs + derived clinical context |
| **M3** | Configurable rule engine + findings + coverage |
| **M4** | Settings, thresholds, sign-off, session confirmation |
| **M5** | Site schema verification, clinical rule review, pilot |

Current status: M0-M4 are implemented. Schema verification against Sra Bos was
completed on 2026-11-04 (`docs/database.md`); the remaining M5 work is the
pharmacist rule review/sign-off, pilot, and the optional
`opitemrece(hn, vstdate)` index.
