
/// Remove constraints from element container
function fullSize(btn: HTMLElement): boolean {
  let img = document.getElementById("element")!;
  img.classList.add("page-container-full");
  btn.remove();
  return false;
}

/// Get term under cursor
function getSelected(box: HTMLInputElement): [string, number, number] | null {
  const cursor = box.selectionStart!;

  let start = box.value.lastIndexOf(" ", cursor - 1);
  let end = box.value.indexOf(" ", cursor);

  if (start == -1) {
    start = 0;
  } else {
    start += 1;
  }
  if (end == -1) {
    end = box.value.length;
  }
  
  return [box.value.substring(start, end), start, end]
}

/// Query db for tag completions
function getCompletions(textbox: HTMLInputElement, selectorId: string) {
  let selector = document.getElementById(selectorId)!;

  let [term, start, end] = getSelected(textbox)!;

  if (term.length == 0) {
    selector.hidden = true;
    return;
  }

  let request = new XMLHttpRequest();
  request.onreadystatechange = () => {
    if (request.readyState == 4 && request.status == 200) {
      interface Tag {
        name: string,
        alt_name: string | null,
        tag_type: string,
        count: number,
      }

      const json: Array<Tag> = JSON.parse(request.responseText);

      const list = json.map(compl => {
        const alt_name = compl.alt_name === null? "" : compl.alt_name;
        return `
          <li onclick="onCompletionClick(
            '${textbox.id}', 
            '${selectorId}', 
            '${compl.name}', 
            ${start}, 
            ${end}
          );">
            <div class="cand-name">${compl.name}</div>
            <div class="cand-info">${alt_name} ${compl.tag_type.toLowerCase()} ${compl.count}</div>
          </li>`
      }).join("");

      selector.innerHTML = `<ul>${list}</ul>`;
      selector.hidden = false;
    }
  }

  request.open("GET", `/api/autocomplete?input=${term}`, true);
  request.send();
}

/// onclick handler for tag completions
function onCompletionClick(
  textboxId: string, 
  selectorId: string,
  completion: string,
  start: number,
  end: number,
) {
  let textbox = document.getElementById(textboxId)!;
  let selector = document.getElementById(selectorId)!;

  if (textbox instanceof HTMLInputElement) {
    const left = textbox.value.slice(0, start);
    const right = textbox.value.slice(end);
   
    textbox.value = left + completion + right;
    textbox.focus();
    selector.innerHTML = "";
    selector.hidden = true;  
  }
}