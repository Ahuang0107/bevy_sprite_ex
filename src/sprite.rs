use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_sprite::Anchor;

/// Specifies the rendering properties of a sprite.
///
/// This is commonly used as a component within [`SpriteBundle`](crate::bundle::SpriteExBundle).
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[repr(C)]
pub struct SpriteEx {
    /// The sprite's color tint
    pub color: Color,
    /// Flip the sprite along the `X` axis
    pub flip_x: bool,
    /// Flip the sprite along the `Y` axis
    pub flip_y: bool,
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image
    pub custom_size: Option<Vec2>,
    /// An optional rectangle representing the region of the sprite's image to render, instead of rendering
    /// the full image.
    pub rect: Option<Rect>,
    /// [`Anchor`] point of the sprite in the world
    pub anchor: Anchor,
    pub blend_mode: BlendMode,
    /// Order, decide if sprite will apply other sprite mask
    pub order: u32,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
#[reflect(Default)]
#[repr(C)]
pub enum BlendMode {
    #[default]
    Normal = 0,
    Darken = 10,
    Multiply = 11,
    ColorBurn = 12,
    Lighten = 20,
    Screen = 21,
    ColorDodge = 22,
    Addition = 23,
    Overlay = 30,
    SoftLight = 31,
    HardLight = 32,
    Difference = 40,
    Exclusion = 41,
    Subtract = 42,
    Divide = 43,
    Hue = 50,
    Saturation = 51,
    Color = 52,
    Luminosity = 53,
}

#[cfg(test)]
mod tests {
    use crate::BlendMode;

    #[test]
    fn test_blend_mode_enum_int() {
        let blend_mode: usize = BlendMode::SoftLight as usize;
        assert_eq!(
            31, blend_mode,
            "Something Wrong: BlendMode::SoftLight enum int not equals to 31."
        );
    }
}
