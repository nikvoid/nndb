
/// Get term under cursor
function getSelected(box: HTMLInputElement): [string | null, string, number, number] | null {
  const PREFIXES = ["!"];
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

  let prefix;
  if (PREFIXES.includes(box.value[start])) {
    prefix = box.value[start];
    start += 1;
  } else {
    prefix = null;
  }
  const term = box.value.substring(start, end);
  
  return [prefix, term, start, end]
}

/// Query db for tag completions
function getCompletions(textbox: HTMLInputElement, selectorId: string) {
  let selector = document.getElementById(selectorId)!;

  let [pref, term, start, end] = getSelected(textbox)!;

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

      if (json.length == 0) {
        selector.hidden = true;
        return;
      }

      const list = json.map(compl => {
        const alt_name = compl.alt_name === null? "" : compl.alt_name;
        const prefix = pref === null? "" : pref;
        return `
          <li onclick="onCompletionClick(
            '${textbox.id}', 
            '${selectorId}', 
            '${compl.name}', 
            ${start}, 
            ${end}
          );">
            <div class="cand-name">${prefix}${compl.name}</div>
            <div class="cand-info">${alt_name} ${compl.tag_type.toLowerCase()} ${compl.count}</div>
          </li>`
      }).join("");

      selector.innerHTML = `<ul>${list}</ul>`;
      selector.hidden = false;
    }
  }

  request.open("GET", `/api/read/autocomplete?input=${term}`, true);
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