use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

use super::errors::AdminError;
use super::types::Page;

type FetchFn<T> = Box<
    dyn Fn(i64) -> Pin<Box<dyn Future<Output = Result<Page<T>, AdminError>> + Send>> + Send + Sync,
>;

/// An async page iterator for admin list endpoints.
///
/// ```ignore
/// let mut pager = client.list_breakers_pager("proj_123", None);
/// while let Some(breaker) = pager.next().await? {
///     println!("{}", breaker.name);
/// }
/// ```
pub struct Pager<T> {
    fetch: FetchFn<T>,
    current_page: i64,
    buffer: VecDeque<T>,
    done: bool,
}

impl<T: Send + 'static> Pager<T> {
    pub(crate) fn new(fetch: FetchFn<T>) -> Self {
        Self {
            fetch,
            current_page: 1,
            buffer: VecDeque::new(),
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

            let page = (self.fetch)(self.current_page).await?;
            self.buffer = page.data.into();

            if self.current_page >= page.total_pages || self.buffer.is_empty() {
                self.done = true;
            }
            self.current_page += 1;
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
