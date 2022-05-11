use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::PrimitiveTopology},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use rand::Rng;
use std::collections::{HashMap, VecDeque};

mod meshing;
use meshing::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
enum GameState {
    Playing,
    Paused,
    GameOver,
}

#[derive(Component)]
pub struct Snake {
    body: Vec<IVec2>,
    input_map: InputMap,
    input_queue: VecDeque<Direction>,
    head_dir: IVec2,
    tail_dir: IVec2,
}

struct InputMap {
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
}

struct Board {
    width: i32,
    height: i32,
    colour1: Color,
    colour2: Color,
}

struct MovmentTimer(Timer);

#[derive(PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

const DIR: [[i32; 2]; 4] = [[0, 1], [0, -1], [-1, 0], [1, 0]];

struct Apples {
    list: HashMap<IVec2, Entity>,
    sprite: Option<Handle<Image>>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Board {
            width: 17,
            height: 15,
            colour1: Color::rgb(0.3, 0.3, 0.3),
            colour2: Color::rgb(0.2, 0.2, 0.2),
        })
        .insert_resource(MovmentTimer(Timer::from_seconds(1.0 / 3.0, true)))
        .insert_resource(Apples {
            list: HashMap::new(),
            sprite: None,
        })
        .add_system(bevy::input::system::exit_on_esc_system)
        .add_system(state_controller)
        .add_startup_system(scene_setup)
        .add_startup_system(snake_setup)
        .add_state(GameState::Playing)
        // .add_system_set(
        //     SystemSet::on_enter(GameState::Playing)
        //         .with_system(scene_setup)
        //         .with_system(snake_setup),
        // )
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(snake_system))
        .run();
}

fn state_controller(mut game_state: ResMut<State<GameState>>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::P) {
        match game_state.current() {
            GameState::Playing => game_state.set(GameState::Paused).unwrap(),
            GameState::Paused => game_state.set(GameState::Playing).unwrap(),
            _ => {}
        }
    }
}

fn spawn_apple(pos: IVec2, apples: &mut Apples, commands: &mut Commands, b: &Board) {
    apples.list.insert(
        pos,
        commands
            .spawn_bundle(SpriteBundle {
                texture: apples.sprite.as_ref().unwrap().clone(),
                transform: Transform::from_xyz(
                    pos.x as f32 - b.width as f32 / 2.0 + 0.5,
                    pos.y as f32 - b.height as f32 / 2.0 + 0.5,
                    5.0,
                )
                .with_scale(Vec3::splat(1.0 / 512.0)),
                ..default()
            })
            .id(),
    );
}

fn scene_setup(
    mut commands: Commands,
    b: Res<Board>,
    mut apples: ResMut<Apples>,
    assets: Res<AssetServer>,
) {
    apples.sprite = Some(assets.load("images/apple.png"));

    spawn_apple(IVec2::new(8, 7), &mut apples, &mut commands, &b);
    spawn_apple(IVec2::new(10, 7), &mut apples, &mut commands, &b);
    spawn_apple(IVec2::new(8, 9), &mut apples, &mut commands, &b);

    commands.spawn_bundle(OrthographicCameraBundle {
        orthographic_projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical,
            scale: b.height as f32 / 2.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 999.9),
        ..OrthographicCameraBundle::new_2d()
    });

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.1, 0.1, 0.1),
            custom_size: Some(Vec2::new(1000.0, 1000.0)),
            ..default()
        },
        ..default()
    });

    for x in 0..b.width {
        for y in 0..b.height {
            let color = if (x + y) % 2 == 0 {
                b.colour1
            } else {
                b.colour2
            };

            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite { color, ..default() },
                transform: Transform {
                    translation: Vec3::new(
                        x as f32 - b.width as f32 / 2.0 + 0.5,
                        y as f32 - b.height as f32 / 2.0 + 0.5,
                        0.0,
                    ),
                    ..default()
                },
                ..default()
            });
        }
    }
}

fn snake_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    b: Res<Board>,
) {
    let snake1 = Snake {
        body: vec![
            IVec2::new(4, 1),
            IVec2::new(3, 1),
            IVec2::new(2, 1),
            IVec2::new(1, 1),
        ],
        input_map: InputMap {
            up: KeyCode::W,
            down: KeyCode::S,
            left: KeyCode::A,
            right: KeyCode::D,
        },
        input_queue: VecDeque::new(),
        head_dir: IVec2::new(1, 0),
        tail_dir: IVec2::new(-1, 0),
    };
    let mesh1 = mesh_snake(&snake1, 0.0);

    let snake2 = Snake {
        body: vec![
            IVec2::new(12, 13),
            IVec2::new(13, 13),
            IVec2::new(14, 13),
            IVec2::new(15, 13),
        ],
        input_map: InputMap {
            up: KeyCode::Up,
            down: KeyCode::Down,
            left: KeyCode::Left,
            right: KeyCode::Right,
        },
        input_queue: VecDeque::new(),
        head_dir: IVec2::new(-1, 0),
        tail_dir: IVec2::new(1, 0),
    };
    let mesh2 = meshing::mesh_snake(&snake2, 0.0);

    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: meshes.add(mesh1).into(),
            material: materials.add(ColorMaterial::from(Color::rgb(0.0, 0.7, 0.25))),
            transform: Transform::from_xyz(-b.width as f32 / 2.0, -b.height as f32 / 2.0, 10.0),
            ..default()
        })
        .insert(snake1);
    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: meshes.add(mesh2).into(),
            material: materials.add(ColorMaterial::from(Color::rgb(0.7, 0.1, 0.0))),
            transform: Transform::from_xyz(-b.width as f32 / 2.0, -b.height as f32 / 2.0, 10.0),
            ..default()
        })
        .insert(snake2);
}

