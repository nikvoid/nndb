
/// Update dashboard
function updateDashboard() {
  // Update log
  let log_window = document.getElementById("log-window");
  if (log_window instanceof HTMLPreElement) {
    let request = new XMLHttpRequest();
    request.onloadend = () => {
      log_window.innerText = request.responseText; 
    };
    request.onerror = () => {
      log_window.innerText = "error fetching log"
    };
    request.open("GET", "/api/read/log", true);
    request.send();
  }  

  // Update import state
  let scan = document.getElementById("scan-files");
  let meta = document.getElementById("update-meta");
  let group = document.getElementById("group-elems");
  let thumbs = document.getElementById("make-thumbs");

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
  /// Periodically update dashboard
  updateDashboard();
  setInterval(() => {
    updateDashboard();
  }, 3000);
})

