# Drug-Ya - HOSxP Schema Verification

This is the read-only schema package for Drug-Ya. Every table/column used by
the app is listed here. **Items marked `SCHEMA-UNVERIFIED` must be checked
against the real HOSxP instance before the pilot.** The app degrades with a
Thai warning when optional tables/columns are missing; it does not silently
pretend the check ran.

Run [`verify-schema.sql`](verify-schema.sql) on a **test/staging** instance or
with a read-only account. Do not run the queries against production during
business hours without approval.

---

## 0. Verified on Sra Bos (2026-11-04)

A full read-only verification against MariaDB 10.5.29 / charset `tis620`
found:

| Finding | Result |
|---|---|
| 15 core tables | ✅ present |
| `ovst.oqueue` | ✅ exists (int); **no `queue` column** |
| `opitemrece` drug filter | ✅ `sub_type='1'`; `item_type` only NULL/`''`/`'P'` |
| `opd_allergy` | ✅ free text `agent` + `icode` (empty), `seriousness_id`, `no_alert`, `force_no_order` |
| Diagnoses | ✅ 248,489 rows / 13,083 patients in 365 days |
| Labs | ⚠️ no item named AST; use **SGOT**; avoid `LIKE '%ast%'` |
| Vitals / PMH | ✅ confirmed |
| Officer auth | ⚠️ `officer_authentication_type_id` is **NULL on all 104 accounts**; query coalesces to 1 |
| Indexes | ⚠️ no composite `opitemrece(hn, vstdate)`; other key indexes present |

The code and this document reflect these findings. See the detailed sections
below for the exact queries and caveats.

---

## 1. Read-only account

The security boundary is the database grant, not the app. Create a dedicated
account:

```sql
-- DBA only, on the HOSxP server.
CREATE USER 'drugya_ro'@'%' IDENTIFIED BY '<strong-password>';
GRANT SELECT ON hos.* TO 'drugya_ro'@'%';
FLUSH PRIVILEGES;
```

Use the real schema name (`hos` is an example). The app also runs
`SET SESSION TRANSACTION READ ONLY` and a SQL guard, but those are defense in
depth.

Verify a write fails:

```sql
-- Must fail with a permission/read-only error.
UPDATE opitemrece SET qty = qty WHERE 1 = 0;
```

Connection settings:

| Field | Value |
|---|---|
| Host | HOSxP MySQL/MariaDB host |
| Port | usually 3306 |
| Database | the HOSxP schema |
| User | `drugya_ro` |
| Charset | app explicitly requests `utf8mb4`; the server/database may be TIS-620 (verified on Sra Bos), so MySQL converts TIS-620 column data to UTF-8 for Rust. The stored charset is never changed. |

---

## 2. Tables and columns used

### 2.1 `officer` (login only)

| Column | Use |
|---|---|
| `officer_id` | session id |
| `officer_login_name` | login |
| `officer_login_password_md5` | MD5 verification |
| `officer_active` | must be `Y` |
| `officer_authentication_type_id` | local DB indicator; on Sra Bos it is NULL for all 104 accounts and the query coalesces NULL to `1` |
| `officer_doctor_code` | display-name fallback |
| `officer_pname`, `officer_fname`, `officer_lname` | display name |

Never select `officer_login_password` (plaintext) or `opduser`.

### 2.2 `patient`

| Column | Use |
|---|---|
| `hn` | identity |
| `cid` | national ID, mask in search results |
| `pname`, `fname`, `lname` | display name |
| `birthday` | age |
| `sex` | `1` male / `2` female (verified on Sra Bos) |

### 2.3 `ovst` (today's visits)

| Column | Use |
|---|---|
| `vn` | visit key |
| `hn` | patient |
| `vstdate` | today filter |
| `vsttime` | visit time |
| `oqueue` | OPD queue number (int); verified on Sra Bos |
| `main_dep` | department |
| `doctor` | visit prescriber fallback |

