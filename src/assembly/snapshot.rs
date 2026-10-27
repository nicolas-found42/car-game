//! Versioned normalized DTOs; catalog and runtime mappings remain external authority.
use super::*;
use crate::{diagnostics::RunHandle, runtime::QueuedCommand};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartDto {
    pub id: PartId,
    pub definition: DefinitionId,
    pub condition: f64,
    pub secured: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentDto {
    pub part: PartId,
    pub parent: PartId,
    pub mount: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootLocationDto {
    pub root: PartId,
    pub location: Location,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblySnapshot {
    pub assembly_version: u32,
    pub catalog_sha256: String,
    pub snapshot_tick: u64,
    pub vehicle: VehicleId,
    pub owner: OwnerId,
    pub revision: u64,
    pub parts: Vec<PartDto>,
    pub attachments: Vec<AttachmentDto>,
    pub root_locations: Vec<RootLocationDto>,
    pub prerequisites: Vec<(PartId, PartId)>,
    pub networks: Vec<NetworkEdge>,
    pub committed: Vec<Transaction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotSource {
    Saved { key: String },
    Inline { snapshot: Box<AssemblySnapshot> },
}
#[derive(Resource, Default)]
pub struct SnapshotStore(pub BTreeMap<String, AssemblySnapshot>);
#[derive(Resource, Default)]
pub struct SnapshotOrigins(pub BTreeMap<String, AssemblyRuntime>);
/// Disposable ECS identities for the persistent ledger, separate from rigid bodies.
#[derive(Component)]
pub struct PartIdentity(pub PartId);
fn catalog_hash(assembly: &Assembly) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&assembly.definitions).expect("catalog"))
    )
}
impl AssemblySnapshot {
    pub fn capture(assembly: &Assembly, snapshot_tick: u64) -> Self {
        let mut snapshot = Self {
            assembly_version: 1,
            catalog_sha256: catalog_hash(assembly),
            snapshot_tick,
            vehicle: assembly.vehicle.clone(),
            owner: assembly.owner.clone(),
            revision: assembly.revision,
            parts: Vec::new(),
            attachments: Vec::new(),
            root_locations: Vec::new(),
            prerequisites: assembly.prerequisites.clone(),
            networks: assembly.networks.clone(),
            committed: assembly.committed.values().cloned().collect(),
        };
        for part in &assembly.parts {
            snapshot.parts.push(PartDto {
                id: part.id.clone(),
                definition: part.definition.clone(),
                condition: part.condition,
                secured: part.secured,
            });
            match &part.attachment {
                Attachment::Mounted { parent, mount } => snapshot.attachments.push(AttachmentDto {
                    part: part.id.clone(),
                    parent: parent.clone(),
                    mount: mount.clone(),
                }),
                Attachment::Root { location } => snapshot.root_locations.push(RootLocationDto {
                    root: part.id.clone(),
                    location: location.clone(),
                }),
            }
        }
        snapshot.parts.sort_by(|a, b| a.id.cmp(&b.id));
        snapshot.attachments.sort_by(|a, b| a.part.cmp(&b.part));
        snapshot.root_locations.sort_by(|a, b| a.root.cmp(&b.root));
        snapshot
    }
    /// Build and validate a complete candidate without touching the active domain.
    pub fn stage(&self, current: &Assembly, expected_revision: u64) -> Result<Assembly, String> {
        if self.assembly_version != 1 {
            return Err("unsupported-assembly-version".into());
        }
        if expected_revision != current.revision {
            return Err("stale-revision".into());
        }
        if self.catalog_sha256 != catalog_hash(current) {
            return Err("catalog-mismatch".into());
        }
        if self.vehicle != current.vehicle || self.owner != current.owner {
            return Err("wrong-vehicle-or-owner".into());
        }
        let attachments = self.locations()?;
        let mut candidate = Assembly {
            vehicle: self.vehicle.clone(),
            owner: self.owner.clone(),
            revision: current
                .revision
                .max(self.revision)
                .checked_add(1)
                .ok_or("revision-overflow")?,
            definitions: current.definitions.clone(),
            parts: Vec::new(),
            prerequisites: self.prerequisites.clone(),
            networks: self.networks.clone(),
            committed: BTreeMap::new(),
        };
        for part in &self.parts {
            candidate.parts.push(Part {
                id: part.id.clone(),
                definition: part.definition.clone(),
                condition: part.condition,
                secured: part.secured,
                attachment: attachments
                    .get(&part.id)
                    .ok_or("missing-root-or-attachment")?
                    .clone(),
            });
        }
        if attachments.len() != candidate.parts.len() {
            return Err("dangling-or-duplicate-location".into());
        }
        candidate.validate()?;
        for transaction in &self.committed {
            if transaction.id.0.is_empty()
                || candidate
                    .committed
                    .insert(transaction.id.clone(), transaction.clone())
                    .is_some()
            {
                return Err("duplicate-or-empty-transaction-id".into());
            }
            validate_history(&candidate, transaction, self.revision)?;
        }
        Ok(candidate)
    }
    fn locations(&self) -> Result<BTreeMap<PartId, Attachment>, String> {
        let mut locations = BTreeMap::new();
        for edge in &self.attachments {
            if locations
                .insert(
                    edge.part.clone(),
                    Attachment::Mounted {
                        parent: edge.parent.clone(),
                        mount: edge.mount.clone(),
                    },
                )
                .is_some()
            {
                return Err("duplicate-parent-or-location".into());
            }
        }
        for root in &self.root_locations {
            if locations
                .insert(
                    root.root.clone(),
                    Attachment::Root {
                        location: root.location.clone(),
                    },
                )
                .is_some()
            {
                return Err("duplicate-parent-or-location".into());
            }
        }
        Ok(locations)
    }
}
fn validate_history(
    assembly: &Assembly,
    transaction: &Transaction,
    revision: u64,
) -> Result<(), String> {
    let part = assembly.part(&transaction.part)?;
    if transaction.expected_revision >= revision {
        return Err("invalid-transaction-history-revision".into());
    }
    match &transaction.destination {
        Attachment::Mounted { parent, mount } => {
            let parent_part = assembly.part(parent)?;
            let socket = assembly.definitions[&parent_part.definition]
                .mounts
                .get(mount)
                .ok_or("unknown-mount")?;
            if parent == &part.id {
                return Err("attachment-cycle".into());
            }
            if socket.interface != assembly.definitions[&part.definition].interface {
                return Err("incompatible-mount".into());
            }
        }
        Attachment::Root {
            location: Location::Vehicle { vehicle, owner },
        } => {
            if vehicle != &assembly.vehicle || owner != &assembly.owner {
                return Err("invalid-transaction-history-location".into());
            }
        }
        Attachment::Root {
            location: Location::Inventory { owner, slot },
        } => {
            if owner != &assembly.owner || slot.is_empty() {
                return Err("invalid-transaction-history-location".into());
            }
        }
    }
    Ok(())
}

