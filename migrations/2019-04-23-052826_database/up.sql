CREATE TABLE users
(
    id              OID          NOT NULL UNIQUE PRIMARY KEY,
    username        VARCHAR(32)  NOT NULL UNIQUE,
    email           VARCHAR(100) NOT NULL UNIQUE,
    hashed_password VARCHAR(64)  NOT NULL,
    avatar_url      VARCHAR(128) NOT NULL,
    signature       VARCHAR(256) NOT NULL,
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_admin        OID          NOT NULL DEFAULT 0,
    blocked         BOOLEAN      NOT NULL DEFAULT FALSE,
    show_email      BOOLEAN      NOT NULL DEFAULT TRUE,
    show_created_at BOOLEAN      NOT NULL DEFAULT TRUE,
    show_updated_at BOOLEAN      NOT NULL DEFAULT TRUE
);

CREATE TABLE categories
(
    id    OID           NOT NULL UNIQUE PRIMARY KEY,
    name  VARCHAR(128)  NOT NULL,
    topic_count OID NOT NULL DEFAULT 0,
    post_count OID NOT NULL DEFAULT 0,
    subscriber_count OID NOT NULL DEFAULT 0,
    thumbnail VARCHAR(256) NOT NULL
);

CREATE TABLE topics
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    category_id     OID           NOT NULL,
    title           VARCHAR(1024) NOT NULL,
    body            VARCHAR(1024) NOT NULL,
    thumbnail       VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE posts
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE associates
(
    id               OID       NOT NULL UNIQUE PRIMARY KEY,
    user_id          OID       NOT NULL UNIQUE,
    psn_id           VARCHAR(128) UNIQUE,
    live_id          VARCHAR(128) UNIQUE,
    last_update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);



--Placeholder data below.Safe to delete
--admin password is 1234asdf
INSERT INTO users (id, username, email, hashed_password, signature, avatar_url, is_admin)
VALUES (1,'adminuser', 'admin@pixelshare', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser', 'avatar_url', 9);

INSERT INTO categories (id, name, thumbnail)
VALUES (1, 'General', ''),
(2, 'Announcement', '');

INSERT INTO categories (id, name, thumbnail)
VALUES (3, 'Armored Core', 'AC.jpg'),
(4, 'Ace Combat', 'ACE.jpg'),
(5, 'Persona', 'persona.jpeg');

INSERT INTO topics ( id, user_id, category_id, title, body, thumbnail)
VALUES (1, 1, 1, 'Welcome To PixelShare', 'PixelShare is a gaming oriented community.', '');