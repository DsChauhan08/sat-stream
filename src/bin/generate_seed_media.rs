use image::{ImageBuffer, Rgba};
use sat_stream::media_assets;

fn graph_line_trend() -> Vec<u8> {
    let width = 900u32;
    let height = 520u32;
    let mut img = ImageBuffer::from_pixel(width, height, Rgba([250, 252, 255, 255]));

    // axes
    for x in 80..860 {
        img.put_pixel(x, 430, Rgba([70, 80, 100, 255]));
    }
    for y in 80..431 {
        img.put_pixel(80, y, Rgba([70, 80, 100, 255]));
    }

    // grid lines
    for gx in (160..860).step_by(100) {
        for y in 90..430 {
            img.put_pixel(gx, y, Rgba([225, 232, 245, 255]));
        }
    }
    for gy in (120..420).step_by(60) {
        for x in 81..860 {
            img.put_pixel(x, gy, Rgba([225, 232, 245, 255]));
        }
    }

    let pts = [
        (120, 390),
        (230, 350),
        (340, 330),
        (450, 290),
        (560, 250),
        (670, 220),
        (780, 190),
    ];
    for w in pts.windows(2) {
        let (x1, y1) = w[0];
        let (x2, y2) = w[1];
        draw_line(&mut img, x1, y1, x2, y2, Rgba([37, 99, 235, 255]));
    }
    for (x, y) in pts {
        draw_dot(&mut img, x, y, 5, Rgba([37, 99, 235, 255]));
    }

    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn table_income() -> Vec<u8> {
    let width = 900u32;
    let height = 420u32;
    let mut img = ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 255]));

    // outer border
    for x in 80..820 {
        img.put_pixel(x, 60, Rgba([60, 60, 60, 255]));
        img.put_pixel(x, 360, Rgba([60, 60, 60, 255]));
    }
    for y in 60..361 {
        img.put_pixel(80, y, Rgba([60, 60, 60, 255]));
        img.put_pixel(820, y, Rgba([60, 60, 60, 255]));
    }

    // horizontal rows
    for y in [120, 180, 240, 300] {
        for x in 80..820 {
            img.put_pixel(x, y, Rgba([140, 140, 140, 255]));
        }
    }

    // vertical cols
    for x in [280, 500, 680] {
        for y in 60..361 {
            img.put_pixel(x, y, Rgba([140, 140, 140, 255]));
        }
    }

    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn scatterplot() -> Vec<u8> {
    let width = 900u32;
    let height = 520u32;
    let mut img = ImageBuffer::from_pixel(width, height, Rgba([248, 251, 247, 255]));

    for x in 90..840 {
        img.put_pixel(x, 430, Rgba([60, 90, 60, 255]));
    }
    for y in 90..431 {
        img.put_pixel(90, y, Rgba([60, 90, 60, 255]));
    }

    let pts = [
        (140, 390),
        (190, 365),
        (240, 360),
        (290, 340),
        (340, 325),
        (390, 315),
        (440, 300),
        (490, 290),
        (540, 265),
        (590, 250),
        (640, 235),
        (690, 225),
        (740, 205),
    ];
    for (x, y) in pts {
        draw_dot(&mut img, x, y, 4, Rgba([22, 101, 52, 255]));
    }

    draw_line(&mut img, 130, 400, 760, 200, Rgba([16, 185, 129, 255]));

    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn draw_line(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    c: Rgba<u8>,
) {
    let dx = (x2 - x1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let dy = -(y2 - y1).abs();
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x1;
    let mut y = y1;
    loop {
        if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
            img.put_pixel(x as u32, y as u32, c);
        }
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn draw_dot(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32, r: i32, c: Rgba<u8>) {
    for dx in -r..=r {
        for dy in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, c);
                }
            }
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let p1 = media_assets::save_png("seed_graph_line_trend.png", &graph_line_trend())
        .map_err(color_eyre::eyre::eyre)?;
    let p2 = media_assets::save_png("seed_table_income.png", &table_income())
        .map_err(color_eyre::eyre::eyre)?;
    let p3 = media_assets::save_png("seed_scatterplot.png", &scatterplot())
        .map_err(color_eyre::eyre::eyre)?;

    println!("Generated media:");
    println!("{}", p1);
    println!("{}", p2);
    println!("{}", p3);
    Ok(())
}
