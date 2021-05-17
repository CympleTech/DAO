-- Add migration script here
CREATE TABLE IF NOT EXISTS messages
(
  id            BIGSERIAL PRIMARY KEY,
  fid           BIGINT NOT NULL,
  mid           BIGINT NOT NULL,
  m_name        CHAR(255) NOT NULL,
  m_type        SMALLINT NOT NULL,
  m_content     TEXT NOT NULL,
  datetime      BIGINT  NOT NULL,
  is_deleted    BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX message_index ON messages (fid);
