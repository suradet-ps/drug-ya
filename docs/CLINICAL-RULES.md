# Drug-Ya - Clinical Rule Pack & Governance

> **This is a starter rule pack, not a validated clinical knowledge base.**
> A site pharmacist must review the evidence, thresholds, local practice, and
> drug dictionary before clinical use. The app is a review aid; it does not
> prescribe, cancel, or change orders.

---

## 1. What the engine does

`drug-ya-core::engine::analyze` receives:

- today's OPD orders for one patient (all today's visits);
- the patient's allergies, recent diagnoses, recent labs, weight, age, sex,
  pregnancy status;
- the site's effective rule pack (defaults + overrides + thresholds);
- the review settings (enabled categories, minimum severity, elderly age,
  renal metric).

It returns:

- **findings** - explicit, sourced explanations;
- **summary** - counts and a risk headline;
- **coverage** - what was checked, partially checked, or skipped, and why.

The engine is deterministic and has no I/O. The same inputs always produce the
same output. This matters for testing and for explaining a finding to a
prescriber.

---

## 2. Severity

| Severity | Meaning | UI action |
|---|---|---|
| **Critical** | Contraindicated / high risk of serious harm. Contact the prescriber before dispensing. | red; acknowledgement required |
| **High** | Avoid, adjust, or intervene; significant risk. | orange; acknowledgement required |
| **Moderate** | Monitor, adjust, or confirm. | amber |
| **Low** | Awareness / small issue. | sage |
| **Info** | High-alert checklist or context; not a problem by itself. | grey |

Severity is site-editable per rule. The rule pack default is a starting point,
not a mandate. Lowering a severity is a clinical governance decision and should
be documented.

---

## 3. Categories

### 3.1 Allergy / ADR

Three mechanisms:

1. **Structured match** - when `opd_allergy.icode` is populated, it is matched
   to `opitemrece.icode`. On Sra Bos, `icode` is empty on all 1,643 rows, but
   the path is kept for sites that use it.
2. **Direct text match** - `opd_allergy.agent` matches the order's name or
   generic key. Severity is **Critical** if the recorded symptom suggests a
   severe reaction (anaphylaxis, angioedema, SJS/TEN, dyspnea, hypotension,
   shock) or if `force_no_order='Y'`; otherwise **High**.
3. **Class cross-reactivity** - explicit rules for common class labels:
   penicillin → cephalosporin; sulfonamide; NSAID; cephalosporin; macrolide;
   fluoroquinolone; opioid; tetracycline.

HOSxP structured flags:

- `force_no_order='Y'` (706 rows on Sra Bos) escalates a direct match to
  **Critical** and recommends not dispensing.
- `no_alert='Y'` (5 rows) is surfaced as a data note; Drug-Ya does **not**
  silently suppress a recorded allergy.
- `seriousness_id` (1,603 rows) is displayed as recorded; the numeric-to-
  severity mapping is site-dependent and is not guessed.

Limitations:

- `opd_allergy.agent` is free text on HOSxP; it may be a trade name, a class,
  or a misspelling.
- Cross-reactivity is not an automatic contraindication. The pharmacist must
  review the original reaction, severity, timing, and alternatives.
- A blank allergy table is not proof of no allergy. The coverage report says
  whether the allergy source loaded.

### 3.2 Drug-drug interactions

The starter pack covers well-established, high-consequence pairs/triples, e.g.:

- warfarin + NSAID / antiplatelet / amiodarone / fluconazole / metronidazole /
  co-trimoxazole / macrolide / rifampicin / antiepileptic;
- DOAC + strong CYP3A4/P-gp inhibitor / rifampicin / antiepileptic / NSAID;
- antiplatelet + NSAID; aspirin + other NSAID;
- ACEI/ARB + potassium-sparing diuretic / potassium / NSAID;
- ACEI/ARB + NSAID + diuretic ("triple whammy");
- spironolactone/eplerenone + potassium;
- digoxin + amiodarone / verapamil / clarithromycin / diuretic;
- methotrexate + NSAID / co-trimoxazole;
- lithium + NSAID / ACEI/ARB / thiazide;
- SSRI + tramadol / MAOI / NSAID;
- opioid + benzodiazepine / gabapentinoid;
- clopidogrel + omeprazole/esomeprazole / strong CYP2C19 inhibitor;
- statin + strong CYP3A4 inhibitor; simvastatin + amiodarone;
- colchicine + clarithromycin/azole;
- sulfonylurea + insulin;
- beta-blocker + verapamil/diltiazem / insulin/sulfonylurea;
- NSAID + corticosteroid / diuretic;
- theophylline + ciprofloxacin / macrolide;
- carbamazepine + macrolide;
- phenytoin + fluconazole;
- valproate + lamotrigine;
- rifampicin + oral contraceptive;
- isotretinoin + tetracycline;
- QT-prolonging combinations; domperidone + macrolide/azole.

