/*
 * test/session.rs
 *
 * deepwell - Database management and migrations service
 * Copyright (C) 2019 Ammon Smith
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

use super::prelude::*;
use ipnetwork::IpNetwork;

lazy_static! {
    static ref IP_ADDRESS: IpNetwork = {
        let ipv6 = "::1".parse().unwrap();
        IpNetwork::new(ipv6, 0).unwrap()
    };
}

#[test]
fn session_manager() {
    run(|server| task::block_on(session_manager_internal(server)));
}

async fn session_manager_internal(server: &Server) {
    // Setup
    let user_id = server
        .create_user("squirrelbird", "jenny@example.net", "blackmoonhowls")
        .await
        .expect("Unable to create user");

    // Invalid checks
    server
        .check_token(user_id, "invalidtoken")
        .await
        .expect_err("Invalid token was accepted");

    server
        .create_session(user_id, "letmein", *IP_ADDRESS)
        .await
        .expect_err("Invalid password was accepted");

    let session = server
        .get_session(user_id)
        .await
        .expect("Unable to get session");

    assert!(session.is_none());

    // Create session
    let token = server
        .create_session(user_id, "blackmoonhowls", *IP_ADDRESS)
        .await
        .expect("Failed to create session");

    server
        .check_token(user_id, &token)
        .await
        .expect("Token wasn't valid");

    let session = server
        .get_session(user_id)
        .await
        .expect("Unable to get session")
        .expect("No active session");

    assert_eq!(session.user_id(), user_id);
    assert_eq!(session.token(), &token);
    assert_eq!(session.ip_address(), *IP_ADDRESS);

    // End session
    let deleted = server
        .end_session(user_id)
        .await
        .expect("Unable to end session");

    assert_eq!(deleted, true);

    // Invalid checks
    let session = server
        .get_session(user_id)
        .await
        .expect("Unable to get session");

    assert!(session.is_none());

    server
        .check_token(user_id, "invalidtoken")
        .await
        .expect_err("Invalid token was accepted");

    server
        .check_token(user_id, &token)
        .await
        .expect_err("Invalid token was accepted");

    let deleted = server
        .end_session(user_id)
        .await
        .expect("Unable to end session");

    assert_eq!(deleted, false);
}
