use axum::{routing::get, Router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    println!("Iniciando o servidor...");

    async fn root_handler() -> &'static str {
        "Servidor em execução!"
    }

    let app = Router::new().route("/", get(root_handler));

    let endereco = "0.0.0.0:3000";
    println!("Servidor escutando em http://{}", endereco);

    let listener = TcpListener::bind(endereco).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
