# 📘 Documentation du Projet LLM Chat App

## 🌟 Introduction

Ce projet est une application de chat moderne et performante permettant d'interagir avec des modèles d'intelligence artificielle (LLM). Elle combine une interface utilisateur réactive construite avec **Next.js** et un backend robuste en **Rust**.

L'application supporte :

- **Multi-modèles** : Llama 3.1 8B (via Groq) et GPT-5 Mini (via OpenAI).
- **Streaming** : Réponses en temps réel via Server-Sent Events (SSE).
- **Fichiers** : Upload et analyse de fichiers (PDF, Images) pour le contexte.
- **Rendu Riche** : Support du Markdown, de la coloration syntaxique et des mathématiques (LaTeX/KaTeX).
- **Historique** : Gestion complète des sessions de chat et archivage.

---

## 🏗 Architecture Technique

Le projet est divisé en deux parties principales : le Frontend et le Backend.

### 🎨 Frontend (Dossier `app/`)

- **Framework** : [Next.js 16](https://nextjs.org/) (App Router)
- **Langage** : TypeScript / React 19
- **Styles** : [Tailwind CSS v4](https://tailwindcss.com/)
- **Composants Clés** :
  - `app/page.tsx` : Interface principale du chat (gestion de l'état, envoi des messages, affichage).
  - `components/MarkdownRenderer.tsx` : Rendu des réponses IA avec support MathJax/KaTeX et coloration syntaxique.

### ⚙️ Backend (Dossier `backend/`)

- **Langage** : [Rust](https://www.rust-lang.org/)
- **Framework Web** : [Axum](https://github.com/tokio-rs/axum)
- **Base de Données** : PostgreSQL (via [SQLx](https://github.com/launchbadge/sqlx))
- **Fonctionnalités** :
  - API REST pour la gestion des chats.
  - Streaming SSE pour les réponses IA.
  - Extraction de texte depuis les PDF (`pdf-extract`).
  - Gestion des uploads de fichiers.

---

## 🚀 Installation et Démarrage

### Prérequis

- **Node.js** (v18+ recommandé)
- **Rust** (Cargo)
- **PostgreSQL** (Serveur de base de données)

### 1. Configuration de la Base de Données

Assurez-vous d'avoir une base de données PostgreSQL active.
Créez un fichier `.env` dans le dossier `backend/` avec les variables suivantes :

```env
DATABASE_URL=postgres://user:password@localhost:5432/nom_de_la_db
UPLOAD_DIR=uploads
UPLOAD_BASE_URL=http://127.0.0.1:4000/uploads
# Clés API pour les modèles
GROQ_API_KEY=votre_cle_groq
OPENAI_API_KEY=votre_cle_openai
```

### 2. Installation des Dépendances

À la racine du projet :

```bash
npm install
```

### 3. Lancement du Projet

Le projet utilise `concurrently` pour lancer le frontend et le backend en même temps avec une seule commande :

```bash
npm run dev
```

- **Frontend** : Accessible sur [http://localhost:3000](http://localhost:3000)
- **Backend** : Accessible sur [http://127.0.0.1:4000](http://127.0.0.1:4000)

---

## 📂 Structure du Projet

```
.
├── app/                 # Code source du Frontend (Next.js App Router)
│   ├── page.tsx         # Page principale (Chat UI)
│   ├── layout.tsx       # Layout global
│   └── globals.css      # Styles globaux
├── backend/             # Code source du Backend (Rust)
│   ├── src/
│   │   └── main.rs      # Point d'entrée et logique API
│   ├── Cargo.toml       # Dépendances Rust
│   └── uploads/         # Dossier de stockage des fichiers uploadés
├── components/          # Composants React réutilisables
├── public/              # Fichiers statiques
└── package.json         # Scripts et dépendances Node
```

---

## 🔌 API Backend

Le backend expose une API RESTful sur le port 4000.

### Santé du service

- `GET /health` : Vérifie si le backend et la base de données sont opérationnels.

### Sessions de Chat

- `GET /api/chat/sessions` : Liste toutes les sessions actives.
- `POST /api/chat/sessions` : Crée une nouvelle session.
- `DELETE /api/chat/sessions/:id` : Supprime une session.
- `POST /api/chat/sessions/:id/archive` : Archive une session.

### Messages

- `POST /api/chat/sessions/:id/messages` : Ajoute un message utilisateur (réponse synchrone).
- `POST /api/chat/sessions/:id/messages/stream` : Ajoute un message et reçoit la réponse de l'IA en **streaming (SSE)**.
- `POST /api/chat/sessions/:id/regenerate` : Régénère le dernier message de l'IA.
- `POST /api/chat/sessions/:id/regenerate/stream` : Régénère en streaming.

### Uploads

- `POST /api/uploads` : Upload de fichiers (Multipart). Retourne l'URL et les métadonnées du fichier.

---

## 🛠 Détails Techniques

### Gestion des Modèles IA

Le backend choisit le modèle en fonction de la requête :

- **Llama 3.1 8B (Groq)** : Modèle par défaut pour le texte rapide.
- **GPT-5 Mini (OpenAI)** : Utilisé automatiquement si des fichiers/images sont attachés au message (multimodal).

### Base de Données (Schéma Simplifié)

- **chat_sessions** : `id`, `title`, `created_at`, `archived`...
- **chat_messages** : `id`, `session_id`, `role` (user/assistant), `content`, `position`...
- **chat_attachments** : `id`, `message_id`, `file_name`, `url`, `storage_key`...

### Système de Prompt

Un `SYSTEM_PROMPT` strict est injecté pour forcer l'IA à répondre en Markdown compatible, avec des règles spécifiques pour les mathématiques (LaTeX) et le code.
