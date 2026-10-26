use super::*;
const PREPARED_SHA: &str = "e79630558ed16c54942afc552838fd465a73bf507754947342bd65a805d37874";
fn visual(spec: &crate::roster::CarSpec, nodes: Vec<(usize, String)>) -> VisualBinding {
    VisualBinding {
        asset: spec.path.clone(),
        prepared_sha256: PREPARED_SHA.into(),
        scene: 0,
        nodes,
        picking: "unavailable until authored P2 picking/access geometry".into(),
    }
}
fn residual(spec: &crate::roster::CarSpec) -> Definition {
    let mounts: BTreeMap<_, _> = spec
        .mounts()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                format!("wheel-{i}"),
                Mount {
                    wheel_corner: Some(i),
                    interface: "350z-wheel-hub".into(),
                    frame: Frame {
                        position: p.as_dvec3().to_array(),
                        ..default()
                    },
                    required: true,
                },
            )
        })
        .collect();
    let target = crate::vehicle::profile::for_car(&spec.id).properties(spec);
    let mass = target.mass as f64 - 100.;
    let target_center = target.local_center_of_mass.as_dvec3();
    let baseline = target.principal_inertia.as_dvec3().to_array();
    let mut center = target_center * target.mass as f64;
    for m in mounts.values() {
        center -= DVec3::from_array(m.frame.position) * 25.;
    }
    center /= mass;
    let inertia = std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            let parallel = |p: DVec3| {
                if r == c {
                    p.length_squared() - p[r] * p[c]
                } else {
                    -p[r] * p[c]
                }
            };
            let mut value = if r == c { baseline[r] } else { 0. };
            value += target.mass as f64 * parallel(target_center) - mass * parallel(center);
            for m in mounts.values() {
                value -= 25. * parallel(DVec3::from_array(m.frame.position))
                    + if r == c { 2. } else { 0. };
            }
            value
        })
    });
    Definition {
        visual: visual(spec, vec![(0, "body".into())]),
        collision: CollisionBinding::ResidualChassis,
        interface: "chassis".into(),
        mass,
        center: center.to_array(),
        inertia,
        mounts,
        provenance:
            "Authored residual decomposes the selected 350Z physics profile; four wheel masses and inertia remain estimates"
                .into(),
    }
}
impl Assembly {
    /// Declared P1 approximation: residual chassis + four 25 kg wheel assemblies.
    /// This is a major-part fixture, not an operating mechanical manifest.
    pub fn pilot(spec: &crate::roster::CarSpec) -> Self {
        assert_eq!(
            spec.id, "350z",
            "pilot fixture requires the prepared baseline car"
        );
        let owner = OwnerId::from("sandbox");
        let vehicle = VehicleId::from("pilot-350z");
        let chassis = DefinitionId::from("350z-residual-chassis");
        let root = PartId::from("350z-chassis");
        let mut parts = vec![Part {
            id: root.clone(),
            definition: chassis.clone(),
            attachment: Attachment::Root {
                location: Location::Vehicle {
                    vehicle: vehicle.clone(),
                    owner: owner.clone(),
                },
            },
            condition: 1.,
            secured: true,
        }];
        let mut definitions = BTreeMap::from([(chassis, residual(spec))]);
        for (i, (suffix, wheel_node, tire_node)) in [
            ("front_left", 1, 3),
            ("front_right", 2, 4),
            ("rear_left", 5, 7),
            ("rear_right", 6, 8),
        ]
        .into_iter()
        .enumerate()
        {
            let wheel = DefinitionId(format!("350z-test-wheel-{i}"));
            definitions.insert(wheel.clone(),Definition {
                visual:visual(spec,vec![(wheel_node,format!("wheel_{suffix}")),(tire_node,format!("tire_{suffix}"))]),
                collision:CollisionBinding::RaycastWheel {corner:i},
                interface:"350z-wheel-hub".into(),mass:25.,center:[0.;3],inertia:[[2.,0.,0.],[0.,2.,0.],[0.,0.,2.]],mounts:BTreeMap::new(),
                provenance:"P1 authored test wheel physical approximation; existing prepared wheel pivot binding".into(),
            });
            parts.push(Part {
                id: PartId(format!("350z-wheel-{i}")),
                definition: wheel,
                attachment: Attachment::Mounted {
                    parent: root.clone(),
                    mount: format!("wheel-{i}"),
                },
                condition: 1.,
                secured: true,
            });
        }
        Self {
            vehicle,
            owner,
            revision: 0,
            definitions,
            parts,
            prerequisites: Vec::new(),
            networks: Vec::new(),
            committed: BTreeMap::new(),
        }
    }
}
