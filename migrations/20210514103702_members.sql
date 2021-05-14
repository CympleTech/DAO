-- Add migration script here
CREATE TABLE IF NOT EXISTS members
(
  id            BIGSERIAL PRIMARY KEY,
  fid           BIGINT NOT NULL,
  m_id          CHAR(64) NOT NULL,
  m_addr        CHAR(64) NOT NULL,
  m_name        CHAR(255) NOT NULL,
  is_manager    BOOLEAN NOT NULL DEFAULT FALSE,
  datetime      BIGINT  NOT NULL,
  is_deleted    BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX member_index ON members (fid, m_id);
