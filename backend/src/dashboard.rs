use crate::db::Database;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use warp::Filter;

const INDEX_HTML: &str = include_str!("../web/index.html");

fn json_error(message: impl ToString) -> warp::reply::Json {
    warp::reply::json(&json!({
        "status": "error",
        "message": message.to_string()
    }))
}

fn json_success(data: Value) -> warp::reply::Json {
    warp::reply::json(&json!({
        "status": "success",
        "data": data
    }))
}

pub async fn start(database: Arc<Database>, host: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let db_filter = warp::any().map(move || database.clone());

    let index = warp::path::end()
        .and(warp::get())
        .map(|| warp::reply::html(INDEX_HTML));

    let health = warp::path!("api" / "dashboard" / "health")
        .and(warp::get())
        .map(|| warp::reply::json(&json!({"status": "ok"})));

    let summary = warp::path!("api" / "dashboard" / "summary")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(handle_summary);

    let clients = warp::path!("api" / "dashboard" / "clients")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(handle_clients);

    let rules = warp::path!("api" / "dashboard" / "rules")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(handle_rules);

    let nodes = warp::path!("api" / "dashboard" / "nodes")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(handle_nodes);

    let client_rules = warp::path!("api" / "dashboard" / "client-rules")
        .and(warp::get())
        .and(db_filter)
        .and_then(handle_client_rules);

    let routes = index
        .or(health)
        .or(summary)
        .or(clients)
        .or(rules)
        .or(nodes)
        .or(client_rules)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_methods(vec!["GET"]),
        );

    let socket_addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("无效的 Dashboard 地址: {}", e))?;

    println!("Web Dashboard 已启动: http://{}:{}/", host, port);
    println!("  - Dashboard 不启用认证，请仅在可信 LAN/ZeroTier 网络中开放此端口");

    warp::serve(routes).run(socket_addr).await;
    Ok(())
}

async fn handle_summary(db: Arc<Database>) -> Result<warp::reply::Json, Infallible> {
    let result = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS connections,
            COALESCE(SUM(conn_download), 0) AS download,
            COALESCE(SUM(conn_upload), 0) AS upload
        FROM connections
        "#,
    )
    .fetch_one(&db.pool)
    .await;

    match result {
        Ok(row) => {
            let connections: i64 = row.get("connections");
            let download: i64 = row.get("download");
            let upload: i64 = row.get("upload");
            Ok(json_success(json!({
                "connections": connections,
                "download": download,
                "upload": upload,
                "total": download + upload
            })))
        }
        Err(e) => Ok(json_error(e)),
    }
}

async fn handle_clients(db: Arc<Database>) -> Result<warp::reply::Json, Infallible> {
    let result = sqlx::query(
        r#"
        SELECT
            COALESCE(NULLIF(source_ip, ''), 'unknown') AS source_ip,
            COUNT(*) AS connections,
            COALESCE(SUM(conn_download), 0) AS download,
            COALESCE(SUM(conn_upload), 0) AS upload
        FROM connections
        GROUP BY COALESCE(NULLIF(source_ip, ''), 'unknown')
        ORDER BY (COALESCE(SUM(conn_download), 0) + COALESCE(SUM(conn_upload), 0)) DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&db.pool)
    .await;

    match result {
        Ok(rows) => {
            let data: Vec<Value> = rows
                .into_iter()
                .map(|row| {
                    let source_ip: String = row.get("source_ip");
                    let connections: i64 = row.get("connections");
                    let download: i64 = row.get("download");
                    let upload: i64 = row.get("upload");
                    json!({
                        "source_ip": source_ip,
                        "connections": connections,
                        "download": download,
                        "upload": upload,
                        "total": download + upload
                    })
                })
                .collect();
            Ok(json_success(Value::Array(data)))
        }
        Err(e) => Ok(json_error(e)),
    }
}

async fn handle_rules(db: Arc<Database>) -> Result<warp::reply::Json, Infallible> {
    let result = sqlx::query(
        r#"
        SELECT
            COALESCE(NULLIF(rule, ''), 'UNKNOWN') AS rule,
            COALESCE(rule_payload, '') AS rule_payload,
            COUNT(*) AS connections,
            COALESCE(SUM(conn_download), 0) AS download,
            COALESCE(SUM(conn_upload), 0) AS upload
        FROM connections
        GROUP BY rule, rule_payload
        ORDER BY (COALESCE(SUM(conn_download), 0) + COALESCE(SUM(conn_upload), 0)) DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&db.pool)
    .await;

    match result {
        Ok(rows) => {
            let data: Vec<Value> = rows
                .into_iter()
                .map(|row| rule_row_to_json(&row, None))
                .collect();
            Ok(json_success(Value::Array(data)))
        }
        Err(e) => Ok(json_error(e)),
    }
}

