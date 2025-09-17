#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod background_optimization;
mod db;
mod encoding;
mod util;

use background_optimization::{optimize_image_and_update, optimize_images_from_database};
use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use log::info;
use rocket::data::ToByteUnit;
use rocket::http::{ContentType, Header, Status};
use rocket::response::{Redirect, status::Custom};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{Data, State};
use rocket_multipart_form_data::{
    mime, MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
};
use std::io::Cursor;
use tokio::{join, task};
use util::ImageId;

lazy_static! {
    static ref HOST: String = std::env::var("HOST").unwrap_or("i.dishis.tech".to_string());
}

// Structs for API Requests and Responses
#[derive(Deserialize)]
struct ApiUploadRequest {
    base64: Option<String>,
    url: Option<String>,
}

#[derive(Serialize)]
struct ApiImageVariant {
    filename: String,
    name: String,
    mime: String,
    extension: String,
    url: String,
}

#[derive(Serialize)]
struct ApiImageData {
    id: String,
    title: String,
    url_viewer: String,
    url: String,
    display_url: String,
    width: String,
    height: String,
    size: String,
    time: String,
    expiration: String,
    image: ApiImageVariant,
    thumb: ApiImageVariant,
    medium: ApiImageVariant,
    delete_url: String,
}

#[derive(Serialize)]
struct ApiResponse {
    data: ApiImageData,
    success: bool,
    status: u16,
}

#[derive(Serialize)]
struct ApiErrorResponse {
    error: String,
    success: bool,
    status: u16,
}

// Helper Functions
async fn download_image_from_url(url: &str) -> Result<(Vec<u8>, String), String> {
    info!("Downloading image from URL: {}", url);
    let response = reqwest::get(url).await.map_err(|e| format!("Network error: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to download image: Server returned status {}", response.status()));
    }
    let content_type = response.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or("application/octet-stream").to_string();
    let image_bytes = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
    info!("Successfully downloaded {} bytes", image_bytes.len());
    Ok((image_bytes, content_type))
}

fn create_error(status: Status, message: &str) -> Custom<Json<ApiErrorResponse>> {
    Custom(status, Json(ApiErrorResponse {
        error: message.to_string(),
        success: false,
        status: status.code,
    }))
}

fn mime_to_extension(mime_type: &str) -> &str {
    mime_type.split('/').last().unwrap_or("jpg")
}

