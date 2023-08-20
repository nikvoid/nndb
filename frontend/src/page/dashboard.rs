use futures::future::join;
use gloo::timers::callback::Interval;
use web_sys::HtmlElement;
use crate::component::ProgressBar;

use super::prelude::*;

/// Dashboard page that displays backend tasks status and can send control commands
#[derive(Default)]
pub struct Dashboard {
    status: StatusResponse,
    summary: SummaryResponse,
    log_ref: NodeRef,
    /// False if log wasn't scrolled to the end
    init_scroll: bool,
}

pub enum Msg {
    Tick,
    Update(StatusResponse, String),
    Summary(SummaryResponse),
    Control(ControlRequest)
}

#[derive(PartialEq, Properties)]
pub struct Props;

impl Component for Dashboard {
    type Message = Msg;

    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        // Send initial tick immediately
        ctx.link().send_message(Msg::Tick);
        // Create ticker interval
        let link = ctx.link().clone();
        Interval::new(
            1000, 
            move || link.send_message(Msg::Tick)
        )
        .forget();
        // Ask for summary only on page reload, 
        // frequently making this request may impact DB performance 
        ctx.link().send_future(async move {
            let resp = backend_get!("/v1/summary")
                .await
                .expect("failed to fetch summary");
            Msg::Summary(resp)
        });
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
                        { "Elements in DB" }
                    </div>
                    <div class="param-value">
                        { self.summary.summary.element_count }
                    </div>
                    <div class="param-name">
                        { "Tags in DB" }
                    </div>
                    <div class="param-value">
                        { self.summary.summary.tag_count }
                    </div>
                    { for tasks }
                    <div class="section-label">
                        { "Control" }
                    </div>
                    { for controls }
                </div>
                <div class="log-window">
                    <pre ref={self.log_ref.clone()}>
                    </pre>
                </div>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
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
                        // Otherwise throw error
                        (_, Err(e))
                        | (Err(e), _) => 
                            panic!("failed to update status: {e}")
                    }
                });
                false
            },
            Msg::Update(stat, log_text) => {
                self.status = stat;
                let log: HtmlElement = self.log_ref.cast().unwrap();
                log.set_inner_text(&log_text);

                // On first update scroll log to the end
                if !self.init_scroll {
                    log.set_scroll_top(i32::MAX);
                    self.init_scroll = true;
                }
                true
            },
            Msg::Summary(summary) => {
                self.summary = summary;
                true
            }
            Msg::Control(req) => {
                ctx.link().send_future(async move {
                    let _: () = backend_post!(&req, "/v1/control")
                        .await
                        .expect("failed to send control request");
                    Msg::Tick
                });
                false
            }
        }
    }
}