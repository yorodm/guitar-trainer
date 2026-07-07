mod app;
mod exercises;
mod tauri_cmd;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    sycamore::render(App);
}
