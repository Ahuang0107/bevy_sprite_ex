use std::ops::Range;

use bevy_asset::{AssetEvent, AssetId, Handle};
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_core_pipeline::{
    core_2d::Transparent2d,
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, DebandDither, Tonemapping,
        TonemappingLuts,
    },
};
use bevy_ecs::{entity::EntityHashMap, query::ROQueryItem};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3A, FloatOrd, Quat, Rect, Vec2, Vec4};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{
        BevyDefault, DefaultImageSampler, FallbackImage, GpuImage, Image, ImageSampler,
        TextureFormatPixelInfo,
    },
    view::{
        ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
        ViewVisibility, VisibleEntities,
    },
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

use crate::{BlendMode, SpriteEx, SpriteMask, WithSprite, SPRITE_SHADER_HANDLE};

#[derive(Resource)]
pub struct SpriteExPipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    mask_material_layout: BindGroupLayout,
    #[allow(dead_code)]
    dummy_white_gpu_image: GpuImage,
}

impl FromWorld for SpriteExPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
        let view_layout = render_device.create_bind_group_layout(
            "sprite_view_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    (0, uniform_buffer::<ViewUniform>(true)),
                    (
                        1,
                        tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                    ),
                    (
                        2,
                        tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
                    ),
                ),
            ),
        );

        let material_layout = render_device.create_bind_group_layout(
            "sprite_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let mask_material_layout = render_device.create_bind_group_layout(
            "sprite_mask_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.size(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpriteExPipeline {
            view_layout,
            material_layout,
            mask_material_layout,
            dummy_white_gpu_image,
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct SpritePipelineKey: u32 {
        const NONE                              = 0;
        const HDR                               = 1 << 0;
        const TONEMAP_IN_SHADER                 = 1 << 1;
        const DEBAND_DITHER                     = 1 << 2;
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_BLENDER_FILMIC     = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const MASK_RESERVED_BITS                = Self::MASK_MASK_BITS << Self::MASK_SHIFT_BITS;
        const MASK_ENABLED                      = 1 << Self::MASK_SHIFT_BITS;
    }
}

impl SpritePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();
    const MASK_MASK_BITS: u32 = 0b11;
    const MASK_SHIFT_BITS: u32 =
        Self::TONEMAP_METHOD_SHIFT_BITS - Self::MASK_MASK_BITS.count_ones();

    #[inline]
    pub const fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    #[inline]
    pub const fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    #[inline]
    pub const fn from_hdr(hdr: bool) -> Self {
        if hdr {
            SpritePipelineKey::HDR
        } else {
            SpritePipelineKey::NONE
        }
    }
}

