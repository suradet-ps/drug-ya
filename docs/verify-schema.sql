-- Drug-Ya - read-only HOSxP schema verification
-- Run on a test/staging instance, or with a SELECT-only account.
-- Replace the two variables before running. No INSERT/UPDATE/DELETE/DDL.
--
-- MySQL/MariaDB CLI suggested:
--   mysql --default-character-set=tis620 -u drugya_ro -p hos < verify-schema.sql
--
-- The app checks these objects; mark the result in docs/database.md.

SET @test_hn = '0000000';
SET @test_login = 'test_officer';

-- ===========================================================================
-- 0. Server / schema
-- ===========================================================================
SELECT VERSION() AS server_version, DATABASE() AS database_name;
SELECT * FROM sys_var WHERE sys_name = 'current_db_structure_version';

-- ===========================================================================
-- 1. Core table presence
-- ===========================================================================
SELECT TABLE_NAME
FROM information_schema.TABLES
WHERE TABLE_SCHEMA = DATABASE()
  AND TABLE_NAME IN (
    'patient', 'ovst', 'kskdepartment', 'doctor', 'opitemrece', 'drugitems',
    'drugusage', 'sp_use', 'opd_allergy', 'ovstdiag', 'lab_head', 'lab_order',
    'lab_items', 'opdscreen', 'officer'
  )
ORDER BY TABLE_NAME;

-- ===========================================================================
-- 2. Columns used by Drug-Ya
-- ===========================================================================
SHOW COLUMNS FROM patient;
SHOW COLUMNS FROM ovst;
SHOW COLUMNS FROM kskdepartment;
SHOW COLUMNS FROM doctor;
SHOW COLUMNS FROM opitemrece;
SHOW COLUMNS FROM drugitems;
SHOW COLUMNS FROM drugusage;
SHOW COLUMNS FROM sp_use;
SHOW COLUMNS FROM opd_allergy;
SHOW COLUMNS FROM ovstdiag;
SHOW COLUMNS FROM lab_head;
SHOW COLUMNS FROM lab_order;
SHOW COLUMNS FROM lab_items;
SHOW COLUMNS FROM opdscreen;
SHOW COLUMNS FROM officer;

-- Queue support:
-- Sra Bos uses ovst.oqueue (int); there is no ovst.queue column.
-- Some other builds use ovst.queue. The connector tries oqueue, then queue.
SHOW COLUMNS FROM ovst LIKE 'oqueue';
SHOW COLUMNS FROM ovst LIKE 'queue';
SHOW COLUMNS FROM ovst LIKE 'rx_queue';
SHOW COLUMNS FROM ovst LIKE 'main_dep_queue';

-- ===========================================================================
-- 3. Queue format today
-- ===========================================================================
SELECT vstdate, vn, hn, oqueue, main_dep, vsttime
FROM ovst
WHERE vstdate = CURDATE()
ORDER BY vsttime
LIMIT 30;

-- ===========================================================================
-- 4. One patient's today visits
-- ===========================================================================
SELECT v.vn, v.hn, v.vstdate, v.vsttime,
       v.oqueue, v.main_dep, k.department,
       v.doctor, d.name AS doctor_name
FROM ovst v
LEFT JOIN kskdepartment k ON k.depcode = v.main_dep
LEFT JOIN doctor d ON d.code = v.doctor
WHERE v.hn = @test_hn
  AND v.vstdate = CURDATE()
ORDER BY v.vsttime;

-- ===========================================================================
-- 5. Today's drug orders (the exact scope the app uses)
--    Non-drug lines are excluded; OPD only; qty > 0.
-- ===========================================================================
SELECT o.vn, o.icode, o.item_no, o.qty,
       o.drugusage, o.sp_use, o.doctor,
       d.name AS drug_name, d.strength, d.units,
       du.name1 AS drugusage_name1, du.name2 AS drugusage_name2, du.name3 AS drugusage_name3,
       su.name1 AS sp_use_name1, su.name2 AS sp_use_name2, su.name3 AS sp_use_name3,
       doc.name AS doctor_name,
       o.rxdate, o.rxtime, o.vstdate, o.an, o.sub_type, o.item_type
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
LEFT JOIN drugusage du ON du.drugusage = o.drugusage
LEFT JOIN sp_use su ON su.sp_use = o.sp_use
LEFT JOIN doctor doc ON doc.code = o.doctor
WHERE o.hn = @test_hn
  AND o.vstdate = CURDATE()
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
ORDER BY o.vn, o.item_no, o.icode;

-- What item_type values exist? Sra Bos has only NULL/''/'P'; the app's
-- item_type <> 'A' filter is harmless insurance.
SELECT item_type, COUNT(*) AS n
FROM opitemrece
WHERE vstdate = CURDATE()
GROUP BY item_type;

-- Are order times populated? (Should be, for the order-based scope.)
SELECT SUM(rxdate IS NULL OR rxdate = '0000-00-00') AS missing_rxdate,
       SUM(rxtime IS NULL OR rxtime = '') AS missing_rxtime,
       COUNT(*) AS total_lines
