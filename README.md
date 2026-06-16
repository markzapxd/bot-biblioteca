# Bibliotecário — Discord Server Management Bot

Bot Discord para gerenciamento completa de servidores, escrito em Rust com PostgreSQL.

## Funcionalidades

- **Verificação de Membros** — Sistema de aprovação por staff com indicação de referência
- **Tracking de Voz** — Registro automático de sessões de voz com duração, participantes e histórico
- **Anti-Raid** — Detecção automática de joins em massa, spam e menções excessivas com lockdown
- **Tickets de Suporte** — Sistema de suporte privado entre membros e staff
- **Logs Detalhados** — Edições/deleções de mensagens, mudanças de cargo, entrada/saída de membros
- **Histórico de Avatar/Username** — Rastreamento automático de mudanças de perfil
- **Monitoramento Específico** — Tracking dedicado de usuários específicos
- **Dashboard Web** — API REST para administração remota

## Stack

| Componente | Tecnologia |
|---|---|
| Linguagem | Rust (edition 2021) |
| Framework Discord | Serenity 0.12 |
| Runtime | Tokio |
| Banco de Dados | PostgreSQL 16 |
| ORM/Queries | SQLx 0.8 |
| API Dashboard | Axum 0.7 |
| Serialização | Serde + serde_json |
| Configuração | dotenvy |
| Logs | tracing + tracing-subscriber |
| Datas | chrono |

## Instalação

### Pré-requisitos