async fn process_and_respond(
    image_bytes: Vec<u8>,
    content_type_string: &str,
    images_collection: &mongodb::Collection<mongodb::bson::Document>,
) -> Result<Json<ApiResponse>, Custom<Json<ApiErrorResponse>>> {
    let mut reader = image::io::Reader::new(Cursor::new(&image_bytes));
    reader.set_format(util::mimetype_to_format(content_type_string));
    let decoded_image = reader.decode().map_err(|e| create_error(Status::BadRequest, &format!("Failed to decode image: {}", e)))?;

    let (encoded_image_result, encoded_thumbnail_result, image_id_result) = join!(
        encoding::from_image(decoded_image.clone(), encoding::FromImageOptions::default()),
        encoding::from_image(decoded_image, encoding::FromImageOptions { max_size: Some(128), ..encoding::FromImageOptions::default() }),
        db::generate_image_id(images_collection)
    );

    let encoded_image = encoded_image_result.map_err(|e| create_error(Status::InternalServerError, &e))?;
    let encoded_thumbnail = encoded_thumbnail_result.map_err(|e| create_error(Status::InternalServerError, &e))?;
    let image_id = image_id_result.map_err(|e| create_error(Status::InternalServerError, &e.to_string()))?;

    let insert_result = db::insert_image(images_collection, &db::NewImage { id: &image_id, data: &encoded_image.data, content_type: &encoded_image.content_type, thumbnail_data: &encoded_thumbnail.data, thumbnail_content_type: &encoded_thumbnail.content_type, size: encoded_image.size, optim_level: 0 }).await;
    let inserted_doc = insert_result.map_err(|_| create_error(Status::InternalServerError, "DB insert failed"))?.ok_or_else(|| create_error(Status::InternalServerError, "DB did not return doc"))?;

    info!("Successfully uploaded image {}", &image_id);

    let doc_for_bg = inserted_doc.clone();
    let owned_images_collection = images_collection.clone();
    task::spawn(async move {
        optimize_image_and_update(&owned_images_collection, &doc_for_bg).await.ok();
    });

    let id_str = image_id.to_string();
    let base_url = format!("https://{}", *HOST);
    let creation_time = inserted_doc.get_datetime("date").unwrap().timestamp_millis() / 1000;
    let image_ext = mime_to_extension(&encoded_image.content_type);
    let thumb_ext = mime_to_extension(&encoded_thumbnail.content_type);

    // UPDATED: Changed the URL structure to /i/<id> to prevent route collisions
    let image_url = format!("{}/i/{}", base_url, id_str);
    let thumb_url = format!("{}/i/{}/thumb", base_url, id_str);

    Ok(Json(ApiResponse {
        data: ApiImageData {
            id: id_str.clone(),
            title: id_str.clone(),
            url_viewer: image_url.clone(),
            url: image_url.clone(),
            display_url: image_url.clone(),
            width: encoded_image.size.0.to_string(),
            height: encoded_image.size.1.to_string(),
            size: encoded_image.data.len().to_string(),
            time: creation_time.to_string(),
            expiration: "0".to_string(),
            delete_url: format!("{}/delete/placeholder", image_url), // Placeholder delete URL
            image: ApiImageVariant { filename: format!("{}.{}", id_str, image_ext), name: id_str.clone(), mime: encoded_image.content_type.clone(), extension: image_ext.to_string(), url: image_url.clone() },
            medium: ApiImageVariant { filename: format!("{}.{}", id_str, image_ext), name: id_str.clone(), mime: encoded_image.content_type.clone(), extension: image_ext.to_string(), url: image_url.clone() },
            thumb: ApiImageVariant { filename: format!("{}.{}", id_str, thumb_ext), name: id_str.clone(), mime: encoded_thumbnail.content_type.clone(), extension: thumb_ext.to_string(), url: thumb_url },
        },
        success: true,
        status: 200,
    }))
}

// Rocket Routes
#[derive(Responder)]
#[response(status = 200)]
struct HtmlResponder(&'static str, Header<'static>);

#[get("/")]
fn index() -> HtmlResponder {
    HtmlResponder(include_str!("../site/index.html"), Header::new("Content-Type", "text/html; charset=utf-8"))
}

#[post("/", data = "<data>")]
async fn upload_from_web_route(
    content_type: &ContentType,
    data: Data<'_>,
    collections: &State<db::Collections>,
) -> Result<Redirect, Custom<Json<ApiErrorResponse>>> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("image").content_type_by_string(Some(mime::IMAGE_STAR)).unwrap(),
    ]);
    let form_data = MultipartFormData::parse(content_type, data, options).await.unwrap();
    if let Some(file_fields) = form_data.files.get("image") {
        let file = &file_fields[0];
        let image_bytes = tokio::fs::read(&file.path).await.map_err(|_| create_error(Status::InternalServerError, "Could not read temp file"))?;
        let content_type = file.content_type.as_ref().ok_or_else(|| create_error(Status::BadRequest, "MIME type is required"))?.to_string();
        
        let response = process_and_respond(image_bytes, &content_type, &collections.images).await?;
        // UPDATED: The uri! macro correctly builds the new path for the named function
        Ok(Redirect::to(uri!(view_image_route(response.data.id.clone()))))
    } else {
        Err(create_error(Status::BadRequest, "No image file found in form."))
    }
}

