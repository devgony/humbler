use std::env;

use humbler_core::humbler::{ApiInfo, Humbler};
use leptos::{html::Input, prelude::*};
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
async fn search(keyword: String) -> Result<Vec<ApiInfo>, ServerFnError> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let swagger_ui_url = "http://localhost:4000/swagger-ui/index.html".to_owned();
    let openapi_json_url = "core/data/pet.json".to_owned();

    let humbler = Humbler::new(swagger_ui_url, openapi_json_url);

    humbler
        .search(keyword)
        .await
        .map_err(|e| ServerFnError::new(format!("Error: {e}")))
        .map(|h| h.api_infos)
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
    let input_ref = NodeRef::<Input>::new();
    let search = Action::new(|input: &String| {
        let input = input.to_owned();
        async move { search(input).await }
    });
    let value = search.value();
    view! {
        <main>
            <h1 class="text-red-300">"Welcome to Humbler!"</h1>
        <form on:submit = move |ev| {
            ev.prevent_default(); // don't reload the page...
            let input = input_ref.get().expect("input to exist");
            search.dispatch(input.value());
        }>
            <input type="text" node_ref=input_ref placeholder="Search Path" />

            <button type="submit">Search</button>
        </form>
            <div class="result">
                <Suspense fallback=move || view!{ <p>"Loading..."</p> }>
                    {move || value.get().map(|api_infos| view! {
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
            </div>
        </main>
    }
}