**Verified on Sra Bos (2026-11-04):** `ovst` has **no `queue` column**. The
OPD queue is `ovst.oqueue` (int; 143/143 rows populated today, range 1–188).
There are also `rx_queue` (pharmacy queue, 39 rows) and `main_dep_queue`.
Drug-Ya uses `oqueue` for the visit queue and queue search.

For portability the connector tries `oqueue` first, then legacy `queue`, then
loads visits without a queue. Queue search returns a Thai message if neither
column exists. `rx_queue` is not used in v1 because it is a pharmacy-dispensing
queue, not the OPD visit queue.

### 2.4 `kskdepartment`

| Column | Use |
|---|---|
| `depcode` | join `ovst.main_dep` |
| `department` | display name |

### 2.5 `doctor`

| Column | Use |
|---|---|
| `code` | join order/visit |
| `name` | display |
| `licenseno` | not used in v1 (no print) |

### 2.6 `opitemrece` (today's orders)

| Column | Use |
|---|---|
| `vn`, `hn` | keys |
| `an` | must be NULL/empty for OPD |
| `icode` | drug key |
| `qty` | quantity; filter `> 0` |
| `item_no` | line identity component |
| `drugusage`, `sp_use` | sig lookup codes |
| `doctor` | line prescriber |
| `rxdate`, `rxtime` | order date/time (fallback to visit date/time) |
| `vstdate` | today filter |
| `sub_type` | `1` = drug |
| `item_type` | exclude `A` automatic charges |

Filter:

```sql
WHERE o.hn = ?
  AND o.vstdate = ?
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
```

`opitemrece` also holds non-drug charge lines; the `drugitems` join is
mandatory. `hos_guid` is **not** used in v1; the app builds a line id from
`vn:item_no:icode` and adds a suffix for duplicates. `item_no` is small
(`tinyint` on some sites), so it is not unique on its own.

### 2.7 `drugitems`

| Column | Use |
|---|---|
| `icode` | join |
| `name` | dictionary resolution / display |
| `strength` | dose estimation |
| `units` | dose estimation / display |

If `strength`/`units` are missing, orders still load; dose checks report
"cannot estimate".

### 2.8 `drugusage` and `sp_use`

| Column | Use |
|---|---|
| code (`drugusage`, `sp_use`) | join |
| `name1`, `name2`, `name3` | sig text; every non-empty part is kept |

The order query selects all six name columns (`drugusage.name1..3` and
`sp_use.name1..3`). The connector joins them in source order, removes exact
duplicates, and drops a short value when a longer value already contains it.
The joined string is both the complete **วิธีใช้** shown to the pharmacist and
the input to dose parsing. The UI shows this source text as-is; it does not
replace it with a generated shorthand such as `1 × 3/วัน`, which can lose the
administration timing and meal instructions.

If these tables are missing, orders load without sig and dose review is
partial. The old "pick only name3/name2/name1" behaviour was incomplete: it
could hide a timing note stored in a different name column or in `sp_use`.

### 2.9 `opd_allergy`

| Column | Use |
|---|---|
| `hn` | patient |
| `agent` | free-text allergen |
| `symptom` | free-text reaction |
| `reporter` | display |
| `report_date` | display |
| `note` | display |
| `icode` | structured drug code; **0/1643 populated on Sra Bos** |
| `seriousness_id` | HOSxP seriousness id; 1,603 rows populated |
| `no_alert` | `Y` on 5 rows; surfaced, never silently hidden |
| `force_no_order` | `Y` on 706 rows; treated as a hard-stop finding |

**Verified on Sra Bos:** `agent` is free text (non-empty on all 1,643 rows).
The app uses `icode` when it is populated, otherwise text matching + class
cross-reactivity. `force_no_order='Y'` escalates a direct match to Critical.
`no_alert='Y'` is shown as a data note ("HOSxP ตั้งค่า no_alert") rather than
silently suppressing the allergy. `seriousness_id` is displayed as recorded;
its numeric-to-severity mapping is site-dependent and is not guessed. The
connector falls back to a minimal allergy query if these optional columns are
missing. A blank table is not proof of no allergy.