impl SpecializedRenderPipeline for SpriteExPipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.contains(SpritePipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                1,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                2,
            ));

            let method = key.intersection(SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == SpritePipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
            {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let mask_enable = key.contains(SpritePipelineKey::MASK_ENABLED);

        if mask_enable {
            shader_defs.push("MASK".into());
        }

        let format = match key.contains(SpritePipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let instance_rate_vertex_buffer_layout = {
            let mut array_stride = 96;
            let mut attributes = vec![
                // @location(0) i_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) i_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // @location(2) i_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                // @location(3) i_color: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 64,
                    shader_location: 4,
                },
                // @location(5) blend_mode: i32,
                VertexAttribute {
                    format: VertexFormat::Sint32,
                    offset: 80,
                    shader_location: 5,
                },
                // @location(6) _padding: vec3<i32>,
                VertexAttribute {
                    format: VertexFormat::Sint32x3,
                    offset: 84,
                    shader_location: 6,
                },
            ];

            if mask_enable {
                array_stride += 64;
                attributes.append(&mut vec![
                    // @location(7) i_mask_model_transpose_col0: vec4<f32>,
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 96,
                        shader_location: 7,
                    },
                    // @location(8) i_mask_model_transpose_col1: vec4<f32>,
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 112,
                        shader_location: 8,
                    },
                    // @location(9) i_mask_model_transpose_col2: vec4<f32>,
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 128,
                        shader_location: 9,
                    },
                    // @location(10) i_mask_uv_offset_scale: vec4<f32>,
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 144,
                        shader_location: 10,
                    },
                ])
            }

            VertexBufferLayout {
                array_stride,
                step_mode: VertexStepMode::Instance,
                attributes,
            }
        };

        let mut pipeline_layout = vec![self.view_layout.clone(), self.material_layout.clone()];

        if mask_enable {
            pipeline_layout.push(self.mask_material_layout.clone());
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: pipeline_layout,
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("sprite_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ExtractedSprite {
    pub transform: GlobalTransform,
    pub color: LinearRgba,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the sprite
    pub custom_size: Option<Vec2>,
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,
    /// For cases where additional [`ExtractedSprites`] are created during extraction, this stores the
    /// entity that caused that creation for use in determining visibility.
    pub original_entity: Option<Entity>,
    pub blend_mode: BlendMode,
    pub order: u32,
}

impl ExtractedSprite {
    fn calculate_transform(&self, image_size: &Vec2) -> Affine3A {
        calculate_transform(
            image_size,
            &self.custom_size,
            &self.rect,
            &self.transform,
            &self.anchor,
        )
    }
    fn calculate_uv_offset_scale(&self, image_size: &Vec2) -> Vec4 {
        calculate_uv_offset_scale(image_size, &self.rect, self.flip_x, self.flip_y)
    }
}

#[derive(Debug)]
pub struct ExtractedSpriteMask {
    pub transform: GlobalTransform,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the mask
    pub custom_size: Option<Vec2>,
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSpriteMask` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,
    pub range_start: u32,
    pub range_end: u32,
}

impl ExtractedSpriteMask {
    fn calculate_transform(&self, image_size: &Vec2) -> Affine3A {
        calculate_transform(
            image_size,
            &self.custom_size,
            &self.rect,
            &self.transform,
            &self.anchor,
        )
    }
    fn calculate_uv_offset_scale(&self, image_size: &Vec2) -> Vec4 {
        calculate_uv_offset_scale(image_size, &self.rect, self.flip_x, self.flip_y)
    }
}

fn calculate_transform(
    image_size: &Vec2,
    custom_size: &Option<Vec2>,
    rect: &Option<Rect>,
    transform: &GlobalTransform,
    anchor: &Vec2,
) -> Affine3A {
    // By default, the size of the quad is the size of the texture, but `rect` or `custom_size` will overwrite
    let quad_size = custom_size.unwrap_or_else(|| rect.map(|r| r.size()).unwrap_or(*image_size));

    transform.affine()
        * Affine3A::from_scale_rotation_translation(
            quad_size.extend(1.0),
            Quat::IDENTITY,
            (quad_size * (-*anchor - Vec2::splat(0.5))).extend(0.0),
        )
}

/// Calculate vertex data for this item
fn calculate_uv_offset_scale(
    image_size: &Vec2,
    rect: &Option<Rect>,
    flip_x: bool,
    flip_y: bool,
) -> Vec4 {
    let mut uv_offset_scale: Vec4;

    // If a rect is specified, adjust UVs and the size of the quad
    if let Some(rect) = rect {
        let rect_size = rect.size();
        uv_offset_scale = Vec4::new(
            rect.min.x / image_size.x,
            rect.max.y / image_size.y,
            rect_size.x / image_size.x,
            -rect_size.y / image_size.y,
        );
    } else {
        uv_offset_scale = Vec4::new(0.0, 1.0, 1.0, -1.0);
    }

    if flip_x {
        uv_offset_scale.x += uv_offset_scale.z;
        uv_offset_scale.z *= -1.0;
    }
    if flip_y {
        uv_offset_scale.y += uv_offset_scale.w;
        uv_offset_scale.w *= -1.0;
    }

    uv_offset_scale
}

#[derive(Resource, Default)]
pub struct ExtractedSprites {
    pub sprites: EntityHashMap<ExtractedSprite>,
    pub masks: EntityHashMap<ExtractedSpriteMask>,
    pub mask_uniform_count: usize,
}

impl ExtractedSprites {
    fn clear(&mut self) {
        self.sprites.clear();
        self.masks.clear();
        self.mask_uniform_count = 0;
    }
}

#[derive(Resource, Default)]
pub struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_sprite_events(
    mut events: ResMut<SpriteAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for event in image_events.read() {
        images.push(*event);
    }
}

pub fn extract_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    sprite_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &SpriteEx,
            &GlobalTransform,
            &Handle<Image>,
        )>,
    >,
    sprite_mask_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &SpriteMask,
            &GlobalTransform,
            &Handle<Image>,
        )>,
    >,
) {
    extracted_sprites.clear();

    for (entity, view_visibility, sprite, transform, handle) in sprite_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let rect = sprite.rect;

        // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
        extracted_sprites.sprites.insert(
            entity,
            ExtractedSprite {
                color: sprite.color.into(),
                transform: *transform,
                rect,
                // Pass the custom size
                custom_size: sprite.custom_size,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: handle.id(),
                anchor: sprite.anchor.as_vec(),
                original_entity: None,
                blend_mode: sprite.blend_mode,
                order: sprite.order,
            },
        );
    }

    for (entity, view_visibility, sprite_mask, transform, handle) in sprite_mask_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let rect = sprite_mask.rect;

        extracted_sprites.masks.insert(
            entity,
            ExtractedSpriteMask {
                transform: *transform,
                rect,
                custom_size: sprite_mask.custom_size,
                image_handle_id: handle.id(),
                flip_x: sprite_mask.flip_x,
                flip_y: sprite_mask.flip_y,
                anchor: sprite_mask.anchor.as_vec(),
                range_start: sprite_mask.range_start,
                range_end: sprite_mask.range_end,
            },
        );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct SpriteInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub i_color: [f32; 4],
    pub i_uv_offset_scale: [f32; 4],
    pub blend_mode: i32,
    // 原来的几个变量都是 4*4 字节的倍数（i_model_transpose 是 [[f32;4];3]）
    // 所以加了 blend_mode 后还得在加一个 _padding 确保依旧是 4*4 字节的倍数
    pub _padding: [i32; 3],
}

impl SpriteInstance {
    #[inline]
    fn from(
        transform: &Affine3A,
        color: &LinearRgba,
        uv_offset_scale: &Vec4,
        blend_mode: BlendMode,
    ) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_color: color.to_f32_array(),
            i_uv_offset_scale: uv_offset_scale.to_array(),
            blend_mode: blend_mode as i32,
            _padding: [0, 0, 0],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct MaskedSpriteInstance {
    pub sprite: SpriteInstance,
    // Affine 4x3 transposed to 3x4
    pub i_mask_model_transpose: [Vec4; 3],
    pub i_mask_uv_offset_scale: [f32; 4],
}

impl MaskedSpriteInstance {
    #[inline]
    fn from(
        sprite_instance: SpriteInstance,
        mask_transform: &Affine3A,
        mask_uv_offset_scale: &Vec4,
    ) -> Self {
        let mask_transpose_model_3x3 = mask_transform.matrix3.transpose();
        Self {
            sprite: sprite_instance,
            i_mask_model_transpose: [
                mask_transpose_model_3x3
                    .x_axis
                    .extend(mask_transform.translation.x),
                mask_transpose_model_3x3
                    .y_axis
                    .extend(mask_transform.translation.y),
                mask_transpose_model_3x3
                    .z_axis
                    .extend(mask_transform.translation.z),
            ],
            i_mask_uv_offset_scale: mask_uv_offset_scale.to_array(),
        }
    }
}

#[derive(Resource)]
pub struct SpriteMeta {
    sprite_index_buffer: RawBufferVec<u32>,
    sprite_instance_buffer: RawBufferVec<SpriteInstance>,
    masked_sprite_instance_buffer: RawBufferVec<MaskedSpriteInstance>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            sprite_index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: RawBufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
            masked_sprite_instance_buffer: RawBufferVec::<MaskedSpriteInstance>::new(
                BufferUsages::VERTEX,
            ),
        }
    }
}

impl SpriteMeta {
    fn clear(&mut self) {
        self.sprite_index_buffer.clear();
        self.sprite_instance_buffer.clear();
        self.masked_sprite_instance_buffer.clear();
    }
}

#[derive(Component)]
pub struct SpriteViewBindGroup {
    pub value: BindGroup,
}

#[derive(Component, PartialEq, Eq, Clone)]
pub struct SpriteBatch {
    image_handle_id: AssetId<Image>,
    range: Range<u32>,
    mask_image_handle_id: Option<AssetId<Image>>,
}

#[derive(Resource, Default)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
    mask_values: HashMap<AssetId<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    sprite_pipeline: Res<SpriteExPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpriteExPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    extracted_sprites: Res<ExtractedSprites>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(
        Entity,
        &VisibleEntities,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());

    let draw_sprite_function = draw_functions.read().id::<DrawSprite>();

    for (view_entity, visible_entities, view, tonemapping, dither) in &mut views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= SpritePipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => SpritePipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => SpritePipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => SpritePipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= SpritePipelineKey::DEBAND_DITHER;
            }
        }

        let unmasked_sprite_pipeline =
            pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key);
        let masked_sprite_pipeline = pipelines.specialize(
            &pipeline_cache,
            &sprite_pipeline,
            view_key | SpritePipelineKey::MASK_ENABLED,
        );

        view_entities.clear();
        view_entities.extend(
            visible_entities
                .iter::<WithSprite>()
                .map(|e| e.index() as usize),
        );

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (entity, extracted_sprite) in extracted_sprites.sprites.iter() {
            let index = extracted_sprite.original_entity.unwrap_or(*entity).index();

            if !view_entities.contains(index as usize) {
                continue;
            }

            // 这里只是根据 order 判断是否有 sprite mask 应用到了 extracted_sprite 身上，从而决定使用哪条管线
            let mut enable_mask = false;
            for (_, extracted_sprite_mask) in extracted_sprites.masks.iter() {
                if extracted_sprite.order >= extracted_sprite_mask.range_start
                    && extracted_sprite.order <= extracted_sprite_mask.range_end
                {
                    enable_mask = true;
                    break;
                }
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline: if enable_mask {
                    masked_sprite_pipeline
                } else {
                    unmasked_sprite_pipeline
                },
                entity: *entity,
                sort_key,
                // batch_range and dynamic_offset will be calculated in prepare_sprites
                batch_range: 0..0,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_sprite_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    sprite_pipeline: Res<SpriteExPipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), With<ExtractedView>>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &sprite_pipeline.view_layout,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, lut_bindings.0),
                (2, lut_bindings.1),
            )),
        );

        commands.entity(entity).insert(SpriteViewBindGroup {
            value: view_bind_group,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_sprite_image_bind_groups(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    sprite_pipeline: Res<SpriteExPipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    extracted_sprites: Res<ExtractedSprites>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Unused { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
                image_bind_groups.mask_values.remove(id);
            }
        };
    }

    let mut batches: Vec<(Entity, SpriteBatch)> = Vec::with_capacity(*previous_len);

    // Clear the sprite instances
    sprite_meta.clear();

    // Index buffer indices
    let mut unmasked_index = 0;
    let mut masked_index = 0;

    let image_bind_groups = &mut *image_bind_groups;

    for transparent_phase in phases.values_mut() {
        let mut batch_item_index = 0;
        let mut batch_image_size = Vec2::ZERO;
        let mut batch_image_handle = AssetId::invalid();

        let mut batch_mask_image_size = Vec2::ZERO;
        let mut batch_mask_handle = None;

        // Iterate through the phase items and detect when successive sprites that can be batched.
        // Spawn an entity with a `SpriteBatch` component for each possible batch.
        // Compatible items share the same entity.
        for item_index in 0..transparent_phase.items.len() {
            let item = &transparent_phase.items[item_index];
            let Some(extracted_sprite) = extracted_sprites.sprites.get(&item.entity) else {
                // If there is a phase item that is not a sprite, then we must start a new
                // batch to draw the other phase item(s) and to respect draw order. This can be
                // done by invalidating the batch_image_handle
                batch_image_handle = AssetId::invalid();
                continue;
            };

            let batch_image_changed = batch_image_handle != extracted_sprite.image_handle_id;
            if batch_image_changed {
                let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                    continue;
                };

                batch_image_size = gpu_image.size.as_vec2();
                batch_image_handle = extracted_sprite.image_handle_id;
                image_bind_groups
                    .values
                    .entry(batch_image_handle)
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &sprite_pipeline.material_layout,
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                            )),
                        )
                    });
            }

            // TODO 目前这里只支持应用一个 mask
            let mut extracted_mask = None;
            for (_, extracted_sprite_mask) in extracted_sprites.masks.iter() {
                if extracted_sprite.order >= extracted_sprite_mask.range_start
                    && extracted_sprite.order <= extracted_sprite_mask.range_end
                {
                    extracted_mask = Some(extracted_sprite_mask);
                    break;
                }
            }
            let mask_asset = extracted_mask.map(|m| m.image_handle_id);

            let batch_mask_changed = batch_mask_handle != mask_asset;

            if batch_mask_changed {
                if let (Some(extracted_mask), Some(mask_asset)) = (extracted_mask, mask_asset) {
                    let Some(gpu_image) = gpu_images.get(extracted_mask.image_handle_id) else {
                        continue;
                    };

                    batch_mask_image_size = gpu_image.size.as_vec2();

                    image_bind_groups
                        .mask_values
                        .entry(mask_asset)
                        .or_insert_with(|| {
                            render_device.create_bind_group(
                                "sprite_mask_material_bind_group",
                                &sprite_pipeline.mask_material_layout,
                                &BindGroupEntries::sequential((
                                    &gpu_image.texture_view,
                                    &gpu_image.sampler,
                                )),
                            )
                        });
                }

                batch_mask_handle = mask_asset;
            }

            let sprite_transform = extracted_sprite.calculate_transform(&batch_image_size);
            let sprite_uv_offset_scale =
                extracted_sprite.calculate_uv_offset_scale(&batch_image_size);

            let sprite_instance = SpriteInstance::from(
                &sprite_transform,
                &extracted_sprite.color,
                &sprite_uv_offset_scale,
                extracted_sprite.blend_mode,
            );

            // Store the vertex data and add the item to the render phase
            let index = if let Some(extracted_mask) = extracted_mask {
                let mask_transform = extracted_mask
                    .calculate_transform(&batch_mask_image_size)
                    .inverse()
                    * sprite_transform;
                let mask_uv_offset_scale =
                    extracted_mask.calculate_uv_offset_scale(&batch_mask_image_size);
                let masked_sprite_instance = MaskedSpriteInstance::from(
                    sprite_instance,
                    &mask_transform,
                    &mask_uv_offset_scale,
                );

                sprite_meta
                    .masked_sprite_instance_buffer
                    .push(masked_sprite_instance);

                &mut masked_index
            } else {
                sprite_meta.sprite_instance_buffer.push(sprite_instance);

                &mut unmasked_index
            };

            if batch_image_changed || batch_mask_changed {
                batch_item_index = item_index;

                let mask_image_handle_id = extracted_mask.map(|em| em.image_handle_id);

                batches.push((
                    item.entity,
                    SpriteBatch {
                        image_handle_id: batch_image_handle,
                        range: *index..*index,
                        mask_image_handle_id,
                    },
                ));
            }

            transparent_phase.items[batch_item_index]
                .batch_range_mut()
                .end += 1;
            batches.last_mut().unwrap().1.range.end += 1;
            *index += 1;
        }
    }
    sprite_meta
        .sprite_instance_buffer
        .write_buffer(&render_device, &render_queue);

    sprite_meta
        .masked_sprite_instance_buffer
        .write_buffer(&render_device, &render_queue);

    if sprite_meta.sprite_index_buffer.len() != 6 {
        sprite_meta.sprite_index_buffer.clear();

        // NOTE: This code is creating 6 indices pointing to 4 vertices.
        // The vertices form the corners of a quad based on their two least significant bits.
        // 10   11
        //
        // 00   01
        // The sprite shader can then use the two least significant bits as the vertex index.
        // The rest of the properties to transform the vertex positions and UVs (which are
        // implicit) are baked into the instance transform, and UV offset and scale.
        // See bevy_sprite/src/render/sprite.wgsl for the details.
        sprite_meta.sprite_index_buffer.push(2);
        sprite_meta.sprite_index_buffer.push(0);
        sprite_meta.sprite_index_buffer.push(1);
        sprite_meta.sprite_index_buffer.push(1);
        sprite_meta.sprite_index_buffer.push(3);
        sprite_meta.sprite_index_buffer.push(2);

        sprite_meta
            .sprite_index_buffer
            .write_buffer(&render_device, &render_queue);
    }

    *previous_len = batches.len();
    commands.insert_or_spawn_batch(batches);
}

/// [`RenderCommand`] for sprite rendering.
pub type DrawSprite = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteTextureBindGroup<1>,
    SetSpriteMaskTextureBindGroup<2>,
    DrawSpriteBatch,
);

pub struct SetSpriteViewBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<SpriteViewBindGroup>);
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        (view_uniform, sprite_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &sprite_view_bind_group.value, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

pub struct SetSpriteTextureBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'_ SpriteBatch>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        pass.set_bind_group(
            I,
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub struct SetSpriteMaskTextureBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteMaskTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'_ SpriteBatch>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

        if let Some(mask_image_handle_id) = &batch.unwrap().mask_image_handle_id {
            pass.set_bind_group(
                I,
                image_bind_groups
                    .mask_values
                    .get(mask_image_handle_id)
                    .unwrap(),
                &[],
            );
        }

        RenderCommandResult::Success
    }
}

pub struct DrawSpriteBatch;

impl<P: PhaseItem> RenderCommand<P> for DrawSpriteBatch {
    type Param = SRes<SpriteMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'_ SpriteBatch>,
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );

        let buffer = if batch.mask_image_handle_id.is_some() {
            sprite_meta.masked_sprite_instance_buffer.buffer()
        } else {
            sprite_meta.sprite_instance_buffer.buffer()
        };
        pass.set_vertex_buffer(0, buffer.unwrap().slice(..));
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}
