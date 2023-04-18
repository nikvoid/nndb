
/// Update log
function updateLog() {
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
}

window.addEventListener('DOMContentLoaded', () => {
  /// Periodically update log
  updateLog();
  setInterval(() => {
    updateLog();
  }, 5000);
})

