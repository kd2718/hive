use crate::common::{game_state::GameStateSignal, svg_pos::SvgPos};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Active(position: Position, level: usize) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);
    let game_state_signal = use_context::<RwSignal<GameStateSignal>>()
        .expect("there to be a `GameState` signal provided");

    let onclick = move |_| {
        game_state_signal.get().reset();
    };

    view! {
        <g on:click=onclick class="active">
            <g id="Active" transform=format!("{}", transform)>
                <use_
                    href="#active"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
