use sigmaos_tu_bylaws::*;

#[test]
fn component_is_constructible_and_runnable() {
    // Arrange
    let c = sigmaos_tu_bylaws::TuBylaws::new("sigmaos-tu-bylaws").expect("valid name");
    // Act
    let out = c.run().expect("run succeeds");
    // Assert
    assert!(out.contains("sigmaos-tu-bylaws"));
}
