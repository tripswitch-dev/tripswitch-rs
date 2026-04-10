use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

use super::errors::AdminError;

/// Result of fetching a page: items + optional next cursor.
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

type FetchFn<T> = Box<
    dyn Fn(
            Option<String>,
        ) -> Pin<Box<dyn Future<Output = Result<CursorPage<T>, AdminError>> + Send>>
        + Send
        + Sync,
>;

/// An async cursor-based page iterator for admin list endpoints.
///
/// ```ignore
/// let mut pager = client.list_events_pager("proj_123", None);
/// while let Some(event) = pager.next().await? {
///     println!("{}", event.id);
/// }
/// ```
pub struct Pager<T> {
    fetch: FetchFn<T>,
    next_cursor: Option<String>,
    buffer: VecDeque<T>,
    started: bool,
    done: bool,
}

impl<T: Send + 'static> Pager<T> {
    pub(crate) fn new(fetch: FetchFn<T>) -> Self {
        Self {
            fetch,
            next_cursor: None,
            buffer: VecDeque::new(),
            started: false,
            done: false,
        }
    }

    /// Returns the next item, fetching the next page when the buffer is exhausted.
    /// Returns `Ok(None)` when all pages have been exhausted.
    pub async fn next(&mut self) -> Result<Option<T>, AdminError> {
        loop {
            if let Some(item) = self.buffer.pop_front() {
                return Ok(Some(item));
            }

            if self.done {
                return Ok(None);
            }

            let cursor = if self.started {
                self.next_cursor.take()
            } else {
                self.started = true;
                None
            };

            // If we've already started and there's no next cursor, we're done
            if self.started && cursor.is_none() && !self.buffer.is_empty() {
                self.done = true;
                continue;
            }

            let page = (self.fetch)(cursor).await?;
            self.buffer = page.items.into();

            match page.next_cursor {
                Some(c) if !c.is_empty() => self.next_cursor = Some(c),
                _ => self.done = true,
            }

            if self.buffer.is_empty() {
                self.done = true;
            }
        }
    }

    /// Collect all remaining items into a Vec.
    pub async fn collect_all(&mut self) -> Result<Vec<T>, AdminError> {
        let mut all = Vec::new();
        while let Some(item) = self.next().await? {
            all.push(item);
        }
        Ok(all)
    }
}
