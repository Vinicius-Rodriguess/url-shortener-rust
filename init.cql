-- init.cql

CREATE KEYSPACE IF NOT EXISTS shortener
WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};

-- Exemplo: Adicione aqui o CREATE TABLE para sua tabela de URLs
-- USE shortener;
-- CREATE TABLE IF NOT EXISTS urls (
--     short_code text PRIMARY KEY,
--     original_url text,
--     created_at timestamp
-- );

-- O comando de checagem do healthcheck já deve garantir que esta parte já esteja processada.
-- Seu backend deve ser capaz de criar as tabelas se não existirem, mas o keyspace é crucial.