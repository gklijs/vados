use crate::bulma::ImageRatio;
use crate::config_files::ImageReference;
use crate::files::{empty, write_raw};
use dashmap::DashMap;
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, MulDiv, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use image::{DynamicImage, ExtendedColorType, GenericImage, GenericImageView, ImageEncoder};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;
use webp::Encoder;

pub(crate) struct ImageProcessor<'a> {
    resizer: Resizer,
    img_source: &'a str,
    destination: &'a str,
    /// The reference keys of the images that also get a link-preview crop:
    /// every page's hero image and the site's default one.
    preview_keys: HashSet<String>,
    pub(crate) meta_cache: DashMap<String, Arc<ProcessedImage>>,
}

#[derive(Debug)]
pub(crate) struct ProcessedImage {
    pub(crate) title: String,
    pub(crate) alt: String,
    pub(crate) ratio: ImageRatio,
    pub(crate) srcset: String,
    pub(crate) src: String,
    /// The image as link previews show it, when it's used as one.
    pub(crate) preview: Option<PreviewImage>,
}

/// A JPEG crop of an image in the 1.91:1 shape LinkedIn, Facebook, X and
/// Mastodon all show link previews in. JPEG, not WebP: LinkedIn doesn't
/// reliably show a WebP preview. See `vados.allium`'s `LinkPreviewImage`.
#[derive(Debug)]
pub(crate) struct PreviewImage {
    pub(crate) path: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

const PREVIEW_WIDTH: u32 = 1200;
const PREVIEW_HEIGHT: u32 = 630;
const PREVIEW_QUALITY: u8 = 85;

#[derive(Debug)]
struct ImageProcessingError {
    details: String,
}

impl Display for ImageProcessingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ImageProcessingError {}

impl<'a> ImageProcessor<'a> {
    pub(crate) fn new(
        img_source: &'a str,
        destination: &'a str,
        preview_keys: HashSet<String>,
    ) -> ImageProcessor<'a> {
        let resizer = Resizer::new();
        ImageProcessor {
            resizer,
            img_source,
            destination,
            preview_keys,
            meta_cache: DashMap::new(),
        }
    }
    pub(crate) fn process_list(&mut self, source: &str, list: Vec<ImageReference>) {
        let path_start = if source.len() == self.img_source.len() {
            String::new()
        } else {
            String::from(source).split_off(self.img_source.len())
        };
        for reference in list {
            match self.process_reference(source, &path_start, reference) {
                Ok((key, value)) => {
                    self.meta_cache.insert(key, value);
                }
                Err(e) => println!("error: {}", e),
            }
        }
    }
    fn process_reference(
        &mut self,
        source: &str,
        path_start: &str,
        reference: ImageReference,
    ) -> Result<(String, Arc<ProcessedImage>), Box<dyn Error>> {
        let path = format!("{}/{}", source, reference.file_name);
        let mut origin = match ImageReader::open(&path) {
            Ok(f) => match f.decode() {
                Ok(image) => image,
                Err(e) => return Err(Box::new(e)),
            },
            Err(e) => return Err(Box::new(e)),
        };
        let file_base = match reference.file_name.rsplit_once('.') {
            None => {
                return Err(Box::new(ImageProcessingError {
                    details: String::from("file name doesn't contain a period"),
                }))
            }
            Some((first, _)) => String::from(first),
        };
        let base_path = if path_start.is_empty() {
            format!("/{}", file_base)
        } else {
            format!("{}/{}", path_start, file_base)
        };
        let preview = if self.preview_keys.contains(&base_path) {
            let path = format!("/img{}/{}-og.jpg", path_start, file_base);
            Some(self.write_preview(&origin, path)?)
        } else {
            None
        };

        let ratio = ImageRatio::best_fitting(&origin.width(), &origin.height());
        let mut width = ratio.get_width(&origin.height());
        let mut height = ratio.get_height(&origin.width());
        let mut x = 0;
        let mut y = 0;
        if width > origin.width() {
            width = origin.width();
            let height_dif = origin.height() - height;
            y = height_dif / 2;
        } else if height > origin.height() {
            height = origin.height();
            let width_dif = origin.width() - width;
            x = width_dif / 2;
        }
        let cropped = origin.sub_image(x, y, width, height);
        let mut src_image = Image::from_vec_u8(
            width,
            height,
            cropped.borrow().to_image().into_raw(),
            PixelType::U8x4,
        )?;
        // Premultiplied once, up front, rather than once per variant: every
        // variant below resizes the same source pixels, so repeating this per
        // iteration would multiply by alpha again on top of the previous
        // iteration's result instead of being a no-op, corrupting every
        // variant after the first for any image with a partial alpha channel.
        let alpha_mul_div = MulDiv::new();
        alpha_mul_div.multiply_alpha_inplace(&mut src_image)?;

        let resize_options =
            ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
        let mut srcset_part = vec![];
        let mut src = String::new();
        for (width, quality) in all_widths(width) {
            let path = format!("/img{}/{}-w{}.webp", path_start, file_base, width);
            if empty(self.destination, &path) {
                let height = ratio.get_height(&width);
                // Create container for data of destination image
                let mut dst_image = Image::new(width, height, src_image.pixel_type());

                self.resizer
                    .resize(&src_image, &mut dst_image, &resize_options)?;

                // Divide RGB channels of destination image by alpha
                alpha_mul_div.divide_alpha_inplace(&mut dst_image)?;

                // Write destination image as webp file
                let enc = Encoder::from_rgba(dst_image.buffer(), width, height);
                let mut result = enc.encode(quality);
                write_raw(self.destination, &path, result.iter_mut());
            }
            srcset_part.push(format!("{} {}w", path, width));
            // `src` is the fallback for browsers without srcset support, so it
            // must be a bare URL: the width descriptor only belongs in srcset.
            src = path;
        }
        let srcset = srcset_part.join(", ");
        Ok((
            base_path,
            Arc::new(ProcessedImage {
                title: reference.title.unwrap_or(file_base),
                alt: reference.alt_text,
                ratio,
                srcset,
                src,
                preview,
            }),
        ))
    }

