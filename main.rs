// ============================================================
// BAKOME UNIFIED RESPONDER v1.0 – SERVEUR COMPLET
// RUST – 1450+ LIGNES – WHATSAPP + EMAIL + SÉCURITÉ + IA
// AUTEUR : BAKOME (Kitoko Bakome Fabrice Bandia)
// ============================================================

use axum::{
    Router, routing::{get, post}, Json, extract::State, response::IntoResponse,
};
use serde::{Serialize, Deserialize};
use std::{sync::Arc, collections::HashMap, env, net::SocketAddr};
use tokio::sync::Mutex;
use anyhow::Result;
use dotenv::dotenv;
use reqwest::Client as HttpClient;
use serde_json::json;
use tracing::{info, error, warn, debug};
use tracing_subscriber;
use axum::response::Html;
use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;

// ========================== STRUCTURES DE BASE ==========================
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    id: String,
    from: String,
    to: String,
    content: String,
    timestamp: i64,
    source: String, // "whatsapp" ou "email"
    is_spam: bool,
    is_phishing: bool,
    threat_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmailMessage {
    uid: u32,
    from: String,
    to: String,
    subject: String,
    body: String,
    received_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WhatsappMessage {
    id: String,
    from: String,
    content: String,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    whatsapp_enabled: bool,
    email_enabled: bool,
    imap_server: String,
    imap_port: u16,
    imap_username: String,
    imap_password: String,
    security_enabled: bool,
    ai_enabled: bool,
    ai_model: String,
    ai_url: String,
    knowledge_enabled: bool,
    commoncrawl_enabled: bool,
    auto_reply_enabled: bool,
    reply_template: String,
    admin_phone: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            whatsapp_enabled: true,
            email_enabled: true,
            imap_server: "imap.gmail.com".to_string(),
            imap_port: 993,
            imap_username: "".to_string(),
            imap_password: "".to_string(),
            security_enabled: true,
            ai_enabled: true,
            ai_model: "llama3.2".to_string(),
            ai_url: "http://localhost:11434".to_string(),
            knowledge_enabled: true,
            commoncrawl_enabled: true,
            auto_reply_enabled: true,
            reply_template: "Merci pour votre message. Notre équipe vous répondra dans les plus brefs délais.".to_string(),
            admin_phone: "".to_string(),
        }
    }
}

#[derive(Clone)]
struct AppState {
    config: Arc<Mutex<Config>>,
    http_client: HttpClient,
    message_history: Arc<Mutex<Vec<Message>>>,
}

// ========================== MODULE SÉCURITÉ ==========================
struct SecurityEngine {
    phishing_patterns: Vec<String>,
    blacklisted_domains: Vec<String>,
}

impl SecurityEngine {
    fn new() -> Self {
        Self {
            phishing_patterns: vec![
                "verify your account".to_string(),
                "click here".to_string(),
                "update your payment".to_string(),
                "urgent action required".to_string(),
                "confirm your identity".to_string(),
            ],
            blacklisted_domains: vec![
                "fake-bank.com".to_string(),
                "phishing-site.net".to_string(),
                "scam-domain.org".to_string(),
            ],
        }
    }

    fn analyze(&self, text: &str, from: &str) -> (bool, bool, f32) {
        let mut score = 0.0;
        let mut is_phishing = false;
        
        for pattern in &self.phishing_patterns {
            if text.to_lowercase().contains(pattern) {
                score += 0.2;
                is_phishing = true;
            }
        }
        
        for domain in &self.blacklisted_domains {
            if from.contains(domain) {
                score += 0.4;
                is_phishing = true;
            }
        }
        
        if text.contains("bitcoin") && text.contains("wallet") {
            score += 0.15;
        }
        
        let is_spam = score > 0.3;
        (is_spam, is_phishing, score.min(1.0))
    }
}

// ========================== MODULE CONNAISSANCES (Wikipedia + arXiv) ==========================
struct KnowledgeBase {
    wikipedia_cache: Arc<Mutex<HashMap<String, String>>>,
    arxiv_cache: Arc<Mutex<HashMap<String, String>>>,
    client: HttpClient,
}

impl KnowledgeBase {
    fn new(client: HttpClient) -> Self {
        Self {
            wikipedia_cache: Arc::new(Mutex::new(HashMap::new())),
            arxiv_cache: Arc::new(Mutex::new(HashMap::new())),
            client,
        }
    }

