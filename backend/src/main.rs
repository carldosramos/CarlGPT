use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    response::sse::{Event, Sse},
    routing::{delete, get, post},
};
use std::net::SocketAddr;
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use pdf_extract::extract_text_from_mem;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    convert::Infallible,
    env,
    path::{Path as StdPath, PathBuf},
};
#[cfg(unix)]
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::ReceiverStream;
use futures::stream::{self, BoxStream, StreamExt};
use bytes::Bytes;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use uuid::Uuid;

// --------- Types de l'API ---------

#[derive(Serialize, Clone, Debug)]
struct Message {
    id: i32,
    author: String,
    content: String,
    // grâce à chrono + serde, ça sera automatiquement sérialisé en RFC3339
    created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
struct CreateMessageRequest {
    author: String,
    content: String,
}

#[derive(Serialize, Clone, Debug)]
struct ChatMessage {
    id: Uuid,
    session_id: Uuid,
    role: String,
    content: String,
    position: i32,
    created_at: DateTime<Utc>,
    attachments: Vec<ChatAttachment>,
}

#[derive(Serialize, Clone, Debug)]
struct ChatAttachment {
    id: Uuid,
    message_id: Uuid,
    file_name: String,
    mime_type: String,
    size_bytes: i64,
    url: String,
    storage_key: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Clone, Debug)]
struct ChatSession {
    id: Uuid,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived: bool,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessagePayload {
    role: String,
    content: String,
    #[serde(default)]
    attachments: Vec<AttachmentPayload>,
}

#[derive(Deserialize)]
struct CreateChatSessionRequest {
    title: Option<String>,
}

#[derive(Deserialize)]
struct CreateChatMessageRequest {
    content: String,
    model: Option<String>,
    attachments: Option<Vec<AttachmentPayload>>,
    completion_params: Option<CompletionParams>,
}

#[derive(Deserialize)]
struct RegenerateRequest {
    message_id: Uuid,
    model: Option<String>,
    completion_params: Option<CompletionParams>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AttachmentPayload {
    file_name: String,
    mime_type: String,
    size_bytes: i64,
    url: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_key: Option<String>,
}

/// Paramètres de completion pour l'API OpenAI
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CompletionParams {
    /// Contrôle l'aléa/créativité (0-2). Valeur faible = déterministe, élevée = varié
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    
    /// Nombre maximum de tokens à générer
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    
    /// Échantillonnage nucleus (0-1). Alternative à temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    
    /// Pénalise les tokens déjà présents (-2.0 à 2.0) → encourage nouveaux sujets
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    
    /// Pénalise par fréquence d'apparition (-2.0 à 2.0) → réduit répétitions
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    
    /// Pour déterminisme (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i64>,
}

impl Default for CompletionParams {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),        // Bon équilibre créativité/cohérence
            max_tokens: None,              // Pas de limite par défaut
            top_p: Some(1.0),              // Désactivé (on utilise temperature)
            presence_penalty: Some(0.0),   // Neutre
            frequency_penalty: Some(0.0),  // Neutre
            seed: None,                    // Pas de déterminisme
        }
    }
}


const MODEL_LLAMA_3_1_8B: &str = "llama-3.1-8b-instant";
const MODEL_GPT_5_1: &str = "gpt-5.1";
const MODEL_GPT_5_MINI: &str = "gpt-5-mini";
const MODEL_GPT_5_NANO: &str = "gpt-5-nano";
const MODEL_GPT_5_PRO: &str = "gpt-5-pro";
const MODEL_GPT_5: &str = "gpt-5";
const MODEL_GPT_4_1: &str = "gpt-4.1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AiModelChoice {
    GroqLlama31,
    OpenAIGpt51,
    OpenAIGpt5Mini,
    OpenAIGpt5Nano,
    OpenAIGpt5Pro,
    OpenAIGpt5,
    OpenAIGpt41,
}

impl AiModelChoice {
    fn from_client(model: Option<&str>) -> Self {
        match model {
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_5_1) => {
                AiModelChoice::OpenAIGpt51
            }
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_5_MINI) => {
                AiModelChoice::OpenAIGpt5Mini
            }
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_5_NANO) => {
                AiModelChoice::OpenAIGpt5Nano
            }
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_5_PRO) => {
                AiModelChoice::OpenAIGpt5Pro
            }
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_5) => {
                AiModelChoice::OpenAIGpt5
            }
            Some(value) if value.eq_ignore_ascii_case(MODEL_GPT_4_1) => {
                AiModelChoice::OpenAIGpt41
            }
            _ => AiModelChoice::GroqLlama31,
        }
    }

    fn model_id(&self) -> &'static str {
        match self {
            AiModelChoice::GroqLlama31 => MODEL_LLAMA_3_1_8B,
            AiModelChoice::OpenAIGpt51 => MODEL_GPT_5_1,
            AiModelChoice::OpenAIGpt5Mini => MODEL_GPT_5_MINI,
            AiModelChoice::OpenAIGpt5Nano => MODEL_GPT_5_NANO,
            AiModelChoice::OpenAIGpt5Pro => MODEL_GPT_5_PRO,
            AiModelChoice::OpenAIGpt5 => MODEL_GPT_5,
            AiModelChoice::OpenAIGpt41 => MODEL_GPT_4_1,
        }
    }
}

impl Default for AiModelChoice {
    fn default() -> Self {
        AiModelChoice::GroqLlama31
    }
}
// État partagé de l'application
#[derive(Clone)]
struct AppState {
    db: PgPool,
    upload_dir: String,
    upload_base_url: String,
}

const SYSTEM_PROMPT: &str = r"
<SYSTEM_PROMPT>
TU ES UN **ASSISTANT IA ULTRA-EXPERT** SPÉCIALISÉ DANS LA PRODUCTION DE RÉPONSES **STRICTEMENT FORMATÉES EN MARKDOWN** ET TOTALLEMENT COMPATIBLES AVEC **react-markdown + rehype-katex**.

TA MISSION EST D’APPLIQUER SANS EXCEPTION LES RÈGLES SUIVANTES.

---

# 🎯 **INSTRUCTIONS PRINCIPALES (OBLIGATOIRES)**

