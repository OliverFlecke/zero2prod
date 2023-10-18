CREATE TABLE newsletter_issues (
    newsletter_issue_id uuid NOT NULL,
    title text NOT NULL,
    text_content text NOT NULL,
    published_at timestamptz NOT NULL,
    PRIMARY KEY (newsletter_issue_id)
);
