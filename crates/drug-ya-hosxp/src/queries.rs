//! Every SQL statement Drug-Ya can execute.
//!
//! Rules:
//! - compile-time constants or `?`-placeholder lists only;
//! - values are always bound;
//! - every statement is exercised by the read-only guard test below;
//! - schema assumptions are documented in `docs/database.md`.

/// Smoke test.
pub const PING_SQL: &str = "SELECT 1 AS one";

/// One local `officer` account for password verification.
///
/// The plaintext `officer_login_password` column and `opduser` never appear.
pub const FETCH_OFFICER_SQL: &str = "\
SELECT CAST(o.officer_id AS SIGNED) AS officer_id,
       o.officer_login_name AS login_name,
       COALESCE(o.officer_login_password_md5, '') AS password_md5,
       COALESCE(o.officer_active, 'N') AS active,
       CAST(COALESCE(o.officer_authentication_type_id, 1) AS SIGNED) AS auth_type_id,
       NULLIF(o.officer_doctor_code, '') AS doctor_code,
       COALESCE(
           NULLIF(TRIM(CONCAT(COALESCE(o.officer_pname, ''),
                              COALESCE(o.officer_fname, ''), ' ',
                              COALESCE(o.officer_lname, ''))), ''),
           NULLIF(TRIM(d.name), ''),
           ''
       ) AS display_name
FROM officer o
LEFT JOIN doctor d ON d.code = NULLIF(o.officer_doctor_code, '')
WHERE o.officer_login_name = ?
LIMIT 1";

/// Patient lookup by exact HN.
pub const PATIENT_SEARCH_BY_HN: &str = "\
SELECT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM patient p
WHERE p.hn = ?
LIMIT ?";

/// Patient lookup by exact CID.
pub const PATIENT_SEARCH_BY_CID: &str = "\
SELECT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM patient p
WHERE p.cid = ?
LIMIT ?";

/// Name search - prefix first so existing indexes can be used.
pub const PATIENT_SEARCH_NAME_PREFIX: &str = "\
SELECT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM patient p
WHERE p.fname LIKE ?
   OR p.lname LIKE ?
   OR TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) LIKE ?
ORDER BY p.fname, p.lname
LIMIT ?";

/// Name search fallback - contains match, used only when prefix finds nothing.
pub const PATIENT_SEARCH_NAME_CONTAINS: &str = "\
SELECT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM patient p
WHERE p.fname LIKE ?
   OR p.lname LIKE ?
   OR TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) LIKE ?
ORDER BY p.fname, p.lname
LIMIT ?";

/// One patient by HN.
pub const PATIENT_BY_HN_SQL: &str = "\
SELECT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM patient p
WHERE p.hn = ?
LIMIT 1";

/// Today's OPD visits for a patient.
///
/// Sra Bos uses `ovst.oqueue` (int) for the OPD queue and has no `queue`
/// column. The connector falls back to [`TODAY_VISITS_BY_HN_LEGACY_QUEUE`] on
/// builds that use `queue`, then to the no-queue shape.
pub const TODAY_VISITS_BY_HN: &str = "\
SELECT v.vn, v.vstdate,
       CAST(COALESCE(v.vsttime, '') AS CHAR) AS vsttime,
       COALESCE(CAST(v.oqueue AS CHAR), '') AS queue,
       COALESCE(v.main_dep, '') AS department_code,
       COALESCE(k.department, '') AS department_name,
       COALESCE(v.doctor, '') AS doctor_code,
       COALESCE(d.name, '') AS doctor_name,
       (SELECT COUNT(*)
          FROM opitemrece o
         WHERE o.vn = v.vn
           AND o.qty > 0
           AND (o.an IS NULL OR o.an = '')
           AND o.sub_type = '1'
           AND IFNULL(o.item_type, '') <> 'A') AS order_count
FROM ovst v
LEFT JOIN kskdepartment k ON k.depcode = v.main_dep
LEFT JOIN doctor d ON d.code = v.doctor
WHERE v.hn = ? AND v.vstdate = ?
ORDER BY v.vsttime, v.vn";

