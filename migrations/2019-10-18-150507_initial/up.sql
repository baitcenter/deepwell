-- Account info

CREATE TABLE users (
    user_id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE CHECK (email = LOWER(email)),
    is_verified BOOLEAN NOT NULL DEFAULT false,
    is_special BOOLEAN NOT NULL DEFAULT false,
    is_bot BOOLEAN NOT NULL DEFAULT false,
    author_page TEXT NOT NULL DEFAULT '',
    website TEXT NOT NULL DEFAULT '',
    about TEXT NOT NULL DEFAULT '',
    gender TEXT NOT NULL DEFAULT '' CHECK (gender = LOWER(gender)),
    location TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

-- Wikis and wiki settings

CREATE TABLE wikis (
    wiki_id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE CHECK (slug ~ '[a-z0-9:_-]+'),
    domain TEXT NOT NULL UNIQUE CHECK (domain = LOWER(domain)),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE wiki_settings (
    wiki_id BIGINT PRIMARY KEY REFERENCES wikis(wiki_id),
    page_lock_duration SMALLINT NOT NULL CHECK (page_lock_duration > 0)
);

CREATE TABLE wiki_membership (
    wiki_id BIGINT NOT NULL REFERENCES wikis(wiki_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    applied_at TIMESTAMP WITH TIME ZONE NOT NULL,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL,
    banned_at TIMESTAMP WITH TIME ZONE, -- null = not banned
    banned_until TIMESTAMP WITH TIME ZONE, -- null = indefinite ban
    PRIMARY KEY (wiki_id, user_id)
);

CREATE TABLE roles (
    role_id BIGSERIAL PRIMARY KEY,
    wiki_id BIGINT NOT NULL REFERENCES wikis(wiki_id),
    name TEXT NOT NULL,
    permset JSONB NOT NULL,
    UNIQUE (wiki_id, name)
);

CREATE TABLE role_membership (
    wiki_id BIGINT REFERENCES wikis(wiki_id),
    role_id BIGINT REFERENCES roles(role_id),
    user_id BIGINT REFERENCES users(user_id),
    applied_at TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (wiki_id, role_Id, user_id)
);

-- Pages and revisions

CREATE TABLE pages (
    page_id BIGSERIAL PRIMARY KEY,
    wiki_id BIGINT NOT NULL REFERENCES wikis(wiki_id),
    slug TEXT NOT NULL CHECK (slug ~ '[a-z0-9:_-]+'),
    title TEXT NOT NULL,
    alt_title TEXT,
    tags TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE,
    UNIQUE (deleted_at, slug)
);

CREATE TABLE page_locks (
    page_id BIGINT PRIMARY KEY REFERENCES pages(page_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    locked_until TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE parents (
    page_id BIGINT NOT NULL REFERENCES pages(page_id),
    parent_page_id BIGINT NOT NULL REFERENCES pages(page_id),
    parented_by BIGINT NOT NULL REFERENCES users(user_id),
    parented_at TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (page_id, parent_page_id)
);

CREATE TABLE revisions (
    revision_id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    page_id BIGINT NOT NULL REFERENCES pages(page_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    message TEXT NOT NULL,
    git_commit CHAR(40) NOT NULL CHECK (git_commit ~ '[a-f0-9]+'),
    change_type VARCHAR(8) NOT NULL CHECK (
        change_type IN (
            'create',
            'modify',
            'delete',
            'restore',
            'rename',
            'undo',
            'tags'
        )
    )
);

CREATE TABLE tag_history (
    revision_id BIGINT REFERENCES revisions(revision_id) PRIMARY KEY,
    added_tags TEXT[] NOT NULL,
    removed_tags TEXT[] NOT NULL,
    CHECK (NOT(added_tags && removed_tags))
);

CREATE TABLE ratings (
    page_id BIGINT NOT NULL REFERENCES pages(page_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    rating SMALLINT NOT NULL,
    PRIMARY KEY (page_id, user_id)
);

CREATE TABLE ratings_history (
    rating_id BIGSERIAL PRIMARY KEY,
    page_id BIGINT NOT NULL REFERENCES pages(page_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    rating SMALLINT
);

CREATE TABLE authors (
    page_id BIGINT NOT NULL REFERENCES pages(page_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    author_type TEXT NOT NULL CHECK (
        author_type IN (
            'author',
            'rewrite',
            'translator',
            'maintainer'
        )
    ),
    written_at DATE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (page_id, user_id, author_type)
);

-- Hosted files

CREATE TABLE files (
    file_id BIGSERIAL PRIMARY KEY,
    file_name TEXT NOT NULL UNIQUE,
    file_uri TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    page_id BIGINT NOT NULL REFERENCES pages(page_id)
);

-- Active sessions

CREATE TABLE passwords (
    user_id BIGINT PRIMARY KEY REFERENCES users(user_id),
    hash BYTEA NOT NULL CHECK (LENGTH(hash) * 8 = 256),
    salt BYTEA NOT NULL CHECK (LENGTH(salt) * 8 = 128),
    logn SMALLINT NOT NULL CHECK (ABS(logn) < 128),
    param_r INTEGER NOT NULL,
    param_p INTEGER NOT NULL
);

CREATE TABLE login_attempts (
    login_attempt_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(user_id),
    username_or_email TEXT,
    remote_address TEXT,
    success BOOLEAN NOT NULL,
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- disallow deletions of login attempts
REVOKE DELETE, TRUNCATE ON TABLE login_attempts FROM public;

CREATE TABLE sessions (
    session_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    login_attempt_id BIGINT NOT NULL REFERENCES login_attempts(login_attempt_id)
);
