use anyhow::Result;
use axum::{
    extract::Multipart,
    response::Html,
    routing::{get, post},
    Router,
};
use openfoodfacts as off;
use serde_json::{json, Value};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tower_http::cors::{Any, CorsLayer};

fn create_nutrional_facts_file(file_name: &str) -> Result<String> {
    let client = off::v2().build()?;
    let bar_code = rxing::helpers::detect_in_file(file_name, None)?;
    let bar_code_text = bar_code.getText();
    let code = bar_code_text;
    let response = client.product(code, None).unwrap();
    let result_json = json!(response.json::<HashMap::<String, Value>>()?);
    let selected_image = &result_json["product"]["selected_images"]["front"]["display"]["en"];
    let serving_size = &result_json["product"]["serving_size"];
    let calories_per = &result_json["product"]["nutriments"]["energy-kcal_serving"];
    let carbs_per = &result_json["product"]["nutriments"]["carbohydrates_serving"];
    let protein_per = &result_json["product"]["nutriments"]["proteins_serving"];
    let fats_per = &result_json["product"]["nutriments"]["fat_serving"];
    Ok(format!(
         "<img src={selected_image}>
         <h1>Tamaño de serving: {serving_size}<br>
    Valores nutricionales (por serving):<br>
    Calorías (kcal): {calories_per}<br>
    Carbos: {carbs_per}g<br>
    Proteína: {protein_per}g<br>
    Grasa: {fats_per}g</h1>"
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);
    let app = Router::new()
        .route("/", get(root))
        .route("/sanity", get(sanity_check))
        .route("/upload", post(upload))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app.into_make_service()).await.unwrap();

    Ok(())
}
async fn root() -> Html<&'static str> {
    Html(
        r#"


<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="author" content="Pirson Bethancourt" />
      <meta name="viewport" content="width=69px" />
  <title>Pana Fit Prototype</title>
  <script src="https://unpkg.com/htmx.org@2.0.2"></script>
  <script src="https://cdn.tailwindcss.com"></script>
  <link rel="stylesheet" href="index.css">
</head>
<body class="bg-[#282828] text-[#ebdbb2] font-sans p-4 sm:p-8 flex items-center justify-center min-h-screen">

  <div class="bg-[#3c3836] p-6 sm:p-8 md:p-12 rounded-lg shadow-md w-full max-w-xs sm:max-w-md">
    <form id="form" hx-encoding="multipart/form-data" hx-post="/upload" hx-swap="afterend swap:1s" class="space-y-4">
      <div>
        <label for="file" class="block text-sm font-medium text-[#d5c4a1]">Upload Image</label>
        <input type="file" name="file" id="file" class="mt-2 block w-full text-sm text-[#ebdbb2] border border-[#504945] rounded-lg cursor-pointer bg-[#282828] focus:outline-none focus:ring-2 focus:ring-[#b8bb26] focus:border-[#b8bb26]">
      </div>
      <button type="submit" class="w-full py-2 px-4 bg-[#98971a] text-[#282828] font-semibold rounded-full shadow-md hover:bg-[#b8bb26] focus:outline-none focus:ring-2 focus:ring-[#b8bb26] focus:ring-offset-2">
        Upload
      </button>
      <progress id="progress" value="0" max="100" class="w-full h-2 rounded-full overflow-hidden bg-[#504945]"></progress>
    </form>
  </div>

  <script>
    htmx.on('#form', 'htmx:xhr:progress', function(evt) {
      htmx.find('#progress').setAttribute('value', evt.detail.loaded / evt.detail.total * 100);
    });
  </script>
</body>
</html>
    "#,
    )
}
async fn sanity_check() -> &'static str {
    "Server is up and runnning!\n"
}
// This function is not working even though the file is saved correctly
async fn upload(mut multipart: Multipart) -> Html<String> {
    let mut file_name = String::new();
    let mut file_data = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let fname = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if !content_type.starts_with("image/") {
            return Html("<p>Please upload only images.</p>".to_string());
        }

        file_name = fname;
        file_data = data.to_vec();
    }
    let file_name_with_extension = Arc::new(String::from(file_name));
    println!("{}", file_name_with_extension);
    let file_name_with_extension_clone = file_name_with_extension.clone();
    let file_name_with_extension_clone_2 = file_name_with_extension.clone();
    let mut file = tokio::fs::File::create(file_name_with_extension.as_str())
        .await
        .unwrap();
    file.write_all(&file_data)
        .await
        .expect("file creation failed");
    let response = tokio::task::spawn_blocking(move || {
        let file_name = file_name_with_extension_clone.as_str();
        println!("{}", file_name);
        create_nutrional_facts_file(file_name).unwrap_or(String::from(
            "could not read file, make sure it is a valid image!",
        ))
    })
    .await
    .unwrap();
    tokio::fs::remove_file(file_name_with_extension_clone_2.as_str())
        .await
        .unwrap();
    Html(response)
}
