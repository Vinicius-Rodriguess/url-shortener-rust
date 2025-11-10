use axum::{
    extract::{Path, Extension},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use scylla::{Session, SessionBuilder, FromRow};
use redis::aio::MultiplexedConnection; 
use redis::AsyncCommands;
use rand::{SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use blake3;

pub struct AppState {
    pub redis: MultiplexedConnection,
    pub cassandra: Session,
}

#[derive(FromRow, Debug)]
struct UrlRow {
    long_url: String,
}

#[derive(Serialize, Deserialize)]
struct Url {
    short_url: Option<String>,
    long_url: String,
}

/// Gera o short URL com base62 e ofuscação via secret_key
fn generate_short_url(secret_key: &str, mut id: u64) -> String {
    // Base62 padrão
    let mut alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();

    // Usa o hash da secret_key como semente do RNG
    let mut rng = ChaCha8Rng::from_seed(blake3::hash(secret_key.as_bytes()).into());

    // Embaralha o alfabeto sempre da mesma forma
    alphabet.shuffle(&mut rng);

    // Converte o ID para base62 (usando o alfabeto embaralhado)
    let mut encoded = Vec::new();
    while id > 0 {
        let remainder = (id % 62) as usize;
        encoded.push(alphabet[remainder]);
        id /= 62;
    }

    if encoded.is_empty() {
        encoded.push(alphabet[0]);
    }

    encoded.iter().rev().collect::<String>()
}

// POST /shorten
async fn create_shorten_url(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<Url>,
) -> impl IntoResponse {
    let long_url = payload.long_url;

    // O clone é necessário para que `redis_conn` possa ser mutável para a chamada `incr`.
    let mut redis_conn = state.redis.clone();
    
    // 1. Incrementa o contador global no Redis
    let id: u64 = match redis_conn.incr("url_id", 1).await {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Redis error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Redis Error").into_response();
        }
    };

    // 2. Ajusta o ID (começa com 14 milhões)
    let id_adjusted = id + 14_000_000;

    // 3. Gera o short URL
    let secret_key = std::env::var("SECRET_KEY").unwrap_or_else(|_| "default_secret".to_string());
    let short_url = generate_short_url(&secret_key, id_adjusted);

    // 4. Salva no Cassandra
    let query = "INSERT INTO urls (short_url, long_url) VALUES (?, ?)";
    if let Err(e) = state
        .cassandra
        .query(query, (short_url.clone(), long_url.clone()))
        .await
    {
        eprintln!("Cassandra error: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }

    // 5. Retorna resposta
    let response = Url {
        short_url: Some(short_url),
        long_url,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

// GET /:short_url
async fn redirect_to_long_url(
    Path(short): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let query = "SELECT long_url FROM urls WHERE short_url = ?";

    match state.cassandra.query(query, (short.clone(),)).await {
        Ok(result) => {
            if let Ok(row) = result.single_row_typed::<UrlRow>() {
                println!("Redirecting '{}' -> {}", short, row.long_url);
                return Redirect::to(&row.long_url).into_response();
            }
            (StatusCode::NOT_FOUND, "URL not found").into_response()
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting URL Shortener Service...");

    const REDIS_URL: &str = "redis://redis:6379/";
    const CASSANDRA_NODE: &str = "cassandra:9042";

    // Redis
    let redis_client = redis::Client::open(REDIS_URL)?;
    let redis_conn = redis_client.get_multiplexed_async_connection().await?;

    // Cassandra
    let cassandra = SessionBuilder::new()
        .known_node(CASSANDRA_NODE)
        .build()
        .await?;

    //  Cria o keyspace se não existir
    cassandra
        .query(
            "CREATE KEYSPACE IF NOT EXISTS shortener WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};",
            &[],
        )
        .await?;

    // Usa o keyspace
    cassandra.use_keyspace("shortener", false).await?;

    // Cria a tabela se não existir
    cassandra
        .query(
            "CREATE TABLE IF NOT EXISTS urls (
                short_url text PRIMARY KEY,
                long_url text,
                created_at timestamp
            );",
            &[],
        )
        .await?;

    println!("Connected to Redis and Cassandra (keyspace ready)");

    // Shared state
    let state = Arc::new(AppState {
        redis: redis_conn,
        cassandra,
    });

    // Rotas
    let app = Router::new()
        .route("/shorten", post(create_shorten_url))
        .route("/:short_url", get(redirect_to_long_url))
        .layer(Extension(state));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on http://{}", addr);

    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
