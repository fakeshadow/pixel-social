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
    id               OID          NOT NULL UNIQUE PRIMARY KEY,
    name             VARCHAR(128) NOT NULL,
    topic_count      INTEGER      NOT NULL DEFAULT 0,
    post_count       INTEGER      NOT NULL DEFAULT 0,
    subscriber_count INTEGER      NOT NULL DEFAULT 0,
    thumbnail        VARCHAR(256) NOT NULL
);

CREATE TABLE topics1
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

CREATE TABLE topics2
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

CREATE TABLE topics3
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

CREATE TABLE topics4
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

CREATE TABLE topics5
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

-- posts[N] N is category_id
CREATE TABLE posts1
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE posts2
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE posts3
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE posts4
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);

CREATE TABLE posts5
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
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

CREATE TABLE talks
(
    id          OID          NOT NULL UNIQUE PRIMARY KEY,
    name        VARCHAR(128) NOT NULL UNIQUE,
    description VARCHAR(128) NOT NULL,
    owner       OID          NOT NULL,
    admin       OID[]        NOT NULL,
    users       OID[]        NOT NULL
);

CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX talks_name ON talks (name);

CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);

--Placeholder data below.Safe to delete
--admin password is 1234asdf
INSERT INTO users (id, username, email, hashed_password, signature, avatar_url, is_admin)
VALUES (1, 'adminuser', 'admin@pixelshare', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser',
        'avatar_url', 9);

INSERT INTO categories (id, name, thumbnail, topic_count, post_count)
VALUES (1, 'General', 'category_default.png', 1, 1);

INSERT INTO categories (id, name, thumbnail)
VALUES (2, 'Announcement', 'category_default.png'),
       (3, 'Armored Core', 'ac.jpg'),
       (4, 'Ace Combat', 'ace.jpg'),
       (5, 'Persona', 'persona.jpg');

INSERT INTO topics1 (id, user_id, category_id, title, body, thumbnail)
VALUES (1, 1, 1, 'Welcome To PixelShare', 'PixelShare is a gaming oriented community.', '');

INSERT INTO posts1 (id, user_id, topic_id, category_id, post_content)
VALUES (1, 1, 1, 1, 'First Reply Only to stop cache build from complaining');

CREATE OR REPLACE FUNCTION adding_topic1() RETURNS trigger AS
$added_reply$
BEGIN
    UPDATE categories
    SET topic_count = topic_count + 1
    WHERE id = 1;
    RETURN NULL;
END;
$added_reply$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic1
    AFTER INSERT
    ON topics1
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic1();

CREATE OR REPLACE FUNCTION adding_topic2() RETURNS trigger AS
$added_reply$
BEGIN
    UPDATE categories
    SET topic_count = topic_count + 1
    WHERE id = 2;
    RETURN NULL;
END;
$added_reply$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic2
    AFTER INSERT
    ON topics2
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic2();

CREATE OR REPLACE FUNCTION adding_topic3() RETURNS trigger AS
$added_reply$
BEGIN
    UPDATE categories
    SET topic_count = topic_count + 1
    WHERE id = 3;
    RETURN NULL;
END;
$added_reply$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic3
    AFTER INSERT
    ON topics3
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic3();

CREATE OR REPLACE FUNCTION adding_topic4() RETURNS trigger AS
$added_reply$
BEGIN
    UPDATE categories
    SET topic_count = topic_count + 1
    WHERE id = 4;
    RETURN NULL;
END;
$added_reply$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic4
    AFTER INSERT
    ON topics4
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic4();


CREATE OR REPLACE FUNCTION adding_topic5() RETURNS trigger AS
$added_reply$
BEGIN
    UPDATE categories
    SET topic_count = topic_count + 1
    WHERE id = 5;
    RETURN NULL;
END;
$added_reply$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic5
    AFTER INSERT
    ON topics5
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic5();


-- ToDo: generate triggers using dynamic methods
-- reject illegal post_id ,topic_id
-- update category and topic table
CREATE OR REPLACE FUNCTION adding_post1() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics1 WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;
    IF NEW.post_id IS NOT NULL AND NOT EXISTS(SELECT id FROM posts1 WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        NEW.post_id = NULL;
    END IF;
    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;
    UPDATE topics1
    SET reply_count     = reply_count + 1,
        last_reply_time = DEFAULT
    WHERE id = NEW.topic_id;

    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION adding_post2() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics2 WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;
    IF NEW.post_id IS NOT NULL AND NOT EXISTS(SELECT id FROM posts2 WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        NEW.post_id = NULL;
    END IF;
    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;
    UPDATE topics2
    SET reply_count     = reply_count + 1,
        last_reply_time = DEFAULT
    WHERE id = NEW.topic_id;
    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION adding_post3() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics3 WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;
    IF NEW.post_id IS NOT NULL AND NOT EXISTS(SELECT id FROM posts3 WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        NEW.post_id = NULL;
    END IF;
    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;
    UPDATE topics3
    SET reply_count     = reply_count + 1,
        last_reply_time = DEFAULT
    WHERE id = NEW.topic_id;
    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION adding_post4() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics4 WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;
    IF NEW.post_id IS NOT NULL AND NOT EXISTS(SELECT id FROM posts4 WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        NEW.post_id = NULL;
    END IF;
    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;
    UPDATE topics4
    SET reply_count     = reply_count + 1,
        last_reply_time = DEFAULT
    WHERE id = NEW.topic_id;
    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION adding_post5() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics5 WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;
    IF NEW.post_id IS NOT NULL AND NOT EXISTS(SELECT id FROM posts5 WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        NEW.post_id = NULL;
    END IF;
    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;
    UPDATE topics5
    SET reply_count     = reply_count + 1,
        last_reply_time = DEFAULT
    WHERE id = NEW.topic_id;
    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE TRIGGER adding_post1
    BEFORE INSERT
    ON posts1
    FOR EACH ROW
EXECUTE PROCEDURE adding_post1();

CREATE TRIGGER adding_post2
    BEFORE INSERT
    ON posts2
    FOR EACH ROW
EXECUTE PROCEDURE adding_post2();

CREATE TRIGGER adding_post3
    BEFORE INSERT
    ON posts3
    FOR EACH ROW
EXECUTE PROCEDURE adding_post3();

CREATE TRIGGER adding_post4
    BEFORE INSERT
    ON posts4
    FOR EACH ROW
EXECUTE PROCEDURE adding_post4();

CREATE TRIGGER adding_post5
    BEFORE INSERT
    ON posts5
    FOR EACH ROW
EXECUTE PROCEDURE adding_post5();