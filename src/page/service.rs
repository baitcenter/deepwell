/*
 * page/service.rs
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

use super::{ChangeType, NewPage, NewRevision, NewTagChange, UpdatePage};
use crate::revision::{CommitInfo, GitHash, RevisionStore};
use crate::schema::{pages, revisions, tag_history};
use crate::service_prelude::*;
use crate::user::{User, UserId};
use crate::wiki::{Wiki, WikiId};
use async_std::fs;
use async_std::sync::RwLockReadGuard;
use either::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

mod page_id {
    make_id_type!(PageId);
}

mod revision_id {
    make_id_type!(RevisionId);
}

pub use self::page_id::PageId;
pub use self::revision_id::RevisionId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PageCommit<'a> {
    pub wiki_id: WikiId,
    pub slug: &'a str,
    pub message: &'a str,
    pub user: &'a User,
}

#[derive(Serialize, Deserialize, Queryable, Debug, Clone, PartialEq, Eq)]
pub struct Page {
    page_id: PageId,
    wiki_id: WikiId,
    slug: String,
    title: String,
    alt_title: Option<String>,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl Page {
    #[inline]
    pub fn id(&self) -> PageId {
        self.page_id
    }

    #[inline]
    pub fn wiki_id(&self) -> WikiId {
        self.wiki_id
    }

    #[inline]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[inline]
    pub fn alt_title(&self) -> Option<&str> {
        self.alt_title.as_ref().map(|s| s as _)
    }

    #[inline]
    pub fn tags(&self) -> &[String] {
        &self.tags
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
    pub fn exists(&self) -> bool {
        self.deleted_at.is_none()
    }
}

#[derive(Debug)]
struct ReadGuard<'a> {
    guard: RwLockReadGuard<'a, HashMap<WikiId, RevisionStore>>,
    wiki_id: WikiId,
}

impl ReadGuard<'_> {
    fn get(&self) -> Result<&RevisionStore> {
        match self.guard.get(&self.wiki_id) {
            Some(store) => Ok(store),
            None => {
                error!("No revision store found for wiki ID {}", self.wiki_id);
                Err(Error::WikiNotFound)
            }
        }
    }
}

pub struct PageService {
    conn: Arc<PgConnection>,
    directory: PathBuf,
    stores: RwLock<HashMap<WikiId, RevisionStore>>,
}

impl PageService {
    #[inline]
    pub fn new(conn: &Arc<PgConnection>, directory: PathBuf) -> Self {
        let conn = Arc::clone(conn);

        PageService {
            conn,
            directory,
            stores: RwLock::new(HashMap::new()),
        }
    }

    fn commit_data(
        &self,
        wiki_id: WikiId,
        page_id: PageId,
        user_id: UserId,
        change_type: ChangeType,
    ) -> String {
        format!(
            "User ID {} {} page ID {} on wiki ID {}",
            user_id,
            change_type.verb(),
            page_id,
            wiki_id,
        )
    }

    pub async fn add_store(&self, wiki: &Wiki) -> Result<()> {
        let repo = self.directory.join(wiki.slug());
        fs::create_dir(&repo).await?;

        let store = RevisionStore::new(repo, wiki.domain());
        store.initial_commit().await?;

        let mut guard = self.stores.write().await;
        guard.insert(wiki.id(), store);

        Ok(())
    }

    async fn store<'a>(&'a self, wiki_id: WikiId) -> ReadGuard<'a> {
        trace!("Getting revision store for wiki ID {}", wiki_id);

        let guard = self.stores.read().await;

        ReadGuard { guard, wiki_id }
    }

    async fn raw_commit(
        &self,
        wiki_id: WikiId,
        slug: &str,
        content: Option<&[u8]>,
        info: CommitInfo<'_>,
    ) -> Result<GitHash> {
        trace!("Committing content to repository");

        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        let hash = store.commit(slug, content, info).await?;
        Ok(hash)
    }

    pub async fn get_page_id(&self, wiki_id: WikiId, slug: &str) -> Result<Option<PageId>> {
        debug!("Getting page id in wiki ID {} for slug '{}'", wiki_id, slug);

        let wiki_id: i64 = wiki_id.into();
        let page_id = pages::table
            .filter(pages::dsl::wiki_id.eq(wiki_id))
            .filter(pages::dsl::slug.eq(slug))
            .select(pages::dsl::page_id)
            .first::<PageId>(&*self.conn)
            .optional()?;

        Ok(page_id)
    }

    pub async fn create(
        &self,
        commit: PageCommit<'_>,
        content: &[u8],
        title: &str,
        alt_title: Option<&str>,
    ) -> Result<(PageId, RevisionId)> {
        info!("Creating page {:?} with title '{}'", commit, title);

        let PageCommit {
            wiki_id,
            slug,
            message,
            user,
        } = commit;

        self.transaction(async {
            let model = NewPage {
                wiki_id: wiki_id.into(),
                slug,
                title,
                alt_title,
            };

            trace!("Checking for existing page");
            if self.get_page_id(wiki_id, slug).await?.is_some() {
                return Err(Error::PageExists);
            }

            trace!("Inserting {:?} into pages table", &model);
            let page_id = diesel::insert_into(pages::table)
                .values(&model)
                .returning(pages::dsl::page_id)
                .get_result::<PageId>(&*self.conn)?;

            let user_id = user.id();
            let change_type = ChangeType::Create;

            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            let hash = self.raw_commit(wiki_id, slug, Some(content), info).await?;
            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            Ok((page_id, revision_id))
        })
        .await
    }

    pub async fn commit(
        &self,
        commit: PageCommit<'_>,
        content: Option<&[u8]>,
        title: Option<&str>,
        alt_title: Option<Nullable<&str>>,
    ) -> Result<RevisionId> {
        info!("Committing change to page {:?}", commit);

        let PageCommit {
            wiki_id,
            slug,
            message,
            user,
        } = commit;

        self.transaction(async {
            let model = UpdatePage {
                slug: None,
                title,
                alt_title,
            };

            let page_id = self
                .get_page_id(wiki_id, slug)
                .await?
                .ok_or(Error::PageNotFound)?;

            trace!("Updating {:?} in pages table", &model);
            {
                use self::pages::dsl;

                let id: i64 = page_id.into();
                diesel::update(dsl::pages.filter(dsl::page_id.eq(id)))
                    .set(&model)
                    .execute(&*self.conn)?;
            }

            let user_id = user.id();
            let change_type = ChangeType::Modify;

            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            let hash = self.raw_commit(wiki_id, slug, content, info).await?;
            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            Ok(revision_id)
        })
        .await
    }

    pub async fn rename(
        &self,
        wiki_id: WikiId,
        old_slug: &str,
        new_slug: &str,
        message: &str,
        user: &User,
    ) -> Result<RevisionId> {
        info!(
            "Renaming page '{}' -> '{}' in wiki ID {}",
            old_slug, new_slug, wiki_id
        );

        self.transaction(async {
            let model = UpdatePage {
                slug: Some(new_slug),
                title: None,
                alt_title: None,
            };

            let page_id = self
                .get_page_id(wiki_id, old_slug)
                .await?
                .ok_or(Error::PageNotFound)?;

            trace!("Updating {:?} in pages table", &model);
            {
                use self::pages::dsl;

                let id: i64 = page_id.into();
                diesel::update(dsl::pages.filter(dsl::page_id.eq(id)))
                    .set(&model)
                    .execute(&*self.conn)?;
            }

            let user_id = user.id();
            let change_type = ChangeType::Rename;

            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            trace!("Committing rename to repository");
            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let hash = store.rename(old_slug, new_slug, info).await?;

            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            Ok(revision_id)
        })
        .await
    }

    pub async fn remove(&self, commit: PageCommit<'_>) -> Result<RevisionId> {
        info!("Removing page {:?}", commit);

        let PageCommit {
            wiki_id,
            slug,
            message,
            user,
        } = commit;

        self.transaction(async {
            use diesel::dsl::now;

            let page_id = self
                .get_page_id(wiki_id, slug)
                .await?
                .ok_or(Error::PageNotFound)?;

            trace!("Marking page as deleted in table");
            {
                use self::pages::dsl;

                let id: i64 = page_id.into();
                diesel::update(dsl::pages.filter(dsl::page_id.eq(id)))
                    .set(pages::dsl::deleted_at.eq(now))
                    .execute(&*self.conn)?;
            }

            let user_id = user.id();
            let change_type = ChangeType::Delete;

            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            trace!("Committing removal to repository");
            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let result = store.remove(slug, info).await?;
            let hash = match result {
                Some(hash) => hash,
                None => return Err(Error::PageNotFound),
            };

            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            Ok(revision_id)
        })
        .await
    }

    pub async fn restore(
        &self,
        commit: PageCommit<'_>,
        page_id: Option<PageId>,
    ) -> Result<RevisionId> {
        info!("Restoring page {:?}", commit);

        let PageCommit {
            wiki_id,
            slug,
            message,
            user,
        } = commit;

        self.transaction(async {
            if self.check_page(wiki_id, slug).await? {
                return Err(Error::PageExists);
            }

            let user_id = user.id();
            let page_id = match page_id {
                Some(id) => id,
                None => {
                    trace!("Finding last page ID for slug");

                    let wiki_id: i64 = wiki_id.into();
                    let page_id = pages::table
                        .filter(pages::wiki_id.eq(wiki_id))
                        .filter(pages::slug.eq(slug))
                        .filter(pages::deleted_at.is_not_null())
                        .order_by(pages::created_at.desc())
                        .select(pages::dsl::page_id)
                        .first::<PageId>(&*self.conn)
                        .optional()?;

                    // If none, found nothing to restore
                    match page_id {
                        Some(id) => id,
                        None => return Err(Error::PageNotFound),
                    }
                }
            };

            trace!("Finding last extant commit for page");

            let (wiki_id, page_id, old_slug, hash) = {
                // Get wiki and slug
                let id: i64 = page_id.into();
                let result = pages::table
                    .find(id)
                    .select((pages::dsl::wiki_id, pages::dsl::slug))
                    .first::<(WikiId, String)>(&*self.conn)
                    .optional()?;

                let (wiki_id, old_slug) = match result {
                    Some(data) => data,
                    None => {
                        error!("Page ID {} found with no last revision", page_id);
                        return Err(Error::PageNotFound);
                    }
                };

                // Get last non-deletion commit
                let change_type: &str = ChangeType::Delete.into();
                let raw_hash = revisions::table
                    .filter(revisions::dsl::page_id.eq(id))
                    .filter(revisions::dsl::change_type.ne(change_type))
                    .order_by(revisions::dsl::revision_id.desc())
                    .select(revisions::dsl::git_commit)
                    .first::<String>(&*self.conn)?;

                let hash = GitHash::from_checked(raw_hash);

                (wiki_id, page_id, old_slug, hash)
            };

            let change_type = ChangeType::Restore;
            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            trace!("Committing page restoration to repository");

            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let hash = store.restore(slug, &old_slug, &hash, info).await?;

            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            Ok(revision_id)
        })
        .await
    }

    pub async fn tags(&self, commit: PageCommit<'_>, tags: &mut [&str]) -> Result<RevisionId> {
        info!("Modifying tags for {:?}: {:?}", commit, tags);

        let PageCommit {
            wiki_id,
            slug,
            message,
            user,
        } = commit;

        self.transaction(async {
            let page_id = self
                .get_page_id(wiki_id, slug)
                .await?
                .ok_or(Error::PageNotFound)?;

            trace!("Getting tag difference");
            let current_tags = {
                let id: i64 = page_id.into();
                pages::table
                    .find(id)
                    .select(pages::dsl::tags)
                    .first::<Vec<String>>(&*self.conn)?
            };

            let (added_tags, removed_tags) = tag_diff(&current_tags, tags);

            // Create commit
            let user_id = user.id();
            let change_type = ChangeType::Tags;

            let commit = self.commit_data(wiki_id, page_id, user_id, change_type);
            let info = CommitInfo {
                username: user.name(),
                message: &commit,
            };

            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let hash = store.empty_commit(info).await?;

            let model = NewRevision {
                page_id: page_id.into(),
                user_id: user_id.into(),
                message,
                git_commit: hash.as_ref(),
                change_type: change_type.into(),
            };

            trace!("Inserting revision {:?} into revisions table", &model);
            let revision_id = diesel::insert_into(revisions::table)
                .values(&model)
                .returning(revisions::dsl::revision_id)
                .get_result::<RevisionId>(&*self.conn)?;

            let model = NewTagChange {
                revision_id: revision_id.into(),
                added_tags: &added_tags,
                removed_tags: &removed_tags,
            };

            trace!("Inserting tag change {:?} into tag history table", &model);
            diesel::insert_into(tag_history::table)
                .values(&model)
                .execute(&*self.conn)?;

            tags.sort();

            trace!("Updating tags for page");
            diesel::update(pages::table)
                .set(pages::dsl::tags.eq(&*tags))
                .execute(&*self.conn)?;

            Ok(revision_id)
        })
        .await
    }

    pub async fn check_page(&self, wiki_id: WikiId, slug: &str) -> Result<bool> {
        info!(
            "Checking if page for exists in wiki ID {}, slug {} exists",
            wiki_id, slug,
        );

        let result = pages::table
            .filter(pages::slug.eq(slug))
            .filter(pages::deleted_at.is_null())
            .select(pages::page_id)
            .first::<PageId>(&*self.conn)
            .optional()?;

        Ok(result.is_some())
    }

    pub async fn get_page(&self, wiki_id: WikiId, slug: &str) -> Result<Option<Page>> {
        info!("Getting page for wiki ID {}, slug {}", wiki_id, slug);

        let wiki_id: i64 = wiki_id.into();
        let page = pages::table
            .filter(pages::wiki_id.eq(wiki_id))
            .filter(pages::slug.eq(slug))
            .filter(pages::deleted_at.is_null())
            .first::<Page>(&*self.conn)
            .optional()?;

        Ok(page)
    }

    pub async fn get_page_by_id(&self, page_id: PageId) -> Result<Option<Page>> {
        info!("Getting page for page ID {}", page_id);

        let id: i64 = page_id.into();
        let page = pages::table
            .find(id)
            .first::<Page>(&*self.conn)
            .optional()?;

        Ok(page)
    }

    pub async fn get_page_contents(
        &self,
        wiki_id: WikiId,
        slug: &str,
    ) -> Result<Option<Box<[u8]>>> {
        info!("Getting contents for wiki ID {}, slug {}", wiki_id, slug);

        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        let contents = store.get_page(slug).await?;

        Ok(contents)
    }

    async fn get_last_hash(&self, page_id: PageId) -> Result<Option<(WikiId, String, GitHash)>> {
        debug!("Getting last commit for page ID {}", page_id);

        let id: i64 = page_id.into();
        let result = pages::table
            .find(id)
            .select((pages::dsl::wiki_id, pages::dsl::slug))
            .first::<(WikiId, String)>(&*self.conn)
            .optional()?;

        let (wiki_id, slug) = match result {
            Some((wiki_id, slug)) => (wiki_id, slug),
            None => return Ok(None),
        };

        let raw_hash = revisions::table
            .filter(revisions::dsl::page_id.eq(id))
            .order_by(revisions::dsl::revision_id.desc())
            .select(revisions::dsl::git_commit)
            .first::<String>(&*self.conn)?;

        let hash = GitHash::from_checked(raw_hash);

        Ok(Some((wiki_id, slug, hash)))
    }

    pub async fn get_page_contents_by_id(&self, page_id: PageId) -> Result<Option<Box<[u8]>>> {
        info!("Getting contents for page ID {}", page_id);

        self.transaction(async {
            let last_hash = self.get_last_hash(page_id).await?;
            let (wiki_id, slug, hash) = match last_hash {
                Some(result) => result,
                None => return Ok(None),
            };

            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let contents = store.get_page_version(&slug, &hash).await?;
            Ok(contents)
        })
        .await
    }

    pub async fn get_blame(&self, wiki_id: WikiId, slug: &str) -> Result<Option<Blame>> {
        info!("Getting blame for wiki ID {}, slug {}", wiki_id, slug);

        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        let blame = store.get_blame(slug, None).await?;
        Ok(blame)
    }

    pub async fn get_blame_by_id(&self, page_id: PageId) -> Result<Option<Blame>> {
        info!("Getting blame for page ID {}", page_id);

        self.transaction(async {
            let last_hash = self.get_last_hash(page_id).await?;
            let (wiki_id, slug, hash) = match last_hash {
                Some(result) => result,
                None => return Ok(None),
            };

            let guard = self.store(wiki_id).await;
            let store = guard.get()?;
            let blame = store.get_blame(&slug, Some(&hash)).await?;
            Ok(blame)
        })
        .await
    }

    async fn revision_commit(&self, revision_id: RevisionId) -> Result<GitHash> {
        debug!("Getting commit hash for revision ID {}", revision_id);

        let id: i64 = revision_id.into();
        let result = revisions::table
            .find(id)
            .select(revisions::dsl::git_commit)
            .first::<String>(&*self.conn)
            .optional()?;

        match result {
            Some(hash) => Ok(GitHash::from_checked(hash)),
            None => Err(Error::RevisionNotFound),
        }
    }

    async fn commit_hash<'a>(
        &self,
        revision: Either<RevisionId, &'a GitHash>,
    ) -> Result<Cow<'a, GitHash>> {
        match revision {
            Left(id) => {
                let hash = self.revision_commit(id).await?;
                Ok(Cow::Owned(hash))
            }
            Right(hash) => Ok(Cow::Borrowed(hash)),
        }
    }

    pub async fn get_page_version(
        &self,
        wiki_id: WikiId,
        slug: &str,
        revision: Either<RevisionId, &GitHash>,
    ) -> Result<Option<Box<[u8]>>> {
        info!(
            "Getting specific page version for wiki ID {}, slug {}",
            wiki_id, slug
        );

        let hash = self.commit_hash(revision).await?;

        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        let contents = store.get_page_version(slug, &hash).await?;
        Ok(contents)
    }

    pub async fn get_diff(
        &self,
        wiki_id: WikiId,
        slug: &str,
        first: Either<RevisionId, &GitHash>,
        second: Either<RevisionId, &GitHash>,
    ) -> Result<Box<[u8]>> {
        info!("Getting diff for wiki ID {}, slug {}", wiki_id, slug);

        // Get both commits
        let (first, second) = join!(self.commit_hash(first), self.commit_hash(second),);

        let (first, second) = (first?, second?);

        // Actually get the diff from the RevisionStore
        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        let diff = store.get_diff(slug, &first, &second).await?;
        Ok(diff)
    }

    pub async fn edit_revision(&self, revision_id: RevisionId, message: &str) -> Result<()> {
        use self::revisions::dsl;

        info!("Editing revision message for ID {}", revision_id);

        let id: i64 = revision_id.into();
        diesel::update(dsl::revisions.filter(dsl::revision_id.eq(id)))
            .set(dsl::message.eq(message))
            .execute(&*self.conn)?;

        Ok(())
    }

    pub async fn set_domain(&self, wiki_id: WikiId, new_domain: &str) -> Result<()> {
        let guard = self.store(wiki_id).await;
        let store = guard.get()?;
        store.set_domain(new_domain).await;
        Ok(())
    }
}

impl_async_transaction!(PageService);

impl Debug for PageService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PageService")
            .field("conn", &"PgConnection { .. }")
            .field("stores", &self.stores)
            .finish()
    }
}

fn tag_diff<'a>(
    current_tags: &'a [String],
    new_tags: &'_ [&'a str],
) -> (Vec<&'a str>, Vec<&'a str>) {
    macro_rules! difference {
        ($first:expr, $second:expr) => {{
            let mut diff: Vec<_> = $first.difference(&$second).copied().collect();
            diff.sort();
            diff
        }};
    }

    let new_tags = new_tags.iter().copied().collect::<HashSet<_>>();
    let old_tags = current_tags
        .iter()
        .map(|s| s.as_str())
        .collect::<HashSet<_>>();

    let added_tags = difference!(new_tags, old_tags);
    let removed_tags = difference!(old_tags, new_tags);

    (added_tags, removed_tags)
}
