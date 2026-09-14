use dioxus::prelude::*;
use dioxus_firebase as _;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx!(
        div {
            display: "flex",
            justify_content: "center",
            h3 {
                "dioxus-firebase — unofficial Firebase Auth bridge for Dioxus mobile"
            }
        }
    )
}