### 2.10 `ovstdiag` + `ovst` (diagnoses)

| Column | Use |
|---|---|
| `ovstdiag.vn`, `hn` | keys |
| `ovstdiag.icd10` | diagnosis code |
| `ovstdiag.diagtype` | type label |
| `ovst.vstdate` | date window |

The app reads ICD-10 codes from the last 365 days, deduplicates by code, and
maps a small set of prefixes to conditions used by rules:

| Prefix | Condition |
|---|---|
| `N18`, `N19` | CKD |
| `I50` | heart failure |
| `K25`–`K27` | peptic ulcer |
| `K92.0`–`K92.2` | GI bleed |
| `K70.3`, `K74` | cirrhosis |
| `J45` | asthma |
| `J44` | COPD |
| `G20` | Parkinson |
| `F00`–`F03`, `G30` | dementia |
| `N40` | BPH |
| `H40` | glaucoma |
| `E10`–`E14` | diabetes |
| `O*` | pregnancy-related |
| `Z33`–`Z35` | pregnancy |

If ICD-10 is missing, the disease category is partial, not negative.

### 2.11 Lab tables

| Table | Column | Use |
|---|---|---|
| `lab_head` | `lab_order_number`, `hn`, `order_date`, `confirm_report` | header |
| `lab_order` | `lab_order_number`, `lab_items_code`, `lab_order_result`, `confirm` | result |
| `lab_items` | `lab_items_code`, `lab_items_name`, `lab_items_unit` | item identity |

The app fetches recent results (365 days), then classifies **by item name**:

| Kind | Name hints |
|---|---|
| Creatinine | creatinine, creat, ครีอะตินิน, ครีเอตินิน (exclude urine/24h) |
| eGFR | egfr, gfr, ckd-epi |
| Potassium | potassium, โพแทสเซียม, K, K+ |
| INR | inr, prothrombin |
| ALT | alt, sgpt, alanine aminotransferase |
| AST | **sgot**, aspartate aminotransferase, or the exact token `AST` |
| HbA1c | hba1c, a1c, glycated |

**Verified on Sra Bos:** the lab master has **no item named AST**; it uses
**SGOT** (397 rows). A naive `LIKE '%ast%'` also matches `Blast`, `Cast`, and
`Yeast` (7 rows). The classifier uses `SGOT` and a word-boundary token match,
not a raw substring. The app's tests include `Blast`/`Cast`/`Yeast` as
non-matches. Creatinine is 78 rows (mg/dL), eGFR 659, Potassium 81 with a unit
and 507 with a NULL unit, SGPT 100, HbA1C 403.

`lab_items_code` varies by hospital (e.g. one site uses `3003` for creatinine,
`3018` for potassium). **Do not hardcode those codes.** The starter app uses
names and shows the item name in the UI. For higher precision, a future
dictionary can map site-specific `lab_items_code`s.

If `lab_items_unit` or `lab_order.confirm` is missing, the connector retries a
narrower query. If all lab queries fail, the renal/lab categories are skipped
with a warning.

Creatinine unit: if the unit contains `umol`/`µmol`, the app converts to mg/dL
using `/88.4`. Otherwise it assumes mg/dL; verify the site unit.

### 2.12 `opdscreen`

| Column | Use |
|---|---|
| `hn`, `vstdate` | keys |
| `bw` | weight kg |
| `height` | height cm |
| `bmi` | BMI (computed if absent) |
| `pmh` | past medical history free text |

HOSxP writes `0` for unmeasured vitals; the app ignores non-positive values.
PMH is displayed but **not** used to trigger hard rules. If the table/columns
are missing, weight-dependent CrCl is skipped and a warning is shown.

