use derive_getters::Getters;
use std::net::TcpListener;

#[derive(Debug, Getters)]
pub struct TestApp {
    address: String,
}

/// Spawn a instance of the app on a random port.
pub async fn spawn_app() -> anyhow::Result<TestApp> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let test_app = TestApp {
        address: format!("http://{}", listener.local_addr().unwrap()),
    };

    let server = zero2prod::App::create().serve(listener);
    let _ = tokio::spawn(server);

    Ok(test_app)
}
