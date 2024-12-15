const { invoke } = window.__TAURI__.core;
const tauri = window.__TAURI__

let inputField;
let outputEl;

async function greet_rust() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  outputEl.textContent = await invoke("greet_rust", { name: inputField.value });
}
async function greet_python() {
  outputEl.textContent = await tauri.python.call.greet_python( inputField.value );
}

window.addEventListener("DOMContentLoaded", () => {
  tauri.python.registerFunction("greet_python", 1);
  inputField = document.querySelector("#input-field");
  outputEl = document.querySelector("#output-element");
  document.querySelector("#callback-form").addEventListener("submit", (e) => {
    e.preventDefault();
    switch (e.submitter.value) {
      case "submit_rust":
        greet_rust();
        break;
      case "submit_python":
        greet_python();
        break;
    }
  });
});
