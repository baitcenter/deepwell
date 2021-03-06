/*
 * test/factory.rs
 *
 * deepwell - Database management and migrations service
 * Copyright (C) 2019-2020 Ammon Smith
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use crate::prelude::*;
use crate::utils::rand_alphanum;

// User
pub async fn create_user_full(server: &Server, password: &str) -> (UserId, String, String) {
    let username = {
        let mut chars = rand_alphanum(16);
        chars.insert_str(0, "user_");
        chars
    };

    let email = format!("{}@example.com", username);

    println!("Creating test user '{}'", username);
    let id = server
        .create_user(&username, &email, password)
        .await
        .expect("Unable to create user");

    (id, username, email)
}

#[inline]
pub async fn create_user(server: &Server) -> UserId {
    create_user_full(server, "defaultpasswordhere2").await.0
}

// Wiki
pub async fn create_wiki_full(server: &Server) -> (WikiId, String) {
    let slug = {
        let mut chars = rand_alphanum(8);
        chars.make_ascii_lowercase();
        chars.insert_str(0, "wiki-");
        chars
    };

    let domain = format!("{}.example.com", slug);

    println!("Creating test wiki '{}'", slug);
    let id = server
        .create_wiki(&slug, &slug, &domain)
        .await
        .expect("Unable to create wiki");

    (id, slug)
}

#[inline]
pub async fn create_wiki(server: &Server) -> WikiId {
    create_wiki_full(server).await.0
}
