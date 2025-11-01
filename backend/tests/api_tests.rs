use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn test_index_page() {
    let app = backend_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("BIG RED INTERNET BUTTON"));
    assert!(body_str.contains("OPERATIONAL"));
}

#[tokio::test]
async fn test_button_press_creates_event() {
    let app = backend_test_app().await;

    let request_body = json!({
        "device_id": "test-device-001"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/destroy")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "WORLD DESTROYED");
    assert_eq!(json["event_id"], 1);
    assert!(json["message"].as_str().unwrap().contains("Nuclear launch"));
    assert!(json["timestamp"].is_string());
}

#[tokio::test]
async fn test_get_events() {
    let app = backend_test_app().await;

    // First, create an event
    let request_body = json!({"device_id": "test-device-002"});
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/destroy")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Now get events
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let events: Value = serde_json::from_slice(&body).unwrap();

    assert!(events.is_array());
    let events_array = events.as_array().unwrap();
    assert!(events_array.len() > 0);

    let first_event = &events_array[0];
    assert!(first_event["id"].is_number());
    assert!(first_event["timestamp"].is_string());
    assert!(first_event["device_id"].is_string());
}

#[tokio::test]
async fn test_admin_page() {
    let app = backend_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("WORLD DESTRUCTION LOG"));
    assert!(body_str.contains("TOTAL WORLD DESTRUCTIONS"));
    assert!(body_str.contains("/api/destroy"));
}

#[tokio::test]
async fn test_multiple_button_presses() {
    let app = backend_test_app().await;

    // Press button 3 times
    for i in 1..=3 {
        let request_body = json!({"device_id": format!("device-{}", i)});
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/destroy")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["event_id"], i);
    }

    // Verify all events exist
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let events: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 3);
}

// Helper function to create a test app instance
async fn backend_test_app() -> axum::Router {
    use axum::{
        extract::State,
        http::StatusCode,
        response::{Html, IntoResponse, Json},
        routing::{get, post},
        Router,
    };
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ButtonPress {
        id: usize,
        timestamp: DateTime<Utc>,
        device_id: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ButtonPressRequest {
        device_id: Option<String>,
    }

    type AppState = Arc<Mutex<Vec<ButtonPress>>>;

    async fn index_handler() -> Html<&'static str> {
        Html(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Big Red Internet Button</title>
            </head>
            <body>
                <h1>🚀 BIG RED INTERNET BUTTON 🚀</h1>
                <p>Nuclear Destruction Simulator Backend</p>
                <p>Status: OPERATIONAL</p>
            </body>
            </html>
            "#,
        )
    }

    async fn button_press_handler(
        State(state): State<AppState>,
        Json(payload): Json<ButtonPressRequest>,
    ) -> impl IntoResponse {
        let mut events = state.lock().unwrap();
        let id = events.len() + 1;

        let button_press = ButtonPress {
            id,
            timestamp: Utc::now(),
            device_id: payload.device_id,
        };

        events.push(button_press.clone());

        (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "WORLD DESTROYED",
                "event_id": id,
                "message": "💥 Nuclear launch successful! Goodbye cruel world!",
                "timestamp": button_press.timestamp,
            })),
        )
    }

    async fn get_events_handler(State(state): State<AppState>) -> impl IntoResponse {
        let events = state.lock().unwrap();
        Json(events.clone())
    }

    async fn admin_page_handler(State(state): State<AppState>) -> Html<String> {
        let events = state.lock().unwrap();
        let total_destructions = events.len();

        Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head><title>Admin - World Destruction Log</title></head>
            <body>
                <h1>💀 WORLD DESTRUCTION LOG 💀</h1>
                <p>TOTAL WORLD DESTRUCTIONS: {}</p>
                <p>POST /api/destroy - Trigger world destruction</p>
            </body>
            </html>
            "#,
            total_destructions
        ))
    }

    let app_state: AppState = Arc::new(Mutex::new(Vec::new()));

    Router::new()
        .route("/", get(index_handler))
        .route("/api/destroy", post(button_press_handler))
        .route("/api/events", get(get_events_handler))
        .route("/admin", get(admin_page_handler))
        .with_state(app_state)
}
