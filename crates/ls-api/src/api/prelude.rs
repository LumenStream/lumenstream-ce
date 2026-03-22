use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    net::IpAddr,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use actix_cors::Cors;
use actix_web::{
    Error, FromRequest, HttpRequest, HttpResponse,
    body::{BoxBody, MessageBody, SizedStream},
    dev::{HttpServiceFactory, Payload, ServiceRequest, ServiceResponse},
    http::{self, StatusCode, header},
    middleware::{self, Next},
    web,
};
use tokio_util::io::ReaderStream;
use chrono::{DateTime, NaiveDate, Utc};
use cron::Schedule;
use ls_agent::AgentRequestCreateInput;
use ls_config::{AgentConfig, BillingConfig, EpayConfig, InviteConfig, SecurityConfig, WebAppConfig};
use ls_domain::{
    jellyfin::{
        AuthenticateByNameRequest, BaseItemDto, CreateUserByName, PlaybackProgressDto,
        PublicSystemInfoDto, QueryResultDto, SessionInfoDto, SystemInfoDto, UpdateUserPassword,
        UserConfiguration, UserDto, UserPolicyUpdate, WakeOnLanInfoDto,
    },
    model::UserRole,
};
use ls_infra::{
    AccountPermissionGroupUpsert, AdminUserSummaryQuery, AgentRequestListQuery,
    AgentReviewRequest, AppInfra, AuthenticateUserResult, BillingPlanUpsert,
    BillingRechargeOrderFilter, InfraError, ItemsQuery as InfraItemsQuery, PasswordCheckResult,
    PlaybackDomainUpdate, PlaylistUpdate, LumenBackendNodeHeartbeat, LumenBackendNodeRegister,
    StreamAccessDeniedReason, TaskDefinitionUpdate, TopPlayedMediaSummary, UserProfileUpdate,
    UserStreamPolicyUpdate,
};
use ls_logging::LogHandle;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use tracing::{Instrument, error, info, warn};
use uuid::Uuid;

struct ApiMetrics {
    started_at: Instant,
    requests_total: AtomicU64,
    auth_failures: AtomicU64,
    status_2xx: AtomicU64,
    status_4xx: AtomicU64,
    status_5xx: AtomicU64,
    stream_attempts_total: AtomicU64,
    stream_success_total: AtomicU64,
    stream_failures_total: AtomicU64,
    stream_fallback_total: AtomicU64,
    stream_upstream_total: AtomicU64,
    stream_cache_hit_total: AtomicU64,
    stream_cache_miss_total: AtomicU64,
    latencies_ms: Mutex<VecDeque<u64>>,
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
            requests_total: AtomicU64::new(0),
            auth_failures: AtomicU64::new(0),
            status_2xx: AtomicU64::new(0),
            status_4xx: AtomicU64::new(0),
            status_5xx: AtomicU64::new(0),
            stream_attempts_total: AtomicU64::new(0),
            stream_success_total: AtomicU64::new(0),
            stream_failures_total: AtomicU64::new(0),
            stream_fallback_total: AtomicU64::new(0),
            stream_upstream_total: AtomicU64::new(0),
            stream_cache_hit_total: AtomicU64::new(0),
            stream_cache_miss_total: AtomicU64::new(0),
            latencies_ms: Mutex::new(VecDeque::with_capacity(2048)),
        }
    }
}

impl ApiMetrics {
    fn record_latency(&self, duration_ms: u64) {
        const WINDOW: usize = 4096;
        let mut guard = self
            .latencies_ms
            .lock()
            .expect("latency metrics mutex poisoned");
        if guard.len() >= WINDOW {
            guard.pop_front();
        }
        guard.push_back(duration_ms);
    }

    fn snapshot(&self) -> Value {
        let requests_total = self.requests_total.load(Ordering::Relaxed);
        let status_5xx = self.status_5xx.load(Ordering::Relaxed);
        let uptime_seconds = self.started_at.elapsed().as_secs_f64().max(1.0);
        let qps = requests_total as f64 / uptime_seconds;
        let error_rate = if requests_total == 0 {
            0.0
        } else {
            status_5xx as f64 / requests_total as f64
        };

        let mut latency_samples = {
            let guard = self
                .latencies_ms
                .lock()
                .expect("latency metrics mutex poisoned");
            guard.iter().copied().collect::<Vec<_>>()
        };
        latency_samples.sort_unstable();
        let latency_p95_ms = percentile_from_sorted(&latency_samples, 0.95);
        let latency_p99_ms = percentile_from_sorted(&latency_samples, 0.99);

        let stream_attempts = self.stream_attempts_total.load(Ordering::Relaxed);
        let stream_success = self.stream_success_total.load(Ordering::Relaxed);
        let playback_success_rate = if stream_attempts == 0 {
            0.0
        } else {
            stream_success as f64 / stream_attempts as f64
        };

        let cache_hits = self.stream_cache_hit_total.load(Ordering::Relaxed);
        let cache_misses = self.stream_cache_miss_total.load(Ordering::Relaxed);
        let cache_total = cache_hits + cache_misses;
        let cache_hit_rate = if cache_total == 0 {
            0.0
        } else {
            cache_hits as f64 / cache_total as f64
        };

        json!({
            "requests_total": requests_total,
            "qps": qps,
            "error_rate": error_rate,
            "latency_p95_ms": latency_p95_ms,
            "latency_p99_ms": latency_p99_ms,
            "auth_failures": self.auth_failures.load(Ordering::Relaxed),
            "status_2xx": self.status_2xx.load(Ordering::Relaxed),
            "status_4xx": self.status_4xx.load(Ordering::Relaxed),
            "status_5xx": status_5xx,
            "playback_stream_attempts_total": stream_attempts,
            "playback_stream_success_total": stream_success,
            "playback_stream_failures_total": self.stream_failures_total.load(Ordering::Relaxed),
            "playback_stream_fallback_total": self.stream_fallback_total.load(Ordering::Relaxed),
            "playback_stream_upstream_total": self.stream_upstream_total.load(Ordering::Relaxed),
            "playback_success_rate": playback_success_rate,
            "cache_hits_total": cache_hits,
            "cache_misses_total": cache_misses,
            "cache_hit_rate": cache_hit_rate,
        })
    }
}

