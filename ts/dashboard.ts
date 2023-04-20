
/// Update dashboard
function updateDashboard() {
  // Update log
  let log_window = document.querySelector("#log-window");
  if (log_window instanceof HTMLPreElement) {
    let request = new XMLHttpRequest();
    request.onloadend = () => {
      let isEnd = log_window.scrollTopMax == log_window.scrollTop;
      log_window.innerText = request.responseText; 
      // Scroll to end
      if (isEnd) {
        log_window.scrollTop = log_window.scrollTopMax;  
      }
    };
    request.onerror = () => {
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

  if (
    scan instanceof HTMLElement
    && meta instanceof HTMLElement
    && group instanceof HTMLElement
    && thumbs instanceof HTMLElement
  ) {
    let requesta = new XMLHttpRequest();

    type Resp = {
      scan_files: boolean,
      update_metadata: boolean,
      group_elements: boolean,
      make_thumbnails: boolean
    };
    
    requesta.onloadend = () => {
      let data: Resp = JSON.parse(requesta.responseText);
      scan.innerText = data.scan_files;
      meta.innerText = data.update_metadata;
      group.innerText = data.group_elements;
      thumbs.innerText = data.make_thumbnails;
    };

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
  let req = new XMLHttpRequest();
  req.open("GET", "/api/write/start_import", true);
  req.send();
}

/// Update tag counts button click handler  
function updateTagCountsOnClick() {
  let req = new XMLHttpRequest();
  req.onerror = () => {
    alert(req.status + " " + req.response);
  }
  req.open("GET", "/api/write/update_tag_counts");
  req.send();
}

/// Clear element groups button onclick handler
function clearGroupsOnClick() {
  let req = new XMLHttpRequest();
  req.onerror = () => {
    alert(req.status + " " + req.response);
  }
  req.open("GET", "/api/write/clear_group_data");
  req.send();
}

/// Fix thumbnails button onclick handler
function fixThumbsOnClick() {
  let req = new XMLHttpRequest();
  req.onerror = () => {
    alert(req.status + " " + req.response);
  }
  req.open("GET", "/api/write/fix_thumbnails");
  req.send();
}

/// Retry imports button onclick handler
function retryImportsOnClick() {
  let req = new XMLHttpRequest();
  req.onerror = () => {
    alert(req.status + " " + req.response);
  }
  req.open("GET", "/api/write/retry_imports");
  req.send();
}
