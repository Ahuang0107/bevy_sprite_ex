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
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(-8.0, 0.0, 1.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 1,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("sprite01.png"),
        transform: Transform::from_xyz(8.0, 0.0, 10.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            order: 10,
            ..default()
        },
        ..default()
    });
    // sprite mask 只会应用在第一个 sprite 上，而不会应用在第二个 sprite 上
    // 同时因为现在只支持一个 mask，所以 mask02 会生效而 mask01 不会生效
    commands.spawn(SpriteMaskBundle {
        texture: asset_server.load("mask01.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        sprite_mask: SpriteMask {
            range_start: 0,
            range_end: 5,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteMaskBundle {
        texture: asset_server.load("mask02.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        sprite_mask: SpriteMask {
            range_start: 0,
            range_end: 5,
            ..default()
        },
        ..default()
    });
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
    mut sprite_mask_position: Query<&mut Transform, With<SpriteMask>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::ArrowLeft) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.x -= STEP;
        }
    }
    if input.pressed(KeyCode::ArrowRight) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.x += STEP;
        }
    }
    if input.pressed(KeyCode::ArrowUp) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.y += STEP;
        }
    }
    if input.pressed(KeyCode::ArrowDown) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.y -= STEP;
        }
    }
}
