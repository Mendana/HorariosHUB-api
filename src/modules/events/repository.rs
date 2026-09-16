use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::events::models::{
        CreateEventInput, EventRow, EventRowWithTotal, OccurrenceRow, RecurrenceInterval,
        UpdateEventInput,
    },
};

#[async_trait::async_trait]
pub trait EventRepository: Send + Sync {
    async fn subject_exists(&self, subject: &str) -> Result<bool, AppError>;

    /// De la lista `candidates`, devuelve los que realmente existen como grupo
    /// de `subject` en `subject_groups`.
    async fn find_existing_groups(
        &self,
        subject: &str,
        candidates: &[String],
    ) -> Result<Vec<String>, AppError>;

    async fn create_event(&self, input: CreateEventInput) -> Result<EventRow, AppError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<EventRow>, AppError>;

    async fn update_event(&self, id: Uuid, input: UpdateEventInput)
    -> Result<EventRow, AppError>;

    async fn delete_event(&self, id: Uuid) -> Result<(), AppError>;

    /// Reemplaza por completo los grupos asociados a un evento. `groups` vacío
    /// significa "aplica a toda la asignatura".
    async fn set_event_groups(
        &self,
        event_id: Uuid,
        subject: &str,
        groups: &[String],
    ) -> Result<(), AppError>;

    async fn find_groups(&self, event_id: Uuid) -> Result<Vec<String>, AppError>;