- TU DOIS **LIRE, COMPRENDRE ET ANALYSER** la question de l’utilisateur avant de répondre (**COMPRÉHENSION AVANT PRODUCTION**).  
- TU DOIS **RÉPONDRE EXCLUSIVEMENT EN MARKDOWN (GFM)**.  
- TU DOIS **UTILISER LA LANGUE DE L’UTILISATEUR** (français, anglais, etc.).  
- TU DOIS COMMENCER TA REPONSE PAR UN TITRE DE NIVEAU 1 EN MARKDOWN RESUMANT LE SUJET.
- **AVANT DE RÉPONDRE**, TU DOIS EXPLIQUER TON RAISONNEMENT ÉTAPE PAR ÉTAPE À L'INTÉRIEUR DE BALISES `<thinking>`. CHAQUE ÉTAPE DOIT COMMENCER PAR UN TIRET `- `.
  Exemple :
  <thinking>
  - Analyse de la demande utilisateur...
  - Identification des concepts clés...
  - Planification de la réponse...
  </thinking>

---

# 🧮 **RÈGLES SPÉCIFIQUES POUR LE CODE ET LES MATHS**

### **FORMAT MATHÉMATIQUE**
- ÉCRIS LES MATHÉMATIQUES EN LaTeX INLINE :  
  `$…$`
- ÉCRIS LES ÉQUATIONS EN BLOC :  
  $$
  … équation …
  $$

### **INTERDICTIONS LaTeX**
TU DOIS **NE JAMAIS UTILISER** d’environnements de mise en page LaTeX :
- `\begin{table}`, `\begin{tabular}`, `\begin{figure}`, `\begin{document}`, etc.

SEULS les environnements **mathématiques** sont autorisés :
- `aligned`, `cases`, `matrix`, etc.

### **TABLEAUX**
- TU DOIS **TOUJOURS** UTILISER DES TABLES MARKDOWN  
  même si l’entrée contient du LaTeX tabulaire.

