#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}

#import bevy_sprite_ex::sprite_view_bindings::view

struct VertexInput {
    @builtin(vertex_index) index: u32,
    // NOTE: Instance-rate vertex buffer members prefixed with i_
    // NOTE: i_model_transpose_colN are the 3 columns of a 3x4 matrix that is the transpose of the
    // affine 4x3 model matrix.
    @location(0) i_model_transpose_col0: vec4<f32>,
    @location(1) i_model_transpose_col1: vec4<f32>,
    @location(2) i_model_transpose_col2: vec4<f32>,
    @location(3) i_color: vec4<f32>,
    @location(4) i_uv_offset_scale: vec4<f32>,
    @location(5) blend_mode: i32,
    @location(6) _padding: vec2<i32>,
    @location(7) mask_count: i32,

#ifdef MASK
    @location(8) i_mask_0_model_transpose_col0: vec4<f32>,
    @location(9) i_mask_0_model_transpose_col1: vec4<f32>,
    @location(10) i_mask_0_model_transpose_col2: vec4<f32>,
    @location(11) i_mask_0_uv_offset_scale: vec4<f32>,

    @location(12) i_mask_1_model_transpose_col0: vec4<f32>,
    @location(13) i_mask_1_model_transpose_col1: vec4<f32>,
    @location(14) i_mask_1_model_transpose_col2: vec4<f32>,
    @location(15) i_mask_1_uv_offset_scale: vec4<f32>,
    
    @location(16) i_mask_2_model_transpose_col0: vec4<f32>,
    @location(17) i_mask_2_model_transpose_col1: vec4<f32>,
    @location(18) i_mask_2_model_transpose_col2: vec4<f32>,
    @location(19) i_mask_2_uv_offset_scale: vec4<f32>,
    
    @location(20) i_mask_3_model_transpose_col0: vec4<f32>,
    @location(21) i_mask_3_model_transpose_col1: vec4<f32>,
    @location(22) i_mask_3_model_transpose_col2: vec4<f32>,
    @location(23) i_mask_3_uv_offset_scale: vec4<f32>,
    
    @location(24) i_mask_4_model_transpose_col0: vec4<f32>,
    @location(25) i_mask_4_model_transpose_col1: vec4<f32>,
    @location(26) i_mask_4_model_transpose_col2: vec4<f32>,
    @location(27) i_mask_4_uv_offset_scale: vec4<f32>,
#endif
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) blend_mode: i32,
    @location(3) @interpolate(flat) mask_count: i32,

#ifdef MASK
    @location(4) mask_0_uv: vec2<f32>,
    @location(5) mask_1_uv: vec2<f32>,
    @location(6) mask_2_uv: vec2<f32>,
    @location(7) mask_3_uv: vec2<f32>,
    @location(8) mask_4_uv: vec2<f32>,
