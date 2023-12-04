use super::IdempotencyKey;
use axum::{
    body::{to_bytes, Body},
    response::{IntoResponse, Response},
};
use http::{HeaderName, StatusCode};
use sqlx::{postgres::PgHasArrayType, PgPool, Postgres, Transaction};
use uuid::Uuid;

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

/// Attempt to process a idempotency response.
#[tracing::instrument(name = "Try processing idempotency")]
pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let n_inserted_rows = sqlx::query!(
        r#"INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING"#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we did not find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}

/// Get saved HTTP responses from the database.
#[tracing::instrument(name = "Get saved idempotency responses", skip(pool))]
pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"SELECT
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE user_id = $1 AND idempotency_key = $2"#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut response = Response::builder()
            .status(status_code)
            .body(Body::from(r.response_body))?;

        for HeaderPairRecord { name, value } in r.response_headers {
            response
                .headers_mut()
                .append(HeaderName::try_from(name)?, value.try_into()?);
        }

        Ok(Some(response.into_response()))
    } else {
        Ok(None)
    }
}

/// Save a HTTP response for a given user and idempotency key
#[tracing::instrument(
    name = "Save idempotency key with response",
    skip(transaction, http_response)
)]
pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
    http_response: Response,
) -> Result<Response, anyhow::Error> {
    let (response_head, body) = http_response.into_parts();
    // TODO: usize::MAX is not the right thing to use here.
    let body = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let status_code = response_head.status.as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers.len());
        for (name, value) in response_head.headers.iter() {
            let name = name.as_str().to_owned();
            let value = value.to_str()?.to_owned().into_bytes();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    // TODO: SQL query
    sqlx::query_unchecked!(
        r#"UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1
            AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok((response_head, Body::from(body)).into_response())
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}
