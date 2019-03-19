CREATE TABLE users
(
  uid             SERIAL       NOT NULL UNIQUE PRIMARY KEY,
  username        VARCHAR(32)  NOT NULL UNIQUE,
  email           VARCHAR(100) NOT NULL UNIQUE,
  hashed_password VARCHAR(64)  NOT NULL,
  avatar_url      VARCHAR(128) NOT NULL,
  signature       VARCHAR(256) NOT NULL,
  created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  is_admin        INTEGER      NOT NULL DEFAULT 0,
  blocked         BOOLEAN      NOT NULL DEFAULT false
);

CREATE TABLE topics
(
  tid           SERIAL        NOT NULL UNIQUE PRIMARY KEY,
  uid           INTEGER       NOT NULL REFERENCES users (uid),
  title_content VARCHAR(1024) NOT NULL,
  post_content  VARCHAR(1024) NOT NULL,
  created_at    TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE posts
(
  pid          SERIAL        NOT NULL UNIQUE PRIMARY KEY,
  uid          INTEGER       NOT NULL REFERENCES users (uid),
  to_tid       INTEGER       NOT NULL REFERENCES topics (tid),
  to_pid       INTEGER       NULL,
  post_content VARCHAR(1024) NOT NULL,
  created_at   TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at   TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);