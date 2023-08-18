use futures::future::join;
use gloo::timers::callback::Timeout;
use crate::component::ProgressBar;

use super::prelude::*;

/// Status of a link between frontend and backend
#[derive(Default)]
pub enum LinkStatus {
    #[default]
    Pending,
    Ok,
    Error(String)
}

/// Dashboard page that displays backend tasks status and can send control commands
#[derive(Default)]
pub struct Dashboard {
    log_data: String,
    status: StatusResponse,
    link_status: LinkStatus,
}

pub enum Msg {
    Reload,
    Update(StatusResponse, String),
    LinkStatus(LinkStatus),
    Control(ControlRequest)
}

#[derive(PartialEq, Properties)]
pub struct Props;

impl Component for Dashboard {
    type Message = Msg;

    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Reload);
        Self::default()
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let tasks = [
            ("File scan", &self.status.scan_files),
            ("Metadata update", &self.status.update_metadata),
            ("Group elements", &self.status.group_elements),
            ("Make thumbnails", &self.status.make_thumbnails),
            ("Wiki fetch", &self.status.wiki_fetch)
        ]
        .into_iter()
        .map(|(name, stat)| html! {
            <>
                <div class="param-name">
                    { name }
                </div>    
                <div class="param-value">
                    if let TaskStatus::Running { done, actions } = stat {
                        { "running: " }{ done }{ "/" }{ actions }
                    } else {
                        { "sleeping" }
                    }
                </div>
                // If running, show progress bar
                if let TaskStatus::Running { done, actions } = stat {
                    <div class="section-data">
                        <ProgressBar progress={*done as f32 / *actions as f32} />
                    </div>
                }
            </>
        });

        let link_stat = match self.link_status {
            LinkStatus::Pending => "pending",
            LinkStatus::Ok => "ok",
            LinkStatus::Error(_) => "error",
        };

        let controls = [
            (ControlRequest::StartImport, "Start import"),
            (ControlRequest::UpdateTagCount, "Update tag counts"),
            (ControlRequest::ClearGroupData, "Clear group data"),
            (ControlRequest::FixThumbnails, "Fix thumbnails"),
            (ControlRequest::RetryImports, "Retry imports"),
            (ControlRequest::FetchWikis, "Fetch wikis"),
        ]
        .into_iter()
        .map(|(req, label)| {
            let onclick = ctx.link().callback(move |_| Msg::Control(req));
            html! {
                <div class="button section-data" {onclick}>
                    { label }
                </div>
            }
        });
        
        html! {
            <div class="dashboard-page">
                <div class="control-panel">
                    <div class="section-label">
                        { "Status" }
                    </div>
                    <div class="param-name">
                        { "Last update" }
                    </div>
                    <div class="param-value">
                        { link_stat }
                    </div>
                    { for tasks }
                    <div class="section-label">
                        { "Control" }
                    </div>
                    { for controls }
                </div>
                <div class="log-window">
                    <pre>
                        { &self.log_data }
                    </pre>
                </div>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => {
                ctx.link().send_future(async move {
                    let req = LogRequest {
                        // TODO: Adjustable log size
                        read_size: 50000
                    };

                    match join(
                        backend_get!("/v1/status"), 
                        backend_post!(&req, "/v1/log")
                    ).await {
                        // If both requests suceeded, send update
                        (Ok(stat), Ok(LogResponse { data })) => Msg::Update(stat, data),
                        // Otherwise send error
                        (_, Err(e))
                        | (Err(e), _) => 
                            Msg::LinkStatus(LinkStatus::Error(e.to_string())),
                    }
                });
                false
            },
            Msg::Update(stat, log) => {
                self.status = stat;
                self.log_data = log;
                ctx.link().send_message(Msg::LinkStatus(LinkStatus::Ok));
                true
            },
            Msg::LinkStatus(stat) => {
                match stat {
                    LinkStatus::Pending
                    | LinkStatus::Ok =>  {
                        // If previous connect suceeded, update
                        let link = ctx.link().clone();
                        Timeout::new(1000, move || link.send_message(Msg::Reload))
                            .forget();
                    },
                    LinkStatus::Error(e) => todo!("Make global error slot: {e}"),
                }
                self.link_status = stat;
                true
            },
            Msg::Control(req) => {
                ctx.link().send_future(async move {
                    match backend_post!(&req, "/v1/control").await {
                        Ok(()) => Msg::Reload,
                        Err(e) => Msg::LinkStatus(LinkStatus::Error(e.to_string()))
                    }
                });
                false
            }
        }
    }
}