/// Legacy queue-column variant for HOSxP builds that use `ovst.queue`.
pub const TODAY_VISITS_BY_HN_LEGACY_QUEUE: &str = "\
SELECT v.vn, v.vstdate,
       CAST(COALESCE(v.vsttime, '') AS CHAR) AS vsttime,
       COALESCE(CAST(v.queue AS CHAR), '') AS queue,
       COALESCE(v.main_dep, '') AS department_code,
       COALESCE(k.department, '') AS department_name,
       COALESCE(v.doctor, '') AS doctor_code,
       COALESCE(d.name, '') AS doctor_name,
       (SELECT COUNT(*)
          FROM opitemrece o
         WHERE o.vn = v.vn
           AND o.qty > 0
           AND (o.an IS NULL OR o.an = '')
           AND o.sub_type = '1'
           AND IFNULL(o.item_type, '') <> 'A') AS order_count
FROM ovst v
LEFT JOIN kskdepartment k ON k.depcode = v.main_dep
LEFT JOIN doctor d ON d.code = v.doctor
WHERE v.hn = ? AND v.vstdate = ?
ORDER BY v.vsttime, v.vn";

/// Same as [`TODAY_VISITS_BY_HN`] but without any queue column.
///
/// Some HOSxP builds have neither `oqueue` nor `queue`; this fallback keeps
/// visits usable and the UI says queue search is unavailable.
pub const TODAY_VISITS_BY_HN_NO_QUEUE: &str = "\
SELECT v.vn, v.vstdate,
       CAST(COALESCE(v.vsttime, '') AS CHAR) AS vsttime,
       '' AS queue,
       COALESCE(v.main_dep, '') AS department_code,
       COALESCE(k.department, '') AS department_name,
       COALESCE(v.doctor, '') AS doctor_code,
       COALESCE(d.name, '') AS doctor_name,
       (SELECT COUNT(*)
          FROM opitemrece o
         WHERE o.vn = v.vn
           AND o.qty > 0
           AND (o.an IS NULL OR o.an = '')
           AND o.sub_type = '1'
           AND IFNULL(o.item_type, '') <> 'A') AS order_count
FROM ovst v
LEFT JOIN kskdepartment k ON k.depcode = v.main_dep
LEFT JOIN doctor d ON d.code = v.doctor
WHERE v.hn = ? AND v.vstdate = ?
ORDER BY v.vsttime, v.vn";

/// Today's queue search using Sra Bos's `ovst.oqueue`. The three queue values
/// cover `12`, `Q12`, and `QH12` storage styles.
pub const TODAY_VISITS_BY_QUEUE: &str = "\
SELECT DISTINCT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM ovst v
JOIN patient p ON p.hn = v.hn
WHERE v.vstdate = ?
  AND TRIM(CAST(v.oqueue AS CHAR)) IN (?, ?, ?)
ORDER BY p.fname, p.lname
LIMIT ?";

/// Legacy queue-column variant for HOSxP builds that use `ovst.queue`.
pub const TODAY_VISITS_BY_QUEUE_LEGACY: &str = "\
SELECT DISTINCT p.hn, COALESCE(p.cid, '') AS cid,
       TRIM(CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), ' ', COALESCE(p.lname, ''))) AS full_name_th,
       p.birthday, COALESCE(p.sex, '') AS sex
FROM ovst v
JOIN patient p ON p.hn = v.hn
WHERE v.vstdate = ?
  AND TRIM(CAST(v.queue AS CHAR)) IN (?, ?, ?)
ORDER BY p.fname, p.lname
LIMIT ?";

/// Today's OPD drug orders for a patient.
pub const TODAY_ORDERS_BY_HN: &str = "\
SELECT o.vn, o.icode,
       COALESCE(CAST(o.item_no AS SIGNED), 0) AS item_no,
       CAST(o.qty AS CHAR) AS qty,
       COALESCE(d.name, '') AS drug_name,
       COALESCE(d.strength, '') AS strength,
       COALESCE(d.units, '') AS units,
       COALESCE(du.name1, '') AS du_name1,
       COALESCE(du.name2, '') AS du_name2,
       COALESCE(du.name3, '') AS du_name3,
       COALESCE(su.name1, '') AS sp_name1,
       COALESCE(su.name2, '') AS sp_name2,
       COALESCE(su.name3, '') AS sp_name3,
       COALESCE(doc.name, '') AS doctor_name,
       COALESCE(o.rxdate, o.vstdate) AS order_date,
       CAST(COALESCE(o.rxtime, '') AS CHAR) AS order_time
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
LEFT JOIN drugusage du ON du.drugusage = o.drugusage
LEFT JOIN sp_use su ON su.sp_use = o.sp_use
LEFT JOIN doctor doc ON doc.code = o.doctor
WHERE o.hn = ? AND o.vstdate = ?
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
ORDER BY o.vn, o.item_no, o.icode";

