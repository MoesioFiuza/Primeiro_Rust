use axum::{
    extract::{Path, State}, // Importe State e Path (vamos adicionar uma rota GET para usuários)
    routing::{get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;

use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<u32, User>>>,
    next_user_id: Arc<Mutex<u32>>,
}

#[derive(Deserialize)]
struct CreateUser {
    nome: String,
    idade: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u32,
    nome: String,
    idade: u32,
}

async fn create_user(State(state): State<AppState>, Json(payload): Json<CreateUser>) -> Json<User> {
    let mut users_lock = state.users.lock().await;
    let mut next_id_lock = state.next_user_id.lock().await;

    let new_id = *next_id_lock;
    *next_id_lock += 1;

    let new_user = User {
        id: new_id,
        nome: payload.nome,
        idade: payload.idade,
    };

    debug!("Criando usuário: {:?}", new_user);

    users_lock.insert(new_id, new_user.clone());

    Json(new_user)
}

async fn get_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users_lock = state.users.lock().await;
    let all_users: Vec<User> = users_lock.values().cloned().collect();

    debug!(
        "Retornando todos os usuários: {} encontrados",
        all_users.len()
    ); // Log

    Json(all_users)
}

async fn get_user_by_id(
    State(state): State<AppState>,
    Path(user_id): Path<u32>, // Extrai o ID do path
) -> Result<Json<User>, String> {
    // Usa Result para indicar sucesso ou falha
    let users_lock = state.users.lock().await; // Bloqueia o Mutex

    // Tenta encontrar o usuário no HashMap
    if let Some(user) = users_lock.get(&user_id) {
        debug!("Usuário {} encontrado", user_id);
        Ok(Json(user.clone())) // Retorna sucesso com o usuário (precisa de clone)
    } else {
        debug!("Usuário {} não encontrado", user_id);
        Err(format!("Usuário com ID {} não encontrado", user_id)) // Retorna erro (Axum converte String para 500 por padrão, podemos melhorar isso depois)
                                                                  // Para retornar 404, precisaria de tratamento de erro mais robusto (tópico 4 da sugestão anterior)
    }
}

#[tokio::main]
async fn main() {
    // --- Configuração do Tracing/Logging ---
    tracing_subscriber::registry()
        .with(
            // Configura o filtro de log. Lê da variável de ambiente RUST_LOG,
            // ou usa INFO para nosso app, DEBUG para tower_http e TRACE para rejeições do Axum
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive("meu_servidor_axum=debug".parse().unwrap()) // Log DEBUG para nosso crate
                .add_directive("tower_http=debug".parse().unwrap()) // Log DEBUG para middleware
                .add_directive("axum::rejection=trace".parse().unwrap()), // Log TRACE para rejeições
        )
        .with(tracing_subscriber::fmt::layer()) // Formata os logs para saída no console
        .init(); // Inicializa o subscriber

    info!("Iniciando o servidor..."); // Use info para logs de nível informativo

    // --- Inicialização do Estado da Aplicação ---
    let shared_state = AppState {
        users: Arc::new(Mutex::new(HashMap::new())), // Inicia HashMap vazio
        next_user_id: Arc::new(Mutex::new(1)),       // Começa IDs a partir de 1
    };

    // --- Configuração do Router com Rotas, Middleware e Estado ---
    let app = Router::new()
        .route("/", get(|| async { "Servidor em execução!" }))
        .route("/users", post(create_user).get(get_users)) // GET e POST na mesma rota /users
        .route("/users/:id", get(get_user_by_id)) // Nova rota GET com parâmetro de path
        // Adiciona o middleware de tracing para TODAS as requisições
        .layer(TraceLayer::new_for_http())
        // Adiciona o estado compartilhado para TODAS as rotas
        .with_state(shared_state);

    let endereco = "0.0.0.0:3000";
    info!("Servidor escutando em http://{}", endereco); // Use info para logs

    let listener = TcpListener::bind(endereco).await.unwrap();

    // O axum::serve automaticamente usa o runtime tokio que configuramos com #[tokio::main]
    axum::serve(listener, app).await.unwrap();
}
