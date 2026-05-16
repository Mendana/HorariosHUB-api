# 1. Copiar variables de entorno

cp .env.example .env

# 2. Levantar la base de datos

docker compose -f docker-compose.dev.yml up db -d

# 3. Ejecutar migraciones (primera vez)

sqlx migrate run --database-url postgres://horariosUser:horariosUser_dev@db:5432/horarioshub

# 4. Generar sqlx-data.json para CI offline

cargo sqlx prepare --database-url postgres://horariosUser:horariosUser_dev@db:5432/horarioshub

# 5. Levantar todo

docker compose -f docker-compose.dev.yml up
