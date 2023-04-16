
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