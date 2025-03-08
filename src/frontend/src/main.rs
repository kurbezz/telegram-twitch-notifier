use stylist::style;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
struct SubscriptionProps {
    username: String,
}

#[function_component]
fn Subscription(props: &SubscriptionProps) -> Html {
    html! {
        <div>
            { props.username.clone() }
        </div>
    }
}

#[function_component]
fn Settings() -> Html {
    let subscriptions = vec!["kurbezz"];

    let header_style = style!(
        r#"
        font-size: 24px;
    "#
    )
    .expect("Failed to mount style");

    html! {
        <div>
            <h1 class={classes!(header_style.get_class_name().to_string())}>{ "Settings" }</h1>
            <div>
                {
                    subscriptions
                        .iter()
                        .map(|sub| html! { <Subscription username={*sub} /> })
                        .collect::<Html>()
                }
            </div>
        </div>
    }
}

#[function_component]
fn App() -> Html {
    html! {
        <Settings />
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