/// Orders without the optional sig/doctor lookups, for sites missing
/// `drugusage`, `sp_use`, or `doctor`.
pub const TODAY_ORDERS_NO_LOOKUPS: &str = "\
SELECT o.vn, o.icode,
       COALESCE(CAST(o.item_no AS SIGNED), 0) AS item_no,
       CAST(o.qty AS CHAR) AS qty,
       COALESCE(d.name, '') AS drug_name,
       COALESCE(d.strength, '') AS strength,
       COALESCE(d.units, '') AS units,
       '' AS du_name1,
       '' AS du_name2,
       '' AS du_name3,
       '' AS sp_name1,
       '' AS sp_name2,
       '' AS sp_name3,
       '' AS doctor_name,
       COALESCE(o.rxdate, o.vstdate) AS order_date,
       CAST(COALESCE(o.rxtime, '') AS CHAR) AS order_time
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.hn = ? AND o.vstdate = ?
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
ORDER BY o.vn, o.item_no, o.icode";

/// Orders without strength/units and optional lookups - the most degraded
/// shape. Dose review will report "cannot estimate" for these rows.
pub const TODAY_ORDERS_MINIMAL: &str = "\
SELECT o.vn, o.icode,
       COALESCE(CAST(o.item_no AS SIGNED), 0) AS item_no,
       CAST(o.qty AS CHAR) AS qty,
       COALESCE(d.name, '') AS drug_name,
       '' AS strength,
       '' AS units,
       '' AS du_name1,
       '' AS du_name2,
       '' AS du_name3,
       '' AS sp_name1,
       '' AS sp_name2,
       '' AS sp_name3,
       '' AS doctor_name,
       COALESCE(o.rxdate, o.vstdate) AS order_date,
       CAST(COALESCE(o.rxtime, '') AS CHAR) AS order_time
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.hn = ? AND o.vstdate = ?
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
ORDER BY o.vn, o.item_no, o.icode";

/// Allergy / ADR records with the structured columns found on Sra Bos.
///
/// `icode` is populated on 0/1643 rows at the surveyed site, but
/// `seriousness_id`, `no_alert`, and `force_no_order` carry useful signal and
/// are preserved. The connector falls back to [`ALLERGIES_BY_HN_MINIMAL`]
/// when these site-dependent columns are missing.
pub const ALLERGIES_BY_HN: &str = "\
SELECT COALESCE(agent, '') AS agent,
       COALESCE(symptom, '') AS symptom,
       COALESCE(reporter, '') AS reporter,
       report_date,
       COALESCE(note, '') AS note,
       CAST(COALESCE(icode, '') AS CHAR) AS icode,
       CAST(COALESCE(seriousness_id, '') AS CHAR) AS seriousness_id,
       CAST(COALESCE(no_alert, '') AS CHAR) AS no_alert,
       CAST(COALESCE(force_no_order, '') AS CHAR) AS force_no_order
FROM opd_allergy
WHERE hn = ?
ORDER BY report_date DESC
LIMIT 100";

/// Allergy fallback for sites without the optional structured columns.
pub const ALLERGIES_BY_HN_MINIMAL: &str = "\
SELECT COALESCE(agent, '') AS agent,
       COALESCE(symptom, '') AS symptom,
       COALESCE(reporter, '') AS reporter,
       report_date,
       COALESCE(note, '') AS note,
       '' AS icode,
       '' AS seriousness_id,
       '' AS no_alert,
       '' AS force_no_order
FROM opd_allergy
WHERE hn = ?
ORDER BY report_date DESC
LIMIT 100";

