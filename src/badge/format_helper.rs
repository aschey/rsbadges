// BSD 3-Clause License
//
// Copyright (c) 2021 RSBadges Authors
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
//    this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors
//    may be used to endorse or promote products derived from this software
//    without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
// INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA,
// OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
// OF SUCH DAMAGE.

//! Different helper functions used when formatting a badge for SVG generation.

use super::badge_type::BadgeError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use css_color::Rgba;
use rusttype::{point, Font, Scale};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

const LIGHT_TEXT_COLOR: &str = "#fff";
const DARK_TEXT_COLOR: &str = "#333";

const LIGHT_SHADOW_COLOR: &str = "#ccc";
const DARK_SHADOW_COLOR: &str = "#010101";

// Gamma-adjusted greyscale midpoint normalized to the 0-1 range
const BRIGHTNESS_THRESHOLD: f32 = 0.579;

pub struct AccentColors {
    pub text_color: &'static str,
    pub shadow_color: &'static str,
}

/// Load a font into Rust.
/// Docs: https://gitlab.redox-os.org/redox-os/rusttype/-/blob/master/dev/examples/ascii.rs
pub fn load_font<'a>(bytes: &'static [u8]) -> Result<Font<'a>, BadgeError> {
    match Font::try_from_bytes(bytes) {
        Some(f) => Ok(f),
        None => Err(BadgeError::CannotLoadFont),
    }
}

/// Produce the text dimensions given the font, text, and requested size of the
/// string.
pub fn get_text_dims(font: &Font, text: &str, font_size: f32) -> (String, f32) {
    let norm_text = text.nfc().collect::<String>();
    let scale = Scale::uniform(font_size);
    let layout = font.layout(&norm_text, scale, point(0.0, 0.0));
    let mut glyphs_width = layout.fold(0.0, |acc, x| {
        acc + x.into_unpositioned().h_metrics().advance_width
    });
    if glyphs_width as usize % 2 == 0 {
        glyphs_width += 1.0;
    }
    (norm_text, glyphs_width)
}

/// Verify that the string passed in is a valid color.
pub fn verify_color(color: &str) -> Result<Rgba, BadgeError> {
    match color.parse::<Rgba>() {
        Ok(c) => Ok(c),
        Err(_) => Err(BadgeError::ColorNotValid(String::from(color))),
    }
}

pub fn format_color(color: &Rgba) -> String {
    format!(
        "rgb({}, {}, {})",
        color.red * 255.0,
        color.green * 255.0,
        color.blue * 255.0
    )
}

pub fn rgb_to_xyz(color_item: f32) -> f32 {
    if color_item <= 0.03928 {
        color_item / 12.92
    } else {
        ((color_item + 0.055) / 1.055).powf(2.4)
    }
}

// From https://stackoverflow.com/a/75110271/2027612
// which uses the method from this answer: https://stackoverflow.com/a/3943023/2027612 but applies it
// using the CIE XYZ color space which is a better model for how the eyes perceive colors
pub fn get_accent_colors(background_color: &Rgba) -> AccentColors {
    let brightness = rgb_to_xyz(background_color.red) * 0.2126
        + rgb_to_xyz(background_color.green) * 0.7152
        + rgb_to_xyz(background_color.blue) * 0.0722;
    // Check if the background color requires light or dark text depending on the brightness of the color
    if brightness <= BRIGHTNESS_THRESHOLD {
        AccentColors {
            text_color: LIGHT_TEXT_COLOR,
            shadow_color: DARK_SHADOW_COLOR,
        }
    } else {
        AccentColors {
            text_color: DARK_TEXT_COLOR,
            shadow_color: LIGHT_SHADOW_COLOR,
        }
    }
}

// Thanks, Shepmaster.
// https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust
/// Make the first character of the given string be uppercase.
pub fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Create an embeddable logo from the given URI.
pub fn create_embedded_logo(logo_uri: &str) -> Result<String, BadgeError> {
    if let Ok(uri) = ureq::get(logo_uri).call() {
        Ok(uri.into_string().unwrap())
    } else {
        Err(BadgeError::CannotEmbedLogo(String::from(logo_uri)))
    }
}

/// Attempt to download a logo from a given URI. This can be a web URL or a local path.
pub fn attempt_logo_download(logo_uri: &str) -> Result<String, BadgeError> {
    // Check for local copy
    let local_path = Path::new(logo_uri);

    let data = match std::fs::read_to_string(local_path) {
        Ok(f) => f,
        Err(_) => create_embedded_logo(logo_uri)?,
    };

    // If not local, download
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(data.as_bytes())
    ))
}