---

## 3. Query verification

Use [`verify-schema.sql`](verify-schema.sql) for copy/paste checks. The key
pass criteria:

### V1. Queue

```sql
SHOW COLUMNS FROM ovst LIKE 'oqueue';
SHOW COLUMNS FROM ovst LIKE 'queue';
SELECT vstdate, vn, hn, oqueue
FROM ovst
WHERE vstdate = CURDATE()
LIMIT 20;
```

- [x] Sra Bos: `oqueue` exists (int), no `queue`; 143/143 rows populated today.
- [ ] Values match the format the pharmacy uses (`12`, `Q12`, `QH12`, etc.).
- [ ] If neither exists, queue search is disabled with a clear Thai message.
- [ ] Confirm whether the pharmacy window wants `oqueue` or `rx_queue`; v1
  uses `oqueue` because that is the OPD visit queue.

### V2. Today's visits

```sql
SELECT v.vn, v.hn, v.vstdate, v.vsttime, v.oqueue,
       v.main_dep, k.department, v.doctor, d.name
FROM ovst v
LEFT JOIN kskdepartment k ON k.depcode = v.main_dep
LEFT JOIN doctor d ON d.code = v.doctor
WHERE v.hn = '<TEST_HN>' AND v.vstdate = CURDATE();
```

- [ ] The expected visit(s) appear.
- [ ] Department and doctor names join.
- [ ] Queue is populated for the test visit.

### V3. Today's orders

```sql
SELECT o.vn, o.icode, o.item_no, o.qty, o.drugusage, o.sp_use,
       d.name, d.strength, d.units,
       du.name1, du.name2, du.name3,
       su.name1, su.name2, su.name3,
       o.doctor, doc.name AS doctor_name,
       o.rxdate, o.rxtime, o.vstdate, o.an, o.sub_type, o.item_type
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
LEFT JOIN drugusage du ON du.drugusage = o.drugusage
LEFT JOIN sp_use su ON su.sp_use = o.sp_use
LEFT JOIN doctor doc ON doc.code = o.doctor
WHERE o.hn = '<TEST_HN>'
  AND o.vstdate = CURDATE()
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
ORDER BY o.vn, o.item_no;
```

- [x] Sra Bos: `sub_type='1'` isolates drug lines; `sub_type='3'` (4,669 rows)
  joins 0 drugitems.
- [x] `item_type` contains only NULL/`''`/`'P'` (no `A`); the app's
  `item_type <> 'A'` filter is harmless insurance.
- [ ] Only drugs appear (no service/charge lines).
- [ ] `rxdate`/`rxtime` are populated (not only `vstdate`/`vsttime`).
- [ ] `opitemrece.doctor` matches the expected prescriber.
- [ ] `drugusage.name1` contains a parseable sig (`1x3`, `1-0-1`, etc.).

### V4. Allergies

```sql
SELECT hn, agent, symptom, reporter, report_date, note,
       icode, seriousness_id, no_alert, force_no_order
FROM opd_allergy
WHERE hn = '<TEST_HN>'
ORDER BY report_date DESC;
```

- [x] Sra Bos: `agent` is free text and non-empty on 1,643/1,643 rows.
- [x] `icode` exists but is empty on all rows.
- [x] `seriousness_id` has 1,603 rows; `no_alert='Y'` has 5 rows;
  `force_no_order='Y'` has 706 rows.
- [ ] `seriousness_id` mapping to local severity is confirmed by the pharmacist.
- [ ] Severe reactions can be recognized in `symptom` (anaphylaxis, angioedema,
  SJS/TEN, dyspnea, shock).
- [ ] Empty is not treated as "no allergy" unless the query succeeded.

### V5. Diagnoses