    /// Devuelve los grupos de varios eventos a la vez (evita N+1 al listar).
    async fn find_groups_for_events(
        &self,
        event_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError>;

    async fn list_events(
        &self,
        subject: Option<&str>,
        search: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<EventRowWithTotal>, i64), AppError>;

    /// Expande eventos (recurrentes y puntuales) en ocurrencias concretas
    /// dentro de `[from, to)`.
    async fn list_occurrences(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        subject: Option<&str>,
    ) -> Result<Vec<OccurrenceRow>, AppError>;
}

pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl EventRepository for PgEventRepository {
    #[tracing::instrument(skip(self), fields(subject = %subject))]
    async fn subject_exists(&self, subject: &str) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM subject_groups WHERE subject = $1) AS "exists!""#,
            subject
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    #[tracing::instrument(skip(self, candidates), fields(subject = %subject))]
    async fn find_existing_groups(
        &self,
        subject: &str,
        candidates: &[String],
    ) -> Result<Vec<String>, AppError> {
        let groups = sqlx::query_scalar!(
            r#"SELECT grp FROM subject_groups WHERE subject = $1 AND grp = ANY($2)"#,
            subject,
            candidates
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(groups)
    }

    #[tracing::instrument(skip(self, input), fields(subject = %input.subject, created_by = %input.created_by))]
    async fn create_event(&self, input: CreateEventInput) -> Result<EventRow, AppError> {
        let row = sqlx::query_as!(
            EventRow,
            r#"
            INSERT INTO events (
                title, description, subject, starts_at, duration_min, classroom,
                recurrence_interval, recurrence_end_date, created_by, created_by_email
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, (SELECT email FROM users WHERE id = $9))
            RETURNING
                id, title, description, subject, starts_at, duration_min, classroom,
                recurrence_interval AS "recurrence_interval: RecurrenceInterval",
                recurrence_end_date, created_by_email, created_at
            "#,
            input.title,
            input.description,
            input.subject,
            input.starts_at,
            input.duration_min,
            input.classroom,
            input.recurrence_interval as Option<RecurrenceInterval>,
            input.recurrence_end_date,
            input.created_by,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    #[tracing::instrument(skip(self), fields(event_id = %id))]
    async fn find_by_id(&self, id: Uuid) -> Result<Option<EventRow>, AppError> {
        let row = sqlx::query_as!(
            EventRow,
            r#"
            SELECT
                id, title, description, subject, starts_at, duration_min, classroom,
                recurrence_interval AS "recurrence_interval: RecurrenceInterval",
                recurrence_end_date, created_by_email, created_at
            FROM events
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    #[tracing::instrument(skip(self, input), fields(event_id = %id))]
    async fn update_event(
        &self,
        id: Uuid,
        input: UpdateEventInput,
    ) -> Result<EventRow, AppError> {
        let (recurrence_interval, recurrence_end_date) = match input.recurrence {
            Some((interval, end_date)) => (Some(interval), Some(end_date)),
            None => (None, None),
        };

        let row = sqlx::query_as!(
            EventRow,
            r#"
            UPDATE events SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                subject = COALESCE($4, subject),
                starts_at = COALESCE($5, starts_at),
                duration_min = COALESCE($6, duration_min),
                classroom = COALESCE($7, classroom),
                recurrence_interval = COALESCE($8, recurrence_interval),
                recurrence_end_date = COALESCE($9, recurrence_end_date),
                updated_at = now()
            WHERE id = $1
            RETURNING
                id, title, description, subject, starts_at, duration_min, classroom,
                recurrence_interval AS "recurrence_interval: RecurrenceInterval",
                recurrence_end_date, created_by_email, created_at
            "#,
            id,
            input.title,
            input.description,
            input.subject,
            input.starts_at,
            input.duration_min,
            input.classroom,
            recurrence_interval as Option<RecurrenceInterval>,
            recurrence_end_date,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(row)
    }

    #[tracing::instrument(skip(self), fields(event_id = %id))]
    async fn delete_event(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!("DELETE FROM events WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, groups), fields(event_id = %event_id, subject = %subject))]
    async fn set_event_groups(
        &self,
        event_id: Uuid,
        subject: &str,
        groups: &[String],
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!("DELETE FROM event_groups WHERE event_id = $1", event_id)
            .execute(&mut *tx)
            .await?;

        if !groups.is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO event_groups (event_id, subject, grp)
                SELECT $1, $2, g FROM UNNEST($3::text[]) AS g
                "#,
                event_id,
                subject,
                groups,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(event_id = %event_id))]
    async fn find_groups(&self, event_id: Uuid) -> Result<Vec<String>, AppError> {
        let groups = sqlx::query_scalar!(
            "SELECT grp FROM event_groups WHERE event_id = $1 ORDER BY grp",
            event_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(groups)
    }

    #[tracing::instrument(skip(self, event_ids))]
    async fn find_groups_for_events(
        &self,
        event_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT event_id, grp
            FROM event_groups
            WHERE event_id = ANY($1)
            ORDER BY event_id, grp
            "#,
            event_ids
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.event_id, r.grp)).collect())
    }

    #[tracing::instrument(skip(self), fields(subject = ?subject, search = ?search))]
    async fn list_events(
        &self,
        subject: Option<&str>,
        search: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<EventRowWithTotal>, i64), AppError> {
        let rows = sqlx::query_as!(
            EventRowWithTotal,
            r#"
            SELECT
                id, title, description, subject, starts_at, duration_min, classroom,
                recurrence_interval AS "recurrence_interval: RecurrenceInterval",
                recurrence_end_date, created_by_email, created_at,
                COUNT(*) OVER() AS "total!"
            FROM events
            WHERE
                ($1::text IS NULL OR subject = $1)
                AND ($2::text IS NULL OR title ILIKE '%' || $2 || '%')
            ORDER BY starts_at ASC
            LIMIT $3 OFFSET $4
            "#,
            subject,
            search,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = rows.first().map(|r| r.total).unwrap_or(0);
        Ok((rows, total))
    }

    #[tracing::instrument(skip(self), fields(subject = ?subject))]
    async fn list_occurrences(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        subject: Option<&str>,
    ) -> Result<Vec<OccurrenceRow>, AppError> {
        let rows = sqlx::query_as!(
            OccurrenceRow,
            r#"
            SELECT
                event_id AS "event_id!",
                title AS "title!",
                description,
                subject AS "subject!",
                classroom,
                occurrence_starts_at AS "occurrence_starts_at!",
                duration_min AS "duration_min!",
                is_recurring AS "is_recurring!"
            FROM (
                SELECT
                    e.id AS event_id,
                    e.title,
                    e.description,
                    e.subject,
                    e.classroom,
                    gs.occ AS occurrence_starts_at,
                    e.duration_min,
                    true AS is_recurring
                FROM events e
                CROSS JOIN LATERAL generate_series(
                    e.starts_at,
                    $2::timestamptz,
                    CASE e.recurrence_interval
                        WHEN 'daily' THEN INTERVAL '1 day'
                        WHEN 'weekly' THEN INTERVAL '1 week'
                        WHEN 'biweekly' THEN INTERVAL '2 weeks'
                        WHEN 'monthly' THEN INTERVAL '1 month'
                    END
                ) AS gs(occ)
                WHERE e.recurrence_interval IS NOT NULL
                    AND gs.occ >= $1::timestamptz
                    AND gs.occ < $2::timestamptz
                    AND gs.occ < ((e.recurrence_end_date + 1)::timestamp AT TIME ZONE 'UTC')
                    AND ($3::text IS NULL OR e.subject = $3)

                UNION ALL

                SELECT
                    e.id AS event_id,
                    e.title,
                    e.description,
                    e.subject,
                    e.classroom,
                    e.starts_at AS occurrence_starts_at,
                    e.duration_min,
                    false AS is_recurring
                FROM events e
                WHERE e.recurrence_interval IS NULL
                    AND e.starts_at >= $1::timestamptz
                    AND e.starts_at < $2::timestamptz
                    AND ($3::text IS NULL OR e.subject = $3)
            ) occurrences
            ORDER BY occurrence_starts_at ASC
            "#,
            from,
            to,
            subject,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
