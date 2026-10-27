use super::*;
use crate::assembly::*;

#[test]
fn bound_subtrees_hide_and_return_without_reparenting_or_duplicate_children() {
    for spec in Fleet::default().cars {
        let Some(assembly) = Assembly::major_parts(&spec) else {
            continue;
        };
        let mut app = App::new();
        app.insert_resource(spec).insert_resource(assembly.clone());
        app.add_systems(Update, bind_part_visuals);
        let visual = app.world_mut().spawn(VisualCar).id();
        let mut roots = Vec::new();
        for part in &assembly.parts {
            for (_, name) in &assembly.definitions[&part.definition].visual.nodes {
                let root = app
                    .world_mut()
                    .spawn((
                        Name::new(name.clone()),
                        Visibility::Inherited,
                        ChildOf(visual),
                    ))
                    .id();
                let leaf = app
                    .world_mut()
                    .spawn((Name::new("authored mesh leaf"), ChildOf(root)))
                    .id();
                roots.push((part.id.clone(), root, leaf));
            }
        }
        app.update();
        let wheel = assembly.parts[1].id.clone();
        let mut removed = assembly.clone();
        removed.parts[1].attachment = Attachment::Root {
            location: Location::Inventory {
                owner: assembly.owner.clone(),
                slot: "rack".into(),
            },
        };
        app.insert_resource(removed);
        app.update();
        for (part, root, leaf) in &roots {
            assert_eq!(app.world().get::<PartVisual>(*root).unwrap().0, *part);
            assert_eq!(
                *app.world().get::<Visibility>(*root).unwrap(),
                if part == &wheel {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                }
            );
            assert_eq!(app.world().get::<ChildOf>(*leaf).unwrap().parent(), *root);
            assert_eq!(app.world().get::<Children>(*root).unwrap().len(), 1);
        }
        app.insert_resource(assembly);
        app.update();
        for (_, root, _) in roots {
            assert_eq!(
                *app.world().get::<Visibility>(root).unwrap(),
                Visibility::Inherited
            );
        }
    }
}

#[test]
fn animation_preserves_authored_reflection_rotation_and_visual_offset() {
    let spec = CarSpec::default();
    let rest = Transform::from_xyz(-0.8, 0., -1.3)
        .with_scale(Vec3::new(-1., 1., 1.))
        .with_rotation(Quat::from_rotation_z(0.2));
    let wheel = WheelVisual(0, rest);
    let mut transform = rest;
    let mut car = Vehicle::default();
    car.steering = 0.3;
    car.wheel_spin = [0.7; 4];
    apply_wheel_pose(&mut transform, &spec, &car, &wheel);
    assert_eq!(transform.scale, rest.scale);
    assert_eq!(transform.translation.x, rest.translation.x);
    assert_eq!(transform.translation.z, rest.translation.z);
    assert_eq!(
        transform.translation.y,
        spec.mounts()[0].y - car.wheel_lengths[0] - spec.visual_offset
    );
    assert!(transform.rotation.abs_diff_eq(
        Quat::from_rotation_y(0.3) * Quat::from_rotation_x(0.7) * rest.rotation,
        1e-6
    ));
}
