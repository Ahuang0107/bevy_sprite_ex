use bevy::prelude::*;

use bevy_sprite_ex::{
    BlendMode, SpriteEx, SpriteExBundle, SpriteExPlugin, SpriteMask, SpriteMaskBundle,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SpriteExPlugin)
        .add_systems(Startup, (setup_camera, setup_sprites))
        .add_systems(Update, update)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_scale(Vec3::splat(0.02)),
        ..default()
    });
}

fn setup_sprites(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SpriteExBundle {
        texture: asset_server.load("icon.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        sprite: SpriteEx {
            blend_mode: BlendMode::Normal,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteMaskBundle {
        texture: asset_server.load("icon.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}

fn update(
    mut sprite_mask_position: Query<&mut Transform, With<SpriteMask>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::ArrowLeft) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.x -= 1.0;
        }
    }
    if input.pressed(KeyCode::ArrowRight) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.x += 1.0;
        }
    }
    if input.pressed(KeyCode::ArrowUp) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.y += 1.0;
        }
    }
    if input.pressed(KeyCode::ArrowDown) {
        for mut transform in sprite_mask_position.iter_mut() {
            transform.translation.y -= 1.0;
        }
    }
}
