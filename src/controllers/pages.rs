use axum::{response::Html, Extension};

pub async fn get_home_page(Extension(sub): Extension<Option<String>>) -> Html<&'static str> {
    if let Some(_) = sub {
        return Html(include_str!("../../static/index.html"));
    }

    Html(include_str!("../../static/login.html"))
}
