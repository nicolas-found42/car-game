use car_game::{assembly::*, roster::CarSpec};
fn pilot() -> Assembly {
    Assembly::pilot(&CarSpec::default())
}
fn remove(revision: u64) -> Transaction {
    Transaction {
        id: TransactionId(format!("remove-{revision}")),
        expected_revision: revision,
        part: PartId::from("350z-wheel-0"),
        destination: Attachment::Root {
            location: Location::Inventory {
                owner: OwnerId::from("sandbox"),
                slot: "rack-0".into(),
            },
        },
    }
}
#[test]
fn atomic_remove_reinstall_and_idempotence_preserve_identity() {
    let mut a = pilot();
    a.validate().unwrap();
    let before = a.clone();
    let t = remove(0);
    a.transact(&t).unwrap();
    assert_eq!(a.inspect().unwrap().readiness, "incomplete-inoperable");
    assert_eq!(a.aggregate().unwrap().mass, 1295.);
    let once = a.clone();
    assert_eq!(a.transact(&t).unwrap(), "already-applied");
    assert_eq!(a, once);
    let mut stale = remove(0);
    stale.id = TransactionId::from("stale");
    assert_eq!(a.transact(&stale).unwrap_err(), "stale-revision");
    assert_eq!(a, once);
    let restore = Transaction {
        id: TransactionId::from("restore"),
        expected_revision: 1,
        part: t.part,
        destination: before.parts[1].attachment.clone(),
    };
    a.transact(&restore).unwrap();
    assert_eq!(a.parts, before.parts);
    assert_eq!(a.aggregate().unwrap(), before.aggregate().unwrap());
    assert_eq!(a.inspect().unwrap().readiness, "ready");
}
#[test]
fn topology_errors_leave_all_records_unchanged() {
    let initial = pilot();
    for (part, destination, reason) in [
        (
            "350z-wheel-0",
            Attachment::Mounted {
                parent: PartId::from("350z-chassis"),
                mount: "wheel-1".into(),
            },
            "occupied-mount",
        ),
        (
            "350z-chassis",
            Attachment::Mounted {
                parent: PartId::from("350z-chassis"),
                mount: "wheel-0".into(),
            },
            "incompatible-mount",
        ),
        (
            "350z-wheel-0",
            Attachment::Mounted {
                parent: PartId::from("absent"),
                mount: "wheel-0".into(),
            },
            "unknown-part",
        ),
        (
            "350z-wheel-0",
            Attachment::Root {
                location: Location::Inventory {
                    owner: OwnerId::from("other"),
                    slot: "rack".into(),
                },
            },
            "wrong-owner-or-location",
        ),
    ] {
        let mut a = initial.clone();
        let t = Transaction {
            id: TransactionId::from("bad"),
            expected_revision: 0,
            part: PartId::from(part),
            destination,
        };
        assert_eq!(a.transact(&t).unwrap_err(), reason);
        assert_eq!(a, initial);
    }
    let mut duplicate = initial.clone();
    duplicate.parts.push(duplicate.parts[1].clone());
    assert_eq!(
        duplicate.validate().unwrap_err(),
        "duplicate-or-empty-part-id"
    );
    let mut cycle = initial.clone();
    let chassis = cycle.parts[0].definition.clone();
    cycle
        .definitions
        .get_mut(&chassis)
        .unwrap()
        .mounts
        .get_mut("wheel-0")
        .unwrap()
        .interface = "chassis".into();
    cycle.parts[0].attachment = Attachment::Mounted {
        parent: cycle.parts[0].id.clone(),
        mount: "wheel-0".into(),
    };
    assert_eq!(cycle.validate().unwrap_err(), "attachment-cycle");
}
#[test]
fn prerequisites_are_acyclic_but_functional_networks_can_loop() {
    let mut a = pilot();
    let x = a.parts[1].id.clone();
    let y = a.parts[2].id.clone();
    a.networks = vec![
        NetworkEdge {
            from: x.clone(),
            to: y.clone(),
            network: "coolant".into(),
        },
        NetworkEdge {
            from: y.clone(),
            to: x.clone(),
            network: "coolant".into(),
        },
    ];
    a.validate().unwrap();
    a.prerequisites = vec![(x.clone(), y.clone())];
    a.transact(&remove(0)).unwrap();
    let before = a.clone();
    let t = Transaction {
        part: y.clone(),
        id: TransactionId::from("dependent"),
        expected_revision: 1,
        destination: Attachment::Root {
            location: Location::Inventory {
                owner: OwnerId::from("sandbox"),
                slot: "rack-1".into(),
            },
        },
    };
    // Removing the prerequisite is allowed once its dependent is already removed.
    a.transact(&t).unwrap();
    let mut cycle = before;
    cycle.prerequisites.push((y, x));
    assert_eq!(cycle.validate().unwrap_err(), "prerequisite-cycle");
    let mut blocked = pilot();
    blocked.prerequisites = vec![(blocked.parts[2].id.clone(), blocked.parts[1].id.clone())];
    let before = blocked.clone();
    assert_eq!(
        blocked.transact(&remove(0)).unwrap_err(),
        "missing-prerequisite"
    );
    assert_eq!(blocked, before);
}
#[test]
fn intact_mass_tensor_is_baseline_and_order_independent() {
    let a = pilot();
    let aggregate = a.aggregate().unwrap();
    assert_eq!(aggregate.mass, 1320.);
    for x in aggregate.center {
        assert!(x.abs() < 1e-12);
    }
    for r in 0..3 {
        for c in 0..3 {
            let expected = if r == c { [1800., 2300., 650.][r] } else { 0. };
            assert!((aggregate.inertia[r][c] - expected).abs() < 1e-9);
        }
    }
    let mut permuted = a.clone();
    permuted.parts.reverse();
    assert_eq!(aggregate, permuted.aggregate().unwrap());
}
#[test]
fn analytic_rotated_tensor_and_parallel_axis() {
    let mut a = pilot();
    a.parts.truncate(2);
    let chassis = a.parts[0].definition.clone();
    let wheel = a.parts[1].definition.clone();
    for id in [&chassis, &wheel] {
        let d = a.definitions.get_mut(id).unwrap();
        d.mass = 2.;
        d.center = [0.; 3];
        d.inertia = [[1., 0., 0.], [0., 2., 0.], [0., 0., 3.]];
    }
    let frame = &mut a
        .definitions
        .get_mut(&chassis)
        .unwrap()
        .mounts
        .get_mut("wheel-0")
        .unwrap()
        .frame;
    frame.position = [2., 0., 0.];
    frame.rotation = [
        0.,
        0.,
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ];
    a.validate().unwrap();
    let mass = a.aggregate().unwrap();
    assert_eq!(mass.mass, 4.);
    assert_eq!(mass.center, [1., 0., 0.]);
    for (i, expected) in [3., 7., 10.].iter().enumerate() {
        assert!((mass.inertia[i][i] - expected).abs() < 1e-9);
    }
    let d = a.definitions.get_mut(&wheel).unwrap();
    d.inertia[0][0] = f64::NAN;
    assert_eq!(a.validate().unwrap_err(), "invalid-mass-tensor");
}

