//! Persistent major-part ownership; ECS identities never enter this ledger.
mod mass;
mod pilot;
mod runtime;
use bevy::math::{DMat3, DQuat, DVec3};
use bevy::prelude::*;
pub use mass::Aggregate;
pub use runtime::{AssemblyPlugin, AssemblyRuntime, command};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
macro_rules! id {
    ($($name:ident),*) => {$ (
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl From<&str> for $name { fn from(value: &str) -> Self { Self(value.into()) } }
    )*};
}
id!(VehicleId, PartId, DefinitionId, OwnerId, TransactionId);
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub position: [f64; 3],
    pub rotation: [f64; 4],
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mount {
    pub wheel_corner: Option<usize>,
    pub interface: String,
    pub frame: Frame,
    pub required: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualBinding {
    pub asset: String,
    pub prepared_sha256: String,
    pub scene: usize,
    pub nodes: Vec<(usize, String)>,
    pub picking: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollisionBinding {
    /// Existing chassis_collider(CarSpec): two authored compound cuboids, density zero.
    ResidualChassis,
    /// Existing raycast corner; no fabricated solid tire/hub collider.
    RaycastWheel { corner: usize },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    pub visual: VisualBinding,
    pub collision: CollisionBinding,
    pub interface: String,
    pub mass: f64,
    pub center: [f64; 3],
    pub inertia: [[f64; 3]; 3],
    pub mounts: BTreeMap<String, Mount>,
    pub provenance: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    Vehicle { vehicle: VehicleId, owner: OwnerId },
    Inventory { owner: OwnerId, slot: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Attachment {
    Root { location: Location },
    Mounted { parent: PartId, mount: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: PartId,
    pub definition: DefinitionId,
    pub attachment: Attachment,
    pub condition: f64,
    pub secured: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub from: PartId,
    pub to: PartId,
    pub network: String,
}
#[derive(Clone, Debug, PartialEq, Resource, Serialize, Deserialize)]
pub struct Assembly {
    pub vehicle: VehicleId,
    pub owner: OwnerId,
    pub revision: u64,
    pub definitions: BTreeMap<DefinitionId, Definition>,
    pub parts: Vec<Part>,
    /// Directed acyclic prerequisites are independent of ownership and fluid loops.
    pub prerequisites: Vec<(PartId, PartId)>,
    pub networks: Vec<NetworkEdge>,
    pub committed: BTreeMap<TransactionId, Transaction>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub expected_revision: u64,
    pub part: PartId,
    pub destination: Attachment,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inspection {
    pub vehicle: VehicleId,
    pub revision: u64,
    pub readiness: String,
    pub missing_mounts: Vec<String>,
    pub aggregate: Aggregate,
    pub parts: Vec<Part>,
    pub mounts: BTreeMap<DefinitionId, BTreeMap<String, Mount>>,
}
impl Assembly {
    pub fn part(&self, id: &PartId) -> Result<&Part, String> {
        self.parts
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| "unknown-part".into())
    }
    pub fn transact(&mut self, transaction: &Transaction) -> Result<&'static str, String> {
        if let Some(previous) = self.committed.get(&transaction.id) {
            return if previous == transaction {
                Ok("already-applied")
            } else {
                Err("transaction-id-conflict".into())
            };
        }
        if transaction.id.0.is_empty() {
            return Err("empty-transaction-id".into());
        }
        if transaction.expected_revision != self.revision {
            return Err("stale-revision".into());
        }
        let mut staged = self.clone();
        let part = staged
            .parts
            .iter_mut()
            .find(|p| p.id == transaction.part)
            .ok_or("unknown-part")?;
        part.attachment = transaction.destination.clone();
        staged.validate()?;
        staged.revision = staged.revision.checked_add(1).ok_or("revision-overflow")?;
        staged
            .committed
            .insert(transaction.id.clone(), transaction.clone());
        *self = staged;
        Ok("assembly-committed")
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.vehicle.0.is_empty() || self.owner.0.is_empty() {
            return Err("empty-vehicle-or-owner-id".into());
        }
        for (id, definition) in &self.definitions {
            if id.0.is_empty() {
                return Err("empty-definition-id".into());
            }
            mass::validate_definition(definition)?;
        }
        let mut ids = BTreeSet::new();
        let mut occupied = BTreeSet::new();
        let mut slots = BTreeSet::new();
        for part in &self.parts {
            if part.id.0.is_empty() || !ids.insert(&part.id) {
                return Err("duplicate-or-empty-part-id".into());
            }
            let definition = self
                .definitions
                .get(&part.definition)
                .ok_or("unknown-definition")?;
            mass::validate_definition(definition)?;
            if !part.condition.is_finite() || !(0.0..=1.0).contains(&part.condition) {
                return Err("invalid-condition".into());
            }
            self.validate_attachment(part, definition, &mut occupied, &mut slots)?;
            self.root(&part.id)?;
        }
        self.validate_edges()?;
        self.aggregate()?;
        Ok(())
    }
    fn validate_attachment<'a>(
        &'a self,
        part: &'a Part,
        definition: &Definition,
        occupied: &mut BTreeSet<(&'a PartId, &'a String)>,
        slots: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        match &part.attachment {
            Attachment::Mounted { parent, mount } => {
                let parent_part = self.part(parent)?;
                let socket = self
                    .definitions
                    .get(&parent_part.definition)
                    .and_then(|d| d.mounts.get(mount))
                    .ok_or("unknown-mount")?;
                if socket.interface != definition.interface {
                    return Err("incompatible-mount".into());
                }
                if !occupied.insert((parent, mount)) {
                    return Err("occupied-mount".into());
                }
            }
            Attachment::Root {
                location: Location::Vehicle { vehicle, owner },
            } => {
                if vehicle != &self.vehicle || owner != &self.owner {
                    return Err("wrong-vehicle-or-owner".into());
                }
                if !slots.insert("vehicle-root".to_string()) {
                    return Err("duplicate-vehicle-root".into());
                }
            }
            Attachment::Root {
                location: Location::Inventory { owner, slot },
            } => {
                if owner != &self.owner || slot.is_empty() {
                    return Err("wrong-owner-or-location".into());
                }
                if !slots.insert(format!("inventory:{slot}")) {
                    return Err("occupied-location".into());
                }
            }
        }
        Ok(())
    }
    fn validate_edges(&self) -> Result<(), String> {
        for edge in &self.networks {
            self.part(&edge.from)?;
            self.part(&edge.to)?;
        }
        for (from, to) in &self.prerequisites {
            self.part(from)?;
            self.part(to)?;
            let mut visiting = BTreeSet::new();
            self.visit_prerequisite(from, &mut visiting)?;
            if self.installed(from)? && !self.installed(to)? {
                return Err("missing-prerequisite".into());
            }
        }
        Ok(())
    }
    fn visit_prerequisite(
        &self,
        id: &PartId,
        visiting: &mut BTreeSet<PartId>,
    ) -> Result<(), String> {
        if !visiting.insert(id.clone()) {
            return Err("prerequisite-cycle".into());
        }
        for (_, required) in self.prerequisites.iter().filter(|(from, _)| from == id) {
            self.visit_prerequisite(required, visiting)?;
        }
        visiting.remove(id);
        Ok(())
    }
    pub fn root(&self, id: &PartId) -> Result<&Location, String> {
        let mut current = self.part(id)?;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(&current.id) {
                return Err("attachment-cycle".into());
            }
            match &current.attachment {
                Attachment::Root { location } => return Ok(location),
                Attachment::Mounted { parent, .. } => current = self.part(parent)?,
            }
        }
    }
    pub fn installed(&self, id: &PartId) -> Result<bool, String> {
        Ok(matches!(self.root(id)?, Location::Vehicle { .. }))
    }
    pub fn wheel_mask(&self) -> Result<[bool; 4], String> {
        let mut mask = [false; 4];
        for part in &self.parts {
            if !self.installed(&part.id)? {
                continue;
            }
            if let Attachment::Mounted { parent, mount } = &part.attachment
                && matches!(
                    self.definitions[&part.definition].collision,
                    CollisionBinding::RaycastWheel { .. }
                )
                && let Some(corner) =
                    self.definitions[&self.part(parent)?.definition].mounts[mount].wheel_corner
            {
                mask[corner] = true;
            }
        }
        Ok(mask)
    }
    pub fn inspect(&self) -> Result<Inspection, String> {
        let mut missing = Vec::new();
        let mut unsafe_parts = false;
        for part in &self.parts {
            if !self.installed(&part.id)? {
                continue;
            }
            unsafe_parts |= !part.secured || part.condition == 0.;
            for (name, socket) in &self.definitions[&part.definition].mounts {
                if socket.required && !self.parts.iter().any(|p| matches!(&p.attachment, Attachment::Mounted {parent, mount} if parent == &part.id && mount == name)) { missing.push(format!("{}:{name}",part.id.0)); }
            }
        }
        let readiness = if unsafe_parts {
            "unsafe"
        } else if !missing.is_empty() {
            "incomplete-inoperable"
        } else {
            "ready"
        };
        Ok(Inspection {
            vehicle: self.vehicle.clone(),
            revision: self.revision,
            readiness: readiness.into(),
            missing_mounts: missing,
            aggregate: self.aggregate()?,
            parts: self.parts.clone(),
            mounts: self
                .definitions
                .iter()
                .map(|(id, d)| (id.clone(), d.mounts.clone()))
                .collect(),
        })
    }
}
