use axum::{Extension, Router};
use axum::http::{header, Method};
use axum_login::AuthManagerLayerBuilder;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::tower_sessions::cookie::time::Duration;
use sqlx::{Pool, Postgres};
use tower_http::cors::CorsLayer;
use tower_sessions_sqlx_store::PostgresStore;
use app_core::database::SqlxManager;

use common::settings::{AppSettings, Protocol};
use crate::app::auth::AuthGate;

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    pub settings: AppSettings,
    pool: Pool<Postgres>
}

pub struct Server {
    pub state: AppState,
}

impl Server {
    pub async fn new(settings: &str) -> Self {
        let settings = AppSettings::new(settings);
        let pool = SqlxManager::new(&settings.database)
            .await;

        Self {
            state: AppState {
                settings,
                pool: pool.pool
            }
        }
    }
    
    pub fn router(state: AppState) -> Router {
        let gate = AuthGate::new(state.pool.clone());
        
        Router::new()
            .merge(routes::router())
            .layer(Self::configure_cors(&state))
            .layer(
                AuthManagerLayerBuilder::new(gate, Self::configure_session(&state))
                    .build()
            )
            .layer(Extension(state))
    }

    pub async fn run(self) {
        log::info!("Starting axum server with tokio...");

        let listener = tokio::net::TcpListener::bind(self.state.settings.web.cdn.url())
            .await
            .unwrap();
        axum::serve(listener, Self::router(self.state))
            .await
            .expect("Failed to start axum server");
    }

    fn configure_cors(state: &AppState) -> CorsLayer {
        CorsLayer::new()
            .allow_credentials(true)
            .allow_origin([
                state.settings.web.client.protocol_url().parse()
                    .unwrap()
            ])
            .allow_headers([
                header::CONTENT_TYPE
            ])
            .allow_methods([
                Method::GET, Method::PUT,
                Method::DELETE, Method::PATCH,
            ])
    }

    fn configure_session(state: &AppState) -> SessionManagerLayer<PostgresStore> {
        let session_store = PostgresStore::new(state.pool.clone())
            .with_table_name("sessions")
            .unwrap();

        SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_name("session_id")
            .with_domain(state.settings.web.rest.host.clone())
            .with_secure(false)
    }
}