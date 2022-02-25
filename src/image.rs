use crate::bulma::ImageRatio;
use crate::config_files::ImageReference;
use crate::files::{empty, write_raw};
use dashmap::DashMap;
use fast_image_resize::{FilterType, Image, MulDiv, PixelType, ResizeAlg, Resizer};
use image::io::Reader;
use image::GenericImage;
use std::borrow::Borrow;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU32;
use std::sync::Arc;
use webp::Encoder;

pub(crate) struct ImageProcessor<'a> {
    resizer: Resizer,
    img_source: &'a str,
    destination: &'a str,
    pub(crate) meta_cache: DashMap<String, Arc<ProcessedImage>>,
}

#[derive(Debug)]
pub(crate) struct ProcessedImage {
    pub(crate) title: String,
    pub(crate) alt: String,
    pub(crate) ratio: ImageRatio,
    pub(crate) srcset: String,
    pub(crate) src: String,
}

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
    pub(crate) fn new(img_source: &'a str, destination: &'a str) -> ImageProcessor<'a> {
        let resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));
        ImageProcessor {
            resizer,
            img_source,
            destination,
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
        let path = format!("{}/{}", &source, &reference.file_name);
        let mut origin = match Reader::open(&path) {
            Ok(f) => match f.decode() {
                Ok(image) => image,
                Err(e) => return Err(Box::new(e)),
            },
            Err(e) => return Err(Box::new(e)),
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
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
            cropped.borrow().to_image().into_raw(),
            PixelType::U8x4,
        )
        .unwrap();
        let file_base = match reference.file_name.rsplit_once('.') {
            None => {
                return Err(Box::new(ImageProcessingError {
                    details: String::from("file name doesn't contain a period"),
                }))
            }
            Some((first, _)) => String::from(first),
        };
        let mut srcset_part = vec![];
        for (width, quality) in all_widths(width) {
            let path = format!("/img{}/{}-w{}.webp", path_start, &file_base, &width);
            if empty(self.destination, &path) {
                let height = ratio.get_height(&width);
                let alpha_mul_div = MulDiv::default();
                // Multiple RGB channels of source image by alpha channel
                alpha_mul_div
                    .multiply_alpha_inplace(&mut src_image.view_mut())
                    .unwrap();
                // Create container for data of destination image
                let dst_width = NonZeroU32::new(width).unwrap();
                let dst_height = NonZeroU32::new(height).unwrap();
                let mut dst_image = Image::new(dst_width, dst_height, src_image.pixel_type());
                let mut dst_view = dst_image.view_mut();

                self.resizer
                    .resize(&src_image.view(), &mut dst_view)
                    .unwrap();

                // Divide RGB channels of destination image by alpha
                alpha_mul_div.divide_alpha_inplace(&mut dst_view).unwrap();

                // Write destination image as webp file
                let enc = Encoder::from_rgba(dst_image.buffer(), dst_width.get(), dst_height.get());
                let mut result = enc.encode(quality);
                write_raw(self.destination, &path, result.iter_mut());
            }
            srcset_part.push(format!("{} {}w", &path, width));
        }
        let src = srcset_part.last().unwrap().clone();
        let srcset = srcset_part.join(", ");
        let base_path = if path_start.is_empty() {
            format!("/{}", &file_base)
        } else {
            format!("{}/{}", path_start, &file_base)
        };
        Ok((
            base_path,
            Arc::new(ProcessedImage {
                title: reference.title.unwrap_or(file_base),
                alt: reference.alt_text,
                ratio,
                srcset,
                src,
            }),
        ))
    }
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
