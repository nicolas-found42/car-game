//! Adjacent shift rules must allow engagement after a ratio change.
use car_game::vehicle::{Vehicle, profile};

#[test]
fn wide_ratio_upshift_recovers_drive_and_still_downshifts_when_slowing() {
    let profile = profile::for_car("exp-231");
    let radius = 0.3260708;
    let speed =
        8752. / profile.gears[0] / profile.final_drive * std::f32::consts::TAU * radius / 60.;
    let mut car = Vehicle::default();
    profile.drive_force(&mut car, speed, radius, 1.);
    assert_eq!(car.gear, 2);
    let mut force = 0.;
    // A small coast-down during disengagement must not immediately undo it.
    for _ in 0..60 {
        force = profile.drive_force(&mut car, speed * 0.99, radius, 1.);
        assert_eq!(car.gear, 2);
    }
    assert!(force > 0., "shift must finish and deliver torque");
    profile.drive_force(&mut car, speed * 0.8, radius, 0.);
    assert_eq!(car.gear, 1, "normal deceleration must permit a downshift");
}
