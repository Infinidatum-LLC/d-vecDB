use vectordb_vectorstore::VectorStore;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    fn to_http_status(&self) -> StatusCode {
        match self {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still accepting traffic
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

/// Component health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub timestamp: u64,
}

impl ComponentHealth {
    fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            timestamp: Self::current_timestamp(),
        }
    }

    fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            timestamp: Self::current_timestamp(),
        }
    }

    fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            timestamp: Self::current_timestamp(),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Overall health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ComponentHealth>>,
}

impl HealthResponse {
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Readiness check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub timestamp: u64,
    pub checks: Vec<ComponentHealth>,
}

/// Liveness check response (simple)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessResponse {
    pub alive: bool,
    pub timestamp: u64,
}

type AppState = Arc<VectorStore>;

// Track server start time (global static)
lazy_static::lazy_static! {
    static ref SERVER_START_TIME: u64 = {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
}

/// Basic health check endpoint (Kubernetes liveness probe)
///
/// This endpoint checks if the server is alive and responding.
/// Returns 200 OK if the server is running.
///
/// **Kubernetes Usage:**
/// ```yaml
/// livenessProbe:
///   httpGet:
///     path: /health
///     port: 8080
///   initialDelaySeconds: 10
///   periodSeconds: 10
///   timeoutSeconds: 5
///   failureThreshold: 3
/// ```
#[instrument]
pub async fn health_liveness() -> Result<Json<LivenessResponse>, StatusCode> {
    let response = LivenessResponse {
        alive: true,
        timestamp: ComponentHealth::current_timestamp(),
    };

    Ok(Json(response))
}

/// Readiness check endpoint (Kubernetes readiness probe)
///
/// This endpoint checks if the server is ready to accept traffic.
/// It performs basic checks on critical components.
///
/// **Kubernetes Usage:**
/// ```yaml
/// readinessProbe:
///   httpGet:
///     path: /ready
///     port: 8080
///   initialDelaySeconds: 5
///   periodSeconds: 5
///   timeoutSeconds: 3
///   successThreshold: 1
///   failureThreshold: 3
/// ```
#[instrument(skip(state))]
pub async fn health_readiness(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ReadinessResponse>), StatusCode> {
    let mut checks = Vec::new();
    let mut ready = true;

    // Check 1: Vector store is accessible
    let collections = state.list_collections();
    checks.push(ComponentHealth::healthy("vectorstore"));

    // Check 2: Can get server stats (tests database access)
    match state.get_server_stats().await {
        Ok(_) => {
            checks.push(ComponentHealth::healthy("database"));
        }
        Err(e) => {
            warn!("Database readiness check failed: {}", e);
            checks.push(ComponentHealth::unhealthy("database", e.to_string()));
            ready = false;
        }
    }

    // Check 3: Memory availability (basic check)
    if let Ok(stats) = state.get_server_stats().await {
        if stats.memory_usage > 0 {
            checks.push(ComponentHealth::healthy("memory"));
        } else {
            checks.push(ComponentHealth::degraded("memory", "Unable to determine memory usage"));
        }
    }

    let response = ReadinessResponse {
        ready,
        timestamp: ComponentHealth::current_timestamp(),
        checks,
    };

    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(response)))
}

/// Comprehensive health check endpoint
///
/// This endpoint performs deep health checks on all components.
/// Use this for monitoring and diagnostics, not for load balancer checks.
///
/// **Returns:**
/// - 200 OK: All components healthy
/// - 200 OK: Some components degraded (still operational)
/// - 503 Service Unavailable: Critical components unhealthy
#[instrument(skip(state))]
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<HealthResponse>), StatusCode> {
    let mut components = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    // Component 1: Vector Store Engine
    match state.list_collections() {
        collections => {
            let collection_count = collections.len();
            components.push(ComponentHealth {
                name: "vectorstore".to_string(),
                status: HealthStatus::Healthy,
                message: Some(format!("{} collections", collection_count)),
                timestamp: ComponentHealth::current_timestamp(),
            });
        }
    }

    // Component 2: Database & Storage
    match state.get_server_stats().await {
        Ok(stats) => {
            let message = format!(
                "{} vectors across {} collections, {:.2} MB memory",
                stats.total_vectors,
                stats.total_collections,
                stats.memory_usage as f64 / 1_024_000.0
            );

            components.push(ComponentHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                message: Some(message),
                timestamp: ComponentHealth::current_timestamp(),
            });
        }
        Err(e) => {
            error!("Database health check failed: {}", e);
            components.push(ComponentHealth::unhealthy("database", e.to_string()));
            overall_status = HealthStatus::Unhealthy;
        }
    }

    // Component 3: Memory Usage
    if let Ok(stats) = state.get_server_stats().await {
        let memory_mb = stats.memory_usage as f64 / 1_024_000.0;

        // Check if memory usage is reasonable (< 8GB for example)
        let status = if memory_mb < 8000.0 {
            HealthStatus::Healthy
        } else if memory_mb < 12000.0 {
            if overall_status == HealthStatus::Healthy {
                overall_status = HealthStatus::Degraded;
            }
            HealthStatus::Degraded
        } else {
            overall_status = HealthStatus::Unhealthy;
            HealthStatus::Unhealthy
        };

        components.push(ComponentHealth {
            name: "memory".to_string(),
            status,
            message: Some(format!("{:.2} MB used", memory_mb)),
            timestamp: ComponentHealth::current_timestamp(),
        });
    }

    // Component 4: Uptime
    let uptime = ComponentHealth::current_timestamp() - *SERVER_START_TIME;
    components.push(ComponentHealth {
        name: "uptime".to_string(),
        status: HealthStatus::Healthy,
        message: Some(format!("{} seconds", uptime)),
        timestamp: ComponentHealth::current_timestamp(),
    });

    let response = HealthResponse {
        status: overall_status,
        timestamp: ComponentHealth::current_timestamp(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        components: Some(components),
    };

    let status_code = overall_status.to_http_status();

    info!(
        "Health check completed: status={:?}, components={}",
        overall_status,
        response.components.as_ref().map(|c| c.len()).unwrap_or(0)
    );

    Ok((status_code, Json(response)))
}

/// Simple health endpoint (backward compatibility)
///
/// Kept for backward compatibility. Use /health/live for liveness checks.
#[instrument]
pub async fn health_simple() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": "OK",
        "error": null
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_http_codes() {
        assert_eq!(HealthStatus::Healthy.to_http_status(), StatusCode::OK);
        assert_eq!(HealthStatus::Degraded.to_http_status(), StatusCode::OK);
        assert_eq!(HealthStatus::Unhealthy.to_http_status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy("test");
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.name, "test");
        assert!(healthy.message.is_none());

        let degraded = ComponentHealth::degraded("test", "warning");
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert_eq!(degraded.message, Some("warning".to_string()));

        let unhealthy = ComponentHealth::unhealthy("test", "error");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, Some("error".to_string()));
    }
}
