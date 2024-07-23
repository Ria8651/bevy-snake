use crate::{
    board::{Board, Cell},
    game::{SnakeInputs, TickTimer},
    GameState, Settings,
};
use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    utils::HashMap,
};

pub struct BoardRenderPlugin;

impl Plugin for BoardRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, draw_board.run_if(in_state(GameState::InGame)));
    }
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource)]
struct RenderResources {
    apple_texture: Handle<Image>,
    circle_mesh: Handle<Mesh>,
    square_mesh: Handle<Mesh>,
    snake_materials: Vec<Handle<ColorMaterial>>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 500.0),
            ..default()
        },
        MainCamera,
    ));

    commands.insert_resource(RenderResources {
        apple_texture: asset_server.load("images/apple.png"),
        // capsule_mesh: meshes.add(Capsule2d::new(0.35, 1.0)),
        circle_mesh: meshes.add(Circle::new(0.35)),
        square_mesh: meshes.add(Rectangle::from_size(Vec2::new(0.7, 1.0))),
        snake_materials: vec![
            materials.add(Color::srgb(0.0, 0.7, 0.25)),
            materials.add(Color::srgb(0.3, 0.4, 0.7)),
            materials.add(Color::srgb(0.7, 0.4, 0.3)),
            materials.add(Color::srgb(0.7, 0.7, 0.7)),
        ],
    });
}

#[derive(Component)]
struct BoardTile;

#[derive(Component)]
struct SnakePart;

#[derive(Component)]
struct DebugTile;

fn draw_board(
    mut commands: Commands,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera>>,
    mut board_size: Local<(usize, usize)>,
    mut apples: Local<HashMap<IVec2, Entity>>,
    mut walls: Local<HashMap<IVec2, Entity>>,
    board: Res<Board>,
    input_queues: Res<SnakeInputs>,
    board_tiles: Query<Entity, With<BoardTile>>,
    snake_parts: Query<Entity, With<SnakePart>>,
    debug_tiles: Query<Entity, With<DebugTile>>,
    render_resources: Res<RenderResources>,
    tick_timer: Res<TickTimer>,
    settings: Res<Settings>,
) {
    let board_pos = |pos: Vec2, depth: f32| -> Transform {
        Transform::from_xyz(
            pos.x - board.width() as f32 / 2.0 + 0.5,
            pos.y - board.height() as f32 / 2.0 + 0.5,
            depth,
        )
    };

    // background
    if (board.width(), board.height()) != *board_size {
        for tile in board_tiles.iter() {
            commands.entity(tile).despawn();
        }

        let mut camera_projection = camera_query.single_mut();
        camera_projection.scaling_mode = ScalingMode::AutoMin {
            min_height: board.height() as f32,
            min_width: board.width() as f32,
        };

        for x in 0..board.width() {
            for y in 0..board.height() {
                let color = if (x + y) % 2 == 0 {
                    Color::srgb(0.3, 0.5, 0.3)
                } else {
                    Color::srgb(0.25, 0.45, 0.25)
                };

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite { color, ..default() },
                        transform: board_pos(Vec2::new(x as f32, y as f32), -10.0),
                        ..default()
                    },
                    BoardTile,
                ));
            }
        }

        for (_, &entity) in apples.iter() {
            commands.entity(entity).despawn();
        }
        apples.clear();

        for (_, &entity) in walls.iter() {
            commands.entity(entity).despawn();
        }
        walls.clear();

        *board_size = (board.width(), board.height());
    }

    // apples
    for (pos, cell) in board.cells() {
        match cell {
            Cell::Apple => {
                if apples.contains_key(&pos) {
                    continue;
                }

                let bundle = SpriteBundle {
                    texture: render_resources.apple_texture.clone(),
                    transform: board_pos(pos.as_vec2(), 10.0).with_scale(Vec3::splat(1.0 / 512.0)),
                    ..default()
                };
                apples.insert(pos, commands.spawn(bundle).id());
            }
            _ => {
                if let Some(entity) = apples.remove(&pos) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // walls
    for (pos, cell) in board.cells() {
        match cell {
            Cell::Wall => {
                if walls.contains_key(&pos) {
                    continue;
                }

                let bundle = SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(0.1, 0.1, 0.1),
                        ..default()
                    },
                    transform: board_pos(pos.as_vec2(), 5.0),
                    ..default()
                };
                walls.insert(pos, commands.spawn(bundle).id());
            }
            _ => {
                if let Some(entity) = walls.remove(&pos) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // snakes
    for entity in snake_parts.iter() {
        commands.entity(entity).despawn();
    }

    let mut interpolation = tick_timer.elapsed_secs() / tick_timer.duration().as_secs_f32();
    interpolation *= settings.interpolation as u32 as f32;

    for (snake_id, snake) in board.snakes().into_iter().enumerate() {
        if snake.len() < 2 {
            continue;
        }

        let mut snake: Vec<_> = snake.iter().map(|(pos, _)| pos.as_vec2()).collect();

        let n = snake.len() - 2; // neck
        let h = snake.len() - 1; // head
        let next_input = input_queues
            .get(snake_id)
            .and_then(|q| q.input_queue.get(0))
            .map(|d| d.as_vec2().as_vec2())
            .filter(|_| interpolation > 0.5)
            .unwrap_or(snake[h] - snake[n]);

        if interpolation > 0.5 {
            snake.insert(h, snake[h]);
        }

        let h = snake.len() - 1;
        snake[0] = snake[0] + (snake[1] - snake[0]) * interpolation;
        snake[h] = snake[h] + next_input * (interpolation - 0.5);
        // snake[h] = snake[n] + (snake[h] - snake[n]) * interpolation;

        for i in 0..snake.len() {
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: Mesh2dHandle(render_resources.circle_mesh.clone()),
                    material: render_resources.snake_materials[snake_id].clone(),
                    transform: board_pos(snake[i], 0.0),
                    ..default()
                },
                SnakePart,
            ));
        }

        for i in 1..snake.len() {
            let pos = snake[i];
            let prev = snake[i - 1];
            let mid_pos = (pos + prev) / 2.0;
            let scale = (pos - prev).length();

            let capsule_pos = board_pos(mid_pos, 0.0);
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: Mesh2dHandle(render_resources.square_mesh.clone()),
                    material: render_resources.snake_materials[snake_id].clone(),
                    transform: capsule_pos
                        .looking_at(
                            capsule_pos.translation + Vec3::Z,
                            (pos - mid_pos).extend(0.0),
                        )
                        .with_scale(Vec3::new(1.0, scale, 1.0)),
                    ..default()
                },
                SnakePart,
            ));
        }
    }

    // debug
    for entity in debug_tiles.iter() {
        commands.entity(entity).despawn();
    }
    if settings.walls_debug {
        for pos in board.get_spawnable() {
            let bundle = SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.15),
                    ..default()
                },
                transform: board_pos(pos.as_vec2(), 0.0),
                ..default()
            };
            commands.spawn((bundle, DebugTile));
        }
    }
}
