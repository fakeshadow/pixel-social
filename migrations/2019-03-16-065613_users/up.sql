CREATE TABLE users
(
  id              SERIAL       NOT NULL UNIQUE PRIMARY KEY,
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

CREATE TABLE categories
(
  id    SERIAL        NOT NULL UNIQUE PRIMARY KEY,
  name  VARCHAR(128)  NOT NULL,
  theme VARCHAR(1024) NOT NULL
);

CREATE TABLE topics
(
  id          SERIAL        NOT NULL UNIQUE PRIMARY KEY,
  user_id     INTEGER       NOT NULL REFERENCES users (id),
  category_id INTEGER       NOT NULL REFERENCES categories (id),
  title       VARCHAR(1024) NOT NULL,
  body        VARCHAR(1024) NOT NULL,
  thumbnail   VARCHAR(1024) NOT NULL,
  created_at  TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at  TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE posts
(
  id           SERIAL        NOT NULL UNIQUE PRIMARY KEY,
  user_id      INTEGER       NOT NULL REFERENCES users (id),
  topic_id       INTEGER       NOT NULL REFERENCES topics (id),
  post_id      INTEGER       NOT NULL DEFAULT -1,
  post_content VARCHAR(1024) NOT NULL,
  created_at   TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at   TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP
);


CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);