#endif
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_position = vec3<f32>(
        f32(in.index & 0x1u),
        f32((in.index & 0x2u) >> 1u),
        0.0
    );

    out.clip_position = view.clip_from_world * affine3_to_square(mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.uv = vec2<f32>(vertex_position.xy) * in.i_uv_offset_scale.zw + in.i_uv_offset_scale.xy;
    out.color = in.i_color;
    out.blend_mode = in.blend_mode;

    out.mask_count = in.mask_count;
#ifdef MASK
    let mask_0_position = affine3_to_square(mat3x4<f32>(
        in.i_mask_0_model_transpose_col0,
        in.i_mask_0_model_transpose_col1,
        in.i_mask_0_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.mask_0_uv = vec2<f32>(mask_0_position.xy) * in.i_mask_0_uv_offset_scale.zw + in.i_mask_0_uv_offset_scale.xy;

    if in.mask_count > 1 {
        let mask_1_position = affine3_to_square(mat3x4<f32>(
            in.i_mask_1_model_transpose_col0,
            in.i_mask_1_model_transpose_col1,
            in.i_mask_1_model_transpose_col2,
        )) * vec4<f32>(vertex_position, 1.0);
        out.mask_1_uv = vec2<f32>(mask_1_position.xy) * in.i_mask_1_uv_offset_scale.zw + in.i_mask_1_uv_offset_scale.xy;
    }
    
    if in.mask_count > 2 {
        let mask_2_position = affine3_to_square(mat3x4<f32>(
            in.i_mask_2_model_transpose_col0,
            in.i_mask_2_model_transpose_col1,
            in.i_mask_2_model_transpose_col2,
        )) * vec4<f32>(vertex_position, 1.0);
        out.mask_2_uv = vec2<f32>(mask_2_position.xy) * in.i_mask_2_uv_offset_scale.zw + in.i_mask_2_uv_offset_scale.xy;
    }
    
    if in.mask_count > 3 {
        let mask_3_position = affine3_to_square(mat3x4<f32>(
            in.i_mask_3_model_transpose_col0,
            in.i_mask_3_model_transpose_col1,
            in.i_mask_3_model_transpose_col2,
        )) * vec4<f32>(vertex_position, 1.0);
        out.mask_3_uv = vec2<f32>(mask_3_position.xy) * in.i_mask_3_uv_offset_scale.zw + in.i_mask_3_uv_offset_scale.xy;
    }

    if in.mask_count > 4 {
        let mask_4_position = affine3_to_square(mat3x4<f32>(
            in.i_mask_4_model_transpose_col0,
            in.i_mask_4_model_transpose_col1,
            in.i_mask_4_model_transpose_col2,
        )) * vec4<f32>(vertex_position, 1.0);
        out.mask_4_uv = vec2<f32>(mask_4_position.xy) * in.i_mask_4_uv_offset_scale.zw + in.i_mask_4_uv_offset_scale.xy;
    }
#endif

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

#ifdef MASK

@group(2) @binding(0) var mask_0_texture: texture_2d<f32>;
@group(2) @binding(1) var mask_0_sampler: sampler;

@group(3) @binding(0) var mask_1_texture: texture_2d<f32>;
@group(3) @binding(1) var mask_1_sampler: sampler;

@group(4) @binding(0) var mask_2_texture: texture_2d<f32>;
@group(4) @binding(1) var mask_2_sampler: sampler;

@group(5) @binding(0) var mask_3_texture: texture_2d<f32>;
@group(5) @binding(1) var mask_3_sampler: sampler;

@group(6) @binding(0) var mask_4_texture: texture_2d<f32>;
@group(6) @binding(1) var mask_4_sampler: sampler;

#endif

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);

    if (in.blend_mode != 0) {
        color = vec4<f32>(0, 0, 0, 1);
    }

#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

#ifdef MASK
    if in.mask_0_uv.x >= 0 && in.mask_0_uv.x <= 1 && in.mask_0_uv.y >= 0 && in.mask_0_uv.y <= 1 {
        var mask_texture = textureSample(mask_0_texture, mask_0_sampler, in.mask_0_uv);

        if mask_texture.x != 0.0 {
            color.a = 0.0;
        }
    }
    if in.mask_count > 1 {
        if in.mask_1_uv.x >= 0 && in.mask_1_uv.x <= 1 && in.mask_1_uv.y >= 0 && in.mask_1_uv.y <= 1 {
            var mask_texture = textureSample(mask_1_texture, mask_1_sampler, in.mask_1_uv);

            if mask_texture.x != 0.0 {
                color.a = 0.0;
            }
        }
    }
    if in.mask_count > 2 {
        if in.mask_2_uv.x >= 0 && in.mask_2_uv.x <= 1 && in.mask_2_uv.y >= 0 && in.mask_2_uv.y <= 1 {
            var mask_texture = textureSample(mask_2_texture, mask_2_sampler, in.mask_2_uv);

            if mask_texture.x != 0.0 {
                color.a = 0.0;
            }
        }
    }
    if in.mask_count > 3 {
        if in.mask_3_uv.x >= 0 && in.mask_3_uv.x <= 1 && in.mask_3_uv.y >= 0 && in.mask_3_uv.y <= 1 {
            var mask_texture = textureSample(mask_3_texture, mask_3_sampler, in.mask_3_uv);

            if mask_texture.x != 0.0 {
                color.a = 0.0;
            }
        }
    }
    if in.mask_count > 4 {
        if in.mask_4_uv.x >= 0 && in.mask_4_uv.x <= 1 && in.mask_4_uv.y >= 0 && in.mask_4_uv.y <= 1 {
            var mask_texture = textureSample(mask_4_texture, mask_4_sampler, in.mask_4_uv);

            if mask_texture.x != 0.0 {
                color.a = 0.0;
            }
        }
    }
#endif

    return color;
}
