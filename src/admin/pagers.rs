use super::pager::Pager;
use super::types::*;
use super::AdminClient;

impl AdminClient {
    /// Create a pager that iterates over all projects.
    pub fn list_projects_pager(&self, per_page: Option<i64>) -> Pager<Project> {
        let client = self.clone();
        let pp = per_page.unwrap_or(100);
        Pager::new(Box::new(move |page| {
            let client = client.clone();
            Box::pin(async move {
                let params = ListParams {
                    page: Some(page),
                    per_page: Some(pp),
                };
                client.list_projects(Some(&params)).await
            })
        }))
    }

    /// Create a pager that iterates over all breakers in a project.
    pub fn list_breakers_pager(
        &self,
        project_id: impl Into<String>,
        per_page: Option<i64>,
    ) -> Pager<Breaker> {
        let client = self.clone();
        let pid = project_id.into();
        let pp = per_page.unwrap_or(100);
        Pager::new(Box::new(move |page| {
            let client = client.clone();
            let pid = pid.clone();
            Box::pin(async move {
                let params = ListParams {
                    page: Some(page),
                    per_page: Some(pp),
                };
                client.list_breakers(&pid, Some(&params)).await
            })
        }))
    }

    /// Create a pager that iterates over all routers in a project.
    pub fn list_routers_pager(
        &self,
        project_id: impl Into<String>,
        per_page: Option<i64>,
    ) -> Pager<Router> {
        let client = self.clone();
        let pid = project_id.into();
        let pp = per_page.unwrap_or(100);
        Pager::new(Box::new(move |page| {
            let client = client.clone();
            let pid = pid.clone();
            Box::pin(async move {
                let params = ListParams {
                    page: Some(page),
                    per_page: Some(pp),
                };
                client.list_routers(&pid, Some(&params)).await
            })
        }))
    }

    /// Create a pager that iterates over all notification channels in a project.
    pub fn list_notification_channels_pager(
        &self,
        project_id: impl Into<String>,
        per_page: Option<i64>,
    ) -> Pager<NotificationChannel> {
        let client = self.clone();
        let pid = project_id.into();
        let pp = per_page.unwrap_or(100);
        Pager::new(Box::new(move |page| {
            let client = client.clone();
            let pid = pid.clone();
            Box::pin(async move {
                let params = ListParams {
                    page: Some(page),
                    per_page: Some(pp),
                };
                client.list_notification_channels(&pid, Some(&params)).await
            })
        }))
    }

    /// Create a pager that iterates over all events in a project.
    pub fn list_events_pager(
        &self,
        project_id: impl Into<String>,
        params: Option<ListEventsParams>,
        per_page: Option<i64>,
    ) -> Pager<Event> {
        let client = self.clone();
        let pid = project_id.into();
        let pp = per_page.unwrap_or(100);
        Pager::new(Box::new(move |page| {
            let client = client.clone();
            let pid = pid.clone();
            let mut p = params.clone().unwrap_or_default();
            p.page = Some(page);
            p.per_page = Some(pp);
            Box::pin(async move { client.list_events(&pid, Some(&p)).await })
        }))
    }
}
