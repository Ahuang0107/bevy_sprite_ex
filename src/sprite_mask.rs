use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_sprite::Anchor;

/// Specifies the rendering properties of a sprite mask.
///
/// This is commonly used as a component within [`SpriteMaskBundle`](crate::bundle::SpriteMaskBundle).
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[repr(C)]
pub struct SpriteMask {
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
}
