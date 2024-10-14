use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_render::{
    mesh::Mesh,
    primitives::Aabb,
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    texture::Image,
    view::{check_visibility, NoFrustumCulling, VisibilitySystems},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_sprite::{
    queue_material2d_meshes, ColorMaterial, ColorMaterialPlugin, Mesh2dHandle, Mesh2dRenderPlugin,
};

pub use render::*;
pub use sprite::*;

mod bundle;
mod render;
mod sprite;

/// Adds support for 2D sprite rendering.
#[derive(Default)]
pub struct SpriteExPlugin;

pub const SPRITE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(8793537950464524391);
pub const SPRITE_VIEW_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(4597317399397146678);

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystem {
    ExtractSprites,
    ComputeSlices,
}

/// A convenient alias for `With<Mesh2dHandle>>`, for use with
/// [`bevy_render::view::VisibleEntities`].
pub type WithMesh2d = With<Mesh2dHandle>;

/// A convenient alias for `With<Sprite>`, for use with
/// [`bevy_render::view::VisibleEntities`].
pub type WithSprite = With<Sprite>;

impl Plugin for SpriteExPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SPRITE_SHADER_HANDLE,
            "render/sprite.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SPRITE_VIEW_BINDINGS_SHADER_HANDLE,
            "render/sprite_view_bindings.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Sprite>()
            .register_type::<Anchor>()
            .register_type::<Mesh2dHandle>()
            .add_plugins((Mesh2dRenderPlugin, ColorMaterialPlugin))
            .add_systems(
                PostUpdate,
                (
                    calculate_bounds_2d.in_set(VisibilitySystems::CalculateBounds),
                    (
                        check_visibility::<WithMesh2d>,
                        check_visibility::<WithSprite>,
                    )
                        .in_set(VisibilitySystems::CheckVisibility),
                ),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_sprites.in_set(SpriteSystem::ExtractSprites),
                        extract_sprite_events,
                    ),
                )
                .add_systems(
                    Render,
                    (
                        queue_sprites
                            .in_set(RenderSet::Queue)
                            .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                        prepare_sprite_image_bind_groups.in_set(RenderSet::PrepareBindGroups),
                        prepare_sprite_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        };
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<SpritePipeline>();
        }
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with either:
/// - a `Mesh2dHandle` component,
/// - a `Sprite` and `Handle<Image>` components,
/// and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    _images: Res<Assets<Image>>,
    meshes_without_aabb: Query<(Entity, &Mesh2dHandle), (Without<Aabb>, Without<NoFrustumCulling>)>,
    sprites_to_recalculate_aabb: Query<
        (Entity, &Sprite, &Handle<Image>),
        (
            Or<(Without<Aabb>, Changed<Sprite>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, mesh_handle) in &meshes_without_aabb {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(aabb) = mesh.compute_aabb() {
                commands.entity(entity).try_insert(aabb);
            }
        }
    }
    for (entity, sprite, _texture_handle) in &sprites_to_recalculate_aabb {
        if let Some(size) = sprite
            .custom_size
            .or_else(|| sprite.rect.map(|rect| rect.size()))
        {
            let aabb = Aabb {
                center: (-sprite.anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).try_insert(aabb);
        }
    }
}
