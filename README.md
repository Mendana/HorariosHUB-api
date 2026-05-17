# horarioshub-api

API REST del backend de HorariosHub. Construida en Rust con [Axum](https://github.com/tokio-rs/axum), PostgreSQL (via sqlx) y autenticación JWT. Gestiona usuarios, autenticación y horarios académicos.

## Requisitos

- Rust 1.88+ (`rustup update stable`)
- Docker y Docker Compose
- sqlx-cli (solo para gestionar migraciones en local):

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

---

## Stack

- **Rust** · Axum, Tower, sqlx, tokio
- **PostgreSQL 16** como base de datos principal
- **Moka** para caché en memoria
- **lettre** para envío de correos (SMTP)
- **Docker** para desarrollo y despliegue

---

## Variables de entorno

Copia el fichero de ejemplo y ajusta los valores:

```bash
cp .env.example .env
```

| Variable | Descripción |
|---|---|
| `DATABASE_URL` | URL de conexión a PostgreSQL |
| `JWT_SECRET` | Clave secreta para firmar JWT (mínimo 32 caracteres) |
| `JWT_ACCESS_TTL_SECONDS` | Duración del token de acceso en segundos |
| `SERVER_PORT` | Puerto en el que escucha la API |
| `RUST_LOG` | Nivel de log (`debug`, `info`, `warn`, `error`) |
| `RUST_ENV` | Entorno de ejecución (`development` / `production`) |
| `SMTP_HOST` | Servidor SMTP |
| `SMTP_PORT` | Puerto SMTP |
| `SMTP_USER` | Usuario SMTP |
| `SMTP_PASSWORD` | Contraseña SMTP |
| `SMTP_FROM` | Dirección y nombre del remitente |
| `BASE_URL` | URL base pública de la aplicación (usada en enlaces de correo) |

---

## Desarrollo

### Levantar el entorno

```bash
# Base de datos en segundo plano
docker compose -f docker-compose.dev.yml up db -d

# API con hot-reload (cargo-watch dentro del contenedor)
docker compose -f docker-compose.dev.yml up backend
```

O todo de una vez:

```bash
docker compose -f docker-compose.dev.yml up
```

### Compilar y ejecutar en local

```bash
# Debug
cargo build

# Release
cargo build --release

# Ejecutar directamente
cargo run
```

---

## Comandos de calidad de código

Estos son los checks que ejecuta la CI. Deben pasar antes de abrir una MR.

```bash
# Formato
cargo fmt --all

# Linting (con warnings como errores, igual que CI)
cargo clippy --all-targets --all-features -- -D warnings

# Tests (requieren Docker en ejecución para los integration tests con testcontainers)
cargo test --all-features --all-targets
```

---

## Migraciones y sqlx

### Crear una migración nueva

```bash
sqlx migrate add <nombre_descriptivo>
```

Genera dos ficheros en `migrations/` con el timestamp como prefijo.

### Aplicar migraciones

```bash
sqlx migrate run --database-url postgres://horariosUser:horariosUser_dev@localhost:5432/horarioshub
```

### Revertir la última migración

```bash
sqlx migrate revert --database-url postgres://horariosUser:horariosUser_dev@localhost:5432/horarioshub
```

### Regenerar `.sqlx` (offline mode)

El directorio `.sqlx` contiene los metadatos de las queries compiladas, necesario para que la CI pueda compilar sin base de datos (`SQLX_OFFLINE=true`). Hay que regenerarlo cada vez que se añade o modifica una query:

```bash
cargo sqlx prepare --database-url postgres://horariosUser:horariosUser_dev@localhost:5432/horarioshub
```

> **Importante:** ejecuta esto antes de hacer commit si has tocado queries SQL. La CI valida con `sqlx prepare --check` y fallará si el `.sqlx` no está actualizado.

Confirma que el `.sqlx` está sincronizado con el código actual (lo que hace la CI):

```bash
cargo sqlx prepare --check --database-url postgres://horariosUser:horariosUser_dev@localhost:5432/horarioshub
```

> Las migraciones se aplican automáticamente al arrancar la aplicación.

---

## Despliegue

La imagen de producción se construye con `docker/backend.Dockerfile`, que usa un build multi-stage para producir un binario optimizado sobre `debian:bookworm-slim`.

La CI publica automáticamente en el registry de GitLab cuando hay un commit en `main`:

```
registry.gitlab.com/<grupo>/<proyecto>:<commit-sha>
registry.gitlab.com/<grupo>/<proyecto>:latest
```

Para construir la imagen de producción manualmente:

```bash
docker build -f docker/backend.Dockerfile -t horarioshub-api:latest .
```
