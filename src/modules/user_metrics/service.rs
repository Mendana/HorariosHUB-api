use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::user_metrics::{
        models::{
            GetUserMetricsResponse, SemesterBreakdown, SemesterSummary, SessionType, SubjectStat,
            TypeStat, UserSessionRow, WeekExtreme, WeekdayStat, WeeklyEvolutionEntry,
        },
        repository::UserMetricsRepository,
    },
};

pub async fn get_user_metrics(
    metrics_repo: &dyn UserMetricsRepository,
    user_id: Uuid,
    semester: Option<u8>,
) -> Result<GetUserMetricsResponse, AppError> {
    let semester = validate_semester(semester)?;

    let rows = metrics_repo.fetch_all_user_rows(&user_id).await?;

    let filtered_rows: Vec<&UserSessionRow> = match semester {
        Some(s) => rows
            .iter()
            .filter(|row| classify_session_semester(row.starts_at) == Some(s))
            .collect(),
        None => rows.iter().collect(),
    };

    let total_hours = sum_hours(&filtered_rows);
    let weeks = weekly_hours_map(&filtered_rows);
    let weekly_average_hours = weekly_average(total_hours, &weeks);
    let (busiest_week, lightest_week) = week_extremes(&weeks);
    let (by_weekday, days_without_class) = weekday_distribution(&filtered_rows);
    let by_type = type_distribution(&filtered_rows);
    let by_subject = subject_distribution(&filtered_rows, total_hours);

    let earliest_start_time = filtered_rows.iter().map(|row| row.starts_at.time()).min();
    let latest_end_time = filtered_rows
        .iter()
        .map(|row| session_end(row).time())
        .max();

    let now = Utc::now();
    let completed_classes = filtered_rows
        .iter()
        .filter(|row| session_end(row) <= now)
        .count() as i64;
    let remaining_classes = filtered_rows.len() as i64 - completed_classes;

    let semesters = semester_breakdown(&rows);

    Ok(GetUserMetricsResponse {
        total_hours,
        weekly_average_hours,
        by_weekday,
        days_without_class,
        by_type,
        by_subject,
        earliest_start_time,
        latest_end_time,
        busiest_week,
        lightest_week,
        completed_classes,
        remaining_classes,
        semesters,
    })
}

/// Serie semanal (para gráfica de evolución) entre la primera y la última semana con datos
/// dentro del filtro pedido. Las semanas intermedias sin clase se rellenan a cero; si no hay
/// ninguna sesión en el filtro, no hay rango que anclar y se devuelve un array vacío.
pub async fn get_weekly_evolution(
    metrics_repo: &dyn UserMetricsRepository,
    user_id: Uuid,
    semester: Option<u8>,
) -> Result<Vec<WeeklyEvolutionEntry>, AppError> {
    let semester = validate_semester(semester)?;

    let rows = metrics_repo.fetch_all_user_rows(&user_id).await?;

    let filtered_rows: Vec<&UserSessionRow> = match semester {
        Some(s) => rows
            .iter()
            .filter(|row| classify_session_semester(row.starts_at) == Some(s))
            .collect(),
        None => rows.iter().collect(),
    };

    if filtered_rows.is_empty() {
        return Ok(Vec::new());
    }

    let now = Utc::now();
    // (horas, nº de clases, nº de clases ya completadas)
    let mut per_week: BTreeMap<(i32, u32), (f64, i64, i64)> = BTreeMap::new();
    for row in &filtered_rows {
        let iso = row.starts_at.iso_week();
        let entry = per_week
            .entry((iso.year(), iso.week()))
            .or_insert((0.0, 0, 0));
        entry.0 += row.duration_min as f64 / 60.0;
        entry.1 += 1;
        if session_end(row) <= now {
            entry.2 += 1;
        }
    }

    let (first_year, first_week) = *per_week.keys().next().expect("per_week no está vacío");
    let (last_year, last_week) = *per_week.keys().next_back().expect("per_week no está vacío");
    let last_week_start = week_bounds(last_year, last_week).0;

    let mut entries = Vec::new();
    let mut cursor = week_bounds(first_year, first_week).0;
    while cursor <= last_week_start {
        let iso = cursor.iso_week();
        let key = (iso.year(), iso.week());
        let (total_hours, class_count, completed_classes) =
            per_week.get(&key).copied().unwrap_or((0.0, 0, 0));
        let (week_start, week_end) = week_bounds(key.0, key.1);

        entries.push(WeeklyEvolutionEntry {
            iso_year: key.0,
            iso_week: key.1,
            week_start,
            week_end,
            total_hours,
            class_count,
            completed_classes,
            remaining_classes: class_count - completed_classes,
        });

        cursor += Duration::days(7);
    }

    Ok(entries)
}

