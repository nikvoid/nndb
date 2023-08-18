use gloo::timers::callback::Timeout;

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
    UpdateStatus(StatusResponse),
    UpdateLog(String),
    LinkStatus(LinkStatus),
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
                        // TODO: Progress-bar
                    } else {
                        { "sleeping" }
                    }
                </div>
            </>
        });

        let link_stat = match self.link_status {
            LinkStatus::Pending => "pending",
            LinkStatus::Ok => "ok",
            LinkStatus::Error(_) => "error",
        };
        
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
                    // TODO: Control buttons
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
                    match backend_get!("/v1/status").await {
                        Ok(s) => Msg::UpdateStatus(s),
                        Err(e) => Msg::LinkStatus(LinkStatus::Error(e.to_string())) 
                    }
                });
                ctx.link().send_future(async move {
                    let req = LogRequest {
                        // TODO: Adjustable log size
                        read_size: 50000
                    };
                    match backend_post!(&req, "/v1/log").await {
                        Ok(resp) => {
                            // Type can't be written in pattern
                            let r: LogResponse = resp;
                            Msg::UpdateLog(r.data)
                        }
                        Err(e) => Msg::LinkStatus(LinkStatus::Error(e.to_string()))
                    }
                });
                false
            },
            Msg::UpdateStatus(stat) => {
                self.status = stat;
                self.link_status = LinkStatus::Ok;
                true
            },
            Msg::UpdateLog(log) => {
                self.log_data = log;
                self.link_status = LinkStatus::Ok;
                true
            },
            Msg::LinkStatus(stat) => {
                match stat {
                    LinkStatus::Pending
                    | LinkStatus::Ok =>  {
                        // If previous connect suceeded, update
                        let link = ctx.link().clone();
                        Timeout::new(5000, move || link.send_message(Msg::Reload))
                            .forget();
                    },
                    LinkStatus::Error(_) => todo!("Make global error slot"),
                }
                self.link_status = stat;
                true
            },
        }
    }
}