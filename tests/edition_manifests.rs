use car_game::{assembly::*, roster::Fleet};
use sha2::{Digest, Sha256};

fn supported() -> Vec<car_game::roster::CarSpec> {
    Fleet::default()
        .cars
        .into_iter()
        .filter(|spec| editions::prepared_hash(&spec.id).is_some())
        .collect()
}

#[test]
fn production_evidence_fixture_is_valid() {
    car_game::scenario::ScenarioDefinition::from_json(include_str!(
        "../examples/scenarios/gt-major-parts.json"
    ))
    .unwrap();
}

#[test]
fn authored_roots_hashes_and_mesh_instances_match_prepared_sources() {
    assert_eq!(supported().len(), 5);
    for spec in supported() {
        let bytes = std::fs::read(format!("assets/{}", spec.path)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            editions::prepared_hash(&spec.id).unwrap()
        );
        let assembly = Assembly::major_parts(&spec).unwrap();
        assembly.validate_visuals(&spec, &bytes).unwrap();
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let document: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
        let children: Vec<Vec<usize>> = if spec.id == "350z" {
            vec![vec![], vec![], vec![], vec![], vec![]]
        } else if spec.id.starts_with("gt4-") {
            vec![vec![5], vec![6, 8], vec![7, 9], vec![10, 12], vec![11, 13]]
        } else {
            vec![vec![5], vec![6], vec![7], vec![8], vec![9]]
        };
        for (index, expected) in children.iter().enumerate() {
            let actual: Vec<usize> = document["nodes"][index]["children"]
                .as_array()
                .map(|nodes| {
                    nodes
                        .iter()
                        .map(|node| node.as_u64().unwrap() as usize)
                        .collect()
                })
                .unwrap_or_default();
            assert_eq!(&actual, expected, "{} node {index}", spec.id);
        }
        for (corner, roots) in if spec.id == "350z" {
            vec![vec![1, 3], vec![2, 4], vec![5, 7], vec![6, 8]]
        } else {
            vec![vec![1], vec![2], vec![3], vec![4]]
        }
        .into_iter()
        .enumerate()
        {
            let part = assembly
                .part(&PartId(format!("{}-wheel-{corner}", spec.id)))
                .unwrap();
            assert_eq!(
                assembly.definitions[&part.definition]
                    .visual
                    .nodes
                    .iter()
                    .map(|node| node.0)
                    .collect::<Vec<_>>(),
                roots
            );
        }
    }
}

#[test]
fn incorrect_missing_and_duplicate_subtree_bindings_are_rejected() {
    for spec in supported() {
        let bytes = std::fs::read(format!("assets/{}", spec.path)).unwrap();
        let good = Assembly::major_parts(&spec).unwrap();
        let definition = good.parts[1].definition.clone();
        for fault in 0..5 {
            let mut bad = good.clone();
            let binding = &mut bad.definitions.get_mut(&definition).unwrap().visual;
            match fault {
                0 => binding.nodes[0].0 = 999,
                1 => binding.nodes[0].1 = "not-the-wheel".into(),
                2 => binding.nodes.clear(),
                3 => binding.prepared_sha256 = "wrong-hash".into(),
                _ => binding.nodes.push(binding.nodes[0].clone()),
            }
            assert!(
                bad.validate_visuals(&spec, &bytes).is_err(),
                "{} fault {fault}",
                spec.id
            );
        }
        if spec.id != "350z" {
            let mut bad = good;
            let binding = &mut bad.definitions.get_mut(&definition).unwrap().visual;
            let name = if spec.id.starts_with("gt4-") {
                "source/wheel_front_left"
            } else {
                "source/Native wheel 0"
            };
            binding.nodes.push((6, name.into()));
            assert_eq!(
                bad.validate_visuals(&spec, &bytes).unwrap_err(),
                "duplicate-visual-ownership"
            );
        }
    }
}

#[test]
fn each_edition_preserves_baseline_tensor_and_all_four_removal_contributions() {
    for spec in supported() {
        let original = Assembly::major_parts(&spec).unwrap();
        original.validate().unwrap();
        let intact = original.aggregate().unwrap();
        assert_eq!(intact.mass, 1320.);
        for row in 0..3 {
            assert!(intact.center[row].abs() < 1e-10);
            for column in 0..3 {
                let expected = if row == column {
                    car_game::vehicle::BASELINE_INERTIA[row] as f64
                } else {
                    0.
                };
                assert!((intact.inertia[row][column] - expected).abs() < 1e-8);
            }
        }
        for corner in 0..4 {
            let mut assembly = original.clone();
            let part = PartId(format!("{}-wheel-{corner}", spec.id));
            assembly
                .transact(&Transaction {
                    id: TransactionId(format!("remove-{corner}")),
                    expected_revision: 0,
                    part: part.clone(),
                    destination: Attachment::Root {
                        location: Location::Inventory {
                            owner: assembly.owner.clone(),
                            slot: format!("rack-{corner}"),
                        },
                    },
                })
                .unwrap();
            let reduced = assembly.aggregate().unwrap();
            assert_removed_tensor(
                &intact,
                &reduced,
                spec.mounts()[corner].as_dvec3().to_array(),
            );
            assert_eq!(reduced.mass, 1295.);
            for axis in 0..3 {
                assert!(
                    (reduced.center[axis] + 25. * spec.mounts()[corner][axis] as f64 / 1295.).abs()
                        < 1e-9
                );
            }
            assert_eq!(
                assembly.wheel_mask().unwrap(),
                std::array::from_fn(|index| index != corner)
            );
            assert_eq!(
                assembly.inspect().unwrap().readiness,
                "incomplete-inoperable"
            );
            assert!(!assembly.installed(&part).unwrap());
            reduced.rapier();
        }
    }
}

fn assert_removed_tensor(intact: &Aggregate, reduced: &Aggregate, position: [f64; 3]) {
    let position_squared: f64 = position.iter().map(|value| value * value).sum();
    let center_squared: f64 = reduced.center.iter().map(|value| value * value).sum();
    for row in 0..3 {
        for column in 0..3 {
            let wheel = if row == column {
                2. + 25. * position_squared
            } else {
                0.
            } - 25. * position[row] * position[column];
            let shift = if row == column {
                reduced.mass * center_squared
            } else {
                0.
            } - reduced.mass * reduced.center[row] * reduced.center[column];
            assert!(
                (reduced.inertia[row][column] - (intact.inertia[row][column] - wheel - shift))
                    .abs()
                    < 1e-8
            );
        }
    }
}
