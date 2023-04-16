//! This script helps to save query between pages
//! It works on assumption that only one page can be active at time
//! And saves/loads query on each page transition

/// Count of transitions to page
let showCount = 0;

const QUERY_KEY = 'query';

/// Hook that will save query each time it changed 
function searchBoxHook(box: HTMLInputElement) {
  localStorage.setItem(QUERY_KEY, box.value);
}

/// Capture current query
window.addEventListener('pageshow', () => {
  let box = document.getElementById("search-box")!;
  if (box instanceof HTMLInputElement) {
    if (showCount > 0 || box.value != "") {
      localStorage.setItem(QUERY_KEY, box.value);
    }
  }
  showCount += 1;
})

/// Set query to search field if it was stored in local storage
 window.addEventListener('DOMContentLoaded', () => {
   let query = localStorage.getItem(QUERY_KEY);
   if (query !== null) {
     let box = document.getElementById("search-box")!;
     if (box instanceof HTMLInputElement && box.value == "") {
       box.value = query;
     }
   }
 })