fn validate_semester(semester: Option<u8>) -> Result<Option<u8>, AppError> {
    match semester {
        Some(s) if s != 1 && s != 2 => {
            tracing::warn!(semester = %s, "Valor de semester inválido");
            Err(AppError::BadRequest(format!("Invalid semester: {s}")))
        }
        Some(s) => Ok(Some(s)),
        None => {
            tracing::info!("No se ha especificado semestre. Devolviendo el año completo");
            Ok(None)
        }
    }
}

fn session_end(row: &UserSessionRow) -> DateTime<Utc> {
    row.starts_at + Duration::minutes(row.duration_min as i64)
}

fn sum_hours(rows: &[&UserSessionRow]) -> f64 {
    rows.iter().map(|row| row.duration_min as f64 / 60.0).sum()
}

/// Horas totales agrupadas por semana ISO (año, número de semana).
fn weekly_hours_map(rows: &[&UserSessionRow]) -> BTreeMap<(i32, u32), f64> {
    let mut weeks: BTreeMap<(i32, u32), f64> = BTreeMap::new();
    for row in rows {
        let iso = row.starts_at.iso_week();
        *weeks.entry((iso.year(), iso.week())).or_insert(0.0) += row.duration_min as f64 / 60.0;
    }
    weeks
}

/// Media de horas por semana, considerando solo semanas con al menos una clase.
fn weekly_average(total_hours: f64, weeks: &BTreeMap<(i32, u32), f64>) -> f64 {
    if weeks.is_empty() {
        0.0
    } else {
        total_hours / weeks.len() as f64
    }
}

fn week_bounds(iso_year: i32, iso_week: u32) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_isoywd_opt(iso_year, iso_week, Weekday::Mon)
        .expect("semana ISO válida ya calculada a partir de una fecha real");
    (start, start + Duration::days(6))
}

/// Semana con más y con menos horas. En caso de empate gana la semana ISO más antigua
/// (el `BTreeMap` ya itera en orden ascendente de `(year, week)`).
fn week_extremes(weeks: &BTreeMap<(i32, u32), f64>) -> (Option<WeekExtreme>, Option<WeekExtreme>) {
    let mut busiest: Option<(&(i32, u32), &f64)> = None;
    let mut lightest: Option<(&(i32, u32), &f64)> = None;

    for entry in weeks.iter() {
        let (_, hours) = entry;
        if busiest.is_none_or(|(_, best)| hours > best) {
            busiest = Some(entry);
        }
        if lightest.is_none_or(|(_, best)| hours < best) {
            lightest = Some(entry);
        }
    }

    let to_extreme = |(&(iso_year, iso_week), &hours): (&(i32, u32), &f64)| {
        let (week_start, week_end) = week_bounds(iso_year, iso_week);
        WeekExtreme {
            iso_year,
            iso_week,
            week_start,
            week_end,
            hours,
        }
    };

    (busiest.map(to_extreme), lightest.map(to_extreme))
}

fn weekday_distribution(rows: &[&UserSessionRow]) -> (Vec<WeekdayStat>, Vec<u8>) {
    let mut by_weekday: BTreeMap<u8, (f64, i64)> = BTreeMap::new();
    for row in rows {
        let weekday = row.starts_at.weekday().number_from_monday() as u8;
        if !(1..=5).contains(&weekday) {
            // Fin de semana: caso anómalo, no debería ocurrir en un horario lectivo real.
            continue;
        }
        let entry = by_weekday.entry(weekday).or_insert((0.0, 0));
        entry.0 += row.duration_min as f64 / 60.0;
        entry.1 += 1;
    }

    let stats = by_weekday
        .iter()
        .map(|(&weekday, &(hours, class_count))| WeekdayStat {
            weekday,
            hours,
            class_count,
        })
        .collect();

    let days_without_class = (1..=5).filter(|d| !by_weekday.contains_key(d)).collect();

    (stats, days_without_class)
}