- Rust 1.75+ (via [rustup](https://rustup.rs))
- PostgreSQL 16+
- Docker e Docker Compose (opcional)

### Configuração do Banco de Dados

**Opção 1: Docker Compose (recomendado)**

```bash
docker compose up -d db
```

**Opção 2: PostgreSQL local**

```bash
createdb bibliotecario
createuser bibliotecario -P
```

### Configuração do Ambiente

```bash
cp .env.example .env
```

Edite `.env` com suas credenciais:

```env
DISCORD_TOKEN=seu_token_discord
CLIENT_ID=seu_client_id_discord
DATABASE_URL=postgres://bibliotecario:bibliotecario@localhost:5432/bibliotecario
LOG_LEVEL=info
PORT=3050
OWNER_ID=seu_discord_user_id
```

### Obtendo o Token Discord

1. Acesse [Discord Developer Portal](https://discord.com/developers/applications)
2. Crie uma nova aplicação
3. Em "Bot", copie o token
4. Em "OAuth2", copie o Client ID
5. Gere um link de convite com as permissões necessárias:
   - Administrator
   - Manage Channels
   - Manage Roles
   - Manage Messages
   - Kick Members
   - Ban Members
   - Moderate Members
   - View Audit Log

### Intents Necessários

No Developer Portal, em "Bot" → "Privileged Gateway Intents", ative:
- Server Members Intent
- Message Content Intent
- Presence Intent

## Execução Local

```bash
cargo run
```

O bot irá:
1. Conectar ao PostgreSQL
2. Executar migrations automaticamente
3. Carregar configurações de guilds
4. Conectar ao Discord Gateway
5. Registrar comandos slash em cada guild
6. Iniciar o dashboard API na porta configurada

## Deploy com Docker

```bash
docker compose up -d --build
```

## Estrutura do Projeto

```
src/
├── main.rs              # Entry point: client Discord + handler registration
├── lib.rs               # Module declarations
├── config/              # Configuração via variáveis de ambiente
├── commands/            # 19 slash commands do Discord
│   ├── admin.rs         # /admin ban|kick|mute|unmute|warn
│   ├── clearuser.rs     # /clearuser — limpeza de mensagens por usuário
│   ├── help.rs          # /help — manual de comandos
│   ├── info.rs          # /info — estatísticas do servidor
│   ├── lastcall.rs      # /lastcall — última sessão de voz
│   ├── lockdown.rs      # /lockdown — atribuição em massa de cargo
│   ├── modulos.rs       # /modulos — gerenciamento de módulos ativos
│   ├── names.rs         # /names — histórico de usernames
│   ├── nuke.rs          # /nuke — destruição e recriação de canal
│   ├── owner.rs         # /owner — comandos exclusivos do dono
│   ├── privacy.rs       # /privacy — controle de privacidade
│   ├── raidmode.rs      # /raidmode — controle manual de raid mode
│   ├── setup.rs         # /setup — setup automático de sistemas
│   ├── setup_frin.rs    # /setup-frin — monitoramento específico
│   ├── stats.rs         # /stats — ranking de tempo em voz
│   ├── ticket.rs        # /ticket — painel de tickets
│   ├── userinfo.rs      # /userinfo — ficha tática de usuário
│   ├── verification.rs  # /v — painel de verificação
│   └── voicehistory.rs  # /voicehistory — histórico de sessões de voz
├── events/              # 11 event handlers do Discord
│   ├── ready.rs
│   ├── interaction_create.rs
│   ├── voice_state_update.rs
│   ├── user_update.rs
│   ├── guild_member_add.rs
│   ├── guild_member_remove.rs
│   ├── guild_member_update.rs
│   ├── message_create.rs
│   ├── message_delete.rs
│   ├── message_update.rs
│   └── presence_update.rs
├── services/            # Lógica de negócio
│   ├── anti_raid.rs     # Detecção e resposta a raids
│   ├── member_manager.rs # Fluxo de verificação de membros
│   ├── ticket_manager.rs # Sistema de tickets
│   ├── log_manager.rs   # Logs centralizados
│   ├── avatar_manager.rs # Histórico de avatares
│   └── user_info_manager.rs # Ficha tática de usuário
├── repositories/        # Acesso a dados (SQLx)
│   ├── guild_repo.rs
│   ├── user_repo.rs
│   └── voice_session_repo.rs
├── database/            # Conexão e migrations
├── models/              # Modelos de dados (FromRow)
│   ├── guild.rs
│   ├── user.rs
│   └── voice_session.rs
├── dto/                 # Data Transfer Objects (API)
├── embeds/              # Builders de embeds Discord
├── permissions/         # Controle de permissões
├── cache/               # Cache em memória (DashMap)
├── jobs/                # Tarefas de fundo
├── utils/               # Utilitários (tempo, logging)
├── errors/              # Tipos de erro
└── tests/               # Testes unitários
```

## Comandos Slash

| Comando | Permissão | Descrição |
|---|---|---|
| `/admin ban` | ModerateMembers | Banir usuário |
| `/admin kick` | ModerateMembers | Expulsar usuário |
| `/admin mute` | ModerateMembers | Silenciar usuário por N minutos |
| `/admin unmute` | ModerateMembers | Remover silenciamento |
| `/admin warn` | ModerateMembers | Enviar advertência via DM |
| `/clearuser` | ManageMessages | Deletar mensagens de um usuário |
| `/help` | — | Listar todos os comandos |
| `/info` | — | Estatísticas do servidor |
| `/lastcall` | — | Última sessão de voz de um usuário |
| `/lockdown` | Administrator | Atribuir cargo a todos os membros |
| `/modulos` | Administrator | Ativar/desativar/listar módulos |
| `/names` | — | Histórico de usernames |
| `/nuke` | Administrator | Destruir e recriar canal |
| `/owner *` | Bot Owner | Comandos exclusivos do dono |
| `/privacy` | — | Ativar/desativar modo privado |
| `/raidmode` | Administrator | Ativar/desativar modo raid manual |
| `/setup` | Administrator | Setup automático de sistemas |
| `/setup-frin` | Administrator | Configurar canal de monitoramento |
| `/stats` | — | Ranking de tempo em voz |
| `/ticket` | ManageGuild | Postar painel de tickets |
| `/userinfo` | — | Ficha tática de usuário |
| `/v` | — | Postar painel de verificação |
| `/voicehistory` | — | Histórico de sessões de voz |

## Dashboard API

| Método | Rota | Descrição |
|---|---|---|
| GET | `/health` | Health check |
| GET | `/api/stats` | Estatísticas gerais |
| GET | `/api/guilds` | Lista de guilds |
| GET | `/api/config/:guildId` | Configuração de guild |
| POST | `/api/config/:guildId` | Atualizar configuração |
| POST | `/api/message` | Enviar mensagem |
| POST | `/api/role` | Criar cargo |
| GET | `/api/messages/:channelId` | Mensagens recentes |

## Banco de Dados

### Tabelas

- **guilds** — Configurações por servidor (cargos, canais, módulos ativos)
- **users** — Dados de usuários (privacidade, tempo de voz, históricos)
- **voice_sessions** — Sessões individuais de voz (duração, participantes)

### Migrations

As migrations são executadas automaticamente na inicialização via `migrations/001_initial_schema.sql`.

## Testes

```bash
cargo test
```

## Licença

Este projeto é de uso interno.
