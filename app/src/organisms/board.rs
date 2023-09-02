use leptos::*;
use crate::atoms::svgs::Svgs;

#[component]
pub fn Board(cx: Scope) -> impl IntoView {
    view! { cx,
        <svg viewBox="0 0 100 100" width="100vw" height="90vh" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            //<Piece/>
            //<LastMove/>
        </svg>
    }
}