    /// Writes the link-preview crop of `origin` to `path`, unless an earlier
    /// run already did: centre-cropped to 1.91:1 and scaled to 1200 pixels
    /// wide, or kept at its own size when it's smaller than that. Any
    /// transparency is flattened onto white, since JPEG has none.
    fn write_preview(
        &mut self,
        origin: &DynamicImage,
        path: String,
    ) -> Result<PreviewImage, Box<dyn Error>> {
        let (origin_width, origin_height) = origin.dimensions();
        let (crop_width, crop_height) =
            if origin_width * PREVIEW_HEIGHT > origin_height * PREVIEW_WIDTH {
                (
                    origin_height * PREVIEW_WIDTH / PREVIEW_HEIGHT,
                    origin_height,
                )
            } else {
                (origin_width, origin_width * PREVIEW_HEIGHT / PREVIEW_WIDTH)
            };
        let (width, height) = if crop_width > PREVIEW_WIDTH {
            (PREVIEW_WIDTH, PREVIEW_HEIGHT)
        } else {
            (crop_width, crop_height)
        };
        if empty(self.destination, &path) {
            let x = (origin_width - crop_width) / 2;
            let y = (origin_height - crop_height) / 2;
            let cropped = origin.view(x, y, crop_width, crop_height).to_image();
            let rgb: Vec<u8> = cropped
                .pixels()
                .flat_map(|p| {
                    let [r, g, b, a] = p.0;
                    let over_white =
                        |c: u8| ((c as u32 * a as u32 + 255 * (255 - a as u32)) / 255) as u8;
                    [over_white(r), over_white(g), over_white(b)]
                })
                .collect();
            let src_image = Image::from_vec_u8(crop_width, crop_height, rgb, PixelType::U8x3)?;
            let mut dst_image = Image::new(width, height, PixelType::U8x3);
            let resize_options =
                ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
            self.resizer
                .resize(&src_image, &mut dst_image, &resize_options)?;
            let mut jpeg = vec![];
            JpegEncoder::new_with_quality(&mut jpeg, PREVIEW_QUALITY).write_image(
                dst_image.buffer(),
                width,
                height,
                ExtendedColorType::Rgb8,
            )?;
            write_raw(self.destination, &path, jpeg);
        }
        Ok(PreviewImage {
            path,
            width,
            height,
        })
    }
}

