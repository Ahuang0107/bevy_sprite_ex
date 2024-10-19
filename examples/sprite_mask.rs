use bevy::prelude::*;

use bevy_sprite_ex::{
    BlendMode, SpriteEx, SpriteExBundle, SpriteExPlugin, SpriteMask, SpriteMaskBundle,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SpriteExPlugin)
        .add_systems(Startup, (setup_camera, setup_sprites))
        .add_systems(FixedUpdate, (update_camera, update))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_scale(Vec3::splat(0.04)),
        ..default()
    });
}

fn setup_sprites(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 表示最底层 sprite，为了方便观察上层 sprite 的显示情况，同时也是为了测试 mask 范围外的 sprite 的显示情况
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite02.png"),
        transform: Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(2.0)),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 1,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(-8.0, -8.0, 2.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 2,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(8.0, -8.0, 3.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 3,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(8.0, 8.0, 4.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 4,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(-8.0, 8.0, 5.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 5,
            ..default()
        },
        ..default()
    });
    commands.spawn((
        SpriteMaskBundle {
            texture: asset_server.load("mask01.png"),
            sprite_mask: SpriteMask {
                range_start: 1,
                range_end: 5,
                ..default()
            },
            ..default()
        },
        MaskKey(1),
    ));
    commands.spawn((
        SpriteMaskBundle {
            texture: asset_server.load("mask02.png"),
            sprite_mask: SpriteMask {
                range_start: 2,
                range_end: 5,
                ..default()
            },
            ..default()
        },
        MaskKey(2),
    ));
    commands.spawn((
        SpriteMaskBundle {
            texture: asset_server.load("mask03.png"),
            sprite_mask: SpriteMask {
                range_start: 3,
                range_end: 5,
                ..default()
            },
            ..default()
        },
        MaskKey(3),
    ));
    commands.spawn((
        SpriteMaskBundle {
            texture: asset_server.load("mask04.png"),
            sprite_mask: SpriteMask {
                range_start: 4,
                range_end: 5,
                ..default()
            },
            ..default()
        },
        MaskKey(4),
    ));
    commands.spawn((
        SpriteMaskBundle {
            texture: asset_server.load("mask05.png"),
            sprite_mask: SpriteMask {
                range_start: 5,
                range_end: 5,
                ..default()
            },
            ..default()
        },
        MaskKey(5),
    ));
}

#[derive(Component)]
struct MaskKey(usize);

impl MaskKey {
    fn key_code(&self) -> KeyCode {
        match self.0 {
            1 => KeyCode::Digit1,
            2 => KeyCode::Digit2,
            3 => KeyCode::Digit3,
            4 => KeyCode::Digit4,
            5 => KeyCode::Digit5,
            _ => KeyCode::Space,
        }
    }
}

const STEP: f32 = 0.05;

fn update_camera(
    mut camera: Query<&mut Transform, With<Camera>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for mut transform in camera.iter_mut() {
        if input.pressed(KeyCode::ControlLeft) {
            transform.scale.x += STEP;
            transform.scale.y += STEP;
        }
        if input.pressed(KeyCode::AltLeft) {
            transform.scale.x -= STEP;
            transform.scale.y -= STEP;
        }
    }
}

fn update(
    mut sprite_mask_position: Query<(&mut Transform, &MaskKey)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::ArrowLeft) {
        for (mut transform, mask_key) in sprite_mask_position.iter_mut() {
            if input.pressed(mask_key.key_code()) {
                transform.translation.x -= STEP;
            }
        }
    }
    if input.pressed(KeyCode::ArrowRight) {
        for (mut transform, mask_key) in sprite_mask_position.iter_mut() {
            if input.pressed(mask_key.key_code()) {
                transform.translation.x += STEP;
            }
        }
    }
    if input.pressed(KeyCode::ArrowUp) {
        for (mut transform, mask_key) in sprite_mask_position.iter_mut() {
            if input.pressed(mask_key.key_code()) {
                transform.translation.y += STEP;
            }
        }
    }
    if input.pressed(KeyCode::ArrowDown) {
        for (mut transform, mask_key) in sprite_mask_position.iter_mut() {
            if input.pressed(mask_key.key_code()) {
                transform.translation.y -= STEP;
            }
        }
    }
}
