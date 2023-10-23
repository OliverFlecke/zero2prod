use std::time::Duration;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    get_connection_pool,
};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

type PgTransaction = Transaction<'static, Postgres>;

/// Represents the outcomes `try_execute_task` can have.
#[derive(Debug)]
pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

/// Try executing tasks to deliver emails.
#[tracing::instrument(
    skip(pool, email_client),
    ret,
    err,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ))]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let Some((transaction, issue_id, email)) = dequeue_task(pool).await? else {
        return Ok(ExecutionOutcome::EmptyQueue);
    };

    Span::current()
        .record("newsletter_issue_id", &display(&issue_id))
        .record("subscriber_email", &display(&email));

    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_issue(pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.text_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. \
                    Skipping",
                );
            }
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. \
                There stored contact details are invalid"
            );
        }
    }

    delete_task(transaction, issue_id, &email).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

/// Dequeue a task from the newsletter issue delivery queue. If any exists, the
/// db transaction used to fetch the task is returned together with the uuid of
/// the task and the email of the subscriber who should receive the email.
#[tracing::instrument(skip(pool))]
async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    Ok(r.map(|r| (transaction, r.newsletter_issue_id, r.subscriber_email)))
}

/// Delete a task from the issue delievery queue.
#[tracing::instrument(skip(transaction, email))]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1
            AND subscriber_email = $2
        "#,
        issue_id,
        email,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
}

/// Get a newsletter issue from the database.
#[tracing::instrument(skip(pool))]
async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
            SELECT title, text_content
            FROM newsletter_issues
            WHERE newsletter_issue_id = $1
            "#,
        issue_id
    )
    .fetch_one(pool)
    .await?;

    Ok(issue)
}

/// Run a loop to try executing all the tasks in the newsletter issue delievery issue queue.
async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    use tokio::time::sleep;
    loop {
        match try_execute_task(&pool, &email_client).await {
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::EmptyQueue) => {
                sleep(Duration::from_secs(10)).await;
            }
            // Just continue with the next task.
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_worker_until_stopped(config: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&config);
    let email_client = config
        .email_client()
        .try_into()
        .expect("Failed to create email client");

    worker_loop(connection_pool, email_client).await
}
