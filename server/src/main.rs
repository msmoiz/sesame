mod config;
mod db;

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use sesame_model::{
    ApiError, DeleteSecretInput, DeleteSecretOutput, ErrorInfo, GetSecretInput, GetSecretOutput,
    HealthInput, HealthOutput, ListSecretsInput, ListSecretsOutput, PASSWORD_HEADER,
    PublishSecretInput, PublishSecretOutput,
};
use tokio::time;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// The keyspace for secrets within the database.
const SECRET_KEYSPACE: &str = "secrets";

/// The application state.
#[derive(Clone)]
struct AppState {
    _database: Database,
    secrets: Keyspace,
    password: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    let config = config::Config::load().context("failed to load server config")?;

    let password = config.password;

    let address: SocketAddr = config
        .address
        .parse()
        .context("failed to parse SESAME_ADDRESS")?;

    let flush_interval = Duration::from_secs(config.flush_interval_secs);

    let db_path = PathBuf::from(config.db_path);
    let database = Database::builder(&db_path)
        .open()
        .with_context(|| format!("failed to open database at {}", db_path.display()))?;

    let secrets = database
        .keyspace(SECRET_KEYSPACE, KeyspaceCreateOptions::default)
        .context("failed to open secrets keyspace")?;

    migrate_legacy_secrets(&secrets).context("failed to migrate legacy secrets")?;

    info!("database_path = {}", db_path.display());
    info!("listening_on = {}", address);

    spawn_flush_loop(database.clone(), flush_interval);

    let state = AppState {
        _database: database,
        secrets,
        password,
    };

    let app = Router::new()
        .route("/health", post(health))
        .route("/publish-secret", post(publish_secret))
        .route("/list-secrets", post(list_secrets))
        .route("/get-secret", post(get_secret))
        .route("/delete-secret", post(delete_secret))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, authenticate))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(address).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Initializes the logger.
fn init_logger() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
}

/// Spawns a background thread to flush the database to disk on an interval.
fn spawn_flush_loop(database: Database, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(error) = database.persist(PersistMode::SyncAll) {
                error!(?error, "failed to persist database");
            }
        }
    });
}

/// Registers a shutdown signal.
async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(?error, "failed to install ctrl-c handler");
    }
}

/// Authenticates requests.
///
/// Requests must include a password in the `x-sesame-password` header that
/// matches the password configured for the server.
async fn authenticate(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    match headers
        .get(PASSWORD_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        Some(password) if password == state.password => next.run(request).await,
        Some(_) => ApiResponse::error(StatusCode::UNAUTHORIZED, ApiError::PasswordInvalid),
        None => ApiResponse::error(StatusCode::UNAUTHORIZED, ApiError::PasswordMissing),
    }
}

/// Logs an internal error.
fn log_internal(error: impl std::fmt::Debug) -> ApiError {
    error!(?error, "internal server error");
    ApiError::InternalError
}

/// Migrates legacy secrets that were stored as plain UTF-8 bytes.
fn migrate_legacy_secrets(secrets: &Keyspace) -> Result<()> {
    use sesame_model::Encoding;

    let mut migrated = 0usize;

    for entry in secrets.iter() {
        let (key, value) = entry
            .into_inner()
            .context("failed to read secret entry during migration")?;

        if serde_json::from_slice::<db::Secret>(&value).is_ok() {
            continue;
        }

        let legacy = match String::from_utf8(value.to_vec()) {
            Ok(value) => value,
            Err(_) => {
                warn!(
                    secret = %String::from_utf8_lossy(&key),
                    "skipping secret migration for non-utf8 legacy value"
                );
                continue;
            }
        };

        let migrated_secret = db::Secret {
            value: legacy.into_bytes(),
            encoding: Encoding::Text,
        };

        let raw = serde_json::to_vec(&migrated_secret)
            .context("failed to serialize migrated secret value")?;

        secrets
            .insert(key.to_vec(), raw)
            .context("failed to write migrated secret value")?;

        migrated += 1;
    }

    if migrated > 0 {
        info!(
            migrated,
            "migrated legacy secrets to structured storage format"
        );
    }

    Ok(())
}

/// An API response.
struct ApiResponse;

impl ApiResponse {
    /// Creates a success response.
    fn ok<T>(status: StatusCode, body: T) -> Response
    where
        T: serde::Serialize,
    {
        (status, Json(body)).into_response()
    }

