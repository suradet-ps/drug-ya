//! The read-only HOSxP client.
//!
//! Every query goes through [`crate::readonly::assert_read_only`] before it is
//! executed. Optional tables/columns degrade with a Thai-facing warning instead
//! of failing the whole review; a missing core table is a hard error.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::NaiveDate;
use drug_ya_core::dictionary::resolve_drug;
use drug_ya_core::dose::{estimate_dose, format_sig_summary};
use drug_ya_core::model::{
    AllergyRecord, ClinicalContext, DiagnosisRecord, LabFlag, LabKind, LabValue, OrderLine,
    Patient, PregnancyStatus, ReviewDataset, Sex, TodayVisit,
};
use drug_ya_core::normalize_date;
use drug_ya_core::query_kind::{ParsedSearch, SearchKind};
use drug_ya_core::renal::{ckd_epi_2021, cockcroft_gault};
use drug_ya_core::sig::compose_sig;
use drug_ya_core::thai;
use secrecy::ExposeSecret;
use sqlx::FromRow;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlSslMode};
use tracing::debug;

use crate::config::HosxpConfig;
use crate::error::{Error, Result};
use crate::queries;
use crate::readonly::{READ_ONLY_SESSION_SQL, assert_read_only};

const MAX_CONNECTIONS: u32 = 5;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(8);
const STATEMENT_TIMEOUT_SQL: &str = "SET SESSION max_execution_time = 5000";

/// Inputs for one patient review.
#[derive(Debug, Clone)]
pub struct ReviewRequest {
    /// Hospital number.
    pub hn: String,
    /// Review date (normally today).
    pub today: NaiveDate,
    /// Session-only pregnancy override from the pharmacist.
    pub pregnancy_override: Option<PregnancyStatus>,
    /// Session-only weight override from the pharmacist.
    pub weight_override: Option<f64>,
}

/// Repository for a single HOSxP site. Cheap to clone (pool handle).
#[derive(Clone)]
pub struct HosxpClient {
    pool: MySqlPool,
    #[allow(dead_code)]
    config: HosxpConfig,
}

