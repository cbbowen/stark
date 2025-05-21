use glam::*;
use palette::{convert::IntoColorUnclamped, Okhsl, Oklab, Srgb, Oklch};

trait ColorExt {
	fn to_css(&self) -> String;
}

impl ColorExt for Srgb {
	fn to_css(&self) -> String {
		let Srgb::<u8> { red, green, blue, .. } = self.into_format();
		format!("rgb({red} {green} {blue})")
	}
}

impl ColorExt for Oklab {
	fn to_css(&self) -> String {
		format!(
			"oklab({} {} {})",
			self.l.clamp(0.0, 1.0),
			self.a.clamp(-0.4, 0.4),
			self.b.clamp(-0.4, 0.4),
		)
	}
}

impl ColorExt for Okhsl {
	fn to_css(&self) -> String {
		let oklab: Oklab = self.clone().into_color_unclamped();
		oklab.to_css()
	}
}

impl ColorExt for Oklch {
	fn to_css(&self) -> String {
		let Oklch { l, chroma, hue } = self;
		format!("oklch({l} {chroma} {hue})", hue = hue.into_degrees())
	}
}