//! Shared arcade braking and roll calibration, retaining prepared wheel geometry.
use super::STEP;

pub(super) fn tire_force(
    demand: bevy::prelude::Vec2,
    load: f32,
    mu: f32,
    lateral_mu: f32,
) -> bevy::prelude::Vec2 {
    if load <= 0. {
        return bevy::prelude::Vec2::ZERO;
    }
    if mu == lateral_mu {
        return demand.clamp_length_max(load * mu);
    }
    let utilization =
        bevy::prelude::Vec2::new(demand.x / (load * mu), demand.y / (load * lateral_mu)).length();
    demand / utilization.max(1.)
}

pub(super) fn braking_force(
    brake: f32,
    load: f32,
    total_load: f32,
    longitudinal: f32,
    rear_slide: bool,
    mass: f32,
    deceleration: f32,
) -> f32 {
    // Preserve the total service-brake demand while following weight transfer.
    // A fixed quarter per wheel clips an unloaded rear tire without using front grip.
    let service = if total_load > 0. {
        brake * mass * deceleration * load / total_load
    } else {
        0.
    };
    (service + if rear_slide { 6500. } else { 0. }).min(longitudinal.abs() * mass / (4. * STEP))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longitudinal_launch_grip_does_not_invent_lateral_grip() {
        use bevy::prelude::Vec2;
        let lateral = tire_force(Vec2::new(0., 1500.), 1000., 1.25, 0.9);
        assert!((lateral.y - 900.).abs() < 0.01, "{lateral:?}");
        let launch = tire_force(Vec2::new(1500., 0.), 1000., 1.25, 0.9);
        assert!((launch.x - 1250.).abs() < 0.01);
        let combined = tire_force(Vec2::new(1250., 900.), 1000., 1.25, 0.9);
        assert!(((combined.x / 1250.).powi(2) + (combined.y / 900.).powi(2) - 1.).abs() < 0.001);
    }

    #[test]
    fn service_braking_uses_loaded_front_grip_without_inventing_total_force() {
        let loads = [4600., 4600., 2000., 2000.];
        let forces = loads.map(|load| braking_force(1., load, 13200., 20., false, 1320., 10.));
        assert!(forces[0] > 1320. * 10. / 4.);
        assert!(forces[2] < 1320. * 10. / 4.);
        assert!((forces.iter().sum::<f32>() - 1320. * 10.).abs() < 0.01);
        assert!(
            forces
                .iter()
                .zip(loads)
                .all(|(force, load)| *force < load * 1.22)
        );
    }

    #[test]
    fn missing_support_has_no_service_brake_and_handbrake_remains_independent() {
        assert_eq!(braking_force(1., 0., 0., 20., false, 1320., 10.), 0.);
        assert_eq!(
            braking_force(0., 2000., 12000., 20., true, 1320., 10.),
            6500.
        );
        assert_eq!(braking_force(1., 4000., 12000., 0., false, 1320., 10.), 0.);
    }
}