/// Derives the key an image would be looked up by, mirroring
/// `process_reference`'s key derivation without decoding anything. Used by
/// `check` to test page/notification image references against the declared
/// image set structurally.
pub(crate) fn reference_key_for(path_start: &str, file_name: &str) -> Option<String> {
    let file_base = file_name.rsplit_once('.').map(|(base, _)| base)?;
    Some(if path_start.is_empty() {
        format!("/{}", file_base)
    } else {
        format!("{}/{}", path_start, file_base)
    })
}

const ALL_VARIANTS: [(u32, f32); 7] = [
    (1972, 75_f32),
    (1479, 75_f32),
    (986, 75_f32),
    (632, 75_f32),
    (425, 75_f32),
    (318, 75_f32),
    (159, 40_f32),
];

/// Removes previously generated image variants at the destination whose
/// source image is no longer declared in this run's `images.json` files.
/// Keeps the destination's `img` directory in sync with the current set of
/// declared images instead of accumulating stale output indefinitely.
pub(crate) fn prune_orphaned_variants(
    destination: &str,
    meta_cache: &DashMap<String, Arc<ProcessedImage>>,
) {
    let img_root = format!("{}/img", destination);
    if !Path::new(&img_root).exists() {
        return;
    }
    for entry in WalkDir::new(&img_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_name = entry.file_name().to_string_lossy();
        let file_base = match file_name
            .strip_suffix(".webp")
            .and_then(|s| s.rsplit_once("-w"))
        {
            Some((base, width))
                if !width.is_empty() && width.chars().all(|c| c.is_ascii_digit()) =>
            {
                base
            }
            _ => match file_name.strip_suffix("-og.jpg") {
                Some(base) => base,
                None => continue,
            },
        };
        let is_preview = file_name.ends_with("-og.jpg");
        let path_start = entry
            .path()
            .parent()
            .and_then(|p| p.strip_prefix(&img_root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let key = if path_start.is_empty() {
            format!("/{}", file_base)
        } else {
            format!("/{}/{}", path_start, file_base)
        };
        // A preview crop also goes once its image is no longer used as one.
        let still_used = meta_cache
            .get(&key)
            .is_some_and(|i| !is_preview || i.preview.is_some());
        if !still_used {
            match fs::remove_file(entry.path()) {
                Ok(_) => println!("Pruned orphaned image variant {}", entry.path().display()),
                Err(e) => println!(
                    "Failed to prune orphaned image variant {}: {}",
                    entry.path().display(),
                    e
                ),
            }
        }
    }
}

fn all_widths(width: u32) -> Vec<(u32, f32)> {
    match width {
        w if w >= ALL_VARIANTS[0].0 => ALL_VARIANTS[..].to_vec(),
        w if w >= ALL_VARIANTS[1].0 => ALL_VARIANTS[1..].to_vec(),
        w if w >= ALL_VARIANTS[2].0 => ALL_VARIANTS[2..].to_vec(),
        w if w >= ALL_VARIANTS[3].0 => ALL_VARIANTS[3..].to_vec(),
        w if w >= ALL_VARIANTS[4].0 => ALL_VARIANTS[4..].to_vec(),
        w if w >= ALL_VARIANTS[5].0 => ALL_VARIANTS[5..].to_vec(),
        w if w >= ALL_VARIANTS[6].0 => ALL_VARIANTS[6..].to_vec(),
        _ => vec![(width, 90_f32)],
    }
}
