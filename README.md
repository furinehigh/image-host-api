# Image Host API by fh

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Rocket](https://img.shields.io/badge/Rocket-000000?style=for-the-badge&logo=rocket&logoColor=white)](https://rocket.rs/)
[![MongoDB](https://img.shields.io/badge/MongoDB-%234ea94b.svg?style=for-the-badge&logo=mongodb&logoColor=white)](https://www.mongodb.com/)
[![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)

A simple, high-performance image hosting service built with Rust, the Rocket web framework, and MongoDB. The application is designed to be deployed easily as a set of Docker containers.

It provides a web interface for manual uploads and a JSON API for programmatic integration. Uploaded images are automatically compressed and optimized in the background.

## Features

- **Fast Image Uploads**: Accepts common image formats via a web form or API.
- **Automatic Optimization**: Converts images to modern, efficient formats like WebP.
- **Background Processing**: Heavy optimization tasks are run in the background to ensure fast API responses.
- **Thumbnail Generation**: Automatically creates small thumbnails for previews.
- **Easy Deployment**: Fully containerized with Docker for simple setup and scaling.
- **JSON API**: Provides endpoints for retrieving image metadata.

## Tech Stack

- **Backend**: Rust, Rocket.rs
- **Database**: MongoDB
- **Image Processing**: `image`, `webp`, `oxipng` crates
- **Deployment**: Docker, Docker Compose

## Getting Started

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

### Installation & Setup

1.  **Clone the repository:**
    ```bash
    git clone <your-repository-url>
    cd image-host-api
    ```

2.  **Configure Environment Variables:**
    The application is configured using a `docker-compose.yml` file. The provided configuration is ready to run out of the box for local development.

    ```yaml
    # docker-compose.yml
    version: "3.8"

    services:
      app:
        build: .
        ports:
          - "8000:8080"
        environment:
          - MONGODB_URI=mongodb://mongo:27017
          - MONGODB_DB_NAME=image_host
          - HOST=localhost:8000 # Important for generating correct response URLs
          - ROCKET_ADDRESS=0.0.0.0
          - ROCKET_PORT=8080
        depends_on:
          - mongo

      mongo:
        image: mongo:latest
        restart: always
        volumes:
          - mongodb_data:/data/db

    volumes:
      mongodb_data:
    ```
    - **`HOST`**: This is crucial. It tells the application what hostname to use when generating full URLs in API responses. For production, you would change `localhost:8000` to your public domain name (e.g., `i.matdoes.dev`).

3.  **Build and Run the Application:**
    Use Docker Compose to build the images and start the services in the background.

    ```bash
    docker compose up --build -d
    ```

4.  **Access the Service:**
    Your image hosting service is now running!
    - **Web Interface**: `http://localhost:8000`
    - **API Base URL**: `http://localhost:8000`

## API Endpoints

### User Interface

---

#### `GET /`

-   **Description**: Serves the main HTML page with the image upload form.
-   **Response**:
    -   `200 OK`
    -   **Content-Type**: `text/html`

#### `POST /`

-   **Description**: Handles image uploads from the web interface. Expects a `multipart/form-data` payload with a file field named `image`.
-   **Response**:
    -   `303 See Other` - Redirects the user to the view page for the newly uploaded image (e.g., `/<image_id>`).

### Image API

---

#### `POST /api/upload`
#### `POST /api/upload/short`

-   **Description**: The primary API endpoint for uploading an image. Expects a `multipart/form-data` payload with a file field named `image`.
-   **Response**:
    -   `200 OK`
    -   **Content-Type**: `application/json`
    -   **Body**: A JSON object containing the ID and URLs for the uploaded image.

    **Example Response Body:**
    ```json
    {
      "hash": "pQrst7wXyZ",
      "url": "http://localhost:8000/pQrst7wXyZ",
      "view": "http://localhost:8000/pQrst7wXyZ"
    }
    ```

#### `GET /<id>`

-   **Description**: Retrieves and displays the raw image data for the specified ID. The `Content-Type` header of the response will match the optimized format of the stored image (e.g., `image/webp`).
-   **Parameters**:
    -   `id` (string): The unique ID of the image.
-   **Response**:
    -   `200 OK` - The binary image data.
    -   `404 Not Found` - If no image with that ID exists.

#### `GET /image/<id>`

-   **Description**: A legacy endpoint for compatibility. It permanently redirects to the `/id` endpoint.
-   **Response**:
    -   `308 Permanent Redirect` to `/<id>`.

#### `GET /json/<id>`

-   **Description**: Retrieves a JSON object containing metadata about the image, including its dimensions and a base64-encoded thumbnail.
-   **Parameters**:
    -   `id` (string): The unique ID of the image.
-   **Response**:
    -   `200 OK`
    -   **Content-Type**: `application/json`
    -   **Body**: A JSON object with image metadata.

    **Example Response Body:**
    ```json
    {
      "_id": "pQrst7wXyZ",
      "id": "pQrst7wXyZ",
      "thumbnail_b64": "UklGRhoCAABXRUJQVlA4TA0CAAAv/8A/AA... (base64 string)",
      "content-type": "image/webp",
      "width": 1024,
      "height": 768,
      "thumbnail-content-type": "image/webp"
    }
    ```

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.