use std::env;

use dotenv::from_filename;
use humbler_core::{humbler::Humbler, utils::openapi};
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/humbler-web.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[server]
async fn run_humbler_handler() -> Result<String, ServerFnError> {
    // dotenv::from_filename("../core/.env.test").ok();
    // let swagger_ui_url = std::env::var("SWAGGER_UI_URL").unwrap();
    // let openapi_json_url = std::env::var("OPENAPI_JSON_URL").unwrap();
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {:?}", current_dir);
    let swagger_ui_url = "http://localhost:4000/swagger-ui/index.html".to_owned();
    let openapi_json_url = "core/data/pet.json".to_owned();

    let humbler = Humbler::new(swagger_ui_url, openapi_json_url);

    println!("executing humbler");
    match humbler.run().await {
        Ok(s) => {
            println!("Humbler result: {}", s);
            Ok(s)
        }
        Err(e) => {
            println!("Humbler error: {}", e);
            Err(ServerFnError::new(format!("Error: {e}")))
        }
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let count = RwSignal::new(0);
    let on_click = move |_| *count.write() += 1;
    let async_data = Resource::new(move || {}, |_| run_humbler_handler());

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
        <Suspense fallback=move || view!{ <p>"Loading..."</p> }>
        {move || async_data.get().map(|data| view! {<p>{data}</p>})}
        </Suspense>
    }
}
