use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use image::{ImageBuffer, RgbImage, DynamicImage};
use std::io::Cursor;
use tempfile::NamedTempFile;

// Mock image processing functions for benchmarking
fn resize_image_basic(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    img.resize(width, height, image::imageops::FilterType::Lanczos3)
}

fn compress_image_basic(img: &DynamicImage, quality: u8) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    img.write_to(&mut cursor, image::ImageOutputFormat::Jpeg(quality))
        .expect("Failed to write image");
    
    buffer
}

fn generate_thumbnail_basic(img: &DynamicImage, size: u32) -> DynamicImage {
    let aspect_ratio = img.width() as f32 / img.height() as f32;
    let (thumb_width, thumb_height) = if aspect_ratio > 1.0 {
        (size, (size as f32 / aspect_ratio) as u32)
    } else {
        ((size as f32 * aspect_ratio) as u32, size)
    };
    
    img.resize_exact(thumb_width, thumb_height, image::imageops::FilterType::Lanczos3)
}

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    let img: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
        let r = (x % 256) as u8;
        let g = (y % 256) as u8;
        let b = ((x + y) % 256) as u8;
        image::Rgb([r, g, b])
    });
    DynamicImage::ImageRgb8(img)
}

fn bench_image_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("image_resize");
    
    let test_image = create_test_image(1920, 1080);
    let sizes = vec![(800, 600), (1024, 768), (1280, 720)];
    
    for (width, height) in sizes {
        group.bench_with_input(
            BenchmarkId::new("resize", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    resize_image_basic(black_box(&test_image), black_box(w), black_box(h))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_image_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("image_compression");
    
    let test_image = create_test_image(1920, 1080);
    let qualities = vec![50, 75, 90, 95];
    
    for quality in qualities {
        group.bench_with_input(
            BenchmarkId::new("compress", format!("quality_{}", quality)),
            &quality,
            |b, &q| {
                b.iter(|| {
                    compress_image_basic(black_box(&test_image), black_box(q))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_thumbnail_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("thumbnail_generation");
    
    let test_image = create_test_image(1920, 1080);
    let sizes = vec![64, 128, 256, 512];
    
    for size in sizes {
        group.bench_with_input(
            BenchmarkId::new("thumbnail", format!("{}px", size)),
            &size,
            |b, &s| {
                b.iter(|| {
                    generate_thumbnail_basic(black_box(&test_image), black_box(s))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_image_format_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_conversion");
    
    let test_image = create_test_image(800, 600);
    
    group.bench_function("to_jpeg", |b| {
        b.iter(|| {
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            test_image.write_to(&mut cursor, image::ImageOutputFormat::Jpeg(85))
                .expect("Failed to convert to JPEG");
            buffer
        })
    });
    
    group.bench_function("to_png", |b| {
        b.iter(|| {
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            test_image.write_to(&mut cursor, image::ImageOutputFormat::Png)
                .expect("Failed to convert to PNG");
            buffer
        })
    });
    
    group.bench_function("to_webp", |b| {
        b.iter(|| {
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            test_image.write_to(&mut cursor, image::ImageOutputFormat::WebP)
                .expect("Failed to convert to WebP");
            buffer
        })
    });
    
    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");
    
    let images: Vec<DynamicImage> = (0..10)
        .map(|i| create_test_image(400 + i * 50, 300 + i * 50))
        .collect();
    
    group.bench_function("batch_resize", |b| {
        b.iter(|| {
            let _results: Vec<DynamicImage> = images
                .iter()
                .map(|img| resize_image_basic(black_box(img), 200, 150))
                .collect();
        })
    });
    
    group.bench_function("batch_thumbnail", |b| {
        b.iter(|| {
            let _results: Vec<DynamicImage> = images
                .iter()
                .map(|img| generate_thumbnail_basic(black_box(img), 128))
                .collect();
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_image_resize,
    bench_image_compression,
    bench_thumbnail_generation,
    bench_image_format_conversion,
    bench_batch_processing
);
criterion_main!(benches);