fn classify_session_type(grp: &str) -> SessionType {
    let g = grp.to_uppercase();
    if g.starts_with("T.") || g.starts_with("CE-") {
        SessionType::Teoria
    } else if g.starts_with("PL-") || g.starts_with("PL.") {
        SessionType::Laboratorio
    } else if g.starts_with("P-") || g.starts_with("P.") {
        SessionType::Practica
    } else if g.starts_with("TG-") || g.starts_with("TG.") {
        SessionType::Tutoria
    } else {
        SessionType::Otros
    }
}

fn type_distribution(rows: &[&UserSessionRow]) -> Vec<TypeStat> {
    let mut totals: BTreeMap<SessionType, (f64, i64)> = BTreeMap::new();
    for row in rows {
        let entry = totals
            .entry(classify_session_type(&row.grp))
            .or_insert((0.0, 0));
        entry.0 += row.duration_min as f64 / 60.0;
        entry.1 += 1;
    }

    // Orden fijo y estable en la respuesta, independiente del orden de inserción.
    [
        SessionType::Teoria,
        SessionType::Laboratorio,
        SessionType::Practica,
        SessionType::Tutoria,
        SessionType::Otros,
    ]
    .into_iter()
    .filter_map(|session_type| {
        totals
            .get(&session_type)
            .map(|&(hours, class_count)| TypeStat {
                session_type: session_type.clone(),
                hours,
                class_count,
            })
    })
    .collect()
}

fn subject_distribution(rows: &[&UserSessionRow], total_hours: f64) -> Vec<SubjectStat> {
    let mut totals: BTreeMap<String, f64> = BTreeMap::new();
    for row in rows {
        *totals.entry(row.subject.clone()).or_insert(0.0) += row.duration_min as f64 / 60.0;
    }

    let mut stats: Vec<SubjectStat> = totals
        .into_iter()
        .map(|(subject, hours)| {
            let percentage = if total_hours > 0.0 {
                hours / total_hours * 100.0
            } else {
                0.0
            };
            SubjectStat {
                subject,
                hours,
                percentage,
            }
        })
        .collect();

    stats.sort_by(|a, b| {
        b.hours
            .total_cmp(&a.hours)
            .then_with(|| a.subject.cmp(&b.subject))
    });
    stats
}

/// Semestre al que pertenece una sesión según su mes: 9-12 -> 1, 1-5 -> 2, resto (verano) -> ninguno.
fn classify_session_semester(starts_at: DateTime<Utc>) -> Option<u8> {
    match starts_at.month() {
        9..=12 => Some(1),
        1..=5 => Some(2),
        _ => None,
    }
}

/// Desglose por semestre, presente solo si hay sesiones en ambos. Ignora el filtro `semester`
/// de la query: se calcula siempre sobre el historial completo del usuario.
fn semester_breakdown(rows: &[UserSessionRow]) -> Option<SemesterBreakdown> {
    let semester_1: Vec<&UserSessionRow> = rows
        .iter()
        .filter(|row| classify_session_semester(row.starts_at) == Some(1))
        .collect();
    let semester_2: Vec<&UserSessionRow> = rows
        .iter()
        .filter(|row| classify_session_semester(row.starts_at) == Some(2))
        .collect();

    if semester_1.is_empty() || semester_2.is_empty() {
        return None;
    }

    Some(SemesterBreakdown {
        semester_1: semester_summary(&semester_1),
        semester_2: semester_summary(&semester_2),
    })
}