FROM opitemrece
WHERE hn = @test_hn
  AND vstdate = CURDATE()
  AND qty > 0
  AND (an IS NULL OR an = '')
  AND sub_type = '1'
  AND IFNULL(item_type, '') <> 'A';

-- ===========================================================================
-- 6. Allergies / ADR
-- ===========================================================================
SELECT hn, agent, symptom, reporter, report_date, note,
       icode, seriousness_id, no_alert, force_no_order
FROM opd_allergy
WHERE hn = @test_hn
ORDER BY report_date DESC;

-- Structured-column coverage at this site
SELECT COUNT(*) AS total,
       SUM(icode IS NOT NULL AND icode <> '') AS with_icode,
       SUM(seriousness_id IS NOT NULL AND seriousness_id <> '') AS with_seriousness,
       SUM(no_alert = 'Y') AS no_alert_rows,
       SUM(force_no_order = 'Y') AS force_no_order_rows
FROM opd_allergy;

-- Check whether agents are class names, trade names, or free text.
SELECT agent, symptom, COUNT(*) AS n
FROM opd_allergy
GROUP BY agent, symptom
ORDER BY n DESC
LIMIT 20;

-- ===========================================================================
-- 7. Diagnoses (last 365 days)
-- ===========================================================================
SELECT d.vn, d.icd10, d.diagtype, v.vstdate
FROM ovstdiag d
JOIN ovst v ON v.vn = d.vn
WHERE d.hn = @test_hn
  AND v.vstdate >= DATE_SUB(CURDATE(), INTERVAL 365 DAY)
ORDER BY v.vstdate DESC, d.icd10;

-- ===========================================================================
-- 8. Labs - list the site's names and units first
-- ===========================================================================
SELECT DISTINCT lab_items_code, lab_items_name, lab_items_unit
FROM lab_items
WHERE lab_items_name LIKE '%creat%'
   OR lab_items_name LIKE '%ไต%'
   OR lab_items_name LIKE '%potassium%'
   OR lab_items_name LIKE '%โพแทสเซียม%'
   OR lab_items_name LIKE '%INR%'
   OR lab_items_name LIKE '%ALT%'
   OR lab_items_name LIKE '%SGPT%'
   OR lab_items_name LIKE '%SGOT%'
   OR lab_items_name LIKE '%HbA1c%'
ORDER BY lab_items_name;

-- Recent results for one patient
SELECT lh.order_date,
       li.lab_items_code, li.lab_items_name, li.lab_items_unit,
       lo.lab_order_result, lh.confirm_report, lo.confirm
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
JOIN lab_items li ON li.lab_items_code = lo.lab_items_code
WHERE lh.hn = @test_hn
  AND lh.order_date >= DATE_SUB(CURDATE(), INTERVAL 365 DAY)
ORDER BY lh.order_date DESC
LIMIT 100;

-- Validity filter counts (for one recent day)
SELECT lh.confirm_report, lo.confirm, COUNT(*) AS n
FROM lab_head lh
JOIN lab_order lo ON lo.lab_order_number = lh.lab_order_number
WHERE lh.order_date >= DATE_SUB(CURDATE(), INTERVAL 30 DAY)
GROUP BY lh.confirm_report, lo.confirm;

-- ===========================================================================
-- 9. Vitals / PMH
-- ===========================================================================
SELECT vstdate, bw, height, bmi, pmh
FROM opdscreen
WHERE hn = @test_hn
ORDER BY vstdate DESC
LIMIT 10;

-- ===========================================================================
-- 10. Officer login
-- ===========================================================================
SELECT officer_id, officer_login_name, officer_active,
       officer_authentication_type_id, officer_doctor_code,
       officer_pname, officer_fname, officer_lname
FROM officer
WHERE officer_login_name = @test_login;

-- Sra Bos: auth type is NULL on all 104 accounts. The app coalesces NULL to 1.
SELECT SUM(officer_authentication_type_id IS NULL) AS null_auth_type,
       COUNT(*) AS total
FROM officer;

-- ===========================================================================
-- 11. Indexes / performance
-- ===========================================================================
SHOW INDEX FROM patient;
SHOW INDEX FROM ovst;
SHOW INDEX FROM opitemrece;
SHOW INDEX FROM lab_head;

EXPLAIN
SELECT o.vn, o.icode, o.qty
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.hn = @test_hn
  AND o.vstdate = CURDATE()
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A';

-- ===========================================================================
-- 12. Data quality counts (record, do not commit patient data)
-- ===========================================================================
SELECT COUNT(*) AS missing_sig
FROM opitemrece o
WHERE o.vstdate = CURDATE()
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
  AND (o.drugusage IS NULL OR o.drugusage = '');

SELECT COUNT(*) AS missing_strength
FROM drugitems
WHERE strength IS NULL OR TRIM(strength) = '';

SELECT COUNT(*) AS missing_units
FROM drugitems
WHERE units IS NULL OR TRIM(units) = '';

SELECT COUNT(*) AS allergies_total FROM opd_allergy;
SELECT COUNT(*) AS diagnoses_total FROM ovstdiag;

-- ===========================================================================
-- 13. Read-only proof
-- ===========================================================================
-- Run this separately; it must fail on a SELECT-only account.
-- UPDATE opitemrece SET qty = qty WHERE 1 = 0;