/// Recent ICD-10 diagnoses, newest first.
pub const DIAGNOSES_BY_HN: &str = "\
SELECT d.vn, d.icd10,
       CAST(COALESCE(d.diagtype, '') AS CHAR) AS diag_type,
       v.vstdate
FROM ovstdiag d
JOIN ovst v ON v.vn = d.vn
WHERE d.hn = ?
  AND v.vstdate >= ?
  AND d.icd10 IS NOT NULL
  AND TRIM(d.icd10) <> ''
ORDER BY v.vstdate DESC, d.icd10
LIMIT 200";

/// Recent lab results with the item name and unit.
pub const LABS_BY_HN: &str = "\
SELECT DATE(lh.order_date) AS order_date,
       COALESCE(li.lab_items_name, '') AS item_name,
       COALESCE(li.lab_items_unit, '') AS unit,
       COALESCE(lo.lab_order_result, '') AS result
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
JOIN lab_items li ON li.lab_items_code = lo.lab_items_code
WHERE lh.hn = ?
  AND lh.order_date >= ?
  AND lh.confirm_report = 'Y'
  AND lo.confirm = 'Y'
  AND lo.lab_order_result IS NOT NULL
  AND lo.lab_order_result <> ''
ORDER BY lh.order_date DESC
LIMIT 1000";

/// Labs without `lab_items_unit`, which some older builds lack.
pub const LABS_BY_HN_NO_UNIT: &str = "\
SELECT DATE(lh.order_date) AS order_date,
       COALESCE(li.lab_items_name, '') AS item_name,
       '' AS unit,
       COALESCE(lo.lab_order_result, '') AS result
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
JOIN lab_items li ON li.lab_items_code = lo.lab_items_code
WHERE lh.hn = ?
  AND lh.order_date >= ?
  AND lh.confirm_report = 'Y'
  AND lo.confirm = 'Y'
  AND lo.lab_order_result IS NOT NULL
  AND lo.lab_order_result <> ''
ORDER BY lh.order_date DESC
LIMIT 1000";

/// Most conservative lab query: no unit and no `lab_order.confirm` filter.
pub const LABS_BY_HN_MINIMAL: &str = "\
SELECT DATE(lh.order_date) AS order_date,
       COALESCE(li.lab_items_name, '') AS item_name,
       '' AS unit,
       COALESCE(lo.lab_order_result, '') AS result
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
JOIN lab_items li ON li.lab_items_code = lo.lab_items_code
WHERE lh.hn = ?
  AND lh.order_date >= ?
  AND lh.confirm_report = 'Y'
  AND lo.lab_order_result IS NOT NULL
  AND lo.lab_order_result <> ''
ORDER BY lh.order_date DESC
LIMIT 1000";

/// Latest screening vitals (`opdscreen`). HOSxP writes 0 for unmeasured
/// values, so the client ignores non-positive numbers.
pub const LATEST_VITALS_BY_HN: &str = "\
SELECT vstdate,
       CAST(COALESCE(bw, 0) AS CHAR) AS weight_kg,
       CAST(COALESCE(height, 0) AS CHAR) AS height_cm,
       CAST(COALESCE(bmi, 0) AS CHAR) AS bmi
FROM opdscreen
WHERE hn = ?
  AND vstdate >= ?
  AND (IFNULL(bw, 0) > 0 OR IFNULL(height, 0) > 0)
ORDER BY vstdate DESC
LIMIT 10";

/// Latest past medical history free text (cumulative; no cutoff).
pub const LATEST_PMH_BY_HN: &str = "\
SELECT vstdate, pmh
FROM opdscreen
WHERE hn = ?
  AND pmh IS NOT NULL
  AND TRIM(pmh) <> ''
ORDER BY vstdate DESC
LIMIT 1";

/// Drug master search for a future dictionary-mapping screen.
pub const DRUG_SEARCH_SQL: &str = "\
SELECT icode, COALESCE(name, '') AS name,
       COALESCE(strength, '') AS strength,
       COALESCE(units, '') AS units
FROM drugitems
WHERE icode LIKE ?
   OR name LIKE ?
ORDER BY name
LIMIT ?";

