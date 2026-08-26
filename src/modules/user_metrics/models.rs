use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSessionRow {
    pub subject: String,
    pub grp: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetUserMetricsQuery {
    pub semester: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct GetUserMetricsResponse {
    pub total_hours: f64,
    pub weekly_average_hours: f64,
    pub by_weekday: Vec<WeekdayStat>,
    pub days_without_class: Vec<u8>,
    pub by_type: Vec<TypeStat>,
    pub by_subject: Vec<SubjectStat>,
    pub earliest_start_time: Option<NaiveTime>,
    pub latest_end_time: Option<NaiveTime>,
    pub busiest_week: Option<WeekExtreme>,
    pub lightest_week: Option<WeekExtreme>,
    pub completed_classes: i64,
    pub remaining_classes: i64,
    pub semesters: Option<SemesterBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct WeekdayStat {
    pub weekday: u8,
    pub hours: f64,
    pub class_count: i64,
}

#[derive(Debug, Serialize)]
pub struct TypeStat {
    pub session_type: SessionType,
    pub hours: f64,
    pub class_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SubjectStat {
    pub subject: String,
    pub hours: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct WeekExtreme {
    pub iso_year: i32,
    pub iso_week: u32,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub hours: f64,
}

#[derive(Debug, Serialize)]
pub struct SemesterBreakdown {
    pub semester_1: SemesterSummary,
    pub semester_2: SemesterSummary,
}

#[derive(Debug, Serialize)]
pub struct SemesterSummary {
    pub total_hours: f64,
    pub class_count: i64,
    pub weekly_average_hours: f64,
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Teoria,
    Laboratorio,
    Practica,
    Tutoria,
    Otros,
}

#[derive(Debug, Serialize)]
pub struct WeeklyEvolutionEntry {
    pub iso_year: i32,
    pub iso_week: u32,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub total_hours: f64,
    pub class_count: i64,
    pub completed_classes: i64,
    pub remaining_classes: i64,
}
