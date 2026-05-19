```markdown
# BAKOME Unified Responder

AI-powered unified responder for WhatsApp & Email – security, knowledge base (Wikipedia/arXiv), Common Crawl integration, auto-reply, and admin dashboard. Built in Rust. Open source (MIT).

## Features

- 📱 **WhatsApp & Email monitoring** (IMAP support)
- 🛡️ **Security engine** – phishing detection, spam filtering, threat scoring
- 📚 **Local knowledge base** – Wikipedia + arXiv (offline-capable)
- 🌐 **Common Crawl integration** – web archive search for URLs
- 🤖 **AI responder** – Ollama integration (Llama 3.2, Mistral, etc.)
- 🎛️ **Web dashboard** – real-time config, testing, logs
- 🔓 **100% open source** – MIT license

## Quick start

```bash
git clone https://github.com/BAKOME-Hub/BAKOME-Unified-Responder.git
cd BAKOME-Unified-Responder
cargo build --release
cargo run --release
```

Then open http://localhost:3001

Configuration

Edit src/main.rs or use the web dashboard to enable/disable:

· WhatsApp / Email connectors
· Security engine
· AI responder
· Auto-reply
· Knowledge base (Wikipedia/arXiv)
· Common Crawl

Environment variables (optional)

```bash
IMAP_SERVER=imap.gmail.com
IMAP_USER=your_email@gmail.com
IMAP_PASS=your_password
OLLAMA_URL=http://localhost:11434
```

Dependencies

· Rust 1.70+
· Ollama (optional, for AI responses)
· IMAP server credentials (for email)

Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Web Dashboard                      │
│              (Axum + HTML/JS)                       │
└─────────────────────┬───────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────┐
│                   API Routes                         │
│          config / messages / test                    │
└─────────────────────┬───────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────┐
│               Response Engine                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │Security  │ │Knowledge │ │Common    │            │
│  │Engine    │ │Base      │ │Crawl     │            │
│  └──────────┘ └──────────┘ └──────────┘            │
│  ┌──────────────────────────────────────┐          │
│  │           AI Responder (Ollama)      │          │
│  └──────────────────────────────────────┘          │
└─────────────────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────┐
│              Connectors Layer                        │
│         WhatsApp (API)  /  Email (IMAP)              │
└─────────────────────────────────────────────────────┘
```

API Endpoints

Method Endpoint Description
GET /api/config Get current configuration
POST /api/config Update configuration
GET /api/messages Get message history
POST /api/test Test bot with custom message

Support

Built by Bakome, a self-taught developer.
If this project helps you, a small donation is appreciated.

Crypto addresses:

· BTC: bc1qhtjp3qpqru4vuqd355dfcn46mqjrlpdfmngk6u0
· ETH: 0x2fD73626714d9e37EA464109F8eCeA2CA5401062
· SOL: 3CfhghA7hSNPBbd1RME5rRDm5UUeesTq9NKTcyzZdkz4
· USDT (TRC20): THkLdiKsmscJFwBPA4tpWeAn1xVw7DTKxq

Thank you. 🙏

License

MIT – see LICENSE file.

---

Built on a phone. Powered by passion. 🚀

```