    /// Creates an error response.
    fn error(status: StatusCode, error: ApiError) -> Response {
        let body: ErrorInfo = error.into();
        let mut response = (status, Json(body)).into_response();
        response.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        response
    }
}

/// Returns an OK response if the server is healthy.
async fn health(Json(_input): Json<HealthInput>) -> Response {
    ApiResponse::ok(StatusCode::OK, HealthOutput {})
}

/// Publishes a secret to the store.
async fn publish_secret(
    State(state): State<AppState>,
    Json(input): Json<PublishSecretInput>,
) -> Response {
    if input.name.trim().is_empty() {
        return ApiResponse::error(StatusCode::BAD_REQUEST, ApiError::InvalidSecretName);
    }

    match publish_secret_inner(&state, input) {
        Ok(output) => ApiResponse::ok(StatusCode::CREATED, output),
        Err(error) => {
            let status = match error {
                ApiError::InvalidSecretName => StatusCode::BAD_REQUEST,
                ApiError::SecretAlreadyExists => StatusCode::CONFLICT,
                ApiError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            };
            ApiResponse::error(status, error)
        }
    }
}

fn publish_secret_inner(
    state: &AppState,
    input: PublishSecretInput,
) -> Result<PublishSecretOutput, ApiError> {
    if state
        .secrets
        .contains_key(input.name.as_bytes())
        .map_err(log_internal)?
    {
        return Err(ApiError::SecretAlreadyExists);
    }

    let secret = db::Secret {
        value: input.value.into_bytes(),
        encoding: input.encoding,
    };

    let raw = serde_json::to_vec(&secret).map_err(log_internal)?;

    state
        .secrets
        .insert(input.name.as_bytes(), raw)
        .map_err(log_internal)?;

    Ok(PublishSecretOutput {})
}

/// Lists secrets in the store.
async fn list_secrets(
    State(state): State<AppState>,
    Json(_input): Json<ListSecretsInput>,
) -> Response {
    match list_secrets_inner(&state) {
        Ok(output) => ApiResponse::ok(StatusCode::OK, output),
        Err(error) => ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

fn list_secrets_inner(state: &AppState) -> Result<ListSecretsOutput, ApiError> {
    let mut secrets = state
        .secrets
        .iter()
        .map(|entry| {
            let key = entry.key().map_err(log_internal)?;
            String::from_utf8(key.to_vec()).map_err(|_| ApiError::InternalError)
        })
        .collect::<Result<Vec<_>, _>>()?;

    secrets.sort();

    Ok(ListSecretsOutput { secrets })
}

/// Gets a secret from the store.
async fn get_secret(State(state): State<AppState>, Json(input): Json<GetSecretInput>) -> Response {
    match get_secret_inner(&state, input) {
        Ok(output) => ApiResponse::ok(StatusCode::OK, output),
        Err(error) => {
            let status = match error {
                ApiError::SecretNotFound => StatusCode::NOT_FOUND,
                ApiError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            };
            ApiResponse::error(status, error)
        }
    }
}

fn get_secret_inner(state: &AppState, input: GetSecretInput) -> Result<GetSecretOutput, ApiError> {
    let Some(value) = state
        .secrets
        .get(input.name.as_bytes())
        .map_err(log_internal)?
    else {
        return Err(ApiError::SecretNotFound);
    };

    let secret: db::Secret =
        serde_json::from_slice(&value.to_vec()).map_err(|_| ApiError::InternalError)?;

    Ok(GetSecretOutput {
        name: input.name,
        value: String::from_utf8(secret.value).map_err(log_internal)?,
        encoding: secret.encoding,
    })
}

/// Deletes a secret from the store.
async fn delete_secret(
    State(state): State<AppState>,
    Json(input): Json<DeleteSecretInput>,
) -> Response {
    match delete_secret_inner(&state, input) {
        Ok(output) => ApiResponse::ok(StatusCode::OK, output),
        Err(error) => {
            let status = match error {
                ApiError::SecretNotFound => StatusCode::NOT_FOUND,
                ApiError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            };
            ApiResponse::error(status, error)
        }
    }
}

fn delete_secret_inner(
    state: &AppState,
    input: DeleteSecretInput,
) -> Result<DeleteSecretOutput, ApiError> {
    state.secrets.remove(input.name).map_err(log_internal)?;
    Ok(DeleteSecretOutput {})
}