Each rule carries a mechanism/evidence note and a source. Severity can be
changed by the site. A negative finding does **not** mean "no interaction
exists"; it means no enabled rule matched a known order with known data.

### 3.3 Duplicate therapy / class burden

- Duplicate-class rules for NSAIDs, SSRIs, benzodiazepines, Z-drugs, PPIs,
  statins, ACEIs, ARBs, opioids, antipsychotics, sulfonylureas, insulin.
- Class-burden rules for anticholinergic and sedative load in older adults
  (age threshold from settings).

Duplicate therapy may be intentional during a cross-taper or switch. The
finding asks the pharmacist to review, not to stop the drug.

### 3.4 Renal

- CKD-EPI 2021 for eGFR (no race coefficient).
- Cockcroft-Gault for CrCl (needs current weight).
- The site chooses which metric to use; the UI always shows both when possible.
- Rules for metformin, NSAIDs, methotrexate, colchicine, digoxin, enoxaparin,
  DOACs, gabapentinoids, potassium-sparing diuretics, potassium supplements,
  allopurinol/febuxostat, metoclopramide, lithium.
- Threshold bands are separate rules; the engine keeps the highest severity for
  the same drug/order set and suppresses the lower band. This avoids showing
  "metformin <45" and "metformin <30" as two confusing findings.

Limitations:

- A single creatinine is a snapshot. Acute kidney injury, muscle mass, pregnancy,
  and amputation affect interpretation.
- CrCl needs weight; if weight is missing, the app says so and cannot run
  CrCl-based rules.
- The app does not compute a full renal dose for every drug. It flags the
  common high-risk ones.

### 3.5 Dose

- Maximum daily dose rules for paracetamol, ibuprofen, diclofenac, naproxen,
  mefenamic acid, tramadol, citalopram, escitalopram, sertraline, fluoxetine,
  amitriptyline, metoclopramide, simvastatin, atorvastatin, rosuvastatin,
  digoxin (elderly).
- Age-specific rules (paracetamol and citalopram/escitalopram in older adults).
- Dose is estimated from `drugitems.strength` × sig frequency. If the sig,
  strength, or units cannot be parsed, the app reports "cannot estimate" and
  does not guess.
- Days supply is **derived**, not stored by HOSxP. It is shown only when the
  sig states a dose per administration and a daily frequency, for example
  `ครั้งละ 2 เม็ด · วันละ 1 ครั้ง` → 2 units/day. Thai dose forms such as
  `ครั้งละ ครึ่ง เม็ด`, `ครั้งละ 1/2 เม็ด`, `วันละ 2 ครั้ง`, `วันละ 2 เวลา`,
  and scheduled `ก่อนนอน` are supported.
- A PRN or conditional sig such as `ก่อนนอน · เวลานอนไม่หลับ` is not given a
  numeric days estimate; the UI shows `ตามอาการ`. A missing frequency shows
  `ไม่ระบุความถี่`, and an unparsed dose shows `อ่านไม่ชัด`. The number is
  always prefixed with `~` and is an estimate, not a guarantee of use.
- Liquids are only converted when the sig states a volume unit; "1x3" for a
  liquid is treated as ambiguous.
- Injections and non-solid forms are not assessed for mg/day.
- Methotrexate weekly dosing is not reduced to a daily max; it is covered by
  high-alert and interaction rules.

### 3.6 Geriatric

Potentially inappropriate medications in older adults, based on a curated
subset aligned with the **AGS Beers Criteria 2023**:

- benzodiazepines, Z-drugs, anticholinergics, sedating antihistamines;
- NSAIDs, long-acting sulfonylurea (glibenclamide), antipsychotics;
- metoclopramide, amitriptyline, oxybutynin, pethidine;
- digoxin > 0.125 mg/day (dose rule).

The site sets the elderly age (default 65). Rules can be disabled or re-graded.
Beers is a US-oriented tool; local practice and availability may differ.

### 3.7 Drug-disease

ICD-10-based rules for:

