use crate::bulma::ImageRatio;
use crate::config_files::ImageReference;
use crate::files::write_raw;
use dashmap::DashMap;
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, MulDiv, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use image::{DynamicImage, ExtendedColorType, GenericImage, GenericImageView, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::sync::Arc;
use webp::Encoder;

pub(crate) struct ImageProcessor<'a> {
    resizer: Resizer,
    #[cfg(test)]
    decodes: usize,
    img_source: &'a str,
    destination: &'a crate::files::Output,
    /// The reference keys of the images that also get a link-preview crop:
    /// every page's hero image and the site's default one.
    preview_keys: HashSet<String>,
    pub(crate) meta_cache: DashMap<String, Arc<ProcessedImage>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProcessedImage {
    pub(crate) title: String,
    pub(crate) alt: String,
    pub(crate) ratio: ImageRatio,
    pub(crate) srcset: String,
    pub(crate) src: String,
    /// The size of the `src` variant, given on the `<img>` so the browser
    /// can reserve its space before it loads.
    pub(crate) src_width: u32,
    pub(crate) src_height: u32,
    /// The image as link previews show it, when it's used as one.
    pub(crate) preview: Option<PreviewImage>,
}

/// A JPEG crop of an image in the 1.91:1 shape LinkedIn, Facebook, X and
/// Mastodon all show link previews in. JPEG, not WebP: LinkedIn doesn't
/// reliably show a WebP preview. See `vados.allium`'s `LinkPreviewImage`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PreviewImage {
    pub(crate) path: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

// Bump when crop rules, widths, quality, or encoder behaviour changes (including
// encoder dependency upgrades). This also versions the cached metadata format.
const SETTINGS_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct CachedImage {
    fingerprint: String,
    settings_version: u32,
    variants: Vec<String>,
    image: ProcessedImage,
}

// Stable FNV-1a plus byte length: this detects accidental source changes, not
// adversarial collisions. Unlike mtimes, it survives checkouts and copied caches.
fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}-{}", bytes.len())
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
        destination: &'a crate::files::Output,
        preview_keys: HashSet<String>,
    ) -> ImageProcessor<'a> {
        let resizer = Resizer::new();
        ImageProcessor {
            resizer,
            #[cfg(test)]
            decodes: 0,
            img_source,
            destination,
            preview_keys,
            meta_cache: DashMap::new(),
        }
    }
    pub(crate) fn process_list(
        &mut self,
        source: &str,
        list: Vec<ImageReference>,
    ) -> Result<(), crate::generator::GenerateError> {
        let path_start = if source.len() == self.img_source.len() {
            String::new()
        } else {
            String::from(source).split_off(self.img_source.len())
        };
        for reference in list {
            let path = format!("{}/{}", source, reference.file_name);
            match self.process_reference(source, &path_start, reference) {
                Ok((key, value)) => {
                    self.meta_cache.insert(key, value);
                }
                Err(e) => return Err(crate::generator::GenerateError::at(path, e)),
            }
        }
        Ok(())
    }
    fn process_reference(
        &mut self,
        source: &str,
        path_start: &str,
        reference: ImageReference,
    ) -> Result<(String, Arc<ProcessedImage>), Box<dyn Error>> {
        let path = format!("{}/{}", source, reference.file_name);
        let bytes = std::fs::read(&path)?;
        let fingerprint = fingerprint(&bytes);
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
        let cache_path = format!("/.vados-images{base_path}.json");
        let needs_preview = self.preview_keys.contains(&base_path);
        if let Some(mut cached) = self.destination.read_cache::<CachedImage>(&cache_path)? {
            if cached.fingerprint == fingerprint
                && cached.settings_version == SETTINGS_VERSION
                && cached.image.preview.is_some() == needs_preview
                && !cached.variants.is_empty()
            {
                let mut complete = true;
                for variant in &cached.variants {
                    complete &= self.destination.is_file(variant)?;
                }
                if complete {
                    for variant in &cached.variants {
                        self.destination.record(variant)?;
                    }
                    self.destination.record(&cache_path)?;
                    // Captions are authoring data, not properties of the pixels.
                    cached.image.title = reference.title.unwrap_or(file_base);
                    cached.image.alt = reference.alt_text;
                    return Ok((base_path, Arc::new(cached.image)));
                }
            }
        }
        // Invalidate before touching variants. A failed or interrupted rebuild
        // must never leave an old record blessing a mixture of old/new files.
        self.destination.remove_cache(&cache_path)?;
        #[cfg(test)]
        {
            self.decodes += 1;
        }
        let mut origin = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()?
            .decode()?;
        let mut variants = Vec::new();
        let preview = if self.preview_keys.contains(&base_path) {
            let path = format!("/img{}/{}-og.jpg", path_start, file_base);
            variants.push(path.clone());
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
        let widths = all_widths(width);
        // `src` is the fallback for browsers and crawlers that don't read
        // srcset, so it's a variant worth looking at: the widest up to
        // FALLBACK_WIDTH, or the narrowest when every one is wider.
        let src_width = widths
            .iter()
            .map(|(w, _)| *w)
            .find(|w| *w <= FALLBACK_WIDTH)
            .unwrap_or_else(|| widths.last().unwrap().0);
        let mut src = String::new();
        for (width, quality) in widths {
            let path = format!("/img{}/{}-w{}.webp", path_start, file_base, width);
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
            write_raw(self.destination, &path, result.iter_mut())?;
            variants.push(path.clone());
            srcset_part.push(format!("{} {}w", path, width));
            // A bare URL: the width descriptor only belongs in srcset.
            if width == src_width {
                src = path;
            }
        }
        let srcset = srcset_part.join(", ");
        let src_height = ratio.get_height(&src_width);
        let cached = CachedImage {
            fingerprint,
            settings_version: SETTINGS_VERSION,
            variants,
            image: ProcessedImage {
                title: reference.title.unwrap_or(file_base),
                alt: reference.alt_text,
                ratio,
                srcset,
                src,
                src_width,
                src_height,
                preview,
            },
        };
        write_raw(self.destination, &cache_path, serde_json::to_vec(&cached)?)?;
        Ok((base_path, Arc::new(cached.image)))
    }

    /// Writes the link-preview crop of `origin` to `path`: centre-cropped to
    /// 1.91:1 and scaled to 1200 pixels wide, or kept at its own size when smaller. Any
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
        write_raw(self.destination, &path, jpeg)?;
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

/// The widest variant `src` falls back to.
const FALLBACK_WIDTH: u32 = 986;

const ALL_VARIANTS: [(u32, f32); 7] = [
    (1972, 75_f32),
    (1479, 75_f32),
    (986, 75_f32),
    (632, 75_f32),
    (425, 75_f32),
    (318, 75_f32),
    (159, 40_f32),
];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Output;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Images {
        root: PathBuf,
    }

    impl Images {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "vados_image_cache_{}_{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("source")).unwrap();
            let site = Self { root };
            site.replace(340, 200, [220, 30, 20]);
            site
        }

        fn replace(&self, width: u32, height: u32, color: [u8; 3]) {
            image::RgbImage::from_pixel(width, height, image::Rgb(color))
                .save(self.root.join("source/photo.png"))
                .unwrap();
        }

        fn output(&self, path: &str) -> PathBuf {
            self.root.join("public").join(path.trim_start_matches('/'))
        }

        fn run(
            &self,
            preview: bool,
            alt: &str,
        ) -> Result<(usize, Arc<ProcessedImage>), crate::generator::GenerateError> {
            let source = self.root.join("source");
            let output = Output::new(self.output("").to_str().unwrap())?;
            let mut processor = ImageProcessor::new(
                source.to_str().unwrap(),
                &output,
                if preview {
                    HashSet::from(["/photo".to_owned()])
                } else {
                    HashSet::new()
                },
            );
            processor.process_list(
                source.to_str().unwrap(),
                vec![ImageReference {
                    file_name: "photo.png".to_owned(),
                    title: Some(format!("Title: {alt}")),
                    alt_text: alt.to_owned(),
                }],
            )?;
            let metadata = processor.meta_cache.get("/photo").unwrap().clone();
            output.finish()?;
            Ok((processor.decodes, metadata))
        }

        fn cache(&self) -> CachedImage {
            serde_json::from_slice(&fs::read(self.output(".vados-images/photo.json")).unwrap())
                .unwrap()
        }

        fn variant_bytes(&self) -> Vec<Vec<u8>> {
            self.cache()
                .variants
                .iter()
                .map(|p| fs::read(self.output(p)).unwrap())
                .collect()
        }
    }

    impl Drop for Images {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn replaced_source_regenerates_every_variant_and_preview() {
        let site = Images::new();
        site.run(true, "before").unwrap();
        let before = site.variant_bytes();
        site.replace(340, 200, [20, 30, 220]);
        assert_eq!(site.run(true, "after").unwrap().0, 1);
        let after = site.variant_bytes();
        assert_eq!(before.len(), after.len());
        assert!(before.iter().zip(&after).all(|(a, b)| a != b));
        // A size change updates layout metadata and prunes obsolete widths.
        site.replace(80, 80, [20, 30, 220]);
        let (_, metadata) = site.run(true, "small").unwrap();
        assert_eq!((metadata.src_width, metadata.src_height), (80, 80));
        assert!(!site.output("img/photo-w318.webp").exists());
    }

    #[test]
    fn unchanged_source_skips_decoding_and_keeps_current_captions() {
        let site = Images::new();
        assert_eq!(site.run(true, "first").unwrap().0, 1);
        let before = site.variant_bytes();
        let times: Vec<_> = site
            .cache()
            .variants
            .iter()
            .map(|p| fs::metadata(site.output(p)).unwrap().modified().unwrap())
            .collect();
        let (decodes, metadata) = site.run(true, "second").unwrap();
        assert_eq!(decodes, 0);
        assert_eq!(metadata.alt, "second");
        assert_eq!(metadata.title, "Title: second");
        assert_eq!(before, site.variant_bytes());
        let after_times: Vec<_> = site
            .cache()
            .variants
            .iter()
            .map(|p| fs::metadata(site.output(p)).unwrap().modified().unwrap())
            .collect();
        assert_eq!(times, after_times);
    }

    #[test]
    fn missing_variant_or_changed_settings_rebuilds_the_whole_image() {
        let site = Images::new();
        site.run(true, "photo").unwrap();
        let expected = site.variant_bytes();
        let variants = site.cache().variants;
        for missing in 0..variants.len() {
            fs::remove_file(site.output(&variants[missing])).unwrap();
            let stale = (missing + 1) % variants.len();
            fs::write(site.output(&variants[stale]), b"stale variant").unwrap();
            assert_eq!(site.run(true, "photo").unwrap().0, 1);
            assert_eq!(expected, site.variant_bytes());
        }
        let mut cache = site.cache();
        cache.settings_version += 1;
        fs::write(
            site.output(".vados-images/photo.json"),
            serde_json::to_vec(&cache).unwrap(),
        )
        .unwrap();
        fs::write(site.output(&variants[1]), b"old encoder output").unwrap();
        assert_eq!(site.run(true, "photo").unwrap().0, 1);
        assert_eq!(expected, site.variant_bytes());
    }

    #[test]
    fn missing_or_malformed_cache_and_preview_changes_are_safe_misses() {
        let site = Images::new();
        site.run(false, "photo").unwrap();
        assert_eq!(site.run(true, "photo").unwrap().0, 1);
        assert!(site.output("img/photo-og.jpg").exists());
        assert_eq!(site.run(false, "photo").unwrap().0, 1);
        assert!(!site.output("img/photo-og.jpg").exists());
        fs::remove_file(site.output(".vados-images/photo.json")).unwrap();
        assert_eq!(site.run(false, "photo").unwrap().0, 1);
        fs::write(site.output(".vados-images/photo.json"), b"{").unwrap();
        assert_eq!(site.run(false, "photo").unwrap().0, 1);
        assert_eq!(site.run(false, "photo").unwrap().0, 0);
    }

    #[test]
    fn failed_rebuild_invalidates_record_even_if_source_is_reverted() {
        let site = Images::new();
        site.run(true, "photo").unwrap();
        let original_source = fs::read(site.root.join("source/photo.png")).unwrap();
        let original_variants = site.variant_bytes();
        let blocked = site.output("img/photo-w318.webp");
        fs::remove_file(&blocked).unwrap();
        fs::create_dir(&blocked).unwrap();
        site.replace(340, 200, [20, 30, 220]);
        assert!(site.run(true, "photo").is_err());
        assert!(!site.output(".vados-images/photo.json").exists());
        fs::remove_dir(&blocked).unwrap();
        fs::write(site.root.join("source/photo.png"), original_source).unwrap();
        assert_eq!(site.run(true, "photo").unwrap().0, 1);
        assert_eq!(original_variants, site.variant_bytes());
    }
}