impl HosxpClient {
    /// Open a read-only pool.
    pub async fn connect(config: HosxpConfig) -> Result<Self> {
        let options = MySqlConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .database(&config.database)
            .username(&config.user)
            .password(config.password.expose_secret())
            // HOSxP data is commonly stored as TIS-620. Asking MySQL for a
            // UTF-8 connection makes the server convert column data for the
            // Rust String decoder; it does not change the stored charset.
            .charset("utf8mb4")
            .ssl_mode(MySqlSslMode::Preferred);

        let pool = MySqlPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query(READ_ONLY_SESSION_SQL)
                        .execute(&mut *conn)
                        .await?;
                    let _ = sqlx::query(STATEMENT_TIMEOUT_SQL).execute(&mut *conn).await;
                    Ok(())
                })
            })
            .connect_with(options)
            .await
            .map_err(|source| Error::Connect {
                host: config.host.clone(),
                port: config.port,
                database: config.database.clone(),
                source,
            })?;

        debug!(
            "connected to HOSxP at {}:{} (read-only)",
            config.host, config.port
        );
        Ok(Self { pool, config })
    }

    /// Close all pooled connections.
    pub async fn disconnect(&self) {
        self.pool.close().await;
    }

    /// Verify the connection is alive.
    pub async fn ping(&self) -> Result<()> {
        self.fetch_rows::<PingRow>(queries::PING_SQL, &[]).await?;
        Ok(())
    }

    /// Search patients by the parsed search term.
    pub async fn search_patients(&self, parsed: &ParsedSearch) -> Result<Vec<Patient>> {
        if parsed.term.trim().is_empty() {
            return Ok(Vec::new());
        }
        let today = chrono::Local::now().date_naive();
        let limit = drug_ya_core::SEARCH_LIMIT as i64;
        let rows: Vec<PatientRow> = match parsed.kind {
            SearchKind::Hn | SearchKind::Auto => {
                self.fetch_rows(
                    queries::PATIENT_SEARCH_BY_HN,
                    &[Param::Str(parsed.term.clone()), Param::Int(limit)],
                )
                .await?
            }
            SearchKind::Cid => {
                self.fetch_rows(
                    queries::PATIENT_SEARCH_BY_CID,
                    &[Param::Str(parsed.term.clone()), Param::Int(limit)],
                )
                .await?
            }
            SearchKind::Name => {
                let prefix = queries::prefix_pattern(&parsed.term);
                let rows: Vec<PatientRow> = self
                    .fetch_rows(
                        queries::PATIENT_SEARCH_NAME_PREFIX,
                        &[
                            Param::Str(prefix.clone()),
                            Param::Str(prefix.clone()),
                            Param::Str(prefix),
                            Param::Int(limit),
                        ],
                    )
                    .await?;
                if rows.is_empty() {
                    let pattern = queries::like_pattern(&parsed.term);
                    self.fetch_rows(
                        queries::PATIENT_SEARCH_NAME_CONTAINS,
                        &[
                            Param::Str(pattern.clone()),
                            Param::Str(pattern.clone()),
                            Param::Str(pattern),
                            Param::Int(limit),
                        ],
                    )
                    .await?
                } else {
                    rows
                }
            }
            SearchKind::Queue => {
                let raw = parsed.term.trim().to_string();
                let params = || {
                    [
                        Param::Date(today),
                        Param::Str(raw.clone()),
                        Param::Str(format!("Q{raw}")),
                        Param::Str(format!("QH{raw}")),
                        Param::Int(limit),
                    ]
                };
                match self
                    .fetch_rows::<PatientRow>(queries::TODAY_VISITS_BY_QUEUE, &params())
                    .await
                {
                    Ok(rows) => rows,
                    Err(Error::Database(source)) if is_schema_variation(&source) => {
                        match self
                            .fetch_rows::<PatientRow>(
                                queries::TODAY_VISITS_BY_QUEUE_LEGACY,
                                &params(),
                            )
                            .await
                        {
                            Ok(rows) => rows,
                            Err(Error::Database(source)) if is_schema_variation(&source) => {
                                return Err(Error::UnsupportedSchema(
                                    "neither ovst.oqueue nor ovst.queue is available".into(),
                                ));
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        };
        Ok(rows
            .into_iter()
            .map(|row| row.into_patient(today))
            .collect())
    }

    /// Load one patient's review dataset: today's visits/orders, allergies,
    /// diagnoses, labs, vitals, and the derived clinical context.
    pub async fn load_review_dataset(&self, request: &ReviewRequest) -> Result<ReviewDataset> {
        let patient = self.load_patient(&request.hn).await?;
        let mut warnings = Vec::new();

        let visits = self
            .load_today_visits(&request.hn, request.today, &mut warnings)
            .await?;
        let orders = self
            .load_today_orders(&request.hn, request.today, &mut warnings)
            .await?;

        let (allergies, allergy_data_available) =
            self.load_allergies(&request.hn, &mut warnings).await?;
        let (diagnoses, diagnosis_data_available) = self
            .load_diagnoses(&request.hn, request.today, &mut warnings)
            .await?;
        let (labs, lab_data_available) = self
            .load_labs(&request.hn, request.today, &mut warnings)
            .await?;
        let (vitals, vitals_data_available) = self
            .load_vitals(&request.hn, request.today, &mut warnings)
            .await?;
        let pmh = self.load_pmh(&request.hn, &mut warnings).await?;

        let clinical = build_clinical(
            &patient,
            request,
            allergies,
            allergy_data_available,
            diagnoses,
            diagnosis_data_available,
            labs,
            lab_data_available,
            vitals,
            vitals_data_available,
            pmh,
            &mut warnings,
        );

        dedupe_warnings(&mut warnings);
        Ok(ReviewDataset {
            patient,
            visits,
            orders,
            clinical,
            warnings,
            review_date: request.today,
        })
    }

    /// Read one local HOSxP officer for password verification.
    pub async fn fetch_officer(
        &self,
        login_name: &str,
    ) -> Result<Option<drug_ya_core::OfficerAccount>> {
        let rows: Vec<OfficerRow> = self
            .fetch_rows(
                queries::FETCH_OFFICER_SQL,
                &[Param::Str(login_name.to_string())],
            )
            .await?;
        Ok(rows
            .into_iter()
            .next()
            .map(|row| drug_ya_core::OfficerAccount {
                officer_id: row.officer_id,
                login_name: row.login_name,
                password_md5: row.password_md5,
                active: row.active,
                auth_type_id: row.auth_type_id,
                doctor_code: row.doctor_code,
                display_name: row.display_name,
            }))
    }

    /// Fetch one patient by HN.
    pub async fn load_patient(&self, hn: &str) -> Result<Patient> {
        let rows: Vec<PatientRow> = self
            .fetch_rows(queries::PATIENT_BY_HN_SQL, &[Param::Str(hn.to_string())])
            .await?;
        rows.into_iter()
            .next()
            .map(|row| row.into_patient(chrono::Local::now().date_naive()))
            .ok_or_else(|| Error::NotFound("ไม่พบผู้ป่วยตาม HN ที่ระบุ".into()))
    }

    async fn load_today_visits(
        &self,
        hn: &str,
        today: NaiveDate,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<TodayVisit>> {
        let params = || [Param::Str(hn.to_string()), Param::Date(today)];
        let rows: Vec<VisitRow> = match self
            .fetch_rows(queries::TODAY_VISITS_BY_HN, &params())
            .await
        {
            Ok(rows) => rows,
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                match self
                    .fetch_rows(queries::TODAY_VISITS_BY_HN_LEGACY_QUEUE, &params())
                    .await
                {
                    Ok(rows) => {
                        warnings.push("ใช้คอลัมน์คิวแบบเดิม (ovst.queue)".into());
                        rows
                    }
                    Err(Error::Database(source)) if is_schema_variation(&source) => {
                        warnings.push("ไม่พบคอลัมน์คิว (oqueue/queue) - ค้นด้วย HN/CID/ชื่อได้".into());
                        self.fetch_rows(queries::TODAY_VISITS_BY_HN_NO_QUEUE, &params())
                            .await?
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        };
        Ok(rows
            .into_iter()
            .map(|row| TodayVisit {
                vn: row.vn,
                date: normalize_date(row.vstdate),
                time: thai::format_time(&row.vsttime),
                queue: none_if_empty(row.queue),
                department_code: row.department_code,
                department_name: if row.department_name.trim().is_empty() {
                    String::new()
                } else {
                    row.department_name
                },
                doctor_code: row.doctor_code,
                doctor_name: row.doctor_name,
                order_count: row.order_count.max(0) as usize,
            })
            .collect())
    }

    async fn load_today_orders(
        &self,
        hn: &str,
        today: NaiveDate,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<OrderLine>> {
        let (rows, degraded): (Vec<OrderRow>, bool) = match self
            .fetch_rows(
                queries::TODAY_ORDERS_BY_HN,
                &[Param::Str(hn.to_string()), Param::Date(today)],
            )
            .await
        {
            Ok(rows) => (rows, false),
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                match self
                    .fetch_rows(
                        queries::TODAY_ORDERS_NO_LOOKUPS,
                        &[Param::Str(hn.to_string()), Param::Date(today)],
                    )
                    .await
                {
                    Ok(rows) => (rows, true),
                    Err(Error::Database(source)) if is_schema_variation(&source) => {
                        let rows = self
                            .fetch_rows(
                                queries::TODAY_ORDERS_MINIMAL,
                                &[Param::Str(hn.to_string()), Param::Date(today)],
                            )
                            .await?;
                        (rows, true)
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        };
        if degraded {
            warnings.push("ตาราง/คอลัมน์ sig หรือ doctor ไม่พร้อมใช้งาน - บางรายการจะไม่มีวิธีใช้".into());
        }

        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut orders = Vec::with_capacity(rows.len());
        for row in rows {
            let qty = parse_number(&row.qty).unwrap_or(0.0);
            if qty <= 0.0 {
                continue;
            }
            let resolution = resolve_drug(&row.drug_name);
            let sig = compose_sig(&[
                row.du_name1,
                row.du_name2,
                row.du_name3,
                row.sp_name1,
                row.sp_name2,
                row.sp_name3,
            ]);
            let dose = estimate_dose(&row.strength, &row.units, &sig, qty);
            let sig_summary = format_sig_summary(&dose);
            let base_id = format!("{}:{}:{}", row.vn, row.item_no, row.icode);
            let count = seen.entry(base_id.clone()).or_insert(0);
            *count += 1;
            let line_id = if *count == 1 {
                base_id
            } else {
                format!("{base_id}#{}", *count)
            };
            orders.push(OrderLine {
                line_id,
                vn: row.vn,
                icode: row.icode,
                drug_name: row.drug_name,
                strength: row.strength,
                units: row.units,
                sig,
                sig_summary,
                qty,
                item_no: row.item_no,
                doctor_name: row.doctor_name,
                ordered_on: row.order_date.map(normalize_date).or(Some(today)),
                ordered_at: thai::format_time(&row.order_time),
                drug_key: resolution.key,
                generic_name: resolution.display_name,
                classes: resolution.classes,
                high_alert: resolution.high_alert,
                known: resolution.known,
                daily_dose_mg: dose.mg_per_day,
                days_supply: dose.days_supply,
                dose_note: dose.note,
            });
        }
        Ok(orders)
    }

    async fn load_allergies(
        &self,
        hn: &str,
        warnings: &mut Vec<String>,
    ) -> Result<(Vec<AllergyRecord>, bool)> {
        let rows: Vec<AllergyRow> = match self
            .fetch_rows::<AllergyRow>(queries::ALLERGIES_BY_HN, &[Param::Str(hn.to_string())])
            .await
        {
            Ok(rows) => rows,
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                warnings.push("คอลัมน์เสริมของ opd_allergy ไม่พร้อมใช้งาน - ใช้ข้อมูลแพ้ยาพื้นฐาน".into());
                self.fetch_rows::<AllergyRow>(
                    queries::ALLERGIES_BY_HN_MINIMAL,
                    &[Param::Str(hn.to_string())],
                )
                .await?
            }
            Err(error) => return Err(error),
        };
        Ok((
            rows.into_iter()
                .map(|row| AllergyRecord {
                    agent: if row.agent.trim().is_empty() {
                        "ไม่ระบุ".into()
                    } else {
                        row.agent.trim().to_string()
                    },
                    symptom: row.symptom,
                    reporter: row.reporter,
                    report_date: row.report_date.map(normalize_date),
                    note: row.note,
                    icode: row.icode.trim().to_string(),
                    seriousness_id: row.seriousness_id.trim().to_string(),
                    no_alert: row.no_alert.eq_ignore_ascii_case("Y"),
                    force_no_order: row.force_no_order.eq_ignore_ascii_case("Y"),
                })
                .collect(),
            true,
        ))
    }

    async fn load_diagnoses(
        &self,
        hn: &str,
        today: NaiveDate,
        warnings: &mut Vec<String>,
    ) -> Result<(Vec<DiagnosisRecord>, bool)> {
        let cutoff = today - chrono::Days::new(365);
        match self
            .fetch_rows::<DiagnosisRow>(
                queries::DIAGNOSES_BY_HN,
                &[Param::Str(hn.to_string()), Param::Date(cutoff)],
            )
            .await
        {
            Ok(rows) => {
                let mut seen = BTreeSet::new();
                let mut diagnoses = Vec::new();
                for row in rows {
                    let code = row.icd10.trim().to_ascii_uppercase();
                    if code.is_empty() || !seen.insert(code.clone()) {
                        continue;
                    }
                    diagnoses.push(DiagnosisRecord {
                        label: condition_label(&code).map(str::to_string),
                        icd10: code,
                        date: normalize_date(row.vstdate),
                        diag_type: row.diag_type,
                    });
                }
                Ok((diagnoses, true))
            }
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                warnings.push("อ่านรหัสวินิจฉัย (ovstdiag) ไม่ได้".into());
                Ok((Vec::new(), false))
            }
            Err(error) => Err(error),
        }
    }

    async fn load_labs(
        &self,
        hn: &str,
        today: NaiveDate,
        warnings: &mut Vec<String>,
    ) -> Result<(BTreeMap<LabKind, LabValue>, bool)> {
        let cutoff = today - chrono::Days::new(365);
        let mut rows: Option<Vec<LabRow>> = None;
        for sql in [
            queries::LABS_BY_HN,
            queries::LABS_BY_HN_NO_UNIT,
            queries::LABS_BY_HN_MINIMAL,
        ] {
            match self
                .fetch_rows(sql, &[Param::Str(hn.to_string()), Param::Date(cutoff)])
                .await
            {
                Ok(found) => {
                    rows = Some(found);
                    break;
                }
                Err(Error::Database(source)) if is_schema_variation(&source) => continue,
                Err(error) => return Err(error),
            }
        }
        let Some(rows) = rows else {
            warnings.push("อ่านผล lab ไม่ได้ - กฎไตและ lab จะถูกข้าม".into());
            return Ok((BTreeMap::new(), false));
        };

        let mut labs = BTreeMap::new();
        for row in rows {
            let Some(kind) = classify_lab(&row.item_name) else {
                continue;
            };
            if labs.contains_key(&kind) {
                continue;
            }
            let Some((value, unit)) = parse_lab_value(kind, &row.result, &row.unit) else {
                continue;
            };
            labs.insert(
                kind,
                LabValue {
                    value,
                    text: row.result.trim().to_string(),
                    unit,
                    measured_on: normalize_date(row.order_date),
                    item_name: row.item_name,
                    flag: lab_flag(kind, value),
                },
            );
        }
        Ok((labs, true))
    }

    async fn load_vitals(
        &self,
        hn: &str,
        today: NaiveDate,
        warnings: &mut Vec<String>,
    ) -> Result<(Option<Vitals>, bool)> {
        let cutoff = today - chrono::Days::new(365);
        let rows: Vec<VitalsRow> = match self
            .fetch_rows(
                queries::LATEST_VITALS_BY_HN,
                &[Param::Str(hn.to_string()), Param::Date(cutoff)],
            )
            .await
        {
            Ok(rows) => rows,
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                warnings.push("อ่านน้ำหนัก/ส่วนสูง (opdscreen) ไม่ได้".into());
                return Ok((None, false));
            }
            Err(error) => return Err(error),
        };
        for row in rows {
            let weight = parse_number(&row.weight_kg).filter(|v| *v > 0.0);
            let height = parse_number(&row.height_cm).filter(|v| *v > 0.0);
            let bmi = parse_number(&row.bmi).filter(|v| *v > 0.0);
            if weight.is_some() || height.is_some() {
                let bmi = bmi.or_else(|| weight.zip(height).map(|(w, h)| w / (h / 100.0).powi(2)));
                return Ok((
                    Some(Vitals {
                        weight_kg: weight,
                        height_cm: height,
                        bmi,
                    }),
                    true,
                ));
            }
        }
        Ok((None, true))
    }

    async fn load_pmh(&self, hn: &str, warnings: &mut Vec<String>) -> Result<Option<String>> {
        match self
            .fetch_rows::<PmhRow>(queries::LATEST_PMH_BY_HN, &[Param::Str(hn.to_string())])
            .await
        {
            Ok(rows) => Ok(rows.into_iter().next().map(|row| row.pmh)),
            Err(Error::Database(source)) if is_schema_variation(&source) => {
                warnings.push("อ่านประวัติการเจ็บป่วย (opdscreen.pmh) ไม่ได้".into());
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    async fn fetch_rows<T>(&self, sql: &str, params: &[Param]) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> + Send + Unpin,
    {
        assert_read_only(sql)
            .map_err(|_| Error::ReadOnlyViolation(sql.chars().take(64).collect()))?;
        let mut query = sqlx::query_as::<_, T>(sql);
        for param in params {
            query = match param {
                Param::Str(value) => query.bind(value.clone()),
                Param::Date(value) => query.bind(*value),
                Param::Int(value) => query.bind(*value),
            };
        }
        query.fetch_all(&self.pool).await.map_err(Error::Database)
    }
}

enum Param {
    Str(String),
    Date(NaiveDate),
    Int(i64),
}

#[derive(Debug, FromRow)]
struct PingRow {
    #[allow(dead_code)]
    one: i32,
}

#[derive(Debug, FromRow)]
struct OfficerRow {
    officer_id: i64,
    login_name: String,
    password_md5: String,
    active: String,
    auth_type_id: i64,
    doctor_code: Option<String>,
    display_name: String,
}

#[derive(Debug, FromRow)]
struct PatientRow {
    hn: String,
    cid: String,
    full_name_th: String,
    birthday: Option<NaiveDate>,
    sex: String,
}

impl PatientRow {
    fn into_patient(self, today: NaiveDate) -> Patient {
        let sex = Sex::from_hosxp(&self.sex);
        let birthday = self.birthday.map(normalize_date);
        let age_years = birthday.map(|birth| thai::age_years(birth, today));
        Patient {
            hn: self.hn,
            cid: self.cid,
            full_name_th: self.full_name_th,
            birth_date: birthday,
            sex,
            age_years,
        }
    }
}

#[derive(Debug, FromRow)]
struct VisitRow {
    vn: String,
    vstdate: NaiveDate,
    vsttime: String,
    queue: String,
    department_code: String,
    department_name: String,
    doctor_code: String,
    doctor_name: String,
    order_count: i64,
}

#[derive(Debug, FromRow)]
struct OrderRow {
    vn: String,
    icode: String,
    item_no: i64,
    qty: String,
    drug_name: String,
    strength: String,
    units: String,
    du_name1: String,
    du_name2: String,
    du_name3: String,
    sp_name1: String,
    sp_name2: String,
    sp_name3: String,
    doctor_name: String,
    order_date: Option<NaiveDate>,
    order_time: String,
}

#[derive(Debug, FromRow)]
struct AllergyRow {
    agent: String,
    symptom: String,
    reporter: String,
    report_date: Option<NaiveDate>,
    note: String,
    icode: String,
    seriousness_id: String,
    no_alert: String,
    force_no_order: String,
}

#[derive(Debug, FromRow)]
struct DiagnosisRow {
    icd10: String,
    diag_type: String,
    vstdate: NaiveDate,
    #[allow(dead_code)]
    vn: String,
}

#[derive(Debug, FromRow)]
struct LabRow {
    order_date: NaiveDate,
    item_name: String,
    unit: String,
    result: String,
}

#[derive(Debug, FromRow)]
struct VitalsRow {
    weight_kg: String,
    height_cm: String,
    bmi: String,
    #[allow(dead_code)]
    vstdate: NaiveDate,
}

#[derive(Debug, FromRow)]
struct PmhRow {
    pmh: String,
    #[allow(dead_code)]
    vstdate: NaiveDate,
}

#[derive(Debug, Clone, Copy)]
struct Vitals {
    weight_kg: Option<f64>,
    height_cm: Option<f64>,
    bmi: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn build_clinical(
    patient: &Patient,
    request: &ReviewRequest,
    allergies: Vec<AllergyRecord>,
    allergy_data_available: bool,
    diagnoses: Vec<DiagnosisRecord>,
    diagnosis_data_available: bool,
    labs: BTreeMap<LabKind, LabValue>,
    lab_data_available: bool,
    vitals: Option<Vitals>,
    vitals_data_available: bool,
    pmh: Option<String>,
    warnings: &mut Vec<String>,
) -> ClinicalContext {
    let age = patient.age_years;
    let sex = patient.sex;
    let weight = request
        .weight_override
        .filter(|w| w.is_finite() && *w > 0.0)
        .or_else(|| vitals.and_then(|v| v.weight_kg));
    if request.weight_override.is_some() {
        warnings.push("ใช้น้ำหนักที่เภสัชกรป้อนในเซสชันนี้สำหรับ CrCl".into());
    }
    let height = vitals.and_then(|v| v.height_cm);
    let bmi = vitals
        .and_then(|v| v.bmi)
        .or_else(|| weight.zip(height).map(|(w, h)| w / (h / 100.0).powi(2)));
    let creatinine = labs.get(&LabKind::Creatinine).cloned();
    let lab_egfr = labs.get(&LabKind::Egfr).map(|lab| lab.value);

    let (egfr, egfr_source) = if let Some(value) = lab_egfr {
        (Some(value), "ผล lab".to_string())
    } else if let (Some(age), Some(scr)) = (age, creatinine.as_ref())
        && sex != Sex::Unknown
    {
        (
            ckd_epi_2021(scr.value, age, sex),
            "CKD-EPI 2021".to_string(),
        )
    } else {
        (None, "ไม่มีข้อมูล".to_string())
    };
    let crcl = match (age, creatinine.as_ref()) {
        (Some(age), Some(scr)) if sex != Sex::Unknown => {
            cockcroft_gault(scr.value, age, weight, sex)
        }
        _ => None,
    };

    if creatinine.is_none() && lab_data_available {
        warnings.push("ไม่พบผล creatinine ล่าสุด".into());
    }
    if weight.is_none() {
        warnings.push("ไม่พบน้ำหนักตัวล่าสุด - คำนวณ CrCl ไม่ได้".into());
    }
    if sex == Sex::Unknown {
        warnings.push("ไม่ทราบเพศผู้ป่วย - ไม่สามารถคำนวณ eGFR/CrCl ได้".into());
    }

    let (pregnancy, pregnancy_source) = if let Some(override_status) = request.pregnancy_override {
        (override_status, "เภสัชกรยืนยัน".to_string())
    } else if sex == Sex::Male {
        (PregnancyStatus::NotApplicable, "ผู้ป่วยชาย".to_string())
    } else if has_recent_pregnancy_code(&diagnoses, request.today) {
        (PregnancyStatus::Pregnant, "รหัส ICD-10".to_string())
    } else {
        (PregnancyStatus::Unknown, "ไม่พบข้อมูล".to_string())
    };

    ClinicalContext {
        age_years: age,
        sex,
        weight_kg: weight,
        height_cm: height,
        bmi,
        creatinine,
        egfr,
        egfr_source,
        crcl,
        potassium: labs.get(&LabKind::Potassium).cloned(),
        inr: labs.get(&LabKind::Inr).cloned(),
        alt: labs.get(&LabKind::Alt).cloned(),
        ast: labs.get(&LabKind::Ast).cloned(),
        hba1c: labs.get(&LabKind::Hba1c).cloned(),
        pregnancy,
        pregnancy_source,
        diagnoses,
        pmh,
        allergies,
        allergy_data_available,
        lab_data_available,
        diagnosis_data_available,
        vitals_data_available,
        medication_scope_note: "ตรวจเฉพาะรายการยาที่สั่งใน OPD วันนี้ (ทุก visit) ไม่รวมยาที่ได้รับจากที่อื่น"
            .into(),
    }
}

fn parse_number(raw: &str) -> Option<f64> {
    let cleaned = raw.trim().replace(',', "");
    let bytes = cleaned.as_bytes();
    let start = bytes.iter().position(|b| b.is_ascii_digit())?;
    let mut end = start;
    let mut seen_dot = false;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_digit() {
            end += 1;
        } else if b == b'.' && !seen_dot {
            seen_dot = true;
            end += 1;
        } else {
            break;
        }
    }
    cleaned[start..end].parse::<f64>().ok()
}

fn parse_lab_value(kind: LabKind, raw: &str, raw_unit: &str) -> Option<(f64, String)> {
    let value = parse_number(raw)?;
    let mut unit = raw_unit.trim().to_string();
    let mut value = value;
    if kind == LabKind::Creatinine {
        let lower = unit.to_lowercase();
        if lower.contains("umol") || lower.contains("µmol") || lower.contains("μmol") {
            value /= 88.4;
            unit = "mg/dL".into();
        }
    }
    if unit.is_empty() {
        unit = kind.unit().to_string();
    }
    Some((value, unit))
}

fn lab_flag(kind: LabKind, value: f64) -> LabFlag {
    match kind {
        LabKind::Creatinine => {
            if value > 1.3 {
                LabFlag::High
            } else if value < 0.6 {
                LabFlag::Low
            } else {
                LabFlag::Normal
            }
        }
        LabKind::Egfr => {
            if value < 60.0 {
                LabFlag::Low
            } else {
                LabFlag::Normal
            }
        }
        LabKind::Potassium => {
            if value > 5.0 {
                LabFlag::High
            } else if value < 3.5 {
                LabFlag::Low
            } else {
                LabFlag::Normal
            }
        }
        LabKind::Inr => {
            if value > 3.0 {
                LabFlag::High
            } else if value < 2.0 {
                LabFlag::Low
            } else {
                LabFlag::Normal
            }
        }
        LabKind::Alt | LabKind::Ast => {
            if value > 40.0 {
                LabFlag::High
            } else {
                LabFlag::Normal
            }
        }
        LabKind::Hba1c => {
            if value > 7.0 {
                LabFlag::High
            } else {
                LabFlag::Normal
            }
        }
    }
}

fn classify_lab(name: &str) -> Option<LabKind> {
    let text = name.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    if text.contains("urine") || text.contains("ปัสสาวะ") || text.contains("24 hr") {
        return None;
    }
    if text.contains("egfr") || text.contains("gfr") || text.contains("ckd-epi") {
        return Some(LabKind::Egfr);
    }
    let is_creatinine = text.contains("creatinine")
        || text.contains("creat")
        || text.contains("ครีอะตินิน")
        || text.contains("ครีเอตินิน");
    if is_creatinine && !text.contains("kinase") {
        return Some(LabKind::Creatinine);
    }
    if text.contains("potassium")
        || text.contains("โพแทสเซียม")
        || text == "k"
        || text.starts_with("k ")
        || text.contains("k+")
    {
        return Some(LabKind::Potassium);
    }
    if text.contains("inr") || text.contains("prothrombin") {
        return Some(LabKind::Inr);
    }
    if text.contains("sgpt") || text.contains("alanine") || has_token(&text, "alt") {
        return Some(LabKind::Alt);
    }
    if text.contains("sgot") || text.contains("aspartate") || has_token(&text, "ast") {
        return Some(LabKind::Ast);
    }
    if text.contains("hba1c") || text.contains("a1c") || text.contains("glycated") {
        return Some(LabKind::Hba1c);
    }
    None
}

fn has_token(text: &str, token: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .any(|part| part == token)
}

fn condition_label(code: &str) -> Option<&'static str> {
    let code = code.to_ascii_uppercase();
    let starts = |prefixes: &[&str]| prefixes.iter().any(|p| code.starts_with(p));
    if starts(&["N18", "N19"]) {
        Some("CKD")
    } else if starts(&["I50"]) {
        Some("heart failure")
    } else if starts(&["K25", "K26", "K27"]) {
        Some("peptic ulcer")
    } else if starts(&["K92.0", "K92.1", "K92.2"]) {
        Some("GI bleed")
    } else if starts(&["K70.3", "K74"]) {
        Some("cirrhosis")
    } else if starts(&["J45"]) {
        Some("asthma")
    } else if starts(&["J44"]) {
        Some("COPD")
    } else if starts(&["G20"]) {
        Some("Parkinson disease")
    } else if starts(&["F00", "F01", "F02", "F03", "G30"]) {
        Some("dementia")
    } else if starts(&["N40"]) {
        Some("BPH")
    } else if starts(&["H40"]) {
        Some("glaucoma")
    } else if starts(&["E10", "E11", "E12", "E13", "E14"]) {
        Some("diabetes")
    } else if starts(&["O0", "O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9"]) {
        Some("pregnancy-related")
    } else if starts(&["Z33", "Z34", "Z35"]) {
        Some("pregnancy")
    } else {
        None
    }
}

fn has_recent_pregnancy_code(diagnoses: &[DiagnosisRecord], today: NaiveDate) -> bool {
    let cutoff = today - chrono::Days::new(280);
    diagnoses.iter().any(|diagnosis| {
        diagnosis.date >= cutoff
            && (diagnosis.icd10.starts_with('O')
                || diagnosis.icd10.starts_with("Z33")
                || diagnosis.icd10.starts_with("Z34")
                || diagnosis.icd10.starts_with("Z35"))
    })
}

fn none_if_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_schema_variation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db) => db
            .code()
            .as_deref()
            .is_some_and(|code| matches!(code, "1054" | "1146" | "42S22" | "42S02")),
        _ => false,
    }
}

fn dedupe_warnings(warnings: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    warnings.retain(|warning| seen.insert(warning.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_lab_names() {
        assert_eq!(
            classify_lab("Creatinine (Serum)"),
            Some(LabKind::Creatinine)
        );
        assert_eq!(classify_lab("Creatinine (Urine)"), None);
        assert_eq!(classify_lab("Potassium"), Some(LabKind::Potassium));
        assert_eq!(classify_lab("INR"), Some(LabKind::Inr));
        assert_eq!(classify_lab("SGPT (ALT)"), Some(LabKind::Alt));
        assert_eq!(classify_lab("HbA1C"), Some(LabKind::Hba1c));
        assert_eq!(classify_lab("eGFR (CKD-EPI)"), Some(LabKind::Egfr));
        assert_eq!(classify_lab("SGOT (AST)"), Some(LabKind::Ast));
        assert_eq!(classify_lab("Blast"), None);
        assert_eq!(classify_lab("Cast"), None);
        assert_eq!(classify_lab("Yeast"), None);
    }

    #[test]
    fn parses_numeric_lab_result_with_qualifiers() {
        assert_eq!(parse_number("<0.5"), Some(0.5));
        assert_eq!(parse_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_number("Negative"), None);
    }

    #[test]
    fn converts_creatinine_umol_to_mg_dl() {
        let (value, unit) = parse_lab_value(LabKind::Creatinine, "88.4", "umol/L").unwrap();
        assert!((value - 1.0).abs() < 0.001);
        assert_eq!(unit, "mg/dL");
    }

    #[test]
    fn condition_labels_cover_rule_prefixes() {
        assert_eq!(condition_label("N18.3"), Some("CKD"));
        assert_eq!(condition_label("I50.9"), Some("heart failure"));
        assert_eq!(condition_label("J45"), Some("asthma"));
    }

    #[test]
    fn recent_pregnancy_code_is_date_bounded() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        let recent = DiagnosisRecord {
            icd10: "O80".into(),
            label: None,
            date: today - chrono::Days::new(10),
            diag_type: "1".into(),
        };
        let old = DiagnosisRecord {
            icd10: "O80".into(),
            label: None,
            date: today - chrono::Days::new(400),
            diag_type: "1".into(),
        };
        assert!(has_recent_pregnancy_code(&[recent], today));
        assert!(!has_recent_pregnancy_code(&[old], today));
    }

    #[test]
    fn unknown_icd_has_no_local_label() {
        assert_eq!(condition_label("Z00.0"), None);
    }
}
