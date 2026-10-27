use super::*;
fn visual(spec: &crate::roster::CarSpec, nodes: Vec<(usize, String)>) -> VisualBinding {
    VisualBinding {
        asset: spec.path.clone(),
        prepared_sha256: super::editions::prepared_hash(&spec.id)
            .expect("supported edition")
            .into(),
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
                    interface: format!("{}-wheel-hub", spec.id),
                    frame: Frame {
                        position: p.as_dvec3().to_array(),
                        ..default()
                    },
                    required: true,
                },
            )
        })
        .collect();
    let baseline = crate::vehicle::BASELINE_INERTIA;
    let mut inertia = std::array::from_fn(|r| {
        std::array::from_fn(|c| if r == c { baseline[r] as f64 } else { 0. })
    });
    let mut center = DVec3::ZERO;
    for m in mounts.values() {
        center -= DVec3::from_array(m.frame.position) * 25. / 1220.;
    }
    for r in 0..3 {
        for c in 0..3 {
            for mount in mounts.values() {
                let p = DVec3::from_array(mount.frame.position);
                inertia[r][c] -= if r == c {
                    2. + 25. * p.length_squared()
                } else {
                    0.
                };
                inertia[r][c] += 25. * p[r] * p[c];
            }
            inertia[r][c] -= 1220. * (if r == c { center.length_squared() } else { 0. });
            inertia[r][c] += 1220. * center[r] * center[c];
        }
    }
    Definition {
        visual: visual(spec, vec![(0, "body".into())]),
        collision: CollisionBinding::ResidualChassis,
        interface: "chassis".into(),
        mass: 1220.,
        center: center.to_array(),
        inertia,
        mounts,
        provenance:
            "P1 authored residual mass calibrated to ADR-0001 baseline; no invented engine geometry"
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
        Self::major_parts(spec).expect("baseline edition")
    }
    pub fn major_parts(spec: &crate::roster::CarSpec) -> Option<Self> {
        super::editions::prepared_hash(&spec.id)?;
        let owner = OwnerId::from("sandbox");
        let vehicle = super::editions::vehicle_id(&spec.id);
        let chassis = DefinitionId(format!("{}-residual-chassis", spec.id));
        let root = PartId(format!("{}-chassis", spec.id));
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
        let mut body = residual(spec);
        if spec.id != "350z" {
            body.visual.nodes = vec![(0, "GRID body".into())];
        }
        let mut definitions = BTreeMap::from([(chassis, body)]);
        for (i, (suffix, wheel_node, tire_node)) in [
            ("front_left", 1, 3),
            ("front_right", 2, 4),
            ("rear_left", 5, 7),
            ("rear_right", 6, 8),
        ]
        .into_iter()
        .enumerate()
        {
            let wheel = DefinitionId(format!("{}-test-wheel-{i}", spec.id));
            let nodes = if spec.id == "350z" {
                vec![
                    (wheel_node, format!("wheel_{suffix}")),
                    (tire_node, format!("tire_{suffix}")),
                ]
            } else {
                vec![(i + 1, format!("wheel_{suffix}"))]
            };
            definitions.insert(wheel.clone(),Definition {
                visual:visual(spec,nodes),
                collision:CollisionBinding::RaycastWheel {corner:i},
                interface:format!("{}-wheel-hub", spec.id),mass:25.,center:[0.;3],inertia:[[2.,0.,0.],[0.,2.,0.],[0.,0.,2.]],mounts:BTreeMap::new(),
                provenance:"P1 authored test wheel physical approximation; existing prepared wheel pivot binding".into(),
            });
            parts.push(Part {
                id: PartId(format!("{}-wheel-{i}", spec.id)),
                definition: wheel,
                attachment: Attachment::Mounted {
                    parent: root.clone(),
                    mount: format!("wheel-{i}"),
                },
                condition: 1.,
                secured: true,
            });
        }
        Some(Self {
            vehicle,
            owner,
            revision: 0,
            definitions,
            parts,
            prerequisites: Vec::new(),
            networks: Vec::new(),
            committed: BTreeMap::new(),
        })
    }
}
