
/// Remove constraints from element container
function fullSize(btn: HTMLElement): boolean {
  let img = document.getElementById("element")!;
  img.classList.add("page-container-full");
  btn.remove();
  return false;
}