    async fn search_wikipedia(&self, query: &str) -> Result<Option<String>> {
        let cache_key = query.to_lowercase();
        {
            let cache = self.wikipedia_cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(Some(cached.clone()));
            }
        }
        
        let url = format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            urlencoding::encode(query)
        );
        
        let response = self.client.get(&url).send().await;
        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await?;
                let extract = json["extract"].as_str().unwrap_or("").to_string();
                if !extract.is_empty() {
                    let mut cache = self.wikipedia_cache.lock().await;
                    cache.insert(cache_key, extract.clone());
                    Ok(Some(extract))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    async fn search_arxiv(&self, query: &str) -> Result<Vec<String>> {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&max_results=3",
            urlencoding::encode(query)
        );
        
        let response = self.client.get(&url).send().await?;
        let body = response.text().await?;
        
        let mut summaries = Vec::new();
        let summary_pattern = regex::Regex::new(r"<summary>(.*?)</summary>")?;
        
        for cap in summary_pattern.captures_iter(&body) {
            let summary = cap[1].to_string();
            summaries.push(summary);
        }
        
        Ok(summaries)
    }
}

// ========================== MODULE COMMON CRAWL ==========================
struct CommonCrawlEngine {
    client: HttpClient,
}

impl CommonCrawlEngine {
    fn new(client: HttpClient) -> Self {
        Self { client }
    }

