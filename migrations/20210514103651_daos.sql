-- Add migration script here
CREATE TABLE IF NOT EXISTS daos
(
  id            BIGSERIAL PRIMARY KEY,
  owner         CHAR(64) NOT NULL,
  height        BIGINT NOT NULL,
  g_id          CHAR(64) NOT NULL,
  g_type        SMALLINT NOT NULL,
  g_name        VARCHAR(255) NOT NULL,
  g_bio         TEXT NOT NULL,
  is_need_agree BOOLEAN NOT NULL DEFAULT FALSE,
  key_hash      CHAR(255) NOT NULL,
  is_closed     BOOLEAN NOT NULL DEFAULT FALSE,
  datetime      BIGINT  NOT NULL,
  is_deleted    BOOLEAN NOT NULL DEFAULT FALSE
);
