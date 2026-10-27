use super::*;

pub fn prepared_hash(id: &str) -> Option<&'static str> {
    match id {
        "350z" => Some("e79630558ed16c54942afc552838fd465a73bf507754947342bd65a805d37874"),
        "gt-rx7" => Some("47fa84d5ec58d9de08f851902aa0880b95ba4416a119f6fbeddaa0f129621ca2"),
        "gt-viper" => Some("7e61f66543554dd21a30f964c911ff30096d0c0cd51c8611b20f23b0205f594c"),
        "gt4-m3" => Some("edfad641515525b0e45873a27aff9079e353636978f4a17cec2c8f76a248b6db"),
        "gt4-nsx" => Some("443537bd92db0c56ef2cd2c36ffc1fb4cecccee8d3ae96c3cbf5d5000bf01788"),
        _ => None,
    }
}

pub fn vehicle_id(id: &str) -> VehicleId {
    if id == "350z" || prepared_hash(id).is_none() {
        VehicleId::from("pilot-350z")
    } else {
        VehicleId(format!("edition-{id}"))
    }
}

#[derive(Resource, Default)]
pub struct RetainedAssemblies(BTreeMap<VehicleId, (Assembly, AssemblyRuntime)>);

pub(crate) fn activate(world: &mut World, id: &str) {
    if prepared_hash(id).is_none() || world.resource::<Assembly>().vehicle == vehicle_id(id) {
        world.resource_mut::<AssemblyRuntime>().applied_revision = None;
        return;
    }
    let current = world.remove_resource::<Assembly>().expect("active graph");
    let runtime = world
        .remove_resource::<AssemblyRuntime>()
        .expect("active runtime");
    let mut retained = world.resource_mut::<RetainedAssemblies>();
    retained
        .0
        .insert(current.vehicle.clone(), (current, runtime));
    let (next, mut runtime) = retained
        .0
        .remove(&vehicle_id(id))
        .expect("retained edition");
    runtime.applied_revision = None;
    world.insert_resource(next);
    world.insert_resource(runtime);
}

pub(crate) fn initialize(world: &mut World) {
    let selected = world.resource::<crate::roster::CarSpec>().id.clone();
    let reference = world.resource::<crate::RuntimeConfig>().reference_physics;
    let mut retained = RetainedAssemblies::default();
    for spec in crate::roster::Fleet::default().cars {
        let Some(assembly) = Assembly::major_parts_configured(&spec, reference) else {
            continue;
        };
        assembly.validate().expect("edition manifest");
        let root = &world.resource::<crate::config::RuntimeConfig>().asset_root;
        let bytes = std::fs::read(root.join(&spec.path)).expect("prepared edition asset");
        assembly
            .validate_visuals(&spec, &bytes)
            .expect("authored visual ownership");
        let mut runtime = AssemblyRuntime::default();
        for part in &assembly.parts {
            runtime.parts.insert(
                part.id.clone(),
                world.spawn(PartIdentity(part.id.clone())).id(),
            );
        }
        runtime.generation = 1;
        retained
            .0
            .insert(assembly.vehicle.clone(), (assembly, runtime));
    }
    let (assembly, runtime) = retained
        .0
        .remove(&vehicle_id(&selected))
        .expect("initial ledger");
    world.insert_resource(assembly);
    world.insert_resource(runtime);
    world.insert_resource(retained);
}