/// Escape `\`, `%`, and `_` in an operator term before wrapping it in `%...%`.
pub fn like_pattern(input: &str) -> String {
    format!("%{}%", escape_like(input))
}

/// Prefix pattern for index-friendly name search.
pub fn prefix_pattern(input: &str) -> String {
    format!("{}%", escape_like(input))
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readonly::assert_read_only;

    #[test]
    fn every_constant_is_read_only() {
        for sql in [
            PING_SQL,
            FETCH_OFFICER_SQL,
            PATIENT_SEARCH_BY_HN,
            PATIENT_SEARCH_BY_CID,
            PATIENT_SEARCH_NAME_PREFIX,
            PATIENT_SEARCH_NAME_CONTAINS,
            PATIENT_BY_HN_SQL,
            TODAY_VISITS_BY_HN,
            TODAY_VISITS_BY_HN_LEGACY_QUEUE,
            TODAY_VISITS_BY_HN_NO_QUEUE,
            TODAY_VISITS_BY_QUEUE,
            TODAY_VISITS_BY_QUEUE_LEGACY,
            TODAY_ORDERS_BY_HN,
            TODAY_ORDERS_NO_LOOKUPS,
            TODAY_ORDERS_MINIMAL,
            ALLERGIES_BY_HN,
            ALLERGIES_BY_HN_MINIMAL,
            DIAGNOSES_BY_HN,
            LABS_BY_HN,
            LABS_BY_HN_NO_UNIT,
            LABS_BY_HN_MINIMAL,
            LATEST_VITALS_BY_HN,
            LATEST_PMH_BY_HN,
            DRUG_SEARCH_SQL,
        ] {
            assert!(assert_read_only(sql).is_ok(), "guard must accept: {sql}");
        }
    }

    #[test]
    fn officer_query_never_reads_plaintext_password_or_opduser() {
        assert!(FETCH_OFFICER_SQL.contains("officer_login_password_md5"));
        assert!(!FETCH_OFFICER_SQL.contains("officer_login_password "));
        assert!(!FETCH_OFFICER_SQL.contains("opduser"));
        assert!(FETCH_OFFICER_SQL.contains("officer_active"));
        assert!(FETCH_OFFICER_SQL.contains("officer_authentication_type_id"));
        assert!(
            FETCH_OFFICER_SQL.contains("COALESCE(o.officer_authentication_type_id, 1)"),
            "NULL auth type must coalesce to local DB (Sra Bos has 104/104 NULL)"
        );
    }

    #[test]
    fn order_query_selects_every_sig_name_field() {
        for field in [
            "du.name1", "du.name2", "du.name3", "su.name1", "su.name2", "su.name3",
        ] {
            assert!(
                TODAY_ORDERS_BY_HN.contains(field),
                "today orders must keep {field} for complete sig display"
            );
        }
        assert!(!TODAY_ORDERS_BY_HN.contains("AS sig_text"));
    }

    #[test]
    fn order_queries_filter_non_drug_and_ipd_lines() {
        for sql in [
            TODAY_ORDERS_BY_HN,
            TODAY_ORDERS_NO_LOOKUPS,
            TODAY_ORDERS_MINIMAL,
        ] {
            assert!(sql.contains("JOIN drugitems"));
            assert!(sql.contains("o.qty > 0"));
            assert!(sql.contains("(o.an IS NULL OR o.an = '')"));
            assert!(sql.contains("o.sub_type = '1'"));
            assert!(sql.contains("IFNULL(o.item_type, '') <> 'A'"));
        }
    }

    #[test]
    fn queue_search_has_five_placeholders() {
        assert_eq!(TODAY_VISITS_BY_QUEUE.matches('?').count(), 5);
        assert!(TODAY_VISITS_BY_QUEUE.contains("v.oqueue"));
        assert!(TODAY_VISITS_BY_QUEUE_LEGACY.contains("v.queue"));
        assert!(TODAY_VISITS_BY_HN.contains("v.oqueue"));
    }

    #[test]
    fn like_escaping_is_explicit() {
        assert_eq!(like_pattern("50%_off\\"), "%50\\%\\_off\\\\%");
        assert_eq!(prefix_pattern("สมชาย"), "สมชาย%");
    }
}