- NSAIDs + CKD, heart failure, peptic ulcer/GI bleed, cirrhosis;
- metformin + liver disease;
- non-selective beta-blocker + asthma/COPD;
- antipsychotic + Parkinson disease / dementia;
- anticholinergic + BPH / glaucoma;
- corticosteroid + diabetes.

Diagnoses are read from `ovstdiag` for the last 365 days and deduplicated by
code. Free-text PMH is displayed but **not** used to trigger hard rules. If
ICD-10 is missing, the category is partial/skipped, not "negative".

### 3.8 Lab

- Potassium high + ACEI/ARB, potassium-sparing diuretic, potassium supplement.
- Potassium low + digoxin.
- INR high + warfarin.
- ALT/AST high + statin.

The app uses the latest relevant lab within 365 days. It reports the result date
and value. A missing lab is a coverage gap, not a normal result.

**Lab-name caveat:** Sra Bos has no item named AST; it uses **SGOT** (397
rows). The classifier uses `SGOT` and an exact `AST` token, never a raw
`LIKE '%ast%'` (which matches Blast/Cast/Yeast). See `docs/database.md`.

### 3.9 Pregnancy

- ACEI/ARB, warfarin, DOAC, methotrexate, retinoids, valproate (Critical).
- Statins, tetracyclines, NSAIDs, lithium (High).

Pregnancy status is derived from an ICD-10 O-code/Z33-Z35 within 280 days when
available. Otherwise it stays **Unknown**; the pharmacist can confirm
pregnant/not pregnant for the session. The engine never assumes "not pregnant".

### 3.10 Pediatric

- Codeine and tramadol under 12 (Critical).
- Aspirin under 18 (High; Reye syndrome context).
- Tetracyclines under 8 (High).
- Fluoroquinolones under 18 (Moderate).
- Promethazine under 2 (Critical).

This is a safety net, not a pediatric dose calculator. Weight-based pediatric
dosing is out of scope for v1.

### 3.11 High-alert

The dictionary marks high-alert drugs (anticoagulants, insulin, opioids,
methotrexate, digoxin, potassium chloride, etc.). The engine shows an **Info**
checklist finding for each. These are not problems by themselves; they require
an independent double-check per local policy. The list can be disabled in
settings.

---

## 4. Coverage and the "false all-clear" problem

A review can say "no findings" only when the engine has enough data. Coverage
tracks:

| Category | Checked when | Partial/skipped when |
|---|---|---|
| Allergy | `opd_allergy` loaded | table missing |
| Interaction | ≥2 orders, drug dictionary coverage | unknown drugs, <2 orders |
| Duplicate | ≥2 orders | unknown drugs |
| Renal | eGFR or CrCl available | no creatinine, no weight for CrCl |
| Dose | sig/strength parsed for all relevant orders | unparsed sig/strength |
| Geriatric | age known and ≥ threshold | age unknown or below threshold |
| Disease | `ovstdiag` loaded with codes | no codes/table missing |
| Lab | relevant lab values available | no potassium/INR/ALT/AST |
| Pregnancy | status known | unknown/not confirmed |
| Pediatric | age known and <18 | age unknown/adult |
| High-alert | list enabled | disabled |

Missing data and unresolved drug names are displayed in the UI, not just stored.
This is a clinical safety requirement, not a nice-to-have.

---

## 5. Evidence and source policy

Every rule must have:

- `evidence` - mechanism or rationale in Thai/English;
- `source` - a citation or reference class.

The starter pack uses:

- product labels / FDA safety communications for specific interactions and
  dose limits;
- **KDIGO 2024** CKD guideline for renal framing;
- **AGS Beers Criteria 2023** for older-adult rules;
- **ISMP High-Alert Medications** for the high-alert list;
- standard tertiary references for drug-disease and pregnancy cautions.

### What is *not* bundled

- DDInter 2.0 / DrugBank / other large interaction datasets are **not** copied
  into this repository. Their licences (often CC BY-NC / proprietary) need a
  separate review before redistribution or commercial use.
- No proprietary Thai interaction database is bundled.
- The default pack is a curated starter set, not a full knowledge base.

If the site wants a larger interaction set, it should import a licensed dataset
through a separate, reviewed adapter and keep the same source/sign-off workflow.
The current engine can support larger rule packs; the limiting factor is
clinical governance, not the data structure.

---

## 6. Site configuration

`ReviewSettings` contains:

- enabled categories;
- minimum severity shown;
- elderly age threshold;
- renal metric preference (`Auto`, CrCl, eGFR);
- pregnancy/pediatric/high-alert toggles;
- finding sort order;
- per-rule `RuleOverride { enabled, severity, threshold }`;
- pharmacist sign-off.