#[test]
fn aggregate_overflow_is_rejected_before_atomic_commit() {
    let mut a = pilot();
    let root = a.parts[0].definition.clone();
    a.definitions.get_mut(&root).unwrap().mounts.insert(
        "far".into(),
        Mount {
            wheel_corner: None,
            interface: "350z-wheel-hub".into(),
            frame: Frame {
                position: [1e160, 0., 0.],
                ..Default::default()
            },
            required: false,
        },
    );
    // An unused distant mount is finite authoring data; using it must fail safely.
    a.validate().unwrap();
    let before = a.clone();
    let bad = Transaction {
        id: TransactionId::from("overflow"),
        expected_revision: 0,
        part: a.parts[1].id.clone(),
        destination: Attachment::Mounted {
            parent: a.parts[0].id.clone(),
            mount: "far".into(),
        },
    };
    assert!(a.transact(&bad).unwrap_err().contains("mass-tensor"));
    assert_eq!(a, before);
    let mut bad = bad;
    bad.id = TransactionId::from("f32-overflow");
    a.definitions
        .get_mut(&root)
        .unwrap()
        .mounts
        .get_mut("far")
        .unwrap()
        .frame
        .position = [1e30, 0., 0.];
    let before = a.clone();
    assert!(a.transact(&bad).unwrap_err().contains("mass-tensor"));
    assert_eq!(a, before);
}

#[test]
fn finite_tensor_entries_cannot_hide_principal_inertia_overflow() {
    let mut a = pilot();
    let chassis = a.parts[0].definition.clone();
    let wheel = a.parts[1].definition.clone();
    let installed = a.parts[1].attachment.clone();
    a.transact(&remove(0)).unwrap();
    a.definitions.get_mut(&chassis).unwrap().inertia = [
        [2.8e38, 0.2e38, 0.2e38],
        [0.2e38, 2.8e38, 0.2e38],
        [0.2e38, 0.2e38, 2.8e38],
    ];
    a.definitions.get_mut(&wheel).unwrap().inertia =
        [[0.4e38, 0., 0.], [0., 0.4e38, 0.], [0., 0., 0.4e38]];
    a.validate().unwrap();
    let before = a.clone();
    let t = Transaction {
        id: TransactionId::from("principal-overflow"),
        expected_revision: 1,
        part: a.parts[1].id.clone(),
        destination: installed,
    };
    assert_eq!(a.transact(&t).unwrap_err(), "invalid-aggregate-mass-tensor");
    assert_eq!(a, before);
}
#[test]
fn mounted_wheels_follow_the_socket_not_the_definition_corner() {
    let mut a = pilot();
    let Attachment::Mounted { parent, .. } = a.parts[1].attachment.clone() else {
        panic!("pilot wheel-0 is mounted");
    };
    a.transact(&remove(0)).unwrap();
    let swap = Transaction {
        id: TransactionId::from("swap"),
        expected_revision: 1,
        part: PartId::from("350z-wheel-1"),
        destination: Attachment::Mounted {
            parent,
            mount: "wheel-0".into(),
        },
    };
    a.transact(&swap).unwrap();
    let wheels = a.mounted_wheels().unwrap();
    assert_eq!(wheels[0], Some(PartId::from("350z-wheel-1")));
    assert_eq!(wheels[1], None, "inventory wheel-0 and empty socket");
    assert_eq!(a.wheel_mask().unwrap(), [true, false, true, true]);
}
