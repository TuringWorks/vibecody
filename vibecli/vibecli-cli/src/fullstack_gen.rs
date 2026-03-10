#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FrontendFramework {
    React,
    Vue,
    Angular,
    Svelte,
    NextJs,
    Nuxt,
    SvelteKit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BackendFramework {
    Express,
    FastAPI,
    Django,
    SpringBoot,
    Actix,
    Gin,
    Rails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    DynamoDB,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuthStrategy {
    JWT,
    OAuth2,
    SessionBased,
    ApiKey,
    SAML,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProjectLayer {
    Frontend,
    Backend,
    Database,
    Infrastructure,
    Testing,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileType {
    Component,
    Route,
    Model,
    Controller,
    Migration,
    Config,
    Test,
    Dockerfile,
    Readme,
}

// ─── Structs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSpec {
    pub name: String,
    pub description: String,
    pub frontend: FrontendFramework,
    pub backend: BackendFramework,
    pub database: DatabaseType,
    pub auth: AuthStrategy,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub file_type: FileType,
    pub layer: ProjectLayer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedProject {
    pub name: String,
    pub files: Vec<GeneratedFile>,
    pub total_lines: usize,
    pub layers_generated: Vec<ProjectLayer>,
}

// ─── Templates ───────────────────────────────────────────────────────────────

fn frontend_templates() -> HashMap<FrontendFramework, Vec<(&'static str, &'static str, FileType)>> {
    let mut m = HashMap::new();

    m.insert(FrontendFramework::React, vec![
        ("frontend/src/App.tsx", "import React from 'react';\nimport { BrowserRouter, Routes, Route } from 'react-router-dom';\nimport Home from './pages/Home';\nimport Layout from './components/Layout';\n\nexport default function App() {\n  return (\n    <BrowserRouter>\n      <Layout>\n        <Routes>\n          <Route path=\"/\" element={<Home />} />\n        </Routes>\n      </Layout>\n    </BrowserRouter>\n  );\n}\n", FileType::Component),
        ("frontend/src/pages/Home.tsx", "import React from 'react';\n\nexport default function Home() {\n  return (\n    <div className=\"container\">\n      <h1>Welcome</h1>\n    </div>\n  );\n}\n", FileType::Component),
        ("frontend/src/components/Layout.tsx", "import React from 'react';\n\ninterface LayoutProps {\n  children: React.ReactNode;\n}\n\nexport default function Layout({ children }: LayoutProps) {\n  return (\n    <div className=\"layout\">\n      <nav className=\"navbar\">Navigation</nav>\n      <main>{children}</main>\n      <footer>Footer</footer>\n    </div>\n  );\n}\n", FileType::Component),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"dependencies\": {\n    \"react\": \"^18.2.0\",\n    \"react-dom\": \"^18.2.0\",\n    \"react-router-dom\": \"^6.0.0\"\n  }\n}\n", FileType::Config),
        ("frontend/tsconfig.json", "{\n  \"compilerOptions\": {\n    \"target\": \"ES2020\",\n    \"module\": \"ESNext\",\n    \"jsx\": \"react-jsx\",\n    \"strict\": true\n  }\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::Vue, vec![
        ("frontend/src/App.vue", "<template>\n  <div id=\"app\">\n    <router-view />\n  </div>\n</template>\n\n<script setup lang=\"ts\">\n</script>\n", FileType::Component),
        ("frontend/src/views/Home.vue", "<template>\n  <div class=\"container\">\n    <h1>Welcome</h1>\n  </div>\n</template>\n\n<script setup lang=\"ts\">\n</script>\n", FileType::Component),
        ("frontend/src/router/index.ts", "import { createRouter, createWebHistory } from 'vue-router';\nimport Home from '../views/Home.vue';\n\nconst routes = [\n  { path: '/', component: Home },\n];\n\nexport default createRouter({\n  history: createWebHistory(),\n  routes,\n});\n", FileType::Route),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": {\n    \"vue\": \"^3.3.0\",\n    \"vue-router\": \"^4.2.0\"\n  }\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::Angular, vec![
        ("frontend/src/app/app.component.ts", "import { Component } from '@angular/core';\n\n@Component({\n  selector: 'app-root',\n  template: '<router-outlet></router-outlet>',\n})\nexport class AppComponent {\n  title = 'app';\n}\n", FileType::Component),
        ("frontend/src/app/app-routing.module.ts", "import { NgModule } from '@angular/core';\nimport { RouterModule, Routes } from '@angular/router';\nimport { HomeComponent } from './home/home.component';\n\nconst routes: Routes = [\n  { path: '', component: HomeComponent },\n];\n\n@NgModule({\n  imports: [RouterModule.forRoot(routes)],\n  exports: [RouterModule],\n})\nexport class AppRoutingModule {}\n", FileType::Route),
        ("frontend/src/app/home/home.component.ts", "import { Component } from '@angular/core';\n\n@Component({\n  selector: 'app-home',\n  template: '<div class=\"container\"><h1>Welcome</h1></div>',\n})\nexport class HomeComponent {}\n", FileType::Component),
        ("frontend/angular.json", "{\n  \"$schema\": \"./node_modules/@angular/cli/lib/config/schema.json\",\n  \"version\": 1,\n  \"projects\": {}\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::Svelte, vec![
        ("frontend/src/App.svelte", "<script>\n  import Router from './Router.svelte';\n</script>\n\n<main>\n  <Router />\n</main>\n\n<style>\n  main { max-width: 1200px; margin: 0 auto; }\n</style>\n", FileType::Component),
        ("frontend/src/routes/Home.svelte", "<script>\n  let greeting = 'Welcome';\n</script>\n\n<div class=\"container\">\n  <h1>{greeting}</h1>\n</div>\n", FileType::Component),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"devDependencies\": {\n    \"svelte\": \"^4.0.0\"\n  }\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::NextJs, vec![
        ("frontend/src/app/page.tsx", "export default function Home() {\n  return (\n    <main>\n      <h1>Welcome</h1>\n    </main>\n  );\n}\n", FileType::Component),
        ("frontend/src/app/layout.tsx", "export default function RootLayout({ children }: { children: React.ReactNode }) {\n  return (\n    <html lang=\"en\">\n      <body>{children}</body>\n    </html>\n  );\n}\n", FileType::Component),
        ("frontend/next.config.js", "/** @type {import('next').NextConfig} */\nconst nextConfig = {\n  reactStrictMode: true,\n};\n\nmodule.exports = nextConfig;\n", FileType::Config),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": {\n    \"next\": \"^14.0.0\",\n    \"react\": \"^18.2.0\",\n    \"react-dom\": \"^18.2.0\"\n  }\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::Nuxt, vec![
        ("frontend/app.vue", "<template>\n  <div>\n    <NuxtPage />\n  </div>\n</template>\n", FileType::Component),
        ("frontend/pages/index.vue", "<template>\n  <div class=\"container\">\n    <h1>Welcome</h1>\n  </div>\n</template>\n\n<script setup lang=\"ts\">\n</script>\n", FileType::Component),
        ("frontend/nuxt.config.ts", "export default defineNuxtConfig({\n  devtools: { enabled: true },\n});\n", FileType::Config),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": {\n    \"nuxt\": \"^3.8.0\"\n  }\n}\n", FileType::Config),
    ]);

    m.insert(FrontendFramework::SvelteKit, vec![
        ("frontend/src/routes/+page.svelte", "<script>\n  let title = 'Welcome';\n</script>\n\n<h1>{title}</h1>\n", FileType::Component),
        ("frontend/src/routes/+layout.svelte", "<script>\n  import '../app.css';\n</script>\n\n<slot />\n", FileType::Component),
        ("frontend/svelte.config.js", "import adapter from '@sveltejs/adapter-auto';\n\n/** @type {import('@sveltejs/kit').Config} */\nconst config = {\n  kit: {\n    adapter: adapter(),\n  },\n};\n\nexport default config;\n", FileType::Config),
        ("frontend/package.json", "{\n  \"name\": \"frontend\",\n  \"version\": \"0.1.0\",\n  \"devDependencies\": {\n    \"@sveltejs/kit\": \"^1.27.0\",\n    \"svelte\": \"^4.0.0\"\n  }\n}\n", FileType::Config),
    ]);

    m
}

fn backend_templates() -> HashMap<BackendFramework, Vec<(&'static str, &'static str, FileType)>> {
    let mut m = HashMap::new();

    m.insert(BackendFramework::Express, vec![
        ("backend/src/index.ts", "import express from 'express';\nimport cors from 'cors';\nimport { router } from './routes';\n\nconst app = express();\nconst PORT = process.env.PORT || 3000;\n\napp.use(cors());\napp.use(express.json());\napp.use('/api', router);\n\napp.listen(PORT, () => {\n  console.log(`Server running on port ${PORT}`);\n});\n", FileType::Controller),
        ("backend/src/routes/index.ts", "import { Router } from 'express';\n\nexport const router = Router();\n\nrouter.get('/health', (_req, res) => {\n  res.json({ status: 'ok' });\n});\n", FileType::Route),
        ("backend/src/models/user.ts", "export interface User {\n  id: string;\n  email: string;\n  name: string;\n  createdAt: Date;\n}\n", FileType::Model),
        ("backend/package.json", "{\n  \"name\": \"backend\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": {\n    \"express\": \"^4.18.0\",\n    \"cors\": \"^2.8.5\"\n  }\n}\n", FileType::Config),
    ]);

    m.insert(BackendFramework::FastAPI, vec![
        ("backend/app/main.py", "from fastapi import FastAPI\nfrom fastapi.middleware.cors import CORSMiddleware\nfrom app.routes import router\n\napp = FastAPI(title=\"API\")\n\napp.add_middleware(\n    CORSMiddleware,\n    allow_origins=[\"*\"],\n    allow_methods=[\"*\"],\n    allow_headers=[\"*\"],\n)\n\napp.include_router(router, prefix=\"/api\")\n\n\n@app.get(\"/health\")\ndef health():\n    return {\"status\": \"ok\"}\n", FileType::Controller),
        ("backend/app/routes.py", "from fastapi import APIRouter\n\nrouter = APIRouter()\n\n\n@router.get(\"/users\")\ndef list_users():\n    return []\n", FileType::Route),
        ("backend/app/models.py", "from pydantic import BaseModel\nfrom datetime import datetime\n\n\nclass User(BaseModel):\n    id: str\n    email: str\n    name: str\n    created_at: datetime\n", FileType::Model),
        ("backend/requirements.txt", "fastapi>=0.104.0\nuvicorn>=0.24.0\npydantic>=2.5.0\n", FileType::Config),
    ]);

    m.insert(BackendFramework::Django, vec![
        ("backend/project/settings.py", "import os\n\nBASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))\nSECRET_KEY = os.environ.get('SECRET_KEY', 'change-me')\nDEBUG = True\nALLOWED_HOSTS = ['*']\n\nINSTALLED_APPS = [\n    'django.contrib.admin',\n    'django.contrib.auth',\n    'django.contrib.contenttypes',\n    'rest_framework',\n    'api',\n]\n\nROOT_URLCONF = 'project.urls'\n", FileType::Config),
        ("backend/project/urls.py", "from django.contrib import admin\nfrom django.urls import path, include\n\nurlpatterns = [\n    path('admin/', admin.site.urls),\n    path('api/', include('api.urls')),\n]\n", FileType::Route),
        ("backend/api/models.py", "from django.db import models\n\n\nclass User(models.Model):\n    email = models.EmailField(unique=True)\n    name = models.CharField(max_length=255)\n    created_at = models.DateTimeField(auto_now_add=True)\n\n    def __str__(self):\n        return self.email\n", FileType::Model),
        ("backend/requirements.txt", "Django>=4.2\ndjangorestframework>=3.14\n", FileType::Config),
    ]);

    m.insert(BackendFramework::SpringBoot, vec![
        ("backend/src/main/java/com/app/Application.java", "package com.app;\n\nimport org.springframework.boot.SpringApplication;\nimport org.springframework.boot.autoconfigure.SpringBootApplication;\n\n@SpringBootApplication\npublic class Application {\n    public static void main(String[] args) {\n        SpringApplication.run(Application.class, args);\n    }\n}\n", FileType::Controller),
        ("backend/src/main/java/com/app/controller/HealthController.java", "package com.app.controller;\n\nimport org.springframework.web.bind.annotation.GetMapping;\nimport org.springframework.web.bind.annotation.RestController;\nimport java.util.Map;\n\n@RestController\npublic class HealthController {\n    @GetMapping(\"/health\")\n    public Map<String, String> health() {\n        return Map.of(\"status\", \"ok\");\n    }\n}\n", FileType::Controller),
        ("backend/src/main/java/com/app/model/User.java", "package com.app.model;\n\nimport jakarta.persistence.*;\nimport java.time.Instant;\n\n@Entity\n@Table(name = \"users\")\npublic class User {\n    @Id\n    @GeneratedValue(strategy = GenerationType.UUID)\n    private String id;\n    private String email;\n    private String name;\n    private Instant createdAt;\n}\n", FileType::Model),
        ("backend/pom.xml", "<project>\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>com.app</groupId>\n  <artifactId>backend</artifactId>\n  <version>0.1.0</version>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.2.0</version>\n  </parent>\n</project>\n", FileType::Config),
    ]);

    m.insert(BackendFramework::Actix, vec![
        ("backend/src/main.rs", "use actix_web::{web, App, HttpServer, HttpResponse};\nuse serde::Serialize;\n\n#[derive(Serialize)]\nstruct Health {\n    status: String,\n}\n\nasync fn health() -> HttpResponse {\n    HttpResponse::Ok().json(Health { status: \"ok\".into() })\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    HttpServer::new(|| {\n        App::new()\n            .route(\"/health\", web::get().to(health))\n    })\n    .bind(\"0.0.0.0:8080\")?\n    .run()\n    .await\n}\n", FileType::Controller),
        ("backend/src/models.rs", "use serde::{Deserialize, Serialize};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct User {\n    pub id: String,\n    pub email: String,\n    pub name: String,\n}\n", FileType::Model),
        ("backend/Cargo.toml", "[package]\nname = \"backend\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nactix-web = \"4\"\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\ntokio = { version = \"1\", features = [\"full\"] }\n", FileType::Config),
    ]);

    m.insert(BackendFramework::Gin, vec![
        ("backend/main.go", "package main\n\nimport (\n\t\"net/http\"\n\t\"github.com/gin-gonic/gin\"\n)\n\nfunc main() {\n\tr := gin.Default()\n\tr.GET(\"/health\", func(c *gin.Context) {\n\t\tc.JSON(http.StatusOK, gin.H{\"status\": \"ok\"})\n\t})\n\tr.Run(\":8080\")\n}\n", FileType::Controller),
        ("backend/models/user.go", "package models\n\nimport \"time\"\n\ntype User struct {\n\tID        string    `json:\"id\"`\n\tEmail     string    `json:\"email\"`\n\tName      string    `json:\"name\"`\n\tCreatedAt time.Time `json:\"created_at\"`\n}\n", FileType::Model),
        ("backend/go.mod", "module backend\n\ngo 1.21\n\nrequire github.com/gin-gonic/gin v1.9.1\n", FileType::Config),
    ]);

    m.insert(BackendFramework::Rails, vec![
        ("backend/config/routes.rb", "Rails.application.routes.draw do\n  namespace :api do\n    resources :users, only: [:index, :show, :create]\n  end\n  get '/health', to: 'health#index'\nend\n", FileType::Route),
        ("backend/app/controllers/health_controller.rb", "class HealthController < ApplicationController\n  def index\n    render json: { status: 'ok' }\n  end\nend\n", FileType::Controller),
        ("backend/app/models/user.rb", "class User < ApplicationRecord\n  validates :email, presence: true, uniqueness: true\n  validates :name, presence: true\nend\n", FileType::Model),
        ("backend/Gemfile", "source 'https://rubygems.org'\n\ngem 'rails', '~> 7.1'\ngem 'pg'\ngem 'puma', '~> 6.0'\n", FileType::Config),
    ]);

    m
}

fn database_schema_template(db: &DatabaseType) -> Vec<(&'static str, &'static str, FileType)> {
    match db {
        DatabaseType::PostgreSQL => vec![
            ("database/migrations/001_create_users.sql", "CREATE TABLE users (\n  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n  email VARCHAR(255) NOT NULL UNIQUE,\n  name VARCHAR(255) NOT NULL,\n  password_hash VARCHAR(255) NOT NULL,\n  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\n);\n\nCREATE INDEX idx_users_email ON users(email);\n", FileType::Migration),
            ("database/migrations/002_create_sessions.sql", "CREATE TABLE sessions (\n  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\n  token VARCHAR(512) NOT NULL UNIQUE,\n  expires_at TIMESTAMPTZ NOT NULL,\n  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\n);\n\nCREATE INDEX idx_sessions_token ON sessions(token);\nCREATE INDEX idx_sessions_user_id ON sessions(user_id);\n", FileType::Migration),
            ("database/seed.sql", "INSERT INTO users (email, name, password_hash)\nVALUES ('admin@example.com', 'Admin', '$2b$12$placeholder');\n", FileType::Migration),
        ],
        DatabaseType::MySQL => vec![
            ("database/migrations/001_create_users.sql", "CREATE TABLE users (\n  id CHAR(36) PRIMARY KEY,\n  email VARCHAR(255) NOT NULL UNIQUE,\n  name VARCHAR(255) NOT NULL,\n  password_hash VARCHAR(255) NOT NULL,\n  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;\n\nCREATE INDEX idx_users_email ON users(email);\n", FileType::Migration),
            ("database/seed.sql", "INSERT INTO users (id, email, name, password_hash)\nVALUES (UUID(), 'admin@example.com', 'Admin', '$2b$12$placeholder');\n", FileType::Migration),
        ],
        DatabaseType::SQLite => vec![
            ("database/migrations/001_create_users.sql", "CREATE TABLE IF NOT EXISTS users (\n  id TEXT PRIMARY KEY,\n  email TEXT NOT NULL UNIQUE,\n  name TEXT NOT NULL,\n  password_hash TEXT NOT NULL,\n  created_at TEXT NOT NULL DEFAULT (datetime('now')),\n  updated_at TEXT NOT NULL DEFAULT (datetime('now'))\n);\n", FileType::Migration),
            ("database/seed.sql", "INSERT INTO users (id, email, name, password_hash)\nVALUES ('1', 'admin@example.com', 'Admin', '$2b$12$placeholder');\n", FileType::Migration),
        ],
        DatabaseType::MongoDB => vec![
            ("database/schemas/user.json", "{\n  \"$jsonSchema\": {\n    \"bsonType\": \"object\",\n    \"required\": [\"email\", \"name\"],\n    \"properties\": {\n      \"email\": { \"bsonType\": \"string\" },\n      \"name\": { \"bsonType\": \"string\" },\n      \"createdAt\": { \"bsonType\": \"date\" }\n    }\n  }\n}\n", FileType::Migration),
            ("database/init.js", "db = db.getSiblingDB('app');\ndb.createCollection('users', {\n  validator: JSON.parse(cat('/schemas/user.json'))\n});\ndb.users.createIndex({ email: 1 }, { unique: true });\n", FileType::Migration),
        ],
        DatabaseType::Redis => vec![
            ("database/schemas/redis.conf", "# Redis configuration\nmaxmemory 256mb\nmaxmemory-policy allkeys-lru\nappendonly yes\nappendfsync everysec\n", FileType::Config),
            ("database/init.sh", "#!/bin/bash\nredis-cli SET app:version \"0.1.0\"\nredis-cli HSET app:config max_users 1000\n", FileType::Migration),
        ],
        DatabaseType::DynamoDB => vec![
            ("database/tables/users.json", "{\n  \"TableName\": \"users\",\n  \"KeySchema\": [\n    { \"AttributeName\": \"id\", \"KeyType\": \"HASH\" }\n  ],\n  \"AttributeDefinitions\": [\n    { \"AttributeName\": \"id\", \"AttributeType\": \"S\" },\n    { \"AttributeName\": \"email\", \"AttributeType\": \"S\" }\n  ],\n  \"GlobalSecondaryIndexes\": [\n    {\n      \"IndexName\": \"email-index\",\n      \"KeySchema\": [{ \"AttributeName\": \"email\", \"KeyType\": \"HASH\" }],\n      \"Projection\": { \"ProjectionType\": \"ALL\" }\n    }\n  ],\n  \"BillingMode\": \"PAY_PER_REQUEST\"\n}\n", FileType::Migration),
        ],
    }
}

fn auth_middleware_template(auth: &AuthStrategy, backend: &BackendFramework) -> Option<(&'static str, &'static str, FileType)> {
    match (auth, backend) {
        (AuthStrategy::None, _) => None,
        (AuthStrategy::JWT, BackendFramework::Express) => Some((
            "backend/src/middleware/auth.ts",
            "import { Request, Response, NextFunction } from 'express';\nimport jwt from 'jsonwebtoken';\n\nconst SECRET = process.env.JWT_SECRET || 'change-me';\n\nexport function authenticate(req: Request, res: Response, next: NextFunction) {\n  const token = req.headers.authorization?.replace('Bearer ', '');\n  if (!token) return res.status(401).json({ error: 'Unauthorized' });\n  try {\n    const payload = jwt.verify(token, SECRET);\n    (req as any).user = payload;\n    next();\n  } catch {\n    res.status(401).json({ error: 'Invalid token' });\n  }\n}\n",
            FileType::Controller,
        )),
        (AuthStrategy::JWT, _) => Some((
            "backend/src/middleware/auth.rs",
            "// JWT authentication middleware placeholder\npub fn verify_jwt(token: &str) -> Result<Claims, AuthError> {\n    todo!(\"implement JWT verification\")\n}\n\npub struct Claims {\n    pub sub: String,\n    pub exp: u64,\n}\n\n#[derive(Debug)]\npub enum AuthError {\n    Expired,\n    Invalid,\n}\n",
            FileType::Controller,
        )),
        (AuthStrategy::OAuth2, _) => Some((
            "backend/src/middleware/oauth.ts",
            "// OAuth2 configuration placeholder\nexport const oauthConfig = {\n  clientId: process.env.OAUTH_CLIENT_ID || '',\n  clientSecret: process.env.OAUTH_CLIENT_SECRET || '',\n  redirectUri: '/auth/callback',\n  authorizationUrl: 'https://provider.example.com/authorize',\n  tokenUrl: 'https://provider.example.com/token',\n};\n",
            FileType::Config,
        )),
        (AuthStrategy::SessionBased, _) => Some((
            "backend/src/middleware/session.ts",
            "// Session-based auth placeholder\nexport const sessionConfig = {\n  secret: process.env.SESSION_SECRET || 'change-me',\n  resave: false,\n  saveUninitialized: false,\n  cookie: { secure: true, httpOnly: true, maxAge: 86400000 },\n};\n",
            FileType::Config,
        )),
        (AuthStrategy::ApiKey, _) => Some((
            "backend/src/middleware/apikey.ts",
            "import { Request, Response, NextFunction } from 'express';\n\nexport function validateApiKey(req: Request, res: Response, next: NextFunction) {\n  const key = req.headers['x-api-key'];\n  if (!key) return res.status(401).json({ error: 'API key required' });\n  // Validate against stored keys\n  next();\n}\n",
            FileType::Controller,
        )),
        (AuthStrategy::SAML, _) => Some((
            "backend/src/middleware/saml.ts",
            "// SAML authentication configuration placeholder\nexport const samlConfig = {\n  entryPoint: 'https://idp.example.com/sso',\n  issuer: 'app',\n  cert: process.env.SAML_CERT || '',\n  callbackUrl: '/auth/saml/callback',\n};\n",
            FileType::Config,
        )),
    }
}

// ─── FullStackGenerator ──────────────────────────────────────────────────────

pub struct FullStackGenerator {
    spec: ProjectSpec,
}

impl FullStackGenerator {
    pub fn new(spec: ProjectSpec) -> Self {
        Self { spec }
    }

    pub fn generate(&self) -> GeneratedProject {
        let mut files = Vec::new();
        let mut layers = Vec::new();

        let frontend = self.generate_frontend();
        if !frontend.is_empty() {
            layers.push(ProjectLayer::Frontend);
        }
        files.extend(frontend);

        let backend = self.generate_backend();
        if !backend.is_empty() {
            layers.push(ProjectLayer::Backend);
        }
        files.extend(backend);

        let database = self.generate_database();
        if !database.is_empty() {
            layers.push(ProjectLayer::Database);
        }
        files.extend(database);

        let infra = self.generate_infra();
        if !infra.is_empty() {
            layers.push(ProjectLayer::Infrastructure);
        }
        files.extend(infra);

        let tests = self.generate_tests();
        if !tests.is_empty() {
            layers.push(ProjectLayer::Testing);
        }
        files.extend(tests);

        // Always add a README
        let readme = GeneratedFile {
            path: "README.md".to_string(),
            content: format!(
                "# {}\n\n{}\n\n## Stack\n\n- Frontend: {:?}\n- Backend: {:?}\n- Database: {:?}\n- Auth: {:?}\n\n## Features\n\n{}\n\n## Getting Started\n\n```bash\ndocker-compose up\n```\n",
                self.spec.name,
                self.spec.description,
                self.spec.frontend,
                self.spec.backend,
                self.spec.database,
                self.spec.auth,
                self.spec.features.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n"),
            ),
            file_type: FileType::Readme,
            layer: ProjectLayer::Documentation,
        };
        files.push(readme);
        layers.push(ProjectLayer::Documentation);

        let total_lines = files.iter().map(|f| f.content.lines().count()).sum();

        GeneratedProject {
            name: self.spec.name.clone(),
            files,
            total_lines,
            layers_generated: layers,
        }
    }

    pub fn generate_frontend(&self) -> Vec<GeneratedFile> {
        let templates = frontend_templates();
        let entries = match templates.get(&self.spec.frontend) {
            Some(e) => e,
            None => return Vec::new(),
        };

        entries
            .iter()
            .map(|(path, content, file_type)| GeneratedFile {
                path: format!("{}/{}", self.spec.name, path),
                content: content.to_string(),
                file_type: file_type.clone(),
                layer: ProjectLayer::Frontend,
            })
            .collect()
    }

    pub fn generate_backend(&self) -> Vec<GeneratedFile> {
        let templates = backend_templates();
        let entries = match templates.get(&self.spec.backend) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut files: Vec<GeneratedFile> = entries
            .iter()
            .map(|(path, content, file_type)| GeneratedFile {
                path: format!("{}/{}", self.spec.name, path),
                content: content.to_string(),
                file_type: file_type.clone(),
                layer: ProjectLayer::Backend,
            })
            .collect();

        // Add auth middleware if applicable
        if let Some((path, content, file_type)) =
            auth_middleware_template(&self.spec.auth, &self.spec.backend)
        {
            files.push(GeneratedFile {
                path: format!("{}/{}", self.spec.name, path),
                content: content.to_string(),
                file_type: file_type.clone(),
                layer: ProjectLayer::Backend,
            });
        }

        files
    }

    pub fn generate_database(&self) -> Vec<GeneratedFile> {
        let entries = database_schema_template(&self.spec.database);
        entries
            .iter()
            .map(|(path, content, file_type)| GeneratedFile {
                path: format!("{}/{}", self.spec.name, path),
                content: content.to_string(),
                file_type: file_type.clone(),
                layer: ProjectLayer::Database,
            })
            .collect()
    }

    pub fn generate_infra(&self) -> Vec<GeneratedFile> {
        let dockerfile = GeneratedFile {
            path: format!("{}/Dockerfile", self.spec.name),
            content: self.dockerfile_content(),
            file_type: FileType::Dockerfile,
            layer: ProjectLayer::Infrastructure,
        };

        let compose = GeneratedFile {
            path: format!("{}/docker-compose.yml", self.spec.name),
            content: self.docker_compose_content(),
            file_type: FileType::Config,
            layer: ProjectLayer::Infrastructure,
        };

        let ci = GeneratedFile {
            path: format!("{}/.github/workflows/ci.yml", self.spec.name),
            content: self.ci_workflow_content(),
            file_type: FileType::Config,
            layer: ProjectLayer::Infrastructure,
        };

        let env_example = GeneratedFile {
            path: format!("{}/.env.example", self.spec.name),
            content: format!(
                "# {} Environment Variables\nDATABASE_URL=\nJWT_SECRET=change-me\nPORT=3000\nNODE_ENV=development\n",
                self.spec.name
            ),
            file_type: FileType::Config,
            layer: ProjectLayer::Infrastructure,
        };

        vec![dockerfile, compose, ci, env_example]
    }

    pub fn generate_tests(&self) -> Vec<GeneratedFile> {
        let mut files = Vec::new();

        // Frontend test
        let frontend_test_content = match self.spec.frontend {
            FrontendFramework::React | FrontendFramework::NextJs => {
                "import { render, screen } from '@testing-library/react';\nimport App from '../src/App';\n\ndescribe('App', () => {\n  it('renders without crashing', () => {\n    render(<App />);\n    expect(screen.getByText('Welcome')).toBeInTheDocument();\n  });\n});\n"
            }
            FrontendFramework::Vue | FrontendFramework::Nuxt => {
                "import { mount } from '@vue/test-utils';\nimport App from '../src/App.vue';\n\ndescribe('App', () => {\n  it('renders without crashing', () => {\n    const wrapper = mount(App);\n    expect(wrapper.exists()).toBe(true);\n  });\n});\n"
            }
            _ => {
                "// Frontend test placeholder\ndescribe('App', () => {\n  it('renders without crashing', () => {\n    expect(true).toBe(true);\n  });\n});\n"
            }
        };

        files.push(GeneratedFile {
            path: format!("{}/frontend/tests/app.test.ts", self.spec.name),
            content: frontend_test_content.to_string(),
            file_type: FileType::Test,
            layer: ProjectLayer::Testing,
        });

        // Backend test
        let backend_test_content = match self.spec.backend {
            BackendFramework::Express => {
                "import request from 'supertest';\nimport app from '../src/index';\n\ndescribe('Health endpoint', () => {\n  it('returns 200', async () => {\n    const res = await request(app).get('/health');\n    expect(res.status).toBe(200);\n    expect(res.body.status).toBe('ok');\n  });\n});\n"
            }
            BackendFramework::FastAPI => {
                "from fastapi.testclient import TestClient\nfrom app.main import app\n\nclient = TestClient(app)\n\n\ndef test_health():\n    response = client.get(\"/health\")\n    assert response.status_code == 200\n    assert response.json() == {\"status\": \"ok\"}\n"
            }
            BackendFramework::Actix => {
                "#[cfg(test)]\nmod tests {\n    use actix_web::test;\n\n    #[actix_web::test]\n    async fn test_health() {\n        // Health endpoint test\n        assert!(true);\n    }\n}\n"
            }
            _ => {
                "// Backend test placeholder\ndescribe('API', () => {\n  it('health check returns ok', () => {\n    expect(true).toBe(true);\n  });\n});\n"
            }
        };

        files.push(GeneratedFile {
            path: format!("{}/backend/tests/api.test.ts", self.spec.name),
            content: backend_test_content.to_string(),
            file_type: FileType::Test,
            layer: ProjectLayer::Testing,
        });

        files
    }

    pub fn estimate_files(&self) -> usize {
        let fe = frontend_templates()
            .get(&self.spec.frontend)
            .map_or(0, |t| t.len());
        let be = backend_templates()
            .get(&self.spec.backend)
            .map_or(0, |t| t.len());
        let db = database_schema_template(&self.spec.database).len();
        let auth_file: usize = if self.spec.auth != AuthStrategy::None { 1 } else { 0 };
        let infra = 4; // Dockerfile, compose, CI, .env.example
        let tests = 2; // frontend + backend test
        let docs = 1; // README

        fe + be + db + auth_file + infra + tests + docs
    }

    pub fn estimate_lines(&self) -> usize {
        let project = self.generate();
        project.total_lines
    }

    // ── Private helpers ──────────────────────────────────────────────────

    fn dockerfile_content(&self) -> String {
        match self.spec.backend {
            BackendFramework::Express => {
                "FROM node:20-alpine AS builder\nWORKDIR /app\nCOPY backend/package*.json ./\nRUN npm ci\nCOPY backend/ .\nRUN npm run build\n\nFROM node:20-alpine\nWORKDIR /app\nCOPY --from=builder /app/dist ./dist\nCOPY --from=builder /app/node_modules ./node_modules\nEXPOSE 3000\nCMD [\"node\", \"dist/index.js\"]\n".to_string()
            }
            BackendFramework::FastAPI => {
                "FROM python:3.12-slim\nWORKDIR /app\nCOPY backend/requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY backend/ .\nEXPOSE 8000\nCMD [\"uvicorn\", \"app.main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8000\"]\n".to_string()
            }
            BackendFramework::Actix => {
                "FROM rust:1.75 AS builder\nWORKDIR /app\nCOPY backend/ .\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/backend /usr/local/bin/\nEXPOSE 8080\nCMD [\"backend\"]\n".to_string()
            }
            BackendFramework::Gin => {
                "FROM golang:1.21-alpine AS builder\nWORKDIR /app\nCOPY backend/ .\nRUN go build -o server .\n\nFROM alpine:3.19\nCOPY --from=builder /app/server /usr/local/bin/\nEXPOSE 8080\nCMD [\"server\"]\n".to_string()
            }
            BackendFramework::SpringBoot => {
                "FROM eclipse-temurin:21-jdk AS builder\nWORKDIR /app\nCOPY backend/ .\nRUN ./mvnw package -DskipTests\n\nFROM eclipse-temurin:21-jre\nCOPY --from=builder /app/target/*.jar /app/app.jar\nEXPOSE 8080\nCMD [\"java\", \"-jar\", \"/app/app.jar\"]\n".to_string()
            }
            BackendFramework::Rails => {
                "FROM ruby:3.3-slim\nWORKDIR /app\nCOPY backend/Gemfile* ./\nRUN bundle install\nCOPY backend/ .\nEXPOSE 3000\nCMD [\"rails\", \"server\", \"-b\", \"0.0.0.0\"]\n".to_string()
            }
            BackendFramework::Django => {
                "FROM python:3.12-slim\nWORKDIR /app\nCOPY backend/requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY backend/ .\nEXPOSE 8000\nCMD [\"python\", \"manage.py\", \"runserver\", \"0.0.0.0:8000\"]\n".to_string()
            }
        }
    }

    fn docker_compose_content(&self) -> String {
        let db_service = match self.spec.database {
            DatabaseType::PostgreSQL => "  db:\n    image: postgres:16-alpine\n    environment:\n      POSTGRES_DB: app\n      POSTGRES_USER: app\n      POSTGRES_PASSWORD: secret\n    ports:\n      - \"5432:5432\"\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n\nvolumes:\n  pgdata:\n",
            DatabaseType::MySQL => "  db:\n    image: mysql:8.0\n    environment:\n      MYSQL_DATABASE: app\n      MYSQL_ROOT_PASSWORD: secret\n    ports:\n      - \"3306:3306\"\n    volumes:\n      - mysqldata:/var/lib/mysql\n\nvolumes:\n  mysqldata:\n",
            DatabaseType::MongoDB => "  db:\n    image: mongo:7\n    ports:\n      - \"27017:27017\"\n    volumes:\n      - mongodata:/data/db\n\nvolumes:\n  mongodata:\n",
            DatabaseType::Redis => "  redis:\n    image: redis:7-alpine\n    ports:\n      - \"6379:6379\"\n",
            DatabaseType::SQLite => "  # SQLite uses a local file, no service needed\n",
            DatabaseType::DynamoDB => "  dynamodb:\n    image: amazon/dynamodb-local:latest\n    ports:\n      - \"8000:8000\"\n",
        };

        format!(
            "version: '3.8'\n\nservices:\n  backend:\n    build: .\n    ports:\n      - \"8080:8080\"\n    env_file:\n      - .env\n    depends_on:\n      - db\n\n{}\n",
            db_service
        )
    }

    fn ci_workflow_content(&self) -> String {
        "name: CI\n\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Run tests\n        run: echo \"Add test commands here\"\n\n  lint:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Lint\n        run: echo \"Add lint commands here\"\n".to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> ProjectSpec {
        ProjectSpec {
            name: "my-app".to_string(),
            description: "A sample full-stack application".to_string(),
            frontend: FrontendFramework::React,
            backend: BackendFramework::Express,
            database: DatabaseType::PostgreSQL,
            auth: AuthStrategy::JWT,
            features: vec!["auth".to_string(), "crud".to_string()],
        }
    }

    #[test]
    fn test_project_spec_creation() {
        let spec = sample_spec();
        assert_eq!(spec.name, "my-app");
        assert_eq!(spec.features.len(), 2);
        assert_eq!(spec.frontend, FrontendFramework::React);
        assert_eq!(spec.backend, BackendFramework::Express);
    }

    #[test]
    fn test_spec_serialization() {
        let spec = sample_spec();
        let json = serde_json::to_string(&spec).expect("should serialize");
        let deserialized: ProjectSpec =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.name, "my-app");
        assert_eq!(deserialized.frontend, FrontendFramework::React);
    }

    #[test]
    fn test_generate_full_project() {
        let gen = FullStackGenerator::new(sample_spec());
        let project = gen.generate();
        assert_eq!(project.name, "my-app");
        assert!(!project.files.is_empty());
        assert!(project.total_lines > 0);
        assert!(project.layers_generated.contains(&ProjectLayer::Frontend));
        assert!(project.layers_generated.contains(&ProjectLayer::Backend));
        assert!(project.layers_generated.contains(&ProjectLayer::Database));
        assert!(project
            .layers_generated
            .contains(&ProjectLayer::Infrastructure));
        assert!(project.layers_generated.contains(&ProjectLayer::Testing));
        assert!(project
            .layers_generated
            .contains(&ProjectLayer::Documentation));
    }

    #[test]
    fn test_generate_frontend_react() {
        let gen = FullStackGenerator::new(sample_spec());
        let files = gen.generate_frontend();
        assert!(files.len() >= 3);
        assert!(files.iter().any(|f| f.path.contains("App.tsx")));
        assert!(files.iter().all(|f| f.layer == ProjectLayer::Frontend));
    }

    #[test]
    fn test_generate_frontend_vue() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::Vue;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("App.vue")));
    }

    #[test]
    fn test_generate_frontend_angular() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::Angular;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("app.component.ts")));
    }

    #[test]
    fn test_generate_frontend_svelte() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::Svelte;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("App.svelte")));
    }

    #[test]
    fn test_generate_frontend_nextjs() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::NextJs;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("page.tsx")));
    }

    #[test]
    fn test_generate_frontend_nuxt() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::Nuxt;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("app.vue")));
    }

    #[test]
    fn test_generate_frontend_sveltekit() {
        let mut spec = sample_spec();
        spec.frontend = FrontendFramework::SvelteKit;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_frontend();
        assert!(files.iter().any(|f| f.path.contains("+page.svelte")));
    }

    #[test]
    fn test_generate_backend_express() {
        let gen = FullStackGenerator::new(sample_spec());
        let files = gen.generate_backend();
        assert!(files.iter().any(|f| f.path.contains("index.ts")));
        assert!(files.iter().any(|f| f.path.contains("auth.ts")));
    }

    #[test]
    fn test_generate_backend_fastapi() {
        let mut spec = sample_spec();
        spec.backend = BackendFramework::FastAPI;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_backend();
        assert!(files.iter().any(|f| f.path.contains("main.py")));
    }

    #[test]
    fn test_generate_backend_actix() {
        let mut spec = sample_spec();
        spec.backend = BackendFramework::Actix;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_backend();
        assert!(files.iter().any(|f| f.path.contains("main.rs")));
    }

    #[test]
    fn test_generate_backend_no_auth() {
        let mut spec = sample_spec();
        spec.auth = AuthStrategy::None;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_backend();
        assert!(!files.iter().any(|f| f.path.contains("middleware")));
    }

    #[test]
    fn test_generate_database_postgresql() {
        let gen = FullStackGenerator::new(sample_spec());
        let files = gen.generate_database();
        assert!(files.len() >= 2);
        assert!(files
            .iter()
            .any(|f| f.path.contains("001_create_users.sql")));
        assert!(files.iter().all(|f| f.layer == ProjectLayer::Database));
    }

    #[test]
    fn test_generate_database_mongodb() {
        let mut spec = sample_spec();
        spec.database = DatabaseType::MongoDB;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_database();
        assert!(files.iter().any(|f| f.path.contains("user.json")));
    }

    #[test]
    fn test_generate_database_dynamodb() {
        let mut spec = sample_spec();
        spec.database = DatabaseType::DynamoDB;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_database();
        assert!(files.iter().any(|f| f.path.contains("users.json")));
    }

    #[test]
    fn test_generate_infra() {
        let gen = FullStackGenerator::new(sample_spec());
        let files = gen.generate_infra();
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|f| f.path.contains("Dockerfile")));
        assert!(files.iter().any(|f| f.path.contains("docker-compose")));
        assert!(files.iter().any(|f| f.path.contains("ci.yml")));
        assert!(files.iter().any(|f| f.path.contains(".env.example")));
        assert!(files
            .iter()
            .all(|f| f.layer == ProjectLayer::Infrastructure));
    }

    #[test]
    fn test_generate_tests() {
        let gen = FullStackGenerator::new(sample_spec());
        let files = gen.generate_tests();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.contains("frontend/tests")));
        assert!(files.iter().any(|f| f.path.contains("backend/tests")));
        assert!(files.iter().all(|f| f.layer == ProjectLayer::Testing));
    }

    #[test]
    fn test_estimate_files() {
        let gen = FullStackGenerator::new(sample_spec());
        let count = gen.estimate_files();
        // React(5) + Express(4) + auth(1) + PostgreSQL(3) + infra(4) + tests(2) + docs(1) = 20
        assert!(count >= 15, "Expected at least 15 files, got {}", count);
    }

    #[test]
    fn test_estimate_lines() {
        let gen = FullStackGenerator::new(sample_spec());
        let lines = gen.estimate_lines();
        assert!(lines > 50, "Expected more than 50 lines, got {}", lines);
    }

    #[test]
    fn test_all_frontend_frameworks_generate() {
        let frameworks = vec![
            FrontendFramework::React,
            FrontendFramework::Vue,
            FrontendFramework::Angular,
            FrontendFramework::Svelte,
            FrontendFramework::NextJs,
            FrontendFramework::Nuxt,
            FrontendFramework::SvelteKit,
        ];
        for fw in frameworks {
            let mut spec = sample_spec();
            spec.frontend = fw.clone();
            let gen = FullStackGenerator::new(spec);
            let files = gen.generate_frontend();
            assert!(
                !files.is_empty(),
                "Framework {:?} should generate files",
                fw
            );
        }
    }

    #[test]
    fn test_all_backend_frameworks_generate() {
        let frameworks = vec![
            BackendFramework::Express,
            BackendFramework::FastAPI,
            BackendFramework::Django,
            BackendFramework::SpringBoot,
            BackendFramework::Actix,
            BackendFramework::Gin,
            BackendFramework::Rails,
        ];
        for fw in frameworks {
            let mut spec = sample_spec();
            spec.backend = fw.clone();
            let gen = FullStackGenerator::new(spec);
            let files = gen.generate_backend();
            assert!(
                !files.is_empty(),
                "Framework {:?} should generate files",
                fw
            );
        }
    }

    #[test]
    fn test_generated_file_paths_contain_project_name() {
        let gen = FullStackGenerator::new(sample_spec());
        let project = gen.generate();
        for file in &project.files {
            assert!(
                file.path.starts_with("my-app") || file.path == "README.md",
                "File path should contain project name: {}",
                file.path
            );
        }
    }

    #[test]
    fn test_auth_strategy_oauth2() {
        let mut spec = sample_spec();
        spec.auth = AuthStrategy::OAuth2;
        let gen = FullStackGenerator::new(spec);
        let files = gen.generate_backend();
        assert!(files.iter().any(|f| f.path.contains("oauth")));
    }

    #[test]
    fn test_docker_compose_includes_database() {
        let gen = FullStackGenerator::new(sample_spec());
        let infra = gen.generate_infra();
        let compose = infra
            .iter()
            .find(|f| f.path.contains("docker-compose"))
            .expect("should have docker-compose");
        assert!(compose.content.contains("postgres"));
    }
}
