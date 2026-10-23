use super::*;
use bevy_rapier3d::rapier::na::{Matrix3, SymmetricEigen, UnitQuaternion, Vector3};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aggregate {
    pub mass: f64,
    pub center: [f64; 3],
    pub inertia: [[f64; 3]; 3],
}
fn tensor(value: [[f64; 3]; 3]) -> Matrix3<f64> {
    Matrix3::from_fn(|r, c| value[r][c])
}
fn array(value: Matrix3<f64>) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| value[(r, c)]))
}
pub(super) fn validate_definition(d: &Definition) -> Result<(), String> {
    if matches!(d.collision,CollisionBinding::RaycastWheel {corner} if corner >=4) {
        return Err("invalid-wheel-corner".into());
    }
    let inertia = tensor(d.inertia);
    if inertia.iter().any(|v| !v.is_finite()) {
        return Err("invalid-mass-tensor".into());
    }
    let eigen = SymmetricEigen::new(inertia);
    if !d.mass.is_finite()
        || d.mass <= 0.
        || d.center.iter().any(|v| !v.is_finite())
        || inertia.iter().any(|v| !v.is_finite())
        || (inertia - inertia.transpose()).norm() > 1e-8
        || eigen.eigenvalues.min() <= 0.
        || eigen.eigenvalues.max() > eigen.eigenvalues.sum() / 2. + 1e-8
    {
        return Err("invalid-mass-tensor".into());
    }
    for mount in d.mounts.values() {
        let q = DQuat::from_array(mount.frame.rotation);
        if mount.wheel_corner.is_some_and(|corner| corner >= 4)
            || mount.interface.is_empty()
            || mount.frame.position.iter().any(|v| !v.is_finite())
            || !q.is_finite()
            || (q.length_squared() - 1.).abs() > 1e-8
        {
            return Err("invalid-mount-frame".into());
        }
    }
    Ok(())
}
impl Assembly {
    fn frame(&self, id: &PartId) -> Result<(DVec3, DQuat), String> {
        let part = self.part(id)?;
        match &part.attachment {
            Attachment::Root { .. } => Ok((DVec3::ZERO, DQuat::IDENTITY)),
            Attachment::Mounted { parent, mount } => {
                let (position, rotation) = self.frame(parent)?;
                let frame = &self.definitions[&self.part(parent)?.definition].mounts[mount].frame;
                Ok((
                    position + rotation * DVec3::from_array(frame.position),
                    rotation * DQuat::from_array(frame.rotation),
                ))
            }
        }
    }
    pub fn aggregate(&self) -> Result<Aggregate, String> {
        let mut samples = Vec::new();
        for part in &self.parts {
            if !self.installed(&part.id)? {
                continue;
            }
            let d = self
                .definitions
                .get(&part.definition)
                .ok_or("unknown-definition")?;
            let (position, rotation) = self.frame(&part.id)?;
            let center = position + rotation * DVec3::from_array(d.center);
            let r = DMat3::from_quat(rotation).to_cols_array_2d();
            let r = Matrix3::from_fn(|row, col| r[col][row]);
            samples.push((
                part.id.clone(),
                d.mass,
                Vector3::from(center.to_array()),
                r * tensor(d.inertia) * r.transpose(),
            ));
        }
        samples.sort_by(|a, b| a.0.cmp(&b.0));
        let mass: f64 = samples.iter().map(|s| s.1).sum();
        if !mass.is_finite() || mass <= 0. {
            return Err("empty-or-invalid-rigid-cluster".into());
        }
        let center = samples.iter().fold(Vector3::zeros(), |c, s| c + s.2 * s.1) / mass;
        let inertia = samples.iter().fold(Matrix3::zeros(), |i, s| {
            let offset = s.2 - center;
            i + s.3
                + s.1 * (Matrix3::identity() * offset.norm_squared() - offset * offset.transpose())
        });
        let aggregate = Aggregate {
            mass,
            center: center.into(),
            inertia: array(inertia),
        };
        aggregate.validate()?;
        Ok(aggregate)
    }
}
impl Aggregate {
    fn validate(&self) -> Result<(), String> {
        let entries = std::iter::once(&self.mass)
            .chain(self.center.iter())
            .chain(self.inertia.iter().flatten());
        if entries
            .into_iter()
            .any(|v| !v.is_finite() || !(*v as f32).is_finite())
            || (self.mass as f32) <= 0.
        {
            return Err("invalid-aggregate-mass-tensor".into());
        }
        let eigen = SymmetricEigen::new(tensor(self.inertia));
        if eigen
            .eigenvalues
            .iter()
            .any(|v| !v.is_finite() || !(*v as f32).is_finite() || (*v as f32) <= 0.)
            || eigen.eigenvalues.min() <= 0.
            || eigen.eigenvalues.max() > eigen.eigenvalues.sum() / 2. + 1e-7
        {
            return Err("invalid-aggregate-mass-tensor".into());
        }
        Ok(())
    }
    pub fn rapier(&self) -> bevy_rapier3d::prelude::AdditionalMassProperties {
        let eigen = SymmetricEigen::new(tensor(self.inertia));
        let mut axes = eigen.eigenvectors;
        if axes.determinant() < 0. {
            axes.set_column(0, &(-axes.column(0)));
        }
        let rotation = UnitQuaternion::from_matrix(&axes);
        let q = rotation.quaternion();
        bevy_rapier3d::prelude::AdditionalMassProperties::MassProperties(
            bevy_rapier3d::prelude::MassProperties {
                mass: self.mass as f32,
                local_center_of_mass: Vec3::from_array(self.center.map(|v| v as f32)),
                principal_inertia: Vec3::from_array(eigen.eigenvalues.data.0[0].map(|v| v as f32)),
                principal_inertia_local_frame: Quat::from_xyzw(
                    q.i as f32, q.j as f32, q.k as f32, q.w as f32,
                ),
            },
        )
    }
}
