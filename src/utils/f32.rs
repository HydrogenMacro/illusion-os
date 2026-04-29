pub fn square_of(a: f32) -> f32 {
    return a * a;
}
pub fn hypot_squared_of(a: f32, b: f32) -> f32 {
    return a * a + b * b;
}
pub fn distance_squared_between(p1: (f32, f32), p2: (f32, f32)) -> f32 {
    return hypot_squared_of(p2.0 - p1.0, p2.1 - p1.1);
}