fn snake_system(
    mut commands: Commands,
    mut snake_query: Query<(&mut Snake, &mut Mesh2dHandle)>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mut timer: ResMut<MovmentTimer>,
    keys: Res<Input<KeyCode>>,
    mut apples: ResMut<Apples>,
    b: Res<Board>,
    mut app_state: ResMut<State<GameState>>,
) {
    timer.0.tick(time.delta());

    let mut num_apples_to_spawn = 0;
    for (mut snake, mut mesh_handle) in snake_query.iter_mut() {
        let head = snake.body[0];
        let neck = snake.body[1];
        let forward = head - neck;

        let last_in_queue = *snake.input_queue.back().unwrap_or(&get_direction(forward));
        if snake.input_queue.len() < 3 {
            if keys.just_pressed(snake.input_map.up) {
                if last_in_queue != Direction::Down && last_in_queue != Direction::Up {
                    snake.input_queue.push_back(Direction::Up);
                }
            }
            if keys.just_pressed(snake.input_map.down) {
                if last_in_queue != Direction::Up && last_in_queue != Direction::Down {
                    snake.input_queue.push_back(Direction::Down);
                }
            }
            if keys.just_pressed(snake.input_map.left) {
                if last_in_queue != Direction::Right && last_in_queue != Direction::Left {
                    snake.input_queue.push_back(Direction::Left);
                }
            }
            if keys.just_pressed(snake.input_map.right) {
                if last_in_queue != Direction::Left && last_in_queue != Direction::Right {
                    snake.input_queue.push_back(Direction::Right);
                }
            }
        }

        if timer.0.just_finished() {
            let current_dir = head - neck;

            if let Some(direction) = snake.input_queue.pop_front() {
                let dir = DIR[direction as usize];
                let dir = IVec2::new(dir[0], dir[1]);
                if current_dir + dir != IVec2::ZERO {
                    snake.body.insert(0, head + dir);
                } else {
                    snake.body.insert(0, head + head - neck);
                }
            } else {
                snake.body.insert(0, head + head - neck);
            }

            let head = snake.body[0];
            if let Some(apple_entity) = apples.list.get(&head) {
                commands.entity(*apple_entity).despawn();
                apples.list.remove(&head);

                num_apples_to_spawn += 1;
            } else {
                let len = snake.body.len();
                snake.tail_dir = snake.body[len - 2] - snake.body[len - 1];
                snake.body.remove(len - 1);
            }
        }

        let interpolation = timer.0.elapsed_secs() / timer.0.duration().as_secs_f32() - 0.5;
        snake.head_dir = if let Some(dir) = snake.input_queue.get(0) {
            DIR[*dir as usize].into()
        } else {
            head - neck
        };

        let mesh = mesh_snake(&snake, interpolation);
        *mesh_handle = meshes.add(mesh).into();
    }

    // Handle end game
    if timer.0.just_finished() {
        for (snake, _) in snake_query.iter() {
            let new_head = snake.body[0];
            if !in_bounds(new_head, &b) {
                app_state.set(GameState::Paused).unwrap();
                return;
            }

            for (other_snake, _) in snake_query.iter() {
                for snake_body in other_snake.body.iter().skip(1) {
                    if *snake_body == new_head {
                        app_state.set(GameState::Paused).unwrap();
                        return;
                    }
                }
            }
        }
    }

    let mut rng = rand::thread_rng();
    let mut count = 0;
    'outer: while num_apples_to_spawn > 0 {
        let pos = IVec2::new(rng.gen_range(0..b.width), rng.gen_range(0..b.height));
        if !apples.list.contains_key(&pos) {
            for (snake, _) in snake_query.iter() {
                if snake.body.contains(&pos) {
                    continue 'outer;
                }
            }

            spawn_apple(pos, &mut apples, &mut commands, &b);
            num_apples_to_spawn -= 1;
        }

        count += 1;
        if count > 1000 {
            break 'outer;
        }
    }
}

fn in_bounds(pos: IVec2, b: &Board) -> bool {
    pos.x >= 0 && pos.x < b.width && pos.y >= 0 && pos.y < b.height
}

fn calculate_flip(dir: IVec2) -> IVec2 {
    match dir.to_array() {
        [0, 1] => IVec2::new(1, 0),
        [0, -1] => IVec2::new(-1, 0),
        [1, 0] => IVec2::new(1, 1),
        [-1, 0] => IVec2::new(-1, 1),
        _ => IVec2::new(1, 1),
    }
}

fn get_direction(dir: IVec2) -> Direction {
    match dir.to_array() {
        [0, 1] => Direction::Up,
        [0, -1] => Direction::Down,
        [1, 0] => Direction::Right,
        [-1, 0] => Direction::Left,
        _ => panic!("Invalid direction"),
    }
}