```sql
SELECT d.vn, d.icd10, d.diagtype, v.vstdate
FROM ovstdiag d
JOIN ovst v ON v.vn = d.vn
WHERE d.hn = '<TEST_HN>'
  AND v.vstdate >= DATE_SUB(CURDATE(), INTERVAL 365 DAY)
ORDER BY v.vstdate DESC;
```

- [ ] ICD-10 codes are present and prefixed as expected.
- [ ] Chronic conditions from prior visits are visible in the window.

### V6. Labs

```sql
SELECT lh.order_date, li.lab_items_code, li.lab_items_name,
       li.lab_items_unit, lo.lab_order_result, lh.confirm_report, lo.confirm
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
JOIN lab_items li ON li.lab_items_code = lo.lab_items_code
WHERE lh.hn = '<TEST_HN>'
  AND lh.order_date >= DATE_SUB(CURDATE(), INTERVAL 365 DAY)
  AND (li.lab_items_name LIKE '%creatinine%'
       OR li.lab_items_name LIKE '%potassium%'
       OR li.lab_items_name LIKE '%INR%'
       OR li.lab_items_name LIKE '%ALT%'
       OR li.lab_items_name LIKE '%SGPT%'
       OR li.lab_items_name LIKE '%SGOT%'
       OR li.lab_items_name LIKE '%HbA1c%')
ORDER BY lh.order_date DESC;
```

- [x] Sra Bos names: Creatinine 78 (mg/dL), eGFR 659, Potassium 81 (mmol/L)
  + 507 (NULL unit), SGPT 100, SGOT 397, HbA1C 403.
- [x] `confirm_report='Y'` + `confirm='Y'` is the correct filter (2,828 rows
  with results; Y/N = 3 without results).
- [ ] The lab names match the classifier (or add aliases).
- [ ] Units are mg/dL for creatinine, mmol/L for potassium, etc.
- [ ] `lab_items_unit` exists; if not, the fallback works. The app falls back to
  the conventional unit when the lab unit is NULL.
- [ ] No lab named `AST`; use `SGOT` or an exact `AST` token. `LIKE '%ast%'`
  matches Blast/Cast/Yeast and must not be used.

### V7. Vitals and PMH

```sql
SHOW COLUMNS FROM opdscreen;
SELECT vstdate, bw, height, bmi, pmh
FROM opdscreen
WHERE hn = '<TEST_HN>'
ORDER BY vstdate DESC
LIMIT 5;
```

- [ ] `bw`, `height`, `bmi`, `pmh` exist.
- [ ] Weight is in kg and height in cm.
- [ ] Zero values are treated as unmeasured.

### V8. Officer

```sql
SELECT officer_id, officer_login_name, officer_active,
       officer_authentication_type_id, officer_doctor_code,
       officer_pname, officer_fname, officer_lname
FROM officer
WHERE officer_login_name = '<TEST_LOGIN>';
```

- [x] Sra Bos: `officer_login_password_md5` is present and 32 characters.
- [x] **`officer_authentication_type_id` is NULL for all 104 accounts.** The
  SQL uses `COALESCE(...,1)`, so NULL is treated as local DB and login is not
  blocked. Do not change the command to require a literal `1`.
- [ ] The MD5 login assumption works (test one real account).
- [ ] Never select `officer_login_password`.

### V9. Indexes / performance

```sql
SHOW INDEX FROM ovst;
SHOW INDEX FROM opitemrece;
SHOW INDEX FROM patient;
SHOW INDEX FROM lab_head;
```

- [x] Sra Bos: `ovst(hn, vstdate)` exists; `lab_head(hn, order_date)` exists;
  patient `hn`/`cid`/name indexes exist.
- [x] `opitemrece` has no composite `(hn, vstdate)` index; only separate indexes.
  The today's-order query may scan more than ideal.
- [ ] Ask the DBA to add `opitemrece(hn, vstdate)` (or an equivalent) before
  the pilot; target < 300 ms.
- [ ] Patient search by HN/CID/name uses existing indexes; name search may do a
  scan on some sites.

