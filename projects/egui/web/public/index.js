// Adapted from https://github.com/emilk/egui/blob/0f290b49041f2a7ce64f859f93bb16d6ca6331dd/web_demo/index.html

console.debug("Loading wasm…");
wasm_bindgen({ module_or_path: "./web_egui_bg.wasm" })
  .then(on_wasm_loaded)
  .catch(on_error);

function on_wasm_loaded() {
  console.debug("Wasm loaded. Starting app…");

  let handle = new wasm_bindgen.WebHandle();
  handle.start(document.getElementById("egui_canvas"))
    .then(on_app_started)
    .then(wasm_bindgen.initialize_audio())
    .catch(on_error);
}

function on_app_started() {
  console.debug("App started.");
  document.getElementById("center_text").innerHTML = '';
  // Make sure the canvas is focused so it can receive keyboard events right away:
  document.getElementById("egui_canvas").focus();
}

function on_error(error) {
  console.error("Failed to start: " + error);
  document.getElementById("egui_canvas").remove();
  document.getElementById("center_text").innerHTML = `
<p>
    An error occurred during loading:
</p>
<p style="font-family:Courier New">
    ${error}
</p>
<p style="font-size:14px">
    Make sure you use a modern browser with WebGL and WASM enabled.
</p>`;
}
