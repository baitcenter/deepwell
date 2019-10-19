INSERT INTO users (user_id, name, email, is_verified, website, about, location)
VALUES (0, 'unknown', 'unknown@example.com', true, 'https://example.com/', 'Standard account for unknown users', 'unknown');

INSERT INTO users (user_id, name, email, is_verified, website, about, location)
VALUES (1, 'administrator', 'noreply@example.com', true, 'https://example.com/', 'Standard account for root-level access', 'Site-01');

INSERT INTO users (user_id, name, email, is_verified, website, about, location)
VALUES (2, 'system', 'system@example.com', true, 'https://example.com/', 'Standard account for system actions', 'everywhere');

INSERT INTO users (user_id, name, email, is_verified, website, about, location)
VALUES (3, 'anonymous', 'anonymous@example.com', true, 'https://example.com/', 'Standard account for anonymous users', 'unknown');

INSERT INTO users (user_id, name, email, is_verified, website, about, location)
VALUES (4, 'nobody', 'nobody@example.com', true, 'https://example.com/', 'Standard account for unprivileged users', '?');