fn rebuild_entities(world: &mut World) {
    let assembly = world.resource::<Assembly>();
    let ids: Vec<_> = assembly.parts.iter().map(|p| p.id.clone()).collect();
    let generation = world.resource::<AssemblyRuntime>().generation + 1;
    let mut fresh = BTreeMap::new();
    for id in ids {
        fresh.insert(id.clone(), world.spawn(PartIdentity(id)).id());
    }
    let mut runtime = world.resource_mut::<AssemblyRuntime>();
    let old = std::mem::replace(&mut runtime.parts, fresh);
    runtime.generation = generation;
    for entity in old.values().copied() {
        world.despawn(entity);
    }
}
pub fn snapshot_command(world: &mut World, queued: &QueuedCommand, key: &str) {
    if super::editions::prepared_hash(&world.resource::<crate::roster::CarSpec>().id).is_none() {
        crate::runtime::outcome(world, queued, "rejected", "unsupported-edition-assembly");
        return;
    }
    if key.is_empty() {
        crate::runtime::outcome(world, queued, "rejected", "empty-snapshot-key");
        return;
    }
    let tick = world.resource::<crate::runtime::SimulationClock>().current;
    let snapshot = AssemblySnapshot::capture(world.resource::<Assembly>(), tick);
    if let Err(reason) = persist_evidence(world, key, &snapshot) {
        crate::runtime::outcome(world, queued, "rejected", &reason);
        return;
    }
    let origin = world.resource::<AssemblyRuntime>().clone();
    world
        .resource_mut::<SnapshotOrigins>()
        .0
        .insert(key.into(), origin);
    world
        .resource_mut::<SnapshotStore>()
        .0
        .insert(key.into(), snapshot.clone());
    record(
        world,
        queued,
        "assembly.snapshot",
        serde_json::json!({"key":key,"snapshot":snapshot}),
    );
    crate::runtime::outcome(world, queued, "applied", "assembly-snapshotted");
}
fn persist_evidence(world: &World, key: &str, snapshot: &AssemblySnapshot) -> Result<(), String> {
    let Some(run) = world.get_resource::<RunHandle>() else {
        return Ok(());
    };
    let directory = run.dir().join("assembly-snapshots");
    let file = directory.join(format!("{:x}.json", Sha256::digest(key.as_bytes())));
    std::fs::create_dir_all(directory)
        .and_then(|()| std::fs::write(file, serde_json::to_vec_pretty(snapshot).expect("DTO")))
        .map_err(|error| format!("snapshot-evidence-write: {error}"))
}