#[post("/api/upload", data = "<data>")]
async fn api_upload_route(
    content_type: &ContentType,
    data: Data<'_>,
    collections: &State<db::Collections>,
) -> Result<Json<ApiResponse>, Custom<Json<ApiErrorResponse>>> {
    if content_type.is_form_data() {
        let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
            MultipartFormDataField::file("image").content_type_by_string(Some(mime::IMAGE_STAR)).unwrap(),
        ]);
        let form_data = MultipartFormData::parse(content_type, data, options).await.unwrap();
        if let Some(file_fields) = form_data.files.get("image") {
            let file = &file_fields[0];
            let image_bytes = tokio::fs::read(&file.path).await.map_err(|_| create_error(Status::InternalServerError, "Could not read temp file"))?;
            let ct = file.content_type.as_ref().ok_or_else(|| create_error(Status::BadRequest, "MIME type is required"))?.to_string();
            process_and_respond(image_bytes, &ct, &collections.images).await
        } else {
            Err(create_error(Status::BadRequest, "Form field 'image' is missing."))
        }
    } else if content_type.is_json() {
        let bytes = data.open(10.megabytes()).into_bytes().await.map_err(|e| create_error(Status::BadRequest, &format!("Failed to read payload: {}", e)))?;
        let payload: ApiUploadRequest = rocket::serde::json::from_slice(&bytes).map_err(|e| create_error(Status::BadRequest, &format!("Invalid JSON: {}", e)))?;

        match (&payload.base64, &payload.url) {
            (Some(ref b64), None) => {
                let image_bytes = general_purpose::STANDARD.decode(b64).map_err(|_| create_error(Status::BadRequest, "Invalid Base64 string"))?;
                let kind = infer::get(&image_bytes).ok_or_else(|| create_error(Status::BadRequest, "Could not determine image type from Base64 data."))?;
                process_and_respond(image_bytes, kind.mime_type(), &collections.images).await
            },
            (None, Some(ref url)) => {
                let (image_bytes, ct) = download_image_from_url(url).await.map_err(|e| create_error(Status::BadRequest, &e))?;
                process_and_respond(image_bytes, &ct, &collections.images).await
            },
            _ => Err(create_error(Status::BadRequest, "Please provide 'base64' or 'url' in the JSON payload, but not both.")),
        }
    } else {
        Err(create_error(Status::UnsupportedMediaType, "Content-Type must be 'multipart/form-data' or 'application/json'."))
    }
}

#[derive(Responder)]
#[response(status = 200)]
struct ImageResponder(Vec<u8>, Header<'static>);

// UPDATED: Route changed from /<id> to /i/<id> to resolve collision
#[get("/i/<id>")]
async fn view_image_route(id: String, collections: &State<db::Collections>) -> Option<ImageResponder> {
    let doc = db::get_image(&collections.images, &id).await.ok()??;
    let data = doc.get_binary_generic("data").unwrap().clone();
    let ct = doc.get_str("content_type").unwrap().to_string();
    
    let images_collection = collections.images.clone();
    task::spawn(async move {
        db::update_last_seen(&images_collection, &ImageId(id)).await.ok();
    });

    Some(ImageResponder(data, Header::new("Content-Type", ct)))
}

// UPDATED: Route changed from /<id>/thumb to /i/<id>/thumb to resolve collision
#[get("/i/<id>/thumb")]
async fn view_thumbnail_route(id: String, collections: &State<db::Collections>) -> Option<ImageResponder> {
    let doc = db::get_image(&collections.images, &id).await.ok()??;
    let data = doc.get_binary_generic("thumbnail_data").unwrap().clone();
    let ct = doc.get_str("thumbnail_content_type").unwrap().to_string();
    Some(ImageResponder(data, Header::new("Content-Type", ct)))
}

// This route now correctly redirects to the non-colliding /i/<id> route
#[get("/image/<id>")]
fn redirect_image_route(id: String) -> Redirect {
    // UPDATED: The uri! macro correctly builds the new path for the named function
    Redirect::to(uri!(view_image_route(id)))
}

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    let images_collection = db::connect().await.unwrap();
    println!("Connected to database");

    let collections = db::Collections { images: images_collection.clone() };
    tokio::spawn(async move {
        optimize_images_from_database(&images_collection).await.expect("Failed optimizing images");
    });

    rocket::build()
        .manage(collections)
        .mount("/", routes![
            index,
            upload_from_web_route,
            api_upload_route,
            view_image_route,
            redirect_image_route,
            view_thumbnail_route
        ])
}