#[derive(Clone)]
pub struct ApiContext {
    pub infra: Arc<AppInfra>,
    pub log_handle: Option<Arc<LogHandle>>,
    metrics: Arc<ApiMetrics>,
}

impl ApiContext {
    pub fn new(infra: Arc<AppInfra>) -> Self {
        Self {
            infra,
            log_handle: None,
            metrics: Arc::new(ApiMetrics::default()),
        }
    }

    pub fn with_log_handle(mut self, log_handle: Arc<LogHandle>) -> Self {
        self.log_handle = Some(log_handle);
        self
    }
}

type Response = HttpResponse;

const INTERNAL_CLIENT_IP_HEADER: &str = "x-ls-client-ip";

trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        HttpResponse::build(self).finish()
    }
}

#[derive(Debug)]
struct Json<T>(T);

impl<T> Json<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for Json<T>
where
    T: serde::de::DeserializeOwned + std::any::Any,
{
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = web::Json::<T>::from_request(req, payload);
        Box::pin(async move { fut.await.map(|json| Self(json.into_inner())) })
    }
}

#[derive(Debug)]
struct Query<T>(T);

impl<T> std::ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for Query<T>
where
    T: serde::de::DeserializeOwned + std::any::Any,
{
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = web::Query::<T>::from_request(req, payload);
        Box::pin(async move { fut.await.map(|query| Self(query.into_inner())) })
    }
}

#[derive(Debug)]
struct AxPath<T>(T);

impl<T> std::ops::Deref for AxPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for AxPath<T>
where
    T: serde::de::DeserializeOwned + std::any::Any,
{
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = web::Path::<T>::from_request(req, payload);
        Box::pin(async move { fut.await.map(|path| Self(path.into_inner())) })
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        HttpResponse::Ok().json(self.into_inner())
    }
}

impl<T> IntoResponse for (StatusCode, Json<T>)
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let (status, body) = self;
        HttpResponse::build(status).json(body.into_inner())
    }
}

#[derive(Clone)]
struct State<T>(T);

impl<T> FromRequest for State<T>
where
    T: Clone + std::any::Any,
{
    type Error = Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match req.app_data::<web::Data<T>>() {
            Some(state) => std::future::ready(Ok(Self(state.get_ref().clone()))),
            None => std::future::ready(Err(actix_web::error::ErrorInternalServerError(
                "missing api state",
            ))),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct HeaderMap(http::header::HeaderMap);

impl HeaderMap {
    #[cfg(test)]
    fn new() -> Self {
        Self(http::header::HeaderMap::new())
    }
}

impl std::ops::Deref for HeaderMap {
    type Target = http::header::HeaderMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HeaderMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromRequest for HeaderMap {
    type Error = Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        std::future::ready(Ok(Self(req.headers().clone())))
    }
}

#[derive(Clone, Debug)]
struct Uri(http::Uri);

impl std::ops::Deref for Uri {
    type Target = http::Uri;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for Uri {
    type Err = http::uri::InvalidUri;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<http::Uri>().map(Self)
    }
}

impl FromRequest for Uri {
    type Error = Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        std::future::ready(Ok(Self(req.uri().clone())))
    }
}

fn build_cors(state: &ApiContext) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers([
            "Authorization",
            "Content-Type",
            "Accept",
            "X-Emby-Token",
            "X-Emby-Client",
            "X-Emby-Device-Name",
            "X-Emby-Device-Id",
            "X-Api-Key",
            "X-Request-Id",
            "Range",
        ]);

    if state.infra.config_snapshot().server.cors_allow_origins.is_empty() {
        cors = cors.allow_any_origin();
    } else {
        for origin in &state.infra.config_snapshot().server.cors_allow_origins {
            if header::HeaderValue::from_str(origin).is_ok() {
                cors = cors.allowed_origin(origin);
            }
        }
    }

    cors
}