fn semester_summary(rows: &[&UserSessionRow]) -> SemesterSummary {
    let total_hours = sum_hours(rows);
    let weeks = weekly_hours_map(rows);
    SemesterSummary {
        total_hours,
        class_count: rows.len() as i64,
        weekly_average_hours: weekly_average(total_hours, &weeks),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::errors::AppError;

    fn row(
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
        duration_min: i32,
    ) -> UserSessionRow {
        UserSessionRow {
            subject: subject.to_string(),
            grp: grp.to_string(),
            starts_at,
            duration_min,
        }
    }

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    struct MockUserMetricsRepository {
        rows: Vec<UserSessionRow>,
    }

    #[async_trait::async_trait]
    impl UserMetricsRepository for MockUserMetricsRepository {
        async fn fetch_all_user_rows(
            &self,
            _user_id: &Uuid,
        ) -> Result<Vec<UserSessionRow>, AppError> {
            Ok(self
                .rows
                .iter()
                .map(|r| row(&r.subject, &r.grp, r.starts_at, r.duration_min))
                .collect())
        }
    }

    #[tokio::test]
    async fn sin_sesiones_devuelve_todo_a_cero() {
        let repo = MockUserMetricsRepository { rows: vec![] };
        let response = get_user_metrics(&repo, Uuid::new_v4(), None).await.unwrap();

        assert_eq!(response.total_hours, 0.0);
        assert_eq!(response.weekly_average_hours, 0.0);
        assert!(response.by_weekday.is_empty());
        assert_eq!(response.days_without_class, vec![1, 2, 3, 4, 5]);
        assert!(response.by_type.is_empty());
        assert!(response.by_subject.is_empty());
        assert_eq!(response.earliest_start_time, None);
        assert_eq!(response.latest_end_time, None);
        assert!(response.busiest_week.is_none());
        assert!(response.lightest_week.is_none());
        assert_eq!(response.completed_classes, 0);
        assert_eq!(response.remaining_classes, 0);
        assert!(response.semesters.is_none());
    }

    #[test]
    fn clasifica_los_cinco_tipos_de_sesion() {
        assert_eq!(classify_session_type("T.1"), SessionType::Teoria);
        assert_eq!(classify_session_type("CE-2"), SessionType::Teoria);
        assert_eq!(classify_session_type("PL-1"), SessionType::Laboratorio);
        assert_eq!(classify_session_type("PL.1"), SessionType::Laboratorio);
        assert_eq!(classify_session_type("P-1"), SessionType::Practica);
        assert_eq!(classify_session_type("P.1"), SessionType::Practica);
        assert_eq!(classify_session_type("TG-1"), SessionType::Tutoria);
        assert_eq!(classify_session_type("TG.1"), SessionType::Tutoria);
        assert_eq!(classify_session_type("X.1"), SessionType::Otros);
    }

    #[test]
    fn clasifica_semestre_por_mes() {
        assert_eq!(classify_session_semester(dt(2026, 9, 1, 9, 0)), Some(1));
        assert_eq!(classify_session_semester(dt(2026, 12, 31, 9, 0)), Some(1));
        assert_eq!(classify_session_semester(dt(2026, 1, 1, 9, 0)), Some(2));
        assert_eq!(classify_session_semester(dt(2026, 5, 31, 9, 0)), Some(2));
        assert_eq!(classify_session_semester(dt(2026, 6, 15, 9, 0)), None);
        assert_eq!(classify_session_semester(dt(2026, 8, 31, 9, 0)), None);
    }

    #[tokio::test]
    async fn reparto_por_dia_de_la_semana_y_dias_libres() {
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60),  // lunes
                row("MAT", "T.1", dt(2026, 9, 9, 9, 0), 120), // miércoles
            ],
        };
        let response = get_user_metrics(&repo, Uuid::new_v4(), None).await.unwrap();

        assert_eq!(response.by_weekday.len(), 2);
        assert_eq!(response.by_weekday[0].weekday, 1);
        assert_eq!(response.by_weekday[0].hours, 1.0);
        assert_eq!(response.by_weekday[1].weekday, 3);
        assert_eq!(response.by_weekday[1].hours, 2.0);
        assert_eq!(response.days_without_class, vec![2, 4, 5]);
    }

    #[tokio::test]
    async fn media_semanal_solo_cuenta_semanas_con_clase() {
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60),
                row("MAT", "T.1", dt(2026, 9, 14, 9, 0), 60),
                row("MAT", "T.1", dt(2026, 9, 21, 9, 0), 120),
            ],
        };
        let response = get_user_metrics(&repo, Uuid::new_v4(), None).await.unwrap();

        assert_eq!(response.total_hours, 4.0);
        assert_eq!(response.weekly_average_hours, 4.0 / 3.0);
    }

    #[tokio::test]
    async fn semana_mas_y_menos_cargada_con_empate_gana_la_mas_antigua() {
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60),
                row("MAT", "T.1", dt(2026, 9, 14, 9, 0), 60),
            ],
        };
        let response = get_user_metrics(&repo, Uuid::new_v4(), None).await.unwrap();

        let busiest = response.busiest_week.unwrap();
        let lightest = response.lightest_week.unwrap();
        assert_eq!(busiest.iso_week, 37);
        assert_eq!(lightest.iso_week, 37);
    }

    #[tokio::test]
    async fn completadas_vs_restantes_segun_ahora() {
        let now = Utc::now();
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", now - Duration::days(7), 60),
                row("MAT", "T.1", now + Duration::days(7), 60),
            ],
        };
        let response = get_user_metrics(&repo, Uuid::new_v4(), None).await.unwrap();

        assert_eq!(response.completed_classes, 1);
        assert_eq!(response.remaining_classes, 1);
    }

    #[tokio::test]
    async fn semester_invalido_devuelve_bad_request() {
        let repo = MockUserMetricsRepository { rows: vec![] };
        let err = get_user_metrics(&repo, Uuid::new_v4(), Some(3))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn desglose_semestral_solo_si_hay_datos_en_ambos_semestres() {
        let repo_un_semestre = MockUserMetricsRepository {
            rows: vec![row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60)],
        };
        let response = get_user_metrics(&repo_un_semestre, Uuid::new_v4(), None)
            .await
            .unwrap();
        assert!(response.semesters.is_none());

        let repo_ambos = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60),
                row("MAT", "T.1", dt(2026, 2, 2, 9, 0), 120),
            ],
        };
        let response = get_user_metrics(&repo_ambos, Uuid::new_v4(), None)
            .await
            .unwrap();
        let semesters = response.semesters.unwrap();
        assert_eq!(semesters.semester_1.total_hours, 1.0);
        assert_eq!(semesters.semester_2.total_hours, 2.0);
    }

    #[tokio::test]
    async fn evolucion_semanal_sin_sesiones_en_el_filtro_devuelve_vacio() {
        let repo = MockUserMetricsRepository { rows: vec![] };
        let entries = get_weekly_evolution(&repo, Uuid::new_v4(), None)
            .await
            .unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn evolucion_semanal_rellena_huecos_con_ceros_y_va_ordenada() {
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60), // semana ISO 37
                row("MAT", "T.1", dt(2026, 9, 28, 9, 0), 120), // semana ISO 40
            ],
        };
        let entries = get_weekly_evolution(&repo, Uuid::new_v4(), None)
            .await
            .unwrap();

        // 37, 38, 39, 40 -> 4 semanas, sin saltarse ninguna
        assert_eq!(entries.len(), 4);
        let weeks: Vec<u32> = entries.iter().map(|e| e.iso_week).collect();
        assert_eq!(weeks, vec![37, 38, 39, 40]);

        assert_eq!(entries[0].total_hours, 1.0);
        assert_eq!(entries[0].class_count, 1);
        // Semanas intermedias sin clase: todo a cero, no desaparecen.
        assert_eq!(entries[1].total_hours, 0.0);
        assert_eq!(entries[1].class_count, 0);
        assert_eq!(entries[1].completed_classes, 0);
        assert_eq!(entries[1].remaining_classes, 0);
        assert_eq!(entries[2].total_hours, 0.0);

        assert_eq!(entries[3].total_hours, 2.0);
        assert_eq!(entries[3].class_count, 1);
    }

    #[tokio::test]
    async fn evolucion_semanal_completadas_vs_pendientes_por_semana() {
        let now = Utc::now();
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", now - Duration::days(21), 60),
                row(
                    "MAT",
                    "T.1",
                    now - Duration::days(21) + Duration::hours(2),
                    60,
                ),
                row("MAT", "T.1", now + Duration::days(21), 60),
            ],
        };
        let entries = get_weekly_evolution(&repo, Uuid::new_v4(), None)
            .await
            .unwrap();

        let past_week = &entries[0];
        assert_eq!(past_week.class_count, 2);
        assert_eq!(past_week.completed_classes, 2);
        assert_eq!(past_week.remaining_classes, 0);

        let future_week = entries.last().unwrap();
        assert_eq!(future_week.class_count, 1);
        assert_eq!(future_week.completed_classes, 0);
        assert_eq!(future_week.remaining_classes, 1);
    }

    #[tokio::test]
    async fn evolucion_semanal_filtra_por_semester() {
        let repo = MockUserMetricsRepository {
            rows: vec![
                row("MAT", "T.1", dt(2026, 9, 7, 9, 0), 60), // semestre 1
                row("FIS", "PL.1", dt(2026, 2, 2, 9, 0), 120), // semestre 2
            ],
        };
        let entries = get_weekly_evolution(&repo, Uuid::new_v4(), Some(1))
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].iso_week, 37);
        assert_eq!(entries[0].total_hours, 1.0);
    }
}
