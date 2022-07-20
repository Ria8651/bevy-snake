use super::*;

pub struct ApplePlugin;

impl Plugin for ApplePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(
                apple_system
                    .after(snake::damage_snake_system)
                    .after(snake::snake_system)
                    .after(reset_game),
            ),
        );
    }
}

pub struct Apples {
    pub list: HashMap<IVec2, Entity>,
    pub sprite: Option<Handle<Image>>,
}

// struct AppleEv(AppleSpawn);

#[derive(Copy, Clone)]
pub enum AppleEv {
    SpawnRandom,
    SpawnPos(IVec2),
}

fn apple_system(
    mut apples: ResMut<Apples>,
    snake_query: Query<&Snake>,
    mut commands: Commands,
    b: Res<Board>,
    mut apple_ev: EventReader<AppleEv>,
) {
    let mut rng = rand::thread_rng();

    for apple_ev in apple_ev.iter() {
        match apple_ev {
            AppleEv::SpawnRandom | AppleEv::SpawnPos(_) => {
                let mut count = 0;
                let mut pos;
                'apple: loop {
                    pos = if let AppleEv::SpawnPos(set_pos) = apple_ev {
                        *set_pos
                    } else {
                        IVec2::new(rng.gen_range(0..b.width), rng.gen_range(0..b.height))
                    };

                    count += 1;
                    if count > 1000 {
                        return;
                    }

                    if apples.list.contains_key(&pos) {
                        continue 'apple;
                    }

                    for snake in snake_query.iter() {
                        if snake.body.contains(&pos) {
                            continue 'apple;
                        }
                    }

                    break 'apple;
                }

                let texture = apples.sprite.as_ref().unwrap().clone();
                apples.list.insert(
                    pos,
                    commands
                        .spawn_bundle(SpriteBundle {
                            texture: texture,
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
        }
    }
}