async fn handle_client_rules(db: Arc<Database>) -> Result<warp::reply::Json, Infallible> {
    let result = sqlx::query(
        r#"
        SELECT
            COALESCE(NULLIF(source_ip, ''), 'unknown') AS source_ip,
            COALESCE(NULLIF(rule, ''), 'UNKNOWN') AS rule,
            COALESCE(rule_payload, '') AS rule_payload,
            COUNT(*) AS connections,
            COALESCE(SUM(conn_download), 0) AS download,
            COALESCE(SUM(conn_upload), 0) AS upload
        FROM connections
        GROUP BY COALESCE(NULLIF(source_ip, ''), 'unknown'), rule, rule_payload
        ORDER BY (COALESCE(SUM(conn_download), 0) + COALESCE(SUM(conn_upload), 0)) DESC
        LIMIT 1000
        "#,
    )
    .fetch_all(&db.pool)
    .await;

    match result {
        Ok(rows) => {
            let data: Vec<Value> = rows
                .into_iter()
                .map(|row| {
                    let source_ip: String = row.get("source_ip");
                    rule_row_to_json(&row, Some(source_ip))
                })
                .collect();
            Ok(json_success(Value::Array(data)))
        }
        Err(e) => Ok(json_error(e)),
    }
}

fn rule_row_to_json(row: &sqlx::sqlite::SqliteRow, source_ip: Option<String>) -> Value {
    let rule: String = row.get("rule");
    let rule_payload: String = row.get("rule_payload");
    let connections: i64 = row.get("connections");
    let download: i64 = row.get("download");
    let upload: i64 = row.get("upload");
    let display = if rule_payload.is_empty() {
        rule.clone()
    } else {
        format!("{} ({})", rule, rule_payload)
    };

    let mut value = json!({
        "rule": rule,
        "rule_payload": rule_payload,
        "display": display,
        "connections": connections,
        "download": download,
        "upload": upload,
        "total": download + upload
    });

    if let Some(ip) = source_ip {
        value["source_ip"] = Value::String(ip);
    }
    value
}

async fn handle_nodes(db: Arc<Database>) -> Result<warp::reply::Json, Infallible> {
    let result = sqlx::query(
        r#"
        SELECT
            COALESCE(chains, '') AS chains,
            COUNT(*) AS connections,
            COALESCE(SUM(conn_download), 0) AS download,
            COALESCE(SUM(conn_upload), 0) AS upload
        FROM connections
        GROUP BY chains
        "#,
    )
    .fetch_all(&db.pool)
    .await;

    match result {
        Ok(rows) => {
            let mut nodes: HashMap<String, (i64, i64, i64)> = HashMap::new();
            for row in rows {
                let chains: String = row.get("chains");
                let connections: i64 = row.get("connections");
                let download: i64 = row.get("download");
                let upload: i64 = row.get("upload");
                let node = extract_node(&chains);
                let entry = nodes.entry(node).or_insert((0, 0, 0));
                entry.0 += connections;
                entry.1 += download;
                entry.2 += upload;
            }

            let mut data: Vec<Value> = nodes
                .into_iter()
                .map(|(node, (connections, download, upload))| {
                    json!({
                        "node": node,
                        "connections": connections,
                        "download": download,
                        "upload": upload,
                        "total": download + upload
                    })
                })
                .collect();

            data.sort_by(|a, b| {
                b.get("total")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .cmp(&a.get("total").and_then(Value::as_i64).unwrap_or(0))
            });
            data.truncate(200);
            Ok(json_success(Value::Array(data)))
        }
        Err(e) => Ok(json_error(e)),
    }
}

fn extract_node(chains: &str) -> String {
    if chains.trim().is_empty() {
        return "UNKNOWN".to_string();
    }

    if let Ok(items) = serde_json::from_str::<Vec<String>>(chains) {
        if let Some(last) = items.last() {
            return last.clone();
        }
    }

    let cleaned = chains.trim().trim_matches(['[', ']']).replace('"', "");
    cleaned
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or("UNKNOWN")
        .to_string()
}
