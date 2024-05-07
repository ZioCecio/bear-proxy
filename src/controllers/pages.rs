use axum::response::Html;

pub async fn get_home_page() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}
