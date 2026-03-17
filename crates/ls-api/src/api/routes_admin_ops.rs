async fn admin_cleanup_storage_cache(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminCleanupStorageCacheRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .cleanup_storage_cache(payload.max_age_seconds)
        .await
    {
        Ok(removed) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.storage_cache.cleanup",
                    "storage_cache",
                    None,
                    json!({"removed": removed, "max_age_seconds": payload.max_age_seconds}),
                )
                .await;
            Json(json!({"removed": removed})).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to cleanup storage cache: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminInvalidateStorageCacheRequest {
    stream_url: String,
}

async fn admin_invalidate_storage_cache(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminInvalidateStorageCacheRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.stream_url.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "stream_url is required");
    }

    match state
        .infra
        .invalidate_cached_stream(payload.stream_url.trim())
        .await
    {
        Ok(invalidated) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.storage_cache.invalidate",
                    "storage_cache",
                    None,
                    json!({"stream_url": payload.stream_url.trim(), "invalidated": invalidated}),
                )
                .await;
            Json(json!({"invalidated": invalidated})).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to invalidate cache entry: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminAuditQuery {
    limit: Option<i64>,
}

async fn admin_list_audit_logs(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminAuditQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    match state.infra.list_audit_logs(limit).await {
        Ok(logs) => Json(logs).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list audit logs: {err}"),
        ),
    }
}

async fn admin_export_audit_logs_csv(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminAuditQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(1000).clamp(1, 5000);
    let logs = match state.infra.list_audit_logs(limit).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list audit logs: {err}"),
            );
        }
    };

    let mut csv = String::from("created_at,actor_username,action,target_type,target_id,detail\n");
    for log in logs {
        let detail = log
            .detail
            .to_string()
            .replace('"', "\"\"")
            .replace('\n', " ")
            .replace('\r', " ");
        let actor = log.actor_username.unwrap_or_default();
        let target_id = log.target_id.unwrap_or_default();
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            log.created_at.to_rfc3339(),
            actor,
            log.action,
            log.target_type,
            target_id,
            detail,
        ));
    }

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/csv; charset=utf-8"))
        .insert_header((
            header::CONTENT_DISPOSITION,
            "attachment; filename=admin-audit-logs.csv",
        ))
        .body(csv)
}

