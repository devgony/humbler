use std::env;

use humbler_core::humbler::{ApiInfo, Humbler};
use leptos::prelude::*;
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
async fn run_humbler_handler() -> Result<Vec<ApiInfo>, ServerFnError> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let swagger_ui_url = "http://localhost:4000/swagger-ui/index.html".to_owned();
    let openapi_json_url = "core/data/pet.json".to_owned();

    let humbler = Humbler::new(swagger_ui_url, openapi_json_url);

    humbler
        .run()
        .await
        .map_err(|e| ServerFnError::new(format!("Error: {e}")))
        .map(|h| {
            println!("Humbler result: {:#?}", h);
            h.api_infos
        })
}

static HEADERS: [&str; 6] = [
    "Path",
    "Method",
    "Parameters",
    "Request Body",
    "Response",
    "Swagger URL",
];

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let async_data = Resource::new(move || {}, |_| run_humbler_handler());

    view! {
        <h1 class="text-red-300">"Welcome to Humbler!"</h1>
        <Suspense fallback=move || view!{ <p>"Loading..."</p> }>
        {move || async_data.get().map(|api_infos| view! {
            <table class="bg-red-300 border border-gray-400">
                {HEADERS.iter().map(|&header| view!{ <th>{header}</th> }).collect::<Vec<_>>()}
                {api_infos.unwrap_or_default().into_iter().map(|api_info| view! {
                    <tr>
                        <td>{api_info.path}</td>
                        <td>{api_info.method}</td>
                        <td>{api_info.parameters.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")}</td>
                        <td>{api_info.request_body}</td>
                        <td>{api_info.response}</td>
                        <td>{api_info.swagger_url}</td>
                    </tr>
                }).collect::<Vec<_>>()}
            </table>
        })}
        </Suspense>
    }
}