### Sign-off

The sign-off records:

- pharmacist name;
- licence number;
- date;
- rule-pack version;
- optional note.

The UI shows the sign-off state in the patient header and the settings modal.
If the pack version changes, the old sign-off no longer counts. A site may
re-sign the new version.

**What sign-off means:** the pharmacist has reviewed the rule conditions,
sources, thresholds, and local applicability and accepts the pack as a review
aid. It does **not** mean the app is a validated medical device or that every
finding is correct for every patient.

### Change control

1. Propose a rule change in `crates/drug-ya-core/src/rules.rs`.
2. Add/update tests and documentation.
3. Bump the pack version (`version` in `default_rule_pack`).
4. Run the full test suite and a manual test set (see below).
5. Site pharmacist re-signs.
6. Release notes mention the changed rules.

Never silently change a severity or threshold. The override system is designed
to make local changes visible and auditable.

---

## 7. Validation plan before clinical use

### 7.1 Rule-level

- [ ] Every rule has a unique id, title, detail, recommendation, evidence, source.
- [ ] Every referenced class has a Thai label (enforced by test).
- [ ] Every threshold is within a clinically plausible range.
- [ ] The pharmacist has reviewed each rule's severity and local applicability.
- [ ] The pharmacist has reviewed the drug dictionary aliases for local HOSxP
      names, including high-volume drugs and trade names.

### 7.2 Data-level

- [x] `ovst.oqueue` exists and is populated (verified on Sra Bos; no `queue`
      column).
- [ ] The pharmacy window confirms whether `oqueue` is the right queue concept
      for its workflow.
- [ ] Today's OPD visit query returns the expected visits.
- [ ] `opitemrece` order query returns only drug lines (`sub_type='1'`,
      `item_type <> 'A'`, `qty > 0`, no `an`).
- [ ] `drugusage`/`sp_use` sig text is populated for the test drugs.
- [ ] `opd_allergy` fields and free-text formats are understood.
- [ ] `ovstdiag` ICD-10 codes are present for test patients.
- [ ] `lab_items` names match the classifier for creatinine/eGFR/K/INR/ALT/AST.
- [ ] `opdscreen.bw/height/bmi/pmh` columns exist.
- [ ] The read-only DB user works and a write fails.

### 7.3 Case-level test set

Create de-identified test scenarios (not real patients) for:

1. Warfarin + ibuprofen + high INR.
2. Metformin + eGFR 25.
3. Penicillin anaphylaxis + amoxicillin.
4. Pregnancy + ACEI.
5. Elderly + benzodiazepine + Z-drug.
6. Duplicate NSAIDs.
7. Paracetamol 5 g/day.
8. Unknown drug + missing labs (coverage must be partial).
9. No orders today (empty state).
10. Missing queue column (fallback message).

### 7.4 Workflow-level

- [ ] Pharmacist can complete a review without a mouse (keyboard/focus).
- [ ] Acknowledging every required finding enables confirmation.
- [ ] Logout clears every patient-scoped value.
- [ ] Changing connection clears the session.
- [ ] The app never writes to HOSxP (network/log review).
- [ ] No PHI appears in logs or error messages.

---

## 8. Known limitations

- Today's OPD orders only; no prior-visit active medication list, no external
  hospital data, no IPD.
- Starter rule pack is not exhaustive; absence of a finding is not proof of
  safety.
- Drug mapping is local text matching, not a national product code.
- Lab matching is by item name; site names vary.
- Free-text allergy and PMH require human interpretation.
- Dose estimation is limited by HOSxP sig quality; ambiguous sigs are not guessed.
- No pharmacogenomics, IV compatibility, TPN, chemotherapy protocols, or full
  pediatric weight-based dosing.
- No local audit trail/print history in v1 (privacy decision).
- No automated order changes; the prescriber remains the decision-maker.

---

## 9. Why not auto-stop or auto-change?

Because the app does not know enough to be the decision-maker:

- the indication may be missing or miscoded;
- the allergy record may be an intolerance, not a true allergy;
- the lab may be stale or from a different clinical context;
- the patient may be on a cross-taper or a deliberate combination;
- the prescriber may have a documented reason.

The safe workflow is: **flag → explain → pharmacist reviews → contact prescriber
if needed → document the decision locally in the session.** The signed order in
HOSxP remains the record; Drug-Ya does not create a second version of the truth.