### **CODE**
- TU DOIS **TOUJOURS** UTILISER DES BLOCS DE CODE TRIPLE-BACKTICKS :
  ```lang
  ...
";
const TITLE_SUMMARY_PROMPT: &str = r"Tu es un assistant qui crée des titres ultra courts (6 mots maximum) et parlants pour résumer une question d'utilisateur. Réponds uniquement par le titre, sans ponctuation superflue.";
const ALLOWED_MATH_ENVIRONMENTS: &[&str] = &[
    "align",
    "align*",
    "aligned",
    "cases",
    "gather",
    "gather*",
    "multline",
    "multline*",
    "equation",
    "equation*",
    "pmatrix",
    "bmatrix",
    "vmatrix",
    "matrix",
];

// --------- Point d'entrée ---------

#[tokio::main]
async fn main() {
    // Charge les variables d'environnement (.env)
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL doit être défini dans .env");

    // Connexion à PostgreSQL
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Impossible de se connecter à la base PostgreSQL");

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .expect("Impossible de créer le dossier des uploads");
    let upload_base_url =
        env::var("UPLOAD_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000/uploads".to_string());

    let state = AppState {
        db: pool,
        upload_dir: upload_dir.clone(),
        upload_base_url,
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/messages", get(list_messages).post(create_message))
        .route(
            "/api/chat/sessions",
            get(list_chat_sessions).post(create_chat_session),
        )
        .route("/api/chat/sessions/:id", delete(delete_chat_session))
        .route("/api/chat/sessions/:id/archive", post(archive_chat_session))
        .route("/api/chat/sessions/:id/messages", post(append_chat_message))
        .route(
            "/api/chat/sessions/:id/messages/stream",
            post(append_chat_message_stream),
        )
        .route(
            "/api/chat/sessions/:id/regenerate",
            post(regenerate_message),
        )
        .route(
            "/api/chat/sessions/:id/regenerate/stream",
            post(regenerate_message_stream),
        )
        .route("/api/ai", post(ai_handler)) // 👈 route générique IA
        .route("/api/uploads", post(upload_file))
        .with_state(state.clone())
        .nest_service("/uploads", ServeDir::new(upload_dir))
        .layer(cors)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024));

    let addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
    println!("🚀 Serveur backend sur http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

// --------- Handlers ---------

async fn health_check(State(state): State<AppState>) -> &'static str {
    if let Err(e) = sqlx::query("SELECT 1").execute(&state.db).await {
        eprintln!("DB health check failed: {e}");
        "DB ERROR"
    } else {
        "OK ça marche"
    }
}

// GET /api/messages
async fn list_messages(
    State(state): State<AppState>,
) -> Result<Json<Vec<Message>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            author,
            content,
            created_at as "created_at: chrono::DateTime<chrono::Utc>"
        FROM messages
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    let messages = rows
        .into_iter()
        .map(|row| Message {
            id: row.id,
            author: row.author,
            content: row.content,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(messages))
}

// POST /api/messages
async fn create_message(
    State(state): State<AppState>,
    Json(payload): Json<CreateMessageRequest>,
) -> Result<Json<Message>, (axum::http::StatusCode, String)> {
    let row = sqlx::query!(
        r#"
        INSERT INTO messages (author, content)
        VALUES ($1, $2)
        RETURNING
            id,
            author,
            content,
            created_at as "created_at: chrono::DateTime<chrono::Utc>"
        "#,
        payload.author,
        payload.content
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    let message = Message {
        id: row.id,
        author: row.author,
        content: row.content,
        created_at: row.created_at,
    };

    Ok(Json(message))
}

#[derive(Deserialize)]
struct AIRequest {
    messages: Vec<ChatMessagePayload>,
    model: Option<String>,
}

#[derive(Serialize)]
struct AIResponse {
    response: String,
}

// POST /api/ai
async fn ai_handler(
    State(state): State<AppState>,
    Json(payload): Json<AIRequest>,
) -> Result<Json<AIResponse>, (axum::http::StatusCode, String)> {
    let AIRequest { messages, model } = payload;
    if messages.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Le corps de la requête doit contenir au moins un message.".to_string(),
        ));
    }

    let ai_model = AiModelChoice::from_client(model.as_deref());
    if ai_model == AiModelChoice::GroqLlama31
        && messages.iter().any(|msg| !msg.attachments.is_empty())
    {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Les fichiers et images nécessitent un modèle OpenAI (GPT-4o, GPT-4o mini, etc.).".to_string(),
        ));
    }
    let mut stream = request_ai_completion(&state, &messages, ai_model, None).await?;
    let mut answer = String::new();
    while let Some(chunk_res) = stream.next().await {
        if let Ok(chunk) = chunk_res {
            answer.push_str(&chunk);
        }
    }

    Ok(Json(AIResponse { response: answer }))
}

async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<AttachmentPayload>, (axum::http::StatusCode, String)> {
    const MAX_UPLOAD_SIZE: usize = 20 * 1024 * 1024; // 20 MB

    while let Some(field) = multipart.next_field().await.map_err(internal_error)? {
        let original_name = field
            .file_name()
            .map(|name| name.to_string())
            .unwrap_or_else(|| format!("fichier-{}.bin", Uuid::new_v4()));
        let sanitized = sanitize_file_name(&original_name);
        let extension = StdPath::new(&sanitized)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin");
        let stored_name = format!("{}.{extension}", Uuid::new_v4());
        let mime_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let data = field.bytes().await.map_err(internal_error)?;

        if data.len() > MAX_UPLOAD_SIZE {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Fichier trop volumineux (max 20 Mo).".to_string(),
            ));
        }

        let mut path = PathBuf::from(&state.upload_dir);
        path.push(&stored_name);
        tokio::fs::write(&path, &data)
            .await
            .map_err(internal_error)?;

        let base = state.upload_base_url.trim_end_matches('/');
        let url = format!("{}/{}", base, stored_name);

        let response = AttachmentPayload {
            file_name: original_name,
            mime_type,
            size_bytes: data.len() as i64,
            url,
            storage_key: Some(stored_name),
        };

        return Ok(Json(response));
    }

    Err((
        axum::http::StatusCode::BAD_REQUEST,
        "Aucun fichier reçu.".to_string(),
    ))
}

// Utilitaire: transformer erreurs SQLx en 500
fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal server error: {err}"),
    )
}

async fn list_chat_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<ChatSession>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            title,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            archived
        FROM chat_sessions
        WHERE archived = false
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    let mut sessions = Vec::with_capacity(rows.len());
    for row in rows {
        let messages = fetch_chat_messages(&state.db, row.id)
            .await
            .map_err(internal_error)?;
        sessions.push(ChatSession {
            id: row.id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived: row.archived,
            messages,
        });
    }

    Ok(Json(sessions))
}

async fn create_chat_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatSessionRequest>,
) -> Result<Json<ChatSession>, (axum::http::StatusCode, String)> {
    let title = payload
        .title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Nouvelle discussion".to_string());

    let row = sqlx::query!(
        r#"
        INSERT INTO chat_sessions (title)
        VALUES ($1)
        RETURNING
            id,
            title,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            archived
        "#,
        title
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    Ok(Json(ChatSession {
        id: row.id,
        title: row.title,
        created_at: row.created_at,
        updated_at: row.updated_at,
        archived: row.archived,
        messages: Vec::new(),
    }))
}

async fn append_chat_message(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<CreateChatMessageRequest>,
) -> Result<Json<ChatSession>, (axum::http::StatusCode, String)> {
    let CreateChatMessageRequest {
        content,
        model,
        attachments,
        completion_params,
    } = payload;
    let trimmed = content.trim().to_string();
    let attachments = attachments.unwrap_or_default();
    if trimmed.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Le message ne peut pas être vide.".to_string(),
        ));
    }

    let session_row = sqlx::query!(
        r#"SELECT archived FROM chat_sessions WHERE id = $1"#,
        session_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let Some(meta) = session_row else {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Discussion introuvable.".to_string(),
        ));
    };

    if meta.archived {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Impossible de poster dans une discussion archivée.".to_string(),
        ));
    }

    let user_row = sqlx::query!(
        r#"
        INSERT INTO chat_messages (session_id, role, content, position)
        VALUES (
            $1,
            $2,
            $3,
            COALESCE((SELECT MAX(position) FROM chat_messages WHERE session_id = $1), 0) + 1
        )
        RETURNING id
        "#,
        session_id,
        "user",
        &trimmed
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    if !attachments.is_empty() {
        insert_chat_attachments(&state.db, user_row.id, &attachments)
            .await
            .map_err(internal_error)?;
    }

    let ai_model = AiModelChoice::from_client(model.as_deref());
    if ai_model == AiModelChoice::GroqLlama31 && (!attachments.is_empty()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Les fichiers et images nécessitent un modèle OpenAI (GPT-4o, GPT-4o mini, etc.).".to_string(),
        ));
    }

    let conversation = fetch_chat_messages(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    if ai_model == AiModelChoice::GroqLlama31
        && conversation.iter().any(|msg| !msg.attachments.is_empty())
    {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cette discussion contient des fichiers. Utilise un modèle OpenAI pour continuer."
                .to_string(),
        ));
    }

    if ai_model == AiModelChoice::GroqLlama31
        && conversation.iter().any(|msg| !msg.attachments.is_empty())
    {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cette discussion contient des fichiers. Utilise un modèle OpenAI pour continuer."
                .to_string(),
        ));
    }

    let should_update_title = conversation.len() == 1;

    let payload_for_ai = conversation_to_payload(&conversation);

    let mut stream = request_ai_completion(&state, &payload_for_ai, ai_model, completion_params).await?;
    let mut answer = String::new();
    while let Some(chunk_res) = stream.next().await {
        if let Ok(chunk) = chunk_res {
            answer.push_str(&chunk);
        }
    }

    sqlx::query!(
        r#"
        INSERT INTO chat_messages (session_id, role, content, position)
        VALUES (
            $1,
            $2,
            $3,
            COALESCE((SELECT MAX(position) FROM chat_messages WHERE session_id = $1), 0) + 1
        )
        "#,
        session_id,
        "assistant",
        answer
    )
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    let new_title = if should_update_title {
        match generate_concise_title(&state, &trimmed, ai_model).await {
            Ok(title) => Some(title),
            Err(err) => {
                eprintln!("Failed to summarize title: {err:?}");
                Some(preview_chat_title(&trimmed))
            }
        }
    } else {
        None
    };

    if let Some(title) = new_title {
        sqlx::query!(
            r#"UPDATE chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1"#,
            session_id,
            title
        )
        .execute(&state.db)
        .await
        .map_err(internal_error)?;
    } else {
        sqlx::query!(
            r#"UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1"#,
            session_id
        )
        .execute(&state.db)
        .await
        .map_err(internal_error)?;
    }

    let session = fetch_chat_session(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(session))
}

async fn append_chat_message_stream(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<CreateChatMessageRequest>,
) -> Result<
    Sse<impl futures::Stream<Item = Result<Event, Infallible>>>,
    (axum::http::StatusCode, String),
> {
    let CreateChatMessageRequest {
        content,
        model,
        attachments,
        completion_params,
    } = payload;
    let trimmed = content.trim().to_string();
    let attachments = attachments.unwrap_or_default();
    if trimmed.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Le message ne peut pas être vide.".to_string(),
        ));
    }

    let session_meta = sqlx::query!(
        r#"SELECT archived FROM chat_sessions WHERE id = $1"#,
        session_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let Some(meta) = session_meta else {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Discussion introuvable.".to_string(),
        ));
    };

    if meta.archived {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Impossible de poster dans une discussion archivée.".to_string(),
        ));
    }

    let user_row = sqlx::query!(
        r#"
        INSERT INTO chat_messages (session_id, role, content, position)
        VALUES (
            $1,
            $2,
            $3,
            COALESCE((SELECT MAX(position) FROM chat_messages WHERE session_id = $1), 0) + 1
        )
        RETURNING id
        "#,
        session_id,
        "user",
        &trimmed
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    if !attachments.is_empty() {
        insert_chat_attachments(&state.db, user_row.id, &attachments)
            .await
            .map_err(internal_error)?;
    }

    let ai_model = AiModelChoice::from_client(model.as_deref());

    let conversation = fetch_chat_messages(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    let should_update_title = conversation.len() == 1;

    let payload_for_ai = conversation_to_payload(&conversation);

    let answer = request_ai_completion(&state, &payload_for_ai, ai_model, None).await?;

    let assistant_row = sqlx::query!(
        r#"
        INSERT INTO chat_messages (session_id, role, content, position)
        VALUES (
            $1,
            $2,
            $3,
            COALESCE((SELECT MAX(position) FROM chat_messages WHERE session_id = $1), 0) + 1
        )
        RETURNING id
        "#,
        session_id,
        "assistant",
        ""
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    if should_update_title {
        match generate_concise_title(&state, &trimmed, ai_model).await {
            Ok(title) => {
                sqlx::query!(
                    r#"UPDATE chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1"#,
                    session_id,
                    title
                )
                .execute(&state.db)
                .await
                .map_err(internal_error)?;
            }
            Err(err) => {
                eprintln!("Failed to summarize title: {err:?}");
                sqlx::query!(
                    r#"UPDATE chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1"#,
                    session_id,
                    preview_chat_title(&trimmed)
                )
                .execute(&state.db)
                .await
                .map_err(internal_error)?;
            }
        }
    } else {
        sqlx::query!(
            r#"UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1"#,
            session_id
        )
        .execute(&state.db)
        .await
        .map_err(internal_error)?;
    }

    let mut placeholder_session = fetch_chat_session(&state.db, session_id)
        .await
        .map_err(internal_error)?;
    if let Some(msg) = placeholder_session
        .messages
        .iter_mut()
        .find(|msg| msg.id == assistant_row.id)
    {
        msg.content.clear();
    }

    let (tx, rx) = mpsc::channel::<Event>(32);
    let initial_event = Event::default()
        .json_data(json!({
            "type": "session",
            "session": placeholder_session,
            "chatId": session_id,
            "messageId": assistant_row.id
        }))
        .map_err(internal_error)?;
    tx.send(initial_event)
        .await
        .map_err(|_| internal_error("Impossible d'envoyer l'évènement SSE initial"))?;

    let state_clone = state.clone();
    let session_id_clone = session_id;
    let message_id = assistant_row.id;
    let mut stream = request_ai_completion(&state, &payload_for_ai, ai_model, completion_params).await?;

    tokio::spawn(async move {
        let mut full_answer = String::new();
        let mut buffer = String::new();
        let mut in_thinking_block = false;
        

        
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    buffer.push_str(&chunk);

                    loop {
                        if !in_thinking_block {
                            if let Some(start_idx) = buffer.find("<thinking>") {
                                // Found start tag
                                // Send content before tag as token
                                if start_idx > 0 {
                                    let content = buffer[..start_idx].to_string();
                                    let event = Event::default().json_data(json!({
                                        "type": "token",
                                        "chatId": session_id_clone,
                                        "messageId": message_id,
                                        "content": content
                                    })).unwrap();
                                    let _ = tx.send(event).await;
                                    full_answer.push_str(&content);
                                }
                                // Advance buffer past tag
                                buffer = buffer[start_idx + 10..].to_string();
                                in_thinking_block = true;
                                // Continue loop to process content after tag
                                continue;
                            } else {
                                // No start tag found
                                // Check for partial tag at end of buffer
                                let partial_tags = ["<", "<t", "<th", "<thi", "<thin", "<think", "<thinki", "<thinkin", "<thinking"];
                                let mut split_idx = buffer.len();
                                
                                for tag in partial_tags.iter() {
                                    if buffer.ends_with(tag) {
                                        split_idx = buffer.len() - tag.len();
                                        break;
                                    }
                                }
                                
                                if split_idx < buffer.len() {
                                    // We have a partial tag at the end
                                    // Send everything before it
                                    if split_idx > 0 {
                                        let content = buffer[..split_idx].to_string();
                                        let event = Event::default().json_data(json!({
                                            "type": "token",
                                            "chatId": session_id_clone,
                                            "messageId": message_id,
                                            "content": content
                                        })).unwrap();
                                        let _ = tx.send(event).await;
                                        full_answer.push_str(&content);
                                    }
                                    // Keep partial tag in buffer
                                    buffer = buffer[split_idx..].to_string();
                                } else {
                                    // No partial tag, send all
                                    if !buffer.is_empty() {
                                        let event = Event::default().json_data(json!({
                                            "type": "token",
                                            "chatId": session_id_clone,
                                            "messageId": message_id,
                                            "content": buffer.clone()
                                        })).unwrap();
                                        let _ = tx.send(event).await;
                                        full_answer.push_str(&buffer);
                                        buffer.clear();
                                    }
                                }
                                break; // Done with this chunk
                            }
                        } else {
                            // Inside thinking block
                            if let Some(end_idx) = buffer.find("</thinking>") {
                                // Found end tag
                                // Send content before tag as reasoning
                                let reasoning = buffer[..end_idx].to_string();
                                if !reasoning.is_empty() {
                                    let event = Event::default().json_data(json!({
                                        "type": "reasoning",
                                        "chatId": session_id_clone,
                                        "messageId": message_id,
                                        "content": reasoning
                                    })).unwrap();
                                    let _ = tx.send(event).await;
                                }
                                // Advance buffer past tag
                                buffer = buffer[end_idx + 11..].to_string();
                                in_thinking_block = false;
                                // Continue loop to process content after tag
                                continue;
                            } else {
                                // No end tag found
                                // Check for partial end tag
                                let partial_tags = ["<", "<", "</", "</t", "</th", "</thi", "</thin", "</think", "</thinki", "</thinkin", "</thinking"];
                                let mut split_idx = buffer.len();
                                
                                for tag in partial_tags.iter() {
                                    if buffer.ends_with(tag) {
                                        split_idx = buffer.len() - tag.len();
                                        break;
                                    }
                                }
                                
                                if split_idx < buffer.len() {
                                    // Partial end tag at end
                                    // Send everything before as reasoning
                                    if split_idx > 0 {
                                        let content = buffer[..split_idx].to_string();
                                        let event = Event::default().json_data(json!({
                                            "type": "reasoning",
                                            "chatId": session_id_clone,
                                            "messageId": message_id,
                                            "content": content
                                        })).unwrap();
                                        let _ = tx.send(event).await;
                                    }
                                    // Keep partial tag in buffer
                                    buffer = buffer[split_idx..].to_string();
                                } else {
                                    // No partial tag, send all as reasoning
                                    if !buffer.is_empty() {
                                        let event = Event::default().json_data(json!({
                                            "type": "reasoning",
                                            "chatId": session_id_clone,
                                            "messageId": message_id,
                                            "content": buffer.clone()
                                        })).unwrap();
                                        let _ = tx.send(event).await;
                                        buffer.clear();
                                    }
                                }
                                break; // Done with this chunk
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Erreur stream: {err}");
                }
            }
        }
        
        // Flush remaining buffer
        if !buffer.is_empty() {
            if in_thinking_block {
                // Still in thinking block, send as reasoning event only
                let event = Event::default().json_data(json!({
                    "type": "reasoning",
                    "chatId": session_id_clone,
                    "messageId": message_id,
                    "content": buffer.clone()
                })).unwrap();
                let _ = tx.send(event).await;
                // DON'T add to full_answer
            } else {
                // Normal content, send as token
                let event = Event::default().json_data(json!({
                    "type": "token",
                    "chatId": session_id_clone,
                    "messageId": message_id,
                    "content": buffer.clone()
                })).unwrap();
                let _ = tx.send(event).await;
                full_answer.push_str(&buffer);
            }
        }

        if let Err(err) = sqlx::query!(
            r#"UPDATE chat_messages SET content = $2 WHERE id = $1"#,
            message_id,
            full_answer
        )
        .execute(&state_clone.db)
        .await
        {
            eprintln!("Impossible de mettre à jour la réponse IA: {err}");
        }

        match fetch_chat_session(&state_clone.db, session_id_clone).await {
            Ok(final_session) => {
                let event = Event::default()
                    .json_data(json!({
                        "type": "final",
                        "session": final_session,
                        "chatId": session_id_clone,
                        "messageId": message_id
                    }))
                    .map_err(|err| {
                        eprintln!("Erreur sérialisation event final: {err}");
                    });
                if let Ok(ev) = event {
                    let _ = tx.send(ev).await;
                }
            }
            Err(err) => {
                let event = Event::default()
                    .json_data(json!({
                        "type": "error",
                        "message": format!("{err}")
                    }))
                    .map_err(|ser_err| {
                        eprintln!("Erreur sérialisation event erreur: {ser_err}");
                    });
                if let Ok(ev) = event {
                    let _ = tx.send(ev).await;
                }
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| Ok(event));
    Ok(Sse::new(stream))
}

async fn regenerate_message(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<RegenerateRequest>,
) -> Result<Json<ChatSession>, (axum::http::StatusCode, String)> {
    let RegenerateRequest { message_id, model, completion_params } = payload;
    let messages = fetch_chat_messages(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    if messages.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Il n'y a aucune réponse à régénérer pour cette discussion.".to_string(),
        ));
    }

    let target_index = messages
        .iter()
        .position(|msg| msg.id == message_id)
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Message à régénérer introuvable.".to_string(),
        ))?;

    let target = &messages[target_index];
    if target.role != "assistant" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Seules les réponses de l'IA peuvent être régénérées.".to_string(),
        ));
    }

    if target_index != messages.len() - 1 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "La régénération n'est possible que sur la dernière réponse.".to_string(),
        ));
    }

    if target_index == 0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Impossible de régénérer sans question utilisateur.".to_string(),
        ));
    }

    let truncated = conversation_to_payload(&messages[..target_index]);

    if truncated.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Impossible de régénérer sans question utilisateur.".to_string(),
        ));
    }

    let ai_model = AiModelChoice::from_client(model.as_deref());
    if ai_model == AiModelChoice::GroqLlama31
        && messages.iter().any(|msg| !msg.attachments.is_empty())
    {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cette discussion contient des fichiers. Utilise un modèle OpenAI pour continuer."
                .to_string(),
        ));
    }
    if ai_model == AiModelChoice::GroqLlama31
        && messages.iter().any(|msg| !msg.attachments.is_empty())
    {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cette discussion contient des fichiers. Utilise un modèle OpenAI pour continuer."
                .to_string(),
        ));
    }
    let mut stream = request_ai_completion(&state, &truncated, ai_model, completion_params).await?;
    let mut answer = String::new();
    while let Some(chunk_res) = stream.next().await {
        if let Ok(chunk) = chunk_res {
            answer.push_str(&chunk);
        }
    }

    sqlx::query!(
        r#"
        UPDATE chat_messages
        SET content = $2
        WHERE id = $1
        "#,
        message_id,
        answer
    )
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    sqlx::query!(
        r#"UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1"#,
        session_id
    )
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    let session = fetch_chat_session(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(session))
}

async fn regenerate_message_stream(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<RegenerateRequest>,
) -> Result<
    Sse<impl futures::Stream<Item = Result<Event, Infallible>>>,
    (axum::http::StatusCode, String),
> {
    let RegenerateRequest { message_id, model, completion_params } = payload;
    let messages = fetch_chat_messages(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    if messages.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Il n'y a aucune réponse à régénérer pour cette discussion.".to_string(),
        ));
    }

    let target_index = messages
        .iter()
        .position(|msg| msg.id == message_id)
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Message à régénérer introuvable.".to_string(),
        ))?;

    let target = &messages[target_index];
    if target.role != "assistant" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Seules les réponses de l'IA peuvent être régénérées.".to_string(),
        ));
    }

    if target_index != messages.len() - 1 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "La régénération n'est possible que sur la dernière réponse.".to_string(),
        ));
    }

    let truncated = conversation_to_payload(&messages[..target_index]);

    if truncated.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Impossible de régénérer sans question utilisateur.".to_string(),
        ));
    }

    let ai_model = AiModelChoice::from_client(model.as_deref());
    let mut stream = request_ai_completion(&state, &truncated, ai_model, completion_params).await?;

    let mut placeholder_session = fetch_chat_session(&state.db, session_id)
        .await
        .map_err(internal_error)?;

    if let Some(msg) = placeholder_session.messages.iter_mut().find(|m| m.id == message_id) {
        msg.content.clear();
    }

    let (tx, rx) = mpsc::channel::<Event>(32);
    tx.send(
        Event::default()
            .json_data(json!({
                "type": "session",
                "session": placeholder_session,
                "chatId": session_id,
                "messageId": message_id
            }))
            .map_err(internal_error)?,
    )
    .await
    .map_err(|_| internal_error("Impossible d'envoyer l'évènement SSE initial"))?;

    let state_clone = state.clone();
    let session_id_clone = session_id;
    let message_id_clone = message_id;

    tokio::spawn(async move {
        let mut full_answer = String::new();
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    full_answer.push_str(&chunk);
                    let event = match Event::default().json_data(json!({
                        "type": "token",
                        "chatId": session_id_clone,
                        "messageId": message_id_clone,
                        "content": chunk
                    })) {
                        Ok(ev) => ev,
                        Err(err) => {
                            eprintln!("Impossible de sérialiser le chunk SSE: {err}");
                            continue;
                        }
                    };
                    if tx.send(event).await.is_err() {
                        return;
                    }
                }
                Err(err) => {
                    eprintln!("Erreur stream: {err}");
                }
            }
        }

        if let Err(err) = sqlx::query!(
            r#"UPDATE chat_messages SET content = $2 WHERE id = $1"#,
            message_id_clone,
            full_answer
        )
        .execute(&state_clone.db)
        .await
        {
            eprintln!("Impossible de mettre à jour la réponse IA: {err}");
        }

        match fetch_chat_session(&state_clone.db, session_id_clone).await {
            Ok(final_session) => {
                let _ = tx
                    .send(
                        Event::default()
                            .json_data(json!({
                                "type": "final",
                                "session": final_session,
                                "chatId": session_id_clone,
                                "messageId": message_id_clone
                            }))
                            .unwrap_or_else(|err| {
                                Event::default()
                                    .data(format!("{{\"type\":\"error\",\"message\":\"{err}\"}}"))
                            }),
                    )
                    .await;
            }
            Err(err) => {
                let _ = tx
                    .send(
                        Event::default()
                            .json_data(json!({
                                "type": "error",
                                "message": format!("{err}")
                            }))
                            .unwrap_or_else(|ser_err| {
                                Event::default().data(format!(
                                    "{{\"type\":\"error\",\"message\":\"{ser_err}\"}}"
                                ))
                            }),
                    )
                    .await;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| Ok(event));
    Ok(Sse::new(stream))
}

async fn archive_chat_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let result = sqlx::query!(
        r#"
        UPDATE chat_sessions
        SET archived = TRUE, updated_at = NOW()
        WHERE id = $1 AND archived = FALSE
        "#,
        session_id
    )
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM chat_sessions WHERE id = $1) AS "exists!""#,
            session_id
        )
        .fetch_one(&state.db)
        .await
        .map_err(internal_error)?;

        if exists {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Cette discussion est déjà archivée.".to_string(),
            ));
        } else {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Discussion introuvable.".to_string(),
            ));
        }
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn delete_chat_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let result = sqlx::query!(r#"DELETE FROM chat_sessions WHERE id = $1"#, session_id)
        .execute(&state.db)
        .await
        .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Discussion introuvable.".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn fetch_chat_messages(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            session_id,
            role,
            content,
            position,
            created_at as "created_at: chrono::DateTime<chrono::Utc>"
        FROM chat_messages
        WHERE session_id = $1
        ORDER BY position ASC
        "#,
        session_id
    )
    .fetch_all(pool)
    .await?;
    let message_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let mut attachments_by_message: HashMap<Uuid, Vec<ChatAttachment>> = HashMap::new();

    if !message_ids.is_empty() {
        let attachment_rows = sqlx::query!(
            r#"
            SELECT
                id,
                message_id,
                file_name,
                mime_type,
                size_bytes,
                url,
                storage_key,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM chat_attachments
            WHERE message_id = ANY($1)
            ORDER BY created_at ASC
            "#,
            &message_ids
        )
        .fetch_all(pool)
        .await?;

        for row in attachment_rows {
            attachments_by_message
                .entry(row.message_id)
                .or_default()
                .push(ChatAttachment {
                    id: row.id,
                    message_id: row.message_id,
                    file_name: row.file_name,
                    mime_type: row.mime_type,
                    size_bytes: row.size_bytes,
                    url: row.url,
                    storage_key: row.storage_key,
                    created_at: row.created_at,
                });
        }
    }

    Ok(rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            session_id: row.session_id,
            role: row.role,
            content: row.content,
            position: row.position,
            created_at: row.created_at,
            attachments: attachments_by_message.remove(&row.id).unwrap_or_default(),
        })
        .collect())
}

async fn fetch_chat_session(pool: &PgPool, session_id: Uuid) -> Result<ChatSession, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT
            id,
            title,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            archived
        FROM chat_sessions
        WHERE id = $1
        "#,
        session_id
    )
    .fetch_one(pool)
    .await?;

    let messages = fetch_chat_messages(pool, session_id).await?;

    Ok(ChatSession {
        id: row.id,
        title: row.title,
        created_at: row.created_at,
        updated_at: row.updated_at,
        archived: row.archived,
        messages,
    })
}

async fn request_ai_completion(
    state: &AppState,
    messages: &[ChatMessagePayload],
    model: AiModelChoice,
    params: Option<CompletionParams>,
) -> Result<BoxStream<'static, Result<String, String>>, (axum::http::StatusCode, String)> {
    request_model_completion(state, &with_system_prompt(messages), model, params).await
}

async fn request_model_completion(
    state: &AppState,
    messages: &[ChatMessagePayload],
    model: AiModelChoice,
    params: Option<CompletionParams>,
) -> Result<BoxStream<'static, Result<String, String>>, (axum::http::StatusCode, String)> {
    match model {
        AiModelChoice::GroqLlama31 => request_groq_completion(messages).await,
        AiModelChoice::OpenAIGpt51
        | AiModelChoice::OpenAIGpt5Mini
        | AiModelChoice::OpenAIGpt5Nano
        | AiModelChoice::OpenAIGpt5Pro
        | AiModelChoice::OpenAIGpt5
        | AiModelChoice::OpenAIGpt41 => request_openai_completion(state, messages, model, params).await,
    }
}

async fn request_groq_completion(
    messages: &[ChatMessagePayload],
) -> Result<BoxStream<'static, Result<String, String>>, (axum::http::StatusCode, String)> {
    if messages.iter().any(|msg| !msg.attachments.is_empty()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Les fichiers ne sont pas supportés par ce modèle.".to_string(),
        ));
    }

    let api_key =
        env::var("GROQ_API_KEY").map_err(|_| internal_error("GROQ_API_KEY manquant dans .env"))?;

    let client = Client::new();

    let simple_messages: Vec<Value> = messages
        .iter()
        .map(|msg| {
            json!({
                "role": msg.role,
                "content": msg.content,
            })
        })
        .collect();

    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": AiModelChoice::GroqLlama31.model_id(),
            "messages": simple_messages,
            "stream": true
        }))
        .send()
        .await
        .map_err(internal_error)?;

    let status = res.status();
    if !status.is_success() {
        let body_text = res.text().await.unwrap_or_default();
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            format!("Erreur Groq: HTTP {status} - {body_text}"),
        ));
    }

    Ok(process_stream(Box::pin(res.bytes_stream())))
}

async fn request_openai_completion(
    state: &AppState,
    messages: &[ChatMessagePayload],
    model: AiModelChoice,
    params: Option<CompletionParams>,
) -> Result<BoxStream<'static, Result<String, String>>, (axum::http::StatusCode, String)> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| internal_error("OPENAI_API_KEY manquant dans .env"))?;

    let client = Client::new();
    let mut formatted_messages = Vec::with_capacity(messages.len());
    for message in messages {
        let mut parts = Vec::new();
        if !message.content.trim().is_empty() {
            parts.push(json!({ "type": "text", "text": message.content }));
        }
        for attachment in &message.attachments {
            match load_attachment_content(attachment, state).await? {
                AttachmentContent::Image(url) => parts.push(json!({
                    "type": "image_url",
                    "image_url": { "url": url }
                })),
                AttachmentContent::Text(text) => parts.push(json!({
                    "type": "text",
                    "text": text
                })),
            }
        }
        if parts.is_empty() {
            parts.push(json!({ "type": "text", "text": "" }));
        }
        formatted_messages.push(json!({
            "role": message.role,
            "content": parts
        }));
    }
    let params = params.unwrap_or_default();
    
    // Construct request body - serde will skip None values
    let mut request_body = json!({
        "model": model.model_id(),
        "messages": formatted_messages,
        "stream": true,
    });
    
    // Manually add optional params only if Some
    if let Some(temp) = params.temperature {
        request_body["temperature"] = json!(temp);
    }
    if let Some(max_tok) = params.max_tokens {
        request_body["max_tokens"] = json!(max_tok);
    }
    if let Some(top) = params.top_p {
        request_body["top_p"] = json!(top);
    }
    if let Some(pres) = params.presence_penalty {
        request_body["presence_penalty"] = json!(pres);
    }
    if let Some(freq) = params.frequency_penalty {
        request_body["frequency_penalty"] = json!(freq);
    }
    if let Some(s) = params.seed {
        request_body["seed"] = json!(s);
    }
    
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("x-openai-processing-tier", "standard")
        .json(&request_body)
        .send()
        .await
        .map_err(internal_error)?;

    let status = res.status();
    if !status.is_success() {
        let body_text = res.text().await.unwrap_or_default();
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            format!("Erreur OpenAI: HTTP {status} - {body_text}"),
        ));
    }

    Ok(process_stream(Box::pin(res.bytes_stream())))
}

fn process_stream(
    stream: BoxStream<'static, Result<Bytes, reqwest::Error>>,
) -> BoxStream<'static, Result<String, String>> {
    Box::pin(stream::unfold(
        (stream, String::new()),
        |(mut stream, mut buffer)| async move {
            loop {
                if let Some(idx) = buffer.find('\n') {
                    let line = buffer[..idx].to_string();
                    buffer = buffer[idx + 1..].to_string();
                    let line = line.trim();
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            return None;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                                return Some((Ok(content.to_string()), (stream, buffer)));
                            }
                        }
                    }
                    continue;
                }

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                    }
                    Some(Err(e)) => return Some((Err(e.to_string()), (stream, buffer))),
                    None => return None,
                }
            }
        },
    ))
}

fn with_system_prompt(messages: &[ChatMessagePayload]) -> Vec<ChatMessagePayload> {
    let mut result = Vec::with_capacity(messages.len() + 1);
    result.push(ChatMessagePayload {
        role: "system".to_string(),
        content: SYSTEM_PROMPT.to_string(),
        attachments: Vec::new(),
    });
    result.extend(messages.iter().cloned());
    result
}

async fn generate_concise_title(
    state: &AppState,
    content: &str,
    model: AiModelChoice,
) -> Result<String, (axum::http::StatusCode, String)> {
    let messages = vec![
        ChatMessagePayload {
            role: "system".to_string(),
            content: TITLE_SUMMARY_PROMPT.to_string(),
            attachments: Vec::new(),
        },
        ChatMessagePayload {
            role: "user".to_string(),
            content: format!("Question: {content}"),
            attachments: Vec::new(),
        },
    ];

    let mut stream = request_model_completion(state, &messages, model, None).await?;
    let mut summary = String::new();
    while let Some(chunk_res) = stream.next().await {
        if let Ok(chunk) = chunk_res {
            summary.push_str(&chunk);
        }
    }

    let cleaned = summary.lines().next().unwrap_or("").trim();
    if cleaned.is_empty() {
        Err((
            axum::http::StatusCode::BAD_GATEWAY,
            "Aucun résumé n'a été renvoyé pour le titre.".to_string(),
        ))
    } else {
        Ok(cleaned.to_string())
    }
}

fn preview_chat_title(message: &str) -> String {
    const MAX_CHARS: usize = 60;
    let mut preview = String::new();
    let mut truncated = false;

    for (idx, ch) in message.chars().enumerate() {
        if idx >= MAX_CHARS {
            truncated = true;
            break;
        }
        preview.push(ch);
    }

    if truncated {
        preview.push('…');
    }

    preview
}

async fn insert_chat_attachments(
    pool: &PgPool,
    message_id: Uuid,
    attachments: &[AttachmentPayload],
) -> Result<(), sqlx::Error> {
    for attachment in attachments {
        let storage_key = attachment
            .storage_key
            .clone()
            .or_else(|| storage_key_from_url(&attachment.url))
            .unwrap_or_default();
        if storage_key.is_empty() {
            continue;
        }
        sqlx::query!(
            r#"
            INSERT INTO chat_attachments (message_id, file_name, mime_type, size_bytes, url, storage_key)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            message_id,
            attachment.file_name,
            attachment.mime_type,
            attachment.size_bytes,
            attachment.url,
            storage_key
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

fn chunk_text_for_streaming(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    let chunk_size = 30;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }

    chunks
}

fn conversation_to_payload(messages: &[ChatMessage]) -> Vec<ChatMessagePayload> {
    messages
        .iter()
        .map(|msg| ChatMessagePayload {
            role: msg.role.clone(),
            content: msg.content.clone(),
            attachments: msg
                .attachments
                .iter()
                .map(|attachment| AttachmentPayload {
                    file_name: attachment.file_name.clone(),
                    mime_type: attachment.mime_type.clone(),
                    size_bytes: attachment.size_bytes,
                    url: attachment.url.clone(),
                    storage_key: Some(attachment.storage_key.clone()),
                })
                .collect(),
        })
        .collect()
}

fn sanitize_ai_response(text: &str) -> String {
    let inline = convert_inline_parentheses(text);
    let display = convert_display_brackets(&inline);
    wrap_allowed_environments(&display)
}

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '-',
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "fichier".to_string()
    } else {
        trimmed.to_string()
    }
}

fn storage_key_from_url(url: &str) -> Option<String> {
    let segment = url.rsplit('/').next()?.split('?').next()?.trim();
    if segment.is_empty() {
        None
    } else {
        Some(segment.to_string())
    }
}

fn attachment_local_path(upload_dir: &str, storage_key: &str) -> PathBuf {
    let mut path = PathBuf::from(upload_dir);
    path.push(storage_key);
    path
}

fn convert_inline_parentheses(text: &str) -> String {
    convert_math_block(text, "\\(", "\\)", "$", "$")
}

fn convert_display_brackets(text: &str) -> String {
    convert_math_block(text, "\\[", "\\]", "$$\n", "\n$$")
}

fn convert_math_block(text: &str, open: &str, close: &str, prefix: &str, suffix: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut idx = 0;
    while let Some(rel_start) = text[idx..].find(open) {
        let start = idx + rel_start;
        result.push_str(&text[idx..start]);
        let inner_start = start + open.len();
        if let Some(rel_end) = text[inner_start..].find(close) {
            let end = inner_start + rel_end;
            let inner = text[inner_start..end].trim();
            result.push_str(prefix);
            result.push_str(inner);
            result.push_str(suffix);
            idx = end + close.len();
        } else {
            result.push_str(&text[start..]);
            return result;
        }
    }
    result.push_str(&text[idx..]);
    result
}

fn wrap_allowed_environments(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 32);
    let mut cursor = 0;
    while let Some(rel_start) = text[cursor..].find("\\begin{") {
        let start = cursor + rel_start;
        let env_name_start = start + "\\begin{".len();
        if let Some(env_name_end_rel) = text[env_name_start..].find('}') {
            let env_name_end = env_name_start + env_name_end_rel;
            let env_name = &text[env_name_start..env_name_end];
            if ALLOWED_MATH_ENVIRONMENTS.contains(&env_name) {
                let end_marker = format!("\\end{{{}}}", env_name);
                if let Some(end_rel) = text[env_name_end + 1..].find(&end_marker) {
                    let block_end = env_name_end + 1 + end_rel + end_marker.len();
                    result.push_str(&text[cursor..start]);

                    let has_prefix = text[..start]
                        .trim_end_matches(|c: char| c.is_whitespace())
                        .ends_with("$$");
                    let has_suffix = text[block_end..]
                        .trim_start_matches(|c: char| c.is_whitespace())
                        .starts_with("$$");

                    if has_prefix && has_suffix {
                        result.push_str(&text[start..block_end]);
                    } else {
                        result.push_str("$$\n");
                        result.push_str(&text[start..block_end]);
                        result.push_str("\n$$");
                    }

                    cursor = block_end;
                    continue;
                }
            }
        }

        let fallback_end = (start + "\\begin{".len()).min(text.len());
        result.push_str(&text[cursor..fallback_end]);
        cursor = fallback_end;
    }

    result.push_str(&text[cursor..]);
    result
}

enum AttachmentContent {
    Image(String),
    Text(String),
}

async fn load_attachment_content(
    attachment: &AttachmentPayload,
    state: &AppState,
) -> Result<AttachmentContent, (axum::http::StatusCode, String)> {
    let storage_key = attachment
        .storage_key
        .clone()
        .or_else(|| storage_key_from_url(&attachment.url));
    if storage_key.is_none() {
        if attachment.mime_type.starts_with("image/") {
            return Ok(AttachmentContent::Image(attachment.url.clone()));
        }
        return Ok(AttachmentContent::Text(format!(
            "Fichier attaché: {} ({}).\n{}",
            attachment.file_name, attachment.mime_type, attachment.url
        )));
    }
    let key = storage_key.unwrap();

    let path = attachment_local_path(&state.upload_dir, &key);
    let data = tokio::fs::read(&path).await.map_err(internal_error)?;

    if attachment.mime_type.starts_with("image/") {
        let data_url = format!(
            "data:{};base64,{}",
            attachment.mime_type,
            general_purpose::STANDARD.encode(data)
        );
        Ok(AttachmentContent::Image(data_url))
    } else if attachment.mime_type == "application/pdf" {
        match suppress_output(|| extract_text_from_mem(&data)) {
            Ok(text) => Ok(AttachmentContent::Text(truncate_text(&text))),
            Err(err) => Err(internal_error(err)),
        }
    } else if let Ok(text) = String::from_utf8(data.clone()) {
        Ok(AttachmentContent::Text(truncate_text(&text)))
    } else {
        Ok(AttachmentContent::Text(format!(
            "Fichier attaché (encodé en base64) {}:\n{}",
            attachment.file_name,
            general_purpose::STANDARD.encode(data)
        )))
    }
}

fn truncate_text(text: &str) -> String {
    const MAX_CHARS: usize = 50_000;
    if text.len() <= MAX_CHARS {
        text.to_string()
    } else {
        format!(
            "{}\n\n[Texte tronqué, {} premiers caractères sur {}]",
            &text[..MAX_CHARS],
            MAX_CHARS,
            text.len()
        )
    }
}

#[cfg(unix)]
fn suppress_output<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    unsafe {
        let stdout_fd = libc::STDOUT_FILENO;
        let stderr_fd = libc::STDERR_FILENO;
        let stdout_dup = libc::dup(stdout_fd);
        let stderr_dup = libc::dup(stderr_fd);
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_WRONLY);
        if devnull >= 0 {
            libc::dup2(devnull, stdout_fd);
            libc::dup2(devnull, stderr_fd);
            libc::close(devnull);
        }
        let result = f();
        if stdout_dup >= 0 {
            libc::dup2(stdout_dup, stdout_fd);
            libc::close(stdout_dup);
        }
        if stderr_dup >= 0 {
            libc::dup2(stderr_dup, stderr_fd);
            libc::close(stderr_dup);
        }
        result
    }
}

#[cfg(not(unix))]
fn suppress_output<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    f()
}
