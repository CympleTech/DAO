-- Add migration script here
CREATE TABLE IF NOT EXISTS managers
(
  id            BIGSERIAL PRIMARY KEY,
  gid           CHAR(64) NOT NULL,
  times         INTEGER NOT NULL,
  datetime      BIGINT  NOT NULL,
  is_closed     BOOLEAN NOT NULL DEFAULT FALSE,
  is_deleted    BOOLEAN NOT NULL DEFAULT FALSE
);
