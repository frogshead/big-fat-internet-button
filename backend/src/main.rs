use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    RequestPartsExt, Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

// Basic auth extractor
struct BasicAuth;

#[async_trait]
impl<S> FromRequestParts<S> for BasicAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get credentials from environment
        let expected_username =
            std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
        let expected_password =
            std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        // Extract the Authorization header
        let auth_header: TypedHeader<Authorization<Basic>> =
            parts.extract().await.map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    [("WWW-Authenticate", "Basic realm=\"Admin Area\"")],
                    "Unauthorized",
                )
                    .into_response()
            })?;

        // Check credentials
        if auth_header.username() == expected_username
            && auth_header.password() == expected_password
        {
            Ok(BasicAuth)
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                [("WWW-Authenticate", "Basic realm=\"Admin Area\"")],
                "Invalid credentials",
            )
                .into_response())
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Shared state for storing button presses
    let app_state: AppState = Arc::new(Mutex::new(Vec::new()));

    // Get admin credentials from environment or use defaults
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    tracing::info!(
        "Admin authentication enabled (username: {})",
        admin_username
    );
    if admin_username == "admin" && admin_password == "admin" {
        tracing::warn!("⚠️  Using default credentials! Set ADMIN_USERNAME and ADMIN_PASSWORD environment variables for production.");
    }

    // Build the router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/destroy", post(button_press_handler))
        .route("/api/events", get(get_events_handler))
        .route("/admin", get(admin_page_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    tracing::info!("Server listening on {}", listener.local_addr().unwrap());
    tracing::info!("Admin page available at http://localhost:4000/admin");

    axum::serve(listener, app).await.unwrap();
}

async fn index_handler(State(state): State<AppState>) -> Html<String> {
    let events = state.lock().unwrap();
    let is_destroyed = !events.is_empty();

    if is_destroyed {
        // World has been destroyed - show Finnish lyrics
        Html(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Olemme tuhoutuneet</title>
                <meta charset="utf-8">
                <style>
                    body {
                        background: linear-gradient(135deg, #1a0000 0%, #000000 100%);
                        color: #ff6666;
                        font-family: 'Courier New', monospace;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        min-height: 100vh;
                        margin: 0;
                        padding: 20px;
                    }
                    .container {
                        max-width: 800px;
                        text-align: center;
                    }
                    h1 {
                        font-size: 2.5em;
                        color: #ff3333;
                        text-shadow: 0 0 20px #ff0000;
                        margin-bottom: 40px;
                        animation: glow 2s ease-in-out infinite alternate;
                    }
                    @keyframes glow {
                        from { text-shadow: 0 0 20px #ff0000; }
                        to { text-shadow: 0 0 30px #ff0000, 0 0 40px #ff0000; }
                    }
                    .lyrics {
                        font-size: 1.1em;
                        line-height: 1.8;
                        color: #ff8888;
                        white-space: pre-line;
                        text-align: left;
                        background: rgba(0, 0, 0, 0.5);
                        padding: 30px;
                        border-radius: 10px;
                        border: 2px solid #ff3333;
                    }
                    .chorus {
                        font-style: italic;
                        color: #ffaaaa;
                        margin: 20px 0;
                    }
                    .credit {
                        margin-top: 30px;
                        font-size: 0.9em;
                        color: #666;
                    }
                    a {
                        color: #ff6666;
                        text-decoration: none;
                    }
                    a:hover {
                        color: #ff3333;
                        text-shadow: 0 0 10px #ff0000;
                    }
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>💀 Maailma on tuhoutunut 💀</h1>
                    <div class="lyrics">
Isä seisoo rannalla, ja ase kourassa myrskyä odottaa
Äiti painaa pikkusiskon syliin, kun taivas oudosti iltaisin punertaa
Veljestä kuulin viimeksi kun tuli kirje vierailta mailta
Kertoi voivansa hyvin, mutta sanoi että tilanne näyttää täällä myös pirun pahalta

<span class="chorus">Tää on kymmenen tikkua laudalla
Me ollaan piilossa viimeisellä rannalla
Eikä kukaan meitä enää etsi, tää on tyhjä ja loputon leikki
Ja vain tuuli kuiskaa: "kaikki pois piilostaan"</span>

Setä on jotenkin sekaisin, se istuu liiterissä ja itkee ja nauraa
Syyttä välillä saatanaa ja jumalaa ja sanoo: "nyt meitä syntisiä rangaistaan"
Naapurissa on oudon hiljaista, on verhot olleet kiinni jo viikon
Isä kielsi käymästä siellä, sanoi: "tehneet mitä tehneet, se niiden asia on, oma asia on"

<span class="chorus">Tää on kymmenen tikkua laudalla...</span>

Minä olen vasta seitsemän mutta silti mä jotain jo ymmärrän
Kun tv on mykkä ja radiossa kuulin uutisten lukijan itkevän
Ne puhuu jostain pilvestä joka matkalla on vielä tänne päin
Pikkusisko kysyi: "montako päivää jouluun?"
Ja isän silmiään pyyhkivän näin, silmiään pyyhkivän näin

<span class="chorus">Tää on kymmenen tikkua laudalla...</span>
                    </div>
                    <p class="credit">
                        "Autiolla rannalla" - Sir Elwoodin Hiljaiset Värit<br>
                        <a href="/admin">Admin Panel</a>
                    </p>
                </div>
            </body>
            </html>
            "#
            .to_string(),
        )
    } else {
        // World is safe - show sunny day
        Html(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Kaikki on hyvin</title>
                <meta charset="utf-8">
                <style>
                    body {
                        background: linear-gradient(135deg, #87CEEB 0%, #98D8E8 100%);
                        color: #333;
                        font-family: 'Courier New', monospace;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                    }
                    .container {
                        text-align: center;
                        background: rgba(255, 255, 255, 0.9);
                        padding: 40px;
                        border-radius: 20px;
                        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
                    }
                    h1 {
                        font-size: 3em;
                        color: #FFD700;
                        text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
                        margin-bottom: 20px;
                    }
                    .message {
                        font-size: 1.5em;
                        color: #2E8B57;
                        margin-bottom: 30px;
                    }
                    img {
                        max-width: 500px;
                        width: 100%;
                        border-radius: 10px;
                        box-shadow: 0 5px 15px rgba(0, 0, 0, 0.2);
                    }
                    a {
                        display: inline-block;
                        margin-top: 20px;
                        padding: 10px 20px;
                        background: #4CAF50;
                        color: white;
                        text-decoration: none;
                        border-radius: 5px;
                        transition: all 0.3s;
                    }
                    a:hover {
                        background: #45a049;
                        transform: scale(1.05);
                    }
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>☀️ Kaikki on hyvin! ☀️</h1>
                    <p class="message">Maailma on turvassa... toistaiseksi.</p>
                    <img src="https://media.giphy.com/media/v1.Y2lkPWVjZjA1ZTQ3ajE2MG9tN2VtamtlN3d0ZWRhNDIyY3h5OHNwbHg2eXgxdmEyN2M2NiZlcD12MV9naWZzX3NlYXJjaCZjdD1n/3ohs4qoyjBWFKxk688/giphy.gif" alt="Sunny day">
                    <br>
                    <a href="/admin">Admin Panel</a>
                </div>
            </body>
            </html>
            "#.to_string(),
        )
    }
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

    tracing::info!("🔥 WORLD DESTRUCTION INITIATED! Event #{}", id);

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

async fn admin_page_handler(_auth: BasicAuth, State(state): State<AppState>) -> Html<String> {
    let events = state.lock().unwrap();
    let total_destructions = events.len();

    let events_html = if events.is_empty() {
        "<tr><td colspan='3' style='text-align: center; padding: 20px;'>No destructions yet. World is safe... for now.</td></tr>".to_string()
    } else {
        events
            .iter()
            .rev()
            .map(|event| {
                format!(
                    "<tr><td>#{}</td><td>{}</td><td>{}</td></tr>",
                    event.id,
                    event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    event.device_id.as_deref().unwrap_or("Unknown")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Admin - World Destruction Log</title>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <style>
                * {{
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                }}
                body {{
                    background: linear-gradient(135deg, #000000 0%, #1a0000 100%);
                    color: #ff3333;
                    font-family: 'Courier New', monospace;
                    padding: 20px;
                    min-height: 100vh;
                }}
                .container {{
                    max-width: 1200px;
                    margin: 0 auto;
                }}
                h1 {{
                    text-align: center;
                    font-size: 3em;
                    margin-bottom: 10px;
                    text-shadow: 0 0 20px #ff0000, 0 0 40px #ff0000;
                    animation: glow 2s ease-in-out infinite alternate;
                }}
                @keyframes glow {{
                    from {{ text-shadow: 0 0 20px #ff0000, 0 0 40px #ff0000; }}
                    to {{ text-shadow: 0 0 30px #ff0000, 0 0 60px #ff0000, 0 0 80px #ff0000; }}
                }}
                .subtitle {{
                    text-align: center;
                    font-size: 1.2em;
                    margin-bottom: 30px;
                    color: #ff6666;
                }}
                .stats {{
                    background: rgba(255, 0, 0, 0.1);
                    border: 2px solid #ff3333;
                    border-radius: 10px;
                    padding: 20px;
                    margin-bottom: 30px;
                    text-align: center;
                }}
                .stats h2 {{
                    font-size: 4em;
                    color: #ff0000;
                    text-shadow: 0 0 10px #ff0000;
                }}
                .stats p {{
                    font-size: 1.2em;
                    color: #ff6666;
                    margin-top: 10px;
                }}
                table {{
                    width: 100%;
                    border-collapse: collapse;
                    background: rgba(0, 0, 0, 0.5);
                    border: 2px solid #ff3333;
                    border-radius: 10px;
                    overflow: hidden;
                }}
                th, td {{
                    padding: 15px;
                    text-align: left;
                    border-bottom: 1px solid #ff3333;
                }}
                th {{
                    background: rgba(255, 0, 0, 0.2);
                    font-size: 1.2em;
                    text-transform: uppercase;
                    color: #ff6666;
                }}
                tr:hover {{
                    background: rgba(255, 0, 0, 0.1);
                }}
                .refresh {{
                    text-align: center;
                    margin-top: 20px;
                }}
                .refresh a {{
                    display: inline-block;
                    padding: 10px 20px;
                    background: #ff0000;
                    color: #000;
                    text-decoration: none;
                    border-radius: 5px;
                    font-weight: bold;
                    transition: all 0.3s;
                }}
                .refresh a:hover {{
                    background: #ff3333;
                    box-shadow: 0 0 20px #ff0000;
                }}
                .api-info {{
                    margin-top: 30px;
                    padding: 20px;
                    background: rgba(0, 255, 0, 0.05);
                    border: 1px solid #0f0;
                    border-radius: 5px;
                    color: #0f0;
                }}
                .api-info h3 {{
                    color: #0f0;
                    margin-bottom: 10px;
                }}
                .api-info code {{
                    background: rgba(0, 0, 0, 0.5);
                    padding: 2px 5px;
                    border-radius: 3px;
                }}
            </style>
            <script>
                // Auto-refresh every 5 seconds
                setTimeout(function(){{
                    location.reload();
                }}, 5000);
            </script>
        </head>
        <body>
            <div class="container">
                <h1>💀 WORLD DESTRUCTION LOG 💀</h1>
                <p class="subtitle">Nuclear Launch Control Center</p>

                <div class="stats">
                    <h2>{}</h2>
                    <p>TOTAL WORLD DESTRUCTIONS</p>
                </div>

                <table>
                    <thead>
                        <tr>
                            <th>Event ID</th>
                            <th>Timestamp</th>
                            <th>Device ID</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>

                <div class="refresh">
                    <a href="/admin">🔄 Refresh Now</a>
                    <p style="margin-top: 10px; color: #666;">Auto-refreshes every 5 seconds</p>
                </div>

                <div class="api-info">
                    <h3>📡 API Endpoints</h3>
                    <p><strong>POST /api/destroy</strong> - Trigger world destruction</p>
                    <p style="margin-top: 5px;">Body: <code>{{"device_id": "esp32-001"}}</code></p>
                    <p style="margin-top: 10px;"><strong>GET /api/events</strong> - Get all destruction events (JSON)</p>
                </div>
            </div>
        </body>
        </html>
        "#,
        total_destructions, events_html
    ))
}
