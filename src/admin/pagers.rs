use super::pager::Pager;
use super::types::*;
use super::AdminClient;

macro_rules! project_pager {
    ($method:ident, $list:ident, $T:ty) => {
        pub fn $method(&self, project_id: impl Into<String>, per_page: Option<i64>) -> Pager<$T> {
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
                    client.$list(&pid, Some(&params)).await
                })
            }))
        }
    };
}

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

    project_pager!(list_breakers_pager, list_breakers, Breaker);
    project_pager!(list_routers_pager, list_routers, Router);
    project_pager!(
        list_notification_channels_pager,
        list_notification_channels,
        NotificationChannel
    );

    /// Create a pager that iterates over all project keys in a project.
    pub fn list_project_keys_pager(&self, project_id: impl Into<String>) -> Pager<ProjectKey> {
        let client = self.clone();
        let pid = project_id.into();
        Pager::new(Box::new(move |_page| {
            let client = client.clone();
            let pid = pid.clone();
            Box::pin(async move { client.list_project_keys(&pid).await })
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