pub fn restore_command(
    world: &mut World,
    queued: &QueuedCommand,
    source: &SnapshotSource,
    expected_revision: u64,
) {
    if super::editions::prepared_hash(&world.resource::<crate::roster::CarSpec>().id).is_none() {
        crate::runtime::outcome(world, queued, "rejected", "unsupported-edition-assembly");
        return;
    }
    let snapshot = match source {
        SnapshotSource::Saved { key } => world
            .resource::<SnapshotStore>()
            .0
            .get(key)
            .cloned()
            .ok_or_else(|| "unknown-snapshot".to_string()),
        SnapshotSource::Inline { snapshot } => Ok(*snapshot.clone()),
    };
    let result = snapshot
        .and_then(|snapshot| snapshot.stage(world.resource::<Assembly>(), expected_revision));
    match result {
        Ok(candidate) => {
            world.insert_resource(candidate);
            rebuild_entities(world);
            world.resource_mut::<AssemblyRuntime>().applied_revision = None;
            record(
                world,
                queued,
                "assembly.restored",
                serde_json::json!({"revision":world.resource::<Assembly>().revision,"generation":world.resource::<AssemblyRuntime>().generation,"part_ids":world.resource::<AssemblyRuntime>().parts.keys().collect::<Vec<_>>()}),
            );
            crate::runtime::outcome(world, queued, "applied", "assembly-restored");
        }
        Err(reason) => crate::runtime::outcome(world, queued, "rejected", &reason),
    }
}
fn record(world: &World, queued: &QueuedCommand, kind: &str, payload: serde_json::Value) {
    if let Some(run) = world.get_resource::<RunHandle>() {
        let mut envelope = queued.envelope.clone();
        envelope.tick = world.resource::<crate::runtime::SimulationClock>().current;
        envelope.boundary = "assembly_commit".into();
        run.event_envelope(kind, payload, envelope);
    }
}

/// Scenario observation through the same runtime identities rebuilt by restore.
pub fn observe_snapshot(world: &World, key: &str, fresh_entities: bool) -> serde_json::Value {
    let Some(saved) = world.resource::<SnapshotStore>().0.get(key) else {
        return serde_json::json!({"passed":false,"reason":"unknown-snapshot"});
    };
    let mut actual = AssemblySnapshot::capture(world.resource::<Assembly>(), saved.snapshot_tick);
    actual.revision = saved.revision;
    let entities = world.resource::<AssemblyRuntime>();
    let origin = world
        .resource::<SnapshotOrigins>()
        .0
        .get(key)
        .expect("snapshot origin");
    let fresh = entities.parts.len() == origin.parts.len()
        && entities.parts.iter().all(|(id, entity)| {
            origin
                .parts
                .get(id)
                .is_some_and(|old| old != entity && world.get_entity(*old).is_err())
                && world
                    .get::<PartIdentity>(*entity)
                    .is_some_and(|part| &part.0 == id)
        });
    serde_json::json!({"passed":actual == *saved && (!fresh_entities || fresh),"same_domain_state":actual == *saved,"fresh_entities":fresh,"generation":entities.generation,"part_count":entities.parts.len(),"snapshot":actual})
}
