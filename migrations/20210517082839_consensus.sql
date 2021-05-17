-- Add migration script here
CREATE TABLE IF NOT EXISTS consensus
(
  id            BIGSERIAL PRIMARY KEY,
  fid           BIGINT NOT NULL,
  height        BIGINT NOT NULL,
  ctype         SMALLINT NOT NULL,
  cid           BIGINT NOT NULL
);
CREATE INDEX consensus_index ON consensus (fid, height);
