# Bibliotecaria — Discord Server Management Bot

Bot Discord para gerenciamento completo de servidores, escrito em Rust com PostgreSQL.

## Funcionalidades

- **Verificação de Membros** — Sistema de aprovação por staff com indicação de referência
- **Tracking de Voz** — Registro automático de sessões de voz com duração, participantes e histórico
- **Anti-Raid** — Detecção automática de joins em massa, spam e menções excessivas com lockdown
- **Tickets de Suporte** — Sistema de suporte privado entre membros e staff com painel, claim, adição/remoção de usuários e fechamento
- **Logs Detalhados** — Edições/deleções de mensagens, mudanças de cargo, entrada/saída de membros, com configuração por módulo
- **Histórico de Avatar/Username/Apelido** — Rastreamento automático de mudanças de perfil com navegação interativa
- **Monitoramento Específico** — Tracking dedicado de usuários específicos
- **Cards Personalizados** — Sistema de cards/embeds customizáveis por servidor
- **Infrações Automáticas** — Contador de infrações com reset automático após 7 dias
- **Lookup de Moderação** — Ficha completa de usuário com ações de moderação
- **Gerenciamento de Assets** — Imagens e thumbnails em embeds via cache em memória
- **Dashboard Web** — API REST para administração remota
- **Tema Unificado** — Paleta de cores e embeds consistentes (#2B2D31)

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
| Configuração | dotenvy + toml |
| Logs | tracing + tracing-subscriber |
| Datas | chrono |
| UUID | uuid |
| HTTP Client | reqwest |
| Cache | DashMap |
| Middleware HTTP | tower + tower-http (CORS) |
| Error Handling | thiserror |

## Instalação

### Pré-requisitos

- Rust 1.75+ (via [rustup](https://rustup.rs))
- PostgreSQL 16+
- Docker e Docker Compose (opcional)

### Configuração do Banco de Dados

**Opção 1: Docker Compose (recomendado)**

```bash
rtk docker compose up -d db
```

**Opção 2: PostgreSQL local**

```bash
rtk createdb bibliotecario
rtk createuser bibliotecario -P
```

### Configuração do Ambiente

```bash
rtk cp .env.example .env
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
rtk cargo run
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
rtk docker compose up -d --build
```

## Estrutura do Projeto

```
src/
├── main.rs              # Entry point: client Discord + handler registration
├── lib.rs               # Module declarations
├── state.rs             # Estado global do bot (pool, cache, asset_manager)
├── config/              # Configuração via variáveis de ambiente
├── commands/            # 27 slash commands do Discord
│   ├── admin.rs         # /admin ban|kick|mute|unmute|warn
│   ├── addticket.rs     # /addticket — adicionar usuário ao ticket
│   ├── botstats.rs      # /botstats — estatísticas do bot (owner)
│   ├── card.rs          # /card — sistema de cards personalizados
│   ├── claimticket.rs   # /claimticket — assumir responsabilidade pelo ticket
│   ├── clearuser.rs     # /clearuser — limpeza de mensagens por usuário
│   ├── closeticket.rs   # /closeticket — fechar ticket de um usuário
│   ├── config.rs        # /config — configurar cargos e canais do servidor
│   ├── configlogs.rs    # /configlogs — configurar funções de logs
│   ├── help.rs          # /help — manual de comandos
│   ├── info.rs          # /info — estatísticas do servidor
│   ├── lastcall.rs      # /lastcall — última sessão de voz
│   ├── lockdown.rs      # /lockdown — atribuição em massa de cargo
│   ├── lookup.rs        # /lookup — ficha completa de moderação
│   ├── modulos.rs       # /modulos — gerenciamento de módulos ativos
│   ├── names.rs         # /names — histórico de usernames/nicknames
│   ├── nuke.rs          # /nuke — destruição e recriação de canal
│   ├── owner.rs         # /owner — comandos exclusivos do dono
│   ├── privacy.rs       # /privacy — controle de privacidade
│   ├── raidmode.rs      # /raidmode — controle manual de raid mode
│   ├── removeticket.rs  # /removeticket — remover usuário do ticket
│   ├── shutdown.rs      # /shutdown — desligar o bot (owner)
│   ├── stats.rs         # /stats — ranking de tempo em voz
│   ├── sync.rs          # /sync — re-registrar comandos em todos os servidores (owner)
│   ├── ticketpanel.rs   # /ticketpanel — postar painel de tickets
│   ├── userinfo.rs      # /userinfo — ficha tática de usuário
│   ├── verification.rs  # /verify — painel de verificação
│   └── voicehistory.rs  # /voicehistory — histórico de sessões de voz
├── events/              # 12 event handlers do Discord
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
│   ├── history_manager.rs # Histórico de nomes/nicknames/avatares (UI)
│   └── user_info_manager.rs # Ficha tática de usuário
├── repositories/        # Acesso a dados (SQLx)
│   ├── guild_repo.rs
│   ├── user_repo.rs
│   ├── voice_session_repo.rs
│   ├── custom_card_repo.rs
│   └── user_infraction_repo.rs
├── models/              # Modelos de dados (FromRow)
│   ├── guild.rs
│   ├── user.rs
│   ├── voice_session.rs
│   └── custom_card.rs
├── database/            # Conexão e migrations
├── dto/                 # Data Transfer Objects (API)
├── embeds/              # Builders de embeds Discord
├── permissions/         # Controle de permissões
├── cache/               # Cache em memória (DashMap)
├── asset_manager/       # Gerenciamento de assets (imagens)
├── theme/               # Tema e paleta de cores
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
| `/addticket` | ModerateMembers | Adicionar usuário ao ticket atual |
| `/botstats` | Bot Owner | Estatísticas do bot (guilds, usuários, uptime) |
| `/card` | ManageGuild | Sistema de cards/embeds personalizados |
| `/claimticket` | ModerateMembers | Assumir responsabilidade pelo ticket |
| `/clearuser` | ManageMessages | Deletar mensagens de um usuário |
| `/closeticket` | ModerateMembers | Fechar ticket de um usuário |
| `/config` | Administrator | Configurar cargos e canais do servidor |
| `/configlogs` | Administrator | Configurar funções de logs por módulo |
| `/help` | — | Listar todos os comandos |
| `/info` | — | Estatísticas do servidor |
| `/lastcall` | — | Última sessão de voz de um usuário |
| `/lockdown` | Administrator | Atribuir cargo a todos os membros |
| `/lookup` | ModerateMembers | Ficha completa de usuário com ações de moderação |
| `/modulos` | Administrator | Ativar/desativar/listar módulos |
| `/names` | — | Histórico de usernames e nicknames |
| `/nuke` | Administrator | Destruir e recriar canal |
| `/owner *` | Bot Owner | Comandos exclusivos do dono |
| `/privacy` | — | Ativar/desativar modo privado |
| `/raidmode` | Administrator | Ativar/desativar modo raid manual |
| `/removeticket` | ModerateMembers | Remover usuário do ticket atual |
| `/shutdown` | Bot Owner | Desligar o bot |
| `/stats` | — | Ranking de tempo em voz |
| `/sync` | Bot Owner | Re-registrar comandos em todos os servidores |
| `/ticketpanel` | ManageGuild | Postar painel de tickets |
| `/userinfo` | — | Ficha tática de usuário |
| `/verify` | — | Postar painel de verificação |
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

- **guilds** — Configurações por servidor (cargos, canais, módulos ativos, flags)
- **users** — Dados de usuários (privacidade, tempo de voz, históricos de username/nickname/avatar)
- **voice_sessions** — Sessões individuais de voz (duração, participantes)
- **custom_cards** — Cards/embeds personalizados por servidor
- **user_infractions** — Contador de infrações por usuário/guild (reset após 7 dias)

### Migrations

As migrations são executadas automaticamente na inicialização:
- `migrations/001_initial_schema.sql`
- `migrations/002_custom_cards.sql`

## Testes

```bash
rtk cargo test
```

## Licença

Este projeto é de uso interno.
