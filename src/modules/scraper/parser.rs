use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

use crate::{
    errors::AppError,
    modules::scraper::models::{CsvRow, ParsedSession},
};

pub fn parse_csv(csv_content: &str) -> Result<Vec<ParsedSession>, AppError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_content.as_bytes());

    let mut sessions = Vec::new();

    for (i, result) in reader.deserialize::<CsvRow>().enumerate() {
        let row = result
            .map_err(|e| AppError::BadRequest(format!("CSV malformado en fila {}: {e}", i + 2)))?;

        let parsed = parse_row(row, i + 2)?;
        sessions.push(parsed);
    }

    Ok(sessions)
}

fn parse_row(row: CsvRow, line: usize) -> Result<ParsedSession, AppError> {
    // Fecha -> DD/MM/YYYY
    let date = NaiveDate::parse_from_str(&row.day, "%d/%m/%Y").map_err(|_| {
        AppError::BadRequest(format!("Fecha inválida '{}' en fila {line}", row.day))
    })?;

    // Hora
    let start_time = NaiveTime::parse_from_str(&row.start, "%H:%M").map_err(|_| {
        AppError::BadRequest(format!(
            "Hora de inicio inválida '{}' en fila {line}",
            row.start
        ))
    })?;

    // Hora de fin
    let end_time = NaiveTime::parse_from_str(&row.end, "%H:%M").map_err(|_| {
        AppError::BadRequest(format!("Hora de fin inválida '{}' en fila {line}", row.end))
    })?;

    // Duración
    let duration_min = (end_time - start_time).num_minutes() as i32;
    if duration_min <= 0 {
        return Err(AppError::BadRequest(format!(
            "Duración inválida en fila {line}: end <= start"
        )));
    }

    let starts_at = Utc.from_utc_datetime(&NaiveDateTime::new(date, start_time));

    let (subject, grp) = parse_subject_grp(&row.subject, line)?;

    let classroom = row.room.filter(|r| !r.trim().is_empty());

    Ok(ParsedSession {
        subject,
        grp,
        starts_at,
        duration_min,
        classroom,
    })
}

fn parse_subject_grp(raw: &str, line: usize) -> Result<(String, String), AppError> {
    let dot_pos = raw.find('.').ok_or_else(|| {
        AppError::BadRequest(format!(
            "Formato de asignatura inválido '{raw}' en fila {line} - esperado 'ASIGNATURA.GRUPO'"
        ))
    })?;

    let subject = raw[..dot_pos].trim().to_string();
    let grp = raw[dot_pos + 1..].trim().to_string();

    if subject.is_empty() || grp.is_empty() {
        return Err(AppError::BadRequest(format!(
            "Subject o grupo vacío en '{raw}' en fila {line}",
        )));
    }

    Ok((subject, grp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subject_grp_teoría() {
        let (s, g) = parse_subject_grp("AL.T.1", 1).unwrap();
        assert_eq!(s, "AL");
        assert_eq!(g, "T.1");
    }

    #[test]
    fn parse_subject_grp_laboratorio() {
        let (s, g) = parse_subject_grp("ALG.L.1", 1).unwrap();
        assert_eq!(s, "ALG");
        assert_eq!(g, "L.1");
    }

    #[test]
    fn parse_subject_grp_ce() {
        let (s, g) = parse_subject_grp("MAT.CE-2", 1).unwrap();
        assert_eq!(s, "MAT");
        assert_eq!(g, "CE-2");
    }

    #[test]
    fn parse_subject_grp_sin_punto_falla() {
        assert!(parse_subject_grp("ALSINDOT", 1).is_err());
    }

    #[test]
    fn parse_csv_completo() {
        let csv = "Day,Start,End,Subject,Room\n\
                   09/09/2025,11:00,12:00,AL.T.1,A-2-01\n\
                   11/09/2025,11:00,12:00,AL.T.1,A-2-01\n";

        let sessions = parse_csv(csv).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].subject, "AL");
        assert_eq!(sessions[0].grp, "T.1");
        assert_eq!(sessions[0].duration_min, 60);
        assert_eq!(sessions[0].classroom, Some("A-2-01".into()));
    }

    #[test]
    fn parse_csv_room_vacio_es_none() {
        let csv = "Day,Start,End,Subject,Room\n\
                   09/09/2025,11:00,12:00,AL.T.1,\n";

        let sessions = parse_csv(csv).unwrap();
        assert!(sessions[0].classroom.is_none());
    }

    #[test]
    fn parse_csv_duracion_negativa_falla() {
        let csv = "Day,Start,End,Subject,Room\n\
                   09/09/2025,12:00,11:00,AL.T.1,A-2-01\n";

        assert!(parse_csv(csv).is_err());
    }
}
