-- Account info

CREATE TABLE users (
    user_id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    author_page TEXT NOT NULL DEFAULT '',
    website TEXT NOT NULL DEFAULT '',
    about TEXT NOT NULL DEFAULT '',
    gender TEXT NOT NULL DEFAULT '' CHECK (gender = LOWER(gender)),
    location TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP
);

CREATE TABLE passwords (
    user_id BIGSERIAL PRIMARY KEY REFERENCES users(user_id),
    hash BYTEA NOT NULL,
    salt BYTEA NOT NULL,
    iterations INTEGER NOT NULL CHECK (iterations > 50000),
    key_size SMALLINT NOT NULL CHECK (key_size % 16 = 0),
    digest VARCHAR(8) NOT NULL CHECK (
        digest IN (
            'sha224',
            'sha256',
            'sha384',
            'sha512'
        )
    )
);

-- Pages and revisions

CREATE TABLE pages (
    page_id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    alt_title TEXT,
    tags TEXT[] NOT NULL,
    UNIQUE (deleted_at, slug)
);

CREATE TABLE parents (
    page_id BIGSERIAL NOT NULL REFERENCES pages(page_id),
    parent_page_id BIGSERIAL NOT NULL REFERENCES pages(page_id),
    parented_by BIGSERIAL NOT NULL REFERENCES users(user_id),
    parented_at TIMESTAMP NOT NULL,
    PRIMARY KEY (page_id, parent_page_id)
);

CREATE TABLE revisions (
    revision_id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    page_id BIGSERIAL NOT NULL REFERENCES pages(page_id),
    user_id BIGSERIAL NOT NULL REFERENCES users(user_id),
    git_commit TEXT NOT NULL UNIQUE,
    change_type VARCHAR(8) NOT NULL CHECK (
        change_type IN (
            'create',
            'modify',
            'delete',
            'metadata'
        )
    )
);

CREATE TABLE ratings (
    page_id BIGSERIAL NOT NULL,
    user_id BIGSERIAL NOT NULL,
    rating SMALLINT NOT NULL,
    PRIMARY KEY (page_id, user_id)
);

CREATE TABLE ratings_history (
    rating_id BIGSERIAL PRIMARY KEY,
    page_id BIGSERIAL NOT NULL REFERENCES pages(page_id),
    user_id BIGSERIAL NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    rating SMALLINT
);

CREATE TABLE authors (
    page_id BIGSERIAL NOT NULL REFERENCES pages(page_id),
    user_id BIGSERIAL NOT NULL REFERENCES users(user_id),
    author_type TEXT NOT NULL CHECK (
        author_type IN (
            'author',
            'rewrite',
            'translator',
            'maintainer'
        )
    ),
    created_at DATE NOT NULL,
    PRIMARY KEY (page_id, user_id, author_type)
);

-- Hosted files

CREATE TABLE files (
    file_id BIGSERIAL PRIMARY KEY,
    file_name TEXT NOT NULL UNIQUE,
    file_uri TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    page_id BIGSERIAL NOT NULL REFERENCES pages(page_id)
);

-- Wikis and wiki settings

CREATE TABLE wikis (
    wiki_id BIGSERIAL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE wiki_membership (
    wiki_id BIGSERIAL NOT NULL REFERENCES wikis(wiki_id),
    user_id BIGSERIAL NOT NULL REFERENCES users(user_id),
    applied_at TIMESTAMP NOT NULL,
    joined_at TIMESTAMP NOT NULL,
    PRIMARY KEY (wiki_id, user_id)
);

CREATE TABLE roles (
    role_id BIGSERIAL PRIMARY KEY,
    wiki_id BIGSERIAL NOT NULL REFERENCES wikis(wiki_id),
    name TEXT NOT NULL,
    permset BIT(20) NOT NULL,
    UNIQUE (wiki_id, name)
);

CREATE TABLE role_membership (
    wiki_id BIGSERIAL REFERENCES wikis(wiki_id),
    role_id BIGSERIAL REFERENCES roles(role_id),
    user_id BIGSERIAL REFERENCES users(user_id),
    applied_at TIMESTAMP NOT NULL,
    PRIMARY KEY (wiki_id, role_Id, user_id)
);
