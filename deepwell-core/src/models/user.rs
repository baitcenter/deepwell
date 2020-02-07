/*
 * models/user.rs
 *
 * deepwell-core - Database management and migrations service
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

use super::prelude::*;

#[derive(Serialize, Deserialize, Queryable, Debug, Clone, PartialEq, Eq)]
pub struct User {
    user_id: UserId,
    name: String,
    email: String,
    is_verified: bool,
    is_bot: bool,
    author_page: String,
    website: String,
    about: String,
    gender: String,
    location: String,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl User {
    #[inline]
    pub fn id(&self) -> UserId {
        self.user_id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[inline]
    pub fn is_verified(&self) -> bool {
        self.is_verified
    }

    #[inline]
    pub fn is_bot(&self) -> bool {
        self.is_bot
    }

    #[inline]
    pub fn author_page(&self) -> &str {
        &self.author_page
    }

    #[inline]
    pub fn website(&self) -> &str {
        &self.website
    }

    #[inline]
    pub fn about(&self) -> &str {
        &self.about
    }

    #[inline]
    pub fn gender(&self) -> &str {
        &self.gender
    }

    #[inline]
    pub fn location(&self) -> &str {
        &self.location
    }

    #[inline]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[inline]
    pub fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none()
    }
}