
/// Update dashboard
function updateDashboard() {
  // Update log
  let log_window = document.querySelector("#log-window");
  if (log_window instanceof HTMLPreElement) {
    let request = new XMLHttpRequestExt();
    request.success_cb = () => {
      let isEnd = log_window.scrollTopMax == log_window.scrollTop;
      log_window.innerText = request.responseText; 
      // Scroll to end
      if (isEnd) {
        log_window.scrollTop = log_window.scrollTopMax;  
      }
    };
    request.error_cb = () => {
      log_window.innerText = "error fetching log"
    };
    request.open("GET", "/api/read/log", true);
    request.send();
  }  

  // Update import state
  let scan = document.querySelector("#scan-files");
  let meta = document.querySelector("#update-meta");
  let group = document.querySelector("#group-elems");
  let thumbs = document.querySelector("#make-thumbs");
  let wikis = document.querySelector("#fetch-wikis");

  if (
    scan instanceof HTMLElement
    && meta instanceof HTMLElement
    && group instanceof HTMLElement
    && thumbs instanceof HTMLElement
    && wikis instanceof HTMLElement
  ) {
    let requesta = new XMLHttpRequestExt();

    type Resp = {
      scan_files: [boolean, number, number],
      update_metadata: boolean,
      group_elements: boolean,
      make_thumbnails: boolean,
      wiki_fetch: [boolean, number]
    };
    
    requesta.success_cb = () => {
      let data: Resp = JSON.parse(requesta.responseText);
      scan.innerText = `${data.scan_files[0]} : ${data.scan_files[2]} / ${data.scan_files[1]}`;
      meta.innerText = data.update_metadata;
      group.innerText = data.group_elements;
      thumbs.innerText = data.make_thumbnails;
      wikis.innerText = data.wiki_fetch;
    };
    requesta.error_cb = () => {};

    requesta.open("GET", "/api/read/import", true);
    requesta.send();
  }
}

window.addEventListener('DOMContentLoaded', () => {
  // Update dashboard
  updateDashboard();
  
  // Periodically update dashboard
  setInterval(() => {
    updateDashboard();
  }, 3000);
})

/// Manual Import button click handler  
function importBtnOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/start_import", true);
  req.send();
}

/// Update tag counts button click handler  
function updateTagCountsOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/update_tag_counts");
  req.send();
}

/// Clear element groups button onclick handler
function clearGroupsOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/clear_group_data");
  req.send();
}

/// Fix thumbnails button onclick handler
function fixThumbsOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/fix_thumbnails");
  req.send();
}

/// Retry imports button onclick handler
function retryImportsOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/retry_imports");
  req.send();
}

/// Fetch danbooru wikis onclick handler
function fetchWikisOnClick() {
  let req = new XMLHttpRequestExt();
  req.open("GET", "/api/write/fetch_wikis");
  req.send();
}
