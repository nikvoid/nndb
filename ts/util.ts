
/// Remove constraints from element container
function fullSize(btn: HTMLElement): boolean {
  let img = document.getElementById("element")!;
  img.classList.add("page-container-full");
  btn.remove();
  return false;
}

interface ErrorState {
  was_error: boolean | undefined
};

/// Element list unit error handler
function elementListOnError(img: HTMLImageElement & ErrorState, full: string) {
  // Try load full image
  if (!img.was_error) {
    img.src = full;
    img.was_error = true;
  } else {
    // Don't retry on fail
    img.src = "";
    img.onerror = () => {}
  }
}

/// Send tags to api enpoint on click, display alert on fail
function addTagOnSubmit(event: Event, form: HTMLFormElement, elementId: number) {
  let input = form.getElementsByClassName('tag-field')[0]!;
  if (input instanceof HTMLInputElement) {

    let payload = {
      element_id: elementId,
      tags: input.value,
    };
    
    let request = new XMLHttpRequest();
    request.onreadystatechange = () => {
      if (request.readyState == 4) {
        if (request.status == 200) {
          input.value = "";
          location.reload();
        } else {
          alert(request.status + " " + request.response);
        }
      }
    }
    
    request.open('POST', "/api/write/add_tags", true);
    request.setRequestHeader("Content-Type", "application/json");
    request.send(JSON.stringify(payload));
  }
  event.preventDefault();
  return false;
}