The app runs a 5-second `max_execution_time` best-effort and a 5-connection
pool. If today's order query is slow, ask the DBA for a composite index on
`opitemrece(hn, vstdate)`.

---

## 4. Data-quality checks

Run these once on a staging copy and record the results:

```sql
-- Missing sig on today's drug lines
SELECT COUNT(*) AS missing_sig
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.vstdate = CURDATE()
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
  AND (o.drugusage IS NULL OR o.drugusage = '');

-- Missing strength/units
SELECT COUNT(*) AS missing_strength
FROM drugitems
WHERE strength IS NULL OR TRIM(strength) = '' OR units IS NULL OR TRIM(units) = '';

-- Empty allergy table vs no rows for one patient
SELECT COUNT(*) AS allergy_rows FROM opd_allergy;
SELECT COUNT(*) AS diagnosis_rows FROM ovstdiag;

-- Queue format
SELECT DISTINCT oqueue FROM ovst WHERE vstdate = CURDATE() ORDER BY oqueue LIMIT 20;

-- Allergy structured columns
SELECT SUM(icode <> '') AS with_icode,
       SUM(seriousness_id IS NOT NULL AND seriousness_id <> '') AS with_seriousness,
       SUM(no_alert = 'Y') AS no_alert_rows,
       SUM(force_no_order = 'Y') AS force_no_order_rows,
       COUNT(*) AS total
FROM opd_allergy;

-- Creatinine lab names at this site
SELECT DISTINCT lab_items_code, lab_items_name, lab_items_unit
FROM lab_items
WHERE lab_items_name LIKE '%creat%'
   OR lab_items_name LIKE '%ไต%'
   OR lab_items_name LIKE '%โพแทสเซียม%'
   OR lab_items_name LIKE '%SGOT%'
   OR lab_items_name LIKE '%SGPT%'
ORDER BY lab_items_name;
```

Record counts and examples in the site verification log. Do not commit patient
identifiers, names, or real lab values.

---

## 5. Fallback behavior in the connector

| Missing object | Behavior |
|---|---|
| `ovst.oqueue` missing | try legacy `ovst.queue`; then visits without queue; queue search message |
| `drugusage`, `sp_use`, `doctor` | orders load without sig/doctor; warning |
| `drugitems.strength`, `units` | orders load minimal; dose review "cannot estimate" |
| `opd_allergy` | allergy category skipped |
| `opd_allergy.icode/seriousness_id/no_alert/force_no_order` | minimal allergy query; text matching still works |
| `ovstdiag` / `ovst` | disease category skipped |
| `lab_head` / `lab_order` / `lab_items` | renal/lab categories skipped |
| `lab_items_unit` | lab query without unit |
| `lab_order.confirm` | lab query without that filter |
| `opdscreen.bw/height` | weight-dependent CrCl skipped |
| `opdscreen.pmh` | PMH absent; no warning if optional |
| `patient` / `opitemrece` / `drugitems` | hard error; cannot review |

The rule engine never infers a value to fill a gap. It reports a gap.

---

## 6. Privacy notes

- All queries are by HN or today's date; no bulk export.
- HN/CID/name are never logged. Error commands carry Thai messages only.
- Drug names in logs are avoided where they could identify a patient; search
  logging records only kind and result count.
- No patient data is written to disk. Settings contain rule overrides and
  sign-off only.
- The user manual / screenshots should not include real patient data.

---

## 7. Sign-off log

| Date | Verifier | Scope | Result | Notes |
|---|---|---|---|---|
| | | queue/visits | | |
| | | orders/sig | | |
| | | allergy | | |
| | | diagnoses | | |
| | | labs | | |
| | | vitals | | |
| | | officer login | | |
| | | indexes/perf | | |
| | | read-only grant | | |
| | | rule pack | | |

Keep this log outside the repository if it contains staff names or site
identifiers that should not be public.
