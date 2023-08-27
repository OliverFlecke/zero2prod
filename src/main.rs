use std::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .map(|p| p.parse::<usize>().expect("PORT is not a valid integer"))
        .unwrap_or(4000);
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;

    zero2prod::App::create().serve(listener).await?;

    Ok(())
}
