/*
 * error.rs
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

use diesel::result::Error as DieselError;
use serde_json as json;
use std::io;
use subprocess::PopenError;

#[must_use = "should handle errors"]
#[derive(Debug, Error)]
pub enum Error {
    #[error("uncommon error: {0}")]
    StaticMsg(&'static str),

    #[error("general I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("database error: {0}")]
    Database(#[from] DieselError),

    #[error("error running subprocess: {0}")]
    Subprocess(#[from] PopenError),

    #[error("error serializing JSON: {0}")]
    JsonSerialize(#[from] json::Error),

    #[error("command failed: {0}")]
    CommandFailed(String),
}