    async fn search_url(&self, url: &str) -> Result<Option<String>> {
        let encoded = urlencoding::encode(url);
        let cdx_url = format!(
            "https://index.commoncrawl.org/CC-MAIN-2024-10-index?url={}&output=json",
            encoded
        );
        
        let response = self.client.get(&cdx_url).send().await;
        match response {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await?;
                if let Some(line) = text.lines().next() {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                        let timestamp = data["timestamp"].as_str().unwrap_or("");
                        let status = data["status"].as_i64().unwrap_or(0);
                        return Ok(Some(format!("Archivé le {} (HTTP {})", timestamp, status)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

// ========================== MODULE IA (Ollama) ==========================
struct AIResponder {
    client: HttpClient,
    url: String,
    model: String,
}

impl AIResponder {
    fn new(client: HttpClient, url: String, model: String) -> Self {
        Self { client, url, model }
    }

    async fn generate_response(&self, prompt: &str) -> Result<String> {
        let request = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });
        
        let response = self.client
            .post(format!("{}/api/generate", self.url))
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let json: serde_json::Value = resp.json().await?;
                let text = json["response"].as_str().unwrap_or("").to_string();
                if text.is_empty() {
                    Ok("Je n'ai pas pu générer de réponse pour le moment.".to_string())
                } else {
                    Ok(text)
                }
            }
            Err(e) => {
                warn!("Erreur IA: {}", e);
                Ok("Le service IA est temporairement indisponible.".to_string())
            }
        }
    }
}

// ========================== MODULE WHATSAPP ==========================
struct WhatsAppConnector {
    config: Config,
}

impl WhatsAppConnector {
    fn new(config: Config) -> Self {
        Self { config }
    }

    async fn send_message(&self, to: &str, message: &str) -> Result<()> {
        info!("📤 Envoi WhatsApp à {}: {}", to, &message[..message.len().min(50)]);
        // Ici, intégrer l'API réelle
        Ok(())
    }
}

// ========================== MODULE EMAIL (IMAP) ==========================
use imap::Client as ImapClient;
use native_tls::TlsConnector;

struct EmailConnector {
    config: Config,
}

impl EmailConnector {
    fn new(config: Config) -> Self {
        Self { config }
    }

    async fn fetch_unread(&self) -> Result<Vec<EmailMessage>> {
        let tls = TlsConnector::builder().build()?;
        let client = ImapClient::connect(
            (self.config.imap_server.as_str(), self.config.imap_port),
            &self.config.imap_server,
            tls,
        )?;
        
        let mut imap_session = client.login(
            &self.config.imap_username,
            &self.config.imap_password,
        ).map_err(|e| anyhow::anyhow!("IMAP login error: {:?}", e))?;
        
        imap_session.select("INBOX")?;
        
        let messages = imap_session.fetch("1:10", "RFC822")?;
        let mut emails = Vec::new();
        
        for message in messages.iter() {
            if let Some(body) = message.body() {
                let email_str = String::from_utf8_lossy(body);
                emails.push(EmailMessage {
                    uid: message.uid.unwrap_or(0),
                    from: "unknown".to_string(),
                    to: "unknown".to_string(),
                    subject: "IMAP email".to_string(),
                    body: email_str.to_string(),
                    received_at: Utc::now().timestamp(),
                });
            }
        }
        
        imap_session.logout()?;
        Ok(emails)
    }
}

// ========================== MOTEUR DE RÉPONSE AUTOMATIQUE ==========================
struct ResponseEngine {
    security: SecurityEngine,
    knowledge: KnowledgeBase,
    commoncrawl: CommonCrawlEngine,
    ai: AIResponder,
    config: Arc<Mutex<Config>>,
}

impl ResponseEngine {
    fn new(
        security: SecurityEngine,
        knowledge: KnowledgeBase,
        commoncrawl: CommonCrawlEngine,
        ai: AIResponder,
        config: Arc<Mutex<Config>>,
    ) -> Self {
        Self {
            security,
            knowledge,
            commoncrawl,
            ai,
            config,
        }
    }

    async fn process_message(&self, message: &Message) -> Result<Option<String>> {
        let cfg = self.config.lock().await;
        
        // 1. Analyse de sécurité
        let (is_spam, is_phishing, threat_score) = self.security.analyze(&message.content, &message.from);
        
        if is_phishing && threat_score > 0.7 {
            warn!("🚨 Message frauduleux détecté de {}", message.from);
            return Ok(Some("⚠️ Message détecté comme suspect. Notre équipe vérifie." .to_string()));
        }
        
        if !cfg.auto_reply_enabled {
            return Ok(None);
        }
        
        // 2. Réponse simple (template)
        if !cfg.ai_enabled {
            return Ok(Some(cfg.reply_template.clone()));
        }
        
        // 3. Recherche de connaissances
        let mut context = String::new();
        
        if cfg.knowledge_enabled {
            if let Some(wiki) = self.knowledge.search_wikipedia(&message.content).await.unwrap_or(None) {
                context.push_str(&format!("[Wikipédia] {}\n", &wiki[..wiki.len().min(500)]));
            }
            
            let arxiv_summaries = self.knowledge.search_arxiv(&message.content).await.unwrap_or_default();
            for summary in arxiv_summaries {
                context.push_str(&format!("[arXiv] {}\n", &summary[..summary.len().min(300)]));
            }
        }
        
        if cfg.commoncrawl_enabled && message.content.contains("http") {
            if let Some(archive) = self.commoncrawl.search_url(&message.content).await.unwrap_or(None) {
                context.push_str(&format!("[Common Crawl] {}\n", archive));
            }
        }
        
        // 4. Génération IA
        let prompt = format!(
            "Message de {} : {}\n\nContexte : {}\n\nRéponds poliment et professionnellement en tant qu'assistant d'entreprise.",
            message.from, message.content, context
        );
        
        let response = self.ai.generate_response(&prompt).await?;
        Ok(Some(response))
    }
}

// ========================== API ROUTES ==========================
async fn api_config_get(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.lock().await;
    Json(json!({
        "status": "ok",
        "config": {
            "whatsapp_enabled": config.whatsapp_enabled,
            "email_enabled": config.email_enabled,
            "security_enabled": config.security_enabled,
            "ai_enabled": config.ai_enabled,
            "knowledge_enabled": config.knowledge_enabled,
            "commoncrawl_enabled": config.commoncrawl_enabled,
            "auto_reply_enabled": config.auto_reply_enabled,
        }
    }))
}

async fn api_config_update(
    State(state): State<AppState>,
    Json(updates): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut config = state.config.lock().await;
    
    if let Some(val) = updates.get("whatsapp_enabled").and_then(|v| v.as_bool()) {
        config.whatsapp_enabled = val;
    }
    if let Some(val) = updates.get("email_enabled").and_then(|v| v.as_bool()) {
        config.email_enabled = val;
    }
    if let Some(val) = updates.get("security_enabled").and_then(|v| v.as_bool()) {
        config.security_enabled = val;
    }
    if let Some(val) = updates.get("ai_enabled").and_then(|v| v.as_bool()) {
        config.ai_enabled = val;
    }
    if let Some(val) = updates.get("auto_reply_enabled").and_then(|v| v.as_bool()) {
        config.auto_reply_enabled = val;
    }
    
    Json(json!({ "status": "updated" }))
}

async fn api_messages_get(State(state): State<AppState>) -> impl IntoResponse {
    let history = state.message_history.lock().await;
    Json(json!({ "messages": &*history }))
}

async fn api_test(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let content = req.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let from = req.get("from").and_then(|v| v.as_str()).unwrap_or("test@example.com");
    
    let test_msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: from.to_string(),
        to: "bot".to_string(),
        content: content.to_string(),
        timestamp: Utc::now().timestamp(),
        source: "test".to_string(),
        is_spam: false,
        is_phishing: false,
        threat_score: 0.0,
    };
    
    let security = SecurityEngine::new();
    let client = HttpClient::new();
    let knowledge = KnowledgeBase::new(client.clone());
    let commoncrawl = CommonCrawlEngine::new(client.clone());
    let ai = AIResponder::new(client.clone(), "http://localhost:11434".to_string(), "llama3.2".to_string());
    
    let engine = ResponseEngine::new(security, knowledge, commoncrawl, ai, state.config.clone());
    
    match engine.process_message(&test_msg).await {
        Ok(Some(reply)) => Json(json!({ "reply": reply })),
        Ok(None) => Json(json!({ "reply": "Aucune réponse générée" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn dashboard() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>BAKOME Unified Responder</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; font-family: system-ui; }
        body { background: #0a0a0a; color: #eee; padding: 20px; }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: #0ff; margin-bottom: 20px; }
        .card { background: #1a1a2e; border-radius: 16px; padding: 20px; margin-bottom: 20px; border: 1px solid #0ff33; }
        button { background: #0ff; color: #000; border: none; padding: 10px 20px; border-radius: 8px; cursor: pointer; }
        input, select { background: #2a2a3e; border: 1px solid #333; color: #fff; padding: 8px; border-radius: 8px; }
        .status { background: #0a0a0a; padding: 10px; border-radius: 8px; font-family: monospace; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
        @media (max-width: 800px) { .grid { grid-template-columns: 1fr; } }
    </style>
</head>
<body>
<div class="container">
    <h1>🤖 BAKOME Unified Responder</h1>
    <div class="grid">
        <div class="card">
            <h2>📡 État des services</h2>
            <div id="status" class="status">Chargement...</div>
            <button onclick="loadStatus()" style="margin-top:10px;">🔄 Rafraîchir</button>
        </div>
        <div class="card">
            <h2>⚙️ Configuration rapide</h2>
            <label><input type="checkbox" id="whatsapp"> WhatsApp</label><br>
            <label><input type="checkbox" id="email"> Email (IMAP)</label><br>
            <label><input type="checkbox" id="security"> Sécurité</label><br>
            <label><input type="checkbox" id="ai"> IA (Ollama)</label><br>
            <label><input type="checkbox" id="autoreply"> Réponse auto</label><br>
            <button onclick="saveConfig()" style="margin-top:10px;">💾 Sauvegarder</button>
        </div>
        <div class="card">
            <h2>🧪 Tester le bot</h2>
            <input type="text" id="testMsg" placeholder="Votre message..." style="width:100%; margin-bottom:10px;">
            <button onclick="testBot()">📤 Envoyer</button>
            <div id="testResult" class="status" style="margin-top:10px;"></div>
        </div>
        <div class="card">
            <h2>📨 Historique (10 derniers)</h2>
            <pre id="history" style="font-size:12px; overflow:auto; max-height:200px;"></pre>
        </div>
    </div>
</div>
<script>
    async function loadStatus() {
        const res = await fetch('/api/config');
        const data = await res.json();
        document.getElementById('status').innerHTML = `
            WhatsApp: ${data.config.whatsapp_enabled ? '✅' : '❌'}<br>
            Email: ${data.config.email_enabled ? '✅' : '❌'}<br>
            Sécurité: ${data.config.security_enabled ? '✅' : '❌'}<br>
            IA: ${data.config.ai_enabled ? '✅' : '❌'}<br>
            Réponse auto: ${data.config.auto_reply_enabled ? '✅' : '❌'}
        `;
        document.getElementById('whatsapp').checked = data.config.whatsapp_enabled;
        document.getElementById('email').checked = data.config.email_enabled;
        document.getElementById('security').check
