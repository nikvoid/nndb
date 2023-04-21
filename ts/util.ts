
/// Extension for request
class XMLHttpRequestExt extends XMLHttpRequest {
  success_cb: () => void;
  error_cb: () => void;
  
  constructor() {
    super();
    this.error_cb = () => {
      alert(this.status + " " + this.response);
    }
    this.onreadystatechange = () => {
      if (this.readyState == 4) {
        if (this.status >= 200 && this.status < 300) {
          this.success_cb();
        } else {
          this.error_cb();
        }
      } 
    }
  }
}

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
    
    let request = new XMLHttpRequestExt();
    request.success_cb = () => {
      input.value = "";
      location.reload();
    }
    
    request.open('POST', "/api/write/add_tags", true);
    request.setRequestHeader("Content-Type", "application/json");
    request.send(JSON.stringify(payload));
  }
  event.preventDefault();
  return false;
}

/// Send tags to api enpoint on click, display alert on fail
function aliasTagOnSubmit(event: Event, form: HTMLFormElement, tag: string) {
  let input = form.querySelector('.tag-field');
  if (input instanceof HTMLInputElement) {

    let payload = {
      tag_name: tag,
      query: input.value,
    };
    
    let request = new XMLHttpRequestExt();
    request.success_cb = () => {
      input.value = "";
      location.reload();
    };
    
    request.open('POST', "/api/write/alias_tag", true);
    request.setRequestHeader("Content-Type", "application/json");
    request.send(JSON.stringify(payload));
  }
  event.preventDefault();
  return false;
}

/// Delete tag from element onclick handler
function deleteTagOnClick(elem_id: number, tag_name: string) {
  let payload = {
    element_id: elem_id,
    tag_name: tag_name
  };

  let request = new XMLHttpRequestExt();
  request.success_cb = () => {
    location.replace(`/element/${elem_id}`);
  };
  
  request.open('POST', "/api/write/delete_tag", true);
  request.setRequestHeader("Content-Type", "application/json");
  request.send(JSON.stringify(payload));
  return false;
}

/// Edit tag onclick handler
function editTagOnClick(event: Event, form: HTMLFormElement, tag_name: string) {
  let type_select = form.getElementsByClassName("set-type")[0]!;
  let alt_name_text = form.getElementsByClassName("alt-name")[0]!;
  let hidden_box = form.getElementsByClassName("is-hidden")[0]!;

  if (type_select instanceof HTMLSelectElement 
    && alt_name_text instanceof HTMLInputElement
    && hidden_box instanceof HTMLInputElement) {

    let payload = {
      tag_name: tag_name,
      alt_name: alt_name_text.value.length == 0? null : alt_name_text.value,
      tag_type: type_select.selectedOptions[0].value,
      hidden: hidden_box.checked
    };

    let request = new XMLHttpRequestExt();
    request.success_cb = () => {
      location.reload();
    };
    
    request.open('POST', "/api/write/edit_tag", true);
    request.setRequestHeader("Content-Type", "application/json");
    request.send(JSON.stringify(payload));
  }
  event.preventDefault();
  return false;
}