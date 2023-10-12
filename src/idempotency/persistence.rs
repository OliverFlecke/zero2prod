use super::IdempotencyKey;
use axum::{
    body::Full,
    response::{IntoResponse, Response},
};
use http::{HeaderName, StatusCode};
use hyper::body::{to_bytes, Bytes};
use sqlx::{postgres::PgHasArrayType, PgPool};
use uuid::Uuid;

/// Get saved HTTP responses from the database.
#[tracing::instrument(name = "Get saved idempotency responses", skip(pool))]
pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<Option<Response<Full<Bytes>>>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"SELECT
            response_status_code,
            response_headers as "response_headers: Vec<HeaderPairRecord>",
            response_body
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
            .body(Full::new(Bytes::from(r.response_body)))?;

        for HeaderPairRecord { name, value } in r.response_headers {
            response
                .headers_mut()
                .append(HeaderName::try_from(name)?, value.try_into()?);
        }

        Ok(Some(response))
    } else {
        Ok(None)
    }
}

/// Save a HTTP response for a given user and idempotency key
#[tracing::instrument(name = "Save idempotency key with response", skip(pool, http_response))]
pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
    http_response: Response,
) -> Result<Response, anyhow::Error> {
    let (response_head, body) = http_response.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{}", e))?;
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
        r#"INSERT INTO idempotency (
            user_id,
            idempotency_key,
            response_status_code,
            response_headers,
            response_body,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, now())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(pool)
    .await?;

    Ok((response_head, Full::new(body)).into_response())
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
